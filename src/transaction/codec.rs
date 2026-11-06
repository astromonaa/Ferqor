//! Wire-кодек транзакций и domain-декодеры.
//!
//! Реализации `CanonicalDecode` перенесены из `codec::reader`, а публичные функции
//! кодирования сохранены на прежнем пути `transaction::codec`.

use ed25519_dalek::{Signature, VerifyingKey};

use crate::{
    address::Address,
    codec::{ByteReader, CanonicalDecode, errors::DecodingError},
    transaction::{Transaction, commitment},
};

pub fn encode_transaction_signing_payload(
    from: &Address,
    to: &Address,
    amount: u64,
    fee: u64,
    nonce: u64,
) -> Vec<u8> {
    commitment::encode_transaction_signing_payload(from, to, amount, fee, nonce)
}

pub fn hash_transaction_signing_payload(
    from: &Address,
    to: &Address,
    amount: u64,
    fee: u64,
    nonce: u64,
) -> [u8; 32] {
    commitment::hash_transaction_signing_payload(from, to, amount, fee, nonce)
}

pub fn encode_transaction(transaction: &Transaction) -> Vec<u8> {
    commitment::encode_transaction(transaction)
}

pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, DecodingError> {
    let mut reader = ByteReader::new(bytes);
    let transaction = reader.read::<Transaction>()?;
    reader.finish()?;
    Ok(transaction)
}

/// Декодирование перенесено из прежнего `codec::reader` без изменения формата.
impl CanonicalDecode for Address {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<20>()?;
        Ok(Address(bytes))
    }
}

/// Декодирование перенесено из прежнего `codec::reader` без изменения формата.
impl CanonicalDecode for Signature {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<64>()?;
        Ok(Signature::from_bytes(&bytes))
    }
}

/// Декодирование перенесено из прежнего `codec::reader` с сохранением позиции ошибки.
impl CanonicalDecode for VerifyingKey {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let position = reader.position();
        let bytes = reader.read_exact::<32>()?;

        let key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| DecodingError::InvalidPublicKey { position })?;

        Ok(key)
    }
}

/// Декодирование перенесено из прежнего `codec::reader` без изменения порядка полей.
impl CanonicalDecode for Transaction {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        reader.expect_tag::<8>(b"TX_ID_V1")?;

        let from = reader.read::<Address>()?;
        let to = reader.read::<Address>()?;
        let amount = reader.read::<u64>()?;
        let fee = reader.read::<u64>()?;
        let nonce = reader.read::<u64>()?;

        let signature = reader.read::<Option<Signature>>()?;
        let public_key = reader.read::<Option<VerifyingKey>>()?;

        Ok(Transaction {
            from,
            to,
            amount,
            signature,
            public_key,
            fee,
            nonce,
        })
    }
}
