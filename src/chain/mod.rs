//! Состояние и валидация цепочки, выделенные из `blockchain::blockchain::Blockchain`
//! и прежнего `blockchain::validator::BlockchainValidator`.

mod chain;
mod types;
pub use types::*;

pub mod validator;

pub use chain::Chain;
