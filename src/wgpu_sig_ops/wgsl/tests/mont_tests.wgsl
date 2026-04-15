{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "constants.wgsl" %}

@group(0) @binding(0) var<storage, read_write> a: BigInt;
@group(0) @binding(1) var<storage, read_write> b: BigInt;
@group(0) @binding(2) var<storage, read_write> c: BigIntMediumWide;

@compute
@workgroup_size(1)
fn test_mont_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var a_bigint = a;
    var b_bigint = b;
    var result: BigInt = mont_mul(&a_bigint, &b_bigint, &p_bigint);

    for (var i = 0u; i < {{ num_limbs }}u; i ++) {
        c.limbs[i] = result.limbs[i];
    }
}
