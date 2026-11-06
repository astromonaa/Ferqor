use std::collections::HashMap;

use crate::{
    address::Address, blockchain::Blockchain, config::GenesisConfig, transaction::Transaction,
    wallet::Wallet,
};
use ed25519_dalek::Signature;

#[test]
fn transaction_fails_if_amount_changes() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);
    tx.amount = 500;

    assert!(!tx.is_valid());
}

#[test]
fn transaction_fails_if_receiver_changes() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let fake = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.to = fake.address;

    assert!(!tx.is_valid());
}

#[test]
fn transaction_fails_if_sender_changes() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let fake = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.from = fake.address;

    assert!(!tx.is_valid());
}

#[test]
fn transaction_fails_if_public_key_is_replaced() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let fake = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.public_key = Some(fake.public_key);

    assert!(!tx.is_valid());
}

#[test]
fn transaction_is_valid() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let tx = alice.create_transaction(bob.address, 100, 5, 1);

    assert!(tx.is_valid());
}

#[test]
fn test_fake_signature() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    if let Some(signature) = &mut tx.signature {
        let mut bytes = signature.to_bytes();
        bytes[0] ^= 1;

        tx.signature = Some(Signature::from_bytes(&bytes));
    }

    assert!(!tx.is_valid());
}

#[test]
fn test_missing_signature() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.signature = None;

    assert!(!tx.is_valid());
}

#[test]
fn test_missing_public_key() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.public_key = None;

    assert!(!tx.is_valid());
}

#[test]
fn should_accept_first_transaction() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();

    let tx = alice.create_transaction(bob.address, 50, 5, 0);

    let success = blockchain.add_transaction(tx);

    assert_eq!(success, true);
}

#[test]
fn genesis_config_threads_consensus_params_into_blockchain() {
    let initial_config =
        GenesisConfig::get_config(None, Some(2_000), 4).expect("valid genesis config");

    let blockchain = Blockchain::new(initial_config).unwrap();

    assert_eq!(blockchain.mining_reward(), 50);
    assert_eq!(blockchain.max_block_weight(), 2_000);
    assert_eq!(blockchain.consensus_params().target_block_time(), 10);
    assert_eq!(
        blockchain
            .consensus_params()
            .difficulty_adjustment_interval(),
        10
    );
}

fn create_transactions(wallet: &Wallet, count: u64, to: Address) -> Vec<Transaction> {
    let mut transactions = Vec::new();
    for i in 0..count {
        let tx = wallet.create_transaction(to, 10, 1, i);
        transactions.push(tx);
    }
    transactions
}

fn add_transactions(blockchain: &mut Blockchain, txs: Vec<Transaction>) {
    for tx in txs {
        let _success = blockchain.add_transaction(tx);
        assert_eq!(_success, true)
    }
}

#[test]
fn should_accept_transaction_with_valid_nonce() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();

    let txs = create_transactions(&alice, 5, bob.address);
    add_transactions(&mut blockchain, txs);

    blockchain
        .mine_pending_transactions(alice.address)
        .expect("mining should succeed");

    let tx5 = alice.create_transaction(bob.address, 5, 1, 5);

    let _success6 = blockchain.add_transaction(tx5);

    assert_eq!(_success6, true);
}

#[test]
fn should_accept_transaction_while_there_are_pending_transactions() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();

    let txs = create_transactions(&alice, 5, bob.address);
    add_transactions(&mut blockchain, txs);

    blockchain
        .mine_pending_transactions(alice.address)
        .expect("mining should succeed");

    let tx6 = alice.create_transaction(bob.address, 10, 1, 5);
    let tx7 = alice.create_transaction(bob.address, 10, 1, 6);

    add_transactions(&mut blockchain, [tx6, tx7].to_vec());

    let tx8 = alice.create_transaction(bob.address, 10, 1, 7);
    let _success8 = blockchain.add_transaction(tx8);

    assert_eq!(_success8, true);
}

#[test]
fn should_not_accept_transaction_with_skipped_nonce() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();

    let txs = create_transactions(&alice, 5, bob.address);
    add_transactions(&mut blockchain, txs);

    blockchain
        .mine_pending_transactions(alice.address)
        .expect("mining should succeed");

    let tx6 = alice.create_transaction(bob.address, 10, 1, 5);
    let tx7 = alice.create_transaction(bob.address, 10, 1, 6);

    add_transactions(&mut blockchain, vec![tx6, tx7]);

    let tx8 = alice.create_transaction(bob.address, 10, 1, 8);
    let tx8_id = tx8.txid();

    let pending_count_before = blockchain.mempool().pending_count(&alice.address);

    let success = blockchain.add_transaction(tx8);

    let pending_count_after = blockchain.mempool().pending_count(&alice.address);

    assert!(!success);
    assert_eq!(pending_count_after, pending_count_before);
    assert!(!blockchain.mempool().contains(&tx8_id));
}

