use std::io::Error;

use crate::{block::EncodingError, codec::DecodingError };

#[derive(Debug)]
pub enum NetworkEncodingError {
    BlockError(EncodingError),
    PayloadLengthOverflow { actual: usize },
    FrameLengthOverflow { actual: usize },
    TypeOverflow,
    IpEncodingError
}

#[derive(Debug)]
pub enum NetworkDecodingError {
    ReadMessageType(DecodingError),
    DecodeMessageType { actual: u8 },
    ReadPayloadLen(DecodingError),
    ReadPayload(DecodingError),
    TrailingBytes(DecodingError),
    BlockDecode(DecodingError),
    TransactionDecode(DecodingError),
    InvalidTag(DecodingError),
    PayloadTooLarge { actual: usize, max: usize },
    SocketAddrDecode(DecodingError),
    InvalidIPVersion(u8),
    HandshakeDecode(DecodingError)
}

#[derive(Debug)]
pub enum TransportError {
    ReadBytesError(Error),
    WriteBytesError(Error),
    InvalidFrame,
    DecodingError(NetworkDecodingError),
    InvalidTag,
    UnknownMessageType,
    InvalidPayloadSize
}