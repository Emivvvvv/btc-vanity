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
const B58_ALPHABET: array<u32, 58> = array<u32, 58>(
    49u, 50u, 51u, 52u, 53u, 54u, 55u, 56u, 57u,
    65u, 66u, 67u, 68u, 69u, 70u, 71u, 72u, 74u, 75u,
    76u, 77u, 78u, 80u, 81u, 82u, 83u, 84u, 85u, 86u,
    87u, 88u, 89u, 90u,
    97u, 98u, 99u, 100u, 101u, 102u, 103u, 104u, 105u,
    106u, 107u, 109u, 110u, 111u, 112u, 113u, 114u, 115u,
    116u, 117u, 118u, 119u, 120u, 121u, 122u,
);

const SHA256_K: array<u32, 64> = array<u32, 64>(
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u,
    0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
    0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
    0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
    0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu,
    0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u,
    0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
    0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
    0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
    0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u,
    0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u,
    0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
    0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
    0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u
);

const RIPEMD_R1: array<u32, 80> = array<u32, 80>(
    0u, 1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u, 9u, 10u, 11u, 12u, 13u, 14u, 15u,
    7u, 4u, 13u, 1u, 10u, 6u, 15u, 3u, 12u, 0u, 9u, 5u, 2u, 14u, 11u, 8u,
    3u, 10u, 14u, 4u, 9u, 15u, 8u, 1u, 2u, 7u, 0u, 6u, 13u, 11u, 5u, 12u,
    1u, 9u, 11u, 10u, 0u, 8u, 12u, 4u, 13u, 3u, 7u, 15u, 14u, 5u, 6u, 2u,
    4u, 0u, 5u, 9u, 7u, 12u, 2u, 10u, 14u, 1u, 3u, 8u, 11u, 6u, 15u, 13u
);

const RIPEMD_R2: array<u32, 80> = array<u32, 80>(
    5u, 14u, 7u, 0u, 9u, 2u, 11u, 4u, 13u, 6u, 15u, 8u, 1u, 10u, 3u, 12u,
    6u, 11u, 3u, 7u, 0u, 13u, 5u, 10u, 14u, 15u, 8u, 12u, 4u, 9u, 1u, 2u,
    15u, 5u, 1u, 3u, 7u, 14u, 6u, 9u, 11u, 8u, 12u, 2u, 10u, 0u, 4u, 13u,
    8u, 6u, 4u, 1u, 3u, 11u, 15u, 0u, 5u, 12u, 2u, 13u, 9u, 7u, 10u, 14u,
    12u, 15u, 10u, 4u, 1u, 5u, 8u, 7u, 6u, 2u, 13u, 14u, 0u, 3u, 9u, 11u
);

const RIPEMD_S1: array<u32, 80> = array<u32, 80>(
    11u, 14u, 15u, 12u, 5u, 8u, 7u, 9u, 11u, 13u, 14u, 15u, 6u, 7u, 9u, 8u,
    7u, 6u, 8u, 13u, 11u, 9u, 7u, 15u, 7u, 12u, 15u, 9u, 11u, 7u, 13u, 12u,
    11u, 13u, 6u, 7u, 14u, 9u, 13u, 15u, 14u, 8u, 13u, 6u, 5u, 12u, 7u, 5u,
    11u, 12u, 14u, 15u, 14u, 15u, 9u, 8u, 9u, 14u, 5u, 6u, 8u, 6u, 5u, 12u,
    9u, 15u, 5u, 11u, 6u, 8u, 13u, 12u, 5u, 12u, 13u, 14u, 11u, 8u, 5u, 6u
);

const RIPEMD_S2: array<u32, 80> = array<u32, 80>(
    8u, 9u, 9u, 11u, 13u, 15u, 15u, 5u, 7u, 7u, 8u, 11u, 14u, 14u, 12u, 6u,
    9u, 13u, 15u, 7u, 12u, 8u, 9u, 11u, 7u, 7u, 12u, 7u, 6u, 15u, 13u, 11u,
    9u, 7u, 15u, 11u, 8u, 6u, 6u, 14u, 12u, 13u, 5u, 14u, 13u, 13u, 7u, 5u,
    15u, 5u, 8u, 11u, 14u, 14u, 6u, 14u, 6u, 9u, 12u, 9u, 12u, 5u, 15u, 8u,
    8u, 5u, 12u, 9u, 12u, 5u, 14u, 6u, 8u, 13u, 6u, 5u, 15u, 13u, 11u, 11u
);

