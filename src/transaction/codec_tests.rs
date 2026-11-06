use ed25519_dalek::{Signature, VerifyingKey};

use crate::{
    address::Address,
    transaction::{
        Transaction, commitment, decode_transaction, encode_transaction,
        encode_transaction_signing_payload, hash_transaction_signing_payload,
    },
};

fn transaction(signature: Option<Signature>, public_key: Option<VerifyingKey>) -> Transaction {
    Transaction {
        from: Address(std::array::from_fn(|index| index as u8)),
        to: Address(std::array::from_fn(|index| (index + 20) as u8)),
        amount: 0x0102030405060708,
        fee: 0x1112131415161718,
        nonce: 0x2122232425262728,
        signature,
        public_key,
    }
}

#[test]
fn signing_commitment_matches_golden_bytes_and_hash() {
    let transaction = transaction(None, None);
    let expected = hex::decode(concat!(
        "54585f5349474e494e475f5631",
        "000102030405060708090a0b0c0d0e0f10111213",
        "1415161718191a1b1c1d1e1f2021222324252627",
        "0102030405060708",
        "1112131415161718",
        "2122232425262728",
    ))
    .unwrap();
    let expected_hash =
        hex::decode("444edc82774720fe333e383a345a8d2db6334c2721aeaabae18e59d6d7fcf115").unwrap();

    assert_eq!(
        encode_transaction_signing_payload(
            &transaction.from,
            &transaction.to,
            transaction.amount,
            transaction.fee,
            transaction.nonce,
        ),
        expected,
    );
    assert_eq!(
        commitment::encode_transaction_signing_payload(
            &transaction.from,
            &transaction.to,
            transaction.amount,
            transaction.fee,
            transaction.nonce,
        ),
        expected,
    );
    assert_eq!(
        hash_transaction_signing_payload(
            &transaction.from,
            &transaction.to,
            transaction.amount,
            transaction.fee,
            transaction.nonce,
        )
        .as_slice(),
        expected_hash,
    );
}

#[test]
fn unsigned_transaction_matches_golden_bytes_and_txid() {
    let transaction = transaction(None, None);
    let expected = hex::decode(concat!(
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
    let expected_txid =
        hex::decode("4a0d4561ba9ff3192cd450cd0ea26aae410c67d5f08d3629a7bd8e995770d886").unwrap();

    assert_eq!(encode_transaction(&transaction), expected);
    assert_eq!(commitment::encode_transaction(&transaction), expected);
    assert_eq!(transaction.txid().as_bytes().as_slice(), expected_txid);

    let decoded = decode_transaction(&expected).unwrap();
    assert_eq!(encode_transaction(&decoded), expected);
}

#[test]
fn signed_transaction_matches_golden_bytes_and_txid() {
    let signature = Signature::from_bytes(&std::array::from_fn(|index| index as u8));
    let public_key = VerifyingKey::from_bytes(
        &hex::decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
            .unwrap()
            .try_into()
            .unwrap(),
    )
    .unwrap();
    let transaction = transaction(Some(signature), Some(public_key));
    let expected = hex::decode(concat!(
        "54585f49445f5631",
        "000102030405060708090a0b0c0d0e0f10111213",
        "1415161718191a1b1c1d1e1f2021222324252627",
        "0102030405060708",
        "1112131415161718",
        "2122232425262728",
        "01",
        "000102030405060708090a0b0c0d0e0f",
        "101112131415161718191a1b1c1d1e1f",
        "202122232425262728292a2b2c2d2e2f",
        "303132333435363738393a3b3c3d3e3f",
        "01",
        "d75a980182b10ab7d54bfed3c964073a",
        "0ee172f3daa62325af021a68f707511a",
    ))
    .unwrap();
    let expected_txid =
        hex::decode("3a70c62b330675274b6f69665f7cc9db18c8aa5d93cd41e1688dc1c17f3666c0").unwrap();

    assert_eq!(encode_transaction(&transaction), expected);
    assert_eq!(transaction.txid().as_bytes().as_slice(), expected_txid);

    let decoded = decode_transaction(&expected).unwrap();
    assert_eq!(encode_transaction(&decoded), expected);
}
