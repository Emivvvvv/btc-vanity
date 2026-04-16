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
const KECCAK_RHO: array<u32, 25> = array<u32, 25>(
    0u, 1u, 62u, 28u, 27u,
    36u, 44u, 6u, 55u, 20u,
    3u, 10u, 43u, 25u, 39u,
    41u, 45u, 15u, 21u, 8u,
    18u, 2u, 61u, 56u, 14u
);
const KECCAK_RC: array<vec2<u32>, 24> = array<vec2<u32>, 24>(
    vec2<u32>(0x00000001u, 0x00000000u),
    vec2<u32>(0x00008082u, 0x00000000u),
    vec2<u32>(0x0000808au, 0x80000000u),
    vec2<u32>(0x80008000u, 0x80000000u),
    vec2<u32>(0x0000808bu, 0x00000000u),
    vec2<u32>(0x80000001u, 0x00000000u),
    vec2<u32>(0x80008081u, 0x80000000u),
    vec2<u32>(0x00008009u, 0x80000000u),
    vec2<u32>(0x0000008au, 0x00000000u),
    vec2<u32>(0x00000088u, 0x00000000u),
    vec2<u32>(0x80008009u, 0x00000000u),
    vec2<u32>(0x8000000au, 0x00000000u),
    vec2<u32>(0x8000808bu, 0x00000000u),
    vec2<u32>(0x0000008bu, 0x80000000u),
    vec2<u32>(0x00008089u, 0x80000000u),
    vec2<u32>(0x00008003u, 0x80000000u),
    vec2<u32>(0x00008002u, 0x80000000u),
    vec2<u32>(0x00000080u, 0x80000000u),
    vec2<u32>(0x0000800au, 0x00000000u),
    vec2<u32>(0x8000000au, 0x80000000u),
    vec2<u32>(0x80008081u, 0x80000000u),
    vec2<u32>(0x00008080u, 0x80000000u),
    vec2<u32>(0x80000001u, 0x00000000u),
    vec2<u32>(0x80008008u, 0x80000000u)
);

fn keccak_rho(index: u32) -> u32 {
    switch (index) {
        case 0u: { return 0u; }
        case 1u: { return 1u; }
        case 2u: { return 62u; }
        case 3u: { return 28u; }
        case 4u: { return 27u; }
        case 5u: { return 36u; }
        case 6u: { return 44u; }
        case 7u: { return 6u; }
        case 8u: { return 55u; }
        case 9u: { return 20u; }
        case 10u: { return 3u; }
        case 11u: { return 10u; }
        case 12u: { return 43u; }
        case 13u: { return 25u; }
        case 14u: { return 39u; }
        case 15u: { return 41u; }
        case 16u: { return 45u; }
        case 17u: { return 15u; }
        case 18u: { return 21u; }
        case 19u: { return 8u; }
        case 20u: { return 18u; }
        case 21u: { return 2u; }
        case 22u: { return 61u; }
        case 23u: { return 56u; }
        default: { return 14u; }
    }
}

