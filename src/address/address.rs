use std::cmp::Ordering;
use std::collections::HashMap;
use ed25519_dalek::VerifyingKey;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Address(pub [u8; 20]);

impl Ord for Address {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type Balances = HashMap<Address, u64>;

pub const COINBASE: Address = Address([0; 20]);



pub fn address_from_public_key(public_key: &VerifyingKey) -> Address {
    let mut hasher = Sha256::new();

    hasher.update(public_key.as_bytes());

    let result = hasher.finalize();

    let mut address = [0u8;20];
    address.copy_from_slice(&result[..20]);

    Address(address)
}