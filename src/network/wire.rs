//! Wire-типы сетевых сообщений, перенесённые из `network::types`.
//!
//! Варианты `NetworkMessage`, значения `MessageType` и формат вывода сохранены.

use std::fmt::Display;

use crate::{block::Block, network::error::NetworkDecodingError, transaction::Transaction};

use super::protocol::Handshake;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageType {
    Transaction = 1,
    Block = 2,
    Handshake = 3,
}

impl TryFrom<u8> for MessageType {
    type Error = NetworkDecodingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Transaction),
            2 => Ok(Self::Block),
            3 => Ok(Self::Handshake),
            actual => Err(NetworkDecodingError::DecodeMessageType { actual }),
        }
    }
}

impl From<MessageType> for u8 {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::Transaction => 1,
            MessageType::Block => 2,
            MessageType::Handshake => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkMessage {
    Transaction(Transaction),
    Block(Block),
    Handshake(Handshake),
}

impl Display for NetworkMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Block(block) => write!(f, "{:?}", block),
            Self::Transaction(transaction) => write!(f, "{:?}", transaction),
            Self::Handshake(handshake) => write!(f, "{:?}", handshake),
        }
    }
}
