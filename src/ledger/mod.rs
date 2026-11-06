//! Состояние реестра аккаунтов, выделенное из
//! `blockchain::state_transitions::apply_transaction_to_state` и прежних полей
//! балансов и nonce в `blockchain::blockchain::Blockchain`.

mod ledger_state;

pub use ledger_state::LedgerState;

#[cfg(test)]
mod ledger_state_tests;
