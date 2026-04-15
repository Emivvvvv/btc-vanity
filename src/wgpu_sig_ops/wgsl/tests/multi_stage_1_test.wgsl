@group(0) @binding(0) var<storage, read_write> input: u32;
@group(0) @binding(1) var<storage, read_write> result: u32;

@compute
@workgroup_size(1)
fn stage_1(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var val = input;
    val += 1u;
    result = val;
}
