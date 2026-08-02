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

fn main() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None);

    let mut blockchain = blockchain::Blockchain::new(4, initial_config);

    let tx = alice.create_transaction(bob.address, 50, 5, 0);

    println!("tx valid {}", tx.is_valid());

    let success = blockchain.add_transaction(tx);
    println!("tx added {}", success);

    blockchain.mine_pending_transactions(alice.address);

    // println!("blockchain balances {:#?}", blockchain.balances);
    // println!("blockchain valid {}", blockchain.is_valid());
}
