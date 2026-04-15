fn get_p() -> BigInt {
    {{ p_bigint }}
    return p;
}

fn get_p_wide() -> BigIntWide {
    {{ p_bigint }}
    var p_wide: BigIntWide;
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        p_wide.limbs[i] = p.limbs[i];
    }
    return p_wide;
}

fn get_scalar_p() -> BigInt {
    {{ scalar_p_bigint }}
    return scalar_p;
}

fn get_scalar_p_wide() -> BigIntWide {
    {{ scalar_p_bigint }}
    var scalar_p_wide: BigIntWide;
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        scalar_p_wide.limbs[i] = scalar_p.limbs[i];
    }
    return scalar_p_wide;
}

fn get_r() -> BigInt {
    {{ r_bigint }}
    return r;
}

fn get_rinv() -> BigInt {
    {{ rinv_bigint }}
    return rinv;
}

fn get_mu_fp() -> BigInt {
    {{ mu_fp_bigint }}
    return mu_fp;
}

fn get_mu_fr() -> BigInt {
    {{ mu_fr_bigint }}
    return mu_fr;
}
