//! Unit tests for the Z(BigInt) integer-exact ring newtype.
//!
//! Substrate for catgraph-magnitude §1.17 Leinster 2008 Cor 1.5
//! integer-exact Möbius inversion. Verifies the ring-axiom obligations
//! and the `Rig` + `ZAlgebra` trait bounds inherited via blanket-impl.
//! (`ZAlgebra` was renamed from `Integer` and sealed.)

use catgraph_applied::ZAlgebra;
use catgraph_applied::rig::Rig;
use catgraph_applied::z::Z;
use deep_causality_num::{One, Zero};
use std::collections::HashSet;

#[test]
fn z_zero_one_distinct() {
    assert_ne!(Z::zero(), Z::one());
}

#[test]
fn z_from_i64_positive() {
    let a = Z::from_i64(42);
    assert_eq!(a, Z::from(42_i64));
}

#[test]
fn z_from_i64_negative() {
    let a = Z::from_i64(-42);
    assert_eq!(a, -Z::from_i64(42));
}

#[test]
fn z_additive_associativity() {
    let a = Z::from_i64(3);
    let b = Z::from_i64(5);
    let c = Z::from_i64(7);
    assert_eq!((a.clone() + b.clone()) + c.clone(), a + (b + c));
}

#[test]
fn z_multiplicative_associativity() {
    let a = Z::from_i64(3);
    let b = Z::from_i64(5);
    let c = Z::from_i64(7);
    assert_eq!((a.clone() * b.clone()) * c.clone(), a * (b * c));
}

#[test]
fn z_distributivity() {
    let a = Z::from_i64(3);
    let b = Z::from_i64(5);
    let c = Z::from_i64(7);
    assert_eq!(a.clone() * (b.clone() + c.clone()), a.clone() * b + a * c);
}

#[test]
fn z_neg_double_neg() {
    let a = Z::from_i64(42);
    assert_eq!(-(-a.clone()), a);
}

#[test]
fn z_sub_via_neg_add() {
    let a = Z::from_i64(10);
    let b = Z::from_i64(3);
    assert_eq!(a.clone() - b.clone(), a + (-b));
}

#[test]
fn z_hashes_consistently() {
    let mut set: HashSet<Z> = HashSet::new();
    set.insert(Z::from_i64(7));
    assert!(set.contains(&Z::from_i64(7)));
    assert!(!set.contains(&Z::from_i64(8)));
}

#[test]
fn z_satisfies_ring_bound() {
    fn assert_rig<R: Rig>(_x: R) {}
    assert_rig(Z::from_i64(42));
}

#[test]
fn z_satisfies_zalgebra_bound() {
    fn assert_zalgebra<I: ZAlgebra>(_x: I) {}
    assert_zalgebra(Z::from_i64(42));
}
