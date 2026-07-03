//! `Para(M, C)` 2-morphisms — reparameterizations.
//!
//! CDL §3.1. A 2-morphism `(P, f) ⇒ (P', f')` is a morphism `r : P' → P`
//! in `M` making the parameter-substitution triangle commute. Weight tying
//! is the special case `r = Δ_P : P → P × P` (diagonal comonoid; CDL
//! Theorem G.10).
//!
//! ## Body shape
//!
//! [`Reparameterization`] carries the underlying morphism `r : P' → P` as
//! a closure `Fn(P_new) -> P_old`. The [`Reparameterization::apply`]
//! method takes a `ParaMorphism<…, P_old, F>` and produces
//! `ParaMorphism<…, P_new, F'>` where
//!
//! ```text
//! F'((p_new, x)) = F((r(p_new), x))
//! ```
//!
//! For the diagonal `Δ : P → P × P` carrying `Δ(p) = (p, p)`, applying it
//! to `(P × P, f)` produces `(P, λ(p, x). f(((p, p), x)))` — the weight-
//! tied morphism. This is the categorical content of weight tying: the
//! 2-cell `Δ` collapses two parameter slots into one.

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
    /// Apply this reparameterization to a `Para(SetMonoidal, C)` 1-morphism
    /// for any `C: Actegory<SetMonoidal>`, pre-composing the parameter
    /// substitution.
    ///
    /// CDL §3.1. Given `r : P' → P` (this object) and a `Para` 1-morphism
    /// `(P, f) : X → Y`, produces `(P', f') : X → Y` where
    ///
    /// ```text
    /// f'((p', x)) = f((r(p'), x))
    /// ```
    ///
    /// The new morphism carries `parameter = parameter_new` directly
    /// (caller-supplied), since `r` is a function from `P'` to `P` and the
    /// `Para` morphism carries its parameter object at the value level.
    ///
    /// The current API widens this from the earlier `SetActegory`-bound impl to
    /// `C: Actegory<SetMonoidal>`. The body is structurally agnostic to the
    /// actegory — it threads the user-supplied substitution closure through
    /// the action's parameter slot without re-touching the actegory's
    /// tensor structure.
    ///
    /// # Examples
    ///
    /// Weight tying via the diagonal `Δ : P → (P, P)`. Given
    /// `(P × P, f : (P, P) × X → Y)`, the diagonal reparameterization
    /// `Δ(p) = (p, p)` collapses the two parameter slots into one and the
    /// resulting `Para` 1-morphism has parameter `P` and action
    /// `λ(p, x). f(((p, p), x))`.
    ///
    /// # Type parameters
    ///
    /// - `C` — the actegory of `Para(SetMonoidal, C)`. The current API accepts
    ///   any `C: Actegory<SetMonoidal>`; the earlier surface was hardcoded
    ///   to `SetActegory`.
    /// - `PNew` — the new (codomain-of-`r` upstream, domain-of-the-result)
    ///   parameter type `P'`.
    /// - `POld` — the original parameter type `P`. Carried by the input
    ///   `ParaMorphism`.
    /// - `F` — the original action `f : P × X → Y`.
    /// - `X`, `Y` — carrier types in the category `C` acts on.
    ///
    /// # Returns
    ///
    /// A `ParaMorphism` whose parameter is `parameter_new` and whose
    /// action is the pre-composition closure.
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
