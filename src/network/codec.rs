
use crate::{
    block::{BlockDecodeLimits, decode_block, encode_block}, codec::ByteReader, network::{
        codec_handshake::{MAX_HANDSHAKE_PAYLOAD_BYTES, decode_handshake, encode_handshake}, error::{NetworkDecodingError, NetworkEncodingError}, types::{MessageType, NetworkMessage},
    }, transaction::{decode_transaction, encode_transaction},
};

#[derive(Debug, Clone, Copy)]
pub struct NetworkDecodeLimits {
    pub max_tx_bytes: usize,
    pub block_limits: BlockDecodeLimits,
}

impl TryFrom<u8> for MessageType {
    type Error = NetworkDecodingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::Transaction),
            2 => Ok(MessageType::Block),
            3 => Ok(MessageType::Handshake),
            actual => Err(NetworkDecodingError::DecodeMessageType { actual }),
        }
    }
}

impl From<MessageType> for u8 {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::Transaction => 1,
            MessageType::Block => 2,
            MessageType::Handshake => 3,
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


/*  Encoding */
pub fn encode_network_message(message: &NetworkMessage) -> Result<Vec<u8>, NetworkEncodingError> {
    let message_type: u8 = message.message_type().into();

    /*  - 10 байтов звголовка - P2P_MSG_V1
        - 1 байт - тип сообщения (Block || Transaction),
        - 4 байта - длина пейлоада (тип u32 - 4 байта)
    */
    const MIN_CAPACITY: usize = 15;

    let message_payload = message.payload()?;

    let payload_len: u32 = message_payload.len().try_into().map_err(|_| {
        NetworkEncodingError::PayloadLengthOverflow {
            actual: message_payload.len(),
        }
    })?;

    let capacity = message_payload.len().checked_add(MIN_CAPACITY).ok_or({
        NetworkEncodingError::FrameLengthOverflow {
            actual: payload_len as usize,
        }
    })?;

    let mut frame: Vec<u8> = Vec::with_capacity(capacity);

    frame.extend_from_slice(b"P2P_MSG_V1");
    frame.extend_from_slice(&message_type.to_be_bytes());
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.extend_from_slice(&message_payload);

    Ok(frame)
}

/*  Decoding */
pub fn decode_network_message(
    bytes: &[u8],
    limits: &NetworkDecodeLimits,
) -> Result<NetworkMessage, NetworkDecodingError> {
    let mut reader = ByteReader::new(bytes);
    reader
        .expect_tag(b"P2P_MSG_V1")
        .map_err(|e| NetworkDecodingError::InvalidTag(e))?;

    let message_type_raw = reader
        .read::<u8>()
        .map_err(|e| NetworkDecodingError::ReadMessageType(e))?;

    let message_type: MessageType = message_type_raw.try_into()?;

    let payload_len = reader
        .read::<u32>()
        .map_err(|e| NetworkDecodingError::ReadPayloadLen(e))?;

    let current_limit = match message_type {
        MessageType::Block => limits.block_limits.max_block_bytes,
        MessageType::Transaction => limits.max_tx_bytes,
        MessageType::Handshake => MAX_HANDSHAKE_PAYLOAD_BYTES,
    };

    if (payload_len as usize) > current_limit {
        return Err(NetworkDecodingError::PayloadTooLarge {
            actual: payload_len as usize,
            max: current_limit,
        });
    }

    let payload = reader
        .read_slice(payload_len as usize)
        .map_err(|e| NetworkDecodingError::ReadPayload(e))?;

    reader
        .finish()
        .map_err(|e| NetworkDecodingError::TrailingBytes(e))?;

    match message_type {
        MessageType::Block => {
            let decoded_block = decode_block(payload, &limits.block_limits)
                .map_err(|e| NetworkDecodingError::BlockDecode(e))?;
            Ok(NetworkMessage::Block(decoded_block))
        }
        MessageType::Transaction => {
            let decoded_transaction = decode_transaction(payload)
                .map_err(|e| NetworkDecodingError::TransactionDecode(e))?;
            Ok(NetworkMessage::Transaction(decoded_transaction))
        },
        MessageType::Handshake => {
            let decoded_handshake = decode_handshake(payload)?;
            Ok(NetworkMessage::Handshake(decoded_handshake))
        }
    }
}
