//! `Para(M, C)` 2-morphisms — reparameterizations.
//!
//! CDL §3.1. A 2-morphism `(P, f) ⇒ (P', f')` is a morphism `r : P' → P`
//! in `M` making the parameter-substitution triangle commute. Weight tying
//! is the special case `r = Δ_P : P → P × P` (diagonal comonoid; CDL
//! Theorem G.10).
//!
//! [`Reparameterization`] carries `r` as `Fn(P_new) -> P_old`;
//! [`Reparameterization::apply`] sends `(P_old, F)` to `(P_new, F')` with
//! `F'((p_new, x)) = F((r(p_new), x))`.

use core::marker::PhantomData;

use super::actegory::Actegory;
use super::monoidal_category::{MonoidalCategory, SetMonoidal};
use super::morphism::ParaMorphism;

/// A 2-morphism `(P, f) ⇒ (P', f')` in `Para(M, C)` — a reparameterization
/// `r : P' → P`.
///
/// Weight tying via the diagonal comonoid `Δ_P : P → P × P` is the
/// canonical instance (CDL Theorem G.10).
#[derive(Debug, Clone)]
pub struct Reparameterization<M, R>
where
    M: MonoidalCategory,
{
    /// The underlying morphism `r : P' → P` in `M`.
    pub map: R,
    _phantom: PhantomData<M>,
}

impl<M, R> Reparameterization<M, R>
where
    M: MonoidalCategory,
{
    /// Construct a reparameterization from an underlying `M`-morphism.
    pub fn new(map: R) -> Self {
        Self {
            map,
            _phantom: PhantomData,
        }
    }
}

impl<R> Reparameterization<SetMonoidal, R> {
    /// `(P, f) : X → Y` to `(P', f') : X → Y` with `f'((p', x)) = f((r(p'), x))`
    /// and parameter `parameter_new`, for any `C: Actegory<SetMonoidal>`
    /// (CDL §3.1).
    #[allow(
        clippy::type_complexity,
        reason = "the fully-qualified return ParaMorphism<SetMonoidal, C, PNew, impl Fn((PNew, X)) -> Y> has every parameter load-bearing — a type alias would still need every parameter"
    )]
    pub fn apply<C, PNew, POld, F, X, Y>(
        self,
        parameter_new: PNew,
        morphism: ParaMorphism<SetMonoidal, C, POld, F>,
    ) -> ParaMorphism<SetMonoidal, C, PNew, impl Fn((PNew, X)) -> Y>
    where
        C: Actegory<SetMonoidal>,
        R: Fn(PNew) -> POld,
        F: Fn((POld, X)) -> Y,
    {
        let r = self.map;
        let ParaMorphism { action: f, .. } = morphism;

        let f_prime = move |(p_new, x): (PNew, X)| -> Y {
            let p_old = r(p_new);
            f((p_old, x))
        };

        ParaMorphism::new(parameter_new, f_prime)
    }
}
