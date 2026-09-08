use sha2::{Digest, Sha256};
use std::{convert::TryFrom, sync::Arc};

use crate::{
    block::{
        Block, EncodingError,
        block::{BlockHeader, MerkleRoot},
    },
    codec::{ByteReader, DecodingError},
    transaction::{Transaction, decode_transaction, encode_transaction},
};

#[derive(Debug, Clone, Copy)]
pub struct BlockDecodeLimits {
    pub max_block_bytes: usize,
    pub max_transactions: u32,
    pub max_tx_bytes: usize,
}

pub fn encode_block_header(header: &BlockHeader) -> Vec<u8> {
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

pub fn encode_merkle_node(pair1: &[u8; 32], pair2: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(78);

    payload.extend_from_slice(b"MERKLE_NODE_V1");
    payload.extend_from_slice(pair1);
    payload.extend_from_slice(pair2);

    payload
}

pub fn calculate_empty_merkle_root() -> MerkleRoot {
    let mut payload = Vec::with_capacity(15);

    payload.extend_from_slice(b"MERKLE_EMPTY_V1");

    let hash: [u8; 32] = Sha256::digest(&payload).into();

    MerkleRoot(hash)
}

pub fn encode_block(block: &Block) -> Result<Vec<u8>, EncodingError> {
    let mut payload = Vec::with_capacity(116);
    let encoded_block_header = encode_block_header(&block.header);
    let len = block.transactions.len();
    let normalized_txs_count =
        u32::try_from(len).map_err(|_| EncodingError::TransactionCountOverflow { count: len })?;

    payload.extend_from_slice(b"BLOCK_V1");
    payload.extend_from_slice(&encoded_block_header);
    payload.extend_from_slice(&normalized_txs_count.to_be_bytes());

    for (index, transaction) in block.transactions.iter().enumerate() {
        let encoded_transaction = encode_transaction(transaction);
        let tx_item_len: u32 = encoded_transaction.len().try_into().map_err(|_| {
            EncodingError::TransactionLengthOverflow {
                transaction_index: index,
                length: encoded_transaction.len(),
            }
        })?;
        payload.extend_from_slice(&tx_item_len.to_be_bytes());
        payload.extend_from_slice(&encoded_transaction);
    }

    Ok(payload)
}

/* та же логика декодирования для Block, что и в CanonicalDecode, только с проверками лимитов  */
pub fn decode_block(bytes: &[u8], limits: &BlockDecodeLimits) -> Result<Block, DecodingError> {
    if bytes.len() > limits.max_block_bytes {
        return Err(DecodingError::EncodedBlockTooLarge {
            actual: bytes.len(),
            max: limits.max_block_bytes,
        });
    };

    let mut reader = ByteReader::new(bytes);

    reader.expect_tag(b"BLOCK_V1")?;

    let header = reader.read::<BlockHeader>()?;
    let hash = Block::calculate_hash(&header);
    let tx_count = reader.read::<u32>()?;

    if tx_count > limits.max_transactions {
        return Err(DecodingError::TooManyTransactions {
            actual: tx_count,
            max: limits.max_transactions,
        });
    };

    let mut transactions: Vec<Arc<Transaction>> = Vec::with_capacity(tx_count as usize);

    for index in 0..tx_count {
        let tx_len = reader.read::<u32>()? as usize;

        if tx_len > limits.max_tx_bytes {
            return Err(DecodingError::EncodedTransactionTooLarge {
                transaction_index: index as usize,
                actual: tx_len,
                max: limits.max_tx_bytes,
            });
        }
        
        let tx_bytes = reader.read_slice(tx_len)?;

        let tx = decode_transaction(tx_bytes)?;
        transactions.push(Arc::new(tx));
    }

    reader.finish()?;

    Ok(Block {
        hash,
        header,
        transactions,
    })
}
