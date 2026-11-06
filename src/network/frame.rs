//! Заголовок и границы сетевого frame, выделенные из прежних
//! `network::codec` и `network::transport`.
//!
//! Тег, длина заголовка, номера типов и порядок байтов сохранены без изменений.

use crate::network::{error::TransportError, wire::MessageType};

pub(crate) const FRAME_TAG: &[u8; 10] = b"P2P_MSG_V1";
pub(crate) const FRAME_HEADER_LEN: usize = 15;
const MESSAGE_TYPE_OFFSET: usize = 10;
const PAYLOAD_LEN_OFFSET: usize = 11;

#[derive(Debug, Clone, Copy)]
pub(crate) struct FrameHeader {
    message_type: MessageType,
    payload_len: usize,
}

impl FrameHeader {
    pub(crate) fn message_type(&self) -> MessageType {
        self.message_type
    }

    pub(crate) fn payload_len(&self) -> usize {
        self.payload_len
    }
}

#[derive(Debug)]
pub(crate) struct NetworkFrame {
    header: FrameHeader,
    payload: Vec<u8>,
}

impl NetworkFrame {
    pub(crate) fn new(header: FrameHeader, payload: Vec<u8>) -> Self {
        Self { header, payload }
    }

    pub(crate) fn message_type(&self) -> MessageType {
        self.header.message_type()
    }

    pub(crate) fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Восстанавливает прежний результат `TransportClient::read_frame`.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        let payload_len = self.header.payload_len() as u32;
        let mut bytes = Vec::with_capacity(FRAME_HEADER_LEN + self.payload.len());
        bytes.extend_from_slice(FRAME_TAG);
        bytes.push(self.header.message_type().into());
        bytes.extend_from_slice(&payload_len.to_be_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }
}

/// Разбирает заголовок логикой прежних helper-функций из `network::transport`.
pub(crate) fn decode_frame_header(
    bytes: &[u8; FRAME_HEADER_LEN],
) -> Result<FrameHeader, TransportError> {
    if &bytes[..FRAME_TAG.len()] != FRAME_TAG {
        return Err(TransportError::InvalidTag);
    }

    let message_type = bytes[MESSAGE_TYPE_OFFSET]
        .try_into()
        .map_err(|_| TransportError::UnknownMessageType)?;
    let payload_len_bytes = bytes[PAYLOAD_LEN_OFFSET..FRAME_HEADER_LEN]
        .try_into()
        .map_err(|_| TransportError::InvalidFrame)?;
    let payload_len = usize::try_from(u32::from_be_bytes(payload_len_bytes))
        .map_err(|_| TransportError::InvalidFrame)?;

    Ok(FrameHeader {
        message_type,
        payload_len,
    })
}
