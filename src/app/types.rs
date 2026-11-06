//! Контракт между `App` и `network::node`.
//!
//! `InboundMessage` и `NodeCommand` перенесены из `network::types` без изменения
//! полей, вариантов команд или используемого `NetworkMessage`.

use std::net::SocketAddr;

use tokio::sync::mpsc::error::SendError;

use crate::{
    address::Address, block::Block, blockchain::error::BlockAcceptanceError,
    network::wire::NetworkMessage, transaction::Transaction,
};

#[derive(Debug)]
pub struct InboundMessage {
    pub source: SocketAddr,
    pub message: NetworkMessage,
}

impl InboundMessage {
    pub fn new(source: SocketAddr, message: NetworkMessage) -> Self {
        Self { source, message }
    }
}

#[derive(Debug)]
pub enum NodeCommand {
    SendTo(SocketAddr, NetworkMessage),
    Broadcast(NetworkMessage),
    BroadcastExcept(SocketAddr, NetworkMessage),
    Disconnect(SocketAddr),
    Shutdown,
}

#[derive(Debug)]
pub enum AppError {
    BroadcastError(SendError<NodeCommand>),
    SendToChannelError(String),
}

impl From<SendError<AppCommand>> for AppError {
    fn from(value: SendError<AppCommand>) -> Self {
        Self::SendToChannelError(value.to_string())
    }
}

impl From<SendError<NodeCommand>> for AppError {
    fn from(value: SendError<NodeCommand>) -> Self {
        Self::BroadcastError(value)
    }
}

pub enum AppCommand {
    SubmitTransaction(Transaction, Option<SocketAddr>),
    SubmitBlock(Block, Option<SocketAddr>),
    MineBlock(Address),
}

#[derive(Debug)]
pub enum BlockchainServiceError {
    BlockAcceptance(BlockAcceptanceError),
    EmptyTransactions,
    MiningError,
    SendToChannelError(String),
}

impl From<BlockAcceptanceError> for BlockchainServiceError {
    fn from(value: BlockAcceptanceError) -> Self {
        Self::BlockAcceptance(value)
    }
}

impl From<SendError<(NetworkMessage, Option<SocketAddr>)>> for BlockchainServiceError {
    fn from(value: SendError<(NetworkMessage, Option<SocketAddr>)>) -> Self {
        Self::SendToChannelError(value.to_string())
    }
}
