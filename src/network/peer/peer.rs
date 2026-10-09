use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::{
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    time::error::Elapsed,
};

use crate::network::{codec::encode_network_message, types::PeerEvent};
use crate::network::{
    codec::{NetworkDecodeLimits, decode_network_message},
    transport::TransportClient,
    types::{NetworkMessage, PeerError},
};
use crate::network::{node::types::ConnectionId, types::Handshake};

#[derive(Debug)]
pub struct Peer {
    writer: OwnedWriteHalf,
    reader: OwnedReadHalf,
}

impl Peer {
    pub fn from_stream(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self { reader, writer }
    }

    pub async fn connect(remote_addr: &SocketAddr) -> Result<Self, PeerError> {
        let timeout =
            tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(remote_addr)).await;

        match timeout {
            Ok(stream_result) => {
                let stream = stream_result.map_err(PeerError::ConnectionError)?;
                Ok(Self::from_stream(stream))
            }
            Err(_) => Err(PeerError::ConnectionTimeout(*remote_addr)),
        }
    }

    pub async fn start(
        &mut self,
        limits: Arc<NetworkDecodeLimits>,
        addr: SocketAddr,
        sender_to_node: Sender<PeerEvent>,
        receiver_in_stream: Receiver<NetworkMessage>,
        local_handshake: Handshake,
        connection_id: ConnectionId,
    ) -> Result<(), PeerError> {
        let Peer { reader, writer } = self;

        /* Перед инициализацией основного потока отправляем handshake */
        Self::send_handshake(local_handshake, writer).await?;

        tokio::select! {
            reader = Self::run_reader(reader, sender_to_node, limits, addr, connection_id) => {
                reader?
            }
            writer = Self::run_writer(writer, receiver_in_stream) => {
                writer?
            }
        };

        Ok(())
    }

    async fn send_handshake(
        local_handshake: Handshake,
        writer: &mut OwnedWriteHalf,
    ) -> Result<(), PeerError> {
        let message = NetworkMessage::Handshake(local_handshake);
        Self::send_message(message, writer).await
    }

    async fn send_message(
        message: NetworkMessage,
        writer: &mut OwnedWriteHalf,
    ) -> Result<(), PeerError> {
        let encoded = encode_network_message(&message)?;

        match TransportClient::write_frame(writer, &encoded).await {
            Ok(_) => println!("Successfully sent message via TCP: {}", message),
            Err(e) => return Err(PeerError::TransportError(e)),
        }

        Ok(())
    }

    pub async fn run_reader(
        reader: &mut OwnedReadHalf,
        sender_to_node: Sender<PeerEvent>,
        limits: Arc<NetworkDecodeLimits>,
        addr: SocketAddr,
        connection_id: ConnectionId,
    ) -> Result<(), PeerError> {
        loop {
            match TransportClient::read_frame(reader, &limits).await {
                Ok(bytes) => {
                    let message: NetworkMessage = decode_network_message(&bytes, &limits)?;
                    sender_to_node
                        .send(PeerEvent::Message(addr, message, connection_id))
                        .await
                        .map_err(PeerError::SendToNodeChannelError)?;
                }
                Err(e) => return Err(PeerError::TransportError(e)),
            }
        }
    }

    pub async fn run_writer(
        writer: &mut OwnedWriteHalf,
        mut receiver_in_stream: Receiver<NetworkMessage>,
    ) -> Result<(), PeerError> {
        while let Some(message) = receiver_in_stream.recv().await {
            Self::send_message(message, writer).await?
        }

        Ok(())
    }
}
