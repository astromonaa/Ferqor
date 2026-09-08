use crate::codec::ByteReader;
use crate::{address::Address, transaction::Transaction};
use crate::codec::errors::{ DecodingError };

use sha2::{ Digest, Sha256 };

pub fn encode_transaction_signing_payload(
    from: &Address,
    to: &Address,
    amount: u64,
    fee: u64,
    nonce: u64,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(77);

    payload.extend_from_slice(b"TX_SIGNING_V1");

    payload.extend_from_slice(&from.0);
    payload.extend_from_slice(&to.0);
    payload.extend_from_slice(&amount.to_be_bytes());
    payload.extend_from_slice(&fee.to_be_bytes());
    payload.extend_from_slice(&nonce.to_be_bytes());

    payload
}


pub fn hash_transaction_signing_payload(
    from: &Address,
    to: &Address,
    amount: u64,
    fee: u64,
    nonce: u64,
) -> [u8; 32] {
    let payload = encode_transaction_signing_payload(from, to, amount, fee, nonce);
    Sha256::digest(payload).into()
}


pub fn encode_transaction(transaction: &Transaction) -> Vec<u8> {

    let mut payload = Vec::with_capacity(170);

    payload.extend_from_slice(b"TX_ID_V1");

    payload.extend_from_slice(&transaction.from.0);
    payload.extend_from_slice(&transaction.to.0);
    payload.extend_from_slice(&transaction.amount.to_be_bytes());
    payload.extend_from_slice(&transaction.fee.to_be_bytes());
    payload.extend_from_slice(&transaction.nonce.to_be_bytes());

    match transaction.signature.as_ref() {
        Some(sign) => {
            payload.push(1);
            payload.extend_from_slice(&sign.to_bytes());
        },
        None => payload.push(0),
    }

    match transaction.public_key.as_ref() {
        Some(pk) => {
            payload.push(1);
            payload.extend_from_slice(&pk.to_bytes());
        },
        None => payload.push(0),
    }

    payload
 }

 pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, DecodingError> {
     let mut reader = ByteReader::new(bytes);
     let transaction = reader.read::<Transaction>()?;
     reader.finish()?;
     Ok(transaction)
 }