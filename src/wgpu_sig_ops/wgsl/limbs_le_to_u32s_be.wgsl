fn byte_from_limbs_le(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    idx: u32,
    log_limb_size: u32,
) -> u32 {
    var i = 31u - idx;
    // Calculate the bit position of the i-th byte
    var bit_pos = i * 8u;

    // Determine which limb and bit within that limb the byte starts at
    var limb_index = bit_pos / log_limb_size;
    var bit_offset = bit_pos % log_limb_size;

    // Ensure the bit_offset + 8 does not exceed the boundary of the limb
    if (bit_offset + 8u <= log_limb_size) {
        // Extract the byte from within a single limb
        return ((*limbs)[limb_index] >> bit_offset) & 0xffu;
    } else {
        var lb = log_limb_size - bit_offset;
        // Extract the byte from across two limbs
        var first_part = ((*limbs)[limb_index] >> bit_offset) & ((1u << lb) - 1u);
        var remaining_bits = 8 - (log_limb_size - bit_offset);
        var second_part = ((*limbs)[limb_index + 1u] & ((1u << remaining_bits) - 1u)) << lb;
        return first_part | second_part;
    };
}

fn bytes_be_to_u32s(bytes: ptr<function, array<u32, 32>>) -> array<u32, 8> {
    var result_arr: array<u32, 8>;
    for (var i = 0u; i < 8u; i ++) {
        var r = 0u;
        r += (*bytes)[i * 4u];
        r += (*bytes)[i * 4u + 1u] << 8u;
        r += (*bytes)[i * 4u + 2u] << 16u;
        r += (*bytes)[i * 4u + 3u] << 24u;
        result_arr[i] = r;
    }
    return result_arr;
}

fn limbs_le_to_bytes_be(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    log_limb_size: u32,
) -> array<u32, 32> {
    var bytes: array<u32, 32>;
    for (var i = 0u; i < 32u; i ++) {
        bytes[i] = byte_from_limbs_le(limbs, i, log_limb_size);
    }

    return bytes;
}

fn limbs_le_to_u32s_be(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    log_limb_size: u32,
) -> array<u32, 8> {
    // Convert limbs to bytes
    var bytes = limbs_le_to_bytes_be(limbs, log_limb_size);

    // Convert bytes to u32s
    return bytes_be_to_u32s(&bytes);
}
