struct BigInt {
    limbs: array<u32, {{ num_limbs }}>
}

struct BigIntMediumWide {
    limbs: array<u32, {{ num_limbs + 1 }}>
}

struct BigIntWide {
    limbs: array<u32, {{ num_limbs * 2 }}>
}

fn bigint_eq(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
) -> bool {
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        if ((*lhs).limbs[i] != (*rhs).limbs[i]) {
            return false;
        }
    }
    return true;
}

fn bigint_wide_add(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
) -> BigIntMediumWide {
    var result: BigIntMediumWide;

    var carry: u32 = 0u;

    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        let c: u32 = (*lhs).limbs[i] + (*rhs).limbs[i] + carry;

        result.limbs[i] = c & {{ mask }}u;
        carry = c >> {{ log_limb_size }}u;
    }
    result.limbs[{{ num_limbs }}] = carry;

    return result;
}

fn bigint_add_unsafe(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
) -> BigInt {
    var result: BigInt;

    var carry: u32 = 0u;

    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        let c: u32 = (*lhs).limbs[i] + (*rhs).limbs[i] + carry;

        result.limbs[i] = c & {{ mask }}u;
        carry = c >> {{ log_limb_size }}u;
    }

    return result;
}

fn bigint_sub(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
) -> BigInt {
    var borrow: u32 = 0u;

    var result: BigInt;

    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        result.limbs[i] = (*lhs).limbs[i] - (*rhs).limbs[i] - borrow;
        if ((*lhs).limbs[i] < ((*rhs).limbs[i] + borrow)) {
            result.limbs[i] += {{ two_pow_word_size }}u;
            borrow = 1u;
        } else {
            borrow = 0u;
        }
    }
    return result;
}

fn bigint_wide_sub(
    lhs: ptr<function, BigIntMediumWide>,
    rhs: ptr<function, BigIntMediumWide>,
) -> BigIntMediumWide {
    var result: BigIntMediumWide;

    var borrow: u32 = 0u;
    for (var i: u32 = 0u; i < {{ num_limbs + 1 }}u; i ++) {
        result.limbs[i] = (*lhs).limbs[i] - (*rhs).limbs[i] - borrow;
        if ((*lhs).limbs[i] < ((*rhs).limbs[i] + borrow)) {
            result.limbs[i] += {{ two_pow_word_size }}u;
            borrow = 1u;
        } else {
            borrow = 0u;
        }
    }
    return result;
}

/*
 * Returns true if lhs >= rhs
 */
fn bigint_gte(
    lhs: ptr<function, BigInt>,
    rhs: ptr<function, BigInt>,
) -> bool {
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        let idx = {{ num_limbs }}u - 1u - i;
        let l_limb = (*lhs).limbs[idx];
        let r_limb = (*rhs).limbs[idx];

        if (l_limb > r_limb) {
            return true;
        } else if (l_limb < r_limb) {
            return false;
        }
    }
    return true;
}

/*
 * Returns true if lhs <= rhs
 */
fn bigint_wide_gte(
    lhs: ptr<function, BigIntMediumWide>,
    rhs: ptr<function, BigIntMediumWide>,
) -> bool {
    for (var i: u32 = 0u; i < {{ num_limbs + 1 }}u; i ++) {
        let idx = {{ num_limbs + 1 }}u - 1u - i;
        let l_limb = (*lhs).limbs[idx];
        let r_limb = (*rhs).limbs[idx];

        if (l_limb > r_limb) {
            return true;
        } else if (l_limb < r_limb) {
            return false;
        }
    }
    return true;
}

fn bigint_is_even(
    val: ptr<function, BigInt>
) -> bool {
    return (*val).limbs[0] % 2u == 0u;
}

fn bigint_is_zero(
    val: ptr<function, BigInt>
) -> bool {
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i ++) {
        if ((*val).limbs[i] != 0u) {
            return false;
        }
    }

    return true;
}
fn bigint_is_one(
    val: ptr<function, BigInt>
) -> bool {
    if ((*val).limbs[0] != 1u) {
        return false;
    }

    for (var i: u32  = 1u; i < {{ num_limbs }}u; i ++) {
        if ((*val).limbs[i] != 0u) {
            return false;
        }
    }

    return true;
}

fn bigint_div2(
    v: ptr<function, BigInt>
) -> BigInt {
    var result: BigInt;

    var rem = 0u;

    let m = {{ two_pow_word_size }}u;

    for (var idx: u32 = 0u; idx < {{ num_limbs }}u; idx ++) {
        var i = {{ num_limbs }}u - idx - 1u;

        var d = (*v).limbs[i] + rem * m;
        var q = d / 2u;
        rem = d % 2u;
        result.limbs[i] = q;
    }

    return result;
}

struct BitsResult {
    bits: array<bool, 256>,
    num_bits: u32
}

/*
 * Calculate the binary expansion of x, which must not be in Montgomery form.
 * Supports up to 256 bits.
 */
fn bigint_to_bits_le(
    x: ptr<function, BigInt>
) -> BitsResult {
    var bits: array<bool, 256>;
    var num_bits = 0u;

    var s = *x;

    while (!bigint_is_zero(&s)) {
        if (!bigint_is_even(&s)) {
            bits[num_bits] = true;
        }
        s = bigint_div2(&s);
        num_bits += 1u;
    }

    return BitsResult(bits, num_bits);
}

fn bigint_shr_384(
    v: ptr<function, BigIntWide>
) -> BigInt {
    var result: BigInt;

    var val = *v;
    var limbs_to_shift = 384u / {{ log_limb_size }}u;
    var bits_remaining = 384u % {{ log_limb_size }}u;
    var mask = (1u << bits_remaining) - 1u;

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        var src_index = i + limbs_to_shift;
        if (src_index < {{ num_limbs * 2 }}u) {
            var shifted = val.limbs[src_index] >> bits_remaining;
            if (bits_remaining > 0u && src_index + 1u < {{ num_limbs * 2 }}u) {
                shifted |= (val.limbs[src_index + 1u] & mask) << ({{ log_limb_size }}u - bits_remaining);
            }
            result.limbs[i] = shifted;
        }
    }
    return result;
}
