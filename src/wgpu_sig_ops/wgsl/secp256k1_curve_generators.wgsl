fn get_secp256k1_generator() -> Point {
    {{ secp256k1_generator_xr_bigint }}
    {{ secp256k1_generator_yr_bigint }}
    var r = get_r();

    return Point(secp256k1_generator_xr, secp256k1_generator_yr, r);
}
