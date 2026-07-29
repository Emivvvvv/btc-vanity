use crate::wgpu_sig_ops::arithmetic::{
    biguint_to_bytes_be, biguint_to_limbs_le, bytes_34_to_limbs_32, calc_bitwidth, calc_mont_radix,
    calc_nsafe, calc_num_limbs, calc_rinv_and_n0, gen_mu,
};
use crate::wgpu_sig_ops::moduli;
use crate::wgpu_sig_ops::precompute::WINDOW_SIZE;
use ark_ec::twisted_edwards::TECurveConfig;
use ark_ec::AffineRepr;
use ark_ed25519::EdwardsAffine;
use ark_ff::{BigInteger, Field, PrimeField};
use minijinja::{context, Environment, Template};
use num_bigint::BigUint;

const WGSL_TEMPLATE_PATH: &str = "src/wgpu_sig_ops/wgsl/";
const WGSL_TESTS_PATH: &str = "src/wgpu_sig_ops/wgsl/";
#[cfg(test)]
const ALL_SHADER_FILES: [&str; 20] = [
    "base58.wgsl",
    "bigint.wgsl",
    "bytes_be_to_limbs_le.wgsl",
    "constants.wgsl",
    "ed25519_constants.wgsl",
    "ed25519_curve.wgsl",
    "ed25519_sol_vanity_search.wgsl",
    "ed25519_utils.wgsl",
    "ff.wgsl",
    "keccak256.wgsl",
    "limbs_le_to_u32s_be.wgsl",
    "mont.wgsl",
    "ripemd160.wgsl",
    "secp256k1_btc_vanity_search.wgsl",
    "secp256k1_curve.wgsl",
    "secp256k1_eth_vanity_search.wgsl",
    "secp_constants.wgsl",
    "secp_curve_utils.wgsl",
    "sha256.wgsl",
    "sha512.wgsl",
];

fn get_secp256k1_b() -> BigUint {
    BigUint::from(7u32)
}

fn get_ed25519_d2() -> BigUint {
    BigUint::parse_bytes(
        b"16295367250680780974490674513165176452449235426866156013048779062215315747161",
        10,
    )
    .unwrap()
}

fn embedded_shader_source(name: &str) -> Option<&'static str> {
    match name {
        "base58.wgsl" => Some(include_str!("wgsl/base58.wgsl")),
        "bigint.wgsl" => Some(include_str!("wgsl/bigint.wgsl")),
        "bytes_be_to_limbs_le.wgsl" => Some(include_str!("wgsl/bytes_be_to_limbs_le.wgsl")),
        "constants.wgsl" => Some(include_str!("wgsl/constants.wgsl")),
        "ed25519_constants.wgsl" => Some(include_str!("wgsl/ed25519_constants.wgsl")),
        "ed25519_curve.wgsl" => Some(include_str!("wgsl/ed25519_curve.wgsl")),
        "ed25519_sol_vanity_search.wgsl" => {
            Some(include_str!("wgsl/ed25519_sol_vanity_search.wgsl"))
        }
        "ed25519_utils.wgsl" => Some(include_str!("wgsl/ed25519_utils.wgsl")),
        "ff.wgsl" => Some(include_str!("wgsl/ff.wgsl")),
        "keccak256.wgsl" => Some(include_str!("wgsl/keccak256.wgsl")),
        "limbs_le_to_u32s_be.wgsl" => Some(include_str!("wgsl/limbs_le_to_u32s_be.wgsl")),
        "mont.wgsl" => Some(include_str!("wgsl/mont.wgsl")),
        "ripemd160.wgsl" => Some(include_str!("wgsl/ripemd160.wgsl")),
        "secp256k1_btc_vanity_search.wgsl" => {
            Some(include_str!("wgsl/secp256k1_btc_vanity_search.wgsl"))
        }
        "secp256k1_curve.wgsl" => Some(include_str!("wgsl/secp256k1_curve.wgsl")),
        "secp256k1_eth_vanity_search.wgsl" => {
            Some(include_str!("wgsl/secp256k1_eth_vanity_search.wgsl"))
        }
        "secp_constants.wgsl" => Some(include_str!("wgsl/secp_constants.wgsl")),
        "secp_curve_utils.wgsl" => Some(include_str!("wgsl/secp_curve_utils.wgsl")),
        "sha256.wgsl" => Some(include_str!("wgsl/sha256.wgsl")),
        "sha512.wgsl" => Some(include_str!("wgsl/sha512.wgsl")),
        _ => None,
    }
}

