fn get_d2r() -> BigInt {
    {{ d2r_bigint }}
    return d2r;
}

fn get_p58_exponent() -> BigInt {
    {{ p58_exponent_bigint }}
    return p58_exponent;
}

fn get_sqrt_m1r() -> BigInt {
    {{ sqrt_m1r_bigint }}
    return sqrt_m1r;
}

fn get_edwards_dr() -> BigInt {
    {{ edwards_dr_bigint }}
    return edwards_dr;
}

fn get_ed25519_generator() -> ETEPoint {
    {{ ed25519_generator_xr_bigint }}
    {{ ed25519_generator_yr_bigint }}
    {{ ed25519_generator_tr_bigint }}
    return ETEPoint(ed25519_generator_xr, ed25519_generator_yr, ed25519_generator_tr, get_r());
}
