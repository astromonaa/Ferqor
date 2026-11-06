use std::{
    collections::{HashSet, VecDeque},
    net::SocketAddr,
};

use crate::network::{node::types::SendersToStream, peer::types::ConnectionDirection};

pub fn get_pending_addresses(addresses: &[SocketAddr]) -> VecDeque<SocketAddr> {
    let mut seen = HashSet::new();
    let mut pending_addresses: VecDeque<SocketAddr> = VecDeque::from(addresses.to_owned());
    pending_addresses.retain(|x| seen.insert(*x));
    pending_addresses
}

/* Общий лимит подключений Ноды */
pub enum ConnectionLimits {
    Inbound = 22,
    Outbound = 8,
}

pub enum ConnectionLimitErrors {
    InboundLimit,
    OutBoundLimit,
}

pub fn get_registered_outbound_count(senders_to_stream: &mut SendersToStream) -> usize {
    senders_to_stream
        .iter()
        .filter(|(_, peer_handle)| {
            peer_handle.get_connection_direction() == ConnectionDirection::Outbound
        })
        .count()
}

pub fn check_outbound_connections_limit(
    senders_to_stream: &mut SendersToStream,
    pending_addresses: &mut VecDeque<SocketAddr>,
    addr: SocketAddr
) -> Result<(), ConnectionLimitErrors> {
    let outbound_connections_count = get_registered_outbound_count(senders_to_stream);

    if outbound_connections_count >= ConnectionLimits::Outbound as usize {
        println!("Connections count limit reach");
        pending_addresses.push_back(addr);
        return Err(ConnectionLimitErrors::OutBoundLimit);
    }

    Ok(())
}



pub fn check_inbound_connections_limit(
    senders_to_stream: &mut SendersToStream,
) -> Result<(), ConnectionLimitErrors> {
    let inbound_connections_count = senders_to_stream
        .iter()
        .filter(|(_, peer_handle)| {
            peer_handle.get_connection_direction() == ConnectionDirection::Inbound
        })
        .count();

    if inbound_connections_count >= ConnectionLimits::Inbound as usize {
        println!("Connections count limit reach");
        return Err(ConnectionLimitErrors::InboundLimit);
    }

    Ok(())
}
