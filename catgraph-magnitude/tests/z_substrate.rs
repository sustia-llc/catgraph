//! Cross-crate test: cg-applied's `Z(BigInt)` satisfies cg-mag's `Ring + ZAlgebra` bound.
//!
//! Phase H-1 Task 5 substrate verification for v0.4.0 §1.17
//! `mobius_function_via_chains_exact<Q: Ring + ZAlgebra>` (Leinster 2008 Cor 1.5
//! finite-category Möbius). Confirms the cross-crate trait dispatch:
//! - cg-mag's `Ring` is a thin extension of `Rig` with `Neg + Sub`, blanket-impl'd.
//! - cg-applied's `Z` (`BigInt` newtype) picks up `Rig` (blanket via `Zero + One +
//!   Add + Mul`) and provides `Neg + Sub` through the `ZAlgebra` supertrait.
//! - Composition: `Z` automatically satisfies cg-mag's `Ring`, and additionally
//!   satisfies `Ring + ZAlgebra` (the bound that v0.4.0 §1.17 requires).
//!
//! The trait was renamed `Integer` → `ZAlgebra` at cg-applied v0.6.0 to clarify
//! that it names a Z-algebra (a unital-ring extension carrying a canonical
//! `ℤ → R` homomorphism), not the mathematical concept of an integer-valued type.

// Both Z and ZAlgebra imported via cg-mag's facade re-exports rather than
// cg-applied directly — exercises the v0.4.0 lib.rs re-export path (M-3
// from Task 5 code-quality review), not just the cross-crate trait bound.
use catgraph_magnitude::{Ring, Z, ZAlgebra};

#[test]
fn z_satisfies_cg_mag_ring_bound() {
    fn assert_bound<Q: Ring>(_x: Q) {}
    assert_bound(Z::from(42_i64));
}

#[test]
fn z_satisfies_ring_plus_zalgebra() {
    fn assert_bound<Q: Ring + ZAlgebra>(_x: Q) {}
    assert_bound(Z::from(42_i64));
}
