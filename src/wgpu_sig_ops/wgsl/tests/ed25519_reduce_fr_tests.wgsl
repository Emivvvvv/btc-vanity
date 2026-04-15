{% include "ed25519_reduce_fr.wgsl" %}

@group(0) @binding(0) var<storage, read_write> input: array<u32, 16>;
@group(0) @binding(1) var<storage, read_write> result: array<u32, 32>;
@group(0) @binding(2) var<storage, read_write> result_wide: array<u32, 64>;

@compute
@workgroup_size(1)
fn test_ed25519_reduce_fr(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var x: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        x[i] = input[i];
    }

    var reduced = ed25519_reduce_fr(&x);
    result = reduced;
    result_wide[0] = 0u;
}

@compute
@workgroup_size(1)
fn test_convert_512_be_to_le(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var x: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        x[i] = input[i];
    }

    result = convert_512_be_to_le(&x);
    result_wide[0] = 0u;
}

@compute
@workgroup_size(1)
fn test_mul_wide(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var x: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        x[i] = input[i];
    }

    {{ fr_reduce_r_limbs_array }}

    var x_limbs = convert_512_be_to_le(&x);
    var xr_limbs = mul(&x_limbs, &fr_reduce_r_limbs);
    result_wide = xr_limbs;
    result[0] = 0u;
}

@compute
@workgroup_size(1)
fn test_shr_512(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var x: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        x[i] = input[i];
    }

    {{ fr_reduce_r_limbs_array }}

    var x_limbs = convert_512_be_to_le(&x);
    var xr_limbs = mul(&x_limbs, &fr_reduce_r_limbs);
    result = shr_512(&xr_limbs);
    result_wide[0] = 0u;
}

@compute
@workgroup_size(1)
fn test_sub_wide(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var x: array<u32, 16>;
    for (var i = 0u; i < 16u; i ++) {
        x[i] = input[i];
    }

    {{ fr_reduce_r_limbs_array }}

    var x_limbs = convert_512_be_to_le(&x);
    result = sub(&x_limbs, &fr_reduce_r_limbs);
    result_wide[0] = 0u;
}
