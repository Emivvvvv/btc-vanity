//! # Key Pair and Address Generation Module
//!
//! This module is used to get a randomly generated key pair and their address.
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

use crate::error::BtcVanityError;
use bitcoin::key::{PrivateKey, PublicKey};
use bitcoin::secp256k1::{rand, All, Secp256k1};
use bitcoin::Address;
use bitcoin::Network::Bitcoin;
use num_bigint::{BigUint, RandBigInt};
use num_traits::Num;

/// A struct to hold bitcoin::secp256k1::SecretKey bitcoin::Key::PublicKey and a string address
pub struct KeysAndAddress {
    private_key: PrivateKey,
    public_key: PublicKey,
    comp_address: String,
}

impl KeysAndAddress {
    /// Generates a randomly generated key pair and their compressed addresses without generating a new Secp256k1.
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

    /// Use safe mode if you're calling this function out of vanity_addr_generator.rs
    /// Generates a randomly generated key pair and their compressed addresses within a custom range for the private key.
    /// Returns them in a KeysAndAddress struct.
    /// The range is defined by `range_min` and `range_max` as BigUint to handle 256-bit values.
    pub fn generate_within_range(
        s: &Secp256k1<All>,
        range_min: &BigUint,
        range_max: &BigUint,
        safe_mode: bool,
    ) -> Result<Self, BtcVanityError> {
        if safe_mode {
            // Ensure range_max is greater than range_min
            if range_max < range_min {
                return Err(BtcVanityError::KeysAndAddressError(
                    "range_max must be greater than range_min",
                ));
            }

            let secp256k1_order = BigUint::from_str_radix(
                "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
                16,
            )
            .map_err(|_| {
                BtcVanityError::KeysAndAddressError("Failed to parse hexadecimal string")
            })?;

            if range_max > &secp256k1_order {
                return Err(BtcVanityError::KeysAndAddressError(
                    "range_max must be within the valid range for Secp256k1",
                ));
            }
        }

        // Generate a random private key within the specified range
        let mut rng = rand::thread_rng();
        let private_key_value = rng.gen_biguint_range(range_min, range_max);

        // Convert the BigUint to a 32-byte array, zero-padded on the left
        let private_key_bytes = {
            let mut bytes = [0u8; 32];
            let private_key_vec = private_key_value.to_bytes_be();
            let start_index = 32 - private_key_vec.len();
            bytes[start_index..].copy_from_slice(&private_key_vec);
            bytes
        };

        let private_key = PrivateKey::from_slice(&private_key_bytes, Bitcoin)
            .map_err(|_| BtcVanityError::KeysAndAddressError("Invalid private key"))?;
        let public_key = PublicKey::from_private_key(s, &private_key);

        Ok(KeysAndAddress {
            private_key,
            public_key,
            comp_address: Address::p2pkh(public_key, Bitcoin).to_string(),
        })
    }

    /// Use safe mode if you're calling this function out of vanity_addr_generator.rs
    /// Generates a the key pair and their compressed addresses from the given private key.
    /// Returns them in a KeysAndAddress struct.
    pub fn generate_from_biguint(
        s: &Secp256k1<All>,
        private_key_biguint: &BigUint,
        safe_mode: bool,
    ) -> Result<Self, BtcVanityError> {
        if safe_mode {
            if private_key_biguint == &BigUint::ZERO {
                return Err(BtcVanityError::KeysAndAddressError("renge_min can't be 0"));
            }

            let secp256k1_order = BigUint::from_str_radix(
                "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
                16,
            )
            .map_err(|_| {
                BtcVanityError::KeysAndAddressError("Failed to parse hexadecimal string")
            })?;

            if private_key_biguint > &secp256k1_order {
                return Err(BtcVanityError::KeysAndAddressError(
                    "range_max must be within the valid range for Secp256k1",
                ));
            }
        }

        // Convert the BigUint to a 32-byte array, zero-padded on the left
        let private_key_bytes = {
            let mut bytes = [0u8; 32];
            let private_key_vec = private_key_biguint.to_bytes_be();
            let start_index = 32 - private_key_vec.len();
            bytes[start_index..].copy_from_slice(&private_key_vec);
            bytes
        };

        let private_key = PrivateKey::from_slice(&private_key_bytes, Bitcoin)
            .map_err(|_| BtcVanityError::KeysAndAddressError("Invalid private key"))?;
        let public_key = PublicKey::from_private_key(s, &private_key);

        Ok(KeysAndAddress {
            private_key,
            public_key,
            comp_address: Address::p2pkh(public_key, Bitcoin).to_string(),
        })
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
    use num_bigint::BigUint;
    use num_traits::{FromPrimitive, One};

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

    #[test]
    fn test_generate_with_custom_range() {
        let secp = Secp256k1::new();

        // We're testing private key `1`
        let range_min = BigUint::from_u8(1).unwrap();
        let range_max = BigUint::from_u8(2).unwrap();

        let result = KeysAndAddress::generate_within_range(&secp, &range_min, &range_max, true);

        let keys_and_address = result.unwrap();

        let private_key_as_array = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ];
        let private_key_wif = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
        let public_key_comp = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let address_comp = "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH";

        // Verify that the private key and other values are correct.
        assert_eq!(
            private_key_as_array,
            keys_and_address.private_key.to_bytes().as_slice()
        );
        assert_eq!(private_key_wif, keys_and_address.private_key.to_wif());
        assert_eq!(public_key_comp, keys_and_address.public_key.to_string());
        assert_eq!(address_comp, keys_and_address.comp_address);
    }

    #[test]
    #[should_panic(expected = "range_max must be greater than range_min")]
    fn test_generate_with_invalid_range() {
        let secp = Secp256k1::new();

        // Set range_min greater than range_max to trigger to assert
        let range_min = BigUint::from(100u32);
        let range_max = BigUint::from(10u32);

        let _ = KeysAndAddress::generate_within_range(&secp, &range_min, &range_max, true).unwrap();
    }

    #[test]
    #[should_panic(expected = "range_max must be within the valid range for Secp256k1")]
    fn test_generate_with_out_of_bounds_range() {
        let secp = Secp256k1::new();

        // Set range_max greater than the secp256k1 order
        let range_min = BigUint::one();
        let range_max = BigUint::from_str_radix(
            "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364142", // One more than the secp256k1 order
            16,
        )
        .unwrap();

        let _ = KeysAndAddress::generate_within_range(&secp, &range_min, &range_max, true).unwrap();
    }
}
