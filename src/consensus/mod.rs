//! Неизменяемая конфигурация консенсуса и правила расчёта сложности.
//!
//! Функции сложности перенесены из `blockchain::difficulty`, а значения параметров
//! заменяют константы и поля, ранее распределённые между
//! `blockchain::Blockchain` и `config::GenesisConfig`.

mod difficulty;
mod params;

pub use difficulty::{
    DIFFICULTY_ADJUSTMENT_INTERVAL, TARGET_BLOCK_TIME, block_work, chain_work,
    expected_difficulty_for_next_block,
};
pub use params::{
    ConsensusParams, DEFAULT_DIFFICULTY_ADJUSTMENT_INTERVAL, DEFAULT_TARGET_BLOCK_TIME, Difficulty,
    MAX_DIFFICULTY,
};

#[cfg(test)]
mod params_tests;