fn add_source_to_env(_template_path: &str, template_file: &str, env: &mut Environment) {
    let source = embedded_shader_source(template_file)
        .unwrap_or_else(|| panic!("unknown embedded shader template: {template_file}"));
    env.add_template_owned(template_file.to_owned(), source.to_owned())
        .unwrap();
}

fn gen_constant_bigint(
    var_name: &str,
    val: &BigUint,
    num_limbs: usize,
    log_limb_size: u32,
) -> String {
    let r_limbs = biguint_to_limbs_le(val, num_limbs, log_limb_size);
    let mut result = format!("var {var_name}: BigInt = BigInt(array<u32, {num_limbs}>(");
    for (i, limb) in r_limbs.iter().enumerate() {
        result.push_str(format!("{limb}u").as_str());
        if i < num_limbs - 1 {
            result.push_str(", ");
        }
    }
    result.push_str("));");
    result
}

fn do_render(
    p: &BigUint,
    scalar_p: &BigUint,
    b: &BigUint,
    log_limb_size: u32,
    template: &Template,
) -> String {
    let num_limbs = calc_num_limbs(log_limb_size, 256);
    let two_pow_word_size = 2u32.pow(log_limb_size);
    let mask = two_pow_word_size - 1u32;
    let nsafe = calc_nsafe(log_limb_size);
    let r = calc_mont_radix(num_limbs, log_limb_size);
    let (rinv, n0) = calc_rinv_and_n0(p, &r, log_limb_size);
    let p_bitlength = calc_bitwidth(p);
    let slack = num_limbs * log_limb_size as usize - p_bitlength;

    let r2 = (&r * &r) % p;
    let scalar_r2 = (&r * &r) % scalar_p;

    let r_bigint = gen_constant_bigint("r", &(&r % p), num_limbs, log_limb_size);
    let r2_bigint = gen_constant_bigint("r2", &r2, num_limbs, log_limb_size);
    let rinv_bigint = gen_constant_bigint("rinv", &(&rinv % p), num_limbs, log_limb_size);
    let p_bigint = gen_constant_bigint("p", p, num_limbs, log_limb_size);
    let scalar_p_bigint = gen_constant_bigint("scalar_p", scalar_p, num_limbs, log_limb_size);
    let scalar_r2_bigint = gen_constant_bigint("scalar_r2", &scalar_r2, num_limbs, log_limb_size);
    let br_bigint = gen_constant_bigint("br", &(b * &r % p), num_limbs, log_limb_size);
    let br3_bigint = gen_constant_bigint(
        "br3",
        &((BigUint::from(3u32) * b * &r) % p),
        num_limbs,
        log_limb_size,
    );
    let mu_fp_bigint = gen_constant_bigint("mu_fp", &gen_mu(p), num_limbs, log_limb_size);
    let mu_fr_bigint = gen_constant_bigint("mu_fr", &gen_mu(scalar_p), num_limbs, log_limb_size);

    let secp256k1_generator_x =
        BigUint::from_bytes_be(&ark_secp256k1::G_GENERATOR_X.into_bigint().to_bytes_be());
    let secp256k1_generator_y =
        BigUint::from_bytes_be(&ark_secp256k1::G_GENERATOR_Y.into_bigint().to_bytes_be());
    let secp256k1_generator_xr_bigint = gen_constant_bigint(
        "secp256k1_generator_xr",
        &(secp256k1_generator_x * &r % p),
        num_limbs,
        log_limb_size,
    );
    let secp256k1_generator_yr_bigint = gen_constant_bigint(
        "secp256k1_generator_yr",
        &(secp256k1_generator_y * &r % p),
        num_limbs,
        log_limb_size,
    );

    let sqrt_case3mod4_exponent = (p + BigUint::from(1u32)) / BigUint::from(4u32);
    let sqrt_case3mod4_exponent_bigint = gen_constant_bigint(
        "sqrt_case3mod4_exponent",
        &sqrt_case3mod4_exponent,
        num_limbs,
        log_limb_size,
    );

    let log_table_size = WINDOW_SIZE;
    let table_size = 2u32.pow(log_table_size);

    template
        .render(context! {
            table_size => table_size,
            log_table_size => log_table_size,
            num_limbs => num_limbs,
            log_limb_size => log_limb_size,
            two_pow_word_size => two_pow_word_size,
            mask => mask,
            nsafe => nsafe,
            n0 => n0,
            slack => slack,
            r_bigint => r_bigint,
            r2_bigint => r2_bigint,
            rinv_bigint => rinv_bigint,
            p_bigint => p_bigint,
            scalar_p_bigint => scalar_p_bigint,
            scalar_r2_bigint => scalar_r2_bigint,
            br_bigint => br_bigint,
            br3_bigint => br3_bigint,
            mu_fp_bigint => mu_fp_bigint,
            mu_fr_bigint => mu_fr_bigint,
            secp256k1_generator_xr_bigint => secp256k1_generator_xr_bigint,
            secp256k1_generator_yr_bigint => secp256k1_generator_yr_bigint,
            sqrt_case3mod4_exponent_bigint => sqrt_case3mod4_exponent_bigint,
        })
        .unwrap()
}

