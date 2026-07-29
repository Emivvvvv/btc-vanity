{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "secp256k1_curve.wgsl" %}
{% include "secp_constants.wgsl" %}
{% include "secp_curve_utils.wgsl" %}
{% include "constants.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read> table_limbs: array<u32>;
@group(0) @binding(1) var<storage, read> seed_limbs: array<u32>;
@group(0) @binding(2) var<storage, read_write> counter_limbs: array<u32>;
@group(0) @binding(3) var<storage, read> pattern_words: array<u32>;
@group(0) @binding(4) var<storage, read_write> result_words: array<u32>;

struct Params {
    line0: vec4<u32>,
    line1: vec4<u32>,
}

@group(0) @binding(5) var<uniform> params: Params;

struct AddressBuf {
    data: array<u32, 44>,
    len: u32,
}

const RESULT_SENTINEL: u32 = 0xffffffffu;
const RESULT_SCALAR_BASE: u32 = 5u;
const RESULT_ADDRESS_BASE: u32 = 13u;
const MODE_PREFIX: u32 = 0u;
const MODE_SUFFIX: u32 = 1u;
const MODE_ANYWHERE: u32 = 2u;
const FLAG_CASE_SENSITIVE: u32 = 0x1u;
const FLAG_STAGE_BTC: u32 = 0x2u;
const STAGED_SURVIVOR_CAPACITY: u32 = 16u;
const STAGED_SURVIVOR_SCALAR_WORDS: u32 = 128u;
const STAGED_SURVIVOR_HASH_WORDS: u32 = 320u;
const TABLE_POINT_STRIDE: u32 = {{ num_limbs * 2 }}u;
var<workgroup> SECP256K1_TABLE_WG: array<PointAffine, {{ table_size }}>;
{% include "sha256.wgsl" %}
{% include "ripemd160.wgsl" %}
{% include "base58.wgsl" %}

fn ascii_lower(c: u32) -> u32 {
    if (c >= 65u && c <= 90u) {
        return c + 32u;
    }
    return c;
}

fn normalize_for_compare(c: u32) -> u32 {
    if ((params.line0.w & FLAG_CASE_SENSITIVE) == 0u) {
        return ascii_lower(c);
    }
    return c;
}

fn stage_btc_enabled() -> bool {
    return (params.line0.w & FLAG_STAGE_BTC) != 0u;
}

fn load_seed_bigint() -> BigInt {
    var out: BigInt;
    let n = min(arrayLength(&seed_limbs), {{ num_limbs }}u);
    for (var i: u32 = 0u; i < n; i = i + 1u) {
        out.limbs[i] = seed_limbs[i] & {{ mask }}u;
    }
    return out;
}

fn add_u64_to_bigint(
    base: ptr<function, BigInt>,
    lo: u32,
    hi: u32,
    out: ptr<function, BigInt>,
) {
    (*out) = *base;
    var tmp_lo = lo;
    var tmp_hi = hi;
    var carry = 0u;

    for (var i: u32 = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        let add_limb = tmp_lo & {{ mask }}u;
        let next_lo = (tmp_lo >> {{ log_limb_size }}u) | (tmp_hi << (32u - {{ log_limb_size }}u));
        let next_hi = tmp_hi >> {{ log_limb_size }}u;
        tmp_lo = next_lo;
        tmp_hi = next_hi;

        let sum = (*out).limbs[i] + add_limb + carry;
        (*out).limbs[i] = sum & {{ mask }}u;
        carry = sum >> {{ log_limb_size }}u;
    }
}

fn derive_secp_scalar(lo: u32, hi: u32, out: ptr<function, BigInt>) {
    var seed = load_seed_bigint();
    add_u64_to_bigint(&seed, lo, hi, out);
    var order = get_scalar_p();
    if (bigint_gte(out, &order)) {
        (*out) = bigint_sub(out, &order);
    }
    if (bigint_is_zero(out)) {
        (*out).limbs[0] = 1u;
    }
}

fn increment_scalar_mod_order_in_place(scalar: ptr<function, BigInt>) {
    var carry = 1u;
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        if (carry == 0u) {
            break;
        }

        let sum = (*scalar).limbs[i] + carry;
        (*scalar).limbs[i] = sum & {{ mask }}u;
        carry = sum >> {{ log_limb_size }}u;
    }

    var order = get_scalar_p();
    if (bigint_gte(scalar, &order)) {
        (*scalar) = bigint_sub(scalar, &order);
    }
    if (bigint_is_zero(scalar)) {
        (*scalar).limbs[0] = 1u;
    }
}

