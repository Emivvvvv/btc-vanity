{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "secp256k1_curve.wgsl" %}
{% include "constants.wgsl" %}
{% include "secp_constants.wgsl" %}

@group(0) @binding(0) var<storage, read_write> pt: Point;
@group(0) @binding(1) var<storage, read_write> xr: BigInt;
@group(0) @binding(2) var<storage, read_write> result: Point;

@compute
@workgroup_size(1)
fn test_scalar_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var p_wide = get_p_wide();
    var mu_fp = get_mu_fp();

    var pt_point = pt;
    var xr_bigint = xr;

    var rinv = get_rinv();

    var x = ff_mul(&xr_bigint, &rinv, &p, &p_wide, &mu_fp);

    result = jacobian_mul(&pt_point, &x, &p);
}
