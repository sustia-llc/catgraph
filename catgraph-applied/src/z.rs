//! Integer-exact ring `Z(BigInt)` — substrate for catgraph-magnitude
//! §1.17 Leinster 2008 Cor 1.5 integer-exact Möbius inversion.
//!
//! `Z` wraps [`num::BigInt`] for unbounded integer arithmetic. Picks up the
//! [`crate::rig::Rig`] blanket impl via `Clone + PartialEq + Zero + One +
//! Add + Mul`, and the [`crate::integer::ZAlgebra`] supertrait via
//! `Neg + Sub` + the crate-private `Sealed` seal (in
//! `crate::integer::private`, which is `pub(crate)`).
//!
//! ## Why `BigInt`
//!
//! Leinster 2008 §1 establishes Möbius inversion on the ℚ-algebra `R(𝔸)`.
//! Cor 1.5 specialises to integer-valued μ for finite skeletal categories
//! with identity-only endomorphisms. Path counts grow combinatorially with
//! `|objects|`; for a 50-vertex DAG paths can exceed `i64`. `BigInt` is the
//! only backing that is literally integer-exact (matches the paper without
//! a width bound).
//!
//! Performance: ~5-10× slower than native `i64` for small values; heap-
//! allocates per arithmetic op. Acceptable for v0.4.0 fixtures (≤ 20
//! objects); v0.5.0 forward-look §2.4 captures `BigInt → i64` opportunistic
//! specialisation if real-world fixtures push the hot path.
//!
//! ## Why a newtype, not raw `BigInt`
//!
//! Wrapping `num::BigInt` in a crate-owned newtype `Z` keeps trait-impl
//! identity local: future trait widenings (e.g. v0.5.0 Tier 2 `Q` rational)
//! attach cleanly to `Z` rather than to the foreign type. It also gives
//! catgraph-magnitude a stable name to bound on in §1.17
//! `mobius_function_via_chains_exact<Q: Rig + ZAlgebra>` without leaking
//! the `num::BigInt` dependency into downstream APIs.

use crate::integer::{ZAlgebra, private::Sealed};
// `Z` is a `Rig`, so it implements `deep_causality_num::{Zero, One}` (the
// Phase-2 re-substrate). The underlying `num::BigInt` only implements `num`'s
// `Zero`/`One`, so those are kept in scope under aliases to drive the inner
// arithmetic (`BigInt::zero()`/`one()`, `self.0.is_zero()`/`is_one()`).
use deep_causality_num::{One, Zero};
use num::{BigInt, One as NumOne, Zero as NumZero};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Mul, Neg, Sub};

/// Integer-exact ring over [`num::BigInt`].
///
/// Newtype wrapper for crate-owned identity. Satisfies the
/// [`crate::rig::Rig`] blanket impl (via `Clone + PartialEq + Zero + One +
/// Add + Mul`) and [`ZAlgebra`] (via `Neg + Sub + from_i64` plus the
/// crate-private `Sealed` supertrait, implemented for `Z` immediately
/// below the [`ZAlgebra`] impl in this file). [`ZAlgebra`] is sealed;
/// only crate-owned types may implement it — see the [`crate::integer`]
/// module rustdoc for the rationale.
///
/// # Examples
///
/// ```
/// use catgraph_applied::ZAlgebra;
/// use catgraph_applied::z::Z;
///
/// let a = Z::from_i64(3);
/// let b = Z::from_i64(5);
/// assert_eq!(a.clone() + b.clone(), Z::from_i64(8));
/// assert_eq!(a * b, Z::from_i64(15));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Z(pub BigInt);

impl Z {
    /// Construct from an existing [`BigInt`].
    #[must_use]
    pub fn new(value: BigInt) -> Self {
        Z(value)
    }
}

impl Zero for Z {
    fn zero() -> Self {
        Z(<BigInt as NumZero>::zero())
    }
    fn is_zero(&self) -> bool {
        NumZero::is_zero(&self.0)
    }
}

impl One for Z {
    fn one() -> Self {
        Z(<BigInt as NumOne>::one())
    }
    fn is_one(&self) -> bool {
        NumOne::is_one(&self.0)
    }
}

impl Add for Z {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Z(self.0 + other.0)
    }
}

impl Mul for Z {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Z(self.0 * other.0)
    }
}

impl Neg for Z {
    type Output = Self;
    fn neg(self) -> Self {
        Z(-self.0)
    }
}

impl Sub for Z {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Z(self.0 - other.0)
    }
}

impl From<i64> for Z {
    fn from(n: i64) -> Self {
        Z(BigInt::from(n))
    }
}

// Manual `Hash` impl; `#[derive(Hash)]` on `Z` would produce identical code
// since `BigInt: Hash`. Kept explicit for clarity — makes the `Eq`/`Hash`
// contract visible at this site rather than relying on the derive list.
impl Hash for Z {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ZAlgebra for Z {
    fn from_i64(n: i64) -> Self {
        Z::from(n)
    }
}

// Seal invariant: external `Sealed` impls are impossible because
// `crate::integer::private` is `pub(crate)` — see the `## Sealing (v0.6.0)`
// section in `crate::integer`'s rustdoc. This is the gated `Sealed` impl
// that licenses `Z`'s `ZAlgebra` impl above.
impl Sealed for Z {}

// The `Rig` blanket impl applies automatically via the bounds
// `Clone + PartialEq + Zero + One + Add + Mul` (all satisfied above).
