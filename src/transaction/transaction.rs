use std::cmp::Ordering;
use crate::address::{Address, COINBASE, address_from_public_key};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub [u8; 32]);

impl Ord for TransactionId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for TransactionId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TransactionId {
    pub fn as_bytes(&self) -> &[u8; 32] {
        return &self.0
    }
}
#[derive(Debug, Clone)]
pub struct Transaction {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub signature: Option<Signature>,
    pub public_key: Option<VerifyingKey>,
    pub fee: u64,
    pub nonce: u64,
}

impl Transaction {

    pub const FIXED_WEIGHT: usize = 1;

    pub fn txid(&self) -> TransactionId {
        let mut hasher = Sha256::new();
        hasher.update(self.from.0);
        hasher.update(self.to.0);
        hasher.update(self.amount.to_be_bytes());
        hasher.update(self.fee.to_be_bytes());
        hasher.update(self.nonce.to_be_bytes());

        if let Some(signature) = &self.signature {
            hasher.update(signature.to_bytes())
        }

        if let Some(public_key) = &self.public_key {
            hasher.update(public_key.to_bytes())
        }

        let result = hasher.finalize();

        let mut tx_id = [0u8; 32];
        tx_id.copy_from_slice(&result);

        TransactionId(tx_id)
    }

    pub fn hash_payload(from: &Address, to: &Address, amount: u64, fee: u64, nonce: u64) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(from.0);
        hasher.update(to.0);
        hasher.update(amount.to_be_bytes());
        hasher.update(fee.to_be_bytes());
        hasher.update(nonce.to_be_bytes());
        hasher.finalize().to_vec()
    }

    pub fn is_valid(&self) -> bool {
        if self.from == COINBASE {
            return true;
        }

        self.verify_public_key_owner() && self.verify_signature()
    }

    pub fn verify_public_key_owner(&self) -> bool {
        let Some(public_key) = &self.public_key else {
            return false;
        };

        let sender_address = address_from_public_key(public_key);

        sender_address == self.from
    }

    pub fn verify_signature(&self) -> bool {
        let Some(public_key) = &self.public_key else {
            return false;
        };

        let Some(signature) = &self.signature else {
            return false;
        };

        let payload_hash = Transaction::hash_payload(&self.from, &self.to, self.amount, self.fee, self.nonce);
        public_key.verify(&payload_hash, signature).is_ok()
    }

    pub fn weight(&self) -> usize {
        Self::FIXED_WEIGHT
    }
}
