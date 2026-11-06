use dotenv::Error;
use std::{
    env,
    io::{self},
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use crate::network::protocol::NodeId;

#[derive(Debug)]
pub struct RuntimeConfig {
    pub listen_addr: SocketAddr,
    pub advertize_addr: SocketAddr,
    pub node_id: Option<NodeId>,
    pub connect: Vec<SocketAddr>,
    pub initial_sender: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::from_octets([0, 0, 0, 0])), 9000),
            advertize_addr: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from_octets([127, 0, 0, 1])),
                9000,
            ),
            node_id: None,
            connect: Vec::new(),
            initial_sender: false,
        }
    }
}

fn to_error(text: String) -> Error {
    Error::Io(io::Error::new(io::ErrorKind::InvalidData, text))
}

pub fn load_app_config() -> Result<RuntimeConfig, Error> {
    let Some(env_path) =
        std::env::args().find_map(|arg| arg.strip_prefix("--env-file=").map(String::from))
    else {
        return Err(Error::LineParse(".env file name required".to_string(), 1));
    };

    dotenv::from_filename(env_path)?;
    let mut config = RuntimeConfig::default();

    for (key, value) in env::vars() {
        match key.as_str() {
            "LISTEN_ADDR" => {
                config.listen_addr = value
                    .parse::<SocketAddr>()
                    .map_err(|_| to_error(format!("Invalid LISTEN_ADDR value")))?;
            }
            "ADVERTIZE_ADDR" => {
                config.advertize_addr = value
                    .parse::<SocketAddr>()
                    .map_err(|_| to_error(format!("Invalid ADVERTIZE_ADDR value")))?;
            }
            "NODE_ID" => {
                let mut bytes = [0u8; 32];
                hex::decode_to_slice(value, &mut bytes)
                    .map_err(|e| to_error(format!("Invalid NODE_ID {}", e.to_string())))?;
                config.node_id = Some(NodeId(bytes));
            }
            "CONNECT" => {
                /* По умолчанию пустой массив */
                if !value.trim().is_empty() {
                    config.connect = value
                        .split(",")
                        .map(|val| val.trim().parse::<SocketAddr>())
                        .collect::<Result<Vec<SocketAddr>, _>>()
                        .map_err(|e| to_error(format!("Invalid CONNECT IDS {}", e.to_string())))?;
                }
            }
            "INITIAL_SENDER" => {
                config.initial_sender = value.trim().parse::<bool>().unwrap_or(false)
            }
            _ => {}
        }
    }

    if config.node_id.is_none() {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid NODE_ID"),
        )));
    }

    Ok(config)
}
