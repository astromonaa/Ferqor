//! Реестр соединений и результат их инициализации.

use std::time::Duration;
use std::{collections::HashMap, net::SocketAddr};

use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::network::error::PeerError;
use crate::network::peer::Peer;
use crate::network::wire::NetworkMessage;

use crate::network::peer::types::{ConnectionId, PeerHandle};

pub type SendersToStream = HashMap<SocketAddr, PeerHandle>;

/// Результат инициализации соединения, перенесённый из `network::types`
/// без изменения состава данных.
pub struct ConnectionInitResult(
    pub SocketAddr,
    pub Sender<NetworkMessage>,
    pub JoinHandle<()>,
    pub ConnectionId,
);

pub type ConnectionAttempts = JoinSet<(SocketAddr, Result<Peer, PeerError>)>;

pub struct ConnectionRetryState {
    pub failures: u32,
    pub next_attempt_at: Instant,
    pub enabled: bool,
}

impl ConnectionRetryState {
    pub fn new() -> Self {
        Self {
            failures: 0,
            next_attempt_at: Instant::now(),
            enabled: true,
        }
    }

    pub fn record_failure(&mut self) {
        self.failures = self.failures.saturating_add(1);

        self.next_attempt_at = match self.failures {
            1 => Instant::now() + Duration::from_secs(rand::random_range(2..=3)),
            2 => Instant::now() + Duration::from_secs(rand::random_range(4..=6)),
            3 => Instant::now() + Duration::from_secs(rand::random_range(8..=12)),
            4 => Instant::now() + Duration::from_secs(rand::random_range(16..=24)),
            5 => Instant::now() + Duration::from_secs(rand::random_range(32..=45)),
            _ => Instant::now() + Duration::from_secs(rand::random_range(55..=60)),
        }
    }

    pub fn reset(&mut self) {
        self.failures = 0;
        self.next_attempt_at = Instant::now()
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

pub type RetryStatesMap = HashMap<SocketAddr, ConnectionRetryState>;
