//! Сервисы майнинга, выделенные из `blockchain::blockchain::Blockchain`.
//!
//! Операции прежнего `blockchain::blockchain::Blockchain` сохранены без
//! переименования: `create_block` и `get_reward_transaction` принадлежат
//! `BlockProducer`, а `add_block` и `mine_pending_transactions` — `Blockchain`.

mod block_producer;

pub use block_producer::BlockProducer;
