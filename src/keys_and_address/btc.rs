//! # Bitcoin Key Pair and Address Generation

use bitcoin::key::rand::rngs::ThreadRng;
use bitcoin::key::{PrivateKey, PublicKey};
use bitcoin::secp256k1::{rand, All, Secp256k1};
use bitcoin::Address;
use bitcoin::Network::Bitcoin;
use std::cell::RefCell;

use crate::keys_and_address::{AddressGenerator, BitcoinKeyPair};

thread_local! {
    static THREAD_LOCAL_SECP256K1: Secp256k1<All> = Secp256k1::new();
    static THREAD_LOCAL_RNG: RefCell<ThreadRng> = RefCell::new(rand::thread_rng());
}

impl AddressGenerator for BitcoinKeyPair {
    /// Generates a randomly generated Bitcoin key pair and their compressed address.
    /// Returns `BitcoinKeyPair` struct.
    fn generate_random() -> Self {
        THREAD_LOCAL_SECP256K1.with(|secp256k1| {
            THREAD_LOCAL_RNG.with(|rng| {
                let mut rng = rng.borrow_mut(); // Mutably borrow the RNG
                let (secret_key, pk) = secp256k1.generate_keypair(&mut *rng); // Use thread-local RNG
                let private_key = PrivateKey::new(secret_key, Bitcoin);

                println!("secret key: {:?}", secret_key);
                println!("private key: {:?}", private_key);

                let public_key = PublicKey::new(pk);

                BitcoinKeyPair {
                    private_key,
                    public_key,
                    comp_address: Address::p2pkh(public_key, Bitcoin).to_string(),
                }
            })
        })
    }

    fn get_vanity_search_address(&self) -> &str {
        unsafe { self.comp_address.get_unchecked(1..) }
    }
}

impl BitcoinKeyPair {
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
        let keys_and_address = BitcoinKeyPair::generate_random();

        // Check if the private key can generate the same public key
        let derived_public_key = PublicKey::from_private_key(&secp, &keys_and_address.private_key);
        assert_eq!(keys_and_address.public_key, derived_public_key);

        // Check if the derived public key generates the same address
        let derived_address = Address::p2pkh(&derived_public_key, Bitcoin).to_string();
        assert_eq!(keys_and_address.comp_address, derived_address);
    }
}