fn keccak_rc(round: u32) -> vec2<u32> {
    switch (round) {
        case 0u: { return vec2<u32>(0x00000001u, 0x00000000u); }
        case 1u: { return vec2<u32>(0x00008082u, 0x00000000u); }
        case 2u: { return vec2<u32>(0x0000808au, 0x80000000u); }
        case 3u: { return vec2<u32>(0x80008000u, 0x80000000u); }
        case 4u: { return vec2<u32>(0x0000808bu, 0x00000000u); }
        case 5u: { return vec2<u32>(0x80000001u, 0x00000000u); }
        case 6u: { return vec2<u32>(0x80008081u, 0x80000000u); }
        case 7u: { return vec2<u32>(0x00008009u, 0x80000000u); }
        case 8u: { return vec2<u32>(0x0000008au, 0x00000000u); }
        case 9u: { return vec2<u32>(0x00000088u, 0x00000000u); }
        case 10u: { return vec2<u32>(0x80008009u, 0x00000000u); }
        case 11u: { return vec2<u32>(0x8000000au, 0x00000000u); }
        case 12u: { return vec2<u32>(0x8000808bu, 0x00000000u); }
        case 13u: { return vec2<u32>(0x0000008bu, 0x80000000u); }
        case 14u: { return vec2<u32>(0x00008089u, 0x80000000u); }
        case 15u: { return vec2<u32>(0x00008003u, 0x80000000u); }
        case 16u: { return vec2<u32>(0x00008002u, 0x80000000u); }
        case 17u: { return vec2<u32>(0x00000080u, 0x80000000u); }
        case 18u: { return vec2<u32>(0x0000800au, 0x00000000u); }
        case 19u: { return vec2<u32>(0x8000000au, 0x80000000u); }
        case 20u: { return vec2<u32>(0x80008081u, 0x80000000u); }
        case 21u: { return vec2<u32>(0x00008080u, 0x80000000u); }
        case 22u: { return vec2<u32>(0x80000001u, 0x00000000u); }
        default: { return vec2<u32>(0x80008008u, 0x80000000u); }
    }
}

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
    var scalar_bits: array<bool, 256>;

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

fn u64_xor(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x ^ b.x, a.y ^ b.y);
}

fn u64_and(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(a.x & b.x, a.y & b.y);
}

fn u64_not(a: vec2<u32>) -> vec2<u32> {
    return vec2<u32>(~a.x, ~a.y);
}

fn rotl64(a: vec2<u32>, n: u32) -> vec2<u32> {
    let r = n & 63u;
    if (r == 0u) {
        return a;
    }
    if (r < 32u) {
        return vec2<u32>(
            (a.x << r) | (a.y >> (32u - r)),
            (a.y << r) | (a.x >> (32u - r)),
        );
    }
    if (r == 32u) {
        return vec2<u32>(a.y, a.x);
    }
    let rr = r - 32u;
    return vec2<u32>(
        (a.y << rr) | (a.x >> (32u - rr)),
        (a.x << rr) | (a.y >> (32u - rr)),
    );
}

fn keccakf(state: ptr<function, array<vec2<u32>, 25>>) {
    for (var round: u32 = 0u; round < 24u; round = round + 1u) {
        var c: array<vec2<u32>, 5>;
        for (var x: u32 = 0u; x < 5u; x = x + 1u) {
            c[x] = u64_xor(
                u64_xor((*state)[x], (*state)[x + 5u]),
                u64_xor((*state)[x + 10u], u64_xor((*state)[x + 15u], (*state)[x + 20u])),
            );
        }

        var d: array<vec2<u32>, 5>;
        for (var x: u32 = 0u; x < 5u; x = x + 1u) {
            d[x] = u64_xor(c[(x + 4u) % 5u], rotl64(c[(x + 1u) % 5u], 1u));
        }

        for (var y: u32 = 0u; y < 5u; y = y + 1u) {
            for (var x: u32 = 0u; x < 5u; x = x + 1u) {
                let i = x + 5u * y;
                (*state)[i] = u64_xor((*state)[i], d[x]);
            }
        }

        var b: array<vec2<u32>, 25>;
        for (var y: u32 = 0u; y < 5u; y = y + 1u) {
            for (var x: u32 = 0u; x < 5u; x = x + 1u) {
                let i = x + 5u * y;
                let nx = y;
                let ny = (2u * x + 3u * y) % 5u;
                b[nx + 5u * ny] = rotl64((*state)[i], keccak_rho(i));
            }
        }

        for (var y: u32 = 0u; y < 5u; y = y + 1u) {
            for (var x: u32 = 0u; x < 5u; x = x + 1u) {
                let i = x + 5u * y;
                let i1 = ((x + 1u) % 5u) + 5u * y;
                let i2 = ((x + 2u) % 5u) + 5u * y;
                (*state)[i] = u64_xor(b[i], u64_and(u64_not(b[i1]), b[i2]));
            }
        }

        (*state)[0] = u64_xor((*state)[0], keccak_rc(round));
    }
}

