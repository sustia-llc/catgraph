//! Comonoid structure on parameter objects — the formal face of weight tying.
//!
//! CDL Theorem G.10: lax (co)algebras for `Para(T)` induce comonoids in the
//! parameter category `M`. The ability to **duplicate** entries in vectors
//! (`δ : P → P ⊗ P`, the comultiplication) or **delete** them
//! (`ε : P → I`, the counit) — the essence of weight tying — *is* the
//! comonoid structure on the parameter object.
//!
//! ## Consumption pathway (catgraph-coalition v0.4.0+)
//!
//! **Layering invariant.** Per `catgraph-dl/CLAUDE.md` "⚠️ CAREFUL — Para
//! is upstream of Quantale": `catgraph-coalition`'s `Quantale` marker trait
//! widens to a full actegory action by **importing**
//! [`catgraph_dl::para::Actegory`](super::Actegory) — never by defining
//! `Actegory` inside `catgraph-coalition`. Any pressure to define
//! `Actegory` downstream is a layering violation: `Para` is upstream of the
//! BTV21 transport layer. The coalition crate widens the marker into an
//! action by writing `impl Actegory<SetMonoidal> for QuantaleActegory` in
//! its own source tree, pulling the trait from this crate.
//!
//! **What coalition v0.4.0 will look like at the call site.** A
//! coalition-flavoured caller defines an `impl Actegory<SetMonoidal> for
//! QuantaleActegory` (with non-trivial action semantics in the coalition's
//! mental model — Tropical-flavoured min-weights, free-monoid concatenation,
//! whatever the BTV21 substrate calls for), builds a
//! `ParaMorphism<SetMonoidal, QuantaleActegory, (P, P), F>` whose action
//! `f(((p1, p2), x))` consumes a paired parameter, and calls
//! [`tie_weights::<C, P, _, X, Y>(parameter_tied, untied)`](tie_weights) to
//! collapse the paired parameter into a single shared `P` (where `C` is the
//! actegory — e.g. `QuantaleActegory` in coalition v0.4.0). The cg-dl
//! `tie_weights` is parametric over the actegory; the coalition v0.4.0
//! caller does not need any cg-dl change to consume it.
//!
//! **What the [`Actegory`](super::Actegory) ▶ widening means for
//! `tie_weights`.** The diagonal `Δ : P → (P, P)` is exact in `(Set, ×, 1)`
//! — the canonical [`DiagonalComonoid`] in this module. In a non-`(Set, ×,
//! 1)` actegory the diagonal is whatever the actegory's [`Comonoid`]
//! gives; `tie_weights` does not care because it consumes via
//! [`Reparameterization::apply`], which threads the user-supplied
//! parameter-substitution closure through the action without re-touching
//! the actegory's tensor structure. Cross-reference to
//! `tests/coalition_consumption_simulation.rs` (in the cg-dl crate) for the
//! in-tree end-to-end simulation of the coalition v0.4.0 caller —
//! defines a local `MockQuantale` ZST playing the role of the future
//! `QuantaleActegory` and exercises the full `tie_weights` pipeline
//! without a coalition dep.
//!
//! ## Trait surface (Phase DL-2)
//!
//! [`Comonoid<M>`] is a uniform-structure trait: an implementor witnesses
//! that *every* object of the parameter category `M` carries the same shape
//! of comonoid (e.g. the diagonal in `Set`). For a one-off comonoid on a
//! specific object, downstream consumers can wrap with their own newtype.
//!
//! Methods are generic in the carrier type `P` so a single implementor
//! (e.g. [`DiagonalComonoid`]) covers the whole "diagonal across `Set`"
//! family without reflection or `dyn`.
//!
//! ## Diagonal in `Set`
//!
//! In `(Set, ×, 1)` the canonical comonoid on every object `P` is the
//! diagonal:
//!
//! ```text
//! δ : P → P × P,  δ(p) = (p, p)
//! ε : P → 1,      ε(p) = ()
//! ```
//!
//! Coassociativity, left counit, and right counit are exact equalities in
//! `Set` (not "up to iso"). The regression tests in
//! `tests/comonoid_laws.rs` are proptest-driven and exercise the laws on
//! several carrier types.
//!
//! ## Weight tying
//!
//! The free function [`tie_weights`] is the consumer-facing API (used by
//! `catgraph-coalition` v0.4.0). It takes a
//! `ParaMorphism<SetMonoidal, C, (P, P), F>` for any
//! `C: Actegory<SetMonoidal>` (v0.4.0 widening; v0.3.x was hardcoded to
//! `SetActegory`) — a 1-morphism whose parameter object is a tensor pair —
//! and returns the diagonal-tied 1-morphism with parameter `P` and action
//! `λ(p, x). f(((p, p), x))`. The categorical content is exactly applying
//! the diagonal `Δ : P → P × P` as a `Reparameterization` 2-morphism.

