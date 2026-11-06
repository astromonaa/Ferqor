use std::net::SocketAddr;

use tokio::{
    sync::mpsc::{Sender, error::TrySendError},
    time::Instant,
};

use crate::{
    app::types::InboundMessage, network::{
        error::PeerError, peer::types::{PeerAction, PeerDisconnectReason, PeerHandle, PeerState}, protocol::{Handshake, validate_handshake}, wire::NetworkMessage,
    },
};

pub trait PeerStrategy {
    fn on_handshake_receive(
        &self,
        local_handshake: Handshake,
        remote_handshake: Handshake,
        sender: &mut PeerHandle,
    ) -> Result<PeerAction, PeerError>;

    fn on_message_receive(
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
                    return Ok(PeerAction::Disconnect(PeerDisconnectReason::ConnectionClosed));
                }
                match validate_handshake(&local_handshake, &remote_handshake) {
                    Ok(_) => {
                        /* Если handshake прошел валидацию, то помечаем текущее соединение активным */
                        sender.activate(remote_handshake);
                    }
                    /* Несовместимая сеть, версия или соединение с собой */
                    Err(e) => {
                        println!("Error while validating handshake: {e:?}");
                        return Ok(PeerAction::Disconnect(PeerDisconnectReason::InvalidProtocol));
                    }
                }

                Ok(PeerAction::Continue)
            }
            PeerState::Active(_) => Ok(PeerAction::Disconnect(PeerDisconnectReason::InvalidProtocol)),
        }
    }

    fn on_message_receive(
        &self,
        network_message: NetworkMessage,
        node_to_app_sender: &Sender<InboundMessage>,
        addr: SocketAddr,
    ) -> Result<PeerAction, PeerError> {
        match self {
            PeerState::AwaitingHandshake { .. } => Ok(PeerAction::Disconnect(PeerDisconnectReason::InvalidProtocol)),
            PeerState::Active(_) => {
                let message_to_app = InboundMessage::new(addr, network_message);
                match node_to_app_sender.try_send(message_to_app) {
                    Ok(()) => Ok(PeerAction::Continue),
                    Err(TrySendError::Full(_)) => {
                        eprintln!("App queue is full; disconnecting peer {addr}");
                        Ok(PeerAction::Disconnect(PeerDisconnectReason::ConnectionClosed))
                    }
                    Err(TrySendError::Closed(_)) => Err(PeerError::AppChannelClosed),
                }
            }
        }
    }
}
