use num_bigint::BigUint;

use crate::{
    block::Block, blockchain::{error::BlockValidationError, utils::{Difficulty, MAX_DIFFICULTY}},
};

pub const TARGET_BLOCK_TIME: u64 = 10; // Время на создание одного блока в секундах
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10; // интервал блоков, по которому происходит обновление значения difficulty (через каждые N блоков происходит перерасчет difficulty)

pub fn expected_difficulty_for_next_block(chain: &[Block]) -> Result<Difficulty, BlockValidationError> {
    let last_block = chain.last().ok_or(BlockValidationError::MissingChainTip)?;

    let next_block_index = last_block
        .header
        .index
        .checked_add(1)
        .ok_or(BlockValidationError::IndexOverflow)?;

    if next_block_index % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 {
        return Ok(last_block.header.difficulty);
    };

    let start_position = chain
        .len()
        .checked_sub(DIFFICULTY_ADJUSTMENT_INTERVAL as usize)
        .ok_or(BlockValidationError::InsufficientDifficultyHistory {
            required: DIFFICULTY_ADJUSTMENT_INTERVAL as usize,
            actual: chain.len(),
        })?;

    let start =
        chain
            .get(start_position)
            .ok_or(BlockValidationError::InsufficientDifficultyHistory {
                required: DIFFICULTY_ADJUSTMENT_INTERVAL as usize,
                actual: chain.len(),
            })?;

    let actual_time = last_block
        .header
        .timestamp
        .checked_sub(start.header.timestamp)
        .ok_or(BlockValidationError::TimestampBeforePrevious {
            previous: start.header.timestamp,
            actual: last_block.header.timestamp,
        })?;

    let expected_time = TARGET_BLOCK_TIME * (DIFFICULTY_ADJUSTMENT_INTERVAL - 1);

    if actual_time < expected_time / 2 {
        return Ok(last_block
            .header
            .difficulty
            .saturating_add(1)
            .min(MAX_DIFFICULTY));
    }

    if actual_time > expected_time * 2 {
        return Ok(last_block.header.difficulty.saturating_sub(1));
    }

    Ok(last_block.header.difficulty)
}



pub fn block_work(difficulty: Difficulty) -> BigUint {
    BigUint::from(16u32).pow(difficulty as u32)
}

pub fn chain_work(chain: &[Block]) -> BigUint {
    chain.iter().fold(BigUint::from(0u32), |acc, block| {
        acc + block_work(block.header.difficulty)
    })
}