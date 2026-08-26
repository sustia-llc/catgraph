//! F-algebras, F-coalgebras, and monad algebras.
//!
//! CDL §2: [`FAlgebra`], [`FCoalgebra`], [`MonadAlgebra`] structure-map
//! wrappers; [`FAlgebraHom`] / [`FCoalgebraHom`] / [`MonadAlgebraHom`] with
//! sampled `verify_commutes` (CDL Def 2.5); [`Group`] / [`Z2Group`] /
//! [`GroupActionEndo`] for the `F = G × −` equivariance example (CDL Ex 2.6).
//! [`HKT`] / [`Functor`] are re-exported from [`crate::endofunctor`].

mod coalgebra;
mod f_algebra;
mod group_action;
mod monad_algebra;

pub use crate::endofunctor::{EndoWitness, Functor, HKT};
pub use coalgebra::{FCoalgebra, FCoalgebraHom};
pub use f_algebra::{FAlgebra, FAlgebraHom};
pub use group_action::{Group, GroupActionEndo, Z2Group};
pub use monad_algebra::{MonadAlgebra, MonadAlgebraHom};
