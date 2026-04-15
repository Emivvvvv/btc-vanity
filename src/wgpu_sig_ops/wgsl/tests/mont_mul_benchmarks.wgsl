{% include "bigint.wgsl" %}
{% include "ff.wgsl" %}
{% include "mont.wgsl" %}
{% include "constants.wgsl" %}

@group(0) @binding(0) var<storage, read_write> ar: BigInt;
@group(0) @binding(1) var<storage, read_write> br: BigInt;
@group(0) @binding(2) var<storage, read_write> cr: BigInt;
@group(0) @binding(3) var<storage, read_write> cost: u32;

@compute
@workgroup_size(1)
fn benchmark_mont_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var p_bigint = get_p();
    var ar_bigint = ar;
    var br_bigint = br;
    var result: BigInt = ar_bigint;

    for (var i = 1u; i < cost; i ++) {
        result = mont_mul(&ar_bigint, &result, &p_bigint);
    }

    result = mont_mul(&result, &br_bigint, &p_bigint);

    cr = result;
}
