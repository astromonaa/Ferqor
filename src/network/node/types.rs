use std::{collections::HashMap, net::SocketAddr};

use tokio::{
    sync::mpsc::{
        Sender,
        error::{TrySendError},
    },
    task::JoinHandle,
    time::Instant,
};

use crate::network::{
    peer::peer_handle_strategy::PeerStrategy,
    types::{Handshake, InboundMessage, NetworkMessage, PeerError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone)]
pub enum PeerAction {
    Continue,
    Disconnect,
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
    ) -> Self {
        Self {
            sender,
            state,
            task,
            connection_id,
        }
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

    pub async fn on_message_receive(
        &self,
        network_message: NetworkMessage,
        node_to_app_sender: &Sender<InboundMessage>,
        addr: SocketAddr,
    ) -> Result<PeerAction, PeerError> {
        self.state
            .on_message_receive(network_message, node_to_app_sender, addr)
            .await
    }
}

impl Drop for PeerHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub type SendersToStream = HashMap<SocketAddr, PeerHandle>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u64);
