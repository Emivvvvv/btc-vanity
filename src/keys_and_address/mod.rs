pub mod btc;
pub mod eth;

use bitcoin::{PrivateKey, PublicKey};
use secp256k1::{PublicKey as SecpPublicKey, SecretKey};

/// A struct to hold generated Bitcoin keypair and their address.
/// private_key: `bitcoin::PrivateKey`
/// public_key: `bitcoin::PublicKey`
/// comp_address: String
pub struct BitcoinKeyPair {
    private_key: PrivateKey,
    public_key: PublicKey,
    comp_address: String,
}

/// A struct to hold generated Bitcoin keypair and their address.
/// private_key: `secp256k1::SecretKey`
/// public_key: `secp256k1::PublicKey`
/// comp_address: String
pub struct EthereumKeyPair {
    private_key: SecretKey,
    #[allow(dead_code)]
    public_key: SecpPublicKey,
    address: String,
}
