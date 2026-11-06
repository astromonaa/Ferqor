use crate::address::Balances;
use crate::app::types::{AppCommand};
use crate::blockchain::error::StartupError;
use crate::config::{get_network_decode_limits, load_app_config};
use crate::utils::run_local_commands;
use crate::wallet::Wallet;
use std::collections::HashMap;

use tokio::runtime::Runtime;

mod address;
mod app;

mod block;
mod blockchain;
mod chain;
mod codec;
mod config;
mod consensus;
mod ledger;
mod mempool;
mod mining;
mod network;
mod runtime;
mod transaction;
mod wallet;
mod utils;

use runtime::runtime::run_runtime;
use tokio::sync::mpsc::{channel};

fn main() -> Result<(), StartupError> {
    let config = load_app_config()?;
    let network_limits = get_network_decode_limits();
    let tokio_runtime = Runtime::new()?;

    let alice = Wallet::from_private_key([0u8; 32]);
    let bob = Wallet::new();

    let initial_balances: Option<Balances> = Some(HashMap::from([(alice.address, 100)]));

    /* канал Main → Application */
    let (main_to_app_sender, main_to_app_receiver) = channel::<AppCommand>(100);

    tokio_runtime.block_on(async {
        tokio::try_join!(
            async {
                run_runtime(
                    config.listen_addr,
                    config.advertize_addr,
                    config.node_id.unwrap(),
                    network_limits,
                    config.connect,
                    main_to_app_receiver,
                    initial_balances,
                )
                .await
                .map_err(StartupError::from)
            },
            run_local_commands(alice, bob, &main_to_app_sender, config.initial_sender)
        )?;
        Ok::<(), StartupError>(())
    })?;

    Ok(())
}