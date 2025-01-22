//! # Key Pair and Address Generation Module
//!
//! This module is to get a randomly generated key pair and their address.
//! Currently, Bitcoin and Ethereum key pair and address generation is supported.

pub mod btc;
pub mod eth;
mod sol;

use bitcoin::{PrivateKey, PublicKey};
use secp256k1::{PublicKey as SecpPublicKey, SecretKey};
use solana_sdk::signature::Keypair;

/// Trait to generic keypair and address creation.
/// Implemented by `BitcoinKeyPair` and `EthereumKeyPair`.
pub trait KeyPairGenerator {
    /// Generates a random keypair.
    fn generate_random() -> Self
    where
        Self: Sized;

    /// Retrieves the address associated with the keypair.
    fn get_address(&self) -> &String;

    /// Retrieves the address bytes associated with the keypair.
    fn get_address_bytes(&self) -> &[u8];
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

/// A struct to hold generated Ethereum keypair and their address.
/// Implements `AddressGenerator` trait.
///
/// private_key: `secp256k1::SecretKey`
/// public_key: `secp256k1::PublicKey`
/// address: String
pub struct EthereumKeyPair {
    pub private_key: SecretKey,
    #[allow(dead_code)]
    pub public_key: SecpPublicKey,
    address: String,
}

unsafe impl Send for EthereumKeyPair {}

/// A struct to hold generated Solana keypair and their address.
/// Implements `AddressGenerator` trait.
///
/// keypair: `solana_sdk::signer::Keypair`
/// address: String
pub struct SolanaKeyPair {
    pub keypair: Keypair,
    address: String,
}

unsafe impl Send for SolanaKeyPair {}
