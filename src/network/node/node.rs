use std::{collections::HashMap, net::SocketAddr, ops::ControlFlow, sync::Arc, time::Duration};

use tokio::{
    net::TcpListener,
    sync::mpsc::{Receiver, Sender, channel},
    task::{JoinError, JoinSet},
    time::{Instant, interval},
};

use crate::network::{
    codec::NetworkDecodeLimits,
    node::{
        node_helpers::{
            accept_listener, handle_app_command, handle_peer_event, init_peer_connection,
        },
        types::{PeerHandle, PeerState, SendersToStream},
    },
    peer::Peer,
    types::{ConnectionInitResult, Handshake, InboundMessage, NodeCommand, PeerError, PeerEvent},
};

pub struct Node {
    node_to_app_sender: Sender<InboundMessage>,
    app_to_node_receiver: Receiver<NodeCommand>,
    local_handshake: Handshake,
    connections_count: u64,
}

/**
* Последовательность во времени такая
*
1. listener.accept() возвращает TcpStream
2. создаётся Peer
3. запускается peer.start(...)
4. Node сохраняет Sender в HashMap
5. reader и writer работают продолжительное время
6. соединение закрывается или возникает ошибка
7. peer.start(...).await завершается
8. отправляется PeerEvent::Disconnected
9. Node удаляет Sender из HashMap
*
*/

impl Node {
    pub fn new(
        node_to_app_sender: Sender<InboundMessage>,
        app_to_node_receiver: Receiver<NodeCommand>,
        local_handshake: Handshake,
    ) -> Self {
        Self {
            node_to_app_sender,
            app_to_node_receiver,
            local_handshake,
            connections_count: 0,
        }
    }

    pub async fn run(
        &mut self,
        limits: &NetworkDecodeLimits,
        listen_addr: &str,
        connect_addrs: &[SocketAddr],
    ) -> Result<(), PeerError> {
        let listener = TcpListener::bind(listen_addr)
            .await
            .map_err(PeerError::ConnectionError)?;

        /* Канал, который слушает стрим и отправляет полученные данные в Node */
        let (sender_to_node, mut receiver_in_node) = channel::<PeerEvent>(100);

        /* Отправители данных из Node в TcpStream */
        let mut senders_to_stream: SendersToStream = HashMap::new();

        let limits = Arc::new(*limits);

        let Node {
            node_to_app_sender,
            app_to_node_receiver,
            local_handshake,
            connections_count,
        } = self;

        /* Инициализация соединений ко внешним слушателям */
        let connection_attempts = Self::init_connections(connect_addrs);

        /* Инициализация слушателей на внешние соединения, бесконечный цикл прослушивания */
        Self::run_event_loop(
            node_to_app_sender,
            app_to_node_receiver,
            local_handshake.clone(),
            connections_count,
            listener,
            limits.clone(),
            sender_to_node.clone(),
            &mut receiver_in_node,
            &mut senders_to_stream,
            connection_attempts,
        )
        .await?;

        Ok(())
    }

    /* Инициализация соединений ко внешним слушателям */
    fn init_connections(
        connect_addrs: &[SocketAddr],
    ) -> JoinSet<(SocketAddr, Result<Peer, PeerError>)> {
        let mut connection_attempts = JoinSet::new();
        for addr in connect_addrs {
            let addr = *addr;
            connection_attempts.spawn(async move {
                let result = Peer::connect(&addr).await;
                (addr, result)
            });
        }

        connection_attempts
    }

    fn handle_initialized_connection(
        connection_result: Result<(SocketAddr, Result<Peer, PeerError>), JoinError>,
        limits: Arc<NetworkDecodeLimits>,
        local_handshake: Handshake,
        sender_to_node: Sender<PeerEvent>,
        senders_to_stream: &mut SendersToStream,
        connections_count: &mut u64,
    ) -> Result<(), PeerError> {
        match connection_result {
            Ok((addr, peer_result)) => match peer_result {
                Ok(peer) => {
                    let connection_result = init_peer_connection(
                        peer,
                        limits,
                        sender_to_node,
                        addr,
                        local_handshake,
                        connections_count,
                    );
                    Self::on_connection_init(connection_result, senders_to_stream)?
                }
                Err(e) => {
                    eprintln!(
                        "Error while connecting to peer: address -> {addr}; errr ->{:?}",
                        e
                    );
                }
            },
            Err(e) => {
                eprintln!("Error while joining to peer task: errr ->{:?}", e);
            }
        }

        Ok(())
    }

