// Optimised Bitcoin P2PKH (compressed) prefix search for OpenCL GPUs.
//
// Each work-item owns a random centre point C = k*G. One dispatch walks the
// 2*GROUP+1 consecutive keys k-GROUP..k+GROUP using affine additions that
// share a single field inversion (Montgomery batch trick), then advances the
// centre by (2*GROUP+1)*G. Every point yields six public keys for free:
// the secp256k1 endomorphism gives (beta*x, y) and (beta^2*x, y) for the
// keys lambda*k and lambda^2*k, and negation flips the y parity for -k.
//
// Matching never base58-encodes: the host turns the requested prefix into
// sorted ranges over the top 64 bits of HASH160, and the host re-derives and
// verifies every reported hit.

typedef uint u32;
typedef ulong u64;

#ifndef GROUP
#define GROUP 256
#endif

__constant u32 BETA[8] = {
    0x719501eeu, 0xc1396c28u, 0x12f58995u, 0x9cf04975u,
    0xac3434e9u, 0x6e64479eu, 0x657c0710u, 0x7ae96a2bu
};

// ---------------------------------------------------------------- field math
// Elements are 8 little-endian 32-bit limbs, always fully reduced mod
// p = 2^256 - 2^32 - 977.

static inline void fe_copy(u32* r, const u32* a) {
    #pragma unroll
    for (int i = 0; i < 8; i++) r[i] = a[i];
}

static inline int fe_gte_p(const u32* a) {
    u32 hi = a[2] & a[3] & a[4] & a[5] & a[6] & a[7];
    if (hi != 0xFFFFFFFFu) return 0;
    if (a[1] == 0xFFFFFFFFu) return 1;
    return (a[1] == 0xFFFFFFFEu) && (a[0] >= 0xFFFFFC2Fu);
}

// a += 2^256 - p (== a - p mod 2^256 when a >= p).
static inline void fe_add_c(u32* a) {
    u64 c = (u64)a[0] + 0x3D1u; a[0] = (u32)c; c >>= 32;
    c += (u64)a[1] + 1u;        a[1] = (u32)c; c >>= 32;
    #pragma unroll
    for (int i = 2; i < 8; i++) { c += a[i]; a[i] = (u32)c; c >>= 32; }
}

static inline void fe_reduce(u32* r, const u32* t) {
    u32 s[8];
    u64 c = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        c += (u64)t[i] + (u64)t[8 + i] * 977u;
        if (i > 0) c += t[7 + i];
        s[i] = (u32)c;
        c >>= 32;
    }
    u64 hi = c + t[15];
    c = (u64)s[0] + hi * 977u; r[0] = (u32)c; c >>= 32;
    c += (u64)s[1] + hi;       r[1] = (u32)c; c >>= 32;
    #pragma unroll
    for (int i = 2; i < 8; i++) { c += s[i]; r[i] = (u32)c; c >>= 32; }
    if (c) fe_add_c(r);
    if (fe_gte_p(r)) fe_add_c(r);
}

static inline void fe_mul(u32* r, const u32* a, const u32* b) {
    u32 t[16];
    #pragma unroll
    for (int i = 0; i < 16; i++) t[i] = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        u64 c = 0;
        #pragma unroll
        for (int j = 0; j < 8; j++) {
            c += (u64)a[i] * b[j] + t[i + j];
            t[i + j] = (u32)c;
            c >>= 32;
        }
        t[i + 8] = (u32)c;
    }
    fe_reduce(r, t);
}

static inline void fe_sqr(u32* r, const u32* a) {
    // Cross products once, doubled, plus the diagonal.
    u32 t[16];
    #pragma unroll
    for (int i = 0; i < 16; i++) t[i] = 0;
    #pragma unroll
    for (int i = 0; i < 7; i++) {
        u64 c = 0;
        #pragma unroll
        for (int j = i + 1; j < 8; j++) {
            c += (u64)a[i] * a[j] + t[i + j];
            t[i + j] = (u32)c;
            c >>= 32;
        }
        t[i + 8] = (u32)c;
    }
    u32 top = 0;
    #pragma unroll
    for (int i = 0; i < 16; i++) {
        u32 v = t[i];
        t[i] = (v << 1) | top;
        top = v >> 31;
    }
    u64 c = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        u64 d = (u64)a[i] * a[i];
        c += (u64)t[2 * i] + (u32)d;
        t[2 * i] = (u32)c; c >>= 32;
        c += (u64)t[2 * i + 1] + (u32)(d >> 32);
        t[2 * i + 1] = (u32)c; c >>= 32;
    }
    fe_reduce(r, t);
}

