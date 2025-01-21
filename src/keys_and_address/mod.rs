//! # Key Pair and Address Generation Module
//!
//! This module is to get a randomly generated key pair and their address.
//! Currently, Bitcoin and Ethereum key pair and address generation is supported.

pub mod btc;
pub mod eth;

use bitcoin::{PrivateKey, PublicKey};
use secp256k1::{PublicKey as SecpPublicKey, SecretKey};

/// Trait to generic keypair and address creation.
/// Implemented by `BitcoinKeyPair` and `EthereumKeyPair`.
pub trait KeyPairGenerator {
    /// Generates a random keypair.
    fn generate_random() -> Self
    where
        Self: Sized;

    /// Retrieves the address associated with the keypair.
    fn get_vanity_search_address(&self) -> &str;
}

/// A struct to hold generated Bitcoin keypair and their address.
/// Implements `AddressGenerator` trait.
///
/// private_key: `bitcoin::PrivateKey`
/// public_key: `bitcoin::PublicKey`
/// comp_address: String
pub struct BitcoinKeyPair {
    private_key: PrivateKey,
    public_key: PublicKey,
    comp_address: String,
}

unsafe impl Send for BitcoinKeyPair {}

/// A struct to hold generated Bitcoin keypair and their address.
/// Implements `AddressGenerator` trait.
///
/// private_key: `secp256k1::SecretKey`
/// public_key: `secp256k1::PublicKey`
/// comp_address: String
pub struct EthereumKeyPair {
    private_key: SecretKey,
    #[allow(dead_code)]
    public_key: SecpPublicKey,
    address: String,
}

unsafe impl Send for EthereumKeyPair {}
