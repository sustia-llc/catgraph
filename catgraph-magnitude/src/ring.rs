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
/// Blanket-impl'd for any `T: Rig + Neg<Output = T> + Sub<Output = T>`, which
/// is the bound the Gaussian elimination of
/// [`mobius_function`](crate::magnitude::mobius_function) needs. `BoolRig`,
/// `UnitInterval` and `Tropical` do not satisfy it.
pub trait Ring: Rig + Neg<Output = Self> + Sub<Output = Self> {}

impl<T> Ring for T where T: Rig + Neg<Output = T> + Sub<Output = T> {}
