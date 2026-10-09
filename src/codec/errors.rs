#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodingError {
    UnexpectedEof {
        position: usize,
        needed: usize,
        remaining: usize,
    },
    InvalidDomainTag {
        position: usize,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    TrailingBytes {
        position: usize,
        remaining: usize,
    },
    InvalidOptionTag {
        position: usize,
        actual: u8,
    },
    InvalidPublicKey {
        position: usize,
    },
    EncodedBlockTooLarge {
        actual: usize,
        max: usize,
    },
    TooManyTransactions {
        actual: u32,
        max: u32,
    },
    EncodedTransactionTooLarge {
        transaction_index: usize,
        actual: usize,
        max: usize,
    },
    UnknownMessageType {
        actual: u8,
    }
}