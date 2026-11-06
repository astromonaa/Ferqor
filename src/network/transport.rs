//! Потоковое чтение и запись сетевых frame.
//!
//! Разбор заголовка перенесён из helper-функций этого модуля в `network::frame`.
//! Публичные методы `read_frame` и `write_frame` сохраняют прежние сигнатуры.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::network::{
    codec::NetworkDecodeLimits,
    error::TransportError,
    frame::{FRAME_HEADER_LEN, NetworkFrame, decode_frame_header},
};

pub struct TransportClient;

impl TransportClient {
    /// Сохраняет прежний результат: полный frame с заголовком и payload.
    pub async fn read_frame<R: AsyncRead + Unpin>(
        stream: &mut R,
        limits: &NetworkDecodeLimits,
    ) -> Result<Vec<u8>, TransportError> {
        Self::read_network_frame(stream, limits)
            .await
            .map(NetworkFrame::into_bytes)
    }

    /// Читает frame для `Peer`, передавая уже разобранный заголовок в codec.
    pub(crate) async fn read_network_frame<R: AsyncRead + Unpin>(
        stream: &mut R,
        limits: &NetworkDecodeLimits,
    ) -> Result<NetworkFrame, TransportError> {
        let mut header_bytes = [0u8; FRAME_HEADER_LEN];
        stream
            .read_exact(&mut header_bytes)
            .await
            .map_err(TransportError::ReadBytesError)?;

        let header = decode_frame_header(&header_bytes)?;
        let payload_len = header.payload_len();
        let current_limit = limits.max_payload_bytes(header.message_type());

        if payload_len > current_limit {
            return Err(TransportError::InvalidPayloadSize);
        }

        payload_len
            .checked_add(FRAME_HEADER_LEN)
            .ok_or(TransportError::InvalidFrame)?;

        let mut payload = vec![0u8; payload_len];
        stream
            .read_exact(&mut payload)
            .await
            .map_err(TransportError::ReadBytesError)?;

        Ok(NetworkFrame::new(header, payload))
    }

    pub async fn write_frame<W: AsyncWrite + Unpin>(
        stream: &mut W,
        bytes: &[u8],
    ) -> Result<(), TransportError> {
        stream
            .write_all(bytes)
            .await
            .map_err(TransportError::WriteBytesError)?;

        Ok(())
    }
}