fn load_table_point(index: u32) -> PointAffine {
    var pt: PointAffine;
    let base = index * TABLE_POINT_STRIDE;
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        pt.x.limbs[i] = table_limbs[base + i];
        pt.y.limbs[i] = table_limbs[base + {{ num_limbs }}u + i];
    }
    return pt;
}

fn projective_fixed_mul_workgroup(
    s: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
    r: ptr<function, BigInt>
) -> Point {
    var temp = *s;
    var scalar_bits: array<bool, 256> = array<bool, 256>();

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
            var t_affine = SECP256K1_TABLE_WG[bits - 1u];
            var t = Point(t_affine.x, t_affine.y, *r);
            if (result_is_inf) {
                result = t;
            } else {
                result = projective_add_2007_bl_unsafe(&result, &t, p);
            }
            result_is_inf = false;
        }
    }

    return result;
}

// Removed inlined Base58, SHA256, RIPEMD160 functions.

fn address_matches(address: ptr<function, AddressBuf>) -> bool {
    let pattern_len = params.line0.z;
    if (pattern_len == 0u) {
        return true;
    }
    if ((*address).len < pattern_len) {
        return false;
    }

    let mode = params.line0.y;

    if (mode == MODE_PREFIX) {
        for (var i: u32 = 0u; i < pattern_len; i = i + 1u) {
            if (normalize_for_compare((*address).data[i]) != pattern_words[i]) {
                return false;
            }
        }
        return true;
    }

    if (mode == MODE_SUFFIX) {
        let start = (*address).len - pattern_len;
        for (var i: u32 = 0u; i < pattern_len; i = i + 1u) {
            if (normalize_for_compare((*address).data[start + i]) != pattern_words[i]) {
                return false;
            }
        }
        return true;
    }

    if (mode == MODE_ANYWHERE) {
        let end = (*address).len - pattern_len;
        for (var off: u32 = 0u; off <= end; off = off + 1u) {
            var ok = true;
            for (var i: u32 = 0u; i < pattern_len; i = i + 1u) {
                if (normalize_for_compare((*address).data[off + i]) != pattern_words[i]) {
                    ok = false;
                    break;
                }
            }
            if (ok) {
                return true;
            }
        }
    }

    return false;
}

fn scalar_to_result_words(scalar: ptr<function, BigInt>) -> array<u32, 8> {
    var bytes = limbs_le_to_bytes_be(&(*scalar).limbs, {{ log_limb_size }}u);
    var out: array<u32, 8>;
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let j = i * 4u;
        out[i] = bytes[j] | (bytes[j + 1u] << 8u) | (bytes[j + 2u] << 16u) | (bytes[j + 3u] << 24u);
    }
    return out;
}

fn write_hash160_to_debug(
    h160: ptr<function, array<u32, 20>>,
    debug_out: ptr<function, array<u32, 64>>,
) {
    for (var i: u32 = 0u; i < 64u; i = i + 1u) {
        if (i < 20u) {
            (*debug_out)[i] = (*h160)[i];
        } else {
            (*debug_out)[i] = 0u;
        }
    }
}

fn prefix_leading_ones_len(pattern_len: u32) -> u32 {
    var count = 0u;
    while (count < pattern_len && pattern_words[count] == 49u) {
        count = count + 1u;
    }
    return count;
}

fn normalized_common_prefix_len(
    a: ptr<function, AddressBuf>,
    b: ptr<function, AddressBuf>,
) -> u32 {
    let n = min((*a).len, (*b).len);
    var i = 0u;
    while (i < n) {
        if (normalize_for_compare((*a).data[i]) != normalize_for_compare((*b).data[i])) {
            break;
        }
        i = i + 1u;
    }
    return i;
}

fn encode_btc_address_checksum_extreme_bounds(
    h160: ptr<function, array<u32, 20>>,
    out_min: ptr<function, AddressBuf>,
    out_max: ptr<function, AddressBuf>,
) {
    var payload_min: array<u32, 64> = array<u32, 64>();
    var payload_max: array<u32, 64> = array<u32, 64>();

    payload_min[0] = 0u;
    payload_max[0] = 0u;
    for (var i: u32 = 0u; i < 20u; i = i + 1u) {
        payload_min[i + 1u] = (*h160)[i];
        payload_max[i + 1u] = (*h160)[i];
    }

    payload_min[21u] = 0u;
    payload_min[22u] = 0u;
    payload_min[23u] = 0u;
    payload_min[24u] = 0u;

    payload_max[21u] = 255u;
    payload_max[22u] = 255u;
    payload_max[23u] = 255u;
    payload_max[24u] = 255u;

    (*out_min) = base58_encode_var(&payload_min, 25u);
    (*out_max) = base58_encode_var(&payload_max, 25u);
}

