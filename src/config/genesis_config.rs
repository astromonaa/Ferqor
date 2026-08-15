use crate::address::{Balances};
use crate::block::Block;
use chrono::Utc;
use std::collections::HashMap;
use crate::block::block::BlockHash;

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

    pub fn initial_balances(balances: Option<Balances>) -> Balances {
        let extended_balances = match balances { 
            Some(balances) => balances,
            _ => HashMap::new()
        };
        
        let mut balances: Balances = HashMap::new();

        balances.extend(extended_balances);

        balances
    }

    pub fn get_config(balances: Option<Balances>, max_block_weight: Option<usize>) -> GenesisData {
        let now = GenesisConfig::get_now_timestamp();
        let genesis_block = Block::new(0, vec![], BlockHash([0u8; 32]), 0, now);
        let initial_balances = GenesisConfig::initial_balances(balances);

        GenesisData {
            genesis_block,
            initial_balances,
            max_block_weight: max_block_weight.unwrap_or(MAX_BLOCK_WEIGHT),
        }
    }
}
