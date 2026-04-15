fn projective_to_affine_non_mont(
    a: ptr<function, Point>,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
) -> Point {
    var xr = (*a).x;
    var yr = (*a).y;
    var zr = (*a).z;

    var x = ff_mul(&xr, rinv, p, p_wide, mu_fp);
    var y = ff_mul(&yr, rinv, p, p_wide, mu_fp);

    var exponent = *p;
    exponent.limbs[0] -= 2u;
    var z_inv_r = modpow(&zr, r, &exponent, p);

    var z_inv = ff_mul(&z_inv_r, rinv, p, p_wide, mu_fp);

    //var z_inv = ff_inverse(&z, p);

    var affine_x = ff_mul(&x, &z_inv, p, p_wide, mu_fp);
    var affine_y = ff_mul(&y, &z_inv, p, p_wide, mu_fp);

    var one: BigInt;
    one.limbs[0] = 1u;
    return Point(affine_x, affine_y, one);
}
