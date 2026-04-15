fn u32s_to_bytes_be(
    input_u32s: ptr<function, array<u32, 16>>
) -> array<u32, 32> {
    var bytes_be: array<u32, 32>;
    for (var i = 0u; i < 8u; i++) {
        let r = (*input_u32s)[i];
        for (var j = 0u; j < 4u; j ++) {
            bytes_be[(i * 4 + j)] = (r >> (j * 8u)) & 255u;
        }
    }

    return bytes_be;
}

fn u32_be_to_le(val: u32) -> u32 {
    let a = val >> 24u;
    let b = (val >> 16u) & 0xff;
    let c = (val >> 8u) & 0xff;
    let d = val & 0xff;
    return a + (b << 8u) + (c << 16u) + (d << 24u);
}

fn bytes_be_to_limbs_le(
    bytes: ptr<function, array<u32, 32>>
) -> BigInt {
    var result: BigInt;
    var log_limb_size = {{ log_limb_size }}u;

    for (var i = 0u; i < (256u / log_limb_size); i ++) {
        var x = (log_limb_size * (i + 1u)) % 8u;
        var y_p_z = log_limb_size - x;
        var y = 0u;
        var z = 0u;

        if (y_p_z > 8u) {
            y = 8u;
            z = log_limb_size - x - y;
        } else {
            y = log_limb_size - x - y;
        }

        let bx = (log_limb_size * (i + 1u)) / 8u;

        if (bx > 31u) {
            break;
        }

        var by = bx - 1u;
        var bz: u32;
        if (z == 0u) {
            bz = by;
        } else {
            bz = by - 1u;
        }

        var x_mask = ((1u << x) - 1u);
        var y_mask = ((1u << y) - 1u);
        var z_mask = ((1u << z) - 1u);

        var byte_x: u32 = (*bytes)[31u - bx];
        var byte_y: u32 = (*bytes)[31u - by];
        var byte_z: u32 = (*bytes)[31u - bz];

        var oz: u32;
        if (z >= 8u) {
            oz = 0u;
        } else {
            oz = 8u - z;
        }

        var limb = 
            ((byte_x & x_mask) << (y + z)) + 
            (((byte_y >> (8u - y)) & y_mask) << (z)) + 
            (byte_z >> oz);

        result.limbs[i] = limb;
    }

    if (log_limb_size == 15u) {
        var limb = (*bytes)[0] >> 7u;
        result.limbs[{{ num_limbs - 2 }}] = limb;
    } else {
        var a: u32 = {{ num_limbs }}u * log_limb_size - 256u;
        var limb: u32;
        if ((log_limb_size - a) > 8) {
            var b: u32 = log_limb_size - a - 8u;
            limb = ((*bytes)[0] << b) + ((*bytes)[1] >> (8u - b));
        } else {
            limb = (*bytes)[0] >> (8u - (log_limb_size - a));
        }
        result.limbs[{{ num_limbs - 1 }}] = limb;
    }
    return result;
}
