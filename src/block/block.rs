use crate::{
    blockchain::utils::{Difficulty, MAX_DIFFICULTY},
    transaction::{Transaction, TransactionId},
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub struct MerkleRoot(pub [u8; 32]);

pub struct BlockHeader {
    pub index: u64,
    pub previous_hash: BlockHash,
    pub nonce: u64,
    pub timestamp: u64,
    pub merkle_root: MerkleRoot,
    pub difficulty: Difficulty,
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
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
        if transactions.is_empty() {
            return MerkleRoot([0u8; 32]);
        };

        let mut hashes: Vec<TransactionId> = transactions.iter().map(|tx| tx.txid()).collect();

        while hashes.len() > 1 {
            if hashes.len() % 2 != 0 {
                let last = *hashes.last().unwrap();
                hashes.push(last);
            }

            hashes = hashes
                .chunks(2)
                .map(|pair| {
                    let mut hasher = Sha256::new();

                    hasher.update(pair[0].as_bytes());
                    hasher.update(pair[1].as_bytes());

                    let result = hasher.finalize();

                    let mut root = [0u8; 32];
                    root.copy_from_slice(&result);

                    TransactionId(root)
                })
                .collect();
        }

        MerkleRoot(hashes[0].0)
    }

    pub fn calculate_hash(header: &BlockHeader) -> BlockHash {
        let mut hasher = Sha256::new();
        hasher.update(header.index.to_be_bytes());
        hasher.update(header.merkle_root.0);
        hasher.update(header.previous_hash.0);
        hasher.update(header.nonce.to_be_bytes());
        hasher.update(header.timestamp.to_be_bytes());
        hasher.update(header.difficulty.to_be_bytes());

        let result = hasher.finalize();
        let mut root = [0u8; 32];

        root.copy_from_slice(&result);

        BlockHash(root)
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
