{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_utils.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "constants.wgsl" %}
{% include "sha512.wgsl" %}
{% include "bytes_be_to_limbs_le.wgsl" %}
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

var<workgroup> ED25519_TABLE_WG: array<ETEXYT, {{ table_size }}>;

const RESULT_SENTINEL: u32 = 0xffffffffu;
const RESULT_SCALAR_BASE: u32 = 5u;
const RESULT_ADDRESS_BASE: u32 = 13u;
const MODE_PREFIX: u32 = 0u;
const MODE_SUFFIX: u32 = 1u;
const MODE_ANYWHERE: u32 = 2u;
const TABLE_POINT_STRIDE: u32 = {{ num_limbs * 3 }}u;
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

fn derive_seed_scalar(lo: u32, hi: u32) -> BigInt {
    var seed = load_seed_bigint();
    var scalar = add_u64_to_bigint(&seed, lo, hi);
    if (bigint_is_zero(&scalar)) {
        scalar.limbs[0] = 1u;
    }
    return scalar;
}

fn seed_scalar_to_result_words(seed_scalar: ptr<function, BigInt>) -> array<u32, 8> {
    var bytes = limbs_le_to_bytes_be(&(*seed_scalar).limbs, {{ log_limb_size }}u);
    var out: array<u32, 8>;
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let j = i * 4u;
        out[i] = bytes[j] | (bytes[j + 1u] << 8u) | (bytes[j + 2u] << 16u) | (bytes[j + 3u] << 24u);
    }
    return out;
}

fn seed_scalar_to_seed_bytes(seed_scalar: ptr<function, BigInt>) -> array<u32, 32> {
    return limbs_le_to_bytes_be(&(*seed_scalar).limbs, {{ log_limb_size }}u);
}

fn sha512_32(input_bytes: ptr<function, array<u32, 32>>) -> array<u32, 64> {
    var block: array<u32, 128> = array<u32, 128>();
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        block[i] = (*input_bytes)[i] & 0xffu;
    }
    block[32] = 0x80u;
    block[126] = 0x01u;

    var w: array<vec2<u32>, 80> = array<vec2<u32>, 80>();
    for (var i: u32 = 0u; i < 16u; i = i + 1u) {
        let b = i * 8u;
        let hi = (block[b] << 24u) | (block[b + 1u] << 16u) | (block[b + 2u] << 8u) | block[b + 3u];
        let lo = (block[b + 4u] << 24u) | (block[b + 5u] << 16u) | (block[b + 6u] << 8u) | block[b + 7u];
        w[i] = vec2<u32>(hi, lo);
    }

    for (var i: u32 = 16u; i < 80u; i = i + 1u) {
        let s0 = xor(xor(right_rotate(w[i - 15u], 1u), right_rotate(w[i - 15u], 8u)), shr(w[i - 15u], 7u));
        let s1 = xor(xor(right_rotate(w[i - 2u], 19u), right_rotate(w[i - 2u], 61u)), shr(w[i - 2u], 6u));
        w[i] = add(add(add(w[i - 16u], s0), w[i - 7u]), s1);
    }

    var rc = round_constants();
    var h = initial_hash();

    var a = h[0];
    var b = h[1];
    var c = h[2];
    var d = h[3];
    var e = h[4];
    var f = h[5];
    var g = h[6];
    var hh = h[7];

    for (var i: u32 = 0u; i < 80u; i = i + 1u) {
        let sum1 = xor(xor(right_rotate(e, 14u), right_rotate(e, 18u)), right_rotate(e, 41u));
        let ch = xor(and(e, f), and(not(e), g));
        let temp1 = add(add(add(add(hh, sum1), ch), rc[i]), w[i]);
        let sum0 = xor(xor(right_rotate(a, 28u), right_rotate(a, 34u)), right_rotate(a, 39u));
        let maj = xor(xor(and(a, b), and(a, c)), and(b, c));
        let temp2 = add(sum0, maj);

        hh = g;
        g = f;
        f = e;
        e = add(d, temp1);
        d = c;
        c = b;
        b = a;
        a = add(temp1, temp2);
    }

    h[0] = add(h[0], a);
    h[1] = add(h[1], b);
    h[2] = add(h[2], c);
    h[3] = add(h[3], d);
    h[4] = add(h[4], e);
    h[5] = add(h[5], f);
    h[6] = add(h[6], g);
    h[7] = add(h[7], hh);

    var out: array<u32, 64>;
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let hv = h[i];
        let j = i * 8u;
        out[j] = (hv.x >> 24u) & 0xffu;
        out[j + 1u] = (hv.x >> 16u) & 0xffu;
        out[j + 2u] = (hv.x >> 8u) & 0xffu;
        out[j + 3u] = hv.x & 0xffu;
        out[j + 4u] = (hv.y >> 24u) & 0xffu;
        out[j + 5u] = (hv.y >> 16u) & 0xffu;
        out[j + 6u] = (hv.y >> 8u) & 0xffu;
        out[j + 7u] = hv.y & 0xffu;
    }

    return out;
}

fn derive_ed25519_scalar_from_seed(seed_bytes: ptr<function, array<u32, 32>>) -> BigInt {
    var digest = sha512_32(seed_bytes);
    digest[0] = digest[0] & 0xf8u;
    digest[31] = (digest[31] & 0x3fu) | 0x40u;

    var scalar_be: array<u32, 32>;
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        scalar_be[i] = digest[31u - i];
    }

    return bytes_be_to_limbs_le(&scalar_be);
}

