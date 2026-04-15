{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "ed25519_utils.wgsl" %}
{% include "constants.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read_write> pt_a: ETEPoint;
@group(0) @binding(1) var<storage, read_write> pt_b: ETEPoint;
@group(0) @binding(2) var<storage, read_write> x: BigInt;
@group(0) @binding(3) var<storage, read_write> y: BigInt;
@group(0) @binding(4) var<storage, read_write> result: ETEPoint;

@compute
@workgroup_size(1)
fn test_strauss_shamir_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var p_wide = get_p_wide();
    var mu_fp = get_mu_fp();

    var pt_a_point = pt_a;
    var pt_b_point = pt_b;

    var x_bigint = x;
    var y_bigint = y;

    result = ete_strauss_shamir_mul(&pt_a_point, &pt_b_point, &x_bigint, &y_bigint, &p);
}
