//! # Key Pair and Address Generation Module
//!
//! This module provides functionality for generating random key pairs and their associated addresses.
//! Supported cryptocurrencies are `Bitcoin`, and `Ethereum`.

pub mod btc;
pub mod eth;

use std::array::from_fn;

use crate::BATCH_SIZE;

use bitcoin::{PrivateKey, PublicKey};
use secp256k1::{PublicKey as SecpPublicKey, SecretKey};

/// A trait to handle generic key pair and address creation.
/// Used in vanity address generation.
///
/// Implemented by `BitcoinKeyPair`, and `EthereumKeyPair`.
pub trait KeyPairGenerator {
    /// Generates a random key pair.
    ///
    /// # Returns
    /// - A new instance of the implementing struct with generated keys and address.
    fn generate_random() -> Self
    where
        Self: Sized;

    /// Retrieves the address associated with the key pair.
    fn get_address(&self) -> &String;

    /// Retrieves the address bytes associated with the key pair.
    fn get_address_bytes(&self) -> &[u8];

    fn generate_batch() -> [Self; BATCH_SIZE]
    where
        Self: Sized,
    {
        from_fn(|_| Self::generate_random())
    }

    /// Fills an existing array with newly generated key pairs.
    ///
    /// We simply iterate and overwrite each slot with a call
    /// to `Self::generate_random()`.
    fn fill_batch(batch_array: &mut [Self; BATCH_SIZE])
    where
        Self: Sized,
    {
        for slot in batch_array.iter_mut() {
            *slot = Self::generate_random();
        }
    }
}

/// A struct representing a Bitcoin key pair and its associated address.
/// Implements `KeyPairGenerator` and `Send` traits.
pub struct BitcoinKeyPair {
    /// A Bitcoin private key. `bitcoin::PrivateKey`
    private_key: PrivateKey,
    /// A Bitcoin public key. `bitcoin::PublicKey`
    public_key: PublicKey,
    /// The compressed Bitcoin address as a `String`.
    comp_address: String,
}

unsafe impl Send for BitcoinKeyPair {}

/// A struct representing an Ethereum key pair and its associated address.
/// Implements `KeyPairGenerator` and `Send` traits.
pub struct EthereumKeyPair {
    /// An Ethereum private key. `secp256k1::SecretKey`
    private_key: SecretKey,
    /// An Ethereum public key. `secp256k1::PublicKey`
    public_key: SecpPublicKey,
    /// The Ethereum address as a `String`.
    address: String,
}

unsafe impl Send for EthereumKeyPair {}