#[test]
fn should_not_accept_if_nonce_overflows() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();

    blockchain
        .ledger_mut_for_test()
        .set_nonce_for_test(alice.address, u64::MAX);

    let tx1 = alice.create_transaction(bob.address, 10, 1, u64::MAX);
    let tx1_id = tx1.txid();

    assert!(blockchain.add_transaction(tx1));
    assert!(blockchain.mempool().contains(&tx1_id));
    assert_eq!(blockchain.mempool().pending_count(&alice.address), 1);

    let tx2 = alice.create_transaction(bob.address, 10, 1, 1);
    let tx2_id = tx2.txid();

    let pending_count_before = blockchain.mempool().pending_count(&alice.address);

    let success = blockchain.add_transaction(tx2);

    let pending_count_after = blockchain.mempool().pending_count(&alice.address);

    assert!(!success);
    assert_eq!(pending_count_after, pending_count_before);
    assert!(blockchain.mempool().contains(&tx1_id));
    assert!(!blockchain.mempool().contains(&tx2_id));
}

#[test]
fn should_accept_last_tx_with_u64_nonce() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();
    blockchain
        .ledger_mut_for_test()
        .set_nonce_for_test(alice.address, u64::MAX - 1);

    let tx = alice.create_transaction(bob.address, 10, 1, u64::MAX - 1);

    let success1 = blockchain.add_transaction(tx);
    assert_eq!(success1, true);

    let tx2 = alice.create_transaction(bob.address, 10, 1, u64::MAX);
    let success2 = blockchain.add_transaction(tx2);

    assert_eq!(success2, true);
}

#[test]
fn should_not_accept_transaction_with_stale_nonce() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let initial_config =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 200)])), None, 4)
            .expect("valid genesis config");

    let mut blockchain = Blockchain::new(initial_config).unwrap();

    blockchain
        .ledger_mut_for_test()
        .set_nonce_for_test(alice.address, 7);

    let tx = alice.create_transaction(bob.address, 10, 1, 6);
    let tx_id = tx.txid();

    let pending_count_before = blockchain.mempool().pending_count(&alice.address);

    let success = blockchain.add_transaction(tx);

    let pending_count_after = blockchain.mempool().pending_count(&alice.address);

    assert!(!success);
    assert_eq!(pending_count_after, pending_count_before);
    assert!(!blockchain.mempool().contains(&tx_id));
}

#[test]
fn transaction_fails_if_fee_changes() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.fee = 10;

    assert!(!tx.is_valid());
}

#[test]
fn transaction_fails_if_nonce_changes() {
    let alice = Wallet::new();
    let bob = Wallet::new();

    let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

    tx.nonce = 2;

    assert!(!tx.is_valid());
}

// Проверяет полный асинхронный mining pipeline: подготовка кандидата в Blockchain,
// выполнение PoW вне владельца состояния и commit возвращённого результата.
#[test]
fn mines_pending_transactions_through_blockchain() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let genesis_data =
        GenesisConfig::get_config(Some(HashMap::from([(alice.address, 100)])), None, 4)
            .expect("valid genesis config");
    let mut blockchain = Blockchain::new(genesis_data).expect("valid genesis block");
    let transaction = alice.create_transaction(bob.address, 50, 5, 0);

    assert!(blockchain.add_transaction(transaction));
    let Some((mut candidate, next_ledger)) = blockchain
        .mine_pending_transactions(alice.address)
        .expect("candidate creation should succeed")
    else {
        panic!("pending transaction should produce a candidate");
    };

    candidate.mine_block();
    blockchain
        .add_block(candidate, next_ledger)
        .expect("mined block should be committed");

    assert_eq!(blockchain.blocks().len(), 2);
    assert_eq!(blockchain.ledger().balance(&alice.address), 100);
    assert_eq!(blockchain.ledger().balance(&bob.address), 50);
    assert_eq!(blockchain.ledger().nonce(&alice.address), 1);
    assert_eq!(blockchain.mempool().pending_count(&alice.address), 0);
    assert!(blockchain.is_valid());
}
