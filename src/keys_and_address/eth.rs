use rand::{rngs::ThreadRng, RngCore};
use secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};
use std::cell::RefCell;

use crate::keys_and_address::EthereumKeyPair;

thread_local! {
    static THREAD_LOCAL_SECP256K1: Secp256k1<All> = Secp256k1::new();
    static THREAD_LOCAL_RNG: RefCell<ThreadRng> = RefCell::new(rand::rng());
}

impl EthereumKeyPair {
    pub fn generate_random() -> Self {
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

    pub fn get_private_key_as_hex(&self) -> String {
        hex::encode(self.private_key.secret_bytes())
    }

    pub fn get_address(&self) -> &String {
        &self.address
    }

    pub fn get_private_key_as_hex_with_prefix(&self) -> String {
        format!("0x{}", hex::encode(self.private_key.secret_bytes()))
    }

    pub fn get_address_with_prefix(&self) -> String {
        format!("0x{}", self.address)
    }
}
