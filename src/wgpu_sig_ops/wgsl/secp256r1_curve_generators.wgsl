fn get_secp256r1_generator() -> Point {
    {{ secp256r1_generator_xr_bigint }}
    {{ secp256r1_generator_yr_bigint }}
    var r = get_r();

    return Point(secp256r1_generator_xr, secp256r1_generator_yr, r);
}
