{% include "bigint.wgsl" %}
{% include "bytes_be_to_limbs_le.wgsl" %}

@group(0) @binding(0) var<storage, read_write> input: array<u32, 16>;
@group(0) @binding(1) var<storage, read_write> result: BigInt;

@compute
@workgroup_size(1)
fn test_bytes_be_to_limbs_le(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var input_u32s: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        input_u32s[i] = input[i];
    }

    var bytes_be = u32s_to_bytes_be(&input_u32s);

    result = bytes_be_to_limbs_le(&bytes_be);
}
