use tokio::sync::mpsc::channel;

use std::{collections::HashMap, net::SocketAddr};

use crate::{
    app::app::App, blockchain::{Blockchain, error::BlockchainError}, config::GenesisConfig, network::{codec::NetworkDecodeLimits, node::node::Node, types::{Handshake, InboundMessage, NetworkId, NodeCommand, NodeId}}, runtime::types::RuntimeError,
};

pub async fn run_runtime(
    /* Локальный адрес запуска Node */
    listen_addr: &str,
    /* Публичный адрес используемый для Handshake */
    advertized_addr: SocketAddr,
    node_id: NodeId,
    limits: &NetworkDecodeLimits,
) -> Result<(), RuntimeError> {
    let initial_config = GenesisConfig::get_config(Some(HashMap::new()), None, 4)?;

    let blockchain =
        Blockchain::new(initial_config).map_err(BlockchainError::InitializationError)?;

    let network_id = NetworkId(blockchain.chain[0].hash.0);
    
    /* канал Node → Application */
    let (node_to_app_sender, node_to_app_receiver) = channel::<InboundMessage>(100);

    /* канал Application → Node. */
    let (app_to_node_sender, app_to_node_receiver) = channel::<NodeCommand>(100);

    let app = App::new(blockchain, app_to_node_sender, node_to_app_receiver);

    let handshake = Handshake::new(1, network_id,node_id, advertized_addr);
    let mut node = Node::new(node_to_app_sender, app_to_node_receiver, handshake);

    tokio::try_join!(
        async move {
            node.run(limits, listen_addr, &[]).await?;
            Ok::<(), RuntimeError>(())
        },
        async {
            app.run().await?;
            Ok::<(), RuntimeError>(())
        }
    )?;

    Ok(())
}
