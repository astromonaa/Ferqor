use std::collections::HashMap;

use num_bigint::BigUint;

use crate::block::{Block, block::BlockHash};

pub enum BlockImportOutcome {
    AlreadyKnown,
    ExtendedTip,
    StoredSideBranch,
    Reorganized,
    MissingParent { parent: BlockHash },
}


pub struct IndexedBlock {
    block: Block,
    chain_work: BigUint
}

impl IndexedBlock {
    pub fn new(block: Block, chain_work: BigUint) -> Self {
        Self { block, chain_work }
    }
}

pub type ChainRegistry = HashMap<BlockHash, IndexedBlock>;