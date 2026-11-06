use crate::{
    block::commitment,
    consensus::{Difficulty, MAX_DIFFICULTY},
    transaction::Transaction,
};
use std::sync::Arc;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct MerkleRoot(pub [u8; 32]);

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub index: u64,
    pub previous_hash: BlockHash,
    pub nonce: u64,
    pub timestamp: u64,
    pub merkle_root: MerkleRoot,
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy)]
pub struct BlockHash(pub [u8; 32]);

impl BlockHash {
    pub fn has_leading_zero_hex_digits(&self, difficulty: Difficulty) -> bool {
        if difficulty > MAX_DIFFICULTY {
            return false;
        }

        /*
          Делаем difficulty / 2 т.к. каждые два hex-символа — это один байт
          Например difficulty = 4 символа, надо  проверять 4/2 = 2 байта
        */
        let full_bytes = difficulty / 2;
        let half_bytes = difficulty % 2;

        /*
          Например difficulty = 5 символов, что равно 2 целым байтам
          Проверяем что целые байты точно равны нулю
        */
        if self.0[..full_bytes as usize].iter().any(|&x| x != 0) {
            return false;
        }

        /*
          После этого остается 1 последний символ, что равно половине байта, т.е. 4 бита
          Если после проверки целых байтов остался один hex-символ,
          проверяем, что старшие 4 бита следующего байта равны нулю
        */
        if half_bytes == 1 {
            return self.0[full_bytes as usize] >> 4 == 0;
        }

        true
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Arc<Transaction>>,
    pub hash: BlockHash,
}

impl Block {
    pub fn new(
        index: u64,
        transactions: Vec<Arc<Transaction>>,
        previous_hash: BlockHash,
        nonce: u64,
        timestamp: u64,
        difficulty: Difficulty,
    ) -> Self {
        let merkle_root = Self::calculate_merkle_root(&transactions);

        let header = BlockHeader {
            index,
            previous_hash,
            nonce,
            timestamp,
            merkle_root,
            difficulty,
        };

        let hash = Self::calculate_hash(&header);

        Self {
            transactions,
            hash,
            header,
        }
    }

    pub fn calculate_merkle_root(transactions: &[Arc<Transaction>]) -> MerkleRoot {
        commitment::calculate_merkle_root(transactions)
    }

    pub fn calculate_hash(header: &BlockHeader) -> BlockHash {
        commitment::calculate_block_hash(header)
    }

    pub fn mine_block(&mut self) {
        while !self
            .hash
            .has_leading_zero_hex_digits(self.header.difficulty)
        {
            self.header.nonce += 1;
            self.hash = Block::calculate_hash(&self.header)
        }
    }

    pub fn weight(&self) -> Option<usize> {
        self.transactions
            .iter()
            .try_fold(0usize, |acc, tx| acc.checked_add(tx.weight()))
    }
}
