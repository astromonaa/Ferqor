//! Состояние, handle и события peer-соединения.
//!
//! `PeerHandle`, `PeerState`, `PeerAction`, `SendOutcome`, `ConnectionDirection` и
//! `ConnectionId` перенесены из `network::node::types`, а `PeerEvent` — из
//! `network::types`. Имена, методы, варианты и поведение сохранены без изменений.

use std::net::SocketAddr;

use tokio::{
    sync::mpsc::{Sender, error::TrySendError},
    task::JoinHandle,
    time::Instant,
};

use crate::{
    app::types::InboundMessage,
    network::{
        error::{PeerError, TransportError},
        peer::peer_handle_strategy::PeerStrategy,
        protocol::Handshake,
        wire::NetworkMessage,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone)]
pub enum PeerAction {
    Continue,
    Disconnect(PeerDisconnectReason),
}

#[derive(Debug, Clone)]
pub enum PeerState {
    AwaitingHandshake { deadline: Instant },
    Active(Handshake),
}

pub struct PeerHandle {
    sender: Sender<NetworkMessage>,
    state: PeerState,
    task: JoinHandle<()>,
    connection_direction: ConnectionDirection,
    pub connection_id: ConnectionId,
}

pub enum SendOutcome {
    Queued,
    SkippedNotReady,
}

impl PeerHandle {
    pub fn new(
        sender: Sender<NetworkMessage>,
        state: PeerState,
        task: JoinHandle<()>,
        connection_id: ConnectionId,
        connection_direction: ConnectionDirection,
    ) -> Self {
        Self {
            sender,
            state,
            task,
            connection_id,
            connection_direction
        }
    }

    pub fn get_connection_direction(&self) -> ConnectionDirection {
        self.connection_direction
    }

    pub fn get_state(&self) -> PeerState {
        self.state.clone()
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, PeerState::Active(_))
    }

    pub fn send(
        &self,
        message: NetworkMessage,
    ) -> Result<SendOutcome, TrySendError<NetworkMessage>> {
        if self.is_active() {
            self.sender.try_send(message)?;
            return Ok(SendOutcome::Queued);
        }

        Ok(SendOutcome::SkippedNotReady)
    }

    pub fn activate(&mut self, handshake: Handshake) {
        self.state = PeerState::Active(handshake);
    }

    pub fn on_handshake_receive(
        &mut self,
        local_handshake: Handshake,
        remote_handshake: Handshake,
    ) -> Result<PeerAction, PeerError> {
        let state = self.state.clone();
        state.on_handshake_receive(local_handshake, remote_handshake, self)
    }

    pub fn on_message_receive(
        &self,
        network_message: NetworkMessage,
        node_to_app_sender: &Sender<InboundMessage>,
        addr: SocketAddr,
    ) -> Result<PeerAction, PeerError> {
        self.state
            .on_message_receive(network_message, node_to_app_sender, addr)
    }
}

impl Drop for PeerHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u64);

#[derive(Debug, Clone)]
pub enum PeerDisconnectReason {
    InvalidProtocol,
    ConnectionClosed,
    LocalStop,
}

#[derive(Debug)]
pub enum PeerEvent {
    Message(SocketAddr, NetworkMessage, ConnectionId),
    Disconnected(SocketAddr, ConnectionId, PeerDisconnectReason),
}

impl From<PeerError> for PeerDisconnectReason {
    fn from(value: PeerError) -> Self {
        match value {
            PeerError::TransportError(TransportError::ReadBytesError(_)) => {
                PeerDisconnectReason::ConnectionClosed
            },
            PeerError::TransportError(TransportError::WriteBytesError(_)) => {
                PeerDisconnectReason::ConnectionClosed
            }
            PeerError::TransportError(_) => PeerDisconnectReason::InvalidProtocol,
            PeerError::ConnectionError(_) => PeerDisconnectReason::ConnectionClosed,
            PeerError::ConnectionTimeout(_) => PeerDisconnectReason::ConnectionClosed,
            PeerError::NetworkDecodingError(_) => PeerDisconnectReason::InvalidProtocol,
            PeerError::HandShake(_) => PeerDisconnectReason::InvalidProtocol,
            PeerError::NetworkEncodingError(_) => PeerDisconnectReason::LocalStop,
            PeerError::ConnectionIdExhausted => PeerDisconnectReason::LocalStop,
            PeerError::SendToStreamChannelError(_) => PeerDisconnectReason::ConnectionClosed,
            PeerError::SendToNodeChannelError(_) => PeerDisconnectReason::LocalStop,
            PeerError::EventChannelClosed => PeerDisconnectReason::LocalStop,
            PeerError::AppChannelClosed => PeerDisconnectReason::LocalStop,
            PeerError::ShutDown => PeerDisconnectReason::LocalStop,
            PeerError::SendToAppChannelError(TrySendError::Full(_)) => {
                PeerDisconnectReason::ConnectionClosed
            }
            PeerError::SendToAppChannelError(TrySendError::Closed(_)) => {
                PeerDisconnectReason::LocalStop
            }
        }
    }
}