fn rotr32(x: u32, n: u32) -> u32 {
    if (n == 0u) {
        return x;
    }
    return (x >> n) | (x << (32u - n));
}

fn rotl32(x: u32, n: u32) -> u32 {
    if (n == 0u) {
        return x;
    }
    return (x << n) | (x >> (32u - n));
}

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

fn sha256_var(input: ptr<function, array<u32, 64>>, input_len: u32) -> array<u32, 32> {
    var sha_k = SHA256_K;
    var block: array<u32, 64>;
    for (var i: u32 = 0u; i < input_len; i = i + 1u) {
        block[i] = (*input)[i] & 0xffu;
    }
    block[input_len] = 0x80u;

    let bit_len = input_len * 8u;
    block[63] = bit_len & 0xffu;
    block[62] = (bit_len >> 8u) & 0xffu;
    block[61] = (bit_len >> 16u) & 0xffu;
    block[60] = (bit_len >> 24u) & 0xffu;

    var w: array<u32, 64>;
    for (var i: u32 = 0u; i < 16u; i = i + 1u) {
        let b = i * 4u;
        w[i] = (block[b] << 24u) | (block[b + 1u] << 16u) | (block[b + 2u] << 8u) | block[b + 3u];
    }

    for (var i: u32 = 16u; i < 64u; i = i + 1u) {
        let s0 = rotr32(w[i - 15u], 7u) ^ rotr32(w[i - 15u], 18u) ^ (w[i - 15u] >> 3u);
        let s1 = rotr32(w[i - 2u], 17u) ^ rotr32(w[i - 2u], 19u) ^ (w[i - 2u] >> 10u);
        w[i] = w[i - 16u] + s0 + w[i - 7u] + s1;
    }

    var a = 0x6a09e667u;
    var b = 0xbb67ae85u;
    var c = 0x3c6ef372u;
    var d = 0xa54ff53au;
    var e = 0x510e527fu;
    var f = 0x9b05688cu;
    var g = 0x1f83d9abu;
    var h = 0x5be0cd19u;

    for (var i: u32 = 0u; i < 64u; i = i + 1u) {
        let s1 = rotr32(e, 6u) ^ rotr32(e, 11u) ^ rotr32(e, 25u);
        let ch = (e & f) ^ ((~e) & g);
        let temp1 = h + s1 + ch + sha_k[i] + w[i];
        let s0 = rotr32(a, 2u) ^ rotr32(a, 13u) ^ rotr32(a, 22u);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0 + maj;

        h = g;
        g = f;
        f = e;
        e = d + temp1;
        d = c;
        c = b;
        b = a;
        a = temp1 + temp2;
    }

    a = a + 0x6a09e667u;
    b = b + 0xbb67ae85u;
    c = c + 0x3c6ef372u;
    d = d + 0xa54ff53au;
    e = e + 0x510e527fu;
    f = f + 0x9b05688cu;
    g = g + 0x1f83d9abu;
    h = h + 0x5be0cd19u;

    var out: array<u32, 32>;
    out[0] = (a >> 24u) & 0xffu;
    out[1] = (a >> 16u) & 0xffu;
    out[2] = (a >> 8u) & 0xffu;
    out[3] = a & 0xffu;
    out[4] = (b >> 24u) & 0xffu;
    out[5] = (b >> 16u) & 0xffu;
    out[6] = (b >> 8u) & 0xffu;
    out[7] = b & 0xffu;
    out[8] = (c >> 24u) & 0xffu;
    out[9] = (c >> 16u) & 0xffu;
    out[10] = (c >> 8u) & 0xffu;
    out[11] = c & 0xffu;
    out[12] = (d >> 24u) & 0xffu;
    out[13] = (d >> 16u) & 0xffu;
    out[14] = (d >> 8u) & 0xffu;
    out[15] = d & 0xffu;
    out[16] = (e >> 24u) & 0xffu;
    out[17] = (e >> 16u) & 0xffu;
    out[18] = (e >> 8u) & 0xffu;
    out[19] = e & 0xffu;
    out[20] = (f >> 24u) & 0xffu;
    out[21] = (f >> 16u) & 0xffu;
    out[22] = (f >> 8u) & 0xffu;
    out[23] = f & 0xffu;
    out[24] = (g >> 24u) & 0xffu;
    out[25] = (g >> 16u) & 0xffu;
    out[26] = (g >> 8u) & 0xffu;
    out[27] = g & 0xffu;
    out[28] = (h >> 24u) & 0xffu;
    out[29] = (h >> 16u) & 0xffu;
    out[30] = (h >> 8u) & 0xffu;
    out[31] = h & 0xffu;
    return out;
}

