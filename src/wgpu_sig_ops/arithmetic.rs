use num_bigint::{BigInt, BigUint, Sign};

pub fn calc_num_limbs(log_limb_size: u32, bit_width: usize) -> usize {
    assert!(log_limb_size > 0);

    if bit_width == 256 && log_limb_size == 15 {
        return 19;
    }

    bit_width / log_limb_size as usize + 1
}

pub fn calc_bitwidth(value: &BigUint) -> usize {
    value.bits() as usize
}

pub fn biguint_to_limbs_le(value: &BigUint, num_limbs: usize, log_limb_size: u32) -> Vec<u32> {
    let mask = BigUint::from((1u32 << log_limb_size) - 1);

    (0..num_limbs)
        .map(|index| {
            let limb = (value >> (index as u32 * log_limb_size)) & &mask;
            limb.to_u32_digits().first().copied().unwrap_or(0)
        })
        .collect()
}

pub fn biguint_to_bytes_be(value: &BigUint, num_bytes: usize) -> Vec<u8> {
    let bytes = value.to_bytes_be();
    if bytes.len() >= num_bytes {
        return bytes;
    }

    let mut padded = vec![0; num_bytes - bytes.len()];
    padded.extend(bytes);
    padded
}

pub fn bytes_34_to_limbs_32(bytes: &[u8]) -> Vec<u32> {
    assert_eq!(bytes.len(), 34);
    let mut limbs = vec![0; 32];

    for (index, pair) in bytes.chunks_exact(2).enumerate() {
        limbs[16 - index] = u16::from_be_bytes([pair[0], pair[1]]) as u32;
    }

    limbs
}

pub fn calc_nsafe(log_limb_size: u32) -> usize {
    let limb_product = 1u64 << (2 * log_limb_size);
    let mut count = 1usize;

    while count as u64 * limb_product <= 1u64 << 32 {
        count += 1;
    }

    count / 2
}

pub fn calc_mont_radix(num_limbs: usize, log_limb_size: u32) -> BigUint {
    BigUint::from(2u32).pow(num_limbs as u32 * log_limb_size)
}

fn extended_gcd(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
    if *a == BigInt::from(0u32) {
        return (b.clone(), BigInt::from(0u32), BigInt::from(1u32));
    }

    let (gcd, x, y) = extended_gcd(&(b % a), a);
    (gcd, y - (b / a) * &x, x)
}

pub fn calc_rinv_and_n0(modulus: &BigUint, radix: &BigUint, log_limb_size: u32) -> (BigUint, u32) {
    assert_ne!(*radix, BigUint::from(0u32));

    let modulus_int = BigInt::from_biguint(Sign::Plus, modulus.clone());
    let radix_int = BigInt::from_biguint(Sign::Plus, radix.clone());
    let (gcd, mut radix_inverse, mut modulus_inverse) = extended_gcd(&radix_int, &modulus_int);
    assert_eq!(gcd, BigInt::from(1u32));

    if radix_inverse.sign() == Sign::Minus {
        radix_inverse += &modulus_int;
    }
    if modulus_inverse.sign() == Sign::Minus {
        modulus_inverse += &radix_int;
    }

    let negative_inverse = &radix_int - modulus_inverse;
    let limb_radix = BigInt::from(1u32 << log_limb_size);
    let n0 = (negative_inverse % limb_radix)
        .to_biguint()
        .unwrap()
        .to_u32_digits()
        .first()
        .copied()
        .unwrap_or(0);

    (radix_inverse.to_biguint().unwrap(), n0)
}

pub fn gen_mu(modulus: &BigUint) -> BigUint {
    assert_ne!(*modulus, BigUint::from(0u32));
    let mut exponent = 1u32;
    while BigUint::from(2u32).pow(exponent) < *modulus {
        exponent += 1;
    }
    BigUint::from(4u32).pow(exponent) / modulus
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    fn secp256k1_modulus() -> BigUint {
        BigUint::parse_bytes(
            b"fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f",
            16,
        )
        .unwrap()
    }

    #[test]
    fn sizing_helpers_cover_supported_limb_sizes() {
        assert_eq!(
            (11..=15)
                .map(|size| calc_num_limbs(size, 256))
                .collect::<Vec<_>>(),
            [24, 22, 20, 19, 19]
        );
        assert_eq!(calc_bitwidth(&BigUint::from(0u32)), 0);
        assert_eq!(calc_bitwidth(&BigUint::from(1u32)), 1);
        assert_eq!(calc_bitwidth(&secp256k1_modulus()), 256);
    }

    #[test]
    fn conversion_helpers_preserve_the_value() {
        let value = secp256k1_modulus();

        for log_limb_size in 11..=15 {
            let num_limbs = calc_num_limbs(log_limb_size, 256);
            let limbs = biguint_to_limbs_le(&value, num_limbs, log_limb_size);
            let restored = limbs
                .iter()
                .enumerate()
                .fold(BigUint::from(0u32), |acc, (index, limb)| {
                    acc + (BigUint::from(*limb) << (index as u32 * log_limb_size))
                });
            assert_eq!(restored, value);
        }

        let bytes = biguint_to_bytes_be(&value, 34);
        assert_eq!(bytes.len(), 34);
        assert_eq!(BigUint::from_bytes_be(&bytes), value);
        assert_eq!(bytes_34_to_limbs_32(&bytes).len(), 32);
    }

    #[test]
    fn montgomery_helpers_satisfy_modular_invariants() {
        let modulus = secp256k1_modulus();
        let expected_nsafe = [512, 128, 32, 8, 2];

        for (log_limb_size, expected_nsafe) in (11..=15).zip(expected_nsafe) {
            let num_limbs = calc_num_limbs(log_limb_size, 256);
            let radix = calc_mont_radix(num_limbs, log_limb_size);
            assert_eq!(
                radix,
                BigUint::from(1u32) << (num_limbs as u32 * log_limb_size)
            );
            assert_eq!(calc_nsafe(log_limb_size), expected_nsafe);

            let (radix_inverse, n0) = calc_rinv_and_n0(&modulus, &radix, log_limb_size);
            assert_eq!((&radix * radix_inverse) % &modulus, BigUint::from(1u32));

            let limb_radix = BigUint::from(1u32 << log_limb_size);
            assert_eq!(
                (&modulus * BigUint::from(n0) + BigUint::from(1u32)) % limb_radix,
                BigUint::from(0u32)
            );
        }
    }

    #[test]
    fn field_constant_has_expected_precision() {
        let modulus = secp256k1_modulus();
        assert_eq!(gen_mu(&modulus), BigUint::from(4u32).pow(256) / modulus);
    }
}
