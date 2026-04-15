struct IntermediateResult {
    u1: BigInt,
    u2: BigInt,
    recovered_r: Point
}

fn secp256r1_ecrecover_0(
    sig_r_bytes: ptr<function, array<u32, 32>>,
    sig_s_bytes: ptr<function, array<u32, 32>>,
    msg_bytes: ptr<function, array<u32, 32>>,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    scalar_p: ptr<function, BigInt>,
    scalar_p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    mu_fr: ptr<function, BigInt>,
) -> IntermediateResult {
    var decoded = decode_signature(sig_s_bytes);
    var ds = decoded.sig;
    var is_y_odd = decoded.is_y_odd;

    var sig_r = bytes_be_to_limbs_le(sig_r_bytes);
    var sig_s = bytes_be_to_limbs_le(&ds);
   
    var z = bytes_be_to_limbs_le(msg_bytes);

    if (bigint_gte(&z, scalar_p)) {
        z = bigint_sub(scalar_p, &z);
    }

    // TODO: check this
    if (bigint_is_zero(&sig_r)) {
        var z: BigInt;
        return IntermediateResult(z, z, Point(z, z, z));
    }

    var r_x = sig_r;

    var r_xr = ff_mul(&r_x, r, p, p_wide, mu_fp);
    var yrs = secp256r1_recover_affine_ys(&r_xr, p);
    var yr0 = yrs[0];
    var yr1 = yrs[1];

    var y0 = ff_mul(&yr0, rinv, p, p_wide, mu_fp);
    /*var y1 = ff_mul(&yr1, rinv, p, p_wide, mu_fp);*/

    // TODO: could checking if yr_0 odd be optimised?
    var y0_is_odd = !bigint_is_even(&y0);

    var yr: BigInt;

    if (is_y_odd) {
        if (y0_is_odd) {
            yr = yr0;
        } else {
            yr = yr1;
        }
    } else { // y is even
        if (y0_is_odd) {
            yr = yr1;
        } else {
            yr = yr0;
        }
    }

    var recovered_r = Point(r_xr, yr, *r);

    if (bigint_gte(&r_x, scalar_p)) {
        r_x = bigint_sub(&r_x, scalar_p);
    }

    // compute inverse(r_x) in the scalar field
    var r_x_inv = ff_inverse(&r_x, scalar_p);

    // compute u1 = -(r_inv * z);
    var r_x_inv_z = ff_mul(&r_x_inv, &z, scalar_p, scalar_p_wide, mu_fr);
    var u1 = ff_negate(&r_x_inv_z, scalar_p);

    // compute u2 = r_inv * s;
    var u2 = ff_mul(&r_x_inv, &sig_s, scalar_p, scalar_p_wide, mu_fr);

    return IntermediateResult(u1, u2, recovered_r);
}

fn secp256r1_ecrecover(
    sig_r_bytes: ptr<function, array<u32, 32>>,
    sig_s_bytes: ptr<function, array<u32, 32>>,
    msg_bytes: ptr<function, array<u32, 32>>,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    scalar_p: ptr<function, BigInt>,
    scalar_p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    mu_fr: ptr<function, BigInt>,
) -> Point {
    var ir = secp256r1_ecrecover_0(
        sig_r_bytes,
        sig_s_bytes,
        msg_bytes,
        p,
        p_wide,
        scalar_p,
        scalar_p_wide,
        r,
        rinv,
        mu_fp,
        mu_fr,
    );

    var u1 = ir.u1;
    var u2 = ir.u2;
    var recovered_r = ir.recovered_r;

    var g = get_secp256r1_generator();
    var u1g = projective_mul(&g, &u1, p);
    var u2r = projective_mul(&recovered_r, &u2, p);
    var result_proj = projective_add_2015_rcb_unsafe(&u1g, &u2r, p);

    // Return the point in affine form
    return projective_to_affine_non_mont(&result_proj, p, p_wide, r, rinv, mu_fp);

    // At a high enough thread count, using the projective_strauss_shamir_mul()
    // function will make the shader silently fail and the result buffer will
    // be 0s
    /*return projective_strauss_shamir_mul(&g, &recovered_r, &u1, &u2, p);*/
}