fn strong_prefix_prefilter_checksum_range(h160: ptr<function, array<u32, 20>>) -> bool {
    let pattern_len = params.line0.z;
    if (pattern_len == 0u || params.line0.y != MODE_PREFIX) {
        return true;
    }

    var addr_min: AddressBuf;
    var addr_max: AddressBuf;
    encode_btc_address_checksum_extreme_bounds(h160, &addr_min, &addr_max);

    let stable_len = normalized_common_prefix_len(&addr_min, &addr_max);
    let check_len = min(pattern_len, stable_len);
    for (var i: u32 = 0u; i < check_len; i = i + 1u) {
        if (normalize_for_compare(addr_min.data[i]) != pattern_words[i]) {
            return false;
        }
    }

    return true;
}

fn quick_prefilter_before_base58(h160: ptr<function, array<u32, 20>>) -> bool {
    let pattern_len = params.line0.z;
    let mode = params.line0.y;

    if (pattern_len == 0u) {
        return true;
    }

    if (mode != MODE_PREFIX) {
        return true;
    }

    // All Bitcoin P2PKH addresses start with '1'.
    if (pattern_words[0] != 49u) {
        return false;
    }

    // Cheap deterministic reject: additional leading '1's require leading zero HASH160 bytes.
    let leading_ones = prefix_leading_ones_len(pattern_len);
    if (leading_ones <= 1u) {
        return true;
    }

    let required_zero_bytes = leading_ones - 1u;
    for (var i: u32 = 0u; i < required_zero_bytes; i = i + 1u) {
        if ((*h160)[i] != 0u) {
            return false;
        }
    }

    // Strong zero-false-negative reject for non-leading-'1' prefixes:
    // compare against checksum-extreme Base58 bounds and validate only the
    // guaranteed stable common prefix segment.
    if (pattern_len > leading_ones) {
        return strong_prefix_prefilter_checksum_range(h160);
    }

    return true;
}

const RESULT_WINNER_INDEX: u32 = 0u;
const RESULT_ATTEMPTS_LO_INDEX: u32 = 1u;
const RESULT_ATTEMPTS_HI_INDEX: u32 = 2u;
const RESULT_BATCHES_INDEX: u32 = 3u;
const RESULT_ADDRESS_LEN_INDEX: u32 = 4u;
const RESULT_SCALAR_INDEX: u32 = 5u;
const RESULT_DEBUG_HASH_INDEX: u32 = 13u;
const RESULT_ADDRESS_INDEX: u32 = 21u;

fn store_match(
    winner_index: u32,
    attempts_lo: u32,
    attempts_hi: u32,
    batches: u32,
    scalar_words: ptr<function, array<u32, 8>>,
    address: ptr<function, AddressBuf>,
    debug_hash: ptr<function, array<u32, 64>>,
) {
    if (result_words[0] != RESULT_SENTINEL) {
        return;
    }
    result_words[RESULT_WINNER_INDEX] = winner_index;
    result_words[RESULT_ATTEMPTS_LO_INDEX] = attempts_lo;
    result_words[RESULT_ATTEMPTS_HI_INDEX] = attempts_hi;
    result_words[RESULT_BATCHES_INDEX] = batches;
    result_words[RESULT_ADDRESS_LEN_INDEX] = (*address).len;

    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        result_words[RESULT_SCALAR_INDEX + i] = (*scalar_words)[i];
    }
    
    // Pack 32-byte hash (first 32 elements of debug_hash) into 8 words
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let b = i * 4u;
        result_words[RESULT_DEBUG_HASH_INDEX + i] = (*debug_hash)[b] |
                                                  ((*debug_hash)[b + 1u] << 8u) |
                                                  ((*debug_hash)[b + 2u] << 16u) |
                                                  ((*debug_hash)[b + 3u] << 24u);
    }
    
    // Pack address characters 4-to-1
    let addr_words = ((*address).len + 3u) / 4u;
    for (var i: u32 = 0u; i < addr_words; i = i + 1u) {
        let b = i * 4u;
        var w = 0u;
        if (b < (*address).len) { w = (*address).data[b]; }
        if (b + 1u < (*address).len) { w |= ((*address).data[b + 1u] << 8u); }
        if (b + 2u < (*address).len) { w |= ((*address).data[b + 2u] << 16u); }
        if (b + 3u < (*address).len) { w |= ((*address).data[b + 3u] << 24u); }
        result_words[RESULT_ADDRESS_INDEX + i] = w;
    }
}

