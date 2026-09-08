use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use crate::{address::{Address, address_from_public_key}, transaction::hash_transaction_signing_payload};
use rand_core::OsRng;
use crate::transaction::Transaction;

pub struct Wallet {
    pub private_key: SigningKey,
    pub public_key: VerifyingKey,
    pub address: Address,
}

impl Wallet {

    pub fn new() -> Self {
        let private_key = SigningKey::generate(&mut OsRng);
        let public_key = private_key.verifying_key();
        let address = address_from_public_key(&public_key);

        Wallet {
            private_key,
            public_key,
            address
        }
    }

    pub fn sign(&self, payload_hash: &[u8]) -> Signature {
        self.private_key.sign(payload_hash)
    }

    pub fn create_transaction(&self, to: Address, amount: u64, fee: u64, nonce: u64) -> Transaction {
        let from = self.address;
        let payload_hash = hash_transaction_signing_payload(&from, &to, amount, fee, nonce);

        let signature = self.sign(&payload_hash);

        Transaction {
            from,
            to,
            amount,
            fee,
            public_key: Some(self.public_key.clone()),
            signature: Some(signature),
            nonce,
        }
    }
}