use core::marker::PhantomData;

use super::monoidal_category::{MonoidalCategory, SetMonoidal};
use super::morphism::ParaMorphism;
use super::reparameterization::Reparameterization;

/// A comonoid structure in a monoidal category `(M, ⊗, I)`.
///
/// CDL Theorem G.10. The implementor witnesses a uniform comonoid structure
/// across the objects of `M`: a comultiplication `δ : P → P ⊗ P` and a
/// counit `ε : P → I` defined for every carrier type `P` (with the
/// per-method bounds the implementor needs — e.g. [`DiagonalComonoid`]
/// requires `P: Clone` to implement `δ(p) = (p, p)`).
///
/// ## Laws (must hold for every implementor)
///
/// Let `δ`, `ε` denote `comultiply` and `counit` respectively, and let
/// `α`, `λ`, `ρ` denote the associator and left/right unitors of `M`.
///
/// - **Coassociativity:** `α ∘ (δ ⊗ id_P) ∘ δ = (id_P ⊗ δ) ∘ δ`
///   (i.e. `δ` followed by duplicating either the left or the right slot
///   gives the same triply-tagged tuple, modulo re-association).
/// - **Left counit:** `λ ∘ (ε ⊗ id_P) ∘ δ = id_P`.
/// - **Right counit:** `ρ ∘ (id_P ⊗ ε) ∘ δ = id_P`.
///
/// For [`DiagonalComonoid`] in `(Set, ×, 1)` these are exact equalities,
/// verified by `tests/comonoid_laws.rs`.
pub trait Comonoid<M: MonoidalCategory> {
    /// Comultiplication `δ : P → P ⊗ P`.
    ///
    /// Applies the comonoid duplication to a value of type `P`. The
    /// returned `M::Tensor<P, P>` is the parameter category's tensor pair
    /// (for `SetMonoidal` this is the Rust tuple `(P, P)`).
    fn comultiply<P: Clone>(&self, p: P) -> M::Tensor<P, P>;

    /// Counit `ε : P → I`.
    ///
    /// Discards the value, returning the monoidal unit. For `SetMonoidal`
    /// the unit is `()`.
    fn counit<P>(&self, p: P) -> M::Unit;
}

/// The diagonal comonoid `(δ_P = (p, p), ε_P = ())` on every object of
/// [`SetMonoidal`].
///
/// CDL Theorem G.10's canonical comonoid in `(Set, ×, 1)`. Zero-sized — the
/// instance carries no runtime data; it is a *witness* that every Rust type
/// carries the diagonal comonoid structure for the Cartesian product.
///
/// # Examples
///
/// ```
/// use catgraph_dl::para::{Comonoid, DiagonalComonoid, SetMonoidal};
///
/// let comonoid = DiagonalComonoid::<SetMonoidal>::new();
/// assert_eq!(comonoid.comultiply(7_i32), (7, 7));
/// assert_eq!(comonoid.counit(7_i32), ());
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiagonalComonoid<M> {
    _phantom: PhantomData<M>,
}

impl<M> DiagonalComonoid<M> {
    /// Construct a fresh diagonal-comonoid witness. Zero-sized; cost-free.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl Comonoid<SetMonoidal> for DiagonalComonoid<SetMonoidal> {
    /// `δ(p) = (p.clone(), p)` — duplicate by cloning the left slot and
    /// moving the right slot into the tensor pair.
    fn comultiply<P: Clone>(&self, p: P) -> <SetMonoidal as MonoidalCategory>::Tensor<P, P> {
        (p.clone(), p)
    }

    /// `ε(p) = ()` — discard the value, returning the monoidal unit `1`.
    fn counit<P>(&self, _p: P) -> <SetMonoidal as MonoidalCategory>::Unit {}
}