static inline void fe_sub(u32* r, const u32* a, const u32* b) {
    u32 br = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        u64 d = (u64)a[i] - b[i] - br;
        r[i] = (u32)d;
        br = (u32)(d >> 32) & 1u;
    }
    if (br) {
        // r += p, i.e. r -= 2^256 - p
        u64 d = (u64)r[0] - 0x3D1u; r[0] = (u32)d; br = (u32)(d >> 32) & 1u;
        d = (u64)r[1] - 1u - br;    r[1] = (u32)d; br = (u32)(d >> 32) & 1u;
        #pragma unroll
        for (int i = 2; i < 8; i++) {
            d = (u64)r[i] - br; r[i] = (u32)d; br = (u32)(d >> 32) & 1u;
        }
    }
}

static inline void fe_add(u32* r, const u32* a, const u32* b) {
    u64 c = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) { c += (u64)a[i] + b[i]; r[i] = (u32)c; c >>= 32; }
    if (c) fe_add_c(r);
    if (fe_gte_p(r)) fe_add_c(r);
}

static inline void fe_neg(u32* r, const u32* a) {
    u32 z[8] = {0, 0, 0, 0, 0, 0, 0, 0};
    fe_sub(r, z, a);
}

static inline void fe_sqr_n(u32* r, const u32* a, int n) {
    fe_sqr(r, a);
    #pragma unroll 1
    for (int i = 1; i < n; i++) fe_sqr(r, r);
}

// r = a^(p-2) via libsecp256k1's addition chain (255 squarings, 15 multiplies).
static inline void fe_inv(u32* r, const u32* a) {
    u32 x2[8], x3[8], x6[8], x9[8], x11[8], x22[8], x44[8], x88[8], x176[8], x220[8], x223[8], t[8];
    fe_sqr(x2, a);          fe_mul(x2, x2, a);
    fe_sqr(x3, x2);         fe_mul(x3, x3, a);
    fe_sqr_n(x6, x3, 3);    fe_mul(x6, x6, x3);
    fe_sqr_n(x9, x6, 3);    fe_mul(x9, x9, x3);
    fe_sqr_n(x11, x9, 2);   fe_mul(x11, x11, x2);
    fe_sqr_n(x22, x11, 11); fe_mul(x22, x22, x11);
    fe_sqr_n(x44, x22, 22); fe_mul(x44, x44, x22);
    fe_sqr_n(x88, x44, 44); fe_mul(x88, x88, x44);
    fe_sqr_n(x176, x88, 88); fe_mul(x176, x176, x88);
    fe_sqr_n(x220, x176, 44); fe_mul(x220, x220, x44);
    fe_sqr_n(x223, x220, 3); fe_mul(x223, x223, x3);
    fe_sqr_n(t, x223, 23);  fe_mul(t, t, x22);
    fe_sqr_n(t, t, 5);      fe_mul(t, t, a);
    fe_sqr_n(t, t, 3);      fe_mul(t, t, x2);
    fe_sqr_n(t, t, 2);      fe_mul(r, t, a);
}

// R = A + B for affine points, given inv = 1/(xB - xA).
static inline void point_add(u32* rx, u32* ry, const u32* ax, const u32* ay,
                      const u32* bx, const u32* by, const u32* inv) {
    u32 lam[8], t[8];
    fe_sub(t, by, ay);
    fe_mul(lam, t, inv);
    fe_sqr(t, lam);
    fe_sub(t, t, ax);
    fe_sub(rx, t, bx);
    fe_sub(t, ax, rx);
    fe_mul(t, lam, t);
    fe_sub(ry, t, ay);
}

// ------------------------------------------------------------------- hashing

#define ROTR(x, n) rotate((x), (u32)(32 - (n)))
#define BSWAP(x) as_uint(as_uchar4(x).wzyx)