fn ripemd_f(j: u32, x: u32, y: u32, z: u32) -> u32 {
    if (j < 16u) {
        return x ^ y ^ z;
    }
    if (j < 32u) {
        return (x & y) | ((~x) & z);
    }
    if (j < 48u) {
        return (x | (~y)) ^ z;
    }
    if (j < 64u) {
        return (x & z) | (y & (~z));
    }
    return x ^ (y | (~z));
}

fn ripemd_kl(j: u32) -> u32 {
    if (j < 16u) {
        return 0x00000000u;
    }
    if (j < 32u) {
        return 0x5a827999u;
    }
    if (j < 48u) {
        return 0x6ed9eba1u;
    }
    if (j < 64u) {
        return 0x8f1bbcdcu;
    }
    return 0xa953fd4eu;
}

fn ripemd_kr(j: u32) -> u32 {
    if (j < 16u) {
        return 0x50a28be6u;
    }
    if (j < 32u) {
        return 0x5c4dd124u;
    }
    if (j < 48u) {
        return 0x6d703ef3u;
    }
    if (j < 64u) {
        return 0x7a6d76e9u;
    }
    return 0x00000000u;
}

fn ripemd160_var(input: ptr<function, array<u32, 64>>, input_len: u32) -> array<u32, 20> {
    var r1 = RIPEMD_R1;
    var r2 = RIPEMD_R2;
    var s1 = RIPEMD_S1;
    var s2 = RIPEMD_S2;
    var block: array<u32, 64>;
    for (var i: u32 = 0u; i < input_len; i = i + 1u) {
        block[i] = (*input)[i] & 0xffu;
    }
    block[input_len] = 0x80u;

    let bit_len = input_len * 8u;
    block[56] = bit_len & 0xffu;
    block[57] = (bit_len >> 8u) & 0xffu;
    block[58] = (bit_len >> 16u) & 0xffu;
    block[59] = (bit_len >> 24u) & 0xffu;

    var m: array<u32, 16>;
    for (var i: u32 = 0u; i < 16u; i = i + 1u) {
        let b = i * 4u;
        m[i] = block[b] | (block[b + 1u] << 8u) | (block[b + 2u] << 16u) | (block[b + 3u] << 24u);
    }

    var h0 = 0x67452301u;
    var h1 = 0xefcdab89u;
    var h2 = 0x98badcfeu;
    var h3 = 0x10325476u;
    var h4 = 0xc3d2e1f0u;

    var al = h0;
    var bl = h1;
    var cl = h2;
    var dl = h3;
    var el = h4;

    var ar = h0;
    var br = h1;
    var cr = h2;
    var dr = h3;
    var er = h4;

    for (var j: u32 = 0u; j < 80u; j = j + 1u) {
        let tl = rotl32(al + ripemd_f(j, bl, cl, dl) + m[r1[j]] + ripemd_kl(j), s1[j]) + el;
        al = el;
        el = dl;
        dl = rotl32(cl, 10u);
        cl = bl;
        bl = tl;

        let jr = 79u - j;
        let tr = rotl32(ar + ripemd_f(jr, br, cr, dr) + m[r2[j]] + ripemd_kr(j), s2[j]) + er;
        ar = er;
        er = dr;
        dr = rotl32(cr, 10u);
        cr = br;
        br = tr;
    }

    let t = h1 + cl + dr;
    h1 = h2 + dl + er;
    h2 = h3 + el + ar;
    h3 = h4 + al + br;
    h4 = h0 + bl + cr;
    h0 = t;

    var out: array<u32, 20>;
    out[0] = h0 & 0xffu;
    out[1] = (h0 >> 8u) & 0xffu;
    out[2] = (h0 >> 16u) & 0xffu;
    out[3] = (h0 >> 24u) & 0xffu;
    out[4] = h1 & 0xffu;
    out[5] = (h1 >> 8u) & 0xffu;
    out[6] = (h1 >> 16u) & 0xffu;
    out[7] = (h1 >> 24u) & 0xffu;
    out[8] = h2 & 0xffu;
    out[9] = (h2 >> 8u) & 0xffu;
    out[10] = (h2 >> 16u) & 0xffu;
    out[11] = (h2 >> 24u) & 0xffu;
    out[12] = h3 & 0xffu;
    out[13] = (h3 >> 8u) & 0xffu;
    out[14] = (h3 >> 16u) & 0xffu;
    out[15] = (h3 >> 24u) & 0xffu;
    out[16] = h4 & 0xffu;
    out[17] = (h4 >> 8u) & 0xffu;
    out[18] = (h4 >> 16u) & 0xffu;
    out[19] = (h4 >> 24u) & 0xffu;
    return out;
}

