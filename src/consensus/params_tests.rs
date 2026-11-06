//! Тесты значений консенсуса, перенесённых из `Blockchain`, конфигурации генезис-блока и констант сложности.

use super::{ConsensusParams, MAX_DIFFICULTY};

#[test]
fn default_params_preserve_existing_consensus_values() {
    let params = ConsensusParams::default();

    assert_eq!(params.mining_reward(), 50);
    assert_eq!(params.max_block_weight(), 1000);
    assert_eq!(params.target_block_time(), 10);
    assert_eq!(params.difficulty_adjustment_interval(), 10);
    assert_eq!(params.max_difficulty(), MAX_DIFFICULTY);
}

#[test]
fn max_block_weight_preserves_genesis_configuration() {
    let params = ConsensusParams::default().with_max_block_weight(2_000);

    assert_eq!(params.mining_reward(), 50);
    assert_eq!(params.max_block_weight(), 2_000);
    assert_eq!(params.target_block_time(), 10);
    assert_eq!(params.difficulty_adjustment_interval(), 10);
    assert_eq!(params.max_difficulty(), MAX_DIFFICULTY);
}
