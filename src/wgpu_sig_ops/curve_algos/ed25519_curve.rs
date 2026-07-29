use crate::wgpu_sig_ops::curve_algos::coords::ETEProjective;
use ark_ed25519::{EdwardsAffine, Fq};
use ark_ff::One;

pub fn affine_to_projective(point: &EdwardsAffine) -> ETEProjective<Fq> {
    ETEProjective {
        x: point.x,
        y: point.y,
        t: point.x * point.y,
        z: Fq::one(),
    }
}
