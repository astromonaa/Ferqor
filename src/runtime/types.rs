use crate::{app::types::AppError, blockchain::error::BlockchainError, network::types::PeerError};

pub enum RuntimeError {
    Blockchain(BlockchainError),
    Node(PeerError),
    App(AppError)
}


impl From<BlockchainError> for RuntimeError {
    fn from(value: BlockchainError) -> Self {
        RuntimeError::Blockchain(value)
    }
}

impl From<PeerError> for RuntimeError {
    fn from(value: PeerError) -> Self {
        Self::Node(value)
    }
}

impl From<AppError> for RuntimeError {
    fn from(value: AppError) -> Self {
        Self::App(value)
    }
}