fn do_render_ed25519(
    p: &BigUint,
    scalar_p: &BigUint,
    d2: &BigUint,
    log_limb_size: u32,
    template: &Template,
) -> String {
    let num_limbs = calc_num_limbs(log_limb_size, 256);
    let two_pow_word_size = 2u32.pow(log_limb_size);
    let mask = two_pow_word_size - 1u32;
    let nsafe = calc_nsafe(log_limb_size);
    let r = calc_mont_radix(num_limbs, log_limb_size);
    let (rinv, n0) = calc_rinv_and_n0(p, &r, log_limb_size);
    let p_bitlength = calc_bitwidth(p);
    let slack = num_limbs * log_limb_size as usize - p_bitlength;

    let r2 = (&r * &r) % p;
    let scalar_r2 = (&r * &r) % scalar_p;

    let r_bigint = gen_constant_bigint("r", &(&r % p), num_limbs, log_limb_size);
    let r2_bigint = gen_constant_bigint("r2", &r2, num_limbs, log_limb_size);
    let rinv_bigint = gen_constant_bigint("rinv", &(&rinv % p), num_limbs, log_limb_size);
    let p_bigint = gen_constant_bigint("p", p, num_limbs, log_limb_size);
    let scalar_p_bigint = gen_constant_bigint("scalar_p", scalar_p, num_limbs, log_limb_size);
    let scalar_r2_bigint = gen_constant_bigint("scalar_r2", &scalar_r2, num_limbs, log_limb_size);
    let d2r_bigint = gen_constant_bigint("d2r", &(d2 * &r % p), num_limbs, log_limb_size);
    let mu_fp_bigint = gen_constant_bigint("mu_fp", &gen_mu(p), num_limbs, log_limb_size);
    let mu_fr_bigint = gen_constant_bigint("mu_fr", &gen_mu(scalar_p), num_limbs, log_limb_size);

    let p58_exponent = BigUint::parse_bytes(
        b"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffd",
        16,
    )
    .unwrap();
    let p58_exponent_bigint =
        gen_constant_bigint("p58_exponent", &p58_exponent, num_limbs, log_limb_size);

    let sqrt_m1 = ark_ed25519::Fq::from(-1i32).sqrt().unwrap();
    let sqrt_m1_bigint: BigUint = sqrt_m1.into_bigint().into();
    let sqrt_m1r_bigint = gen_constant_bigint(
        "sqrt_m1r",
        &(sqrt_m1_bigint * &r % p),
        num_limbs,
        log_limb_size,
    );

    let edwards_dr: BigUint = ark_ed25519::EdwardsConfig::COEFF_D.into_bigint().into();
    let edwards_dr_bigint = gen_constant_bigint(
        "edwards_dr",
        &(edwards_dr * &r % p),
        num_limbs,
        log_limb_size,
    );

    let generator = EdwardsAffine::generator();
    let ed25519_generator_x = BigUint::from_bytes_be(&generator.x.into_bigint().to_bytes_be());
    let ed25519_generator_y = BigUint::from_bytes_be(&generator.y.into_bigint().to_bytes_be());
    let ed25519_generator_xr_bigint = gen_constant_bigint(
        "ed25519_generator_xr",
        &(&ed25519_generator_x * &r % p),
        num_limbs,
        log_limb_size,
    );
    let ed25519_generator_yr_bigint = gen_constant_bigint(
        "ed25519_generator_yr",
        &(&ed25519_generator_y * &r % p),
        num_limbs,
        log_limb_size,
    );
    let ed25519_generator_tr_bigint = gen_constant_bigint(
        "ed25519_generator_tr",
        &((&ed25519_generator_x * &ed25519_generator_y) * &r % p),
        num_limbs,
        log_limb_size,
    );

    let (fr_reduce_r_limbs_array, scalar_p_limbs_array) = gen_ed25519_reduce_fr_constants(scalar_p);
    let log_table_size = WINDOW_SIZE;
    let table_size = 2u32.pow(log_table_size);

    template
        .render(context! {
            table_size => table_size,
            log_table_size => log_table_size,
            num_limbs => num_limbs,
            log_limb_size => log_limb_size,
            two_pow_word_size => two_pow_word_size,
            mask => mask,
            nsafe => nsafe,
            n0 => n0,
            slack => slack,
            r_bigint => r_bigint,
            r2_bigint => r2_bigint,
            rinv_bigint => rinv_bigint,
            p_bigint => p_bigint,
            scalar_p_bigint => scalar_p_bigint,
            scalar_r2_bigint => scalar_r2_bigint,
            d2r_bigint => d2r_bigint,
            mu_fp_bigint => mu_fp_bigint,
            mu_fr_bigint => mu_fr_bigint,
            p58_exponent_bigint => p58_exponent_bigint,
            sqrt_m1r_bigint => sqrt_m1r_bigint,
            edwards_dr_bigint => edwards_dr_bigint,
            ed25519_generator_xr_bigint => ed25519_generator_xr_bigint,
            ed25519_generator_yr_bigint => ed25519_generator_yr_bigint,
            ed25519_generator_tr_bigint => ed25519_generator_tr_bigint,
            scalar_p_limbs_array => scalar_p_limbs_array,
            fr_reduce_r_limbs_array => fr_reduce_r_limbs_array,
        })
        .unwrap()
}

