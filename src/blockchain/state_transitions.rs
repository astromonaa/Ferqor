use std::collections::HashMap;

use crate::{
    address::{Address, Balances},
    blockchain::error::TransactionError,
    transaction::Transaction,
};

pub fn get_new_balances_by_tx_members_roles(
    balances: &Balances,
    transaction: &Transaction,
) -> Result<(u64, u64), TransactionError> {
    let is_same_users = transaction.from == transaction.to;
    let from_balance = balances.get(&transaction.from).copied().unwrap_or(0);
    let to_balance = balances.get(&transaction.to).copied().unwrap_or(0);

    let total_cost: u64 = transaction
        .amount
        .checked_add(transaction.fee)
        .ok_or(TransactionError::CostOverflow)?;

    if from_balance < total_cost {
        return Err(TransactionError::InsufficientBalance {
            available: from_balance,
            required: total_cost,
        });
    }

    /* Если отправитель и получатель совпадают, то только вычитать комиссию */
    let actual_debit = if is_same_users {
        transaction.fee
    } else {
        total_cost
    };

    let from_balance_new =
        from_balance
            .checked_sub(actual_debit)
            .ok_or(TransactionError::InsufficientBalance {
                available: from_balance,
                required: total_cost,
            })?;

    let to_balance_new = if is_same_users {
        from_balance_new
    } else {
        to_balance
            .checked_add(transaction.amount)
            .ok_or(TransactionError::BalanceOverflow)?
    };

    Ok((from_balance_new, to_balance_new))
}

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

pub fn apply_transaction_to_state(
    transaction: &Transaction,
    balances: &mut Balances,
    nonces: &mut HashMap<Address, u64>,
    coinbase: Address,
) -> Result<(), TransactionError> {
    if transaction.from == coinbase {
        return pay_miner_reward(balances, transaction);
    }

    if !transaction.is_valid() {
        return Err(TransactionError::InvalidTransaction);
    }

    let current_nonce = nonces.get(&transaction.from).copied().unwrap_or(0);

    if transaction.nonce != current_nonce {
        return Err(TransactionError::InvalidNonce {
            expected: current_nonce,
            actual: transaction.nonce,
        });
    }

    let next_nonce = current_nonce
        .checked_add(1)
        .ok_or(TransactionError::NonceOverflow)?;

    let (from_new, to_new) = get_new_balances_by_tx_members_roles(balances, transaction)?;

    balances.insert(transaction.from, from_new);
    balances.insert(transaction.to, to_new);
    nonces.insert(transaction.from, next_nonce);

    Ok(())
}
