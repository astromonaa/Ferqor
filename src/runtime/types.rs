use crate::{app::types::{AppError, BlockchainServiceError}, blockchain::error::BlockchainError, network::error::PeerError};

#[derive(Debug)]
pub enum RuntimeError {
    Blockchain(BlockchainError),
    Node(PeerError),
    App(AppError),
    BlockchainService(BlockchainServiceError)
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


impl From<BlockchainServiceError> for RuntimeError {
    fn from(value: BlockchainServiceError) -> Self {
        Self::BlockchainService(value)
    }
}
