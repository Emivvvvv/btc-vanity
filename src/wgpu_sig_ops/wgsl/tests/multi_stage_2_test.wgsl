@group(0) @binding(0) var<storage, read_write> a: u32;
@group(0) @binding(1) var<storage, read_write> b: u32;
@group(0) @binding(2) var<storage, read_write> result: u32;

@compute
@workgroup_size(1)
fn stage_2(@builtin(global_invocation_id) global_id: vec3<u32>) {
    result = a + b;
}

