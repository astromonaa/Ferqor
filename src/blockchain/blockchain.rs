use crate::address::{Address, Balances, COINBASE};
use crate::block::Block;
use crate::blockchain::error::{BlockAcceptanceError, BlockValidationError, MiningError};
use crate::blockchain::utils::Difficulty;
use crate::blockchain::validator::BlockchainValidator;
use crate::config::{GenesisConfig, GenesisData};
use crate::mempool::Mempool;
use crate::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: Difficulty,
    pub mempool: Mempool,
    pub balances: Balances,
    pub mining_reward: u64,
    pub initial_balances: Balances,
    pub nonces: HashMap<Address, u64>,
    pub initial_nonces: HashMap<Address, u64>,
    pub max_block_weight: usize,
}

impl Blockchain {
    pub fn new(genesis_data: GenesisData) -> Self {
        let mut chain = Vec::new();
        let GenesisData {
            mut genesis_block,
            initial_balances,
            max_block_weight,
        } = genesis_data;

        let difficulty = genesis_block.header.difficulty;

        genesis_block.mine_block();
        chain.push(genesis_block);

        Self {
            chain,
            difficulty,
            mempool: Mempool::new(),
            balances: initial_balances.clone(),
            mining_reward: 50,
            initial_balances,
            nonces: HashMap::new(),
            initial_nonces: HashMap::new(),
            max_block_weight,
        }
    }

    fn create_block(
        &self,
        transactions: Vec<Arc<Transaction>>,
    ) -> Result<Block, BlockValidationError> {
        let Some(previous_block) = self.chain.last() else {
            return Err(BlockValidationError::MissingChainTip);
        };
        let index = previous_block
            .header
            .index
            .checked_add(1)
            .ok_or(BlockValidationError::IndexOverflow)?;
        let now = GenesisConfig::get_now_timestamp();
        let new_block = Block::new(
            index,
            transactions,
            previous_block.hash,
            0,
            now,
            self.difficulty,
        );
        Ok(new_block)
    }

    fn accept_block(
        &mut self,
        candidate: Block,
        next_balances: Balances,
        next_nonces: HashMap<Address, u64>,
    ) -> Result<(), BlockAcceptanceError> {
        /* Проверяем валидность нового блока */
        BlockchainValidator::validate_next_block(self, &candidate, COINBASE)
            .map_err(BlockAcceptanceError::Validation)?;

        /* После успешной проверки добавляем блок в цепь */
        self.balances = next_balances;
        self.nonces = next_nonces;
        self.chain.push(candidate);
        Ok(())
    }

