use std::net::SocketAddr;

use tokio::{sync::mpsc::Sender, time::Instant};

use crate::network::{
    node::types::{PeerAction, PeerHandle, PeerState}, types::{Handshake, HandshakeError, InboundMessage, NetworkMessage, PeerError},
};

pub trait PeerStrategy {
    fn on_handshake_receive(
        &self,
        local_handshake: Handshake,
        remote_handshake: Handshake,
        sender: &mut PeerHandle,
    ) -> Result<PeerAction, PeerError>;

    async fn on_message_receive(
        &self,
        network_message: NetworkMessage,
        node_to_app_sender: &Sender<InboundMessage>,
        addr: SocketAddr,
    ) -> Result<PeerAction, PeerError>;
}

impl PeerStrategy for PeerState {
    fn on_handshake_receive(
        &self,
        local_handshake: Handshake,
        remote_handshake: Handshake,
        sender: &mut PeerHandle,
    ) -> Result<PeerAction, PeerError> {
        match self {
            PeerState::AwaitingHandshake { deadline } => {
                if Instant::now() >= *deadline {
                    return Ok(PeerAction::Disconnect)
                }
                match validate_handshake(&local_handshake, &remote_handshake) {
                    Ok(_) => {
                        /* Если handshake прошел валидацию, то помечаем текущее соединение активным */
                        sender.activate(remote_handshake);
                    }
                    Err(e) => {
                        println!("Error while validating handshake: {e:?}");
                        return Ok(PeerAction::Disconnect)
                    }
                }

                Ok(PeerAction::Continue)
            }
            PeerState::Active(_) => Ok(PeerAction::Disconnect),
        }
    }

    async fn on_message_receive(
        &self,
        network_message: NetworkMessage,
        node_to_app_sender: &Sender<InboundMessage>,
        addr: SocketAddr,
    ) -> Result<PeerAction, PeerError> {
        match self {
            PeerState::AwaitingHandshake { .. } => Ok(PeerAction::Disconnect),
            PeerState::Active(_) => {
                let message_to_app = InboundMessage::new(addr, network_message);
                node_to_app_sender
                    .send(message_to_app)
                    .await
                    .map_err(PeerError::SendToAppChannelError)?;
                Ok(PeerAction::Continue)
            }
        }
    }
}

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
