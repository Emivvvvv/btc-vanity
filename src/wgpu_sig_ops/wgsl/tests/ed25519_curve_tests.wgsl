{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "ed25519_utils.wgsl" %}
{% include "constants.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read_write> a: ETEPoint;
@group(0) @binding(1) var<storage, read_write> b: ETEPoint;
@group(0) @binding(2) var<storage, read_write> result: ETEPoint;

@compute
@workgroup_size(1)
fn test_ete_add_2008_hwcd_3(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var a_pt = a;
    var b_pt = b;
    var result_pt = ete_add_2008_hwcd_3(&a_pt, &b_pt, &p);
    result = result_pt;
}

@compute
@workgroup_size(1)
fn test_ete_dbl_2008_hwcd(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var a_pt = a;
    var b_pt = b;
    var result_pt = ete_dbl_2008_hwcd(&a_pt, &p);
    result = result_pt;
}

@compute
@workgroup_size(1)
fn test_ete_to_affine(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var r = get_r();
    var rinv = get_rinv();
    var p_wide = get_p_wide();
    var mu_fp = get_mu_fp();

    var a_pt = a;
    var b_pt = b;
    var result_pt = ete_to_affine_non_mont(&a_pt, &p, &p_wide, &r, &rinv, &mu_fp);

    var x = result_pt.x;    
    var y = result_pt.y;    
    var t = ff_mul(&x, &y, &p, &p_wide, &mu_fp);
    var z: BigInt;
    z.limbs[0] = 1u;

    result = ETEPoint(x, y, t, z);
}

@compute
@workgroup_size(1)
fn test_ete_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var scalar_p = get_scalar_p();
    var a_pt = a;
    var b_pt = b;
    var one: BigInt; one.limbs[0] = 1u;
    var s = bigint_sub(&scalar_p, &one);

    var result_pt = ete_mul(&a_pt, &s, &p);
    result = result_pt;
}
