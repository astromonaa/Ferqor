
use tokio::sync::mpsc::error::SendError;

use crate::network::types::{NodeCommand};



#[derive(Debug)]
pub enum AppError {
    BroadcastBlockError(SendError<NodeCommand>),
    BroadcastTransactionError(SendError<NodeCommand>)
}