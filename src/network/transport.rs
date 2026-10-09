
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::network::codec::NetworkDecodeLimits;
use crate::network::codec_handshake::MAX_HANDSHAKE_PAYLOAD_BYTES;
use crate::network::error::TransportError;
use crate::network::types::MessageType;


const META_INFO_SIZE: usize = 15;
const META_INFO_HEADER_SIZE: usize = 10;
const PAYLOAD_LEN_VALUE_SIZE: usize = 4; // 4 байта - длина пейлоада (тип u32 - 4 байта) 

pub struct TransportClient;

fn check_header(meta_buffer: &[u8; META_INFO_SIZE]) -> Result<(), TransportError> {
    let header = &meta_buffer[0..META_INFO_HEADER_SIZE];

    if header != b"P2P_MSG_V1" {
        return Err(TransportError::InvalidTag);
    }

    Ok(())
}

fn get_message_type(meta_buffer: &[u8; META_INFO_SIZE]) -> Result<MessageType, TransportError> {
    let message_type: MessageType = meta_buffer[10]
        .try_into()
        .map_err(|_| TransportError::UnknownMessageType)?;
    Ok(message_type)
}

fn get_payload_len(meta_buffer: &[u8; META_INFO_SIZE]) -> Result<usize, TransportError> {
    let mut payload_len_bytes: [u8; PAYLOAD_LEN_VALUE_SIZE] = [0u8; PAYLOAD_LEN_VALUE_SIZE];
    payload_len_bytes.copy_from_slice(&meta_buffer[11..META_INFO_SIZE]);
    let payload_len = u32::from_be_bytes(payload_len_bytes);

    let payload_len = usize::try_from(payload_len).map_err(|_| TransportError::InvalidFrame)?;

    Ok(payload_len)
}

async fn get_result_frame<R: AsyncRead + Unpin>(
    stream: &mut R,
    meta_buffer: &[u8; META_INFO_SIZE],
    payload_len: usize,
) -> Result<Vec<u8>, TransportError> {
    let frame_len = payload_len
        .checked_add(META_INFO_SIZE)
        .ok_or(TransportError::InvalidFrame)?;

    let mut frame = vec![0u8; frame_len];

    frame[0..META_INFO_SIZE].copy_from_slice(meta_buffer);

    stream
        .read_exact(&mut frame[META_INFO_SIZE..frame_len])
        .await
        .map_err(TransportError::ReadBytesError)?;

    Ok(frame)
}

impl TransportClient {
    pub async fn read_frame<R: AsyncRead + Unpin>(
        stream: &mut R,
        limits: &NetworkDecodeLimits,
    ) -> Result<Vec<u8>, TransportError> {
        let mut meta_buffer = [0u8; META_INFO_SIZE];
        stream
            .read_exact(&mut meta_buffer)
            .await
            .map_err(TransportError::ReadBytesError)?;

        check_header(&meta_buffer)?;

        let message_type = get_message_type(&meta_buffer)?;

        let current_limit = match message_type {
            MessageType::Block => limits.block_limits.max_block_bytes,
            MessageType::Transaction => limits.max_tx_bytes,
            MessageType::Handshake => MAX_HANDSHAKE_PAYLOAD_BYTES
        };

        let payload_len = get_payload_len(&meta_buffer)?;

        if payload_len > current_limit {
            return Err(TransportError::InvalidPayloadSize);
        }

        get_result_frame(stream, &meta_buffer, payload_len).await
    }

    pub async fn write_frame<W: AsyncWrite + Unpin>(
        stream: &mut W,
        bytes: &[u8],
    ) -> Result<(), TransportError> {
        // 1. Отправляем все байты сообщения
        stream
            .write_all(bytes)
            .await
            .map_err(TransportError::WriteBytesError)?;

        Ok(())
    }
}
