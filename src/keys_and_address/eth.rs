//! # Ethereum Key Pair and Address Generation

use rand::{rngs::ThreadRng, RngCore};
use secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};
use std::cell::RefCell;

use crate::keys_and_address::{EthereumKeyPair, KeyPairGenerator};

thread_local! {
    static THREAD_LOCAL_SECP256K1: Secp256k1<All> = Secp256k1::new();
    static THREAD_LOCAL_RNG: RefCell<ThreadRng> = RefCell::new(rand::rng());
}

impl KeyPairGenerator for EthereumKeyPair {
    /// Generates a randomly generated Ethereum key pair and their address.
    /// Returns `EthereumKeyPair` struct.
    fn generate_random() -> Self {
        THREAD_LOCAL_SECP256K1.with(|secp256k1| {
            THREAD_LOCAL_RNG.with(|rng| {
                let mut secret_bytes = [0u8; 32];
                rng.borrow_mut().fill_bytes(&mut secret_bytes); // Mutably borrow RNG

                let secret_key = SecretKey::from_byte_array(&secret_bytes)
                    .expect("32 bytes, within curve order");
                let public_key = PublicKey::from_secret_key(secp256k1, &secret_key);

                // Derive the Ethereum address (Keccak256 hash of the public key, last 20 bytes)
                let public_key_bytes = public_key.serialize_uncompressed();
                let public_key_hash = Keccak256::digest(&public_key_bytes[1..]); // Skip the 0x04 prefix
                let address = hex::encode(&public_key_hash[12..]); // Use the last 20 bytes

                EthereumKeyPair {
                    private_key: secret_key,
                    public_key,
                    address,
                }
            })
        })
    }

    fn get_vanity_search_address(&self) -> &str {
        &self.address
    }
}

impl EthereumKeyPair {
    pub fn get_private_key_as_hex(&self) -> String {
        hex::encode(self.private_key.secret_bytes())
    }

    pub fn get_private_key_as_hex_with_prefix(&self) -> String {
        format!("0x{}", hex::encode(self.private_key.secret_bytes()))
    }

    pub fn get_address_with_prefix(&self) -> String {
        format!("0x{}", self.address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::Secp256k1;
    use sha3::{Digest, Keccak256};

    #[test]
    fn test_generate_random() {
        let secp = Secp256k1::new();

        // Generate a random Ethereum key pair and address
        let key_pair = EthereumKeyPair::generate_random();

        // Derive public key from private key
        let derived_public_key = PublicKey::from_secret_key(&secp, &key_pair.private_key);
        assert_eq!(key_pair.public_key, derived_public_key);

        // Derive Ethereum address from public key
        let public_key_bytes = derived_public_key.serialize_uncompressed();
        let public_key_hash = Keccak256::digest(&public_key_bytes[1..]); // Skip the 0x04 prefix
        let derived_address = hex::encode(&public_key_hash[12..]); // Use the last 20 bytes

        assert_eq!(key_pair.address, derived_address);
    }
}
