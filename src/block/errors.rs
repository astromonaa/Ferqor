#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodingError {
    TransactionCountOverflow {
        count: usize,
    },
    TransactionLengthOverflow {
        transaction_index: usize,
        length: usize,
    },
}