//! `ZAlgebra` trait — sealed marker for rings admitting a unital ring
//! homomorphism `ℤ → R`.
//!
//! Marker + helper for rings carrying integer-exact arithmetic. Distinct
//! from the floating-point [`crate::rig::F64Rig`] and the unit-interval /
//! tropical rigs in [`crate::rig`]. Used by catgraph-magnitude §1.17
//! paper-faithful Cor 1.5 chain-sum Möbius (Leinster 2008).
//!
//! For the Z-algebra terminology and Bourbaki *Algèbre* Ch. I §8
//! (ℤ as initial object of the category of unital rings) anchor,
//! see [`ZAlgebra`]. (The previous name `Integer` was renamed to
//! `ZAlgebra` to clarify that the trait names a Z-algebra — a
//! ring admitting a unique unital ring homomorphism `ℤ → R` — not a
//! wrapper for `i64` / `BigInt`.)
//!
//! ## Relationship to [`Rig`]
//!
//! cg-applied does not expose a standalone `Ring` trait — see the comment at
//! `rig.rs` lines 223-228 ("The ring/field bound stays off `Rig` itself").
//! [`ZAlgebra`] bridges that gap for the integer sub-case: it extends
//! [`Rig`] with [`Neg`] and [`Sub`] so callers can negate
//! and subtract integer ring elements, while keeping the floating-point
//! [`Div`](std::ops::Div) bound off the trait. Lifting an `i64` into the
//! ring is handled by [`ZAlgebra::from_i64`].
//!
//! ## Sealing
//!
//! [`ZAlgebra`] carries a `private::Sealed` supertrait bound. The
//! `private` module is `pub(crate)`, so external crates cannot name (and
//! therefore cannot satisfy) the `Sealed` bound and therefore cannot
//! implement [`ZAlgebra`]. This is the standard "sealed trait" pattern
//! (precedent: `catgraph-dl`'s `para::monoidal_category::private::Sealed`
//! soft-seal on `SetCategoryDefaults`; cg-applied tightens that
//! pattern from `pub mod private` to `pub(crate) mod private` so the seal
//! is unbypassable rather than dual-impl-coordinated). Rationale: the
//! [`ZAlgebra::from_i64`] axioms below demand a genuine unital
//! ring-homomorphism `ℤ → Self`; the rig blanket impl on
//! [`crate::rig::F64Rig`] does NOT satisfy these axioms (floating-point
//! arithmetic is not exact), so allowing external impls invites soundness
//! bugs in the catgraph-magnitude §1.17 integer-exact Möbius path.
//! Sealing keeps `Z(BigInt)` (and any future crate-owned integer rig) the
//! only path.
//!
//! ## Why `from_i64`, not `From<i64>`?
//!
//! The constructor is an associated function on [`ZAlgebra`] rather than a
//! `From<i64>` supertrait bound. This matches `num_traits::FromPrimitive`
//! precedent and avoids constraining implementors to provide `From<i64>`
//! before they can implement [`ZAlgebra`] — `Z(BigInt)` and any future
//! crate-owned fixed-width-integer rig pick up the trait without orphan-rule
//! friction. Implementors providing `impl From<i64>` independently are free
//! to define `from_i64(n) = Self::from(n)` as a one-liner.
//!
//! ## Paper anchor
//!
//! Leinster 2008 "The Euler characteristic of a category" (arXiv:0610260):
//! the algebra `R(𝔸)` is the ℚ-algebra of functions `ob𝔸 × ob𝔸 → ℚ`.
//! For Cor 1.5 (finite skeletal categories with identity-only endomorphisms),
//! Möbius values μ are integer-valued. This trait bounds the integer
//! sub-ring of `R(𝔸)`.

use crate::rig::Rig;
use std::ops::{Neg, Sub};

/// Sealing module for [`ZAlgebra`].
///
/// The trait `Sealed` inside this module is the supertrait of
/// [`super::ZAlgebra`]; because this module is `pub(crate)`, no downstream
/// crate can name `Sealed` in an `impl` block, so no downstream crate
/// can satisfy the [`ZAlgebra`](super::ZAlgebra) supertrait bound.
///
/// This is an **implementation detail**; downstream code should not
/// reference `Sealed` directly. The seal exists to enforce the
/// [`ZAlgebra::from_i64`](super::ZAlgebra::from_i64) axioms — see the
/// "## Sealing" section in the crate-level rustdoc above for
/// the full rationale. Precedent: `catgraph-dl`'s
/// `para::monoidal_category::private::Sealed` (soft-seal on
/// `SetCategoryDefaults`).
pub(crate) mod private {
    /// Sealing trait for [`super::ZAlgebra`]. Implementing this
    /// trait is the gated step that lets a type also implement
    /// [`super::ZAlgebra`]. The enclosing `private` module is
    /// `pub(crate)`, so downstream crates cannot name this trait in an
    /// `impl` block.
    pub trait Sealed {}
}

