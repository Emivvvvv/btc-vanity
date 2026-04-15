/// A custom SHA512 hash function that accepts a 96-byte input.
/// Based on https://gist.github.com/illia-v/7883be942da5d416521375004cecb68f

fn initial_hash() -> array<vec2<u32>, 8> {
    return array<vec2<u32>, 8>(
        vec2(0x6a09e667u, 0xf3bcc908u), vec2(0xbb67ae85u, 0x84caa73bu),
        vec2(0x3c6ef372u, 0xfe94f82bu), vec2(0xa54ff53au, 0x5f1d36f1u),
        vec2(0x510e527fu, 0xade682d1u), vec2(0x9b05688cu, 0x2b3e6c1fu),
        vec2(0x1f83d9abu, 0xfb41bd6bu), vec2(0x5be0cd19u, 0x137e2179u)
    );
}

fn round_constants() -> array<vec2<u32>, 80> {
    return array<vec2<u32>, 80>(
        vec2(0x428a2f98u, 0xd728ae22), vec2(0x71374491u, 0x23ef65cd),
        vec2(0xb5c0fbcfu, 0xec4d3b2f), vec2(0xe9b5dba5u, 0x8189dbbc),
        vec2(0x3956c25bu, 0xf348b538), vec2(0x59f111f1u, 0xb605d019),
        vec2(0x923f82a4u, 0xaf194f9b), vec2(0xab1c5ed5u, 0xda6d8118),
        vec2(0xd807aa98u, 0xa3030242), vec2(0x12835b01u, 0x45706fbe),
        vec2(0x243185beu, 0x4ee4b28c), vec2(0x550c7dc3u, 0xd5ffb4e2),
        vec2(0x72be5d74u, 0xf27b896f), vec2(0x80deb1feu, 0x3b1696b1),
        vec2(0x9bdc06a7u, 0x25c71235), vec2(0xc19bf174u, 0xcf692694),
        vec2(0xe49b69c1u, 0x9ef14ad2), vec2(0xefbe4786u, 0x384f25e3),
        vec2(0x0fc19dc6u, 0x8b8cd5b5), vec2(0x240ca1ccu, 0x77ac9c65),
        vec2(0x2de92c6fu, 0x592b0275), vec2(0x4a7484aau, 0x6ea6e483),
        vec2(0x5cb0a9dcu, 0xbd41fbd4), vec2(0x76f988dau, 0x831153b5),
        vec2(0x983e5152u, 0xee66dfab), vec2(0xa831c66du, 0x2db43210),
        vec2(0xb00327c8u, 0x98fb213f), vec2(0xbf597fc7u, 0xbeef0ee4),
        vec2(0xc6e00bf3u, 0x3da88fc2), vec2(0xd5a79147u, 0x930aa725),
        vec2(0x06ca6351u, 0xe003826f), vec2(0x14292967u, 0x0a0e6e70),
        vec2(0x27b70a85u, 0x46d22ffc), vec2(0x2e1b2138u, 0x5c26c926),
        vec2(0x4d2c6dfcu, 0x5ac42aed), vec2(0x53380d13u, 0x9d95b3df),
        vec2(0x650a7354u, 0x8baf63de), vec2(0x766a0abbu, 0x3c77b2a8),
        vec2(0x81c2c92eu, 0x47edaee6), vec2(0x92722c85u, 0x1482353b),
        vec2(0xa2bfe8a1u, 0x4cf10364), vec2(0xa81a664bu, 0xbc423001),
        vec2(0xc24b8b70u, 0xd0f89791), vec2(0xc76c51a3u, 0x0654be30),
        vec2(0xd192e819u, 0xd6ef5218), vec2(0xd6990624u, 0x5565a910),
        vec2(0xf40e3585u, 0x5771202a), vec2(0x106aa070u, 0x32bbd1b8),
        vec2(0x19a4c116u, 0xb8d2d0c8), vec2(0x1e376c08u, 0x5141ab53),
        vec2(0x2748774cu, 0xdf8eeb99), vec2(0x34b0bcb5u, 0xe19b48a8),
        vec2(0x391c0cb3u, 0xc5c95a63), vec2(0x4ed8aa4au, 0xe3418acb),
        vec2(0x5b9cca4fu, 0x7763e373), vec2(0x682e6ff3u, 0xd6b2b8a3),
        vec2(0x748f82eeu, 0x5defb2fc), vec2(0x78a5636fu, 0x43172f60),
        vec2(0x84c87814u, 0xa1f0ab72), vec2(0x8cc70208u, 0x1a6439ec),
        vec2(0x90befffau, 0x23631e28), vec2(0xa4506cebu, 0xde82bde9),
        vec2(0xbef9a3f7u, 0xb2c67915), vec2(0xc67178f2u, 0xe372532b),
        vec2(0xca273eceu, 0xea26619c), vec2(0xd186b8c7u, 0x21c0c207),
        vec2(0xeada7dd6u, 0xcde0eb1e), vec2(0xf57d4f7fu, 0xee6ed178),
        vec2(0x06f067aau, 0x72176fba), vec2(0x0a637dc5u, 0xa2c898a6),
        vec2(0x113f9804u, 0xbef90dae), vec2(0x1b710b35u, 0x131c471b),
        vec2(0x28db77f5u, 0x23047d84), vec2(0x32caab7bu, 0x40c72493),
        vec2(0x3c9ebe0au, 0x15c9bebc), vec2(0x431d67c4u, 0x9c100d4c),
        vec2(0x4cc5d4beu, 0xcb3e42b6), vec2(0x597f299cu, 0xfc657e2a),
        vec2(0x5fcb6fabu, 0x3ad6faec), vec2(0x6c44198cu, 0x4a475817),
    );
}

