 use std::sync::Arc;

 use crate::mempool::Mempool;
 use crate::wallet::Wallet;

 #[test]
 fn undefined_sender_cost_should_be_zero() {
     let alice = Wallet::new();
     let bob = Wallet::new();

     let mempool = Mempool::new();

     assert_eq!(mempool.pending_cost(&alice.address), Some(0));
     assert_eq!(mempool.pending_cost(&bob.address), Some(0));
 }

 #[test]
 fn should_calculate_one_tx_pending_cost_correctly() {
     let alice = Wallet::new();
     let bob = Wallet::new();
     let tx = alice.create_transaction(bob.address, 100, 5, 0);

     let mut mempool = Mempool::new();

     mempool.insert(tx);

     assert_eq!(mempool.pending_cost(&alice.address), Some(105))
 }

 #[test]
 fn should_calculate_several_txs_pending_cost_correctly() {
     let alice = Wallet::new();
     let bob = Wallet::new();
     let tx = alice.create_transaction(bob.address, 100, 5, 0);
     let tx2 = alice.create_transaction(bob.address, 40, 2, 1);

     let mut mempool = Mempool::new();

     mempool.insert(tx);
     mempool.insert(tx2);

     assert_eq!(mempool.pending_cost(&alice.address), Some(147))
 }

 #[test]
 fn should_return_none_if_type_overflows_with_one_tx() {
     let alice = Wallet::new();
     let bob = Wallet::new();
     let tx = alice.create_transaction(bob.address, u64::MAX, 1, 0);

     let mut mempool = Mempool::new();

     mempool.insert(tx);

     assert_eq!(mempool.pending_cost(&alice.address), None);
 }

 #[test]
 fn should_return_none_if_type_overflows_with_several_txs() {
     let alice = Wallet::new();
     let bob = Wallet::new();
     let tx = alice.create_transaction(bob.address, u64::MAX-1, 1, 0);
     let tx2 = alice.create_transaction(bob.address, 1, 0, 1);

     let mut mempool = Mempool::new();

     mempool.insert(tx);
     mempool.insert(tx2);

     assert_eq!(mempool.pending_cost(&alice.address), None);
 }

 #[test]
 fn transactions_and_by_sender_should_be_sync() {
     let alice = Wallet::new();
     let bob = Wallet::new();
     let tx = alice.create_transaction(bob.address, 100, 5, 0);
     let txid = tx.txid();


     let mut mempool = Mempool::new();

     mempool.insert(tx);

     mempool.transactions.remove(&txid);

     assert_eq!(mempool.pending_cost(&alice.address), None);
 }

 #[test]
 fn transactions_should_not_be_mixed() {
     let alice = Wallet::new();
     let bob = Wallet::new();
     let bob_tx = bob.create_transaction(bob.address, 100, 5, 0);
     let alice_tx = alice.create_transaction(bob.address, 100, 5, 1);
     
     let mut mempool = Mempool::new();
             
     mempool.insert(bob_tx);

     let bob_map = mempool.by_sender.entry(bob.address).or_default();

     bob_map.insert(alice_tx.nonce, alice_tx.txid());
     mempool.transactions.insert(alice_tx.txid(), Arc::new(alice_tx));

     assert_eq!(mempool.pending_cost(&bob.address), None);
 }

 #[test]
 fn sender_map_nonce_should_be_correct() {
     let bob = Wallet::new();
     let bob_tx = bob.create_transaction(bob.address, 100, 5, 0);

     let mut mempool = Mempool::new();

     let bob_map = mempool.by_sender.entry(bob.address).or_default();

     bob_map.insert(1, bob_tx.txid());
     mempool.transactions.insert(bob_tx.txid(), Arc::new(bob_tx));

     assert_eq!(mempool.pending_cost(&bob.address), None);
 }