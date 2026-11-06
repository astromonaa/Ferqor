//! Публичный фасад блокчейна, сохранённый из прежнего
//! `blockchain::blockchain::Blockchain`.
//!
//! Тип координирует выделенные `Chain`, `Mempool` и `BlockProducer`, сохраняя имена
//! операций `new`, `add_block`, `receive_block`, `add_transaction`,
//! `mine_pending_transactions` и `is_valid` без изменения их семантики.

use std::sync::Arc;

use crate::address::{Address, COINBASE};
use crate::block::Block;
use crate::blockchain::error::{BlockAcceptanceError, BlockValidationError, MiningError};
use crate::chain::Chain;
use crate::config::GenesisData;
use crate::consensus::ConsensusParams;
use crate::ledger::LedgerState;
use crate::mempool::Mempool;
use crate::mining::BlockProducer;
use crate::transaction::Transaction;

/// Объединяет `Chain`, `Mempool` и `BlockProducer` логикой, перенесённой из
/// прежнего `blockchain::blockchain::Blockchain`.
pub struct Blockchain {
    chain: Chain,
    mempool: Mempool,
    block_producer: BlockProducer,
}

impl Blockchain {
    /// Инициализирует состояние логикой из прежнего
    /// `blockchain::blockchain::Blockchain::new`.
    ///
    /// Добыча по-прежнему выполняется исходным `Block::mine_block`, сохраняя порядок
    /// проверок, ошибки и байты блока.
    pub fn new(mut genesis_data: GenesisData) -> Result<Self, BlockValidationError> {
        Chain::validate_genesis_candidate(&genesis_data.genesis_block)?;

        let block_producer = BlockProducer;
        genesis_data.genesis_block.mine_block();
        let chain = Chain::from_validated_genesis(genesis_data);

        Ok(Self {
            chain,
            mempool: Mempool::new(),
            block_producer,
        })
    }

    /// Предоставляет агрегат вместо прежнего публичного поля `Blockchain::chain`.
    pub fn chain(&self) -> &Chain {
        &self.chain
    }

    /// Предоставляет блоки из прежнего публичного поля `Blockchain::chain`.
    pub fn blocks(&self) -> &[Block] {
        self.chain.blocks()
    }

    /// Предоставляет реестр вместо прежних полей `Blockchain::balances` и
    /// `Blockchain::nonces`.
    pub fn ledger(&self) -> &LedgerState {
        self.chain.ledger()
    }

    /// Предоставляет пул из прежнего публичного поля `Blockchain::mempool`.
    pub fn mempool(&self) -> &Mempool {
        &self.mempool
    }

    /// Предоставляет сгруппированные прежние поля `Blockchain::mining_reward`,
    /// `Blockchain::max_block_weight` и константы сложности.
    pub fn consensus_params(&self) -> &ConsensusParams {
        self.chain.consensus_params()
    }

    /// Возвращает значение прежнего поля `Blockchain::mining_reward`.
    pub fn mining_reward(&self) -> u64 {
        self.consensus_params().mining_reward()
    }

    /// Возвращает значение прежнего поля `Blockchain::max_block_weight`.
    pub fn max_block_weight(&self) -> usize {
        self.consensus_params().max_block_weight()
    }

    /// Сохраняет порядок операций прежнего
    /// `blockchain::blockchain::Blockchain::receive_block`: валидация, расчёт
    /// следующего состояния, очистка пула и только затем фиксация цепочки.
    pub fn receive_block(&mut self, candidate: Block) -> Result<(), BlockAcceptanceError> {
        self.chain.validate_candidate(&candidate)?;
        let next_ledger = self.chain.ledger().apply_block(&candidate, COINBASE)?;
        self.remove_committed_transactions(&candidate);
        self.chain.commit_block(candidate, next_ledger);
        Ok(())
    }

    /// Сохраняет логику прежнего `Blockchain::add_block`:
    /// `BlockProducer` создаёт блок, а исходный `Block::mine_block` выполняет добычу.
    pub fn create_block(
        &mut self,
        transactions: Vec<Arc<Transaction>>,
    ) -> Result<Option<(Block, LedgerState)>, BlockAcceptanceError> {
        let candidate = self
            .block_producer
            .create_block(&self.chain, transactions)
            .map_err(BlockAcceptanceError::Validation)?;

        // Прежний `Blockchain::add_block` вычислял состояние ровно один раз до PoW.
        let next_ledger = self.chain.ledger().apply_block(&candidate, COINBASE)?;

        // candidate.mine_block();
        Ok(Some((candidate, next_ledger)))
    }

    pub fn add_block(&mut self, block: Block, next_ledger: LedgerState) -> Result<Block, BlockAcceptanceError> {
        self.chain.validate_candidate(&block)?;
        self.remove_committed_transactions(&block);
        self.chain.commit_block(block.clone(), next_ledger);

        Ok(block)
    }

