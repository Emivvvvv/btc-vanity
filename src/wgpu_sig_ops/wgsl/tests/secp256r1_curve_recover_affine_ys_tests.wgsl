{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "secp256r1_curve.wgsl" %}
{% include "constants.wgsl" %}
{% include "secp_constants.wgsl" %}

@group(0) @binding(0) var<storage, read_write> xr: BigInt;
@group(0) @binding(1) var<storage, read_write> yr_0: BigInt;
@group(0) @binding(2) var<storage, read_write> yr_1: BigInt;

@compute
@workgroup_size(1)
fn test_secp256r1_recover_affine_ys(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var xr_bigint = xr;

    var result = secp256r1_recover_affine_ys(&xr_bigint, &p_bigint);
    yr_0 = result[0];
    yr_1 = result[1];
}
