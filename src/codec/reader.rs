use super::errors::DecodingError;

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
