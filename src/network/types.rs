use std::{fmt::Display, io::Error, net::SocketAddr};

use tokio::{
    sync::mpsc::{Sender, error::SendError},
    task::JoinHandle,
};

use crate::{
    block::Block, network::{error::{NetworkDecodingError, NetworkEncodingError, TransportError}, node::types::ConnectionId}, transaction::Transaction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId(pub [u8; 32]);

impl NodeId {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl NetworkId {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Handshake {
    pub protocol_version: u8,
    pub network_id: NetworkId,
    pub node_id: NodeId,
    pub listen_addr: SocketAddr,
}

impl Handshake {
    pub fn new(
        protocol_version: u8,
        network_id: NetworkId,
        node_id: NodeId,
        listen_addr: SocketAddr,
    ) -> Self {
        Self {
            protocol_version,
            network_id,
            node_id,
            listen_addr,
        }
    }
}

#[derive(Debug)]
pub enum HandshakeError {
    ProtocolVersionMismatch(u8),
    NetworkIdMismatch(NetworkId),
    SelfConnection(NodeId),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageType {
    Transaction = 1,
    Block = 2,
    Handshake = 3,
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
            Self::Transaction(tr) => write!(f, "{:?}", tr),
            Self::Handshake(hsh) => write!(f, "{:?}", hsh),
        }
    }
}

#[derive(Debug)]
pub enum PeerError {
    ConnectionError(Error),
    TransportError(TransportError),
    NetworkDecodingError(NetworkDecodingError),
    NetworkEncodingError(NetworkEncodingError),
    SendToNodeChannelError(SendError<PeerEvent>),
    SendToStreamChannelError(SendError<NetworkMessage>),
    SendToAppChannelError(SendError<InboundMessage>),
    EventChannelClosed,
    AppChannelClosed,
    ShutDown,
    HandShake(HandshakeError),
    ConnectionIdExhausted,
    ConnectionTimeout(SocketAddr)
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

#[derive(Debug)]
pub enum PeerEvent {
    Message(SocketAddr, NetworkMessage, ConnectionId),
    Disconnected(SocketAddr, ConnectionId),
}

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

pub struct ConnectionInitResult(
    pub SocketAddr,
    pub Sender<NetworkMessage>,
    pub JoinHandle<()>,
    pub ConnectionId,
);
