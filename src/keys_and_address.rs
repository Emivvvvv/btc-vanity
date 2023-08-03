//! # Key Pair and Address Generation Module
//!
//! This module is used to get a randomly generated key pair and their address.
//!
//! # Example Usage At Your Code
//! ```rust
//! use btc_vanity::keys_and_address::KeysAndAddress;
//!
//! let random_address = KeysAndAddress::generate_random();
//!
//! println!("A randomly generated key pair and their address\n\
//!           private_key (wif): {}\n\
//!           public_key (compressed): {}\n\
//!           address (compressed): {}\n\n",
//!                 random_address.get_wif_private_key(),
//!                 random_address.get_comp_public_key(),
//!                 random_address.get_comp_address())
//! ```

use bitcoin::{Address, PrivateKey};
use bitcoin::key::PublicKey;
use bitcoin::Network::Bitcoin;
use bitcoin::secp256k1::{rand, Secp256k1};

/// A struct to hold wif private_key, compressed public_key and their compressed address
pub struct KeysAndAddress {
    wif_private_key: String,
    comp_public_key: String,
    comp_address: String,
}

impl KeysAndAddress {
    /// Generates a randomly generated key pair (wif private key and compressed public key) and their compressed addresses
    /// and Returns them in a KeysAndAddress struct.
    pub fn generate_random() -> Self {
        // Generate random key pair.
        let s = Secp256k1::new();
        let (sk , pk) = s.generate_keypair(&mut rand::thread_rng());
        let private_key = PrivateKey::new(sk, Bitcoin);
        let public_key = PublicKey::new(pk);

        // Generate pay-to-pubkey-hash address.
        let address = Address::p2pkh(&public_key, Bitcoin);

        KeysAndAddress {
            wif_private_key: private_key.to_wif(),
            comp_public_key: public_key.to_string(),
            comp_address: address.to_string(),
        }
    }

    pub fn get_wif_private_key(&self) -> &String {
        &self.wif_private_key
    }

    pub fn get_comp_public_key(&self) -> &String {
        &self.comp_public_key
    }

    pub fn get_comp_address(&self) -> &String {
        &self.comp_address
    }
}