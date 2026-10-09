use std::net::SocketAddr;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    app::types::AppError,
    block::Block,
    blockchain::Blockchain,
    network::types::{InboundMessage, NetworkMessage, NodeCommand},
    transaction::{Transaction},
};

pub struct App {
    blockchain: Blockchain,
    app_to_node_sender: Sender<NodeCommand>,
    node_to_app_receiver: Receiver<InboundMessage>,
}

impl App {
    pub fn new(
        blockchain: Blockchain,
        app_to_node_sender: Sender<NodeCommand>,
        node_to_app_receiver: Receiver<InboundMessage>,
    ) -> Self {
        Self {
            blockchain,
            app_to_node_sender,
            node_to_app_receiver,
        }
    }

    pub async fn run(mut self) -> Result<(), AppError> {
        while let Some(message) = self.node_to_app_receiver.recv().await {
            match message.message {
                NetworkMessage::Block(block) => {
                    match self.blockchain.receive_block(block.clone()) {
                        Ok(_) => {
                            println!("Succesfully added new block");
                            self.handle_block_accept(message.source, block).await?
                        }
                        Err(e) => println!("Error while adding block to blockchain: {:?}", e),
                    }
                }
                NetworkMessage::Transaction(transaction) => {
                    if self.blockchain.add_transaction(transaction.clone()) {
                        println!("Succesfully added new transaction");
                        self.handle_transaction_accept(message.source, transaction)
                            .await?
                    }
                },
                NetworkMessage::Handshake(_) => {
                    
                }
            }
        }

        Ok(())
    }

    async fn handle_block_accept(
        &self,
        source: SocketAddr,
        block: Block,
    ) -> Result<(), AppError> {
        let response_command = NodeCommand::BroadcastExcept(source, NetworkMessage::Block(block));
        self.app_to_node_sender
            .send(response_command)
            .await
            .map_err(AppError::BroadcastBlockError)?;

        Ok(())
    }

    async fn handle_transaction_accept(
        &self,
        source: SocketAddr,
        transaction: Transaction,
    ) -> Result<(), AppError> {
        let response_command =
            NodeCommand::BroadcastExcept(source, NetworkMessage::Transaction(transaction));

        self.app_to_node_sender
            .send(response_command)
            .await
            .map_err(AppError::BroadcastTransactionError)?;
        Ok(())
    }
}