__constant u32 K256[64] = {
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u,
    0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u, 0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u,
    0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u, 0x06ca6351u, 0x14292967u,
    0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u, 0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
    0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu, 0x682e6ff3u,
    0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u, 0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u
};

static inline void sha256_block(u32* w, u32* out) {
    u32 a = 0x6a09e667u, b = 0xbb67ae85u, c = 0x3c6ef372u, d = 0xa54ff53au;
    u32 e = 0x510e527fu, f = 0x9b05688cu, g = 0x1f83d9abu, h = 0x5be0cd19u;
    #pragma unroll
    for (int t = 0; t < 64; t++) {
        u32 wt;
        if (t < 16) {
            wt = w[t];
        } else {
            u32 w15 = w[(t - 15) & 15], w2 = w[(t - 2) & 15];
            wt = w[t & 15] + (ROTR(w2, 17) ^ ROTR(w2, 19) ^ (w2 >> 10)) + w[(t - 7) & 15]
               + (ROTR(w15, 7) ^ ROTR(w15, 18) ^ (w15 >> 3));
            w[t & 15] = wt;
        }
        u32 t1 = h + (ROTR(e, 6) ^ ROTR(e, 11) ^ ROTR(e, 25)) + bitselect(g, f, e) + K256[t] + wt;
        u32 t2 = (ROTR(a, 2) ^ ROTR(a, 13) ^ ROTR(a, 22)) + bitselect(b, a, b ^ c);
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    out[0] = a + 0x6a09e667u; out[1] = b + 0xbb67ae85u; out[2] = c + 0x3c6ef372u; out[3] = d + 0xa54ff53au;
    out[4] = e + 0x510e527fu; out[5] = f + 0x9b05688cu; out[6] = g + 0x1f83d9abu; out[7] = h + 0x5be0cd19u;
}

#define F1(x, y, z) ((x) ^ (y) ^ (z))
#define F2(x, y, z) bitselect((z), (y), (x))
#define F3(x, y, z) (((x) | ~(y)) ^ (z))
#define F4(x, y, z) bitselect((y), (x), (z))
#define F5(x, y, z) ((x) ^ ((y) | ~(z)))
#define RMD_L(F, X, K, S) { u32 T = rotate(AL + F(BL, CL, DL) + (X) + (K), (u32)(S)) + EL; \
    AL = EL; EL = DL; DL = rotate(CL, 10u); CL = BL; BL = T; }
#define RMD_R(F, X, K, S) { u32 T = rotate(AR + F(BR, CR, DR) + (X) + (K), (u32)(S)) + ER; \
    AR = ER; ER = DR; DR = rotate(CR, 10u); CR = BR; BR = T; }

