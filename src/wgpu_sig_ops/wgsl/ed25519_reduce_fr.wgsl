fn mul(
    a: ptr<function, array<u32, 32>>,
    b: ptr<function, array<u32, 32>>
) -> array<u32, 64> {
    var num_words = 32u;
    var log_limb_size = 16u;

    var w_mask = (1u << log_limb_size) - 1u;

    var res: array<u32, 64>;
    for (var i = 0u; i < num_words; i ++) {
        for (var j = 0u; j < num_words; j ++) {
            var c = (*a)[i] * (*b)[j];
            res[i + j] += c & w_mask;
            res[i + j + 1u] += c >> log_limb_size;
        }
    }

    for (var i = 0u; i < num_words * 2u - 1u; i ++) {
        res[i + 1] += res[i] >> log_limb_size;
        res[i] = res[i] & w_mask;
    }

    return res;
}

fn sub(
    lhs: ptr<function, array<u32, 32>>,
    rhs: ptr<function, array<u32, 32>>,
) -> array<u32, 32> {
    var num_limbs = 32u;
    var log_limb_size = 16u;
    var borrow: u32 = 0u;

    var result: array<u32, 32>;

    for (var i: u32 = 0u; i < num_limbs; i ++) {
        result[i] = (*lhs)[i] - (*rhs)[i] - borrow;
        if ((*lhs)[i] < ((*rhs)[i] + borrow)) {
            result[i] += 65536u;
            borrow = 1u;
        } else {
            borrow = 0u;
        }
    }
    return result;
}

fn gte(
    lhs: ptr<function, array<u32, 32>>,
    rhs: ptr<function, array<u32, 32>>,
) -> bool {
    for (var i: u32 = 0u; i < 32u; i ++) {
        let idx = 31u - i;
        let l_limb = (*lhs)[idx];
        let r_limb = (*rhs)[idx];

        if (l_limb > r_limb) {
            return true;
        } else if (l_limb < r_limb) {
            return false;
        }
    }
    return true;
}

fn shr_512(
    a: ptr<function, array<u32, 64>>
) -> array<u32, 32> {
    var limbs: array<u32, 32>;
    for (var i = 32u; i < 64u; i ++) {
        limbs[i - 32u] = (*a)[i];
    }
    return limbs;
}

fn convert_512_be_to_le(
    x: ptr<function, array<u32, 16>>
) -> array<u32, 32> {
    var x_limbs: array<u32, 32>;
    for (var i = 0u; i < 16u; i ++) {
        x_limbs[i * 2 + 1] = (*x)[15 - i] >> 16u;
        x_limbs[i * 2 + 0] = (*x)[15 - i] & 0xffff;
    }
    return x_limbs;
}

fn ed25519_reduce_fr(
    x: ptr<function, array<u32, 16>>
) -> array<u32, 32> {
    var result: array<u32, 32>;

    {{ fr_reduce_r_limbs_array }}
    {{ scalar_p_limbs_array }}

    // Convert x (16 * 32-bit values in big-endian) to 32 * 16-bit values in little-endian
    var x_limbs: array<u32, 32> = convert_512_be_to_le(x);

    var xr_limbs = mul(&x_limbs, &fr_reduce_r_limbs);
    var xr_shr_512_limbs = shr_512(&xr_limbs);

    var xr_shr_512_p_limbs = mul(&xr_shr_512_limbs, &scalar_p_limbs);

    var rhs_limbs: array<u32, 32>;
    for (var i = 0u; i < 32u; i ++) {
        rhs_limbs[i] = xr_shr_512_p_limbs[i];
    }

    var t_limbs: array<u32, 32> = sub(&x_limbs, &rhs_limbs);

    while (gte(&t_limbs, &scalar_p_limbs)) {
        t_limbs = sub(&t_limbs, &scalar_p_limbs);
    }

    // t_limbs = [8.8, 8.8, 8.8...]
    var t_bytes: array<u32, 32>;
    for (var i = 0u; i < 16; i ++) {
        var hi = t_limbs[i] >> 8u;
        var lo = t_limbs[i] & 0xff;
        t_bytes[i * 2    ] = lo;
        t_bytes[i * 2 + 1] = hi;
    }

    return t_bytes;
}
