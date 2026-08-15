use crate::address::Address;
use crate::transaction::{Transaction, TransactionId};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::sync::Arc;

pub type TransactionMap = HashMap<TransactionId, Arc<Transaction>>;
pub type SenderMap = HashMap<Address, BTreeMap<u64, TransactionId>>;

type MineCandidate = (u64, Reverse<u64>, TransactionId);

pub struct Mempool {
    pub transactions: TransactionMap,
    pub by_sender: SenderMap,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            by_sender: HashMap::new(),
        }
    }

    pub fn contains(&self, txid: &TransactionId) -> bool {
        self.transactions.contains_key(txid)
    }

    pub fn pending_count(&self, sender: &Address) -> u64 {
        self.by_sender
            .get(sender)
            .map(|txs| txs.len() as u64)
            .unwrap_or(0)
    }

    pub fn pending_cost(&self, sender: &Address) -> Option<u64> {
        let Some(sender_map) = self.by_sender.get(sender) else {
            return Some(0);
        };

        sender_map.iter().try_fold(0u64, |acc, (nonce, txid)| {
            let tx = self.transactions.get(txid)?;

            if tx.from != *sender || tx.nonce != *nonce {
                return None;
            }

            let amount = tx.amount.checked_add(tx.fee)?;
            acc.checked_add(amount)
        })
    }

    pub fn insert(&mut self, transaction: Transaction) -> bool {
        let txid = transaction.txid();

        if self.transactions.contains_key(&txid) {
            return false;
        }

        let sender_map = self.by_sender.entry(transaction.from).or_default();

        if sender_map.contains_key(&transaction.nonce) {
            return false;
        }

        sender_map.insert(transaction.nonce, txid);

        self.transactions.insert(txid, Arc::new(transaction));

        true
    }

    pub fn remove(&mut self, txid: &TransactionId) -> bool {
        let (sender, nonce) = {
            let Some(tx) = self.transactions.get(txid) else {
                return false;
            };
            (tx.from, tx.nonce)
        };

        let Some(sender_map) = self.by_sender.get_mut(&sender) else {
            return false;
        };

        let Some(found_txid) = sender_map.get(&nonce) else {
            return false;
        };

        if found_txid != txid {
            return false;
        }

        self.transactions.remove(txid);
        sender_map.remove(&nonce);

        if sender_map.is_empty() {
            self.by_sender.remove(&sender);
        }

        true
    }

    fn push_candidate(
        &self,
        heap: &mut BinaryHeap<MineCandidate>,
        sender_map: &BTreeMap<u64, TransactionId>,
        nonce: u64,
    ) {
        let Some(transaction_id) = sender_map.get(&nonce) else {
            return;
        };

        let Some(transaction) = self.transactions.get(transaction_id) else {
            return;
        };

        heap.push((transaction.fee, Reverse(nonce), *transaction_id))
    }

    pub fn select_transactions(
        &self,
        max_block_weight: usize,
        account_nonces: &HashMap<Address, u64>,
    ) -> Vec<Arc<Transaction>> {
        let mut selected = Vec::new();
        let mut binary_heap = BinaryHeap::new();

        for (sender, sender_map) in &self.by_sender {
            let expected_nonce = account_nonces.get(sender).copied().unwrap_or(0);

            self.push_candidate(&mut binary_heap, sender_map, expected_nonce);
        }

        let mut current_weight: usize = 0;

        while let Some((_, Reverse(nonce), txid)) = binary_heap.pop() {
            let Some(best_transaction) = self.transactions.get(&txid) else {
                continue;
            };

            let tx_weight = best_transaction.weight();

            let Some(next_weight) = current_weight.checked_add(tx_weight) else {
                continue;
            };

            if next_weight > max_block_weight {
                continue;
            };

            current_weight = next_weight;

            selected.push(Arc::clone(best_transaction));

            let Some(next_nonce) = nonce.checked_add(1) else {
                continue;
            };

            let Some(sender_map) = self.by_sender.get(&best_transaction.from) else {
                continue;
            };

            self.push_candidate(&mut binary_heap, sender_map, next_nonce);
        }

        selected
    }

    pub fn remove_selected_transactions(&mut self, transactions: &[Arc<Transaction>]) {
        for transaction in transactions {
            self.remove(&transaction.txid());
        }
    }
}
