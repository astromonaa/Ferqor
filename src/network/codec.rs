//! Кодирование сетевых сообщений и декодирование их payload.
//!
//! Разбор frame для готового byte slice сохраняет прежний публичный метод
//! `decode_network_message`, а потоковый путь принимает уже разобранный `NetworkFrame`.

use crate::{
    block::{BlockDecodeLimits, decode_block, encode_block},
    codec::ByteReader,
    network::{
        codec_handshake::{MAX_HANDSHAKE_PAYLOAD_BYTES, decode_handshake, encode_handshake},
        error::{NetworkDecodingError, NetworkEncodingError},
        frame::{FRAME_HEADER_LEN, FRAME_TAG, NetworkFrame},
        wire::{MessageType, NetworkMessage},
    },
    transaction::{decode_transaction, encode_transaction},
};

#[derive(Debug, Clone, Copy)]
pub struct NetworkDecodeLimits {
    pub max_tx_bytes: usize,
    pub block_limits: BlockDecodeLimits,
}

impl NetworkDecodeLimits {
    /// Сохраняет выбор лимита из прежних `network::codec` и `network::transport`.
    pub(crate) fn max_payload_bytes(&self, message_type: MessageType) -> usize {
        match message_type {
            MessageType::Block => self.block_limits.max_block_bytes,
            MessageType::Transaction => self.max_tx_bytes,
            MessageType::Handshake => MAX_HANDSHAKE_PAYLOAD_BYTES,
        }
    }
}

impl NetworkMessage {
    pub fn message_type(&self) -> MessageType {
        match self {
            Self::Transaction(_) => MessageType::Transaction,
            Self::Block(_) => MessageType::Block,
            Self::Handshake(_) => MessageType::Handshake,
        }
    }

    fn payload(&self) -> Result<Vec<u8>, NetworkEncodingError> {
        match self {
            Self::Transaction(transaction) => Ok(encode_transaction(transaction)),
            Self::Block(block) => encode_block(block).map_err(NetworkEncodingError::BlockError),
            Self::Handshake(handshake) => encode_handshake(handshake),
        }
    }
}

/// Сохраняет прежний публичный метод `network::codec::encode_network_message`.
pub fn encode_network_message(message: &NetworkMessage) -> Result<Vec<u8>, NetworkEncodingError> {
    let message_type: u8 = message.message_type().into();
    let message_payload = message.payload()?;
    let payload_len: u32 = message_payload.len().try_into().map_err(|_| {
        NetworkEncodingError::PayloadLengthOverflow {
            actual: message_payload.len(),
        }
    })?;
    let capacity = message_payload.len().checked_add(FRAME_HEADER_LEN).ok_or(
        NetworkEncodingError::FrameLengthOverflow {
            actual: payload_len as usize,
        },
    )?;
    let mut frame = Vec::with_capacity(capacity);

    frame.extend_from_slice(FRAME_TAG);
    frame.push(message_type);
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.extend_from_slice(&message_payload);

    Ok(frame)
}

/// Сохраняет прежний публичный метод `network::codec::decode_network_message`
/// и точную классификацию ошибок неполного или некорректного заголовка.
pub fn decode_network_message(
    bytes: &[u8],
    limits: &NetworkDecodeLimits,
) -> Result<NetworkMessage, NetworkDecodingError> {
    let mut reader = ByteReader::new(bytes);
    reader
        .expect_tag(FRAME_TAG)
        .map_err(NetworkDecodingError::InvalidTag)?;
    let message_type_raw = reader
        .read::<u8>()
        .map_err(NetworkDecodingError::ReadMessageType)?;
    let message_type: MessageType = message_type_raw.try_into()?;
    let payload_len = reader
        .read::<u32>()
        .map_err(NetworkDecodingError::ReadPayloadLen)? as usize;

    validate_payload_len(message_type, payload_len, limits)?;

    let payload = reader
        .read_slice(payload_len)
        .map_err(NetworkDecodingError::ReadPayload)?;
    reader
        .finish()
        .map_err(NetworkDecodingError::TrailingBytes)?;

    decode_payload(message_type, payload, limits)
}

/// Декодирует frame, заголовок которого уже был разобран в `network::transport`.
pub(crate) fn decode_network_frame(
    frame: &NetworkFrame,
    limits: &NetworkDecodeLimits,
) -> Result<NetworkMessage, NetworkDecodingError> {
    validate_payload_len(frame.message_type(), frame.payload().len(), limits)?;
    decode_payload(frame.message_type(), frame.payload(), limits)
}

fn validate_payload_len(
    message_type: MessageType,
    payload_len: usize,
    limits: &NetworkDecodeLimits,
) -> Result<(), NetworkDecodingError> {
    let max = limits.max_payload_bytes(message_type);

    if payload_len > max {
        return Err(NetworkDecodingError::PayloadTooLarge {
            actual: payload_len,
            max,
        });
    }

    Ok(())
}

fn decode_payload(
    message_type: MessageType,
    payload: &[u8],
    limits: &NetworkDecodeLimits,
) -> Result<NetworkMessage, NetworkDecodingError> {
    match message_type {
        MessageType::Block => decode_block(payload, &limits.block_limits)
            .map(NetworkMessage::Block)
            .map_err(NetworkDecodingError::BlockDecode),
        MessageType::Transaction => decode_transaction(payload)
            .map(NetworkMessage::Transaction)
            .map_err(NetworkDecodingError::TransactionDecode),
        MessageType::Handshake => decode_handshake(payload).map(NetworkMessage::Handshake),
    }
}
