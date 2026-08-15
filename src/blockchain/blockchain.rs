use crate::address::{Address, Balances, COINBASE};
use crate::block::Block;
use crate::blockchain::utils::get_new_balances_by_tx_members_roles;
use crate::blockchain::validator::BlockchainValidator;
use crate::config::{GenesisConfig, GenesisData};
use crate::mempool::Mempool;
use crate::transaction::Transaction;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
    pub mempool: Mempool,
    pub balances: Balances,
    pub mining_reward: u64,
    pub initial_balances: Balances,
    pub nonces: HashMap<Address, u64>,
    pub initial_nonces: HashMap<Address, u64>,
    pub max_block_weight: usize,
}

const COINBASE_TRANSACTION_COUNT: usize = 1;

impl Blockchain {
    pub fn new(difficulty: usize, genesis_data: GenesisData) -> Self {
        let mut chain = Vec::new();
        let GenesisData {
            mut genesis_block,
            initial_balances,
            max_block_weight,
        } = genesis_data;
        genesis_block.mine_block(difficulty);
        chain.push(genesis_block);

        Self {
            chain,
            difficulty,
            mempool: Mempool::new(),
            balances: initial_balances.clone(),
            mining_reward: 50,
            initial_balances,
            nonces: HashMap::new(),
            initial_nonces: HashMap::new(),
            max_block_weight,
        }
    }

    pub fn add_block(&mut self, transactions: Vec<Arc<Transaction>>) {
        let Some(previous_block) = self.chain.last() else {
            return;
        };
        let index = previous_block.header.index + 1;
        let now = GenesisConfig::get_now_timestamp();
        let mut new_block = Block::new(index, transactions, previous_block.hash, 0, now);
        new_block.mine_block(self.difficulty);
        self.chain.push(new_block);
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> bool {
        if transaction.from == COINBASE {
            return false;
        }

        let from_balance = self.balances.get(&transaction.from).copied().unwrap_or(0);
        let current_nonce = self.nonces.get(&transaction.from).copied().unwrap_or(0);

        let pending_count = self.mempool.pending_count(&transaction.from);

        /* Даем возможность отправить несколько транзакций в mempool */
        let Some(expected_nonce) = current_nonce.checked_add(pending_count) else {
            return false;
        };

        /* Проверяем, что nonce в порядке */
        if transaction.nonce != expected_nonce {
            return false;
        };

        /* Считаем стоимость всех транзакций пользователя в mempool */
        let Some(pending_cost) = self.mempool.pending_cost(&transaction.from) else {
            return false;
        };

        /* Считаем стоимость новой транзакции */
        let Some(tx_cost) = transaction.amount.checked_add(transaction.fee) else {
            return false;
        };

        /* Считаем общую стоимость всех транзакций пользователя с учетом текущей */
        let Some(total_cost) = pending_cost.checked_add(tx_cost) else {
            return false;
        };

        /* Если стоимость больше баланса пользователя, не добавляем */
        if from_balance < total_cost {
            return false;
        };

        if !transaction.is_valid() {
            return false;
        };

        self.mempool.insert(transaction)
    }

    fn pay_miner_reward(&mut self, transaction: &Transaction) {
        let miner_balance = self.balances.get(&transaction.to).copied().unwrap_or(0);
        let Some(new_balance) = miner_balance.checked_add(transaction.amount) else {
            return;
        };
        self.balances.insert(transaction.to, new_balance);
    }

    fn get_reward_transaction(
        &self,
        miner_address: Address,
        total_fees: u64,
    ) -> Option<Arc<Transaction>> {
        let amount = self.mining_reward.checked_add(total_fees)?;
        Some(Arc::new(Transaction {
            from: COINBASE,
            to: miner_address,
            amount,
            signature: None,
            public_key: None,
            fee: 0,
            nonce: 0,
        }))
    }

    pub fn mine_pending_transactions(&mut self, miner_address: Address) {
        /* Из максимальной весовой емкости вычитываем сперва вес Coinbase транзакции */
        let Some(available_weight) = self.max_block_weight.checked_sub(Transaction::FIXED_WEIGHT)
        else {
            return;
        };

        let mut pending_transactions = self
            .mempool
            .select_transactions(available_weight, &self.nonces);

        if pending_transactions.is_empty() {
            return;
        };

        let Some(total_fees) = pending_transactions
            .iter()
            .try_fold(0u64, |acc, tx| acc.checked_add(tx.fee))
        else {
            return;
        };

        let Some(reward_tx) = self.get_reward_transaction(miner_address, total_fees) else {
            return;
        };

        let mut transactions = vec![reward_tx];
        transactions.append(&mut pending_transactions);

        self.add_block(transactions.clone());

        for transaction in &transactions {
            self.apply_transaction(transaction);
        }
        self.mempool
            .remove_selected_transactions(&transactions[COINBASE_TRANSACTION_COUNT..])
    }

    pub fn apply_transaction(&mut self, transaction: &Transaction) {
        if transaction.from == COINBASE {
            self.pay_miner_reward(transaction);
            return;
        }

        let Some((from_new, to_new)) =
            get_new_balances_by_tx_members_roles(&self.balances, transaction)
        else {
            return;
        };

        let nonce = self.nonces.get(&transaction.from).copied().unwrap_or(0);

        let Some(next_nonce) = nonce.checked_add(1) else {
            return;
        };

        self.balances.insert(transaction.from, from_new);

        self.balances.insert(transaction.to, to_new);

        self.nonces.insert(transaction.from, next_nonce);
    }

    pub fn is_valid(&self) -> bool {
        /* Проверяем вес генезис-блока */
        let Some(genesis_block) = self.chain.first() else {
            return false;
        };

        if !BlockchainValidator::check_block_weight(genesis_block, self.max_block_weight) {
            return false;
        }

        /* Проверяем вес и транзакции остальных блоков */
        for index in 1..self.chain.len() {
            let current_block = &self.chain[index];
            let previous_block = &self.chain[index - 1];

            if !BlockchainValidator::check_block_weight(current_block, self.max_block_weight) {
                return false;
            }

            let is_valid_coinbase_transaction = BlockchainValidator::check_coinbase_transaction(
                current_block,
                self.mining_reward,
                COINBASE,
            );

            let calculated_merkle_root = Block::calculate_merkle_root(&current_block.transactions);

            let is_valid_merkle_root = calculated_merkle_root == current_block.header.merkle_root;

            if !is_valid_merkle_root {
                return false;
            }

            let calculated_hash = Block::calculate_hash(&current_block.header);

            let is_valid_hash = calculated_hash == current_block.hash
                && calculated_hash.has_leading_zero_hex_digits(self.difficulty);

            if !is_valid_hash {
                return false;
            }

            if current_block.header.previous_hash != previous_block.hash {
                return false;
            }

            if !is_valid_coinbase_transaction {
                return false;
            }
        }

        BlockchainValidator::validate_state_transitions(self, COINBASE)
    }
}
