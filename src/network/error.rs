//! Ошибки сетевого слоя.
//!
//! `PeerError` перенесён из `network::types`; его варианты и вложенные типы
//! сохранены без изменений.

use std::{io::Error, net::SocketAddr};

use tokio::sync::mpsc::error::{SendError, TrySendError};

use crate::{
    app::types::InboundMessage,
    block::EncodingError,
    codec::DecodingError,
    network::{
        peer::types::PeerEvent,
        protocol::HandshakeError,
        wire::NetworkMessage,
    },
};

#[derive(Debug)]
pub enum NetworkEncodingError {
    BlockError(EncodingError),
    PayloadLengthOverflow { actual: usize },
    FrameLengthOverflow { actual: usize },
    TypeOverflow,
    IpEncodingError,
}

#[derive(Debug)]
pub enum NetworkDecodingError {
    ReadMessageType(DecodingError),
    DecodeMessageType { actual: u8 },
    ReadPayloadLen(DecodingError),
    ReadPayload(DecodingError),
    TrailingBytes(DecodingError),
    BlockDecode(DecodingError),
    TransactionDecode(DecodingError),
    InvalidTag(DecodingError),
    PayloadTooLarge { actual: usize, max: usize },
    SocketAddrDecode(DecodingError),
    InvalidIPVersion(u8),
    HandshakeDecode(DecodingError),
}

#[derive(Debug)]
pub enum TransportError {
    ReadBytesError(Error),
    WriteBytesError(Error),
    InvalidFrame,
    DecodingError(NetworkDecodingError),
    InvalidTag,
    UnknownMessageType,
    InvalidPayloadSize,
}

#[derive(Debug)]
pub enum PeerError {
    ConnectionError(Error),
    TransportError(TransportError),
    NetworkDecodingError(NetworkDecodingError),
    NetworkEncodingError(NetworkEncodingError),
    SendToNodeChannelError(SendError<PeerEvent>),
    SendToStreamChannelError(SendError<NetworkMessage>),
    SendToAppChannelError(TrySendError<InboundMessage>),
    EventChannelClosed,
    AppChannelClosed,
    ShutDown,
    HandShake(HandshakeError),
    ConnectionIdExhausted,
    ConnectionTimeout(SocketAddr),
}

impl From<TransportError> for PeerError {
    fn from(value: TransportError) -> Self {
        PeerError::TransportError(value)
    }
}

impl From<NetworkDecodingError> for PeerError {
    fn from(value: NetworkDecodingError) -> Self {
        PeerError::NetworkDecodingError(value)
    }
}

impl From<NetworkEncodingError> for PeerError {
    fn from(value: NetworkEncodingError) -> Self {
        PeerError::NetworkEncodingError(value)
    }
}

impl From<HandshakeError> for PeerError {
    fn from(value: HandshakeError) -> Self {
        PeerError::HandShake(value)
    }
}