// HASH160 of the 33-byte compressed key (prefix byte, big-endian x).
// Returns the first 8 digest bytes as a big-endian 64-bit integer, and the
// first digest word (little-endian) through h0_out.
static inline u64 hash160_top64(u32 prefix, const u32* x, u32* h0_out) {
    u32 w[16];
    w[0] = (prefix << 24) | (x[7] >> 8);
    w[1] = (x[7] << 24) | (x[6] >> 8);
    w[2] = (x[6] << 24) | (x[5] >> 8);
    w[3] = (x[5] << 24) | (x[4] >> 8);
    w[4] = (x[4] << 24) | (x[3] >> 8);
    w[5] = (x[3] << 24) | (x[2] >> 8);
    w[6] = (x[2] << 24) | (x[1] >> 8);
    w[7] = (x[1] << 24) | (x[0] >> 8);
    w[8] = (x[0] << 24) | 0x00800000u;
    w[9] = 0; w[10] = 0; w[11] = 0; w[12] = 0; w[13] = 0; w[14] = 0;
    w[15] = 33u * 8u;
    u32 s[8];
    sha256_block(w, s);

    const u32 X0 = BSWAP(s[0]), X1 = BSWAP(s[1]), X2 = BSWAP(s[2]), X3 = BSWAP(s[3]);
    const u32 X4 = BSWAP(s[4]), X5 = BSWAP(s[5]), X6 = BSWAP(s[6]), X7 = BSWAP(s[7]);
    const u32 X8 = 0x80u, X9 = 0, X10 = 0, X11 = 0, X12 = 0, X13 = 0, X14 = 256u, X15 = 0;

    u32 AL = 0x67452301u, BL = 0xEFCDAB89u, CL = 0x98BADCFEu, DL = 0x10325476u, EL = 0xC3D2E1F0u;
    u32 AR = AL, BR = BL, CR = CL, DR = DL, ER = EL;
    RMD_L(F1, X0, 0x00000000u, 11);
    RMD_R(F5, X5, 0x50A28BE6u, 8);
    RMD_L(F1, X1, 0x00000000u, 14);
    RMD_R(F5, X14, 0x50A28BE6u, 9);
    RMD_L(F1, X2, 0x00000000u, 15);
    RMD_R(F5, X7, 0x50A28BE6u, 9);
    RMD_L(F1, X3, 0x00000000u, 12);
    RMD_R(F5, X0, 0x50A28BE6u, 11);
    RMD_L(F1, X4, 0x00000000u, 5);
    RMD_R(F5, X9, 0x50A28BE6u, 13);
    RMD_L(F1, X5, 0x00000000u, 8);
    RMD_R(F5, X2, 0x50A28BE6u, 15);
    RMD_L(F1, X6, 0x00000000u, 7);
    RMD_R(F5, X11, 0x50A28BE6u, 15);
    RMD_L(F1, X7, 0x00000000u, 9);
    RMD_R(F5, X4, 0x50A28BE6u, 5);
    RMD_L(F1, X8, 0x00000000u, 11);
    RMD_R(F5, X13, 0x50A28BE6u, 7);
    RMD_L(F1, X9, 0x00000000u, 13);
    RMD_R(F5, X6, 0x50A28BE6u, 7);
    RMD_L(F1, X10, 0x00000000u, 14);
    RMD_R(F5, X15, 0x50A28BE6u, 8);
    RMD_L(F1, X11, 0x00000000u, 15);
    RMD_R(F5, X8, 0x50A28BE6u, 11);
    RMD_L(F1, X12, 0x00000000u, 6);
    RMD_R(F5, X1, 0x50A28BE6u, 14);
    RMD_L(F1, X13, 0x00000000u, 7);
    RMD_R(F5, X10, 0x50A28BE6u, 14);
    RMD_L(F1, X14, 0x00000000u, 9);
    RMD_R(F5, X3, 0x50A28BE6u, 12);
    RMD_L(F1, X15, 0x00000000u, 8);
    RMD_R(F5, X12, 0x50A28BE6u, 6);
    RMD_L(F2, X7, 0x5A827999u, 7);
    RMD_R(F4, X6, 0x5C4DD124u, 9);
    RMD_L(F2, X4, 0x5A827999u, 6);
    RMD_R(F4, X11, 0x5C4DD124u, 13);
    RMD_L(F2, X13, 0x5A827999u, 8);
    RMD_R(F4, X3, 0x5C4DD124u, 15);
    RMD_L(F2, X1, 0x5A827999u, 13);
    RMD_R(F4, X7, 0x5C4DD124u, 7);
    RMD_L(F2, X10, 0x5A827999u, 11);
    RMD_R(F4, X0, 0x5C4DD124u, 12);
    RMD_L(F2, X6, 0x5A827999u, 9);
    RMD_R(F4, X13, 0x5C4DD124u, 8);
    RMD_L(F2, X15, 0x5A827999u, 7);
    RMD_R(F4, X5, 0x5C4DD124u, 9);
    RMD_L(F2, X3, 0x5A827999u, 15);
    RMD_R(F4, X10, 0x5C4DD124u, 11);
    RMD_L(F2, X12, 0x5A827999u, 7);
    RMD_R(F4, X14, 0x5C4DD124u, 7);
    RMD_L(F2, X0, 0x5A827999u, 12);
    RMD_R(F4, X15, 0x5C4DD124u, 7);
    RMD_L(F2, X9, 0x5A827999u, 15);
    RMD_R(F4, X8, 0x5C4DD124u, 12);
    RMD_L(F2, X5, 0x5A827999u, 9);
    RMD_R(F4, X12, 0x5C4DD124u, 7);
    RMD_L(F2, X2, 0x5A827999u, 11);
    RMD_R(F4, X4, 0x5C4DD124u, 6);
    RMD_L(F2, X14, 0x5A827999u, 7);
    RMD_R(F4, X9, 0x5C4DD124u, 15);
    RMD_L(F2, X11, 0x5A827999u, 13);
    RMD_R(F4, X1, 0x5C4DD124u, 13);
    RMD_L(F2, X8, 0x5A827999u, 12);
    RMD_R(F4, X2, 0x5C4DD124u, 11);
    RMD_L(F3, X3, 0x6ED9EBA1u, 11);
    RMD_R(F3, X15, 0x6D703EF3u, 9);
    RMD_L(F3, X10, 0x6ED9EBA1u, 13);
    RMD_R(F3, X5, 0x6D703EF3u, 7);
    RMD_L(F3, X14, 0x6ED9EBA1u, 6);
    RMD_R(F3, X1, 0x6D703EF3u, 15);
    RMD_L(F3, X4, 0x6ED9EBA1u, 7);
    RMD_R(F3, X3, 0x6D703EF3u, 11);
    RMD_L(F3, X9, 0x6ED9EBA1u, 14);
    RMD_R(F3, X7, 0x6D703EF3u, 8);
    RMD_L(F3, X15, 0x6ED9EBA1u, 9);
    RMD_R(F3, X14, 0x6D703EF3u, 6);
    RMD_L(F3, X8, 0x6ED9EBA1u, 13);
    RMD_R(F3, X6, 0x6D703EF3u, 6);
    RMD_L(F3, X1, 0x6ED9EBA1u, 15);
    RMD_R(F3, X9, 0x6D703EF3u, 14);
    RMD_L(F3, X2, 0x6ED9EBA1u, 14);
    RMD_R(F3, X11, 0x6D703EF3u, 12);
    RMD_L(F3, X7, 0x6ED9EBA1u, 8);
    RMD_R(F3, X8, 0x6D703EF3u, 13);
    RMD_L(F3, X0, 0x6ED9EBA1u, 13);
    RMD_R(F3, X12, 0x6D703EF3u, 5);
    RMD_L(F3, X6, 0x6ED9EBA1u, 6);
    RMD_R(F3, X2, 0x6D703EF3u, 14);
    RMD_L(F3, X13, 0x6ED9EBA1u, 5);
    RMD_R(F3, X10, 0x6D703EF3u, 13);
    RMD_L(F3, X11, 0x6ED9EBA1u, 12);
    RMD_R(F3, X0, 0x6D703EF3u, 13);
    RMD_L(F3, X5, 0x6ED9EBA1u, 7);
    RMD_R(F3, X4, 0x6D703EF3u, 7);
    RMD_L(F3, X12, 0x6ED9EBA1u, 5);
    RMD_R(F3, X13, 0x6D703EF3u, 5);
    RMD_L(F4, X1, 0x8F1BBCDCu, 11);
    RMD_R(F2, X8, 0x7A6D76E9u, 15);
    RMD_L(F4, X9, 0x8F1BBCDCu, 12);
    RMD_R(F2, X6, 0x7A6D76E9u, 5);
    RMD_L(F4, X11, 0x8F1BBCDCu, 14);
    RMD_R(F2, X4, 0x7A6D76E9u, 8);
    RMD_L(F4, X10, 0x8F1BBCDCu, 15);
    RMD_R(F2, X1, 0x7A6D76E9u, 11);
    RMD_L(F4, X0, 0x8F1BBCDCu, 14);
    RMD_R(F2, X3, 0x7A6D76E9u, 14);
    RMD_L(F4, X8, 0x8F1BBCDCu, 15);
    RMD_R(F2, X11, 0x7A6D76E9u, 14);
    RMD_L(F4, X12, 0x8F1BBCDCu, 9);
    RMD_R(F2, X15, 0x7A6D76E9u, 6);
    RMD_L(F4, X4, 0x8F1BBCDCu, 8);
    RMD_R(F2, X0, 0x7A6D76E9u, 14);
    RMD_L(F4, X13, 0x8F1BBCDCu, 9);
    RMD_R(F2, X5, 0x7A6D76E9u, 6);
    RMD_L(F4, X3, 0x8F1BBCDCu, 14);
    RMD_R(F2, X12, 0x7A6D76E9u, 9);
    RMD_L(F4, X7, 0x8F1BBCDCu, 5);
    RMD_R(F2, X2, 0x7A6D76E9u, 12);
    RMD_L(F4, X15, 0x8F1BBCDCu, 6);
    RMD_R(F2, X13, 0x7A6D76E9u, 9);
    RMD_L(F4, X14, 0x8F1BBCDCu, 8);
    RMD_R(F2, X9, 0x7A6D76E9u, 12);
    RMD_L(F4, X5, 0x8F1BBCDCu, 6);
    RMD_R(F2, X7, 0x7A6D76E9u, 5);
    RMD_L(F4, X6, 0x8F1BBCDCu, 5);
    RMD_R(F2, X10, 0x7A6D76E9u, 15);
    RMD_L(F4, X2, 0x8F1BBCDCu, 12);
    RMD_R(F2, X14, 0x7A6D76E9u, 8);
    RMD_L(F5, X4, 0xA953FD4Eu, 9);
    RMD_R(F1, X12, 0x00000000u, 8);
    RMD_L(F5, X0, 0xA953FD4Eu, 15);
    RMD_R(F1, X15, 0x00000000u, 5);
    RMD_L(F5, X5, 0xA953FD4Eu, 5);
    RMD_R(F1, X10, 0x00000000u, 12);
    RMD_L(F5, X9, 0xA953FD4Eu, 11);
    RMD_R(F1, X4, 0x00000000u, 9);
    RMD_L(F5, X7, 0xA953FD4Eu, 6);
    RMD_R(F1, X1, 0x00000000u, 12);
    RMD_L(F5, X12, 0xA953FD4Eu, 8);
    RMD_R(F1, X5, 0x00000000u, 5);
    RMD_L(F5, X2, 0xA953FD4Eu, 13);
    RMD_R(F1, X8, 0x00000000u, 14);
    RMD_L(F5, X10, 0xA953FD4Eu, 12);
    RMD_R(F1, X7, 0x00000000u, 6);
    RMD_L(F5, X14, 0xA953FD4Eu, 5);
    RMD_R(F1, X6, 0x00000000u, 8);
    RMD_L(F5, X1, 0xA953FD4Eu, 12);
    RMD_R(F1, X2, 0x00000000u, 13);
    RMD_L(F5, X3, 0xA953FD4Eu, 13);
    RMD_R(F1, X13, 0x00000000u, 6);
    RMD_L(F5, X8, 0xA953FD4Eu, 14);
    RMD_R(F1, X14, 0x00000000u, 5);
    RMD_L(F5, X11, 0xA953FD4Eu, 11);
    RMD_R(F1, X0, 0x00000000u, 15);
    RMD_L(F5, X6, 0xA953FD4Eu, 8);
    RMD_R(F1, X3, 0x00000000u, 13);
    RMD_L(F5, X15, 0xA953FD4Eu, 5);
    RMD_R(F1, X9, 0x00000000u, 11);
    RMD_L(F5, X13, 0xA953FD4Eu, 6);
    RMD_R(F1, X11, 0x00000000u, 11);
    u32 h0 = 0xEFCDAB89u + CL + DR;
    u32 h1 = 0x98BADCFEu + DL + ER;
    *h0_out = h0;
    return ((u64)BSWAP(h0) << 32) | (u64)BSWAP(h1);
}