/// Tie weights of a `Para(SetMonoidal, C)` 1-morphism via the diagonal
/// comonoid, for any `C: Actegory<SetMonoidal>`.
///
/// CDL Theorem G.10 in concrete form. Given `(P × P, f) : X → Y` with
/// action `f(((p1, p2), x)) → y`, returns the diagonal-tied 1-morphism
/// `(P, f') : X → Y` with action `f'((p, x)) = f(((p, p), x))`. The
/// resulting morphism has both formerly-independent parameter slots driven
/// by a single `p`.
///
/// v0.4.0 widens this from the v0.3.x `SetActegory`-bound function to
/// `C: Actegory<SetMonoidal>`. The diagonal `Δ : P → (P, P)` is exact in
/// `(Set, ×, 1)`; for richer actegories `C` the body is structurally
/// agnostic — it constructs `Δ` as a `Reparameterization<SetMonoidal, …>`
/// and delegates to [`Reparameterization::apply`], which carries the
/// actegory through the parameter substitution without re-touching the
/// actegory's tensor structure.
///
/// This is the *consumer-facing* weight-tying API used by
/// `catgraph-coalition` v0.5.0+ (per workspace plan slot 2). The
/// `catgraph-coalition` caller defines an
/// `impl Actegory<SetMonoidal> for {UnitIntervalQ, TropicalQ,
/// QuantaleDefault}` and calls
/// `tie_weights::<UnitIntervalQ, P, F, X, Y>(p, untied)`. Internally
/// `tie_weights` constructs the diagonal `Δ : P → (P, P)` as a
/// [`Reparameterization`] and delegates to
/// [`Reparameterization::apply`].
///
/// # Type parameters
///
/// - `C` — the actegory of `Para(SetMonoidal, C)`. v0.4.0 widening: any
///   `C: Actegory<SetMonoidal>` is accepted; v0.3.x was hardcoded to
///   `SetActegory`. Placed at LEFTMOST position so callers using inference
///   (`tie_weights(p, untied)` with full inference) work unchanged; callers
///   using explicit turbofish move from 4-parameter
///   `tie_weights::<P, F, X, Y>` to 5-parameter
///   `tie_weights::<C, P, F, X, Y>`.
/// - `P` — the (collapsed) parameter type. Must be `Clone` so the diagonal
///   can duplicate it.
/// - `F` — the original action `f : (P, P) × X → Y`.
/// - `X`, `Y` — carrier types in the category `C` acts on.
///
/// # Arguments
///
/// - `parameter_tied` — the value of the new (single) parameter. The
///   `Para` morphism carries its parameter at the value level, so the
///   caller supplies one here. This corresponds to the observation that
///   *applying* a 2-morphism in CDL §3.1 is a substitution of parameter
///   value, not just type.
/// - `untied` — the original 1-morphism with paired-parameter object
///   `(P, P)`.
///
/// # Returns
///
/// A `ParaMorphism` whose parameter is `parameter_tied: P` and whose
/// action is `λ(p, x). untied.action(((p, p), x))`.
///
/// # Examples
///
/// ```
/// use catgraph_dl::para::{ParaMorphism, SetActegory, SetMonoidal, tie_weights};
///
/// let untied: ParaMorphism<SetMonoidal, SetActegory, (i64, i64), _> = ParaMorphism::new(
///     (0_i64, 0_i64),
///     |((p1, p2), x): ((i64, i64), i64)| p1 + p2 + x,
/// );
///
/// let tied = tie_weights::<SetActegory, i64, _, i64, i64>(3_i64, untied);
/// assert_eq!((tied.action)((3_i64, 5_i64)), 11_i64);
/// ```
#[allow(
    clippy::type_complexity,
    reason = "the fully-qualified return ParaMorphism<SetMonoidal, C, P, impl Fn((P, X)) -> Y> has every parameter load-bearing — a type alias would still need every parameter"
)]
pub fn tie_weights<C, P, F, X, Y>(
    parameter_tied: P,
    untied: ParaMorphism<SetMonoidal, C, (P, P), F>,
) -> ParaMorphism<SetMonoidal, C, P, impl Fn((P, X)) -> Y>
where
    C: super::actegory::Actegory<SetMonoidal>,
    P: Clone,
    F: Fn(((P, P), X)) -> Y,
{
    // Δ : P → (P, P). Implemented directly here rather than via
    // `DiagonalComonoid::comultiply` because `Reparameterization::apply`
    // wants a `Fn(PNew) -> POld` closure, not a method invocation
    // borrowing `&self` against the comonoid witness.
    let diagonal: Reparameterization<SetMonoidal, _> =
        Reparameterization::new(|p: P| (p.clone(), p));

    diagonal.apply::<C, P, (P, P), F, X, Y>(parameter_tied, untied)
}
