//! Тесты правил handshake после переноса из `network::peer::peer_handle_strategy`.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use crate::network::protocol::{Handshake, HandshakeError, NetworkId, NodeId, validate_handshake};

fn handshake(protocol_version: u8, network: u8, node: u8) -> Handshake {
    Handshake::new(
        protocol_version,
        NetworkId([network; 32]),
        NodeId([node; 32]),
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8333)),
    )
}

#[test]
fn validate_handshake_preserves_rules() {
    let local = handshake(1, 2, 3);
    let remote = handshake(1, 2, 4);

    validate_handshake(&local, &remote).expect("compatible peer should pass");

    let incompatible = handshake(2, 2, 4);
    assert!(matches!(
        validate_handshake(&local, &incompatible),
        Err(HandshakeError::ProtocolVersionMismatch(2))
    ));
}
