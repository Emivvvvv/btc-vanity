@group(0) @binding(0) var<storage, read_write> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> result: u32;

@compute
@workgroup_size(1)
fn test_buffer(@builtin(global_invocation_id) global_id: vec3<u32>) {
    result = input[0];
}
