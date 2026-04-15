struct IntermediateResult {
    is_valid_y_coord: bool,
    neg_a_pt: ETEPoint,
}

fn compute_neg_a_pt(
    s: ptr<function, BigInt>,
    k: ptr<function, BigInt>,
    ayr: ptr<function, BigInt>,
    x_sign: bool,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
) -> IntermediateResult {
    var r = reconstruct_ete_from_y(ayr, x_sign, p);

    var is_valid_y_coord = r.is_valid_y_coord;

    if (!is_valid_y_coord) {
        var empty: ETEPoint;
        return IntermediateResult(false, empty);
    }

    var a_pt = r.pt;
    var a_pt_x = a_pt.x;
    var a_pt_t = a_pt.t;
    var neg_a_pt_x = ff_negate(&a_pt_x, p);
    var neg_a_pt_t = ff_negate(&a_pt_t, p);

    return IntermediateResult(is_valid_y_coord, ETEPoint(neg_a_pt_x, a_pt.y, neg_a_pt_t, a_pt.z));
}

fn ed25519_verify(
    s: ptr<function, BigInt>,
    k: ptr<function, BigInt>,
    ayr: ptr<function, BigInt>,
    x_sign: bool,
    p: ptr<function, BigInt>,
    p_wide: ptr<function, BigIntWide>,
    rinv: ptr<function, BigInt>,
    mu_fp: ptr<function, BigInt>,
) -> ETEPoint {
    var res = compute_neg_a_pt(s, k, ayr, x_sign, p, p_wide, rinv, mu_fp);
    var neg_a_pt = res.neg_a_pt;

    if (!res.is_valid_y_coord) {
        var empty: ETEPoint;
        return empty;
    }

    // Get the ed25519 curve generator
    var g = get_ed25519_generator();

    /*return ete_strauss_shamir_mul(&g, &neg_a_pt, s, k, p);*/

    
    // This is about 2x slower than ete_strauss_shamir_mul:
    var gs = ete_mul(&g, s, p);
    var neg_a_pt_k = ete_mul(&neg_a_pt, k, p);
    return ete_add_2008_hwcd_3(&gs, &neg_a_pt_k, p);
    
}
