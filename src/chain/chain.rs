//! Агрегат цепочки блоков, выделенный из `blockchain::blockchain::Blockchain`.
//!
//! Он владеет только каноническими блоками, текущим и начальным состоянием реестра,
//! а пул транзакций и майнинг из прежнего `Blockchain` остаются снаружи `Chain`.

use std::collections::HashMap;

use num_bigint::BigUint;

use super::validator::BlockchainValidator;
use crate::address::COINBASE;
use crate::block::Block;
use crate::block::block::BlockHash;
use crate::blockchain::error::{BlockAcceptanceError, BlockValidationError};
use crate::chain::{ChainRegistry, IndexedBlock};
use crate::config::GenesisData;
use crate::consensus::ConsensusParams;
use crate::ledger::LedgerState;

/// Владеет данными, ранее представленными `Blockchain::chain`, балансами,
/// nonce, начальным состоянием и конфигурацией консенсуса.
pub struct Chain {
    blocks: Vec<Block>,
    ledger: LedgerState,
    initial_ledger: LedgerState,
    consensus_params: ConsensusParams,
    registry: ChainRegistry,
}

impl Chain {
    /// Инициализирует каноническое состояние цепочки логикой из прежнего `Blockchain::new`.
    ///
    /// Переданный генезис-блок уже должен пройти `mine_block` через сервис майнинга.
    /// Проверки конструктора сохраняют прежнюю семантику индекса и корня Меркла;
    /// полная валидация генезис-блока остаётся в `is_valid`.
    pub fn new(genesis_data: GenesisData) -> Result<Self, BlockValidationError> {
        Self::validate_genesis_candidate(&genesis_data.genesis_block)?;
        Ok(Self::from_validated_genesis(genesis_data))
    }

    /// Собирает цепочку после проверки, выполненной прежним
    /// `Blockchain::new` до вызова `Block::mine_block`.
    pub(crate) fn from_validated_genesis(genesis_data: GenesisData) -> Self {
        let GenesisData {
            genesis_block,
            initial_balances,
            max_block_weight,
        } = genesis_data;
        let initial_ledger = LedgerState::from_balances(initial_balances);

        Self {
            blocks: vec![genesis_block.clone()],
            ledger: initial_ledger.clone(),
            initial_ledger,
            consensus_params: ConsensusParams::default().with_max_block_weight(max_block_weight),
            registry: HashMap::from([(
                genesis_block.hash,
                IndexedBlock::new(genesis_block, BigUint::from(1u32)),
            )]),
        }
    }

    /// Выполняет проверки генезис-блока до майнинга, ранее находившиеся в `Blockchain::new`.
    pub fn validate_genesis_candidate(block: &Block) -> Result<(), BlockValidationError> {
        let expected_merkle_root = Block::calculate_merkle_root(&block.transactions);

        if block.header.merkle_root != expected_merkle_root {
            return Err(BlockValidationError::MerkleRootMismatch);
        }

        if block.header.index != 0 {
            return Err(BlockValidationError::UnexpectedIndex {
                expected: 0,
                actual: block.header.index,
            });
        }

        Ok(())
    }

    /// Выполняет полные проверки генезис-блока из прежнего
    /// `Blockchain::is_valid_genesis_block`.
    fn validate_genesis_block(
        block: &Block,
        params: &ConsensusParams,
    ) -> Result<(), BlockValidationError> {
        if block.header.index != 0 {
            return Err(BlockValidationError::UnexpectedIndex {
                expected: 0,
                actual: block.header.index,
            });
        }

        let expected_merkle_root = Block::calculate_merkle_root(&block.transactions);

        if block.header.merkle_root != expected_merkle_root {
            return Err(BlockValidationError::MerkleRootMismatch);
        }

        if block.header.previous_hash != BlockHash([0u8; 32]) {
            return Err(BlockValidationError::PreviousHashMismatch);
        }

        let weight = block.weight().ok_or(BlockValidationError::WeightOverflow)?;

        if weight > params.max_block_weight() {
            return Err(BlockValidationError::BlockTooHeavy {
                weight,
                max_weight: params.max_block_weight(),
            });
        }

        let calculated_hash = Block::calculate_hash(&block.header);

        if calculated_hash != block.hash {
            return Err(BlockValidationError::HashMismatch);
        }

        if !calculated_hash.has_leading_zero_hex_digits(block.header.difficulty) {
            return Err(BlockValidationError::InvalidProofOfWork);
        }

        Ok(())
    }

