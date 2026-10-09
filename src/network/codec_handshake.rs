use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use crate::{codec::ByteReader, network::{error::{NetworkDecodingError, NetworkEncodingError}, types::{Handshake, NetworkId, NodeId}}};

pub const MAX_HANDSHAKE_PAYLOAD_BYTES: usize= 96;

pub fn encode_handshake(handshake: &Handshake) -> Result<Vec<u8>, NetworkEncodingError> {
    /* 
     * Минимальный capacity для handshake - 77 
     1. Тег HANDSHAKE_V1	12 байт
     2. protocol_version, u8	1 байт
     3. network_id	32 байта
     4. node_id	32 байта
     12 + 1 + 32 + 32 = 77
    */
    let fixed_capacity = 77usize;
    let encoded_addr = encode_socket_addr(handshake.listen_addr)?;
    let total_capacity = fixed_capacity.checked_add(encoded_addr.len()).ok_or(NetworkEncodingError::TypeOverflow)?;
    let mut payload = Vec::with_capacity(total_capacity);

    payload.extend_from_slice(b"HANDSHAKE_V1");
    payload.extend_from_slice(&handshake.protocol_version.to_be_bytes());
    payload.extend_from_slice(&handshake.network_id.to_bytes());
    payload.extend_from_slice(&handshake.node_id.to_bytes());
    payload.extend_from_slice(&encoded_addr);

    Ok(payload)
}

pub fn decode_handshake(bytes: &[u8]) -> Result<Handshake, NetworkDecodingError> {
    let mut reader = ByteReader::new(bytes);
 
    reader.expect_tag(b"HANDSHAKE_V1").map_err(NetworkDecodingError::InvalidTag)?;

    let protocol_version: u8 = reader.read().map_err(NetworkDecodingError::HandshakeDecode)?;
    let network_id = reader.read_exact::<32>().map_err(NetworkDecodingError::HandshakeDecode)?;
    let node_id = reader.read_exact::<32>().map_err(NetworkDecodingError::HandshakeDecode)?;
    let socket_addr = decode_socket_addr(&mut reader)?;

    reader.finish().map_err(NetworkDecodingError::TrailingBytes)?;

    Ok(Handshake { 
        protocol_version, 
        network_id: NetworkId(network_id), 
        node_id: NodeId(node_id), 
        listen_addr: socket_addr,
    })
}

fn encode_socket_addr(addr: SocketAddr) -> Result<Vec<u8>, NetworkEncodingError> {
    /*
    * Payload состоит из
    * 1. Маркер семейства - 1 байт
    * 2. Байты IP (4 или 16)
    * 3. Порт - 2 байта
    *
    * IPv4: 1 + 4 + 2 = 7 байт;
      IPv6: 1 + 16 + 2 = 19 байт.
    */

    /* Размер порта */
    const PORT_SIZE: usize = 2;

    /* (Маркер семейства, количество байтов IP) */
    let (address_family, ip_bytes_len): (u8, usize) = match addr {
        SocketAddr::V4(_) => (4, 4),
        SocketAddr::V6(socket) => {
            if socket.flowinfo() != 0 || socket.scope_id() != 0 {
                return Err(NetworkEncodingError::IpEncodingError);
            };
            (6, 16)
        }
    };

    /* Общий capacity */
    let capacity = 1 + ip_bytes_len + PORT_SIZE;

    let mut payload = Vec::with_capacity(capacity);

    /* Записываем маркер семейства */
    payload.extend_from_slice(&address_family.to_be_bytes());

    /* Записываем байты самого IP */
    match addr.ip() {
        std::net::IpAddr::V4(ip_addr) => {
            payload.extend_from_slice(&ip_addr.octets());
        }
        std::net::IpAddr::V6(ip_addr) => {
            payload.extend_from_slice(&ip_addr.octets());
        }
    };

    /* Записываем порт */
    payload.extend_from_slice(&addr.port().to_be_bytes());

    Ok(payload)
}

fn decode_socket_addr(reader: &mut ByteReader<'_>) -> Result<SocketAddr, NetworkDecodingError> {
    const IP4_LEN: usize = 4;
    const IP6_LEN: usize = 16;

    /* Читаем маркер семейства */
    let address_family: u8 = reader
        .read()
        .map_err(NetworkDecodingError::SocketAddrDecode)?;

    match address_family {
        4 => {
            /* Читаем байты IP */
            let bytes = reader
                .read_exact::<IP4_LEN>()
                .map_err(NetworkDecodingError::SocketAddrDecode)?;
            /* Читаем порт */
            let port: u16 = reader
                .read()
                .map_err(NetworkDecodingError::SocketAddrDecode)?;
            return Ok(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from_octets(bytes),
                port,
            )));
        }
        6 => {
            /* Читаем байты IP */
            let bytes = reader
                .read_exact::<IP6_LEN>()
                .map_err(NetworkDecodingError::SocketAddrDecode)?;
            /* Читаем порт */
            let port: u16 = reader
                .read()
                .map_err(NetworkDecodingError::SocketAddrDecode)?;
            return Ok(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from_octets(bytes),
                port,
                0,
                0,
            )));
        }
        _ => Err(NetworkDecodingError::InvalidIPVersion(address_family)),
    }
}