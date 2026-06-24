//! [`Ring`] super-trait over [`Rig`].
//!
//! Adds additive inverses (`Neg` + `Sub`), enabling Gaussian elimination on
//! `Matrix<Q>`. Required by Möbius inversion `ζ · μ = I` and the
//! `Q: Ring`-bounded magnitude functions in the
//! [`magnitude`](crate::magnitude) module.
//!
//! `F64Rig` satisfies `Ring`; `BoolRig`, `UnitInterval`, `Tropical` do not
//! (no additive-inverse operation in those rigs).

use catgraph_applied::rig::Rig;
use std::ops::{Neg, Sub};

/// Ring: a [`Rig`] with additive inverses.
///
/// Blanket-impl'd for any `T: Rig + Neg<Output = T> + Sub<Output = T>`.
///
/// This is a thin extension of `Rig` for v0.1.0 magnitude — Gaussian
/// elimination on `Matrix<Q>` (used in [`mobius_function`](crate::magnitude::mobius_function))
/// requires subtraction. `BoolRig` / `UnitInterval` / `Tropical` are
/// excluded; the chain-sum Möbius variant deferred to v0.2.0 will handle
/// rig-generic case.
pub trait Ring: Rig + Neg<Output = Self> + Sub<Output = Self> {}

impl<T> Ring for T where T: Rig + Neg<Output = T> + Sub<Output = T> {}
