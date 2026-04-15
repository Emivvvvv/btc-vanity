use crate::wgpu_sig_ops::curve_algos::coords::{ETEProjective, ProjectiveXYZ};
use crate::wgpu_sig_ops::curve_algos::ed25519_curve::affine_to_projective;
use crate::wgpu_sig_ops::curve_algos::precompute::precompute_table;
use crate::wgpu_sig_ops::curve_algos::secp256k1_curve;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ed25519::{EdwardsAffine, EdwardsProjective, Fq};
use ark_ff::{BigInteger, PrimeField};
use multiprecision::{bigint, mont};
use num_bigint::BigUint;

pub const WINDOW_SIZE: u32 = 4;

fn fq_to_biguint<F: PrimeField>(val: F) -> BigUint {
    let b = val.into_bigint().to_bytes_be();
    BigUint::from_bytes_be(&b)
}

fn projectivexy_to_mont_limbs<F: PrimeField>(
    a: &ProjectiveXYZ<F>,
    p: &BigUint,
    log_limb_size: u32,
) -> Vec<u32> {
    let num_limbs = multiprecision::utils::calc_num_limbs(log_limb_size, 256);
    let r = mont::calc_mont_radix(num_limbs, log_limb_size);
    let a_x_r = fq_to_biguint::<F>(a.x) * &r % p;
    let a_y_r = fq_to_biguint::<F>(a.y) * &r % p;
    let a_x_r_limbs = bigint::from_biguint_le(&a_x_r, num_limbs, log_limb_size);
    let a_y_r_limbs = bigint::from_biguint_le(&a_y_r, num_limbs, log_limb_size);
    let mut pt_a_limbs = Vec::<u32>::with_capacity(num_limbs * 2);
    pt_a_limbs.extend_from_slice(&a_x_r_limbs);
    pt_a_limbs.extend_from_slice(&a_y_r_limbs);
    pt_a_limbs
}

fn eteprojective_to_xyt_mont_limbs<F: PrimeField>(
    a: &ETEProjective<F>,
    p: &BigUint,
    log_limb_size: u32,
) -> Vec<u32> {
    let num_limbs = multiprecision::utils::calc_num_limbs(log_limb_size, 256);
    let r = mont::calc_mont_radix(num_limbs, log_limb_size);
    let a_x_r = fq_to_biguint::<F>(a.x) * &r % p;
    let a_y_r = fq_to_biguint::<F>(a.y) * &r % p;
    let a_t_r = fq_to_biguint::<F>(a.t) * &r % p;
    let a_x_r_limbs = bigint::from_biguint_le(&a_x_r, num_limbs, log_limb_size);
    let a_y_r_limbs = bigint::from_biguint_le(&a_y_r, num_limbs, log_limb_size);
    let a_t_r_limbs = bigint::from_biguint_le(&a_t_r, num_limbs, log_limb_size);
    let mut pt_a_limbs = Vec::<u32>::with_capacity(num_limbs * 3);
    pt_a_limbs.extend_from_slice(&a_x_r_limbs);
    pt_a_limbs.extend_from_slice(&a_y_r_limbs);
    pt_a_limbs.extend_from_slice(&a_t_r_limbs);
    pt_a_limbs
}

fn generate_table<P: CurveGroup, Q: PrimeField>(
    log_limb_size: u32,
    affine_to_projectivexyz: fn(&P::Affine) -> ProjectiveXYZ<Q>,
) -> Vec<u32> {
    let g = P::Affine::generator();
    let p = BigUint::from_bytes_be(&Q::MODULUS.to_bytes_be());
    let table = precompute_table::<P>(g.into(), WINDOW_SIZE);
    let mut table_limbs = vec![];
    for t in &table {
        let pt_xyz = affine_to_projectivexyz(t);
        table_limbs.extend(projectivexy_to_mont_limbs(&pt_xyz, &p, log_limb_size));
    }
    table_limbs
}

pub fn secp256k1_bases(log_limb_size: u32) -> Vec<u32> {
    generate_table::<ark_secp256k1::Projective, ark_secp256k1::Fq>(
        log_limb_size,
        secp256k1_curve::affine_to_projectivexyz,
    )
}

pub fn ed25519_bases(log_limb_size: u32) -> Vec<u32> {
    let g = EdwardsAffine::generator();
    let p = BigUint::from_bytes_be(&Fq::MODULUS.to_bytes_be());
    let table = precompute_table::<EdwardsProjective>(g.into(), WINDOW_SIZE);

    let mut table_limbs = vec![];
    for t in &table {
        let pt_xytz = affine_to_projective(t);
        table_limbs.extend(eteprojective_to_xyt_mont_limbs(&pt_xytz, &p, log_limb_size));
    }
    table_limbs
}

