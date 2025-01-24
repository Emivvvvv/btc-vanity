//! # Bitcoin Key Pair and Address Generation
//!
//! This module provides functionality to generate Bitcoin key pairs and their associated compressed addresses.

use std::cell::RefCell;

use crate::keys_and_address::{BitcoinKeyPair, KeyPairGenerator};

use bitcoin::key::rand::rngs::ThreadRng;
use bitcoin::key::{PrivateKey, PublicKey};
use bitcoin::secp256k1::{rand, All, Secp256k1};
use bitcoin::Address;
use bitcoin::Network::Bitcoin;

thread_local! {
    static THREAD_LOCAL_SECP256K1: Secp256k1<All> = Secp256k1::new();
    static THREAD_LOCAL_RNG: RefCell<ThreadRng> = RefCell::new(rand::thread_rng());
}

impl KeyPairGenerator for BitcoinKeyPair {
    /// Generates a random Bitcoin key pair and its compressed address.
    ///
    /// # Returns
    /// - A [BitcoinKeyPair] struct containing the private key, public key, and address.
    #[inline(always)]
    fn generate_random() -> Self {
        THREAD_LOCAL_SECP256K1.with(|secp256k1| {
            THREAD_LOCAL_RNG.with(|rng| {
                let mut rng = rng.borrow_mut();
                let (secret_key, pk) = secp256k1.generate_keypair(&mut *rng);

                let private_key = PrivateKey::new(secret_key, Bitcoin);
                let public_key = PublicKey::new(pk);

                BitcoinKeyPair {
                    private_key,
                    public_key,
                    comp_address: Address::p2pkh(public_key, Bitcoin).to_string(),
                }
            })
        })
    }

    /// Retrieves the compressed Bitcoin address as `String` reference.
    #[inline(always)]
    fn get_address(&self) -> &String {
        &self.comp_address
    }

    /// Retrieves the compressed Bitcoin address in byte slice format.
    #[inline(always)]
    fn get_address_bytes(&self) -> &[u8] {
        self.comp_address.as_bytes()
    }
}

impl BitcoinKeyPair {
    /// Retrieves the [PrivateKey] reference of the [BitcoinKeyPair].
    pub fn get_private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    /// Retrieves the [PublicKey] reference of the [BitcoinKeyPair].
    pub fn get_public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Retrieves the compressed address reference.
    pub fn get_comp_address(&self) -> &String {
        &self.comp_address
    }

    /// Retrieves the private key in Wallet Import Format (WIF) as a `String`.
    pub fn get_wif_private_key(&self) -> String {
        self.private_key.to_wif()
    }

    /// Retrieves the compressed public key as a `String`.
    pub fn get_comp_public_key(&self) -> String {
        self.public_key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::Secp256k1;

    #[test]
    fn test_generate_random() {
        let secp = Secp256k1::new();

        // Generate a random key pair and address
        let keys_and_address = BitcoinKeyPair::generate_random();

        // Check if the private key can generate the same public key
        let derived_public_key = PublicKey::from_private_key(&secp, &keys_and_address.private_key);
        assert_eq!(keys_and_address.public_key, derived_public_key);

        // Check if the derived public key generates the same address
        let derived_address = Address::p2pkh(derived_public_key, Bitcoin).to_string();
        assert_eq!(keys_and_address.comp_address, derived_address);
    }
}