    /* Запуска обработки потока сообщений */
    async fn run_event_loop(
        node_to_app_sender: &mut Sender<InboundMessage>,
        app_to_node_receiver: &mut Receiver<NodeCommand>,
        local_handshake: Handshake,
        connections_count: &mut u64,
        listener: TcpListener,
        limits: Arc<NetworkDecodeLimits>,
        sender_to_node: Sender<PeerEvent>,
        receiver_in_node: &mut Receiver<PeerEvent>,
        senders_to_stream: &mut SendersToStream,
        mut connection_attempts: JoinSet<(SocketAddr, Result<Peer, PeerError>)>,
    ) -> Result<(), PeerError> {
        let mut handshake_check_interval = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                /*
                    Установка слушателей на внешнее (Tcp) соединение и сохранение клиента,
                    с которым произошло соединение
                */
                connection_result = accept_listener(&listener, limits.clone(), sender_to_node.clone(), local_handshake.clone(), connections_count) => {
                    Self::on_connection_init(connection_result, senders_to_stream)?
                }
                
                /* Обработка соединений к внешним клиентам */
                Some(connection_result) = connection_attempts.join_next() => {
                    Self::handle_initialized_connection(connection_result, limits.clone(), local_handshake.clone(), sender_to_node.clone(), senders_to_stream, connections_count)?
                }

                /* Обработка сообщений поступающих из соединения (Tcp) */
                received_from_stream = receiver_in_node.recv() => {
                    Self::on_message_from_stream(received_from_stream, senders_to_stream, &node_to_app_sender, local_handshake.clone()).await?
                }

                /* Обработка сообщений из поступающих из приложения (App) */
                received_from_app = app_to_node_receiver.recv() => {
                    match Self::on_message_from_app(received_from_app, senders_to_stream) {
                        Ok(ControlFlow::Break(_)) => return Ok(()),
                        Err(e) =>  return Err(e),
                        _ => {},
                    }
                }

                _ = handshake_check_interval.tick() => {
                    Self::check_expired_connections(senders_to_stream);
                }
            }
        }
    }

    fn check_expired_connections(senders_to_stream: &mut SendersToStream) {
        let mut expired = vec![];
        let now = Instant::now();
        for (_, (addr, peer_handle)) in senders_to_stream.iter().enumerate() {
            match peer_handle.get_state() {
                PeerState::Active(_) => continue,
                PeerState::AwaitingHandshake { deadline } => {
                    if now >= deadline {
                        expired.push(addr.clone());
                    }
                }
            }
        }

        for addr in expired {
            senders_to_stream.remove(&addr);
        }
    }

    /* Обработка результата соединения ко внешней Node*/
    fn on_connection_init(
        connection_result: Result<ConnectionInitResult, PeerError>,
        senders_to_stream: &mut SendersToStream,
    ) -> Result<(), PeerError> {
        let ConnectionInitResult(addr, sender, task, connection_id) = connection_result?;
        let peer_state = PeerState::AwaitingHandshake {
            deadline: Instant::now() + Duration::from_secs(5),
        };
        let peer_handle = PeerHandle::new(sender, peer_state, task, connection_id);

        /* Сохраняем отправителя в stream */
        senders_to_stream.insert(addr, peer_handle);
        Ok(())
    }

    async fn on_message_from_stream(
        received_from_stream: Option<PeerEvent>,
        senders_to_stream: &mut SendersToStream,
        node_to_app_sender: &Sender<InboundMessage>,
        local_handshake: Handshake,
    ) -> Result<(), PeerError> {
        match received_from_stream {
            Some(message) => {
                handle_peer_event(
                    message,
                    senders_to_stream,
                    node_to_app_sender,
                    local_handshake,
                )
                .await?;
            }
            None => {
                eprintln!("Error getting message from Stream to Node channel");
                return Err(PeerError::EventChannelClosed);
            }
        }

        Ok(())
    }

    fn on_message_from_app(
        received_from_app: Option<NodeCommand>,
        senders_to_stream: &mut SendersToStream,
    ) -> Result<ControlFlow<()>, PeerError> {
        let result = match received_from_app {
            Some(command) => {
                println!("Got command from App to Node: {:?}", command);
                handle_app_command(command, senders_to_stream)
            }
            None => {
                eprintln!("Error getting message from App to Node channel");
                return Err(PeerError::AppChannelClosed);
            }
        };

        result
    }
}
