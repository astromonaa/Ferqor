//! Тесты границ frame, сохраняющие поведение прежнего `network::transport`.

use tokio::io::{AsyncWriteExt, duplex};

use crate::{
    block::BlockDecodeLimits,
    network::{codec::NetworkDecodeLimits, error::TransportError, transport::TransportClient},
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

fn frame(message_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(15 + payload.len());
    frame.extend_from_slice(b"P2P_MSG_V1");
    frame.push(message_type);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

#[tokio::test]
async fn read_frame_preserves_two_consecutive_boundaries() {
    let first = frame(1, &[1, 2, 3]);
    let second = frame(3, &[4, 5]);
    let (mut writer, mut reader) = duplex(first.len() + second.len());

    writer.write_all(&first).await.expect("first frame write");
    writer.write_all(&second).await.expect("second frame write");

    assert_eq!(
        TransportClient::read_frame(&mut reader, &limits())
            .await
            .expect("first frame read"),
        first
    );
    assert_eq!(
        TransportClient::read_frame(&mut reader, &limits())
            .await
            .expect("second frame read"),
        second
    );
}

#[tokio::test]
async fn read_frame_rejects_oversized_payload_before_reading_body() {
    let mut header = Vec::from(&b"P2P_MSG_V1"[..]);
    header.push(1);
    header.extend_from_slice(&1_001u32.to_be_bytes());
    let (mut writer, mut reader) = duplex(header.len());
    writer.write_all(&header).await.expect("header write");

    let error = TransportClient::read_frame(&mut reader, &limits())
        .await
        .expect_err("oversized frame should fail");

    assert!(matches!(error, TransportError::InvalidPayloadSize));
}
