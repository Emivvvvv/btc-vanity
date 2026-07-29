var<private> RIPEMD_R1: array<u32, 80> = array<u32, 80>(
    0u, 1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u, 9u, 10u, 11u, 12u, 13u, 14u, 15u,
    7u, 4u, 13u, 1u, 10u, 6u, 15u, 3u, 12u, 0u, 9u, 5u, 2u, 14u, 11u, 8u,
    3u, 10u, 14u, 4u, 9u, 15u, 8u, 1u, 2u, 7u, 0u, 6u, 13u, 11u, 5u, 12u,
    1u, 9u, 11u, 10u, 0u, 8u, 12u, 4u, 13u, 3u, 7u, 15u, 14u, 5u, 6u, 2u,
    4u, 0u, 5u, 9u, 7u, 12u, 2u, 10u, 14u, 1u, 3u, 8u, 11u, 6u, 15u, 13u
);

var<private> RIPEMD_R2: array<u32, 80> = array<u32, 80>(
    5u, 14u, 7u, 0u, 9u, 2u, 11u, 4u, 13u, 6u, 15u, 8u, 1u, 10u, 3u, 12u,
    6u, 11u, 3u, 7u, 0u, 13u, 5u, 10u, 14u, 15u, 8u, 12u, 4u, 9u, 1u, 2u,
    15u, 5u, 1u, 3u, 7u, 14u, 6u, 9u, 11u, 8u, 12u, 2u, 10u, 0u, 4u, 13u,
    8u, 6u, 4u, 1u, 3u, 11u, 15u, 0u, 5u, 12u, 2u, 13u, 9u, 7u, 10u, 14u,
    12u, 15u, 10u, 4u, 1u, 5u, 8u, 7u, 6u, 2u, 13u, 14u, 0u, 3u, 9u, 11u
);

var<private> RIPEMD_S1: array<u32, 80> = array<u32, 80>(
    11u, 14u, 15u, 12u, 5u, 8u, 7u, 9u, 11u, 13u, 14u, 15u, 6u, 7u, 9u, 8u,
    7u, 6u, 8u, 13u, 11u, 9u, 7u, 15u, 7u, 12u, 15u, 9u, 11u, 7u, 13u, 12u,
    11u, 13u, 6u, 7u, 14u, 9u, 13u, 15u, 14u, 8u, 13u, 6u, 5u, 12u, 7u, 5u,
    11u, 12u, 14u, 15u, 14u, 15u, 9u, 8u, 9u, 14u, 5u, 6u, 8u, 6u, 5u, 12u,
    9u, 15u, 5u, 11u, 6u, 8u, 13u, 12u, 5u, 12u, 13u, 14u, 11u, 8u, 5u, 6u
);

var<private> RIPEMD_S2: array<u32, 80> = array<u32, 80>(
    8u, 9u, 9u, 11u, 13u, 15u, 15u, 5u, 7u, 7u, 8u, 11u, 14u, 14u, 12u, 6u,
    9u, 13u, 15u, 7u, 12u, 8u, 9u, 11u, 7u, 7u, 12u, 7u, 6u, 15u, 13u, 11u,
    9u, 7u, 15u, 11u, 8u, 6u, 6u, 14u, 12u, 13u, 5u, 14u, 13u, 13u, 7u, 5u,
    15u, 5u, 8u, 11u, 14u, 14u, 6u, 14u, 6u, 9u, 12u, 9u, 12u, 5u, 15u, 8u,
    8u, 5u, 12u, 9u, 12u, 5u, 14u, 6u, 8u, 13u, 6u, 5u, 15u, 13u, 11u, 11u
);

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

fn ripemd_msg_word(m: ptr<function, array<u32, 16>>, idx: u32) -> u32 {
    switch (idx) {
        case 0u: { return (*m)[0]; }
        case 1u: { return (*m)[1]; }
        case 2u: { return (*m)[2]; }
        case 3u: { return (*m)[3]; }
        case 4u: { return (*m)[4]; }
        case 5u: { return (*m)[5]; }
        case 6u: { return (*m)[6]; }
        case 7u: { return (*m)[7]; }
        case 8u: { return (*m)[8]; }
        case 9u: { return (*m)[9]; }
        case 10u: { return (*m)[10]; }
        case 11u: { return (*m)[11]; }
        case 12u: { return (*m)[12]; }
        case 13u: { return (*m)[13]; }
        case 14u: { return (*m)[14]; }
        default: { return (*m)[15]; }
    }
}

fn ripemd160_var(input: ptr<function, array<u32, 64>>, input_len: u32) -> array<u32, 20> {
    var block: array<u32, 64> = array<u32, 64>();
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
        let ml = ripemd_msg_word(&m, RIPEMD_R1[j]);
        let tl = rotl32(al + ripemd_f(j, bl, cl, dl) + ml + ripemd_kl(j), RIPEMD_S1[j]) + el;
        al = el;
        el = dl;
        dl = rotl32(cl, 10u);
        cl = bl;
        bl = tl;

        let jr = 79u - j;
        let mr = ripemd_msg_word(&m, RIPEMD_R2[j]);
        let tr = rotl32(ar + ripemd_f(jr, br, cr, dr) + mr + ripemd_kr(j), RIPEMD_S2[j]) + er;
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