    /// Возвращает блоки, ранее доступные через `Blockchain::chain`.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Возвращает вершину цепочки, ранее получаемую через `Blockchain::chain.last()`.
    pub fn tip(&self) -> Option<&Block> {
        self.blocks.last()
    }

    /// Возвращает состояние, ранее хранившееся в `Blockchain::balances` и `Blockchain::nonces`.
    pub fn ledger(&self) -> &LedgerState {
        &self.ledger
    }

    /// Возвращает значения, ранее хранившиеся в `Blockchain::mining_reward` и
    /// `Blockchain::max_block_weight`, а также константы сложности.
    pub fn consensus_params(&self) -> &ConsensusParams {
        &self.consensus_params
    }

    /// Сохраняет прежний приватный метод `Blockchain::validate_candidate` у владельца
    /// цепочки без изменения порядка валидации и возвращаемых ошибок.
    pub(crate) fn validate_candidate(&self, candidate: &Block) -> Result<(), BlockAcceptanceError> {
        BlockchainValidator::validate_next_block(
            &self.blocks,
            candidate,
            COINBASE,
            &self.consensus_params,
        )
        .map_err(BlockAcceptanceError::Validation)
    }

    /// Сохраняет часть прежнего приватного метода `Blockchain::commit_block`,
    /// относящуюся к состоянию цепочки. Очистка пула транзакций остаётся в
    /// `Blockchain::receive_block`, у владельца пула.
    pub(crate) fn commit_block(&mut self, candidate: Block, next_ledger: LedgerState) {
        self.ledger = next_ledger;
        self.blocks.push(candidate);
    }

    /// Применяет логику приёма, перенесённую из `Blockchain::receive_block`.
    ///
    /// Метод используется, когда координация с пулом транзакций не требуется.
    pub fn receive_block(&mut self, candidate: Block) -> Result<(), BlockAcceptanceError> {
        self.validate_candidate(&candidate)?;
        let next_ledger = self.ledger.apply_block(&candidate, COINBASE)?;
        self.commit_block(candidate, next_ledger);
        Ok(())
    }

    /// Сохраняет прежнюю приватную проверку `Blockchain::is_valid_genesis_block`.
    fn is_valid_genesis_block(&self) -> bool {
        let Some(genesis_block) = self.blocks.first() else {
            return false;
        };

        Self::validate_genesis_block(genesis_block, &self.consensus_params).is_ok()
    }

    /// Сохраняет проверку цепочки и состояния из прежнего `Blockchain::is_valid`.
    pub fn is_valid(&self) -> bool {
        if !self.is_valid_genesis_block() {
            return false;
        }

        for index in 1..self.blocks.len() {
            let current_block = &self.blocks[index];
            let previous_chain = &self.blocks[..index];

            if BlockchainValidator::validate_next_block(
                previous_chain,
                current_block,
                COINBASE,
                &self.consensus_params,
            )
            .is_err()
            {
                return false;
            }
        }

        BlockchainValidator::validate_state_transitions(self, COINBASE)
    }

    /// Предоставляет начальное состояние, ранее считываемое из `Blockchain::initial_balances`
    /// и `Blockchain::initial_nonces` при повторном выполнении валидатором.
    pub(crate) fn initial_ledger(&self) -> &LedgerState {
        &self.initial_ledger
    }

    /// Заменяет прямое изменение прежнего публичного поля `Blockchain::nonces` в тестах.
    #[cfg(test)]
    pub(crate) fn ledger_mut_for_test(&mut self) -> &mut LedgerState {
        &mut self.ledger
    }
}