fn base58_encode_var(input: ptr<function, array<u32, 32>>, input_len: u32) -> AddressBuf {
    var out: AddressBuf;
    var alphabet = B58_ALPHABET;
    var digits: array<u32, 64>;
    var data: array<u32, 32>;

    for (var i: u32 = 0u; i < input_len; i = i + 1u) {
        data[i] = (*input)[i] & 0xffu;
    }

    var zero_count = 0u;
    while (zero_count < input_len && data[zero_count] == 0u) {
        zero_count = zero_count + 1u;
    }

    var digits_len = 0u;
    for (var i: u32 = zero_count; i < input_len; i = i + 1u) {
        var carry = data[i];
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

fn derive_btc_address(
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    r: ptr<function, BigInt>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
    scalar: ptr<function, BigInt>,
) -> AddressBuf {
    var proj = projective_fixed_mul_workgroup(scalar, p, r);
    var aff = projective_to_affine_non_mont(&proj, p, p_wide, r, rinv, mu_fp);

    var x_bytes = limbs_le_to_bytes_be(&aff.x.limbs, {{ log_limb_size }}u);
    var pubkey: array<u32, 64>;
    pubkey[0] = select(3u, 2u, bigint_is_even(&aff.y));
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        pubkey[i + 1u] = x_bytes[i];
    }

    var sha1 = sha256_var(&pubkey, 33u);
    var sha_in: array<u32, 64>;
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        sha_in[i] = sha1[i];
    }
    var h160 = ripemd160_var(&sha_in, 32u);

    var payload21: array<u32, 64>;
    payload21[0] = 0u;
    for (var i: u32 = 0u; i < 20u; i = i + 1u) {
        payload21[i + 1u] = h160[i];
    }

    var chk1 = sha256_var(&payload21, 21u);
    var chk_in: array<u32, 64>;
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        chk_in[i] = chk1[i];
    }
    var chk2 = sha256_var(&chk_in, 32u);

    var payload25: array<u32, 32>;
    for (var i: u32 = 0u; i < 21u; i = i + 1u) {
        payload25[i] = payload21[i];
    }
    for (var i: u32 = 0u; i < 4u; i = i + 1u) {
        payload25[21u + i] = chk2[i];
    }

    return base58_encode_var(&payload25, 25u);
}

@compute
@workgroup_size(256)
fn secp256k1_btc_vanity_search(
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
        var address = derive_btc_address(
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

