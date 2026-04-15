{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}
{% include "constants.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "ed25519_utils.wgsl" %}

@group(0) @binding(0) var<storage, read_write> yr: BigInt;
@group(0) @binding(1) var<storage, read_write> x_sign: u32;
@group(0) @binding(2) var<storage, read_write> result: ETEPoint;
@group(0) @binding(3) var<storage, read_write> is_valid: u32;

@compute
@workgroup_size(1)
fn test_reconstruct_ete_from_y(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var yr_val = yr;
    var p = get_p();

    var r = reconstruct_ete_from_y(&yr_val, x_sign == 1u, &p);

    result = r.pt;
    if (r.is_valid_y_coord) {
        is_valid = 1u;
    }
}
