//! Ring-homomorphism axiom tests for [`catgraph_applied::ZAlgebra`].
//!
//! Verifies the implementor obligations declared in the `# Implementor axioms`
//! section of [`ZAlgebra`]:
//!
//! - `Self::from_i64(0) == Self::zero()`
//! - `Self::from_i64(1) == Self::one()`
//! - `Self::from_i64(-n) == -Self::from_i64(n)`
//! - `Self::from_i64(a + b) == Self::from_i64(a) + Self::from_i64(b)`
//! - `Self::from_i64(a * b) == Self::from_i64(a) * Self::from_i64(b)`
//!
//! Together these axioms make `from_i64` a unital ring homomorphism
//! `ℤ → Z(BigInt)`. The Bourbaki anchor is *Algèbre*, Ch. I §8
//! (ℤ as initial object of the category of unital rings): every ring `R`
//! admits a unique unital ring homomorphism `ℤ → R`;
//! [`ZAlgebra::from_i64`] is that homomorphism, restricted to `i64`-
//! representable integers.
//!
//! Cross-references the `compile_fail` doctest on [`ZAlgebra`] for the v0.6.0
//! sealed-impl-guard (the v0.6.0 equivalent of T2's nominal `sealed_impl_guard`
//! sub-target). That doctest prevents external crates from implementing
//! [`ZAlgebra`] on a non-axiomatic backing such as `F64Rig`.
//!
//! Axioms covered here are sample-based: three are unit tests (zero, one,
//! negation), two are proptests with type-driven generation. Overflow on
//! the proptests is sidestepped by `/ 2` (additive) and `i32` lifting
//! (multiplicative) so that the LHS `from_i64` argument stays inside `i64`'s
//! range while the RHS `BigInt` arithmetic is unbounded — making the
//! comparison mathematically faithful.

use catgraph_applied::ZAlgebra;
use catgraph_applied::z::Z;
use deep_causality_num::{One, Zero};
use proptest::prelude::*;

#[test]
fn from_i64_zero_is_ring_zero() {
    assert_eq!(Z::from_i64(0), Z::zero());
}

#[test]
fn from_i64_one_is_ring_one() {
    assert_eq!(Z::from_i64(1), Z::one());
}

#[test]
fn from_i64_negation_distributes() {
    // Avoid `i64::MIN` directly: `-i64::MIN` overflows `i64`. Using
    // `i64::MIN / 2` keeps `-n` representable in `i64` so the construction
    // `-Z::from_i64(n)` is faithful at the `i64` boundary.
    for n in [-42, -1, 0, 1, 42, i64::MAX / 2, i64::MIN / 2] {
        assert_eq!(Z::from_i64(-n), -Z::from_i64(n));
    }
}

proptest! {
    /// `from_i64(a + b) == from_i64(a) + from_i64(b)` for arbitrary `i64`
    /// pairs. The `/ 2` trick bounds `a + b` by `i64::MAX - 1` so the LHS
    /// `from_i64(a + b)` argument is representable without `i64` wrap;
    /// the RHS `Z(BigInt)` arithmetic is unbounded so the comparison is
    /// mathematically faithful.
    #[test]
    fn from_i64_additive_homomorphism(a: i64, b: i64) {
        let (a, b) = (a / 2, b / 2);
        prop_assert_eq!(
            Z::from_i64(a + b),
            Z::from_i64(a) + Z::from_i64(b)
        );
    }

    /// `from_i64(a * b) == from_i64(a) * from_i64(b)` for arbitrary `i32`
    /// pairs lifted into `i64`. `i32 * i32` fits in `i64` exactly, so the
    /// LHS `from_i64(i64::from(a) * i64::from(b))` argument is representable;
    /// both sides land in `Z(BigInt)` and the equality holds exactly.
    #[test]
    fn from_i64_multiplicative_homomorphism(a: i32, b: i32) {
        prop_assert_eq!(
            Z::from_i64(i64::from(a) * i64::from(b)),
            Z::from_i64(i64::from(a)) * Z::from_i64(i64::from(b))
        );
    }
}

/// Explicit boundary regression: `i64::MIN/2 + i64::MAX/2` near the representable
/// edge. Ensures the additive homomorphism holds at boundary values that proptest
/// at default 256 cases may not reach reliably.
#[test]
fn from_i64_boundary_regression() {
    let a = i64::MIN / 2;
    let b = i64::MAX / 2;
    assert_eq!(Z::from_i64(a + b), Z::from_i64(a) + Z::from_i64(b));
    // Negation at boundary
    assert_eq!(Z::from_i64(-a), -Z::from_i64(a));
    assert_eq!(Z::from_i64(-b), -Z::from_i64(b));
}
