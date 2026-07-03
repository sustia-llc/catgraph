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
//! ## HKT shape
//!
//! The result of acting on an object is a Generic Associated Type
//! [`Actegory::ActionResult`]. For [`SetActegory`] (the only instance
//! currently shipped), `▶` is Cartesian product, so `ActionResult<P, X> = (P, X)`.
//! Other instances may project differently — e.g. an `R`-module
//! actegory would project `(P, X) ↦ P · X` for some scalar action.
//!
//! Closure convention: `Fn((P, X)) -> Y` (tuple-as-single-argument). See the
//! `monoidal_category` module for the rationale.
//!
//! ## Why methods take `&self`
//!
//! [`Actegory::act`] and [`Actegory::compose_action`] take `&self` for the
//! same future-proofing reason as [`MonoidalCategory`] — see the
//! [`super::monoidal_category`] "Why methods take `&self`" section. Future
//! instances over richer actegories (R-module action carrying a base ring;
//! vector-bundle action carrying a connection; coalition's `QuantaleActegory`
//! carrying Tropical-flavoured min-weight semantics) will use the `&self`
//! slot for runtime data. [`SetActegory`] is a ZST so the receiver is
//! unobservable today, but freezing the trait at static methods would force
//! a breaking change later.
//!
//! **Rationale validation:** a downstream coalition
//! `impl Actegory<SetMonoidal>` for the three quantale ZSTs is the first
//! consumer expected to carry runtime data (Tropical zero / one
//! for the underlying min-plus semiring; BTV21 free-monoid generator
//! references; Lawvere-metric embedding parameter). The shipped surface commits to the
//! `&self` slot for future-proofing; the audit checkpoint fires at that
//! consumer's post-shipping review and either ratifies the choice
//! or opens a follow-up to consider static dispatch. See
//! [`AUDIT-CHECKPOINT-v0.4.0.md`](../../docs/AUDIT-CHECKPOINT-v0.4.0.md)
//! for audit criteria.

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
/// This is the only actegory currently shipped; richer actegories
/// (R-module, vector-bundle, fibration-based) are deferred.
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