fn gen_ed25519_reduce_fr_constants(scalar_p: &BigUint) -> (String, String) {
    let r = BigUint::parse_bytes(
        b"fffffffffffffffffffffffffffffffeb2106215d086329a7ed9ce5a30a2c131b",
        16,
    )
    .unwrap();
    let r_bytes = biguint_to_bytes_be(&r, 34);
    let r_limbs = bytes_34_to_limbs_32(&r_bytes);
    let mut fr_reduce_r_limbs_array = String::from("var fr_reduce_r_limbs = array<u32, 32>(");
    for (i, limb) in r_limbs.iter().enumerate() {
        fr_reduce_r_limbs_array.push_str(format!("{limb}u").as_str());
        if i < r_limbs.len() - 1 {
            fr_reduce_r_limbs_array.push_str(", ");
        }
    }
    fr_reduce_r_limbs_array.push_str(");");

    let scalar_p_bytes = biguint_to_bytes_be(scalar_p, 34);
    let scalar_p_limbs = bytes_34_to_limbs_32(&scalar_p_bytes);
    let mut scalar_p_limbs_array = String::from("var scalar_p_limbs = array<u32, 32>(");
    for (i, limb) in scalar_p_limbs.iter().enumerate() {
        scalar_p_limbs_array.push_str(format!("{limb}u").as_str());
        if i < scalar_p_limbs.len() - 1 {
            scalar_p_limbs_array.push_str(", ");
        }
    }
    scalar_p_limbs_array.push_str(");");
    (fr_reduce_r_limbs_array, scalar_p_limbs_array)
}

