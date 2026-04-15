struct Point {
    x: BigInt,
    y: BigInt,
    z: BigInt
}

struct PointAffine {
    x: BigInt,
    y: BigInt,
}

fn mont_mul_neg_3(
    val: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> BigInt {
    var val2 = ff_add(val, val, p);
    var val3 = ff_add(&val2, val, p);
    var neg_val_3 = ff_negate(&val3, p);
    return neg_val_3;
}

fn mont_sqrt_case3mod4(
    xr: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> array<BigInt, 2> {
    var exponent = get_sqrt_case3mod4_exponent();
    var r = get_r();
    var a = modpow(xr, &r, &exponent, p);
    var b = ff_sub(p, &a, p);
    return array(a, b);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective-3.html#addition-add-2015-rcb
fn projective_add_2015_rcb_unsafe(
    a: ptr<function, Point>,
    b: ptr<function, Point>,
    p: ptr<function, BigInt>
) -> Point {
    var x1 = (*a).x;
    var y1 = (*a).y;
    var z1 = (*a).z;
    var x2 = (*b).x;
    var y2 = (*b).y;
    var z2 = (*b).z;

    if (bigint_is_zero(&x1) && bigint_is_zero(&z1)) {
        return *b;
    } else if (bigint_is_zero(&x2) && bigint_is_zero(&z2)) {
        return *a;
    }

    var b3 = get_br3();

    var t0 = mont_mul(&x1, &x2, p);
    var t1 = mont_mul(&y1, &y2, p);
    var t2 = mont_mul(&z1, &z2, p);
    var t3 = ff_add(&x1, &y1, p);
    var t4 = ff_add(&x2, &y2, p);
    t3 = mont_mul(&t3, &t4, p);
    t4 = ff_add(&t0, &t1, p);
    t3 = ff_sub(&t3, &t4, p);
    t4 = ff_add(&x1, &z1, p);
    var t5 = ff_add(&x2, &z2, p);
    t4 = mont_mul(&t4, &t5, p);
    t5 = ff_add(&t0, &t2, p);
    t4 = ff_sub(&t4, &t5, p);
    t5 = ff_add(&y1, &z1, p);
    var x3 = ff_add(&y2, &z2, p);
    t5 = mont_mul(&t5, &x3, p);
    x3 = ff_add(&t1, &t2, p);
    t5 = ff_sub(&t5, &x3, p);
    var z3 = mont_mul_neg_3(&t4, p);
    x3 = mont_mul(&b3, &t2, p);
    z3 = ff_add(&x3, &z3, p);
    x3 = ff_sub(&t1, &z3, p);
    z3 = ff_add(&t1, &z3, p);
    var y3 = mont_mul(&x3, &z3, p);
    t1 = ff_add(&t0, &t0, p);
    t1 = ff_add(&t1, &t0, p);
    t2 = mont_mul_neg_3(&t2, p);
    t4 = mont_mul(&b3, &t4, p);
    t1 = ff_add(&t1, &t2, p);
    t2 = ff_sub(&t0, &t2, p);
    t2 = mont_mul_neg_3(&t2, p);
    t4 = ff_add(&t4, &t2, p);
    t0 = mont_mul(&t1, &t4, p);
    y3 = ff_add(&y3, &t0, p);
    t0 = mont_mul(&t5, &t4, p);
    x3 = mont_mul(&t3, &x3, p);
    x3 = ff_sub(&x3, &t0, p);
    t0 = mont_mul(&t3, &t1, p);
    z3 = mont_mul(&t5, &z3, p);
    z3 = ff_add(&z3, &t0, p);

    return Point(x3, y3, z3);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective-3.html#doubling-dbl-2015-rcb
fn projective_dbl_2015_rcb(
    a: ptr<function, Point>,
    p: ptr<function, BigInt>
) -> Point {
    var x1 = (*a).x;
    var y1 = (*a).y;
    var z1 = (*a).z;

    if (bigint_is_zero(&x1) && bigint_is_zero(&z1)) {
        return *a;
    }

    var b3 = get_br3();

    var t0 = mont_mul(&x1, &x1, p);
    var t1 = mont_mul(&y1, &y1, p);
    var t2 = mont_mul(&z1, &z1, p);
    var t3 = mont_mul(&x1, &y1, p);
    t3 = ff_add(&t3, &t3, p);
    var z3 = mont_mul(&x1, &z1, p);
    z3 = ff_add(&z3, &z3, p);
    var x3 = mont_mul_neg_3(&z3, p);
    var y3 = mont_mul(&b3, &t2, p);
    y3 = ff_add(&x3, &y3, p);
    x3 = ff_sub(&t1, &y3, p);
    y3 = ff_add(&t1, &y3, p);
    y3 = mont_mul(&x3, &y3, p);
    x3 = mont_mul(&t3, &x3, p);
    z3 = mont_mul(&b3, &z3, p);
    t2 = mont_mul_neg_3(&t2, p);
    t3 = ff_sub(&t0, &t2, p);
    t3 = mont_mul_neg_3(&t3, p);
    t3 = ff_add(&t3, &z3, p);
    z3 = ff_add(&t0, &t0, p);
    t0 = ff_add(&z3, &t0, p);
    t0 = ff_add(&t0, &t2, p);
    t0 = mont_mul(&t0, &t3, p);
    y3 = ff_add(&y3, &t0, p);
    t2 = mont_mul(&y1, &z1, p);
    t2 = ff_add(&t2, &t2, p);
    t0 = mont_mul(&t2, &t3, p);
    x3 = ff_sub(&x3, &t0, p);
    z3 = mont_mul(&t2, &t1, p);
    z3 = ff_add(&z3, &z3, p);
    z3 = ff_add(&z3, &z3, p);

    return Point(x3, y3, z3);
}

/*
 * Return the two possible Y-coordinates of an affine point, given its X-coordinate
 * a = 3
 */
fn secp256r1_recover_affine_ys(
    xr: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> array<BigInt, 2> {
    var xr_squared = mont_mul(xr, xr, p);
    var xr_cubed = mont_mul(&xr_squared, xr, p);

    var xr2 = ff_add(xr, xr, p);
    var xr3 = ff_add(&xr2, xr, p);
    var axr = ff_negate(&xr3, p);
    var xr_cubed_plus_ar = ff_add(&xr_cubed, &axr, p);

    var br = get_br();
    var xr_cubed_plus_ar_plus_br = ff_add(&xr_cubed_plus_ar, &br, p);

    var ys = mont_sqrt_case3mod4(&xr_cubed_plus_ar_plus_br, p);

    return ys;
}

/*
 * Scalar multiplication using double-and-add
 */
fn projective_mul(
    pt: ptr<function, Point>,
    x: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> Point {
    var result: Point;
    var result_is_inf = true;

    var s = *x;
    var temp = *pt;

    while (!bigint_is_zero(&s)) {
        if (!bigint_is_even(&s)) {
            if (result_is_inf) {
                result = temp;
                result_is_inf = false;
            } else {
                result = projective_add_2015_rcb_unsafe(&result, &temp, p);
            }
        }
        temp = projective_dbl_2015_rcb(&temp, p);
        s = bigint_div2(&s);
    }

    return result;
}

/*
 * Determine ax + by where x and y are scalars and a and b are points.
 * x and y must not be in Montgomery form.
 */
fn projective_strauss_shamir_mul(
    a: ptr<function, Point>,
    b: ptr<function, Point>,
    x: ptr<function, BigInt>,
    y: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> Point {
    // From https://github.com/mratsim/constantine/issues/36
    var zero: BigInt;
    var one: BigInt;
    one.limbs[0] = 1u;

    var result = Point(zero, one, zero);
    var result_is_inf = true;

    var s0 = *x;
    var s1 = *y;

    // Compute the bit decomposition of the scalars
    var s0_bitsresult = bigint_to_bits_le(&s0);
    var s1_bitsresult = bigint_to_bits_le(&s1);

    // Precompute a + b
    var ab = projective_add_2015_rcb_unsafe(a, b, p);
    var point_to_add: Point;

    // Determine the length of the longest bitstring to avoid doing more loop
    // iterations than necessary
    var max_bits = max(s0_bitsresult.num_bits, s1_bitsresult.num_bits);

    for (var idx = 0u; idx < max_bits; idx ++) {
        var i = max_bits - 1u - idx;

        let a_bit = s0_bitsresult.bits[i];
        let b_bit = s1_bitsresult.bits[i];

        if (!result_is_inf) {
            result = projective_dbl_2015_rcb(&result, p);
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
        result = projective_add_2015_rcb_unsafe(&result, &point_to_add, p);
    }

    return result;
}

/*
 * Scalar multiplication using the windowed method for a fixed base
 */
fn projective_fixed_mul(
    table: ptr<function, array<PointAffine, {{ table_size }}>>,
    s: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
    r: ptr<function, BigInt>
) -> Point {
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

    var result: Point;
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
                result = projective_dbl_2015_rcb(&result, p);
            }
        }

        if (bits != 0u) {
            var t_affine = (*table)[bits - 1u];
            var t = Point(t_affine.x, t_affine.y, *r);
            if (result_is_inf) {
                result = t;
            } else {
                result = projective_add_2015_rcb_unsafe(&result, &t, p);
            }
            result_is_inf = false;
        }
    }

    return result;
}