    /// Сохраняет приём в пул из прежнего `Blockchain::add_transaction` без изменений.
    pub fn add_transaction(&mut self, transaction: Transaction) -> bool {
        if transaction.from == COINBASE {
            return false;
        }

        let from_balance = self.ledger().balance(&transaction.from);
        let current_nonce = self.ledger().nonce(&transaction.from);
        let pending_count = self.mempool.pending_count(&transaction.from);

        let Some(expected_nonce) = current_nonce.checked_add(pending_count) else {
            return false;
        };

        if transaction.nonce != expected_nonce {
            return false;
        };

        let Some(pending_cost) = self.mempool.pending_cost(&transaction.from) else {
            return false;
        };
        let Some(transaction_cost) = transaction.amount.checked_add(transaction.fee) else {
            return false;
        };
        let Some(total_cost) = pending_cost.checked_add(transaction_cost) else {
            return false;
        };

        if from_balance < total_cost || !transaction.is_valid() {
            return false;
        }

        self.mempool.insert(transaction)
    }

    /// Формирует и добывает блок из подходящих транзакций mempool.
    ///
    /// Метод сохраняет расчёт вместимости, комиссий и награды из прежнего
    /// `Blockchain::mine_pending_transactions`. Он выполняет следующие шаги:
    ///
    /// 1. Резервирует в блоке место для обязательной coinbase-транзакции.
    /// 2. Выбирает из mempool обычные транзакции, которые помещаются в оставшийся
    ///    вес и образуют корректные последовательности nonce для отправителей.
    /// 3. Безопасно суммирует комиссии и создаёт coinbase с базовой наградой и fee.
    /// 4. Помещает coinbase на обязательную позицию `0` и передаёт транзакции в
    ///    `add_block`, где блок создаётся, добывается, валидируется и фиксируется.
    ///
    /// Если подходящих обычных транзакций нет, новый блок не создаётся.
    pub fn mine_pending_transactions(&mut self, miner_address: Address) -> Result<Option<(Block, LedgerState)>, MiningError> {
        let max_block_weight = self.consensus_params().max_block_weight();

        // `select_transactions` выбирает только обычные транзакции из mempool, но
        // итоговый блок дополнительно содержит coinbase. Поэтому её фиксированный
        // вес заранее вычитается из общего consensus-лимита блока.
        //
        // `checked_sub` не позволяет `usize` уйти ниже нуля, если конфигурация
        // задаёт лимит меньше веса обязательной coinbase. Вместо переполнения метод
        // возвращает явную ошибку `BlockCapacityTooSmall`.
        let available_weight = max_block_weight
            .checked_sub(Transaction::FIXED_WEIGHT)
            .ok_or(MiningError::BlockCapacityTooSmall {
                required: Transaction::FIXED_WEIGHT,
                max_weight: max_block_weight,
            })?;

        // `available_weight` — часть лимита блока, оставшаяся для обычных транзакций
        // после резервирования coinbase. Текущие nonce реестра задают первый
        // допустимый nonce каждого отправителя; mempool не пропускает разрывы.
        let pending_transactions = self
            .mempool
            .select_transactions(available_weight, self.ledger().nonces());

        // Пустой блок только с наградой здесь не добывается: майнинг запускается,
        // только если mempool предоставил хотя бы одну подходящую транзакцию.
        if pending_transactions.is_empty() {
            return Ok(None);
        }

        // `try_fold` складывает fee без оборачивания `u64`. Переполнение означает,
        // что корректную итоговую coinbase-награду представить невозможно.
        let total_fees = pending_transactions
            .iter()
            .try_fold(0u64, |acc, transaction| acc.checked_add(transaction.fee))
            .ok_or(MiningError::FeeOverflow)?;

        // Итоговая награда равна базовой награде из ConsensusParams плюс total_fees.
        // Внутри также используется checked_add, поэтому переполнение возвращается
        // как `RewardOverflow`, а не создаёт транзакцию с некорректной суммой.
        let reward_transaction = self
            .block_producer
            .get_reward_transaction(self.consensus_params(), miner_address, total_fees)
            .ok_or(MiningError::RewardOverflow)?;

        // Ёмкость известна заранее: одна coinbase и все выбранные транзакции.
        // Coinbase обязана находиться первой — валидатор интерпретирует позицию `0`
        // как единственную наградную транзакцию и считает fee начиная с позиции `1`.
        // При `cloned` копируется только `Arc`, а не содержимое каждой Transaction.
        let mut transactions = Vec::with_capacity(pending_transactions.len() + 1);
        transactions.push(reward_transaction);
        transactions.extend(pending_transactions.iter().cloned());

        // `add_block` сохраняет исходный порядок операций: создаёт кандидата,
        // рассчитывает следующее состояние, выполняет PoW, валидирует и фиксирует блок.
        self.create_block(transactions)
            .map_err(MiningError::BlockAcceptance)
    }

    /// Сохраняет проверку из прежнего `Blockchain::is_valid`.
    pub fn is_valid(&self) -> bool {
        self.chain.is_valid()
    }

    /// Удаляет из пула транзакции блока логикой из прежнего
    /// `Blockchain::commit_block`, пропуская coinbase на позиции `0`.
    fn remove_committed_transactions(&mut self, candidate: &Block) {
        for transaction in candidate.transactions.iter().skip(1) {
            self.mempool.remove(&transaction.txid());
        }
    }

    /// Заменяет прямое изменение прежних полей состояния в тестах.
    #[cfg(test)]
    pub(crate) fn ledger_mut_for_test(&mut self) -> &mut LedgerState {
        self.chain.ledger_mut_for_test()
    }
}