    pub fn add_block(
        &mut self,
        transactions: Vec<Arc<Transaction>>,
    ) -> Result<(), BlockAcceptanceError> {
        let mut candidate = self
            .create_block(transactions)
            .map_err(BlockAcceptanceError::Validation)?;
        /* Сперва проверям возможное применение нового блока */
        let (next_balances, next_nonces) = BlockchainValidator::calculate_next_state(
            &self.balances,
            &self.nonces,
            &candidate,
            COINBASE,
        )?;

        /* После успешной проверки майним блок */
        candidate.mine_block();

        /* После успешного майнинга добавляем блок в цепь */
        self.accept_block(candidate, next_balances, next_nonces)
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> bool {
        if transaction.from == COINBASE {
            return false;
        }

        let from_balance = self.balances.get(&transaction.from).copied().unwrap_or(0);
        let current_nonce = self.nonces.get(&transaction.from).copied().unwrap_or(0);

        let pending_count = self.mempool.pending_count(&transaction.from);

        /* Даем возможность отправить несколько транзакций в mempool */
        let Some(expected_nonce) = current_nonce.checked_add(pending_count) else {
            return false;
        };

        /* Проверяем, что nonce в порядке */
        if transaction.nonce != expected_nonce {
            return false;
        };

        /* Считаем стоимость всех транзакций пользователя в mempool */
        let Some(pending_cost) = self.mempool.pending_cost(&transaction.from) else {
            return false;
        };

        /* Считаем стоимость новой транзакции */
        let Some(tx_cost) = transaction.amount.checked_add(transaction.fee) else {
            return false;
        };

        /* Считаем общую стоимость всех транзакций пользователя с учетом текущей */
        let Some(total_cost) = pending_cost.checked_add(tx_cost) else {
            return false;
        };

        /* Если стоимость больше баланса пользователя, не добавляем */
        if from_balance < total_cost {
            return false;
        };

        if !transaction.is_valid() {
            return false;
        };

        self.mempool.insert(transaction)
    }

    fn get_reward_transaction(
        &self,
        miner_address: Address,
        total_fees: u64,
    ) -> Option<Arc<Transaction>> {
        let amount = self.mining_reward.checked_add(total_fees)?;
        Some(Arc::new(Transaction {
            from: COINBASE,
            to: miner_address,
            amount,
            signature: None,
            public_key: None,
            fee: 0,
            nonce: 0,
        }))
    }

    pub fn mine_pending_transactions(&mut self, miner_address: Address) -> Result<(), MiningError> {
        /* Из максимальной весовой емкости вычитываем сперва вес Coinbase транзакции */
        let available_weight = self
            .max_block_weight
            .checked_sub(Transaction::FIXED_WEIGHT)
            .ok_or(MiningError::BlockCapacityTooSmall {
                required: Transaction::FIXED_WEIGHT,
                max_weight: self.max_block_weight,
            })?;

        let pending_transactions = self
            .mempool
            .select_transactions(available_weight, &self.nonces);

        if pending_transactions.is_empty() {
            return Ok(());
        };

        let total_fees = pending_transactions
            .iter()
            .try_fold(0u64, |acc, tx| acc.checked_add(tx.fee))
            .ok_or(MiningError::FeeOverflow)?;

        let reward_tx = self
            .get_reward_transaction(miner_address, total_fees)
            .ok_or(MiningError::RewardOverflow)?;

        let mut transactions = Vec::with_capacity(pending_transactions.len() + 1);
        transactions.push(reward_tx);
        transactions.extend(pending_transactions.iter().cloned());

        self.add_block(transactions)
            .map_err(MiningError::BlockAcceptance)?;

        self.mempool
            .remove_selected_transactions(&pending_transactions);

        Ok(())
    }

    fn is_valid_genesis_block(&self) -> bool {
        /* Проверяем вес генезис-блока */
        let Some(genesis_block) = self.chain.first() else {
            return false;
        };

        if !BlockchainValidator::check_block_weight(genesis_block, self.max_block_weight) {
            return false;
        }

        let calculated_hash = Block::calculate_hash(&genesis_block.header);

        if calculated_hash != genesis_block.hash {
            return false;
        }

        if !calculated_hash.has_leading_zero_hex_digits(genesis_block.header.difficulty) {
            return false;
        }

        true
    }

    pub fn is_valid(&self) -> bool {
        /* Проверяем валидность генезис-блока */
        if !self.is_valid_genesis_block() {
            return false;
        }
        /* Проверяем вес и транзакции остальных блоков */
        for index in 1..self.chain.len() {
            let current_block = &self.chain[index];
            let previous_block = &self.chain[index - 1];

            if !BlockchainValidator::check_block_weight(current_block, self.max_block_weight) {
                return false;
            }

            let is_valid_coinbase_transaction = BlockchainValidator::check_coinbase_transaction(
                current_block,
                self.mining_reward,
                COINBASE,
            );

            let calculated_merkle_root = Block::calculate_merkle_root(&current_block.transactions);

            let is_valid_merkle_root = calculated_merkle_root == current_block.header.merkle_root;

            if !is_valid_merkle_root {
                return false;
            }

            if current_block.header.difficulty != self.difficulty {
                return false;
            }

            let calculated_hash = Block::calculate_hash(&current_block.header);

            let is_valid_hash = calculated_hash == current_block.hash
                && calculated_hash.has_leading_zero_hex_digits(current_block.header.difficulty);

            if !is_valid_hash {
                return false;
            }

            if current_block.header.previous_hash != previous_block.hash {
                return false;
            }

            if !is_valid_coinbase_transaction {
                return false;
            }
        }

        BlockchainValidator::validate_state_transitions(self, COINBASE)
    }
}