fn derive_btc_hash160_from_point(
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    proj: ptr<function, Point>,
    out_h160: ptr<function, array<u32, 20>>,
) {
    var aff = projective_to_affine_non_mont(proj, p, p_wide, r, rinv, mu_fp);
    
    var x_bytes = limbs_le_to_bytes_be(&aff.x.limbs, {{ log_limb_size }}u);
    var pubkey: array<u32, 64> = array<u32, 64>();
    pubkey[0] = select(3u, 2u, bigint_is_even(&aff.y));
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        pubkey[i + 1u] = x_bytes[i];
    }

    var h256 = sha256_var(&pubkey, 33u);

    var ripemd_in: array<u32, 64> = array<u32, 64>();
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        ripemd_in[i] = h256[i];
    }
    var h160 = ripemd160_var(&ripemd_in, 32u);
    for (var i: u32 = 0u; i < 20u; i = i + 1u) {
        (*out_h160)[i] = h160[i];
    }
}

fn encode_btc_address_from_hash160(
    h160: ptr<function, array<u32, 20>>,
    out_address: ptr<function, AddressBuf>,
) {

    var payload21: array<u32, 64> = array<u32, 64>();
    payload21[0] = 0u; // Bitcoin P2PKH version byte
    for (var i: u32 = 0u; i < 20u; i = i + 1u) {
        payload21[i + 1u] = (*h160)[i];
    }

    var chk1 = sha256_var(&payload21, 21u);
    var chk_in: array<u32, 64> = array<u32, 64>();
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        chk_in[i] = chk1[i];
    }
    var chk2 = sha256_var(&chk_in, 32u);

    var payload25: array<u32, 64> = array<u32, 64>();
    for (var i: u32 = 0u; i < 21u; i = i + 1u) {
        payload25[i] = payload21[i];
    }
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        payload25[21u + i] = chk2[i]; // Append first 4 bytes of double-SHA256
    }

    (*out_address) = base58_encode_var(&payload25, 25u);
}

