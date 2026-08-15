use crate::address::{Address, Balances};
use crate::block::Block;
use crate::blockchain::Blockchain;
use crate::blockchain::utils::get_new_balances_by_tx_members_roles;
use crate::transaction::Transaction;
use std::collections::HashMap;

pub struct BlockchainValidator;

impl BlockchainValidator {
    pub fn check_block_weight(block: &Block, max_block_weight: usize) -> bool {
        matches!(
            block.weight(),
            Some(weight) if weight <= max_block_weight
        )
    }

    pub fn check_coinbase_transaction(
        current_block: &Block,
        mining_reward: u64,
        coinbase: Address,
    ) -> bool {
        let Some(first_tx) = current_block.transactions.first() else {
            return false;
        };

        if first_tx.from != coinbase {
            return false;
        }

        /* Делаем skip(1), чтобы пропустить первый Coinbase транзакцию, т.к. Coinbase не платит fee */
        let Some(total_fees) = current_block
            .transactions
            .iter()
            .skip(1)
            .try_fold(0u64, |acc, tx| acc.checked_add(tx.fee))
        else {
            return false;
        };

        let Some(expected_reward) = mining_reward.checked_add(total_fees) else {
            return false;
        };

        if first_tx.amount != expected_reward {
            return false;
        }

        let coinbase_transactions = current_block
            .transactions
            .iter()
            .filter(|tx| tx.from == coinbase)
            .count();

        coinbase_transactions == 1
    }

    pub fn validate_state_transitions(blockchain: &Blockchain, coinbase: Address) -> bool {
        let mut balances = blockchain.initial_balances.clone();
        let mut nonces = blockchain.initial_nonces.clone();

        fn apply(
            transaction: &Transaction,
            balances: &mut Balances,
            coinbase: Address,
            nonces: &mut HashMap<Address, u64>,
        ) -> bool {
            if transaction.from == coinbase {
                let to_balance = balances.get(&transaction.to).copied().unwrap_or(0);
                let Some(to_balance_new) = to_balance.checked_add(transaction.amount) else {
                    return false;
                };
                balances.insert(transaction.to, to_balance_new);
                return true;
            }

            let current_nonce = nonces.get(&transaction.from).copied().unwrap_or(0);

            let Some(next_nonce) = current_nonce.checked_add(1) else {
                return false;
            };

            if current_nonce != transaction.nonce {
                return false;
            }

            let Some((from_balance_new, to_balance_new)) =
                get_new_balances_by_tx_members_roles(&balances, transaction)
            else {
                return false;
            };

            balances.insert(transaction.from, from_balance_new);
            balances.insert(transaction.to, to_balance_new);

            nonces.insert(transaction.from, next_nonce);

            true
        }

        for block in &blockchain.chain {
            for transaction in &block.transactions {
                if !transaction.is_valid() {
                    return false;
                }
                let success = apply(transaction, &mut balances, coinbase, &mut nonces);
                if !success {
                    return false;
                }
            }
        }

        balances == blockchain.balances && nonces == blockchain.nonces
    }
}