/// Sealed marker trait for ℤ-algebras: rings admitting a unique unital ring
/// homomorphism `ℤ → Self`.
///
/// Extends [`Rig`] (the cg-applied semiring trait) with [`Neg`] and [`Sub`]
/// so that integer ring elements support negation and subtraction, then
/// adds an [`ZAlgebra::from_i64`] constructor for lifting `i64` constants
/// into the ring.
///
/// # Bourbaki anchor
///
/// Bourbaki, *Algèbre*, Ch. I §8 (ℤ as initial object of the category of
/// unital rings): every ring `R` admits a **unique** unital ring homomorphism
/// `ℤ → R`; a ℤ-algebra **is** a ring viewed through this canonical
/// homomorphism. The trait's [`ZAlgebra::from_i64`] method **is** that
/// homomorphism, restricted to `i64`-representable integers.
///
/// # Sealing
///
/// [`ZAlgebra`] is sealed via a crate-private supertrait
/// (`private::Sealed`; module is `pub(crate)`). External crates cannot
/// implement [`ZAlgebra`] for their own types — the `compile_fail` example
/// below demonstrates this. Only types defined inside `catgraph-applied`
/// may carry an impl.
///
/// # Implementor axioms
///
/// Implementations should satisfy, for all `a: i64`, `b: i64`:
/// - `Self::from_i64(0) == Self::zero()`
/// - `Self::from_i64(1) == Self::one()`
/// - `Self::from_i64(-n) == -Self::from_i64(n)`
/// - `Self::from_i64(a + b) == Self::from_i64(a) + Self::from_i64(b)`
/// - `Self::from_i64(a * b) == Self::from_i64(a) * Self::from_i64(b)`
///
/// These axioms make `from_i64` a unital ring homomorphism `ℤ → Self`.
///
/// # Intended implementors
///
/// - [`crate::z::Z`] (`BigInt` newtype) — integer-exact arithmetic for
///   `mobius_function_via_chains_exact<Q: Ring + ZAlgebra>` in
///   catgraph-magnitude (Leinster 2008 Cor 1.5).
/// - Any future fixed-width-integer rig (e.g. `i64`, `i128`) defined
///   inside `catgraph-applied` that wishes to opt into the integer-exact
///   Möbius path.
///
/// # External impls are rejected
///
/// The sealed-trait pattern makes attempts to implement [`ZAlgebra`]
/// outside `catgraph-applied` fail to compile because the
/// `private::Sealed` supertrait bound cannot be satisfied — the
/// `private` module is `pub(crate)`. The doctest below isolates this
/// failure: the local newtype `MyRig` satisfies the orphan rule (it is
/// defined in the doctest's own crate), so the orphan-rule check passes
/// and the **proximate** compile failure becomes an unsatisfied
/// `private::Sealed` supertrait bound (rustc emits a diagnostic of the
/// form *"`ZAlgebra` is a sealed trait, because to implement it you
/// also need to implement `Sealed`, which is not accessible"*).
/// External impls fail at the seal, not at the orphan rule.
///
/// ```compile_fail
/// // Attempting to implement `ZAlgebra` for a downstream-defined rig.
/// // The local newtype `MyRig` is defined in this crate, so the orphan
/// // rule is satisfied — the *only* remaining barrier is the unnameable
/// // `private::Sealed` supertrait bound on `ZAlgebra`.
/// use catgraph_applied::ZAlgebra;
/// use deep_causality_num::{One, Zero};
/// use std::ops::{Add, Mul, Neg, Sub};
///
/// #[derive(Clone, PartialEq)]
/// struct MyRig;
///
/// impl Zero for MyRig {
///     fn zero() -> Self { MyRig }
///     fn is_zero(&self) -> bool { true }
/// }
/// impl One for MyRig {
///     fn one() -> Self { MyRig }
///     fn is_one(&self) -> bool { true }
/// }
/// impl Add for MyRig {
///     type Output = Self;
///     fn add(self, _: Self) -> Self { MyRig }
/// }
/// impl Mul for MyRig {
///     type Output = Self;
///     fn mul(self, _: Self) -> Self { MyRig }
/// }
/// impl Neg for MyRig {
///     type Output = Self;
///     fn neg(self) -> Self { MyRig }
/// }
/// impl Sub for MyRig {
///     type Output = Self;
///     fn sub(self, _: Self) -> Self { MyRig }
/// }
///
/// // `Rig` is satisfied via the blanket impl (Clone + PartialEq + Zero +
/// // One + Add + Mul are all in scope). `Neg + Sub` are direct supertraits
/// // of `ZAlgebra`. The seal is the only remaining barrier — and it is
/// // unnameable from outside `catgraph-applied`.
/// impl ZAlgebra for MyRig {
///     fn from_i64(_: i64) -> Self { MyRig }
/// }
/// ```
pub trait ZAlgebra: Rig + Neg<Output = Self> + Sub<Output = Self> + private::Sealed {
    /// The unique unital ring homomorphism `ℤ → Self` (Bourbaki, *Algèbre*
    /// Ch. I §8 — ℤ is the initial object in the category of unital rings),
    /// restricted to `i64`-representable integers. Axioms verified at
    /// `catgraph-applied/tests/zalgebra_axioms.rs`.
    fn from_i64(n: i64) -> Self;
}
