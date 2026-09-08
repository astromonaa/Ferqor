use crate::blockchain::error::BlockchainError;
use crate::config::GenesisConfig;
use crate::wallet::Wallet;
use std::collections::HashMap;

mod address;
mod block;
mod blockchain;
mod config;
mod mempool;
mod transaction;
mod wallet;
mod codec;

fn main() -> Result<(), BlockchainError> {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)?;

    let mut blockchain = blockchain::Blockchain::new(initial_config).map_err(BlockchainError::InitializationError)?;

    let tx = alice.create_transaction(bob.address, 50, 5, 0);

    println!("tx valid {}", tx.is_valid());

    let success = blockchain.add_transaction(tx);
    println!("tx added {}", success);

    blockchain.mine_pending_transactions(alice.address).map_err(BlockchainError::Mining)?;

    Ok(())
    // println!("blockchain balances {:#?}", blockchain.balances);
    // println!("blockchain valid {}", blockchain.is_valid());
}
