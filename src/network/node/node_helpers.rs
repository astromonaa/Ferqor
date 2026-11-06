use std::{collections::VecDeque, net::SocketAddr, ops::ControlFlow, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::mpsc::{Sender, channel},
};

use crate::{
    app::types::{InboundMessage, NodeCommand},
    network::{
        codec::NetworkDecodeLimits,
        error::PeerError,
        node::types::{ConnectionInitResult, RetryStatesMap, SendersToStream},
        peer::{
            Peer,
            types::{ConnectionId, PeerAction, PeerDisconnectReason, PeerEvent},
        },
        protocol::Handshake,
        wire::NetworkMessage,
    },
};

/* Установка слушателей на внешнее (Tcp) соединение */
pub async fn accept_listener(listener: &TcpListener) -> Result<(SocketAddr, Peer), PeerError> {
    let (stream, addr) = listener
        .accept()
        .await
        .map_err(PeerError::ConnectionError)?;

    Ok((addr, Peer::from_stream(stream)))
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

    let mut disconnect_reason = PeerDisconnectReason::ConnectionClosed;

    /* Peer start */
    let task = tokio::spawn(async move {
        match peer
            .start(
                limits.clone(),
                addr,
                sender_to_node.clone(),
                receiver_in_stream,
                local_handshake,
                connection_id,
            )
            .await
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error while handling stream data {e:?}");
                disconnect_reason = e.into();
            }
        }
        /* После завершения таски дропаем соединение */
        drop(peer);
        /* После отправляем событие о закрытии соединения в канал */
        if sender_to_node
            .send(PeerEvent::Disconnected(
                addr,
                connection_id,
                disconnect_reason,
            ))
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
pub fn handle_peer_event(
    message: PeerEvent,
    senders_to_stream: &mut SendersToStream,
    node_to_app_sender: &Sender<InboundMessage>,
    local_handshake: Handshake,
    pending_addresses: &mut VecDeque<SocketAddr>,
    retry_states: &mut RetryStatesMap,
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

                    if matches!(action, PeerAction::Continue) {
                        println!("Peer activated: addr={addr}, connection_id={connection_id:?}");
                        if let Some(retry_state) = retry_states.get_mut(&addr) {
                            retry_state.reset();
                        }
                    }

                    handle_peer_action(
                        action,
                        senders_to_stream,
                        addr,
                        connection_id,
                        pending_addresses,
                        retry_states,
                    );
                }
                _ => {
                    let action = peer_handle.on_message_receive(
                        network_message,
                        node_to_app_sender,
                        addr,
                    )?;
                    handle_peer_action(
                        action,
                        senders_to_stream,
                        addr,
                        connection_id,
                        pending_addresses,
                        retry_states,
                    );
                }
            }
        }
        /*
           В случае Disconnect удаляем адрес из HashMap
        */
        PeerEvent::Disconnected(addr, connection_id, reason) => handle_peer_action(
            PeerAction::Disconnect(reason),
            senders_to_stream,
            addr,
            connection_id,
            pending_addresses,
            retry_states,
        ),
    }

    Ok(())
}

/* В случае Disconnect удаляем адрес из HashMap */
fn handle_peer_action(
    action: PeerAction,
    senders_to_stream: &mut SendersToStream,
    addr: SocketAddr,
    connection_id: ConnectionId,
    pending_addresses: &mut VecDeque<SocketAddr>,
    retry_states: &mut RetryStatesMap,
) {
    if matches!(action, PeerAction::Disconnect(_)) {
        let Some(peer_handle) = senders_to_stream.get(&addr) else {
            return;
        };

        if peer_handle.connection_id != connection_id {
            return;
        }

        senders_to_stream.remove(&addr);
    }

    match action {
        PeerAction::Disconnect(PeerDisconnectReason::ConnectionClosed) => {
            if let Some(retry_state) = retry_states.get_mut(&addr) {
                retry_state.record_failure();
                pending_addresses.push_back(addr);
            }
        }
        _ => {}
    }
}

/* Обработчик комманд, поступаюших из приложения */
pub fn handle_app_command(
    command: NodeCommand,
    senders_to_stream: &mut SendersToStream,
    pending_addresses: &mut VecDeque<SocketAddr>,
    retry_states: &mut RetryStatesMap,
) -> Result<ControlFlow<()>, PeerError> {
    match command {
        NodeCommand::SendTo(addr, message) => {
            if let Some(sender) = senders_to_stream.get(&addr) {
                if let Err(error) = sender.send(message) {
                    senders_to_stream.remove(&addr);
                    eprintln!("Cannot queue message for peer {addr}: {error}");
                    /* Зафиксировать failure и добавить адрес в pendind_addresses */
                    if let Some(retry_state) = retry_states.get_mut(&addr) {
                        retry_state.record_failure();
                        pending_addresses.push_back(addr);
                    }
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
                if let Err(error) = sender.send(message.clone()) {
                    failed_senders.push(*addr);
                    eprintln!("Cannot queue message for peer {addr}: {error}");
                }
            }
            for addr in failed_senders.iter() {
                senders_to_stream.remove(addr);
                /* Зафиксировать failure и добавить адрес в pendind_addresses */
                if let Some(retry_state) = retry_states.get_mut(&addr) {
                    retry_state.record_failure();
                    pending_addresses.push_back(*addr);
                }
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
                if let Err(error) = sender.send(message.clone()) {
                    failed_senders.push(*address);
                    eprintln!("Cannot queue message for peer {address}: {error}");
                }
            }
            for addr in failed_senders.iter() {
                senders_to_stream.remove(addr);
                if let Some(retry_state) = retry_states.get_mut(&addr) {
                    /* Зафиксировать failure и добавить адрес в pendind_addresses */
                    retry_state.record_failure();
                    pending_addresses.push_back(*addr);
                }
            }
        }
        NodeCommand::Disconnect(addr) => {
            senders_to_stream.remove(&addr);
            pending_addresses.retain(|address| *address != addr);
            if let Some(retry_state) = retry_states.get_mut(&addr) {
                retry_state.disable();
            }
        }
        NodeCommand::Shutdown => return Ok(ControlFlow::Break(())),
    }

    Ok(ControlFlow::Continue(()))
}
