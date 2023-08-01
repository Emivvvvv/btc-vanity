use bitcoin::{Address, PrivateKey};
use bitcoin::key::PublicKey;
use bitcoin::Network::Bitcoin;
use bitcoin::secp256k1::{rand, Secp256k1};

pub struct KeysAndAddress {
    wif_private_key: String,
    comp_public_key: String,
    comp_address: String,
}

impl KeysAndAddress {
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