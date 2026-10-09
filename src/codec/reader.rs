use super::errors::DecodingError;
use crate::{
    address::Address,
    block::block::{BlockHash, BlockHeader, MerkleRoot},
    transaction::Transaction,
};
use ed25519_dalek::{Signature, VerifyingKey};

pub struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    pub fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DecodingError> {
        let remaining = self.remaining();
        if remaining < N {
            return Err(DecodingError::UnexpectedEof {
                needed: N,
                remaining,
                position: self.position,
            });
        }

        let mut result: [u8; N] = [0u8; N];
        let end = self.position + N;

        result.copy_from_slice(&self.bytes[self.position..end]);
        self.position = end;

        Ok(result)
    }

    pub fn read_slice(&mut self, length: usize) -> Result<&'a [u8], DecodingError> {
        let remaining = self.remaining();
        if remaining < length {
            return Err(DecodingError::UnexpectedEof {
                needed: length,
                remaining,
                position: self.position,
            });
        }

        let end = self.position + length;

        let result = &self.bytes[self.position..end];

        self.position = end;

        Ok(result)
    }

    pub fn read<T: CanonicalDecode>(&mut self) -> Result<T, DecodingError> {
        T::decode(self)
    }

    pub fn expect_tag<const N: usize>(&mut self, expected: &[u8; N]) -> Result<(), DecodingError> {
        let position = self.position;
        let bytes = self.read_exact::<N>()?;
        if bytes != *expected {
            return Err(DecodingError::InvalidDomainTag {
                position,
                expected: expected.to_vec(),
                actual: bytes.to_vec(),
            });
        }
        Ok(())
    }

    pub fn finish(self) -> Result<(), DecodingError> {
        if self.remaining() > 0 {
            return Err(DecodingError::TrailingBytes {
                position: self.position,
                remaining: self.remaining(),
            });
        }

        Ok(())
    }
}

pub trait CanonicalDecode: Sized {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError>;
}

macro_rules! impl_canonical_decode {
    ($type:ty, $size:literal) => {
        impl CanonicalDecode for $type {
            fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
                let bytes = reader.read_exact::<$size>()?;
                Ok(Self::from_be_bytes(bytes))
            }
        }
    };
}

impl_canonical_decode!(u8, 1);
impl_canonical_decode!(u16, 2);
impl_canonical_decode!(u32, 4);
impl_canonical_decode!(u64, 8);

impl<T: CanonicalDecode> CanonicalDecode for Option<T> {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let position = reader.position();
        let marker = reader.read::<u8>()?;

        match marker {
            0 => Ok(None),
            1 => {
                let value = reader.read::<T>()?;
                Ok(Some(value))
            }
            _ => Err(DecodingError::InvalidOptionTag {
                position,
                actual: marker,
            }),
        }
    }
}

impl CanonicalDecode for Address {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<20>()?;
        Ok(Address(bytes))
    }
}

impl CanonicalDecode for Signature {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<64>()?;
        Ok(Signature::from_bytes(&bytes))
    }
}

impl CanonicalDecode for VerifyingKey {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let position = reader.position();
        let bytes = reader.read_exact::<32>()?;

        let key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| DecodingError::InvalidPublicKey { position })?;

        Ok(key)
    }
}

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

impl CanonicalDecode for BlockHash {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<32>()?;
        Ok(BlockHash(bytes))
    }
}

impl CanonicalDecode for MerkleRoot {
    fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
        let bytes = reader.read_exact::<32>()?;
        Ok(MerkleRoot(bytes))
    }
}

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

// impl CanonicalDecode for Block {

//     fn decode(reader: &mut ByteReader<'_>) -> Result<Self, DecodingError> {
//         reader.expect_tag(b"BLOCK_V1")?;
//         let header = reader.read::<BlockHeader>()?;
//         let hash = Self::calculate_hash(&header);
//         let tx_count = reader.read::<u32>()?;
//         let mut transactions: Vec<Arc<Transaction>> = Vec::new();

//         for _ in 0..tx_count {
//             let tx_len = reader.read::<u32>()?;
//             let tx_bytes = reader.read_slice(tx_len as usize)?;

//             let tx = decode_transaction(tx_bytes)?;
//             transactions.push(Arc::new(tx));
//         }

//         Ok(Block {
//             hash,
//             header,
//             transactions
//         })
//     }
// }

