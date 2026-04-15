// Requires bigint.wgsl

/*
 * Returns lhs + rhs % p
 */
fn ff_add(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
) -> BigInt {
    var p_wide: BigIntMediumWide;
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        p_wide.limbs[i] = (*p).limbs[i];
    }

    // Compute a + b
    var sum_wide = bigint_wide_add(lhs, rhs);
    
    var result_wide: BigIntMediumWide;
    if (bigint_wide_gte(&sum_wide, &p_wide)) {
        result_wide = bigint_wide_sub(&sum_wide, &p_wide);
    } else {
        result_wide = sum_wide;
    }

    var result: BigInt;

    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        result.limbs[i] = result_wide.limbs[i];
    }
    return result;
}

/*
 * Returns lhs - rhs % p
 */
fn ff_sub(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
) -> BigInt {
    if (bigint_gte(lhs, rhs)) {
        return bigint_sub(lhs, rhs);
    } else {
        var r = bigint_sub(rhs, lhs);
        return bigint_sub(p, &r);
    }
}

/*
  From "Efficient Software-Implementation of Finite Fields with Applications
  to Cryptography" by Guajardo, et al, Algorithm 16
  https://www.sandeep.de/my/papers/2006_ActaApplMath_EfficientSoftFiniteF.pdf

  Note that x must not be in Montgomery form.

  TODO: find out why this, when used, may cause the whole shader to silently fail
  TODO: find out why this fails if x == 0
*/
fn ff_inverse(
    x: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
) -> BigInt {
    var c: BigInt;
    var b: BigInt;
    b.limbs[0] = 1u;

    // u = x; v = p
    var u: BigInt;
    var v: BigInt;
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        u.limbs[i] = (*x).limbs[i];
        v.limbs[i] = (*p).limbs[i];
    }

    while (!bigint_is_one(&u) && !bigint_is_one(&v)) {
        while (bigint_is_even(&u)) {
            u = bigint_div2(&u);

            if (bigint_is_even(&b)) {
                b = bigint_div2(&b);
            } else {
                var bp = bigint_add_unsafe(&b, p);
                b = bigint_div2(&bp);
            }
        }

        while (bigint_is_even(&v)) {
            v = bigint_div2(&v);

            if (bigint_is_even(&c)) {
                c = bigint_div2(&c);
            } else {
                var cp = bigint_add_unsafe(&c, p);
                c = bigint_div2(&cp);
            }
        }

        if (bigint_gte(&u, &v)) {
            u = ff_sub(&u, &v, p);
            b = ff_sub(&b, &c, p);
        } else {
            v = ff_sub(&v, &u, p);
            c = ff_sub(&c, &b, p);
        }
    }

    var result: BigInt;
    if (bigint_is_one(&u)) {
        result = b;
    } else {
        result = c;
    }

    if bigint_gte(&result, p) {
        result = bigint_sub(&result, p);
    }

    return result;
}

fn bigint_mul(a: ptr<function, BigInt>, b: ptr<function, BigInt>) -> BigIntWide {
    var res: BigIntWide;
    for (var i = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        for (var j = 0u; j < {{ num_limbs }}u; j = j + 1u) {
            let c = (*a).limbs[i] * (*b).limbs[j];
            res.limbs[i+j] += c & {{ mask }}u;
            res.limbs[i+j+1] += c >> {{ log_limb_size }}u;
        }   
    }

    /// Start from 0 and carry the extra over to the next index.
    for (var i = 0u; i < 2 * {{ num_limbs }}u - 1; i = i + 1u) {
        res.limbs[i+1] += res.limbs[i] >> {{ log_limb_size }}u;
        res.limbs[i] = res.limbs[i] & {{ mask }}u;
    }
    return res;
}

fn sub_512(a: ptr<function, BigIntWide>, b: ptr<function, BigIntWide>, res: ptr<function, BigIntWide>) -> u32 {
    var borrow = 0u;
    for (var i = 0u; i < 2u * {{ num_limbs }}u; i = i + 1u) {
        (*res).limbs[i] = (*a).limbs[i] - (*b).limbs[i] - borrow;
        if ((*a).limbs[i] < ((*b).limbs[i] + borrow)) {
            (*res).limbs[i] += {{ two_pow_word_size }}u;
            borrow = 1u;
        } else {
            borrow = 0u;
        }
    }
    return borrow;
}

fn get_higher_with_slack(a: ptr<function, BigIntWide>) -> BigInt {
    var out: BigInt;
    for (var i = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        out.limbs[i] = (
            ((*a).limbs[i + {{ num_limbs }}u] << {{ slack }}u) +
                ((*a).limbs[i + {{ num_limbs }}u - 1] >> 
                ({{ log_limb_size }}u - {{ slack }}u))
        ) & {{ mask }}u;
    }
    return out;
}

/*
 * Returns lhs * rhs % p
 */
fn ff_mul(
    a: ptr<function, BigInt>,
    b: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    mu: ptr<function, BigInt>,
) -> BigInt {
    var xy: BigIntWide = bigint_mul(a, b);
    var xy_hi: BigInt = get_higher_with_slack(&xy);
    var l: BigIntWide = bigint_mul(&xy_hi, mu);
    var l_hi: BigInt = get_higher_with_slack(&l);
    var lp: BigIntWide = bigint_mul(&l_hi, p);
    var r_wide: BigIntWide;
    sub_512(&xy, &lp, &r_wide);

    var r_wide_reduced: BigIntWide;
    var underflow = sub_512(&r_wide, p_wide, &r_wide_reduced);
    if (underflow == 0u) {
        r_wide = r_wide_reduced;
    }
    var r: BigInt;
    for (var i = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        r.limbs[i] = r_wide.limbs[i];
    }

    if (bigint_gte(&r, p)) {
        return bigint_sub(&r, p);
    }
    return r;
}

fn ff_negate(
    a: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> BigInt {
    return ff_sub(p, a, p);
}
