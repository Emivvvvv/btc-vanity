{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "secp256r1_curve.wgsl" %}
{% include "constants.wgsl" %}
{% include "secp_constants.wgsl" %}
{% include "secp_curve_utils.wgsl" %}

@group(0) @binding(0) var<storage, read_write> a: Point;
@group(0) @binding(1) var<storage, read_write> b: Point;
@group(0) @binding(2) var<storage, read_write> result: Point;

@compute
@workgroup_size(1)
fn test_projective_add_2015_rcb_unsafe(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_pt = a;
    var b_pt = b;
    var result_pt = projective_add_2015_rcb_unsafe(&a_pt, &b_pt, &p_bigint);
    result = result_pt;
}

@compute
@workgroup_size(1)
fn test_projective_dbl_2015_rcb(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_pt = a;
    var b_pt = b;
    var result_pt = projective_dbl_2015_rcb(&a_pt, &p_bigint);
    result = result_pt;
}

@compute
@workgroup_size(1)
fn test_projective_to_affine(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var p_wide = get_p_wide();
    var r = get_r();
    var rinv = get_rinv();
    var mu_fp = get_mu_fp();

    var a_pt = a;
    var b_pt = b;
    var result_pt = projective_to_affine_non_mont(&a_pt, &p_bigint, &p_wide, &r, &rinv, &mu_fp);
    result = result_pt;
}

@compute
@workgroup_size(1)
fn test_projective_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var scalar_p = get_scalar_p();
    var a_pt = a;
    var b_pt = b;
    var one: BigInt; one.limbs[0] = 1u;
    var s = bigint_sub(&scalar_p, &one);

    var result_pt = projective_mul(&a_pt, &s, &p);
    result = result_pt;
}
