//! Consensus-байты блока, выделенные из прежних `block::codec` и `block::Block`.
//!
//! Реализация сохраняет исходные теги, Merkle-алгоритм и SHA-256 preimage без изменений.

use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::{
    block::block::{BlockHash, BlockHeader, MerkleRoot},
    transaction::Transaction,
};

/// Кодирует заголовок логикой прежней `block::codec::encode_block_header`.
pub(super) fn encode_block_header(header: &BlockHeader) -> Vec<u8> {
    let mut payload = Vec::with_capacity(104);

    payload.extend_from_slice(b"BLOCK_HEADER_V1");
    payload.extend_from_slice(&header.index.to_be_bytes());
    payload.extend_from_slice(&header.previous_hash.0);
    payload.extend_from_slice(&header.timestamp.to_be_bytes());
    payload.extend_from_slice(&header.merkle_root.0);
    payload.extend_from_slice(&header.difficulty.to_be_bytes());
    payload.extend_from_slice(&header.nonce.to_be_bytes());

    payload
}

/// Кодирует узел дерева логикой прежней `block::codec::encode_merkle_node`.
pub(super) fn encode_merkle_node(pair1: &[u8; 32], pair2: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(78);

    payload.extend_from_slice(b"MERKLE_NODE_V1");
    payload.extend_from_slice(pair1);
    payload.extend_from_slice(pair2);

    payload
}

/// Вычисляет пустой корень логикой прежней
/// `block::codec::calculate_empty_merkle_root`.
pub(super) fn calculate_empty_merkle_root() -> MerkleRoot {
    let mut payload = Vec::with_capacity(15);

    payload.extend_from_slice(b"MERKLE_EMPTY_V1");

    let hash: [u8; 32] = Sha256::digest(&payload).into();

    MerkleRoot(hash)
}

/// Вычисляет дерево логикой прежнего `Block::calculate_merkle_root`.
pub(super) fn calculate_merkle_root(transactions: &[Arc<Transaction>]) -> MerkleRoot {
    if transactions.is_empty() {
        return calculate_empty_merkle_root();
    };

    let mut hashes: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.txid().0).collect();

    while hashes.len() > 1 {
        if hashes.len() % 2 != 0 {
            let last = *hashes.last().unwrap();
            hashes.push(last);
        }

        hashes = hashes
            .chunks(2)
            .map(|pair| {
                let payload = encode_merkle_node(&pair[0], &pair[1]);
                Sha256::digest(payload).into()
            })
            .collect();
    }

    MerkleRoot(hashes[0])
}

/// Вычисляет hash логикой прежнего `Block::calculate_hash`.
pub(super) fn calculate_block_hash(header: &BlockHeader) -> BlockHash {
    let payload = encode_block_header(header);
    let hash: [u8; 32] = Sha256::digest(payload).into();

    BlockHash(hash)
}
