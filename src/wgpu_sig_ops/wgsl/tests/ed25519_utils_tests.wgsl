{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "ed25519_curve.wgsl" %}
{% include "constants.wgsl" %}
{% include "ed25519_constants.wgsl" %}
{% include "ed25519_utils.wgsl" %}
{% include "limbs_le_to_u32s_be.wgsl" %}

@group(0) @binding(0) var<storage, read_write> a: BigInt;
@group(0) @binding(1) var<storage, read_write> b: BigInt;
@group(0) @binding(2) var<storage, read_write> result: BigInt;
@group(0) @binding(3) var<storage, read_write> result2: BigInt;

@compute
@workgroup_size(1)
fn test_is_negative(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var a_val = a;
    var b_val = b;
    var r2 = result2;

    var one: BigInt;
    one.limbs[0] = 1u;

    if (is_negative(&a_val)) {
        result = one;
    }
}

@compute
@workgroup_size(1)
fn test_conditional_assign_true(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var a_val = a;
    var b_val = b;
    var r2 = result2;
    var choice = true;

    result = conditional_assign(&a_val, &b_val, choice);
}

@compute
@workgroup_size(1)
fn test_conditional_assign_false(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var a_val = a;
    var b_val = b;
    var r2 = result2;
    var choice = false;

    result = conditional_assign(&a_val, &b_val, choice);
}

@compute
@workgroup_size(1)
fn test_conditional_negate_true(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var a_val = a;
    var b_val = b;
    var r2 = result2;
    var p = get_p();
    var choice = true;

    result = conditional_negate(&a_val, &p, choice);
}

@compute
@workgroup_size(1)
fn test_conditional_negate_false(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var a_val = a;
    var b_val = b;
    var r2 = result2;
    var p = get_p();
    var choice = false;

    result = conditional_negate(&a_val, &p, choice);
}

@compute
@workgroup_size(1)
fn test_pow_p58(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var ar_val = a;
    var b_val = b;
    var r2 = result2;
    var p = get_p();

    result = mont_pow_p58(&ar_val, &p);
}

@compute
@workgroup_size(1)
fn test_sqrt_ratio_i(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var ur_val = a;
    var vr_val = b;
    var r2 = result2;
    var p = get_p();

    var r = sqrt_ratio_i(&ur_val, &vr_val, &p);

    result = r.r;

    if (r.was_nonzero_square) {
        var one: BigInt;
        one.limbs[0] = 1u;
        result2 = one;
    }
}