// ---------------------------------------------------------------- matching

static inline int in_ranges(u64 t, __constant const u64* ranges, u32 nranges) {
    if (t < ranges[0]) return 0;
    u32 lo = 0, hi = nranges - 1;
    while (lo < hi) {
        u32 mid = (lo + hi + 1) >> 1;
        if (ranges[2 * mid] <= t) lo = mid; else hi = mid - 1;
    }
    return t <= ranges[2 * lo + 1];
}

// Hashes the six public keys derived from point (x, y) and reports matches.
// flags: bits 0-1 = endomorphism power e, bit 2 = negated.
static inline void check_point(const u32* x, const u32* y, u32 jidx, u32 gid,
                        __constant const u64* ranges, u32 nranges,
                        __global u32* out, u32 max_out) {
    u32 parity = y[0] & 1u;
    u32 xe[8], xb[8];
    #pragma unroll 1
    for (u32 e = 0; e < 3; e++) {
        if (e == 0) {
            fe_copy(xe, x);
        } else if (e == 1) {
            u32 beta[8];
            #pragma unroll
            for (int l = 0; l < 8; l++) beta[l] = BETA[l];
            fe_mul(xb, x, beta);
            fe_copy(xe, xb);
        } else {
            // beta^2 * x = -(x + beta * x)
            u32 t[8];
            fe_add(t, x, xb);
            fe_neg(xe, t);
        }
        #pragma unroll 1
        for (u32 neg = 0; neg < 2; neg++) {
            u32 h0;
            u64 top = hash160_top64(2u + (parity ^ neg), xe, &h0);
            if (in_ranges(top, ranges, nranges)) {
                u32 slot = atomic_inc(&out[0]);
                if (slot < max_out) {
                    out[1 + slot * 4 + 0] = gid;
                    out[1 + slot * 4 + 1] = jidx;
                    out[1 + slot * 4 + 2] = e | (neg << 2);
                    out[1 + slot * 4 + 3] = h0;
                }
            }
        }
    }
}

