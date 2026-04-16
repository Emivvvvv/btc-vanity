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
    if (params.line0.w == 0u) {
        return ascii_lower(c);
    }
    return c;
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

fn derive_btc_address_from_point(
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    proj: ptr<function, Point>,
    debug_out: ptr<function, array<u32, 64>>,
    out_address: ptr<function, AddressBuf>,
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
    
    // Fill debug_out with HASH160(pubkey) bytes and ensure the rest is zeroed.
    for(var i: u32 = 0u; i < 64u; i = i + 1u) {
        if (i < 20u) {
            (*debug_out)[i] = h160[i];
        } else {
            (*debug_out)[i] = 0u;
        }
    }

    var payload21: array<u32, 64> = array<u32, 64>();
    payload21[0] = 0u; // Bitcoin P2PKH version byte
    for (var i: u32 = 0u; i < 20u; i = i + 1u) {
        payload21[i + 1u] = h160[i];
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

    for (var lane: u32 = 0u; lane < candidates_per_invocation; lane = lane + 1u) {
        if (lane >= lane_count || current_candidate_index >= params.line0.x) {
            break;
        }

        if (result_words[0] != RESULT_SENTINEL) {
            return;
        }

        var digest: array<u32, 64> = array<u32, 64>();
        var address: AddressBuf;
        derive_btc_address_from_point(
            &p,
            &p_wide,
            &r,
            &rinv,
            &mu_fp,
            &current_point,
            &digest,
            &address,
        );
        if (!address_matches(&address)) {
            if (lane + 1u < lane_count) {
                current_candidate_index = current_candidate_index + 1u;
                current_lo = current_lo + 1u;
                if (current_lo == 0u) {
                    current_hi = current_hi + 1u;
                }
                increment_scalar_mod_order_in_place(&scalar);
                current_point = projective_madd_1998_cmo_unsafe(&current_point, &g_point, &p);
            }
            continue;
        }

        var attempts_lo = current_lo + 1u;
        var attempts_hi = current_hi;
        if (attempts_lo == 0u) {
            attempts_hi = attempts_hi + 1u;
        }

        var scalar_words = scalar_to_result_words(&scalar);

        store_match(
            current_candidate_index,
            attempts_lo,
            attempts_hi,
            params.line1.z + 1u,
            &scalar_words,
            &address,
            &digest,
        );
        return;
    }
}
