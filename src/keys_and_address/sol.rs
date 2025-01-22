//! # Solana Key Pair and Address Generation

use rand::{rngs::ThreadRng, RngCore};
use solana_sdk::bs58;
use solana_sdk::signature::{Keypair, SeedDerivable, Signer};
use std::cell::RefCell;

use crate::keys_and_address::{KeyPairGenerator, SolanaKeyPair};

thread_local! {
    static THREAD_LOCAL_RNG: RefCell<ThreadRng> = RefCell::new(rand::rng());
}

impl KeyPairGenerator for SolanaKeyPair {
    /// Generates a randomly generated Solana key pair and their address.
    /// Returns `SolanaKeyPair` struct.
    fn generate_random() -> Self {
        THREAD_LOCAL_RNG.with(|rng| {
            let mut seed = [0u8; 32];
            rng.borrow_mut().fill_bytes(&mut seed); // Fill the seed with random bytes
            let keypair = Keypair::from_seed(&seed).expect("Valid seed");
            let address = keypair.pubkey().to_string();

            SolanaKeyPair { keypair, address }
        })
    }

    fn get_address(&self) -> &String {
        &self.address
    }
}

impl SolanaKeyPair {
    pub fn get_private_key_as_base58(&self) -> String {
        bs58::encode(self.keypair.secret().to_bytes()).into_string()
    }

    pub fn get_public_key_as_base58(&self) -> String {
        self.keypair.pubkey().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bs58;
    use solana_sdk::signature::Signer;

    #[test]
    fn test_generate_random() {
        // Generate a random Solana key pair
        let key_pair = SolanaKeyPair::generate_random();

        // Ensure the public key is derived correctly
        let derived_public_key = key_pair.keypair.pubkey();
        assert_eq!(key_pair.keypair.pubkey(), derived_public_key);

        // Ensure the address matches the public key
        let address = key_pair.get_address();
        assert_eq!(address, &derived_public_key.to_string());
    }

    #[test]
    fn test_get_private_key_as_base58() {
        let key_pair = SolanaKeyPair::generate_random();
        let private_key_base58 = bs58::encode(key_pair.keypair.secret().to_bytes()).into_string();

        assert_eq!(key_pair.get_private_key_as_base58(), private_key_base58);
    }

    #[test]
    fn test_get_public_key_as_base58() {
        let key_pair = SolanaKeyPair::generate_random();
        let public_key_base58 = key_pair.keypair.pubkey().to_string();

        assert_eq!(key_pair.get_public_key_as_base58(), public_key_base58);
    }

    #[test]
    fn test_unique_keypairs() {
        // Generate multiple key pairs and ensure they are unique
        let key_pair_1 = SolanaKeyPair::generate_random();
        let key_pair_2 = SolanaKeyPair::generate_random();

        assert_ne!(key_pair_1.keypair.pubkey(), key_pair_2.keypair.pubkey());
        assert_ne!(
            key_pair_1.get_private_key_as_base58(),
            key_pair_2.get_private_key_as_base58()
        );
    }
}
