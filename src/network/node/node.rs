use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    ops::ControlFlow,
    sync::Arc,
    time::Duration,
};

use tokio::{
    net::TcpListener,
    sync::mpsc::{Receiver, Sender, channel},
    task::{JoinError, JoinSet},
    time::{Instant, interval},
};

use crate::{
    app::types::{InboundMessage, NodeCommand},
    network::{
        codec::NetworkDecodeLimits,
        error::PeerError,
        node::{
            node_helpers::{
                accept_listener, handle_app_command, handle_peer_event, init_peer_connection,
            },
            types::{
                ConnectionAttempts, ConnectionInitResult, ConnectionRetryState, RetryStatesMap,
                SendersToStream,
            },
            utils::{
                ConnectionLimits, check_inbound_connections_limit,
                check_outbound_connections_limit, get_pending_addresses,
                get_registered_outbound_count,
            },
        },
        peer::{
            Peer,
            types::{ConnectionDirection, PeerEvent, PeerHandle, PeerState},
        },
        protocol::Handshake,
    },
};

/* Общий лимит подключений Ноды */
const CONNECTIONS_LIMIT: usize = 30;
/*
 * Максимальное число одновременных исходящих попыток подключения
 * Т.е. итеративный процесс подключения к внешним слушателям будет выполняться
 * до тех пор, пока не будет достигнут лимит CONNECT_ATTEMPTS_LIMIT
*/
const CONNECT_ATTEMPTS_LIMIT: usize = 4;

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
        listen_addr: SocketAddr,
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
            connect_addrs,
        )
        .await?;

        Ok(())
    }

    /* Инициализация соединений ко внешним слушателям */
    fn fill_connections_attempts(
        pending_addresses: &mut VecDeque<SocketAddr>,
        connection_attempts: &mut ConnectionAttempts,
        senders_to_stream: &mut SendersToStream,
        retry_states: &mut RetryStatesMap,
    ) {
        let registered_peers_count = senders_to_stream.len();

        let outbound_limit = ConnectionLimits::Outbound as usize;
        let registered_outbounds_count = get_registered_outbound_count(senders_to_stream);
        let free_outbound_slots = outbound_limit.saturating_sub(registered_outbounds_count);

        let free_peer_slots = CONNECTIONS_LIMIT.saturating_sub(registered_peers_count);
        let available_limit = CONNECT_ATTEMPTS_LIMIT
            .min(free_peer_slots)
            .min(free_outbound_slots);
        /*
         * Пока количество соединений меньше лимита, создаём новые соединения к внешним слушателям
         */
        let now = Instant::now();

        for _ in 0..pending_addresses.len() {
            if connection_attempts.len() >= available_limit {
                return;
            }
            match pending_addresses.pop_front() {
                Some(addr) => {
                    if let Some(retry_state) = retry_states.get(&addr) {
                        if !retry_state.enabled {
                            continue;
                        }
                        if now < retry_state.next_attempt_at {
                            pending_addresses.push_back(addr);
                            continue;
                        }
                    }
                    connection_attempts.spawn(async move {
                        let result = Peer::connect(&addr).await;
                        (addr, result)
                    });
                }
                None => break,
            };
        }
    }

    /* Обработка соединений к внешним клиентам */
    fn handle_connection_attempt(
        connection_result: Result<(SocketAddr, Result<Peer, PeerError>), JoinError>,
        limits: Arc<NetworkDecodeLimits>,
        local_handshake: Handshake,
        sender_to_node: Sender<PeerEvent>,
        senders_to_stream: &mut SendersToStream,
        connections_count: &mut u64,
        pending_addresses: &mut VecDeque<SocketAddr>,
        retry_states: &mut RetryStatesMap,
    ) -> Result<(), PeerError> {
        match connection_result {
            Ok((addr, peer_result)) => {
                if let Some(retry_state) = retry_states.get_mut(&addr) {
                    if !retry_state.enabled {
                        return Ok(());
                    };
                }

                match peer_result {
                    Ok(peer) => {
                        /*
                         * Если количество соединений достигло лимита, то не создаём новое соединение
                         */

                        if check_outbound_connections_limit(
                            senders_to_stream,
                            pending_addresses,
                            addr,
                        )
                        .is_err()
                        {
                            return Ok(());
                        }

                        let connection_result = init_peer_connection(
                            peer,
                            limits,
                            sender_to_node,
                            addr,
                            local_handshake,
                            connections_count,
                        );
                        Self::on_connection_init(
                            connection_result,
                            senders_to_stream,
                            ConnectionDirection::Outbound,
                        )?
                    }
                    Err(e) => {
                        if let Some(retry_state) = retry_states.get_mut(&addr) {
                            retry_state.record_failure();
                            pending_addresses.push_back(addr);
                        }
                        eprintln!(
                            "Error while connecting to peer: address -> {addr}; errr ->{:?}",
                            e
                        );
                    }
                }
            }
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
        connect_addrs: &[SocketAddr],
    ) -> Result<(), PeerError> {
        let mut handshake_check_interval = interval(Duration::from_secs(1));

        let mut retry_states: RetryStatesMap = connect_addrs
            .iter()
            .map(|addr| (addr.clone(), ConnectionRetryState::new()))
            .collect();

        /* Инициализация очереди адресов для подключения */
        let mut pending_addresses = get_pending_addresses(connect_addrs);

        /* Инициализация попыток соединений ко внешним слушателям */
        let mut connection_attempts: ConnectionAttempts = JoinSet::new();

        loop {
            /* Заполнение очереди адресов для подключения */
            Self::fill_connections_attempts(
                &mut pending_addresses,
                &mut connection_attempts,
                senders_to_stream,
                &mut retry_states,
            );

            tokio::select! {
                /*
                    Установка слушателей на внешнее (Tcp) соединение и сохранение клиента,
                    с которым произошло соединение
                */
                accepted_connection = accept_listener(&listener) => {
                    let (addr, peer) = accepted_connection?;
                    /*
                     * Если количество соединений достигло лимита, то не создаём новое соединение
                     */
                    if check_inbound_connections_limit(senders_to_stream).is_err() {
                        continue;
                    }

                    let connection_result = init_peer_connection(
                        peer, limits.clone(),
                        sender_to_node.clone(),
                        addr,
                        local_handshake.clone(),
                        connections_count
                    );

                    Self::on_connection_init(connection_result, senders_to_stream, ConnectionDirection::Inbound)?
                }

                /* Обработка соединений к внешним клиентам */
                Some(connection_result) = connection_attempts.join_next() => {
                    Self::handle_connection_attempt(
                        connection_result,
                        limits.clone(),
                        local_handshake.clone(),
                        sender_to_node.clone(),
                        senders_to_stream,
                        connections_count,
                        &mut pending_addresses,
                        &mut retry_states
                    )?
                }

                /* Обработка сообщений поступающих из соединения (Tcp) */
                received_from_stream = receiver_in_node.recv() => {
                    Self::on_message_from_stream(
                        received_from_stream,
                        senders_to_stream,
                        &node_to_app_sender,
                        local_handshake.clone(),
                        &mut pending_addresses,
                        &mut retry_states
                    )?
                }

                /* Обработка сообщений из поступающих из приложения (App) */
                received_from_app = app_to_node_receiver.recv() => {
                    match Self::on_message_from_app(
                        received_from_app,
                        senders_to_stream,
                        &mut pending_addresses,
                        &mut retry_states
                    ) {
                        Ok(ControlFlow::Break(_)) => return Ok(()),
                        Err(e) =>  return Err(e),
                        _ => {},
                    }
                }

                _ = handshake_check_interval.tick() => {
                    Self::check_expired_connections(senders_to_stream, &mut retry_states, &mut pending_addresses);
                }
            }
        }
    }

    fn check_expired_connections(
        senders_to_stream: &mut SendersToStream,
        retry_states: &mut RetryStatesMap,
        pending_addresses: &mut VecDeque<SocketAddr>,
    ) {
        let mut expired = vec![];
        let now = Instant::now();
        for (_, (addr, peer_handle)) in senders_to_stream.iter().enumerate() {
            match peer_handle.get_state() {
                PeerState::Active(_) => continue,
                PeerState::AwaitingHandshake { deadline } => {
                    if now >= deadline {
                        expired.push(addr.clone());
                        if let Some(retry_state) = retry_states.get_mut(&addr) {
                            retry_state.record_failure();
                            pending_addresses.push_back(addr.clone());
                        }
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
        direction: ConnectionDirection,
    ) -> Result<(), PeerError> {
        let ConnectionInitResult(addr, sender, task, connection_id) = connection_result?;
        let peer_state = PeerState::AwaitingHandshake {
            deadline: Instant::now() + Duration::from_secs(5),
        };
        let peer_handle = PeerHandle::new(sender, peer_state, task, connection_id, direction);

        /* Сохраняем отправителя в stream */
        senders_to_stream.entry(addr).or_insert(peer_handle);
        Ok(())
    }

    fn on_message_from_stream(
        received_from_stream: Option<PeerEvent>,
        senders_to_stream: &mut SendersToStream,
        node_to_app_sender: &Sender<InboundMessage>,
        local_handshake: Handshake,
        pending_addresses: &mut VecDeque<SocketAddr>,
        retry_states: &mut RetryStatesMap,
    ) -> Result<(), PeerError> {
        match received_from_stream {
            Some(message) => {
                handle_peer_event(
                    message,
                    senders_to_stream,
                    node_to_app_sender,
                    local_handshake,
                    pending_addresses,
                    retry_states,
                )?;
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
        pending_addresses: &mut VecDeque<SocketAddr>,
        retry_states: &mut RetryStatesMap,
    ) -> Result<ControlFlow<()>, PeerError> {
        let result = match received_from_app {
            Some(command) => {
                println!("Got command from App to Node: {:?}", command);
                handle_app_command(command, senders_to_stream, pending_addresses, retry_states)
            }
            None => {
                eprintln!("Error getting message from App to Node channel");
                return Err(PeerError::AppChannelClosed);
            }
        };

        result
    }
}
