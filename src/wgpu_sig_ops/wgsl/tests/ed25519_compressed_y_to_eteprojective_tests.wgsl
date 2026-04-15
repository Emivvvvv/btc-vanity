{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "constants.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "ed25519_utils.wgsl" %}
{% include "bytes_be_to_limbs_le.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read_write> compressed_y: array<u32, 16>;
@group(0) @binding(1) var<storage, read_write> result: ETEPoint;
@group(0) @binding(2) var<storage, read_write> is_valid: u32;

@compute
@workgroup_size(1)
fn test_compressed_y_to_eteprojective(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var y_u32s: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        y_u32s[i] = compressed_y[i];
    }
    var y_bytes_be = u32s_to_bytes_be(&y_u32s);

    var compressed_sign_bit = y_bytes_be[31] >> 7u;

    y_bytes_be[31] &= 0x7fu;

    var y_bytes_le: array<u32, 32>;
    for (var i = 0u; i < 32u; i ++) {
        y_bytes_le[i] = y_bytes_be[31u - i];
    }
    var y_val = bytes_be_to_limbs_le(&y_bytes_le);

    var p = get_p();

    // Reduce y_val
    if (bigint_gte(&y_val, &p)) {
        y_val = bigint_sub(&y_val, &p);
    }

    // Convert y_val to Montgomery form
    var r = get_r();
    var p_wide = get_p_wide();
    var mu_fp = get_mu_fp();
    var yr_val = ff_mul(&y_val, &r, &p, &p_wide, &mu_fp);

    var reconstructed = reconstruct_ete_from_y(&yr_val, compressed_sign_bit == 1u, &p);

    result = reconstructed.pt;

    if (reconstructed.is_valid_y_coord) {
        is_valid = 1u;
    }
}
