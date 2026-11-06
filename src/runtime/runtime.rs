use tokio::sync::mpsc::{Receiver, channel};

use std::net::SocketAddr;

use crate::{
    address::Balances, app::{
        BlockchainService,
        app::App,
        types::{AppCommand, InboundMessage, NodeCommand},
    }, blockchain::{Blockchain, error::BlockchainError}, config::GenesisConfig, network::{
        codec::NetworkDecodeLimits, node::node::Node, protocol::{Handshake, NetworkId, NodeId}, wire::NetworkMessage,
    }, runtime::types::RuntimeError,
};

pub async fn run_runtime(
    /* Локальный адрес запуска Node */
    listen_addr: SocketAddr,
    /* Публичный адрес используемый для Handshake */
    advertized_addr: SocketAddr,
    node_id: NodeId,
    limits: NetworkDecodeLimits,
    connect_addrs: Vec<SocketAddr>,
    main_to_app_receiver: Receiver<AppCommand>,
    initial_balances: Option<Balances>,
) -> Result<(), RuntimeError> {
    let initial_config = GenesisConfig::get_config(initial_balances, None, 4)?;

    let blockchain =
        Blockchain::new(initial_config).map_err(BlockchainError::InitializationError)?;

    let network_id = NetworkId(blockchain.blocks()[0].hash.0);

    /* Канал взаимодействи BlockchainService -> App */
    let (service_to_app_sender, service_to_app_receiver) = channel::<(NetworkMessage, Option<SocketAddr>)>(100);

    /* Канал взаимодействи App -> BlockchainService */
    let (app_to_service_sender, app_to_service_receiver) = channel::<AppCommand>(100);

    let mut blockchain_service = BlockchainService::new(blockchain, service_to_app_sender, app_to_service_receiver);

    /* канал Node → Application */
    let (node_to_app_sender, node_to_app_receiver) = channel::<InboundMessage>(100);

    /* канал Application → Node. */
    let (app_to_node_sender, app_to_node_receiver) = channel::<NodeCommand>(100);

    let app = App::new(
        app_to_node_sender,
        node_to_app_receiver,
        main_to_app_receiver,
        service_to_app_receiver,
        app_to_service_sender
    );

    let handshake = Handshake::new(1, network_id, node_id, advertized_addr);
    let mut node = Node::new(node_to_app_sender, app_to_node_receiver, handshake);

    tokio::try_join!(
        async move {
            node.run(&limits, listen_addr, &connect_addrs).await?;
            Ok::<(), RuntimeError>(())
        },
        async {
            app.run().await?;
            Ok::<(), RuntimeError>(())
        },
        async {
            blockchain_service.run().await?;
            Ok::<(), RuntimeError>(())
        }
    )?;

    Ok(())
}
