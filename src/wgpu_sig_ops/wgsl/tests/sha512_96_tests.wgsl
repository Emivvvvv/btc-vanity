{% include "sha512.wgsl" %}

@group(0) @binding(0) var<storage, read_write> input: array<u32, 24>;
@group(0) @binding(1) var<storage, read_write> result: array<u32, 16>;

@compute
@workgroup_size(1)
fn test_sha512_96(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var input_u32s: array<u32, 24>;

    for (var i = 0u; i < 24u; i ++) {
        input_u32s[i] = input[i];
    }

    result = sha512_96(&input_u32s);
}
