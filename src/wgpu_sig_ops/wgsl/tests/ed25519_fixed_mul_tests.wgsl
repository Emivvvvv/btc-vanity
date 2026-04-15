{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "ed25519_utils.wgsl" %}
{% include "constants.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read_write> table: array<ETEXYT>;
@group(0) @binding(1) var<storage, read_write> s: BigInt;
@group(0) @binding(2) var<storage, read_write> result: ETEPoint;

@compute
@workgroup_size(1)
fn test_ete_fixed_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p = get_p();
    var r = get_r();

    var table_size = {{ table_size }}u;
    var table_pts: array<ETEXYT, {{ table_size }}>;
    for (var i = 0u; i < table_size; i ++) {
        table_pts[i] = table[i];
    }
    var s_bigint = s;

    result = ete_fixed_mul(&table_pts, &s_bigint, &p, &r);
}
