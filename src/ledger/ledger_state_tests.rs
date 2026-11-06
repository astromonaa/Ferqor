//! Тесты переходов состояния, перенесённых из `blockchain::state_transitions` в `LedgerState`.

use std::collections::HashMap;

use crate::address::COINBASE;
use crate::blockchain::error::TransactionError;
use crate::transaction::Transaction;
use crate::wallet::Wallet;

use super::LedgerState;

#[test]
fn applies_signed_transaction_to_balances_and_nonce() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let transaction = alice.create_transaction(bob.address, 40, 5, 0);
    let mut ledger = LedgerState::from_balances(HashMap::from([(alice.address, 100)]));

    ledger
        .apply_transaction_to_state(&transaction, COINBASE)
        .expect("valid transaction should apply");

    assert_eq!(ledger.balance(&alice.address), 55);
    assert_eq!(ledger.balance(&bob.address), 40);
    assert_eq!(ledger.nonce(&alice.address), 1);
}

#[test]
fn applies_coinbase_reward_without_advancing_nonce() {
    let miner = Wallet::new();
    let reward = Transaction {
        from: COINBASE,
        to: miner.address,
        amount: 55,
        signature: None,
        public_key: None,
        fee: 0,
        nonce: 0,
    };
    let mut ledger = LedgerState::from_balances(HashMap::new());

    ledger
        .apply_transaction_to_state(&reward, COINBASE)
        .expect("coinbase reward should apply");

    assert_eq!(ledger.balance(&miner.address), 55);
    assert_eq!(ledger.nonce(&COINBASE), 0);
}

#[test]
fn rejects_wrong_nonce_without_mutating_state() {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let transaction = alice.create_transaction(bob.address, 40, 5, 1);
    let mut ledger = LedgerState::from_balances(HashMap::from([(alice.address, 100)]));
    let original = ledger.clone();

    let error = ledger
        .apply_transaction_to_state(&transaction, COINBASE)
        .expect_err("wrong nonce should be rejected");

    assert_eq!(
        error,
        TransactionError::InvalidNonce {
            expected: 0,
            actual: 1,
        }
    );
    assert_eq!(ledger, original);
}