fn not(a: vec2<u32>) -> vec2<u32> {
    return vec2(~a[0], ~a[1]);
}

fn xor(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2(a[0] ^ b[0], a[1] ^ b[1]);
}

fn and(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    return vec2(a[0] & b[0], a[1] & b[1]);
}

fn shr(a: vec2<u32>, num_bits: u32) -> vec2<u32> {
    var n0 = a[0];
    var n1 = a[1];
    if (num_bits == 0u) {
        return a;
    } 
    var new_high = n0 >> num_bits;
    var carry_bits = (n0 & 0x7fu) << (32u - num_bits);
    var new_low = (n1 >> num_bits) | carry_bits;

    return vec2(new_high, new_low);
}

fn add(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
    var sum1: u32 = a[1] + b[1];
    var c = 0u;
    if (sum1 < a[1]) {
        c = 1u;
    }
    var sum0 = a[0] + b[0] + c;

    return vec2(sum0, sum1);
}

fn right_rotate(n: vec2<u32>, b: u32) -> vec2<u32> {
    // return (n >> bits) | (n << (64 - bits)) & 0xFFFFFFFFFFFFFFFF
    // Ensure bits is within 0 to 63
    var bits = b % 64u;

    // Split the rotation into two parts
    if (bits < 32u) {
        // Rotate within and across the 32-bit boundaries
        var high = (n[0] >> bits) | (n[1] << (32u - bits));
        var low = (n[1] >> bits) | (n[0] << (32u - bits));
        return vec2(high, low);
    } else {
        // When rotating by 32 or more bits, effectively swap high and low parts
        var bits = bits - 32u;
        var high = (n[1] >> bits) | (n[0] << (32u - bits));
        var low = (n[0] >> bits) | (n[1] << (32u - bits));
        return vec2(high, low);
    }
}

fn sha512_96(
    input_bytes: ptr<function, array<u32, 24>>
) -> array<u32, 16> {
    var message_array = array<u32, 32>();

    for (var i = 0u; i < 24u; i ++) {
        message_array[i] = (*input_bytes)[i];
    }
    message_array[24] = 2147483648u;
    message_array[31] = 768u;

    var rc = round_constants();

    var sha512_hash = initial_hash();

    var w = array<u32, 160>();

    for (var i = 0u; i < 32u; i ++) {
        w[i] = message_array[i];
    }

    for (var i = 16u; i < 80u; i ++) {
        let i2 = i * 2u;
        var wim15 = vec2(w[i2 - 30u], w[i2 - 29u]);
        var wim2 = vec2(w[i2 - 4u], w[i2 - 3u]);
        var wim16 = vec2(w[i2 - 32u], w[i2 - 31u]);
        var wim7 = vec2(w[i2 - 14u], w[i2 - 13u]);

        let s0 = xor(xor(right_rotate(wim15, 1u), right_rotate(wim15, 8u)), shr(wim15, 7u));
        let s1 = xor(xor(right_rotate(wim2, 19u), right_rotate(wim2, 61u)), shr(wim2, 6u));
        
        let rr = add(add(add(wim16, s0), wim7), s1);
        w[i * 2] = rr[0];
        w[i * 2 + 1] = rr[1];
    }

    var a = sha512_hash[0];
    var b = sha512_hash[1];
    var c = sha512_hash[2];
    var d = sha512_hash[3];
    var e = sha512_hash[4];
    var f = sha512_hash[5];
    var g = sha512_hash[6];
    var h = sha512_hash[7];

    for (var i = 0u; i < 80u; i ++) {
        var sum1 = xor(xor(right_rotate(e, 14u), right_rotate(e, 18u)), right_rotate(e, 41u));
        var ch = xor(
            and(e, f),
            and(not(e), g)
        );

        var i2 = i * 2u;
        var temp1 = add(add(add(add(h, sum1), ch), rc[i]), vec2(w[i2], w[i2 + 1u]));
        var sum0 = xor(xor(right_rotate(a, 28u), right_rotate(a, 34u)), right_rotate(a, 39u));
        var maj = xor(xor(and(a, b), and(a, c)), and(b, c));
        var temp2 = add(sum0, maj);
        h = g;
        g = f;
        f = e;
        e = add(d, temp1);
        d = c;
        c = b;
        b = a;
        a = add(temp1, temp2);
    }

    var lhs = array<vec2<u32>, 8>(a, b, c, d, e, f, g, h);
    for (var i = 0u; i < 8u; i ++) {
        sha512_hash[i] = add(sha512_hash[i], lhs[i]);
    }

    var result = array<u32, 16>();

    for (var i = 0u; i < 8u; i ++) {
        result[(i * 2u)] = sha512_hash[i][0];
        result[(i * 2u) + 1u] = sha512_hash[i][1];
    }

    return result;
}