pub fn render_secp256k1_search_shader(template_file: &str, log_limb_size: u32) -> String {
    let mut env = Environment::new();
    let p = moduli::secp256k1_fq_modulus_biguint();
    let scalar_p = moduli::secp256k1_fr_modulus_biguint();
    let b = get_secp256k1_b();

    for file in [
        "bigint.wgsl",
        "ff.wgsl",
        "mont.wgsl",
        "secp256k1_curve.wgsl",
        "secp_constants.wgsl",
        "secp_curve_utils.wgsl",
        "constants.wgsl",
        "limbs_le_to_u32s_be.wgsl",
        "keccak256.wgsl",
        "sha256.wgsl",
        "ripemd160.wgsl",
        "base58.wgsl",
    ] {
        add_source_to_env(WGSL_TEMPLATE_PATH, file, &mut env);
    }
    add_source_to_env(WGSL_TESTS_PATH, template_file, &mut env);
    let template = env.get_template(template_file).unwrap();
    do_render(&p, &scalar_p, &b, log_limb_size, &template)
}

pub fn render_ed25519_search_shader(template_file: &str, log_limb_size: u32) -> String {
    let mut env = Environment::new();
    let p = moduli::ed25519_fq_modulus_biguint();
    let scalar_p = moduli::ed25519_fr_modulus_biguint();
    let d2 = get_ed25519_d2();

    for file in [
        "bigint.wgsl",
        "ff.wgsl",
        "mont.wgsl",
        "ed25519_curve.wgsl",
        "ed25519_utils.wgsl",
        "constants.wgsl",
        "ed25519_constants.wgsl",
        "bytes_be_to_limbs_le.wgsl",
        "limbs_le_to_u32s_be.wgsl",
        "sha512.wgsl",
        "base58.wgsl",
    ] {
        add_source_to_env(WGSL_TEMPLATE_PATH, file, &mut env);
    }
    add_source_to_env(WGSL_TESTS_PATH, template_file, &mut env);
    let template = env.get_template(template_file).unwrap();
    do_render_ed25519(&p, &scalar_p, &d2, log_limb_size, &template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shader_template_is_embedded() {
        for name in ALL_SHADER_FILES {
            let source = embedded_shader_source(name)
                .unwrap_or_else(|| panic!("missing embedded shader: {name}"));
            assert!(!source.trim().is_empty(), "empty embedded shader: {name}");
        }
    }

    #[test]
    fn final_chain_shaders_render_from_embedded_sources() {
        assert!(
            render_secp256k1_search_shader("secp256k1_btc_vanity_search.wgsl", 13)
                .contains("@compute")
        );
        assert!(
            render_secp256k1_search_shader("secp256k1_eth_vanity_search.wgsl", 13)
                .contains("@compute")
        );
        assert!(
            render_ed25519_search_shader("ed25519_sol_vanity_search.wgsl", 13).contains("@compute")
        );
    }

    #[test]
    fn shader_result_claim_is_atomic() {
        let shaders = [
            render_secp256k1_search_shader("secp256k1_btc_vanity_search.wgsl", 13),
            render_secp256k1_search_shader("secp256k1_eth_vanity_search.wgsl", 13),
            render_ed25519_search_shader("ed25519_sol_vanity_search.wgsl", 13),
        ];

        for source in shaders {
            assert!(source.contains("atomicMin"));
            assert!(!source.contains("if (result_words[0] != RESULT_SENTINEL)"));
        }
    }

    #[test]
    fn solana_shader_hashes_each_seed_once() {
        let source = render_ed25519_search_shader("ed25519_sol_vanity_search.wgsl", 13);

        assert_eq!(
            source.matches("sha512_32(").count(),
            2,
            "Solana candidate generation should reuse the SHA-512 digest"
        );
    }
}
