use std::sync::Arc;

use crate::{
    address::Address,
    block::{
        Block, BlockDecodeLimits,
        block::{BlockHash, BlockHeader, MerkleRoot},
        calculate_empty_merkle_root, commitment, decode_block, encode_block, encode_block_header,
        encode_merkle_node,
    },
    transaction::Transaction,
};

fn transaction() -> Transaction {
    Transaction {
        from: Address(std::array::from_fn(|index| index as u8)),
        to: Address(std::array::from_fn(|index| (index + 20) as u8)),
        amount: 0x0102030405060708,
        fee: 0x1112131415161718,
        nonce: 0x2122232425262728,
        signature: None,
        public_key: None,
    }
}

fn header() -> BlockHeader {
    BlockHeader {
        index: 0x0102030405060708,
        previous_hash: BlockHash(std::array::from_fn(|index| index as u8)),
        timestamp: 0x1112131415161718,
        merkle_root: MerkleRoot(std::array::from_fn(|index| (index + 32) as u8)),
        difficulty: 0x1a,
        nonce: 0x2122232425262728,
    }
}

#[test]
fn block_header_commitment_matches_golden_bytes_and_hash() {
    let header = header();
    let expected = hex::decode(concat!(
        "424c4f434b5f4845414445525f5631",
        "0102030405060708",
        "000102030405060708090a0b0c0d0e0f",
        "101112131415161718191a1b1c1d1e1f",
        "1112131415161718",
        "202122232425262728292a2b2c2d2e2f",
        "303132333435363738393a3b3c3d3e3f",
        "1a",
        "2122232425262728",
    ))
    .unwrap();
    let expected_hash =
        hex::decode("a2cae518d46a8231a1df11c83a2d0adbc93b76967e0d1b51f507dd59a97e9415").unwrap();

    assert_eq!(encode_block_header(&header), expected);
    assert_eq!(commitment::encode_block_header(&header), expected);
    assert_eq!(Block::calculate_hash(&header).0.as_slice(), expected_hash);
}

#[test]
fn merkle_commitments_match_golden_bytes_and_hashes() {
    let left = std::array::from_fn(|index| index as u8);
    let right = std::array::from_fn(|index| (index + 32) as u8);
    let expected_node = hex::decode(concat!(
        "4d45524b4c455f4e4f44455f5631",
        "000102030405060708090a0b0c0d0e0f",
        "101112131415161718191a1b1c1d1e1f",
        "202122232425262728292a2b2c2d2e2f",
        "303132333435363738393a3b3c3d3e3f",
    ))
    .unwrap();
    let expected_empty_root =
        hex::decode("e2d523cbd61a7de7f1aece09a5919a463d275bdb3f6c48f65c524a9daba15733").unwrap();

    assert_eq!(encode_merkle_node(&left, &right), expected_node);
    assert_eq!(commitment::encode_merkle_node(&left, &right), expected_node);
    assert_eq!(
        calculate_empty_merkle_root().0.as_slice(),
        expected_empty_root
    );
    assert_eq!(
        Block::calculate_merkle_root(&[]).0.as_slice(),
        expected_empty_root
    );
}

#[test]
fn block_codec_matches_golden_bytes() {
    let header = header();
    let block = Block {
        hash: Block::calculate_hash(&header),
        header,
        transactions: vec![Arc::new(transaction())],
    };
    let expected = hex::decode(concat!(
        "424c4f434b5f5631",
        "424c4f434b5f4845414445525f5631",
        "0102030405060708",
        "000102030405060708090a0b0c0d0e0f",
        "101112131415161718191a1b1c1d1e1f",
        "1112131415161718",
        "202122232425262728292a2b2c2d2e2f",
        "303132333435363738393a3b3c3d3e3f",
        "1a",
        "2122232425262728",
        "00000001",
        "0000004a",
        "54585f49445f5631",
        "000102030405060708090a0b0c0d0e0f10111213",
        "1415161718191a1b1c1d1e1f2021222324252627",
        "0102030405060708",
        "1112131415161718",
        "2122232425262728",
        "00",
        "00",
    ))
    .unwrap();
    let limits = BlockDecodeLimits {
        max_block_bytes: expected.len(),
        max_transactions: 1,
        max_tx_bytes: 74,
    };

    assert_eq!(encode_block(&block).unwrap(), expected);

    let decoded = decode_block(&expected, &limits).unwrap();
    assert_eq!(encode_block(&decoded).unwrap(), expected);
    assert_eq!(decoded.hash, block.hash);
}
