//! Хранилище реестра и переходы состояния, выделенные из `blockchain::state_transitions`
//! и прежних полей балансов и nonce в `blockchain::Blockchain`.

use std::collections::HashMap;

use crate::address::{Address, Balances};
use crate::block::Block;
use crate::blockchain::error::{BlockAcceptanceError, TransactionError};
use crate::transaction::Transaction;

/// Владеет балансами и nonce аккаунтов, которые ранее были отдельными публичными
/// полями `blockchain::blockchain::Blockchain`.
///
/// Это не замена `BlockchainValidator`: из валидатора сюда перенесён только расчёт
/// следующего состояния из `BlockchainValidator::calculate_next_state`. Проверки
/// структуры блока и правил консенсуса остаются в `chain::validator`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerState {
    balances: Balances,
    nonces: HashMap<Address, u64>,
}

impl LedgerState {
    /// Создаёт начальное состояние из `config::GenesisData::initial_balances`,
    /// заменяя `Blockchain::initial_balances` вместе с пустой картой nonce.
    pub fn from_balances(balances: Balances) -> Self {
        Self {
            balances,
            nonces: HashMap::new(),
        }
    }

    /// Читает одно значение, ранее получаемое напрямую из `Blockchain::balances`.
    pub fn balance(&self, address: &Address) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    /// Читает одно значение, ранее получаемое напрямую из `Blockchain::nonces`.
    pub fn nonce(&self, address: &Address) -> u64 {
        self.nonces.get(address).copied().unwrap_or(0)
    }

    /// Предоставляет доступ только для чтения вместо прежнего публичного поля `Blockchain::balances`.
    pub fn balances(&self) -> &Balances {
        &self.balances
    }

    /// Предоставляет доступ только для чтения вместо прежнего публичного поля `Blockchain::nonces`.
    pub fn nonces(&self) -> &HashMap<Address, u64> {
        &self.nonces
    }

    /// Заменяет прямое изменение карты nonce в прежних тестах `Blockchain`.
    #[cfg(test)]
    pub(crate) fn set_nonce_for_test(&mut self, address: Address, nonce: u64) {
        self.nonces.insert(address, nonce);
    }

    /// Применяет прежнюю in-place реализацию
    /// `blockchain::state_transitions::apply_transaction_to_state` к текущему реестру.
    pub(crate) fn apply_transaction_to_state(
        &mut self,
        transaction: &Transaction,
        coinbase: Address,
    ) -> Result<(), TransactionError> {
        if transaction.from == coinbase {
            return Self::pay_miner_reward(&mut self.balances, transaction);
        }

        if !transaction.is_valid() {
            return Err(TransactionError::InvalidTransaction);
        }

        let current_nonce = self.nonces.get(&transaction.from).copied().unwrap_or(0);

        if transaction.nonce != current_nonce {
            return Err(TransactionError::InvalidNonce {
                expected: current_nonce,
                actual: transaction.nonce,
            });
        }

        let next_nonce = current_nonce
            .checked_add(1)
            .ok_or(TransactionError::NonceOverflow)?;

        let (from_balance, to_balance) =
            Self::get_new_balances_by_tx_members_roles(&self.balances, transaction)?;

        self.balances.insert(transaction.from, from_balance);
        self.balances.insert(transaction.to, to_balance);
        self.nonces.insert(transaction.from, next_nonce);

        Ok(())
    }

    /// Атомарно применяет блок с помощью цикла, перенесённого из
    /// `BlockchainValidator::calculate_next_state`.
    pub(crate) fn apply_block(
        &self,
        block: &Block,
        coinbase: Address,
    ) -> Result<Self, BlockAcceptanceError> {
        let mut next_state = self.clone();

        for (transaction_index, transaction) in block.transactions.iter().enumerate() {
            next_state
                .apply_transaction_to_state(transaction, coinbase)
                .map_err(|source| BlockAcceptanceError::StateTransition {
                    transaction_index,
                    source,
                })?;
        }

        Ok(next_state)
    }

    /// Сохраняет прежнюю реализацию
    /// `state_transitions::get_new_balances_by_tx_members_roles` над переданной картой.
    pub(crate) fn get_new_balances_by_tx_members_roles(
        balances: &Balances,
        transaction: &Transaction,
    ) -> Result<(u64, u64), TransactionError> {
        let is_same_users = transaction.from == transaction.to;
        let from_balance = balances.get(&transaction.from).copied().unwrap_or(0);
        let to_balance = balances.get(&transaction.to).copied().unwrap_or(0);

        let total_cost = transaction
            .amount
            .checked_add(transaction.fee)
            .ok_or(TransactionError::CostOverflow)?;

        if from_balance < total_cost {
            return Err(TransactionError::InsufficientBalance {
                available: from_balance,
                required: total_cost,
            });
        }

        let actual_debit = if is_same_users {
            transaction.fee
        } else {
            total_cost
        };

        let from_balance_new = from_balance.checked_sub(actual_debit).ok_or(
            TransactionError::InsufficientBalance {
                available: from_balance,
                required: total_cost,
            },
        )?;

        let to_balance_new = if is_same_users {
            from_balance_new
        } else {
            to_balance
                .checked_add(transaction.amount)
                .ok_or(TransactionError::BalanceOverflow)?
        };

        Ok((from_balance_new, to_balance_new))
    }

    /// Применяет логику начисления coinbase-награды, перенесённую из
    /// `state_transitions::pay_miner_reward`.
    fn pay_miner_reward(
        balances: &mut Balances,
        transaction: &Transaction,
    ) -> Result<(), TransactionError> {
        let miner_balance = balances.get(&transaction.to).copied().unwrap_or(0);
        let new_balance = miner_balance
            .checked_add(transaction.amount)
            .ok_or(TransactionError::BalanceOverflow)?;
        balances.insert(transaction.to, new_balance);
        Ok(())
    }
}
