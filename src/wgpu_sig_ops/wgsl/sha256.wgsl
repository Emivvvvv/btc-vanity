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

fn sha256_var(input: ptr<function, array<u32, 64>>, input_len: u32) -> array<u32, 32> {
    var sha_k = SHA256_K;
    var block: array<u32, 64> = array<u32, 64>();
    var w: array<u32, 64> = array<u32, 64>();
    for (var i: u32 = 0u; i < input_len; i = i + 1u) {
        block[i] = (*input)[i] & 0xffu;
    }
    block[input_len] = 0x80u;

    let bit_len = input_len * 8u;
    block[63] = bit_len & 0xffu;
    block[62] = (bit_len >> 8u) & 0xffu;
    block[61] = (bit_len >> 16u) & 0xffu;
    block[60] = (bit_len >> 24u) & 0xffu;
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
