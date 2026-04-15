struct ETEPoint {
    x: BigInt,
    y: BigInt,
    t: BigInt,
    z: BigInt
}

struct ETEAffinePoint {
    x: BigInt,
    y: BigInt,
}

struct ETEXYT {
    x: BigInt,
    y: BigInt,
    t: BigInt,
}

fn compress_eteaffine(
    pt: ptr<function, ETEAffinePoint>,
    log_limb_size: u32,
) -> array<u32, 8> {
    var y_limbs = (*pt).y.limbs;
    var y_bytes = limbs_le_to_bytes_be(&y_limbs, {{ log_limb_size }}u);

    var pt_x = (*pt).x;
    if (is_negative(&pt_x)) {
        y_bytes[0] ^= 0x80u;
    }

    return bytes_be_to_u32s(&y_bytes);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-twisted-extended-1.html#addition-add-2008-hwcd-3
fn ete_add_2008_hwcd_3(
    pt_0: ptr<function, ETEPoint>,
    pt_1: ptr<function, ETEPoint>,
    p: ptr<function, BigInt>
) -> ETEPoint {
    var x1 = (*pt_0).x;
    var y1 = (*pt_0).y;
    var t1 = (*pt_0).t;
    var z1 = (*pt_0).z;
    var x2 = (*pt_1).x;
    var y2 = (*pt_1).y;
    var t2 = (*pt_1).t;
    var z2 = (*pt_1).z;

    var d2r = get_d2r();

    var y1_m_x1 = ff_sub(&y1, &x1, p);
    var y2_m_x2 = ff_sub(&y2, &x2, p);
    var y1_p_x1 = ff_add(&y1, &x1, p);
    var y2_p_x2 = ff_add(&y2, &x2, p);
    var a = mont_mul(&y1_m_x1, &y2_m_x2, p);
    var b = mont_mul(&y1_p_x1, &y2_p_x2, p);
    var t1_d2r = mont_mul(&t1, &d2r, p);
    var c = mont_mul(&t1_d2r, &t2, p);
    var z1_p_z1 = ff_add(&z1, &z1, p);
    var d = mont_mul(&z1_p_z1, &z2, p);
    var e = ff_sub(&b, &a, p);
    var f = ff_sub(&d, &c, p);
    var g = ff_add(&d, &c, p);
    var h = ff_add(&b, &a, p);
    var x3 = mont_mul(&e, &f, p);
    var y3 = mont_mul(&g, &h, p);
    var t3 = mont_mul(&e, &h, p);
    var z3 = mont_mul(&f, &g, p);

    return ETEPoint(x3, y3, t3, z3);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-twisted-extended-1.html#doubling-dbl-2008-hwcd
fn ete_dbl_2008_hwcd(
    pt_0: ptr<function, ETEPoint>,
    p: ptr<function, BigInt>
) -> ETEPoint {
    var x1 = (*pt_0).x;
    var y1 = (*pt_0).y;
    var t1 = (*pt_0).t;
    var z1 = (*pt_0).z;

    var a = mont_mul(&x1, &x1, p);
    var b = mont_mul(&y1, &y1, p);
    var z1z1 = mont_mul(&z1, &z1, p);
    var c = ff_add(&z1z1, &z1z1, p);
    var d = bigint_sub(p, &a);
    var x1y1 = ff_add(&x1, &y1, p);
    var x1y12 = mont_mul(&x1y1, &x1y1, p);
    var ab = ff_add(&a, &b, p);
    var e = ff_sub(&x1y12, &ab, p);
    var g = ff_add(&d, &b, p);
    var f = ff_sub(&g, &c, p);
    var h = ff_sub(&d, &b, p);
    var x3 = mont_mul(&e, &f, p);
    var y3 = mont_mul(&g, &h, p);
    var t3 = mont_mul(&e, &h, p);
    var z3 = mont_mul(&f, &g, p);

    return ETEPoint(x3, y3, t3, z3);
}

/*
 * Scalar multiplication using double-and-add
 */
fn ete_mul(
    pt: ptr<function, ETEPoint>,
    x: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> ETEPoint {
    var zero: BigInt;
    var one: BigInt;
    one.limbs[0] = 1u;

    var result = ETEPoint(zero, one, zero, one);
    var result_is_inf = true;

    var s = *x;
    var temp = *pt;

    while (!bigint_is_zero(&s)) {
        if (!bigint_is_even(&s)) {
            if (result_is_inf) {
                result = temp;
                result_is_inf = false;
            } else {
                result = ete_add_2008_hwcd_3(&result, &temp, p);
            }
        }
        temp = ete_dbl_2008_hwcd(&temp, p);
        s = bigint_div2(&s);
    }

    return result;
}

/*
 * Determine ax + by where x and y are scalars and a and b are points.
 * x and y must not be in Montgomery form.
 */
fn ete_strauss_shamir_mul(
    a: ptr<function, ETEPoint>,
    b: ptr<function, ETEPoint>,
    x: ptr<function, BigInt>,
    y: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> ETEPoint {
    // From https://github.com/mratsim/constantine/issues/36
    var zero: BigInt;
    var one: BigInt;
    one.limbs[0] = 1u;

    var result = ETEPoint(zero, one, zero, one);
    var result_is_inf = true;

    var s0 = *x;
    var s1 = *y;

    // Compute the bit decomposition of the scalars
    var s0_bitsresult = bigint_to_bits_le(&s0);
    var s1_bitsresult = bigint_to_bits_le(&s1);

    // Precompute a + b
    var ab = ete_add_2008_hwcd_3(a, b, p);
    var point_to_add: ETEPoint;

    // Determine the length of the longest bitstring to avoid doing more loop
    // iterations than necessary
    var max_bits = max(s0_bitsresult.num_bits, s1_bitsresult.num_bits);

    for (var idx = 0u; idx < max_bits; idx ++) {
        var i = max_bits - 1u - idx;

        let a_bit = s0_bitsresult.bits[i];
        let b_bit = s1_bitsresult.bits[i];

        if (!result_is_inf) {
            result = ete_dbl_2008_hwcd(&result, p);
        }

        if (a_bit && !b_bit) {
            point_to_add = *a;
        } else if (!a_bit && b_bit) {
            point_to_add = *b;
        } else if (a_bit && b_bit) {
            point_to_add = ab;
        } else {
            continue;
        }

        if (result_is_inf) {
            // Assign instead of adding point_to_add to the point at
            // infinity, which jacobian_add_2007_bl_unsafe doesn't support
            result = point_to_add;
            result_is_inf = false;
            continue;
        }
        result = ete_add_2008_hwcd_3(&result, &point_to_add, p);
    }

    return result;
}

fn ete_to_affine_non_mont(
    a: ptr<function, ETEPoint>,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
) -> ETEAffinePoint {
    var xr = (*a).x;
    var yr = (*a).y;
    var zr = (*a).z;

    var x = ff_mul(&xr, rinv, p, p_wide, mu_fp);
    var y = ff_mul(&yr, rinv, p, p_wide, mu_fp);
    var exponent = *p;
    exponent.limbs[0] -= 2u;
    var z_inv_r = modpow(&zr, r, &exponent, p);

    var z_inv = ff_mul(&z_inv_r, rinv, p, p_wide, mu_fp);

    var affine_x = ff_mul(&x, &z_inv, p, p_wide, mu_fp);
    var affine_y = ff_mul(&y, &z_inv, p, p_wide, mu_fp);

    return ETEAffinePoint(affine_x, affine_y);
}

/*
 * Scalar multiplication using the windowed method for a fixed base
 */
fn ete_fixed_mul(
    table: ptr<function, array<ETEXYT, {{ table_size }}>>,
    s: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
    r: ptr<function, BigInt>,
) -> ETEPoint {
    // Convert s to bits
    var temp = *s;
    var scalar_bits: array<bool, 256>;

    for (var i = 0u; i < 256u; i ++) {
        if bigint_is_zero(&temp) {
            break;
        }

        scalar_bits[i] = !bigint_is_even(&temp);

        temp = bigint_div2(&temp);
    }

    var result: ETEPoint;
    var result_is_inf = true;

    var i = 256u;
    while (i > 0u) {
        var bits = 0u;
        for (var j = 0u; j < {{ log_table_size }}u; j ++){
            if (i > 0u) {
                i -= 1u;
                bits <<= 1u;
                if (scalar_bits[i]) {
                    bits |= 1u;
                }
            }
        }

        if (!result_is_inf) {
            for (var j = 0u; j < {{ log_table_size }}u; j ++){
                result = ete_dbl_2008_hwcd(&result, p);
            }
        }

        if (bits != 0u) {
            var table_pt = (*table)[bits - 1u];
            var t = ETEPoint(table_pt.x, table_pt.y, table_pt.t, *r);
            if (result_is_inf) {
                result = t;
            } else {
                result = ete_add_2008_hwcd_3(&result, &t, p);

                // mixed addition has fewer multiplications but the benchmarks
                // show a slowdown for unknown reasons 
                // result = projective_madd_1998_cmo_unsafe(&result, &t, p);
            }
            result_is_inf = false;
        }
    }

    return result;
}
