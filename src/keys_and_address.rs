//! # Key Pair and Address Generation Module
//!
//! This module is to get a randomly generated key pair and their address.
//!
//! # Example Usage At Your Code
//! ```rust
//! use btc_vanity::keys_and_address::KeysAndAddress;
//!
//! // reason of using heavy is not importing bitcoin::secp256k1::Secp256k1
//! // if you want to create secp256k1 once and use it for every generation
//! // use KeysAndAddress::generate_random(secp256k1); instead.
//! let random_address = KeysAndAddress::generate_random_heavy();
//!
//! println!("A randomly generated key pair and their address\n\
//!           private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 random_address.get_wif_private_key(),
//!                 random_address.get_comp_public_key(),
//!                 random_address.get_comp_address())
//! ```

use bitcoin::key::{PrivateKey, PublicKey};
use bitcoin::secp256k1::{rand, All, Secp256k1};
use bitcoin::Address;
use bitcoin::Network::Bitcoin;

/// A struct to hold bitcoin::secp256k1::SecretKey bitcoin::Key::PublicKey and a string address
pub struct KeysAndAddress {
    private_key: PrivateKey,
    public_key: PublicKey,
    comp_address: String,
}

impl KeysAndAddress {
    /// Generates a randomly generated key pair and their compressed addresses with using given Secp256k1.
    /// and Returns them in a KeysAndAddress struct.
    pub fn generate_random(secp256k1: &Secp256k1<All>) -> Self {
        let (secret_key, pk) = secp256k1.generate_keypair(&mut rand::thread_rng());
        let private_key = PrivateKey::new(secret_key, Bitcoin);
        let public_key = PublicKey::new(pk);

        KeysAndAddress {
            private_key,
            public_key,
            comp_address: Address::p2pkh(public_key, Bitcoin).to_string(),
        }
    }

    /// Generates a randomly generated key pair and their compressed addresses with generating a new Secp256k1.
    /// and Returns them in a KeysAndAddress struct.
    pub fn generate_random_heavy() -> Self {
        let secp256k1 = Secp256k1::new();
        let (secret_key, pk) = secp256k1.generate_keypair(&mut rand::thread_rng());
        let private_key = PrivateKey::new(secret_key, Bitcoin);
        let public_key = PublicKey::new(pk);

        KeysAndAddress {
            private_key,
            public_key,
            comp_address: Address::p2pkh(public_key, Bitcoin).to_string(),
        }
    }

    pub fn get_private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn get_public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn get_comp_address(&self) -> &String {
        &self.comp_address
    }

    pub fn get_wif_private_key(&self) -> String {
        self.private_key.to_wif()
    }

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
        let keys_and_address = KeysAndAddress::generate_random(&secp);

        // Check if the private key can generate the same public key
        let derived_public_key = PublicKey::from_private_key(&secp, &keys_and_address.private_key);
        assert_eq!(keys_and_address.public_key, derived_public_key);

        // Check if the derived public key generates the same address
        let derived_address = Address::p2pkh(&derived_public_key, Bitcoin).to_string();
        assert_eq!(keys_and_address.comp_address, derived_address);
    }

    #[test]
    fn test_generate_random_heavy() {
        // Generate a random key pair and address
        let keys_and_address = KeysAndAddress::generate_random_heavy();

        // Check if the private key can generate the same public key
        let secp = Secp256k1::new();
        let derived_public_key = PublicKey::from_private_key(&secp, &keys_and_address.private_key);
        assert_eq!(keys_and_address.public_key, derived_public_key);

        // Check if the derived public key generates the same address
        let derived_address = Address::p2pkh(&derived_public_key, Bitcoin).to_string();
        assert_eq!(keys_and_address.comp_address, derived_address);
    }
}