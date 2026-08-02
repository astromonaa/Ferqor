#[cfg(test)]
mod tests {
    use ed25519_dalek::Signature;
    use crate::wallet::Wallet;
    use super::*;

    #[test]
    fn transaction_fails_if_amount_changes() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);
        tx.amount = 500;

        assert!(!tx.is_valid());
    }

    #[test]
    fn transaction_fails_if_receiver_changes() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let fake = Wallet::new();

        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

        tx.to = fake.address;

        assert!(!tx.is_valid());
    }

    #[test]
    fn transaction_fails_if_sender_changes() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let fake = Wallet::new();

        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

        tx.from = fake.address;

        assert!(!tx.is_valid());
    }

    #[test]
    fn transaction_fails_if_public_key_is_replaced() {
        let alice = Wallet::new();
        let bob = Wallet::new();
        let fake = Wallet::new();

        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

        tx.public_key = Some(fake.public_key);

        assert!(!tx.is_valid());
    }

    #[test]
    fn transaction_is_valid() {
        let alice = Wallet::new();
        let bob = Wallet::new();

        let tx = alice.create_transaction(bob.address, 100, 5, 1);

        assert!(tx.is_valid());
    }

    #[test]
    fn test_fake_signature() {
        let alice = Wallet::new();
        let bob = Wallet::new();

        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

        if let Some(signature) = &mut tx.signature {
            let mut bytes = signature.to_bytes();
            bytes[0] ^= 1;

            tx.signature = Some(Signature::from_bytes(&bytes));
        }

        assert!(!tx.is_valid());

    }

    #[test]
    fn test_missing_signature() {
        let alice = Wallet::new();
        let bob = Wallet::new();

        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

        tx.signature = None;

        assert!(!tx.is_valid());
    }

    #[test]
    fn test_missing_public_key() {
        let alice = Wallet::new();
        let bob = Wallet::new();

        let mut tx = alice.create_transaction(bob.address, 100, 5, 1);

        tx.public_key = None;

        assert!(!tx.is_valid());
    }
}