// ------------------------------------------------------------------- kernel
//
// centers:  [16][N] limb-major (x limbs 0-7, y limbs 8-15)
// table:    [GROUP + 1][16]: entry i < GROUP is (i+1)*G, entry GROUP is (2*GROUP+1)*G
// scratch:  [GROUP + 1][8][N] prefix products for the batch inversion
// ranges:   [nranges][2] inclusive (lo, hi) bounds on HASH160's top 64 bits, sorted
// out:      [0] = hit count, then 4 words per hit

__kernel void btc_search(__global u32* restrict centers,
                         __constant const u32* table,
                         __global u32* restrict scratch,
                         __constant const u64* ranges,
                         const u32 nranges,
                         __global u32* restrict out,
                         const u32 max_out) {
    const u32 gid = get_global_id(0);
    const u32 n = get_global_size(0);

    u32 cx[8], cy[8];
    #pragma unroll
    for (int l = 0; l < 8; l++) {
        cx[l] = centers[l * n + gid];
        cy[l] = centers[(8 + l) * n + gid];
    }

    u32 acc[8], dx[8], tx[8], ty[8];
    #pragma unroll 1
    for (u32 i = 0; i <= GROUP; i++) {
        #pragma unroll
        for (int l = 0; l < 8; l++) tx[l] = table[i * 16 + l];
        fe_sub(dx, tx, cx);
        if (i == 0) fe_copy(acc, dx); else fe_mul(acc, acc, dx);
        #pragma unroll
        for (int l = 0; l < 8; l++) scratch[(i * 8 + l) * n + gid] = acc[l];
    }

    u32 inv[8];
    fe_inv(inv, acc);

    u32 nx[8], ny[8];
    #pragma unroll 1
    for (int i = GROUP; i >= 0; i--) {
        #pragma unroll
        for (int l = 0; l < 8; l++) {
            tx[l] = table[i * 16 + l];
            ty[l] = table[i * 16 + 8 + l];
        }
        u32 di[8];
        if (i > 0) {
            u32 prev[8];
            #pragma unroll
            for (int l = 0; l < 8; l++) prev[l] = scratch[((i - 1) * 8 + l) * n + gid];
            fe_mul(di, inv, prev);
            fe_sub(dx, tx, cx);
            fe_mul(inv, inv, dx);
        } else {
            fe_copy(di, inv);
        }

        #pragma unroll 1
        for (u32 s = 0; s < 2; s++) {
            u32 px[8], py[8], jidx;
            if (i == GROUP && s == 1) {
                // The centre itself.
                fe_copy(px, cx);
                fe_copy(py, cy);
                jidx = GROUP;
            } else {
                u32 by[8];
                if (s == 0) fe_copy(by, ty); else fe_neg(by, ty);
                point_add(px, py, cx, cy, tx, by, di);
                if (i == GROUP) {
                    fe_copy(nx, px);
                    fe_copy(ny, py);
                    continue;
                }
                jidx = (s == 0) ? (u32)(GROUP + i + 1) : (u32)(GROUP - i - 1);
            }
            check_point(px, py, jidx, gid, ranges, nranges, out, max_out);
        }
    }

    #pragma unroll
    for (int l = 0; l < 8; l++) {
        centers[l * n + gid] = nx[l];
        centers[(8 + l) * n + gid] = ny[l];
    }
}
