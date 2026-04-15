struct DecodedSig {
    sig: array<u32, 32>,
    is_y_odd: bool
}

fn decode_signature(
    sig_s: ptr<function, array<u32, 32>>
) -> DecodedSig {
    var s: array<u32, 32>;
    for (var i = 0u; i < 32; i ++) {
        s[i] = (*sig_s)[i];
    }
    
    var is_y_odd = (s[0] & 0x80u) != 0;
    s[0] &= 0x7fu;

    /*let is_y_odd = (sig[32] & 0x80) != 0;*/
    /*sig.as_mut()[32] &= 0x7f;*/

    return DecodedSig(s, is_y_odd);
}