@compute
@workgroup_size(256)
fn secp256k1_btc_vanity_search(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    if (global_id.x == 0u) {
        counter_limbs[0] = params.line1.z;
    }

    let gid = global_id.x;
    let candidates_per_invocation = max(params.line1.w, 1u);
    let base_index = gid * candidates_per_invocation;
    if (local_id.x < {{ table_size }}u) {
        SECP256K1_TABLE_WG[local_id.x] = load_table_point(local_id.x);
    }
    workgroupBarrier();

    if (base_index >= params.line0.x) {
        return;
    }

    let attempt_base_lo = params.line1.x;
    let attempt_base_hi = params.line1.y;

    var p = get_p();
    var p_wide = get_p_wide();
    var r = get_r();
    var rinv = get_rinv();
    var mu_fp = get_mu_fp();

    let lane_count = min(candidates_per_invocation, params.line0.x - base_index);
    if (lane_count == 0u) {
        return;
    }

    var current_candidate_index = base_index;
    var current_lo = attempt_base_lo + current_candidate_index;
    var current_hi = attempt_base_hi;
    if (current_lo < attempt_base_lo) {
        current_hi = current_hi + 1u;
    }

    var scalar: BigInt;
    derive_secp_scalar(current_lo, current_hi, &scalar);
    var current_point = projective_fixed_mul_workgroup(&scalar, &p, &r);

    let g_affine = SECP256K1_TABLE_WG[0];
    var g_point = Point(g_affine.x, g_affine.y, r);

    var survivor_count = 0u;
    var survivor_candidate_idx: array<u32, STAGED_SURVIVOR_CAPACITY>;
    var survivor_attempt_lo: array<u32, STAGED_SURVIVOR_CAPACITY>;
    var survivor_attempt_hi: array<u32, STAGED_SURVIVOR_CAPACITY>;
    var survivor_scalar_words: array<u32, STAGED_SURVIVOR_SCALAR_WORDS>;
    var survivor_h160: array<u32, STAGED_SURVIVOR_HASH_WORDS>;

    let staged_mode = stage_btc_enabled();

    // Stage 1: EC + HASH160 + quick reject + survivor compaction.
    for (var lane: u32 = 0u; lane < candidates_per_invocation; lane = lane + 1u) {
        if (lane >= lane_count || current_candidate_index >= params.line0.x) {
            break;
        }

        if (result_words[0] != RESULT_SENTINEL) {
            return;
        }

        var h160: array<u32, 20> = array<u32, 20>();
        derive_btc_hash160_from_point(
            &p,
            &p_wide,
            &r,
            &rinv,
            &mu_fp,
            &current_point,
            &h160,
        );

        if (!staged_mode || quick_prefilter_before_base58(&h160)) {
            if (staged_mode && survivor_count < STAGED_SURVIVOR_CAPACITY) {
                let slot = survivor_count;
                survivor_candidate_idx[slot] = current_candidate_index;
                survivor_attempt_lo[slot] = current_lo;
                survivor_attempt_hi[slot] = current_hi;

                let scalar_words = scalar_to_result_words(&scalar);
                let scalar_base = slot * 8u;
                survivor_scalar_words[scalar_base + 0u] = scalar_words[0u];
                survivor_scalar_words[scalar_base + 1u] = scalar_words[1u];
                survivor_scalar_words[scalar_base + 2u] = scalar_words[2u];
                survivor_scalar_words[scalar_base + 3u] = scalar_words[3u];
                survivor_scalar_words[scalar_base + 4u] = scalar_words[4u];
                survivor_scalar_words[scalar_base + 5u] = scalar_words[5u];
                survivor_scalar_words[scalar_base + 6u] = scalar_words[6u];
                survivor_scalar_words[scalar_base + 7u] = scalar_words[7u];

                let h160_base = slot * 20u;
                for (var i: u32 = 0u; i < 20u; i = i + 1u) {
                    survivor_h160[h160_base + i] = h160[i];
                }
                survivor_count = survivor_count + 1u;
            } else {
                // Overflow or non-staged fallback: evaluate immediately.
                var address: AddressBuf;
                encode_btc_address_from_hash160(&h160, &address);
                if (address_matches(&address)) {
                    var attempts_lo = current_lo + 1u;
                    var attempts_hi = current_hi;
                    if (attempts_lo == 0u) {
                        attempts_hi = attempts_hi + 1u;
                    }

                    var scalar_words = scalar_to_result_words(&scalar);
                    var debug_hash: array<u32, 64> = array<u32, 64>();
                    write_hash160_to_debug(&h160, &debug_hash);

                    store_match(
                        current_candidate_index,
                        attempts_lo,
                        attempts_hi,
                        params.line1.z + 1u,
                        &scalar_words,
                        &address,
                        &debug_hash,
                    );
                    return;
                }
            }
        }

        if (lane + 1u < lane_count) {
            current_candidate_index = current_candidate_index + 1u;
            current_lo = current_lo + 1u;
            if (current_lo == 0u) {
                current_hi = current_hi + 1u;
            }
            increment_scalar_mod_order_in_place(&scalar);
            current_point = projective_madd_1998_cmo_unsafe(&current_point, &g_point, &p);
        }
    }

    // Stage 2: only encode and verify compacted survivors.
    for (var s: u32 = 0u; s < survivor_count; s = s + 1u) {
        if (result_words[0] != RESULT_SENTINEL) {
            return;
        }

        var h160: array<u32, 20> = array<u32, 20>();
        let h160_base = s * 20u;
        for (var i: u32 = 0u; i < 20u; i = i + 1u) {
            h160[i] = survivor_h160[h160_base + i];
        }

        var address: AddressBuf;
        encode_btc_address_from_hash160(&h160, &address);
        if (!address_matches(&address)) {
            continue;
        }

        var attempts_lo = survivor_attempt_lo[s] + 1u;
        var attempts_hi = survivor_attempt_hi[s];
        if (attempts_lo == 0u) {
            attempts_hi = attempts_hi + 1u;
        }

        var scalar_words: array<u32, 8>;
        let scalar_base = s * 8u;
        for (var i: u32 = 0u; i < 8u; i = i + 1u) {
            scalar_words[i] = survivor_scalar_words[scalar_base + i];
        }
        var debug_hash: array<u32, 64> = array<u32, 64>();
        write_hash160_to_debug(&h160, &debug_hash);

        store_match(
            survivor_candidate_idx[s],
            attempts_lo,
            attempts_hi,
            params.line1.z + 1u,
            &scalar_words,
            &address,
            &debug_hash,
        );
        return;
    }
}
