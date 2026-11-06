//! Создание блоков, выделенное из `blockchain::blockchain::Blockchain`.
//!
//! Имена методов соответствуют исходным операциям: `create_block` и
//! `get_reward_transaction` сохраняют семантику прежних приватных методов
//! `Blockchain`. Исходный `Block::mine_block` вызывается напрямую владельцем блока.

use std::sync::Arc;

use crate::address::{Address, COINBASE};
use crate::block::Block;
use crate::blockchain::error::BlockValidationError;
use crate::chain::Chain;
use crate::config::GenesisConfig;
use crate::consensus::{ConsensusParams, expected_difficulty_for_next_block};
use crate::transaction::Transaction;

/// Создаёт блоки-кандидаты и наградные транзакции логикой из прежнего
/// `blockchain::blockchain::Blockchain`.
///
/// Эта ответственность перенесена из методов `blockchain::blockchain::Blockchain`:
/// `create_block` и `get_reward_transaction`.
#[derive(Debug, Default, Clone, Copy)]
pub struct BlockProducer;

impl BlockProducer {
    /// Создаёт недобытый блок точно так же, как прежний метод
    /// `blockchain::blockchain::Blockchain::create_block`.
    pub fn create_block(
        &self,
        chain: &Chain,
        transactions: Vec<Arc<Transaction>>,
    ) -> Result<Block, BlockValidationError> {
        let previous_block = chain.tip().ok_or(BlockValidationError::MissingChainTip)?;
        let index = previous_block
            .header
            .index
            .checked_add(1)
            .ok_or(BlockValidationError::IndexOverflow)?;
        // В прежнем `Blockchain::create_block` время фиксировалось до расчёта сложности.
        let now = GenesisConfig::get_now_timestamp();
        let difficulty =
            expected_difficulty_for_next_block(chain.blocks(), chain.consensus_params())?;

        Ok(Block::new(
            index,
            transactions,
            previous_block.hash,
            0,
            now,
            difficulty,
        ))
    }

    /// Создаёт транзакцию награды, ранее возвращавшуюся методом
    /// `blockchain::blockchain::Blockchain::get_reward_transaction`.
    pub fn get_reward_transaction(
        &self,
        params: &ConsensusParams,
        miner_address: Address,
        total_fees: u64,
    ) -> Option<Arc<Transaction>> {
        let amount = params.mining_reward().checked_add(total_fees)?;

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
}
