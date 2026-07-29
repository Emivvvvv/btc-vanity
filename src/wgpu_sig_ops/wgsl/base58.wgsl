const B58_ALPHABET: array<u32, 58> = array<u32, 58>(
    49u, 50u, 51u, 52u, 53u, 54u, 55u, 56u, 57u,
    65u, 66u, 67u, 68u, 69u, 70u, 71u, 72u, 74u, 75u,
    76u, 77u, 78u, 80u, 81u, 82u, 83u, 84u, 85u, 86u,
    87u, 88u, 89u, 90u,
    97u, 98u, 99u, 100u, 101u, 102u, 103u, 104u, 105u,
    106u, 107u, 109u, 110u, 111u, 112u, 113u, 114u, 115u,
    116u, 117u, 118u, 119u, 120u, 121u, 122u,
);

fn base58_encode_var(input: ptr<function, array<u32, 64>>, input_len: u32) -> AddressBuf {
    var out: AddressBuf;
    var alphabet = B58_ALPHABET;
    var digits: array<u32, 64> = array<u32, 64>();
    var data: array<u32, 64> = array<u32, 64>();

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
        out.data[out_len] = 49u; // '1'
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
