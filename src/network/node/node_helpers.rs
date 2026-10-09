use std::{net::SocketAddr, ops::ControlFlow, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::mpsc::{Sender, channel},
};

use crate::network::{
    codec::NetworkDecodeLimits,
    node::types::{ConnectionId, PeerAction, SendersToStream},
    peer::Peer,
    types::{
        ConnectionInitResult, Handshake, InboundMessage, NetworkMessage, NodeCommand, PeerError,
        PeerEvent,
    },
};

/* Установка слушателей на внешнее (Tcp) соединение */
pub async fn accept_listener(
    listener: &TcpListener,
    limits: Arc<NetworkDecodeLimits>,
    sender_to_node: Sender<PeerEvent>,
    local_handshake: Handshake,
    connections_count: &mut u64,
) -> Result<ConnectionInitResult, PeerError> {
    match listener.accept().await {
        Ok((stream, addr)) => {
            let peer = Peer::from_stream(stream);
            init_peer_connection(
                peer,
                limits,
                sender_to_node,
                addr,
                local_handshake,
                connections_count,
            )
        }
        Err(e) => return Err(PeerError::ConnectionError(e)),
    }
}

pub fn init_peer_connection(
    mut peer: Peer,
    limits: Arc<NetworkDecodeLimits>,
    sender_to_node: Sender<PeerEvent>,
    addr: SocketAddr,
    local_handshake: Handshake,
    connections_count: &mut u64,
) -> Result<ConnectionInitResult, PeerError> {
    /*  Канал, который слушает Node и пишет данные в stream */
    let (sender_to_stream, receiver_in_stream) = channel::<NetworkMessage>(100);

    *connections_count = (*connections_count)
        .checked_add(1)
        .ok_or(PeerError::ConnectionIdExhausted)?;
    
    let connection_id = ConnectionId(*connections_count);

    /* Peer start */
    let task = tokio::spawn(async move {
        match peer
            .start(
                limits.clone(),
                addr,
                sender_to_node.clone(),
                receiver_in_stream,
                local_handshake,
                connection_id
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error while handling stream data {e:?}");
            }
        }
        /* После завершения таски дропаем соединение */
        drop(peer);
        /* После отправляем событие о закрытии соединения в канал */
        if sender_to_node
            .send(PeerEvent::Disconnected(addr, connection_id))
            .await
            .is_err()
        {
            eprintln!("Error while sending Disconnection Event to Node")
        }
    });

    Ok(ConnectionInitResult(
        addr,
        sender_to_stream,
        task,
        connection_id,
    ))
}

/* Обработка сообщений поступающих из соединения (Tcp) */
pub async fn handle_peer_event(
    message: PeerEvent,
    senders_to_stream: &mut SendersToStream,
    node_to_app_sender: &Sender<InboundMessage>,
    local_handshake: Handshake,
) -> Result<(), PeerError> {
    match message {
        PeerEvent::Message(addr, network_message, connection_id) => {
            let Some(peer_handle) = senders_to_stream.get_mut(&addr) else {
                return Ok(());
            };

            if peer_handle.connection_id != connection_id {
                return Ok(());
            }

            match network_message {
                /* Если это handshake, обрабатываем его */
                NetworkMessage::Handshake(remote_handshake) => {
                    let action =
                        peer_handle.on_handshake_receive(local_handshake, remote_handshake)?;
                    handle_peer_action(action, senders_to_stream, addr, connection_id);
                }
                _ => {
                    let action = peer_handle
                        .on_message_receive(network_message, node_to_app_sender, addr)
                        .await?;
                    handle_peer_action(action, senders_to_stream, addr, connection_id);
                }
            }
        }
        /* В случае Disconnect удаляем адрес из HashMap */
        PeerEvent::Disconnected(addr, connection_id) => {
            handle_peer_action(PeerAction::Disconnect, senders_to_stream, addr, connection_id)
        }
    }

    Ok(())
}

/* В случае Disconnect удаляем адрес из HashMap */
fn handle_peer_action(
    action: PeerAction,
    senders_to_stream: &mut SendersToStream,
    addr: SocketAddr,
    connection_id: ConnectionId
) {
    if matches!(action, PeerAction::Disconnect) {
        let Some(peer_handle) = senders_to_stream.get(&addr) else {
            return;
        };

        if peer_handle.connection_id != connection_id {
            return;
        }
        
        senders_to_stream.remove(&addr);
    }
}

/* Обработчик комманд, поступаюших из приложения */
pub fn handle_app_command(
    command: NodeCommand,
    senders_to_stream: &mut SendersToStream,
) -> Result<ControlFlow<()>, PeerError> {
    match command {
        NodeCommand::SendTo(addr, message) => {
            let sender_to_stream = senders_to_stream.get(&addr);
            if let Some(sender) = sender_to_stream {
                if sender.send(message).is_err() {
                    senders_to_stream.remove(&addr);
                }
            } else {
                println!("Sender not found");
            }
        }
        NodeCommand::Broadcast(message) => {
            let mut failed_senders = Vec::new();
            for (addr, sender) in senders_to_stream.iter() {
                /*
                  Если произошла ошибка отправки конкретному адресу,
                  закрываем канал общения с данным клиентом
                */
                if sender.send(message.clone()).is_err() {
                    failed_senders.push(*addr);
                }
            }
            for addr in failed_senders.iter() {
                senders_to_stream.remove(addr);
            }
        }
        NodeCommand::BroadcastExcept(addr, message) => {
            let mut failed_senders = Vec::new();
            for (address, sender) in senders_to_stream.iter() {
                if addr == *address {
                    continue;
                }
                /*
                  Если произошла ошибка отправки конкретному адресу,
                  закрываем канал общения с данным клиентом
                */
                if sender.send(message.clone()).is_err() {
                    failed_senders.push(*address);
                }
            }
            for addr in failed_senders.iter() {
                senders_to_stream.remove(addr);
            }
        }
        NodeCommand::Disconnect(addr) => {
            senders_to_stream.remove(&addr);
        }
        NodeCommand::Shutdown => return Ok(ControlFlow::Break(())),
    }

    Ok(ControlFlow::Continue(()))
}
