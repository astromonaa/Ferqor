//! Данные идентификации и handshake, перенесённые из `network::types`.
//!
//! Состав полей, варианты ошибок и методы сохранены без изменений.

use std::net::SocketAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

/// Проверяет handshake логикой прежней
/// `network::peer::peer_handle_strategy::validate_handshake`.
pub fn validate_handshake(local: &Handshake, remote: &Handshake) -> Result<(), HandshakeError> {
    if local.protocol_version != remote.protocol_version {
        return Err(HandshakeError::ProtocolVersionMismatch(
            remote.protocol_version,
        ));
    }

    if local.network_id != remote.network_id {
        return Err(HandshakeError::NetworkIdMismatch(remote.network_id));
    }

    if local.node_id == remote.node_id {
        return Err(HandshakeError::SelfConnection(remote.node_id));
    }

    Ok(())
}
