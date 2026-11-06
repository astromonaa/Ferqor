use crate::{
    app::{
        BlockchainService,
        types::{AppCommand, BlockchainServiceError},
    },
    network::wire::NetworkMessage,
};

pub async fn handle_app_command(
    service: &mut BlockchainService,
    command: AppCommand,
) -> Result<(), BlockchainServiceError> {
    match command {
        AppCommand::SubmitBlock(block, source) => match service.blockchain.receive_block(block.clone()) {
            Ok(_) => {
                println!("Succesfully added new block from APP");
                service
                    .service_to_app_sender
                    .send((NetworkMessage::Block(block), source))
                    .await?;
            }
            Err(e) => println!("Error while adding block to blockchain from MAIN: {:?}", e),
        },
        AppCommand::SubmitTransaction(transaction, source) => {
            if service.blockchain.add_transaction(transaction.clone()) {
                println!("Succesfully added new transaction from APP");
                service
                    .service_to_app_sender
                    .send((NetworkMessage::Transaction(transaction), source))
                    .await?;
            }
        }
        AppCommand::MineBlock(miner_address) => {
            match service.mine_pending_transactions(miner_address) {
                Ok(_) => println!("Succesfully sent for minig"),
                Err(e) => println!("Error while mining block, {:?}", e),
            }
        }
    }

    Ok(())
}
