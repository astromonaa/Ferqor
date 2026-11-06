//! Валидация блоков, перенесённая из `blockchain::validator` к владельцу `chain`.

use crate::address::Address;
use crate::block::Block;
use crate::blockchain::error::BlockValidationError;
use crate::chain::Chain;
use crate::consensus::{ConsensusParams, expected_difficulty_for_next_block};

/// Валидатор без состояния, перенесённый из `blockchain::validator::BlockchainValidator`.
pub struct BlockchainValidator;

impl BlockchainValidator {
    /// Сохраняет `blockchain::validator::BlockchainValidator::check_block_weight`.
    pub fn check_block_weight(block: &Block, max_block_weight: usize) -> bool {
        matches!(
            block.weight(),
            Some(weight) if weight <= max_block_weight
        )
    }

    /// Сохраняет
    /// `blockchain::validator::BlockchainValidator::check_coinbase_transaction`.
    pub fn check_coinbase_transaction(
        current_block: &Block,
        mining_reward: u64,
        coinbase: Address,
    ) -> bool {
        let Some(first_tx) = current_block.transactions.first() else {
            return false;
        };

        if first_tx.from != coinbase
            || first_tx.fee != 0
            || first_tx.nonce != 0
            || first_tx.signature.is_some()
            || first_tx.public_key.is_some()
        {
            return false;
        }

        /* Как в прежнем `BlockchainValidator::check_coinbase_transaction`, пропускаем первую наградную транзакцию: она не платит комиссию. */
        let Some(total_fees) = current_block
            .transactions
            .iter()
            .skip(1)
            .try_fold(0u64, |acc, tx| acc.checked_add(tx.fee))
        else {
            return false;
        };

        let Some(expected_reward) = mining_reward.checked_add(total_fees) else {
            return false;
        };

        if first_tx.amount != expected_reward {
            return false;
        }

        let coinbase_transactions = current_block
            .transactions
            .iter()
            .filter(|tx| tx.from == coinbase)
            .count();

        coinbase_transactions == 1
    }

    /// Сохраняет `BlockchainValidator::validate_state_transitions`, теперь повторно
    /// вычисляя состояние через аксессоры `Chain`, а не через поля `Blockchain`.
    pub fn validate_state_transitions(chain: &Chain, coinbase: Address) -> bool {
        let mut ledger = chain.initial_ledger().clone();

        for block in chain.blocks() {
            let Ok(next_ledger) = ledger.apply_block(block, coinbase) else {
                return false;
            };
            ledger = next_ledger;
        }

        &ledger == chain.ledger()
    }

    /// Валидирует следующий блок реализацией, перенесённой из
    /// `blockchain::validator::BlockchainValidator::validate_next_block`.
    pub fn validate_next_block(
        chain: &[Block],
        block: &Block,
        coinbase: Address,
        params: &ConsensusParams,
    ) -> Result<(), BlockValidationError> {
        let last = chain.last().ok_or(BlockValidationError::MissingChainTip)?;

        let expected_index = last
            .header
            .index
            .checked_add(1)
            .ok_or(BlockValidationError::IndexOverflow)?;

        if block.header.index != expected_index {
            return Err(BlockValidationError::UnexpectedIndex {
                expected: expected_index,
                actual: block.header.index,
            });
        }

        if block.header.previous_hash != last.hash {
            return Err(BlockValidationError::PreviousHashMismatch);
        }

        if block.header.timestamp < last.header.timestamp {
            return Err(BlockValidationError::TimestampBeforePrevious {
                previous: last.header.timestamp,
                actual: block.header.timestamp,
            });
        }

        let expected_merkle_root = Block::calculate_merkle_root(&block.transactions);

        if block.header.merkle_root != expected_merkle_root {
            return Err(BlockValidationError::MerkleRootMismatch);
        }

        let calculated_block_hash = Block::calculate_hash(&block.header);

        if block.hash != calculated_block_hash {
            return Err(BlockValidationError::HashMismatch);
        }

        let expected_difficulty = expected_difficulty_for_next_block(chain, params)?;

        if block.header.difficulty != expected_difficulty {
            return Err(BlockValidationError::UnexpectedDifficulty {
                expected: expected_difficulty,
                actual: block.header.difficulty,
            });
        }

        if !calculated_block_hash.has_leading_zero_hex_digits(block.header.difficulty) {
            return Err(BlockValidationError::InvalidProofOfWork);
        }

        let weight = block.weight().ok_or(BlockValidationError::WeightOverflow)?;

        if weight > params.max_block_weight() {
            return Err(BlockValidationError::BlockTooHeavy {
                weight,
                max_weight: params.max_block_weight(),
            });
        }

        let is_valid_coinbase_transaction = BlockchainValidator::check_coinbase_transaction(
            block,
            params.mining_reward(),
            coinbase,
        );

        if !is_valid_coinbase_transaction {
            return Err(BlockValidationError::InvalidCoinbaseTransaction);
        }

        Ok(())
    }
}
