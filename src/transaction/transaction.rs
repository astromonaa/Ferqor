use std::cmp::Ordering;
use crate::{address::{Address, COINBASE, address_from_public_key}, transaction::{encode_transaction, hash_transaction_signing_payload}};
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
        let payload = encode_transaction(&self);
        let hash: [u8; 32] = Sha256::digest(payload).into();
        TransactionId(hash)
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

        let payload_hash = hash_transaction_signing_payload(&self.from, &self.to, self.amount, self.fee, self.nonce);
        public_key.verify(&payload_hash, signature).is_ok()
    }

    pub fn weight(&self) -> usize {
        Self::FIXED_WEIGHT
    }
}
