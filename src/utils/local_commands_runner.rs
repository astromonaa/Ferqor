use std::{time::Duration};

use tokio::sync::mpsc::Sender;

use crate::{
    address::Address, app::types::AppCommand, blockchain::error::StartupError, wallet::Wallet,
};

pub async fn run_local_commands(
    alice: Wallet,
    bob: Wallet,
    main_to_app_sender: &Sender<AppCommand>,
    initial_sender: bool,
) -> Result<(), StartupError> {
    if !initial_sender {
        return Ok(());
    }
    let miner_address = alice.address;
    try_send_transaction(alice, bob, main_to_app_sender).await?;
    try_send_block(main_to_app_sender, miner_address).await?;

    Ok(())
}

async fn try_send_transaction(
    alice: Wallet,
    bob: Wallet,
    main_to_app_sender: &Sender<AppCommand>,
) -> Result<(), StartupError> {
    tokio::time::sleep(Duration::from_secs(5)).await;
    let tx = alice.create_transaction(bob.address, 50, 5, 0);
    main_to_app_sender
        .send(AppCommand::SubmitTransaction(tx, None))
        .await?;
    Ok(())
}

async fn try_send_block(
    main_to_app_sender: &Sender<AppCommand>,
    miner_address: Address,
) -> Result<(), StartupError> {
    tokio::time::sleep(Duration::from_secs(5)).await;
    main_to_app_sender
        .send(AppCommand::MineBlock(miner_address))
        .await?;
    Ok(())
}
