//! # Key Pair and Address Generation Module
//!
//! This module is used to get a randomly generated key pair and their address.
//!
//! # Example Usage At Your Code
//! ```rust
//! use btc_vanity::keys_and_address::KeysAndAddressString;
//!
//! let random_address = KeysAndAddressString::generate_random();
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
use bitcoin::secp256k1::{All, rand, Secp256k1, SecretKey};

/// A struct to hold wif private_key, compressed public_key and their compressed address
pub struct KeysAndAddressString {
    wif_private_key: String,
    comp_public_key: String,
    comp_address: String,
}

impl KeysAndAddressString {
    /// Generates a randomly generated key pair (wif private key and compressed public key) and their compressed addresses
    /// and Returns them in a KeysAndAddressString struct.
    pub fn generate_random() -> Self {
        // Generate random key pair.
        let s = Secp256k1::new();
        let (sk , pk) = s.generate_keypair(&mut rand::thread_rng());
        let private_key = PrivateKey::new(sk, Bitcoin);
        let public_key = PublicKey::new(pk);

        // Generate pay-to-pubkey-hash address.
        let address = Address::p2pkh(&public_key, Bitcoin);

        KeysAndAddressString {
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

    pub fn fast_engine_get(secret_key: &SecretKey, public_key: PublicKey, address: String) -> Self {
        KeysAndAddressString {
            wif_private_key: PrivateKey::new(*secret_key, Bitcoin).to_wif(),
            comp_public_key: public_key.to_string(),
            comp_address: address,
        }
    }
}

/// A struct to hold bitcoin::secp256k1::SecretKey bitcoin::Key::PublicKey and a string address
pub struct KeysAndAddress {
    secret_key: SecretKey,
    public_key: PublicKey,
    comp_address: String,
}

impl KeysAndAddress {
    /// Generates a randomly generated key pair and their compressed addresses without generating a new Secp256k1.
    /// and Returns them in a KeysAndAddress struct.
    pub fn generate_random(s: &Secp256k1<All>) -> Self {
        let (secret_key, pk) = s.generate_keypair(&mut rand::thread_rng());
        let public_key = PublicKey::new(pk);

        KeysAndAddress {
            secret_key,
            public_key,
            comp_address: Address::p2pkh(&public_key, Bitcoin).to_string(),
        }
    }

    pub fn get_secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn get_public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn get_comp_address(&self) -> &String {
        &self.comp_address
    }
}