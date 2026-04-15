//! # Key Pair and Address Generation Module
//!
//! This module provides functionality for generating random key pairs and their associated addresses.
//! Supported cryptocurrencies are `Bitcoin`, `Ethereum`, and `Solana`.

pub mod btc;
#[cfg(feature = "ethereum")]
pub mod eth;
#[cfg(feature = "solana")]
pub mod sol;

use std::array::from_fn;

use crate::BATCH_SIZE;

use bitcoin::{PrivateKey, PublicKey};
#[cfg(feature = "ethereum")]
use secp256k1::{PublicKey as SecpPublicKey, SecretKey};
#[cfg(feature = "solana")]
use solana_sdk::signature::Keypair;

/// A trait to handle generic key pair and address creation.
/// Used in vanity address generation.
///
/// Implemented by `BitcoinKeyPair`, `EthereumKeyPair`, and `SolanaKeyPair`.
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
    pub(crate) private_key: PrivateKey,
    /// A Bitcoin public key. `bitcoin::PublicKey`
    pub(crate) public_key: PublicKey,
    /// The compressed Bitcoin address as a `String`.
    pub(crate) comp_address: String,
}

// SAFETY: `BitcoinKeyPair` contains owned key/address value types with no
// interior thread-affine state; moving ownership across threads is sound.
unsafe impl Send for BitcoinKeyPair {}

/// A struct representing an Ethereum key pair and its associated address.
/// Implements `KeyPairGenerator` and `Send` traits.
#[cfg(feature = "ethereum")]
pub struct EthereumKeyPair {
    /// An Ethereum private key. `secp256k1::SecretKey`
    pub(crate) private_key: SecretKey,
    /// An Ethereum public key. `secp256k1::PublicKey`
    pub(crate) public_key: SecpPublicKey,
    /// The Ethereum address as a `String`.
    pub(crate) address: String,
}

#[cfg(feature = "ethereum")]
// SAFETY: `EthereumKeyPair` stores owned secp256k1 key values and a `String`.
// It has no shared mutable aliasing or thread-affine resources.
unsafe impl Send for EthereumKeyPair {}

/// A struct representing a Solana key pair and its associated address.
/// Implements `KeyPairGenerator` and `Send` traits.
#[cfg(feature = "solana")]
pub struct SolanaKeyPair {
    /// A Solana `solana_sdk::signer::Keypair` struct.
    pub(crate) keypair: Keypair,
    /// The Solana address as a `String`.
    pub(crate) address: String,
}

#[cfg(feature = "solana")]
// SAFETY: `SolanaKeyPair` owns its key material and address string and does
// not rely on thread-local state; moving it between threads is safe.
unsafe impl Send for SolanaKeyPair {}
