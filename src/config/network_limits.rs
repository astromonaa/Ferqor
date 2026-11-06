use crate::{block::BlockDecodeLimits, network::codec::NetworkDecodeLimits};

pub fn get_network_decode_limits() -> NetworkDecodeLimits {
    NetworkDecodeLimits {
        max_tx_bytes: 170,
        block_limits: BlockDecodeLimits {
            max_block_bytes: 174_116,
            max_transactions: 1_000,
            max_tx_bytes: 170,
        },
    }
}
