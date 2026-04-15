{% include "bigint.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read_write> input: BigInt;
@group(0) @binding(1) var<storage, read_write> result: array<u32, 8>;

@compute
@workgroup_size(1)
fn test_limbs_le_to_bytes_be(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var val = input;
    var limbs = input.limbs;
    var log_limb_size = {{ log_limb_size }}u;

    result = limbs_le_to_u32s_be(&limbs, log_limb_size);
}
