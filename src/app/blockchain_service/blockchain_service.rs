use std::net::SocketAddr;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    address::Address,
    app::{
        handle_app_command,
        types::{AppCommand, BlockchainServiceError},
    },
    block::Block,
    blockchain::Blockchain,
    ledger::LedgerState,
    network::wire::NetworkMessage,
};

pub struct BlockchainService {
    pub blockchain: Blockchain,
    pub service_to_app_sender: Sender<(NetworkMessage, Option<SocketAddr>)>,
    mining_thread_sender: Sender<(Block, LedgerState)>,
    mining_thread_receiver: Receiver<(Block, LedgerState)>,
    app_to_service_receiver: Receiver<AppCommand>,
}

impl BlockchainService {
    pub fn new(
        blockchain: Blockchain,
        service_to_app_sender: Sender<(NetworkMessage, Option<SocketAddr>)>,
        app_to_service_receiver: Receiver<AppCommand>,
    ) -> Self {
        let (mining_thread_sender, mining_thread_receiver) =
            tokio::sync::mpsc::channel::<(Block, LedgerState)>(100);
        Self {
            blockchain,
            service_to_app_sender,
            mining_thread_receiver,
            mining_thread_sender,
            app_to_service_receiver,
        }
    }

    pub async fn run(&mut self) -> Result<(), BlockchainServiceError> {
        loop {
            tokio::select! {
                Some((message, ledger_state)) = self.mining_thread_receiver.recv() => {
                    let block = self.blockchain.add_block(message, ledger_state)?;
                    self.service_to_app_sender.send((NetworkMessage::Block(block), None)).await?;
                }

                Some(command_from_main) = self.app_to_service_receiver.recv() => {
                    handle_app_command(&mut *self, command_from_main).await?;
                }
            }
        }
    }

    pub fn mine_pending_transactions(
        &mut self,
        miner_address: Address,
    ) -> Result<(), BlockchainServiceError> {
        match self.blockchain.mine_pending_transactions(miner_address) {
            Ok(pending_result) => match pending_result {
                Some((mut candidate, ledger_state)) => {
                    let sender = self.mining_thread_sender.clone();
                    std::thread::spawn(move || {
                        candidate.mine_block();
                        if let Err(error) = sender.blocking_send((candidate, ledger_state)) {
                            eprintln!("Cannot return mining result: {error}");
                        }
                    });
                }
                None => {
                    eprintln!("Transactions list is empty");
                    return Err(BlockchainServiceError::EmptyTransactions)
                }
            },
            Err(e) => {
                eprintln!("Error while mining pending transactions {:?}", e);
                return Err(BlockchainServiceError::MiningError)
            }
        }

        Ok(())
    }
}
