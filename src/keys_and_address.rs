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

use bitcoin::key::{PrivateKey, PublicKey};
use bitcoin::secp256k1::{rand, All, Secp256k1};
use bitcoin::Address;
use bitcoin::Network::Bitcoin;
use num_bigint::{BigUint, RandBigInt};
use num_traits::Num;

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
        let (sk, pk) = s.generate_keypair(&mut rand::thread_rng());
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

    pub fn fast_engine_get(
        private_key: &PrivateKey,
        public_key: PublicKey,
        address: String,
    ) -> Self {
        KeysAndAddressString {
            wif_private_key: private_key.to_wif(),
            comp_public_key: public_key.to_string(),
            comp_address: address,
        }
    }
}

/// A struct to hold bitcoin::secp256k1::SecretKey bitcoin::Key::PublicKey and a string address
pub struct KeysAndAddress {
    private_key: PrivateKey,
    public_key: PublicKey,
    comp_address: String,
}

impl KeysAndAddress {
    /// Generates a randomly generated key pair and their compressed addresses without generating a new Secp256k1.
    /// and Returns them in a KeysAndAddress struct.
    pub fn generate_random(s: &Secp256k1<All>) -> Self {
        let (secret_key, pk) = s.generate_keypair(&mut rand::thread_rng());
        let private_key = PrivateKey::new(secret_key, Bitcoin);
        let public_key = PublicKey::new(pk);

        KeysAndAddress {
            private_key,
            public_key,
            comp_address: Address::p2pkh(&public_key, Bitcoin).to_string(),
        }
    }

    /// Generates a randomly generated key pair and their compressed addresses within a custom range for the private key.
    /// Returns them in a KeysAndAddress struct.
    /// The range is defined by `range_min` and `range_max` as BigUint to handle 256-bit values.
    pub fn generate_with_custom_range(
        s: &Secp256k1<All>,
        range_min: BigUint,
        range_max: BigUint,
    ) -> Self {
        // Ensure range_max is greater than range_min
        assert!(
            range_max > range_min,
            "range_max must be greater than range_min"
        );

        let secp256k1_order = BigUint::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .expect("Failed to parse hexadecimal string");

        assert!(
            range_max > secp256k1_order,
            "range_max must be within the valid range for Secp256k1"
        );

        // Generate a random private key within the specified range
        let mut rng = rand::thread_rng();
        let private_key_value = rng.gen_biguint_range(&range_min, &range_max);

        let mut private_key_bytes = [0u8; 32];
        let private_key_vec = private_key_value.to_bytes_be();
        private_key_bytes[..private_key_vec.len()].copy_from_slice(&private_key_vec);

        // Generate the key pair
        let private_key =
            PrivateKey::from_slice(&private_key_bytes, Bitcoin).expect("Invalid private key");
        let public_key = PublicKey::from_private_key(s, &private_key);

        KeysAndAddress {
            private_key,
            public_key,
            comp_address: Address::p2pkh(&public_key, Bitcoin).to_string(),
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
}