fn load_table_point(index: u32) -> ETEXYT {
    var pt: ETEXYT;
    let base = index * TABLE_POINT_STRIDE;
    for (var i: u32 = 0u; i < {{ num_limbs }}u; i = i + 1u) {
        pt.x.limbs[i] = table_limbs[base + i];
        pt.y.limbs[i] = table_limbs[base + {{ num_limbs }}u + i];
        pt.t.limbs[i] = table_limbs[base + (2u * {{ num_limbs }}u) + i];
    }
    return pt;
}

// Using base58_encode_var from base58.wgsl

fn base58_encode_32(input: ptr<function, array<u32, 32>>, input_len: u32) -> AddressBuf {
    var out: AddressBuf;
    var alphabet = B58_ALPHABET;
    var digits: array<u32, 64> = array<u32, 64>();

    var zero_count = 0u;
    while (zero_count < input_len && (*input)[zero_count] == 0u) {
        zero_count = zero_count + 1u;
    }

    var digits_len = 0u;
    for (var i: u32 = zero_count; i < input_len; i = i + 1u) {
        var carry = (*input)[i] & 0xffu;
        for (var j: u32 = 0u; j < digits_len; j = j + 1u) {
            let v = digits[j] * 256u + carry;
            digits[j] = v % 58u;
            carry = v / 58u;
        }
        while (carry > 0u) {
            digits[digits_len] = carry % 58u;
            digits_len = digits_len + 1u;
            carry = carry / 58u;
        }
    }

    var out_len = 0u;
    for (var i: u32 = 0u; i < zero_count; i = i + 1u) {
        out.data[out_len] = 49u;
        out_len = out_len + 1u;
    }
    for (var i: u32 = 0u; i < digits_len; i = i + 1u) {
        let idx = digits_len - 1u - i;
        out.data[out_len] = alphabet[digits[idx]];
        out_len = out_len + 1u;
    }

    out.len = out_len;
    return out;
}

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

fn ete_fixed_mul_workgroup(
    s: ptr<function, BigInt>,
    p: ptr<function, BigInt>,
    r: ptr<function, BigInt>,
) -> ETEPoint {
    var temp = *s;
    var scalar_bits: array<bool, 256> = array<bool, 256>();

    for (var i = 0u; i < 256u; i ++) {
        if bigint_is_zero(&temp) {
            break;
        }
        scalar_bits[i] = !bigint_is_even(&temp);
        temp = bigint_div2(&temp);
    }

    var result: ETEPoint;
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
                result = ete_dbl_2008_hwcd(&result, p);
            }
        }

        if (bits != 0u) {
            var table_pt = ED25519_TABLE_WG[bits - 1u];
            var t = ETEPoint(table_pt.x, table_pt.y, table_pt.t, *r);
            if (result_is_inf) {
                result = t;
            } else {
                result = ete_add_2008_hwcd_3(&result, &t, p);
            }
            result_is_inf = false;
        }
    }

    return result;
}

fn derive_solana_address(seed_scalar: ptr<function, BigInt>, debug_out: ptr<function, array<u32, 64>>) -> AddressBuf {
    var seed_bytes = seed_scalar_to_seed_bytes(seed_scalar);
    var ed_scalar = derive_ed25519_scalar_from_seed(&seed_bytes);
    
    // Write first 64 bytes of SHA-512 into debug_out for verification
    var digest_full = sha512_32(&seed_bytes);
    for(var i: u32 = 0u; i < 64u; i = i + 1u) {
        (*debug_out)[i] = digest_full[i];
    }

    var p = get_p();
    var p_wide = get_p_wide();
    var r = get_r();
    var rinv = get_rinv();
    var mu_fp = get_mu_fp();

    var ext = ete_fixed_mul_workgroup(&ed_scalar, &p, &r);
    var aff = ete_to_affine_non_mont(&ext, &p, &p_wide, &r, &rinv, &mu_fp);
    var compressed_words = compress_eteaffine(&aff, {{ log_limb_size }}u);

    var compressed_bytes: array<u32, 32> = array<u32, 32>();
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let w = compressed_words[i];
        let j = i * 4u;
        compressed_bytes[j] = w & 0xffu;
        compressed_bytes[j + 1u] = (w >> 8u) & 0xffu;
        compressed_bytes[j + 2u] = (w >> 16u) & 0xffu;
        compressed_bytes[j + 3u] = (w >> 24u) & 0xffu;
    }

    return base58_encode_32(&compressed_bytes, 32u);
}

@compute
@workgroup_size(256)
fn ed25519_sol_vanity_search(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    if (local_id.x < {{ table_size }}u) {
        ED25519_TABLE_WG[local_id.x] = load_table_point(local_id.x);
    }
    workgroupBarrier();

    if (global_id.x == 0u) {
        counter_limbs[0] = params.line1.z;
    }

    let gid = global_id.x;
    let candidates_per_invocation = max(params.line1.w, 1u);
    let base_index = gid * candidates_per_invocation;
    if (base_index >= params.line0.x) {
        return;
    }

    let attempt_base_lo = params.line1.x;
    let attempt_base_hi = params.line1.y;

    for (var lane: u32 = 0u; lane < candidates_per_invocation; lane = lane + 1u) {
        let candidate_index = base_index + lane;
        if (candidate_index >= params.line0.x) {
            break;
        }

        if (result_words[0] != RESULT_SENTINEL) {
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

        var seed_scalar = derive_seed_scalar(add_lo, add_hi);
        var digest: array<u32, 64> = array<u32, 64>();
        var address = derive_solana_address(&seed_scalar, &digest);
        if (!address_matches(&address)) {
            continue;
        }

        var attempts_lo = add_lo + 1u;
        var attempts_hi = add_hi;
        if (attempts_lo == 0u) {
            attempts_hi = attempts_hi + 1u;
        }

        var scalar_words = seed_scalar_to_result_words(&seed_scalar);
        store_match(
            candidate_index,
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
