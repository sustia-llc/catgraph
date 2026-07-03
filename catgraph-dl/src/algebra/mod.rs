//! F-algebras, F-coalgebras, and monad algebras.
//!
//! CDL §2 — the unifying observation that a neural-network layer can be
//! viewed as an F-algebra homomorphism between two algebras for the same
//! endofunctor `F`. When `F = G × −` for a group `G`, this recovers
//! Geometric Deep Learning's equivariant maps as monad-algebra
//! homomorphisms (CDL Example 2.6). When `F = 1 + A × −`, list folds
//! emerge as the unique algebra homomorphism from the initial algebra
//! `List(A)` (CDL Remark 2.13).
//!
//! ## Status
//!
//! - [`FAlgebra`], [`FCoalgebra`], [`MonadAlgebra`] — structure-map
//!   wrappers.
//! - [`FAlgebraHom`] / [`FCoalgebraHom`] / [`MonadAlgebraHom`] —
//!   homomorphism types with caller-driven `verify_commutes`
//!   entry points (CDL Definition 2.5 + dual).
//! - [`EndoFunctor`] / [`Group`] / [`Z2Group`] / [`GroupActionEndo`] —
//!   the group-action monad witness for the **CDL Example 2.6
//!   Geometric-Deep-Learning recovery**: F-algebra homomorphisms over
//!   `F = G × −` are exactly `G`-equivariant maps.
//!
//! ## `EndoFunctor` location
//!
//! The [`EndoFunctor`] trait is canonical in [`crate::endofunctor`] and
//! re-exported through the private `group_action` submodule for this
//! module's consumers.

mod coalgebra;
mod f_algebra;
mod group_action;
mod monad_algebra;

pub use coalgebra::{FCoalgebra, FCoalgebraHom};
pub use f_algebra::{FAlgebra, FAlgebraHom};
pub use group_action::{EndoFunctor, Group, GroupActionEndo, Z2Group};
pub use monad_algebra::{MonadAlgebra, MonadAlgebraHom};
