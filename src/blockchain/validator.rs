use std::collections::HashMap;

use crate::address::{Address, Balances};
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::blockchain::difficulty::expected_difficulty_for_next_block;
use crate::blockchain::error::{BlockAcceptanceError, BlockValidationError};
use crate::blockchain::state_transitions::apply_transaction_to_state;

pub struct BlockchainValidator;

impl BlockchainValidator {
    pub fn check_block_weight(block: &Block, max_block_weight: usize) -> bool {
        matches!(
            block.weight(),
            Some(weight) if weight <= max_block_weight
        )
    }

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

        /* Делаем skip(1), чтобы пропустить первый Coinbase транзакцию, т.к. Coinbase не платит fee */
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

    pub fn calculate_next_state(
        balances: &Balances,
        nonces: &HashMap<Address, u64>,
        block: &Block,
        coinbase: Address,
    ) -> Result<(Balances, HashMap<Address, u64>), BlockAcceptanceError> {
        let mut next_balances = balances.clone();
        let mut next_nonces = nonces.clone();

        for (transaction_index, transaction) in block.transactions.iter().enumerate() {
            apply_transaction_to_state(transaction, &mut next_balances, &mut next_nonces, coinbase)
                .map_err(|source| BlockAcceptanceError::StateTransition {
                    transaction_index,
                    source,
                })?
        }

        Ok((next_balances, next_nonces))
    }

    pub fn validate_state_transitions(blockchain: &Blockchain, coinbase: Address) -> bool {
        let mut balances = blockchain.initial_balances.clone();
        let mut nonces = blockchain.initial_nonces.clone();

        for block in &blockchain.chain {
            let Ok((next_balances, next_nonces)) =
                BlockchainValidator::calculate_next_state(&balances, &nonces, block, coinbase)
            else {
                return false;
            };
            balances = next_balances;
            nonces = next_nonces;
        }

        balances == blockchain.balances && nonces == blockchain.nonces
    }

    pub fn validate_next_block(
        chain: &[Block],
        block: &Block,
        coinbase: Address,
        max_block_weight: usize,
        mining_reward: u64,
    ) -> Result<(), BlockValidationError> {
        let last = chain
            .last()
            .ok_or(BlockValidationError::MissingChainTip)?;

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
            })
        }

        let expected_merkle_root = Block::calculate_merkle_root(&block.transactions);

        if block.header.merkle_root != expected_merkle_root {
            return Err(BlockValidationError::MerkleRootMismatch);
        }

        let expected_block_hash = Block::calculate_hash(&block.header);

        if block.hash != expected_block_hash {
            return Err(BlockValidationError::HashMismatch);
        }

        let expected_difficulty = expected_difficulty_for_next_block(chain)?;

        if block.header.difficulty != expected_difficulty {
            return Err(BlockValidationError::UnexpectedDifficulty {
                expected: expected_difficulty,
                actual: block.header.difficulty,
            });
        }

        if !expected_block_hash.has_leading_zero_hex_digits(block.header.difficulty) {
            return Err(BlockValidationError::InvalidProofOfWork);
        }

        let weight = block.weight().ok_or(BlockValidationError::WeightOverflow)?;

        if weight > max_block_weight {
            return Err(BlockValidationError::BlockTooHeavy {
                weight,
                max_weight: max_block_weight,
            });
        }

        let is_valid_coinbase_transaction = BlockchainValidator::check_coinbase_transaction(
            block,
            mining_reward,
            coinbase,
        );

        if !is_valid_coinbase_transaction {
            return Err(BlockValidationError::InvalidCoinbaseTransaction);
        }

        Ok(())
    }
}
