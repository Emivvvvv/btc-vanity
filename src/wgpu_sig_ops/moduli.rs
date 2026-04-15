use ark_ff::{BigInteger, PrimeField};
use num_bigint::BigUint;

pub fn secp256k1_fq_modulus_biguint() -> BigUint {
    BigUint::from_bytes_be(&ark_secp256k1::Fq::MODULUS.to_bytes_be())
}

pub fn secp256k1_fr_modulus_biguint() -> BigUint {
    BigUint::from_bytes_be(&ark_secp256k1::Fr::MODULUS.to_bytes_be())
}

pub fn ed25519_fq_modulus_biguint() -> BigUint {
    BigUint::from_bytes_be(&ark_ed25519::Fq::MODULUS.to_bytes_be())
}

pub fn ed25519_fr_modulus_biguint() -> BigUint {
    BigUint::from_bytes_be(&ark_ed25519::Fr::MODULUS.to_bytes_be())
}

