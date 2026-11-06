use std::net::SocketAddr;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    app::types::{AppCommand, AppError, InboundMessage, NodeCommand},
    network::wire::NetworkMessage,
};

pub struct App {
    pub app_to_node_sender: Sender<NodeCommand>,
    node_to_app_receiver: Receiver<InboundMessage>,
    main_to_app_receiver: Receiver<AppCommand>,
    pub service_to_app_receiver: Receiver<(NetworkMessage, Option<SocketAddr>)>,
    pub app_to_service_sender: Sender<AppCommand>,
}

impl App {
    pub fn new(
        app_to_node_sender: Sender<NodeCommand>,
        node_to_app_receiver: Receiver<InboundMessage>,
        main_to_app_receiver: Receiver<AppCommand>,
        service_to_app_receiver: Receiver<(NetworkMessage, Option<SocketAddr>)>,
        app_to_service_sender: Sender<AppCommand>,
    ) -> Self {
        Self {
            app_to_node_sender,
            node_to_app_receiver,
            main_to_app_receiver,
            service_to_app_receiver,
            app_to_service_sender,
        }
    }

    pub async fn run(mut self) -> Result<(), AppError> {
        loop {
            tokio::select! {
                received_from_node = self.node_to_app_receiver.recv() => {
                    let Some(message) = received_from_node else {
                        return Ok(());
                    };
                    self.handle_node_message(message).await?;
                }

                Some(command_from_main) = self.main_to_app_receiver.recv() => {
                    self.app_to_service_sender.send(command_from_main).await?;
                }
                Some((message, source)) = self.service_to_app_receiver.recv() => self.handle_service_message(message, source).await?
            }
        }
    }

    async fn handle_node_message(&mut self, message: InboundMessage) -> Result<(), AppError> {
        let source = Some(message.source);
        match message.message {
            NetworkMessage::Block(block) => match self
                .app_to_service_sender
                .send(AppCommand::SubmitBlock(block.clone(), source))
                .await
            {
                Ok(_) => {
                    println!("Succesfully send new block to blockchain service");
                }
                Err(e) => println!("Error while sending block to blockchain service: {:?}", e),
            },
            NetworkMessage::Transaction(transaction) => match self
                .app_to_service_sender
                .send(AppCommand::SubmitTransaction(transaction.clone(), source))
                .await
            {
                Ok(_) => {
                    println!("Succesfully send new transaction to blockchain service");
                }
                Err(e) => println!("Error while sending transaction to blockchain service {:?}", e),
            },
            NetworkMessage::Handshake(_) => {}
        }

        Ok(())
    }

    async fn handle_service_message(
        &self,
        message: NetworkMessage,
        source: Option<SocketAddr>,
    ) -> Result<(), AppError> {
        let node_command = if let Some(s) = source {
            NodeCommand::BroadcastExcept(s, message)
        } else {
            NodeCommand::Broadcast(message)
        };

        self.app_to_node_sender.send(node_command).await?;
        Ok(())
    }
}
