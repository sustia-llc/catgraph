//! `M`-actegories — categories acted on by a monoidal category.
//!
//! CDL §3.1: an `M`-actegory `(C, ▶)` consists of a category `C` and an
//! action `▶ : M × C → C` together with coherence witnesses that play the
//! role of the monoid laws of the parameter category. The
//! [`Actegory::compose_action`] method captures the pseudo-functorial
//! coherence
//!
//! ```text
//! μ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X
//! ```
//!
//! used in the sequential composition rule for `Para(M, C)`.
//!
//! The action result is the GAT [`Actegory::ActionResult`]; for
//! [`SetActegory`] it is `(P, X)`, for [`RActegory`](super::RActegory) it is
//! [`DirectSum`](super::DirectSum). Closure convention: `Fn((P, X)) -> Y`.

use super::monoidal_category::{MonoidalCategory, SetMonoidal};

/// A category `C` together with a left action `▶ : M × C → C` of a monoidal
/// category `M`.
///
/// CDL §3.1, paraphrasing Capucci et al. 2022 / Cruttwell et al. 2022.
///
/// The trait carries:
///
/// - [`Actegory::Object`] — kind of objects of `C` (marker; actual objects
///   are Rust types at the value level).
/// - [`Actegory::Morphism`] — kind of morphisms of `C` (marker).
/// - [`Actegory::ActionResult`] — the GAT projecting `(P, X) ↦ P ▶ X`.
/// - [`Actegory::act`] — apply the action: `(P, X) ↦ P ▶ X`.
/// - [`Actegory::compose_action`] — the coherence isomorphism
///   `μ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X`.
///
/// For `M = (Set, ×, 1)` and `C = Set` (the [`SetActegory`] instance), `▶`
/// is Cartesian product `(P, X) ↦ (P, X)` and `μ` is the canonical tuple
/// re-association `(q, (p, x)) ↦ ((q, p), x)`.
pub trait Actegory<M: MonoidalCategory> {
    /// Marker for the kind of objects of the underlying category `C`.
    type Object;

    /// Marker for the kind of morphisms of `C`.
    type Morphism;

    /// The result of acting on an object: `P ▶ X` as a GAT, parameterised
    /// by both the parameter type `P` and the carrier `X`.
    ///
    /// For [`SetActegory`] this projects to `(P, X)`.
    type ActionResult<P, X>;

    /// Apply the action: `(P, X) ↦ P ▶ X`.
    ///
    /// CDL §3.1 — the underlying map of `▶ : M × C → C` at the
    /// value level.
    fn act<P, X>(&self, parameter: P, x: X) -> Self::ActionResult<P, X>;

    /// Coherence isomorphism `μ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X`.
    ///
    /// CDL §3.1. This witnesses pseudo-functoriality of the action — the
    /// "associativity up to iso" linking iterated single-step action with
    /// tensored-parameter single-step action. Used in [`super::ParaMorphism::compose`].
    ///
    /// For [`SetActegory`] this is the tuple re-association
    /// `(q, (p, x)) ↦ ((q, p), x)`.
    fn compose_action<Q, P, X>(&self, q: Q, p: P, x: X) -> Self::ActionResult<M::Tensor<Q, P>, X>;
}

/// The Cartesian-product actegory of `(Set, ×, 1)` acting on `Set`.
///
/// CDL §3.1 default. Action is `▶ : Set × Set → Set, (P, X) ↦ (P, X)`. The
/// coherence `μ` is the canonical tuple re-association
/// `(q, (p, x)) ↦ ((q, p), x)` — exact in `Set`, not "up to iso".
///
/// Shipped alongside the R-module self-action
/// [`F64Actegory`](super::F64Actegory) (`(FinReal, ⊕, R⁰)` acting on itself,
/// issue #36); richer actegories (vector-bundle, fibration-based) remain
/// deferred.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SetActegory;

impl SetActegory {
    /// Construct a fresh `SetActegory` instance. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Actegory<SetMonoidal> for SetActegory {
    type Object = super::monoidal_category::SetObject;
    type Morphism = super::monoidal_category::SetMorphism;
    type ActionResult<P, X> = (P, X);

    fn act<P, X>(&self, parameter: P, x: X) -> Self::ActionResult<P, X> {
        (parameter, x)
    }

    fn compose_action<Q, P, X>(
        &self,
        q: Q,
        p: P,
        x: X,
    ) -> Self::ActionResult<<SetMonoidal as MonoidalCategory>::Tensor<Q, P>, X> {
        // μ : Q ▶ (P ▶ X) = (q, (p, x))  →  (Q ⊗ P) ▶ X = ((q, p), x)
        ((q, p), x)
    }
}
