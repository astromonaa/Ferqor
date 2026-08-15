use crate::{address::Balances, transaction::Transaction};

pub fn get_new_balances_by_tx_members_roles(
    balances: &Balances,
    transaction: &Transaction,
) -> Option<(u64, u64)> {
    let is_same_users = transaction.from == transaction.to;
    let from_balance = balances.get(&transaction.from).copied().unwrap_or(0);
    let to_balance = balances.get(&transaction.to).copied().unwrap_or(0);

    let total_cost = transaction.amount.checked_add(transaction.fee)?;

    if from_balance < total_cost {
        return None;
    }

    /* Если отправитель и получатель совпадают, то только вычитать комиссию */
    let actual_debit = if is_same_users {
        transaction.fee
    } else {
        total_cost
    };
    
    let from_balance_new = from_balance.checked_sub(actual_debit)?;

    let to_balance_new = if is_same_users {
        from_balance_new
    } else {
        to_balance.checked_add(transaction.amount)?
    };


    Some((from_balance_new, to_balance_new))
}
