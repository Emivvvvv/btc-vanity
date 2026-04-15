{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "constants.wgsl" %}

@group(0) @binding(0) var<storage, read_write> a: BigInt;
@group(0) @binding(1) var<storage, read_write> b: BigInt;
@group(0) @binding(2) var<storage, read_write> c: BigIntMediumWide;

@compute
@workgroup_size(1)
fn test_bigint_wide_add(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;
    var result: BigIntMediumWide = bigint_wide_add(&a_bigint, &b_bigint);
    c = result;
}

@compute
@workgroup_size(1)
fn test_bigint_add_unsafe(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;
    var result: BigInt = bigint_add_unsafe(&a_bigint, &b_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_bigint_sub(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;
    var result: BigInt = bigint_sub(&a_bigint, &b_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_bigint_wide_sub(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint: BigIntMediumWide;
    var b_bigint: BigIntMediumWide;
    for (var i = 0u; i < {{ num_limbs + 1 }}u; i ++) {
        a_bigint.limbs[i] = a.limbs[i];
        b_bigint.limbs[i] = b.limbs[i];
    }
    var result: BigIntMediumWide = bigint_wide_sub(&a_bigint, &b_bigint);

    c = result;
}

@compute
@workgroup_size(1)
fn test_bigint_gte(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;

    if (bigint_gte(&a_bigint, &b_bigint) == true) {
        var result: BigInt;
        result.limbs[0] = 1u;
        for (var i = 0u; i < {{ num_limbs }}u; i ++) {
            c.limbs[i] = result.limbs[i];
        }
    }
}

@compute
@workgroup_size(1)
fn test_bigint_wide_gte(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint: BigIntMediumWide;
    var b_bigint: BigIntMediumWide;
    for (var i = 0u; i < {{ num_limbs + 1 }}u; i ++) {
        a_bigint.limbs[i] = a.limbs[i];
        b_bigint.limbs[i] = b.limbs[i];
    }

    var result: BigIntMediumWide;
    if (bigint_wide_gte(&a_bigint, &b_bigint)) {
        c.limbs[0] = 1u;
    }
}

@compute
@workgroup_size(1)
fn test_ff_add(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;
    var result: BigInt = ff_add(&a_bigint, &b_bigint, &p_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_ff_sub(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;
    var result: BigInt = ff_sub(&a_bigint, &b_bigint, &p_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_ff_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;

    var p_wide = get_p_wide();
    var mu_fp = get_mu_fp();

    var result: BigInt = ff_mul(&a_bigint, &b_bigint, &p_bigint, &p_wide, &mu_fp);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_ff_inverse(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b; // Must be present or else wgpu cannot autogenerate the bind group layout 
    var result: BigInt = ff_inverse(&a_bigint, &p_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_bigint_div_2(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b; // Must be present or else wgpu cannot autogenerate the bind group layout 
    var result: BigInt = bigint_div2(&a_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}

@compute
@workgroup_size(1)
fn test_bigint_shr_384(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var p_wide = get_p_wide();
    var mu_fp = get_mu_fp();

    var a_bigint = a;
    var b_bigint = b;

    var ab: BigIntWide = bigint_mul(&a_bigint, &b_bigint);

    var result: BigInt = bigint_shr_384(&ab);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}