fn keccak256_64(input: ptr<function, array<u32, 64>>) -> array<u32, 32> {
    var state: array<vec2<u32>, 25>;

    for (var lane: u32 = 0u; lane < 8u; lane = lane + 1u) {
        let b = lane * 8u;
        let lo = (*input)[b]
            | ((*input)[b + 1u] << 8u)
            | ((*input)[b + 2u] << 16u)
            | ((*input)[b + 3u] << 24u);
        let hi = (*input)[b + 4u]
            | ((*input)[b + 5u] << 8u)
            | ((*input)[b + 6u] << 16u)
            | ((*input)[b + 7u] << 24u);
        state[lane] = vec2<u32>(lo, hi);
    }

    state[8].x = state[8].x ^ 0x00000001u;
    state[16].y = state[16].y ^ 0x80000000u;

    keccakf(&state);

    var out: array<u32, 32>;
    for (var lane: u32 = 0u; lane < 4u; lane = lane + 1u) {
        let v = state[lane];
        let b = lane * 8u;
        out[b] = v.x & 0xffu;
        out[b + 1u] = (v.x >> 8u) & 0xffu;
        out[b + 2u] = (v.x >> 16u) & 0xffu;
        out[b + 3u] = (v.x >> 24u) & 0xffu;
        out[b + 4u] = v.y & 0xffu;
        out[b + 5u] = (v.y >> 8u) & 0xffu;
        out[b + 6u] = (v.y >> 16u) & 0xffu;
        out[b + 7u] = (v.y >> 24u) & 0xffu;
    }
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

fn scalar_to_result_words(scalar: ptr<function, BigInt>) -> array<u32, 8> {
    var bytes = limbs_le_to_bytes_be(&(*scalar).limbs, {{ log_limb_size }}u);
    var out: array<u32, 8>;
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let j = i * 4u;
        out[i] = bytes[j] | (bytes[j + 1u] << 8u) | (bytes[j + 2u] << 16u) | (bytes[j + 3u] << 24u);
    }
    return out;
}

fn store_match(
    winner_index: u32,
    attempts_lo: u32,
    attempts_hi: u32,
    batches: u32,
    scalar_words: ptr<function, array<u32, 8>>,
    address: ptr<function, AddressBuf>,
) {
    if (result_words[0] != RESULT_SENTINEL) {
        return;
    }
    result_words[0] = winner_index;

    result_words[1] = attempts_lo;
    result_words[2] = attempts_hi;
    result_words[3] = batches;
    result_words[4] = (*address).len;

    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        result_words[RESULT_SCALAR_BASE + i] = (*scalar_words)[i];
    }
    for (var i: u32 = 0u; i < (*address).len; i = i + 1u) {
        result_words[RESULT_ADDRESS_BASE + i] = (*address).data[i];
    }
}

fn derive_eth_address(
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    scalar: ptr<function, BigInt>,
) -> AddressBuf {
    var proj = projective_fixed_mul_workgroup(scalar, p, r);
    var aff = projective_to_affine_non_mont(&proj, p, p_wide, r, rinv, mu_fp);

    var x = limbs_le_to_bytes_be(&aff.x.limbs, {{ log_limb_size }}u);
    var y = limbs_le_to_bytes_be(&aff.y.limbs, {{ log_limb_size }}u);

    var pubkey_xy: array<u32, 64>;
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        pubkey_xy[i] = x[i];
        pubkey_xy[32u + i] = y[i];
    }

    var digest = keccak256_64(&pubkey_xy);

    var out: AddressBuf;
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
    if (result_words[0] != RESULT_SENTINEL) {
        return;
    }

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

    if (local_id.x < {{ table_size }}u) {
        SECP256K1_TABLE_WG[local_id.x] = load_table_point(local_id.x);
    }
    workgroupBarrier();

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

        var scalar = derive_secp_scalar(add_lo, add_hi);
        var address = derive_eth_address(
            &p,
            &p_wide,
            &r,
            &rinv,
            &mu_fp,
            &scalar,
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
            candidate_index,
            attempts_lo,
            attempts_hi,
            params.line1.z + 1u,
            &scalar_words,
            &address,
        );
        return;
    }
}
