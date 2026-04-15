struct Point {
    x: BigInt,
    y: BigInt,
    z: BigInt
}

struct PointAffine {
    x: BigInt,
    y: BigInt,
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

/// https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective.html#addition-add-2007-bl
/// Unsafe as it does not work with the point at infinity!
/// Cost: 16M
fn projective_add_2007_bl_unsafe(
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

    var u1 = mont_mul(&x1, &z2, p);
    var u2 = mont_mul(&x2, &z1, p);
    var s1 = mont_mul(&y1, &z2, p);
    var s2 = mont_mul(&y2, &z1, p);
    var zz = mont_mul(&z1, &z2, p);
    var t = ff_add(&u1, &u2, p);
    var tt = mont_mul(&t, &t, p);
    var m = ff_add(&s1, &s2, p);
    var u1u2 = mont_mul(&u1, &u2, p);
    var r = ff_sub(&tt, &u1u2, p);
    var f = mont_mul(&zz, &m, p);
    var l = mont_mul(&m, &f, p);
    var ll = mont_mul(&l, &l, p);
    var ttll = ff_add(&tt, &ll, p);
    var tl = ff_add(&t, &l, p);
    var tl2 = mont_mul(&tl, &tl, p);
    var g = ff_sub(&tl2, &ttll, p);
    var r2 = mont_mul(&r, &r, p);
    var r22 = ff_add(&r2, &r2, p);
    var w = ff_sub(&r22, &g, p);
    var f2 = ff_add(&f, &f, p);
    var x3 = mont_mul(&f2, &w, p);
    var ll2 = ff_add(&ll, &ll, p);
    var w2 = ff_add(&w, &w, p);
    var g2w = ff_sub(&g, &w2, p);
    var rg2w = mont_mul(&r, &g2w, p);
    var y3 = ff_sub(&rg2w, &ll2, p);
    var ff = mont_mul(&f, &f, p);
    var f4 = ff_add(&f2, &f2, p);
    var z3 = mont_mul(&f4, &ff, p);

    return Point(x3, y3, z3);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective.html#doubling-dbl-2007-bl
fn projective_dbl_2007_bl_unsafe(
    pt: ptr<function, Point>,
    p: ptr<function, BigInt>
) -> Point {
    var x1 = (*pt).x;
    var y1 = (*pt).y;
    var z1 = (*pt).z;

    var xx = mont_mul(&x1, &x1, p);
    var xx2 = ff_add(&xx, &xx, p);
    var w = ff_add(&xx2, &xx, p);
    var y1z1 = mont_mul(&y1, &z1, p);
    var s = ff_add(&y1z1, &y1z1, p);
    var ss = mont_mul(&s, &s, p);
    var sss = mont_mul(&s, &ss, p);
    var r = mont_mul(&y1, &s, p);
    var rr = mont_mul(&r, &r, p);
    var xxrr = ff_add(&xx, &rr, p);
    var x1r = ff_add(&x1, &r, p);
    var x1r2 = mont_mul(&x1r, &x1r, p);
    var b = ff_sub(&x1r2, &xxrr, p);
    var b2 = ff_add(&b, &b, p);
    var w2 = mont_mul(&w, &w, p);
    var h = ff_sub(&w2, &b2, p);
    var x3 = mont_mul(&h, &s, p);
    var rr2 = ff_add(&rr, &rr, p);
    var bh = ff_sub(&b, &h, p);
    var wbh = mont_mul(&w, &bh, p);
    var y3 = ff_sub(&wbh, &rr2, p);
    var z3 = sss;

    return Point(x3, y3, z3);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective.html#addition-add-2007-bl
/// Assumes that Z2 = 1, a != b, and a is not the point at infinity
fn projective_madd_1998_cmo_unsafe(
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
    }

    var y2z1 = mont_mul(&y2, &z1, p);
    var u = ff_sub(&y2z1, &y1, p);
    var uu = mont_mul(&u, &u, p);
    var x2z1 = mont_mul(&x2, &z1, p);
    var v = ff_sub(&x2z1, &x1, p);
    var vv = mont_mul(&v, &v, p);
    var vvv = mont_mul(&v, &vv, p);
    var r = mont_mul(&vv, &x1, p);
    var uuz1 = mont_mul(&uu, &z1, p);
    var r2 = ff_add(&r, &r, p);
    var vvvr2 = ff_add(&vvv, &r2, p);
    var aa = ff_sub(&uuz1, &vvvr2, p);
    var x3 = mont_mul(&v, &aa, p);

    var ra = ff_sub(&r, &aa, p);
    var ura = mont_mul(&u, &ra, p);
    var vvvy1 = mont_mul(&vvv, &y1, p);
    var y3 = ff_sub(&ura, &vvvy1, p);
    var z3 = mont_mul(&vvv, &z1, p);

    return Point(x3, y3, z3);
}

fn jacobian_negate(
    a: ptr<function, Point>,
    p: ptr<function, BigInt>
) -> Point {
    var x = (*a).x;
    var y = (*a).y;
    var z = (*a).z;

    y = ff_sub(p, &y, p);

    Point(x, y, z);
}

/// https://www.hyperelliptic.org/EFD/g1p/auto-shortw-jacobian-0.html#addition-add-2007-bl
/// ark-ec-0.4.2/src/models/short_weierstrass/group.rs
/// Unsafe as it does not work with the point at infinity!
/// Cost: 16M
fn jacobian_add_2007_bl_unsafe(
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

    var z1z1 = mont_mul(&z1, &z1, p);
    var z2z2 = mont_mul(&z2, &z2, p);
    var u1 = mont_mul(&x1, &z2z2, p);
    var u2 = mont_mul(&x2, &z1z1, p);
    var y1z2 = mont_mul(&y1, &z2, p);
    var s1 = mont_mul(&y1z2, &z2z2, p);
    var y2z1 = mont_mul(&y2, &z1, p);
    var s2 = mont_mul(&y2z1, &z1z1, p);

    if (bigint_eq(&u1, &u2) && bigint_eq(&s1, &s2)) {
        return jacobian_dbl_2009_l(a, p);
    }

    var h = ff_sub(&u2, &u1, p);
    var h2 = ff_add(&h, &h, p);
    var i = mont_mul(&h2, &h2, p);
    var j = mont_mul(&h, &i, p);
    var s2s1 = ff_sub(&s2, &s1, p);
    var r = ff_add(&s2s1, &s2s1, p);
    var v = mont_mul(&u1, &i, p);
    var v2 = ff_add(&v, &v, p);
    var r2 = mont_mul(&r, &r, p);
    var jv2 = ff_add(&j, &v2, p);
    var x3 = ff_sub(&r2, &jv2, p);
    var vx3 = ff_sub(&v, &x3, p);
    var rvx3 = mont_mul(&r, &vx3, p);
    var s12 = ff_add(&s1, &s1, p);
    var s12j = mont_mul(&s12, &j, p);
    var y3 = ff_sub(&rvx3, &s12j, p);
    var z1z2 = mont_mul(&z1, &z2, p);
    var z1z2h = mont_mul(&z1z2, &h, p);
    var z3 = ff_add(&z1z2h, &z1z2h, p);

    return Point(x3, y3, z3);
}

/// ark-ec-0.4.2/src/models/short_weierstrass/group.rs
/// http://www.hyperelliptic.org/EFD/g1p/auto-shortw-jacobian-0.html#doubling-dbl-2009-l
/// Cost: 7M
fn jacobian_dbl_2009_l(
    pt: ptr<function, Point>,
    p: ptr<function, BigInt>
) -> Point {
    var x = (*pt).x;
    var y = (*pt).y;
    var z = (*pt).z;

    var a = mont_mul(&x, &x, p);
    var b = mont_mul(&y, &y, p);
    var c = mont_mul(&b, &b, p);
    var x1b = ff_add(&x, &b, p);
    var x1b2 = mont_mul(&x1b, &x1b, p);
    var ac = ff_add(&a, &c, p);
    var x1b2ac = ff_sub(&x1b2, &ac, p);
    var d = ff_add(&x1b2ac, &x1b2ac, p);
    var a2 = ff_add(&a, &a, p);
    var e = ff_add(&a2, &a, p);
    var f = mont_mul(&e, &e, p);
    var d2 = ff_add(&d, &d, p);
    var x3 = ff_sub(&f, &d2, p);
    var c2 = ff_add(&c, &c, p);
    var c4 = ff_add(&c2, &c2, p);
    var c8 = ff_add(&c4, &c4, p);
    var dx3 = ff_sub(&d, &x3, p);
    var edx3 = mont_mul(&e, &dx3, p);
    var y3 = ff_sub(&edx3, &c8, p);
    var y1z1 = mont_mul(&y, &z, p);
    var z3 = ff_add(&y1z1, &y1z1, p);

    return Point(x3, y3, z3);
}

/*
 * Return the two possible Y-coordinates of an affine point, given its X-coordinate
 */
fn secp256k1_recover_affine_ys(
    xr: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> array<BigInt, 2> {
    // Assumes that a = 0
    var xr_squared = mont_mul(xr, xr, p);
    var xr_cubed = mont_mul(&xr_squared, xr, p);

    var br = get_br();
    var xr_cubed_plus_b = ff_add(&xr_cubed, &br, p);

    var ys = mont_sqrt_case3mod4(&xr_cubed_plus_b, p);

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

    var temp = *pt;
    var b = bigint_to_bits_le(x);
    for (var i = 0u; i < b.num_bits; i ++) {
        if (b.bits[i]) {
            result = projective_add_2007_bl_unsafe(&result, &temp, p);
        }
        temp = projective_dbl_2007_bl_unsafe(&temp, p);
    }

    return result;
}

/*
 * Scalar multiplication using double-and-add
 */
fn jacobian_mul(
    pt: ptr<function, Point>,
    x: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> Point {
    var zero: BigInt;
    var one: BigInt;
    one.limbs[0] = 1u;

    var result = Point(one, one, zero);
    var result_is_inf = true;

    var s = *x;
    var temp = *pt;

    while (!bigint_is_zero(&s)) {
        if (!bigint_is_even(&s)) {
            if (result_is_inf) {
                // This check is needed to prevent jacobian_add_2007_bl_unsafe
                // from getting the point at infinity as an input.
                result = temp;
                result_is_inf = false;
            } else {
                result = jacobian_add_2007_bl_unsafe(&result, &temp, p);
            }
        }
        temp = jacobian_dbl_2009_l(&temp, p);
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
    var result: Point;

    var s0 = *x;
    var s1 = *y;

    // Compute the bit decomposition of the scalars
    var s0_bitsresult = bigint_to_bits_le(&s0);
    var s1_bitsresult = bigint_to_bits_le(&s1);

    // Precompute a + b
    var ab = projective_add_2007_bl_unsafe(a, b, p);

    // Determine the length of the longest bitstring to avoid doing more loop
    // iterations than necessary
    var max_bits = max(s0_bitsresult.num_bits, s1_bitsresult.num_bits);

    var point_to_add: Point;
    for (var idx = 0u; idx < max_bits; idx ++) {
        var i = max_bits - 1u - idx;

        let a_bit = s0_bitsresult.bits[i];
        let b_bit = s1_bitsresult.bits[i];

        result = projective_dbl_2007_bl_unsafe(&result, p);

        if (a_bit && !b_bit) {
            point_to_add = *a;
        } else if (!a_bit && b_bit) {
            point_to_add = *b;
        } else if (a_bit && b_bit) {
            point_to_add = ab;
        } else {
            continue;
        }

        result = projective_add_2007_bl_unsafe(&result, &point_to_add, p);
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
                result = projective_dbl_2007_bl_unsafe(&result, p);
            }
        }

        if (bits != 0u) {
            var t_affine = (*table)[bits - 1u];
            var t = Point(t_affine.x, t_affine.y, *r);
            if (result_is_inf) {
                result = t;
            } else {
                result = projective_add_2007_bl_unsafe(&result, &t, p);

                // mixed addition has fewer multiplications but the benchmarks
                // show a slowdown for unknown reasons 
                // result = projective_madd_1998_cmo_unsafe(&result, &t, p);
            }
            result_is_inf = false;
        }
    }

    return result;
}

/*
 * Scalar multiplication using the GLV method
fn projective_glv_mul(
    pt: ptr<function, Point>,
    x: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> Point {
    var result: Point;

    // Split k into k1 and k2
    // Map pt to pt_prime
    // Normalise k1 and k2 to roughly half the bitlength of the scalar field
    // Use the Strauss-Shamir method

    return result;
}
 */
