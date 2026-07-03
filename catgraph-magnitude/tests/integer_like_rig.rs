//! Parametric tests verifying the `IntegerLikeRig` widening.
//!
//! `magnitude_homology_rank` and `snf_rank_*` must produce identical results
//! over `F64Rig` and `Z(BigInt)`.
//!
//! Substrate verification for the rig widening:
//! the rank-recovery surface — previously locked to `F64Rig` via the
//! private `RankQ` alias — now accepts any `IntegerLikeRig`. Existing
//! `F64Rig` callers compile unchanged via the blanket impl; `Z(BigInt)`
//! becomes a first-class rank-recovery rig.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::F64Rig;
use catgraph_applied::z::Z;
use catgraph_magnitude::chain_complex::{ChainIndex, IntegerLikeRig, magnitude_homology_rank};

#[test]
fn magnitude_homology_rank_agrees_across_f64rig_and_z() {
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 1.0 });
    let idx = ChainIndex::new(&space, 3);
    for k in 0..=3 {
        for &ell in idx.grades() {
            let rank_f64 = magnitude_homology_rank::<F64Rig>(&idx, &space, k, ell).unwrap();
            let rank_z = magnitude_homology_rank::<Z>(&idx, &space, k, ell).unwrap();
            assert_eq!(rank_f64, rank_z, "(k={k}, ell={ell}): F64Rig vs Z disagree");
        }
    }
}

#[test]
fn integer_like_rig_to_i64_roundtrip_f64() {
    let x = F64Rig(42.0);
    assert_eq!(x.to_i64().unwrap(), 42);
}

#[test]
fn integer_like_rig_to_i64_roundtrip_z() {
    let x = Z::from(42_i64);
    assert_eq!(x.to_i64().unwrap(), 42);
}

#[test]
fn integer_like_rig_to_i64_overflow_z_errors() {
    use num::BigInt;
    let big = Z(BigInt::parse_bytes(b"99999999999999999999999", 10).unwrap());
    assert!(big.to_i64().is_err());
}
