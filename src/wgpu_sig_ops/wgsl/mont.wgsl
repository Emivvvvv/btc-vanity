/*
 * An optimised variant of the Montgomery product algorithm from
 * https://github.com/mitschabaude/montgomery#13-x-30-bit-multiplication.
 */
fn mont_mul(
    x: ptr<function, BigInt>,
    y: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> BigInt {
    var s: BigInt;
    // -------------------------------------------------------------------------------------------
    {% if log_limb_size > 10 and log_limb_size < 14 %}
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        var t = s.limbs[0] + (*x).limbs[i] * (*y).limbs[0];
        var tprime = t & {{ mask }}u;
        var qi = ({{ n0 }}u * tprime) & {{ mask }}u;
        var c = (t + qi * (*p).limbs[0]) >> {{ log_limb_size }}u;
        s.limbs[0] = s.limbs[1] + (*x).limbs[i] * (*y).limbs[1] + qi * (*p).limbs[1] + c;

        // Since nSafe = 32 when num_limbs = 20, we can perform the following
        // iterations without performing a carry.
        for (var j = 2u; j < {{ num_limbs }}u; j ++) {
            s.limbs[j - 1u] = s.limbs[j] + (*x).limbs[i] * (*y).limbs[j] + qi * (*p).limbs[j];
        }
        s.limbs[{{ num_limbs }}u - 2u] = (*x).limbs[i] * (*y).limbs[{{ num_limbs }}u - 1u] + qi * (*p).limbs[{{ num_limbs }}u - 1u];
    }

    // To paraphrase mitschabaude: a last round of carries to ensure that each
    // limb is at most {{ log_limb_size }}u bits.
    var c = 0u;
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        var v = s.limbs[i] + c;
        c = v >> {{ log_limb_size }}u;
        s.limbs[i] = v & {{ mask }}u;
    }
    // -------------------------------------------------------------------------------------------
    {% elif log_limb_size > 13 and log_limb_size < 16 %}
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        var t = s.limbs[0] + (*x).limbs[i] * (*y).limbs[0];
        var tprime = t & {{ mask }}u;
        var qi = ({{ n0 }}u * tprime) & {{ mask }}u;
        var c = (t + qi * (*p).limbs[0]) >> {{ log_limb_size }}u;

        for (var j = 1u; j < {{ num_limbs }}u - 1u; j ++) {
            var t = s.limbs[j] + (*x).limbs[i] * (*y).limbs[j] + qi * (*p).limbs[j];
            if ((j - 1u) % {{ nsafe }}u == 0u) {
                t += c;
            }

            c = t >> {{ log_limb_size }}u;
            if (j % {{ nsafe }}u == 0u) {
                c = t >> {{ log_limb_size }}u;
                s.limbs[j - 1u] = t & {{ mask }}u;
            } else {
                s.limbs[j - 1u] = t;
            }
        }

        s.limbs[{{ num_limbs }}u - 2u] = (*x).limbs[i] * (*y).limbs[{{ num_limbs }}u - 1u] + qi * (*p).limbs[{{ num_limbs }}u - 1u];
    }

    var c = 0u;
    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        var v = s.limbs[i] + c;
        c = v >> {{ log_limb_size }}u;
        s.limbs[i] = v & {{ mask }}u;
    }
    {% endif %}
    return conditional_reduce(&s, p);
}

fn conditional_reduce(x: ptr<function, BigInt>, y: ptr<function, BigInt>) -> BigInt {
    if (bigint_gte(x, y)) {
        return bigint_sub(x, y);
    }

    return *x;
}

fn modpow(
    xr: ptr<function, BigInt>,
    r: ptr<function, BigInt>,
    exponent: ptr<function, BigInt>,
    p: ptr<function, BigInt>
) -> BigInt {
    var result = *r;
    var temp = *xr;
    var s = *exponent;

    while (!bigint_is_zero(&s)) {
        if (!bigint_is_even(&s)) {
            result = mont_mul(&result, &temp, p);
        }
        temp = mont_mul(&temp, &temp, p);
        s = bigint_div2(&s);
    }

    return result;
}
