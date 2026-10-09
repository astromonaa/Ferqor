use crate::address::Balances;
use crate::block::Block;
use crate::block::block::BlockHash;
use crate::blockchain::error::BlockchainError;
use crate::blockchain::utils::{Difficulty, MAX_DIFFICULTY};
use chrono::{DateTime, TimeZone, Utc};
use std::collections::HashMap;

pub struct GenesisConfig {}

pub struct GenesisData {
    pub genesis_block: Block,
    pub initial_balances: Balances,
    pub max_block_weight: usize,
}

const MAX_BLOCK_WEIGHT: usize = 1000;

impl GenesisConfig {
    pub fn get_now_timestamp() -> u64 {
        Utc::now().timestamp() as u64
    }

    fn get_genesis_timestamp() -> Result<u64, BlockchainError> {
        match Utc.with_ymd_and_hms(2026, 01, 01, 12, 0, 0) {
            chrono::offset::LocalResult::Single(value) => Ok(value.timestamp() as u64),
            chrono::offset::LocalResult::Ambiguous(_, value) => Ok(value.timestamp() as u64),
            chrono::offset::LocalResult::None => Err(BlockchainError::GenesisDateTime),
        }
    }

    pub fn initial_balances(balances: Option<Balances>) -> Balances {
        let extended_balances = match balances {
            Some(balances) => balances,
            _ => HashMap::new(),
        };

        let mut balances: Balances = HashMap::new();

        balances.extend(extended_balances);

        balances
    }

    pub fn get_config(
        balances: Option<Balances>,
        max_block_weight: Option<usize>,
        difficulty: Difficulty,
    ) -> Result<GenesisData, BlockchainError> {
        if difficulty > MAX_DIFFICULTY {
            return Err(BlockchainError::InvalidDifficulty);
        }

        let datetime = GenesisConfig::get_genesis_timestamp()?;
        let genesis_block = Block::new(0, vec![], BlockHash([0u8; 32]), 0, datetime, difficulty);
        let initial_balances = GenesisConfig::initial_balances(balances);

        Ok(GenesisData {
            genesis_block,
            initial_balances,
            max_block_weight: max_block_weight.unwrap_or(MAX_BLOCK_WEIGHT),
        })
    }
}
