//! Golden-тесты wire-формата, зафиксированного прежним `network::codec`.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use crate::{
    address::Address,
    block::BlockDecodeLimits,
    network::{
        codec::{
            NetworkDecodeLimits, decode_network_frame, decode_network_message,
            encode_network_message,
        },
        codec_handshake::encode_handshake,
        frame::{FRAME_HEADER_LEN, NetworkFrame, decode_frame_header},
        protocol::{Handshake, NetworkId, NodeId},
        wire::NetworkMessage,
    },
    transaction::Transaction,
};

fn limits() -> NetworkDecodeLimits {
    NetworkDecodeLimits {
        max_tx_bytes: 1_000,
        block_limits: BlockDecodeLimits {
            max_block_bytes: 10_000,
            max_transactions: 100,
            max_tx_bytes: 1_000,
        },
    }
}

fn unsigned_transaction() -> Transaction {
    Transaction {
        from: Address(core::array::from_fn(|index| index as u8)),
        to: Address(core::array::from_fn(|index| (index + 20) as u8)),
        amount: 0x0102_0304_0506_0708,
        fee: 0x1112_1314_1516_1718,
        nonce: 0x2122_2324_2526_2728,
        signature: None,
        public_key: None,
    }
}

#[test]
fn transaction_frame_matches_existing_wire_format() {
    let encoded = encode_network_message(&NetworkMessage::Transaction(unsigned_transaction()))
        .expect("transaction frame should encode");
    let expected = hex::decode(
        "5032505f4d53475f5631010000004a54585f49445f5631000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526270102030405060708111213141516171821222324252627280000",
    )
    .expect("valid golden hex");

    assert_eq!(encoded, expected);

    let NetworkMessage::Transaction(decoded) =
        decode_network_message(&expected, &limits()).expect("golden frame should decode")
    else {
        panic!("expected transaction message");
    };

    let original = unsigned_transaction();
    assert_eq!(decoded.from, original.from);
    assert_eq!(decoded.to, original.to);
    assert_eq!(decoded.amount, original.amount);
    assert_eq!(decoded.fee, original.fee);
    assert_eq!(decoded.nonce, original.nonce);
    assert!(decoded.signature.is_none());
    assert!(decoded.public_key.is_none());

    let header_bytes: [u8; FRAME_HEADER_LEN] = expected[..FRAME_HEADER_LEN]
        .try_into()
        .expect("complete frame header");
    let header = decode_frame_header(&header_bytes).expect("valid frame header");
    let frame = NetworkFrame::new(header, expected[FRAME_HEADER_LEN..].to_vec());
    let NetworkMessage::Transaction(decoded) =
        decode_network_frame(&frame, &limits()).expect("parsed frame should decode")
    else {
        panic!("expected transaction message");
    };

    assert_eq!(decoded.txid(), original.txid());
}

#[test]
fn handshake_payload_matches_existing_wire_format() {
    let handshake = Handshake::new(
        1,
        NetworkId(core::array::from_fn(|index| index as u8)),
        NodeId(core::array::from_fn(|index| (index + 32) as u8)),
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8333)),
    );
    let encoded = encode_handshake(&handshake).expect("handshake should encode");
    let expected = hex::decode(
        "48414e445348414b455f563101000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f047f000001208d",
    )
    .expect("valid golden hex");

    assert_eq!(encoded, expected);
}
