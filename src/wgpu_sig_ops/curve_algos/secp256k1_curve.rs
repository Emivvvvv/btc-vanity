use crate::wgpu_sig_ops::curve_algos::coords::ProjectiveXYZ;
use ark_ec::AffineRepr;
use ark_ff::{One, Zero};
use ark_secp256k1::{Affine, Fq};

pub fn affine_to_projectivexyz(point: &Affine) -> ProjectiveXYZ<Fq> {
    if point.is_zero() {
        return ProjectiveXYZ {
            x: Fq::zero(),
            y: Fq::one(),
            z: Fq::zero(),
        };
    }
    ProjectiveXYZ {
        x: point.x,
        y: point.y,
        z: Fq::one(),
    }
}

