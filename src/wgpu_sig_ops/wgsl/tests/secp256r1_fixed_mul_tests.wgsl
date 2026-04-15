{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "secp256r1_curve.wgsl" %}
{% include "constants.wgsl" %}
{% include "secp_constants.wgsl" %}
{% include "secp_curve_utils.wgsl" %}

{% include "secp256r1_curve_generators.wgsl" %}

@group(0) @binding(0) var<storage, read_write> table: array<PointAffine>;
@group(0) @binding(1) var<storage, read_write> s: BigInt;
@group(0) @binding(2) var<storage, read_write> result: Point;

@compute
@workgroup_size(1)
fn test_projective_fixed_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var r = get_r();

    var table_size = {{ table_size }}u;
    var table_pts: array<PointAffine, {{ table_size }}>;
    for (var i = 0u; i < table_size; i ++) {
        table_pts[i] = table[i];
    }
    var s_bigint = s;

    result = projective_fixed_mul(&table_pts, &s_bigint, &p, &r);
}
