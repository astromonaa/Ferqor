//! Wire-кодек блоков и domain-декодеры.
//!
//! Реализации `CanonicalDecode` перенесены из `codec::reader`, а публичные функции
//! кодирования сохранены на прежнем пути `block::codec`.

use std::{convert::TryFrom, sync::Arc};

use crate::{
    block::{
        Block, EncodingError,
        block::{BlockHash, BlockHeader, MerkleRoot},
        commitment,
    },
    codec::{ByteReader, CanonicalDecode, DecodingError},
    transaction::{Transaction, decode_transaction, encode_transaction},
};

#[derive(Debug, Clone, Copy)]
pub struct BlockDecodeLimits {
    pub max_block_bytes: usize,
    pub max_transactions: u32,
    pub max_tx_bytes: usize,
}

pub fn encode_block_header(header: &BlockHeader) -> Vec<u8> {
    commitment::encode_block_header(header)
}

pub fn encode_merkle_node(pair1: &[u8; 32], pair2: &[u8; 32]) -> Vec<u8> {
    commitment::encode_merkle_node(pair1, pair2)
}

pub fn calculate_empty_merkle_root() -> MerkleRoot {
    commitment::calculate_empty_merkle_root()
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

/// Декодирование перенесено из прежнего `codec::reader` без изменения формата.
impl CanonicalDecode for BlockHash {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<32>()?;
        Ok(BlockHash(bytes))
    }
}

/// Декодирование перенесено из прежнего `codec::reader` без изменения формата.
impl CanonicalDecode for MerkleRoot {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<32>()?;
        Ok(MerkleRoot(bytes))
    }
}

/// Декодирование перенесено из прежнего `codec::reader` без изменения порядка полей.
impl CanonicalDecode for BlockHeader {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        reader.expect_tag(b"BLOCK_HEADER_V1")?;
        let index = reader.read::<u64>()?;
        let previous_hash = reader.read::<BlockHash>()?;
        let timestamp = reader.read::<u64>()?;
        let merkle_root = reader.read::<MerkleRoot>()?;
        let difficulty = reader.read::<u8>()?;
        let nonce = reader.read::<u64>()?;

        Ok(BlockHeader {
            index,
            previous_hash,
            nonce,
            timestamp,
            merkle_root,
            difficulty,
        })
    }
}
