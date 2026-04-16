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
    var state: array<vec2<u32>, 25> = array<vec2<u32>, 25>();

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

    var out: array<u32, 32> = array<u32, 32>();
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
