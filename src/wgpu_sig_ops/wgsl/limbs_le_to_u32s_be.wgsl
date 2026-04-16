fn byte_from_limbs_le_le(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    idx: u32,
    log_limb_size: u32,
) -> u32 {
    let bit_pos = idx * 8u;
    let limb_index = bit_pos / log_limb_size;
    let bit_offset = bit_pos % log_limb_size;
    if (bit_offset + 8u <= log_limb_size) {
        return ((*limbs)[limb_index] >> bit_offset) & 0xffu;
    } else {
        let lb = log_limb_size - bit_offset;
        let first_part = ((*limbs)[limb_index] >> bit_offset) & ((1u << lb) - 1u);
        let remaining_bits = 8u - lb;
        let second_part = ((*limbs)[limb_index + 1u] & ((1u << remaining_bits) - 1u)) << lb;
        return first_part | second_part;
    }
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

fn limbs_le_to_bytes_le(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    log_limb_size: u32,
) -> array<u32, 32> {
    var bytes: array<u32, 32>;
    for (var i = 0u; i < 32u; i ++) {
        bytes[i] = byte_from_limbs_le_le(limbs, i, log_limb_size);
    }
    return bytes;
}

fn limbs_le_to_u32s_be(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    log_limb_size: u32,
) -> array<u32, 8> {
    var bytes: array<u32, 32>;
    for (var i = 0u; i < 32u; i ++) {
        bytes[i] = byte_from_limbs_le_be(limbs, i, log_limb_size);
    }
    return bytes_be_to_u32s(&bytes);
}

fn byte_from_limbs_le_be(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    idx: u32,
    log_limb_size: u32,
) -> u32 {
    let i = 31u - idx;
    let bit_pos = i * 8u;
    let limb_index = bit_pos / log_limb_size;
    let bit_offset = bit_pos % log_limb_size;
    if (bit_offset + 8u <= log_limb_size) {
        return ((*limbs)[limb_index] >> bit_offset) & 0xffu;
    } else {
        let lb = log_limb_size - bit_offset;
        let first_part = ((*limbs)[limb_index] >> bit_offset) & ((1u << lb) - 1u);
        let remaining_bits = 8u - lb;
        let second_part = ((*limbs)[limb_index + 1u] & ((1u << remaining_bits) - 1u)) << lb;
        return first_part | second_part;
    }
}

// Backward-compatible helper used by runtime shaders expecting BE byte extraction.
fn byte_from_limbs_le(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    idx: u32,
    log_limb_size: u32,
) -> u32 {
    return byte_from_limbs_le_be(limbs, idx, log_limb_size);
}

fn limbs_le_to_bytes_be(
    limbs: ptr<function, array<u32, {{ num_limbs }}>>,
    log_limb_size: u32,
) -> array<u32, 32> {
    var bytes: array<u32, 32>;
    for (var i = 0u; i < 32u; i ++) {
        bytes[i] = byte_from_limbs_le_be(limbs, i, log_limb_size);
    }
    return bytes;
}
