//! `ZAlgebra` trait — sealed marker for rings admitting a unital ring
//! homomorphism `ℤ → R`.
//!
//! Marker + helper for rings carrying integer-exact arithmetic. Distinct
//! from the floating-point [`crate::rig::F64Rig`] and the unit-interval /
//! tropical rigs in [`crate::rig`].
//!
//! ## Relationship to [`Rig`]
//!
//! cg-applied exposes no standalone `Ring` trait; [`ZAlgebra`] bridges that
//! gap for the integer sub-case by extending [`Rig`] with [`Neg`] and [`Sub`],
//! while keeping the floating-point [`Div`](std::ops::Div) bound off the
//! trait. Lifting an `i64` into the ring is [`ZAlgebra::from_i64`], an
//! associated function rather than a `From<i64>` supertrait bound, so an
//! implementor need not provide `From<i64>` first.
//!
//! ## Sealing
//!
//! [`ZAlgebra`] carries a `private::Sealed` supertrait bound whose module is
//! `pub(crate)`, so external crates cannot name it and therefore cannot
//! implement [`ZAlgebra`]. Only types defined inside `catgraph-applied` —
//! `Z(BigInt)` and any future crate-owned integer rig — can carry an impl.
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
pub(crate) mod private {
    /// Sealing trait for [`super::ZAlgebra`]: implementing it is the gated
    /// step that lets a type also implement [`super::ZAlgebra`].
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
/// [`ZAlgebra`] is sealed via a crate-private supertrait (`private::Sealed`;
/// module is `pub(crate)`). Only types defined inside `catgraph-applied` may
/// carry an impl.
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
/// [`crate::z::Z`] (a `BigInt` newtype), and any future fixed-width-integer
/// rig defined inside `catgraph-applied`.
///
/// # External impls are rejected
///
/// An impl outside `catgraph-applied` fails to compile at the unsatisfiable
/// `private::Sealed` supertrait bound — not at the orphan rule, which a local
/// newtype like the `MyRig` below satisfies.
///
/// ```compile_fail
/// use catgraph_applied::ZAlgebra;
/// use catgraph_applied::rig::{One, Zero};
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
