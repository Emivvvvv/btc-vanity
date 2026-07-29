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
@group(0) @binding(4) var<storage, read_write> result_words: array<atomic<u32>>;

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
{% include "keccak256.wgsl" %}

// Removed inlined Keccak functions.

fn ascii_lower(c: u32) -> u32 {
    if (c >= 65u && c <= 90u) {
        return c + 32u;
    }
    return c;
}

fn hex_lower(nibble: u32) -> u32 {
    if (nibble < 10u) {
        return 48u + nibble;
    }
    return 87u + nibble;
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

fn add_u64_to_bigint(base: ptr<function, BigInt>, lo: u32, hi: u32) -> BigInt {
    var out = *base;
    var tmp_lo = lo;
    var tmp_hi = hi;
    var carry = 0u;

    for (var i: u32 = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        let add_limb = tmp_lo & {{ mask }}u;
        let next_lo = (tmp_lo >> {{ log_limb_size }}u) | (tmp_hi << (32u - {{ log_limb_size }}u));
        let next_hi = tmp_hi >> {{ log_limb_size }}u;
        tmp_lo = next_lo;
        tmp_hi = next_hi;

        let sum = out.limbs[i] + add_limb + carry;
        out.limbs[i] = sum & {{ mask }}u;
        carry = sum >> {{ log_limb_size }}u;
    }

    return out;
}

fn derive_secp_scalar(lo: u32, hi: u32) -> BigInt {
    var seed = load_seed_bigint();
    var scalar = add_u64_to_bigint(&seed, lo, hi);
    var order = get_scalar_p();
    if (bigint_gte(&scalar, &order)) {
        scalar = bigint_sub(&scalar, &order);
    }
    if (bigint_is_zero(&scalar)) {
        scalar.limbs[0] = 1u;
    }
    return scalar;
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

// Removed inlined Keccak core logic.

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
    attempts_lo: u32,
    attempts_hi: u32,
    batches: u32,
    scalar_words: ptr<function, array<u32, 8>>,
    address: ptr<function, AddressBuf>,
    debug_hash: ptr<function, array<u32, 64>>,
) -> bool {
    let previous_claim = atomicMin(&result_words[RESULT_WINNER_INDEX], 0u);
    if (previous_claim != RESULT_SENTINEL) {
        return false;
    }
    atomicStore(&result_words[RESULT_ATTEMPTS_LO_INDEX], attempts_lo);
    atomicStore(&result_words[RESULT_ATTEMPTS_HI_INDEX], attempts_hi);
    atomicStore(&result_words[RESULT_BATCHES_INDEX], batches);
    atomicStore(&result_words[RESULT_ADDRESS_LEN_INDEX], (*address).len);

    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        atomicStore(&result_words[RESULT_SCALAR_INDEX + i], (*scalar_words)[i]);
    }

    // Pack 32-byte hash (first 32 elements of debug_hash) into 8 words
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let b = i * 4u;
        let word = (*debug_hash)[b] |
                   ((*debug_hash)[b + 1u] << 8u) |
                   ((*debug_hash)[b + 2u] << 16u) |
                   ((*debug_hash)[b + 3u] << 24u);
        atomicStore(&result_words[RESULT_DEBUG_HASH_INDEX + i], word);
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
        atomicStore(&result_words[RESULT_ADDRESS_INDEX + i], w);
    }
    return true;
}

fn derive_eth_address(
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    scalar: ptr<function, BigInt>,
    digest_out: ptr<function, array<u32, 64>>,
) -> AddressBuf {
    var proj = projective_fixed_mul_workgroup(scalar, p, r);
    var aff = projective_to_affine_non_mont(&proj, p, p_wide, r, rinv, mu_fp);

    var x = limbs_le_to_bytes_be(&aff.x.limbs, {{ log_limb_size }}u);
    var y = limbs_le_to_bytes_be(&aff.y.limbs, {{ log_limb_size }}u);

    var pubkey_xy: array<u32, 64> = array<u32, 64>();
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        pubkey_xy[i] = x[i];
        pubkey_xy[32u + i] = y[i];
    }

    var digest = keccak256_64(&pubkey_xy);
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        (*digest_out)[i] = digest[i];
    }

    var out: AddressBuf = AddressBuf();
    out.len = 40u;
    for (var i: u32 = 0u; i < 20u; i = i + 1u) {
        let b = digest[12u + i];
        out.data[i * 2u] = hex_lower((b >> 4u) & 0x0fu);
        out.data[i * 2u + 1u] = hex_lower(b & 0x0fu);
    }
    return out;
}

@compute
@workgroup_size(256)
fn secp256k1_eth_vanity_search(
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

    for (var lane: u32 = 0u; lane < candidates_per_invocation; lane = lane + 1u) {
        let candidate_index = base_index + lane;
        if (candidate_index >= params.line0.x) {
            break;
        }

        if (atomicLoad(&result_words[RESULT_WINNER_INDEX]) != RESULT_SENTINEL) {
            return;
        }

        var add_lo = candidate_index;
        var add_hi = 0u;
        let new_lo = attempt_base_lo + add_lo;
        if (new_lo < attempt_base_lo) {
            add_hi = 1u;
        }
        add_lo = new_lo;
        add_hi = add_hi + attempt_base_hi;

        var scalar = derive_secp_scalar(add_lo, add_hi);
        var digest: array<u32, 64> = array<u32, 64>();
        var address = derive_eth_address(
            &p,
            &p_wide,
            &r,
            &rinv,
            &mu_fp,
            &scalar,
            &digest,
        );
        if (!address_matches(&address)) {
            continue;
        }

        var attempts_lo = add_lo + 1u;
        var attempts_hi = add_hi;
        if (attempts_lo == 0u) {
            attempts_hi = attempts_hi + 1u;
        }

        var scalar_words = scalar_to_result_words(&scalar);
        store_match(
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
