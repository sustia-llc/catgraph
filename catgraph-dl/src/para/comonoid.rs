//! Comonoids on parameter objects (CDL Thm G.10): comultiplication
//! `δ : P → P ⊗ P` and counit `ε : P → I`. [`Comonoid<M>`] is uniform over
//! every carrier `P`; [`DiagonalComonoid`] is `δ(p) = (p, p)`, `ε(p) = ()` in
//! `(Set, ×, 1)`; [`tie_weights`] applies `δ` as a [`Reparameterization`]
//! 2-morphism to a `ParaMorphism` with paired parameter `(P, P)`.

use core::marker::PhantomData;

use super::monoidal_category::{MonoidalCategory, SetMonoidal};
use super::morphism::ParaMorphism;
use super::reparameterization::Reparameterization;

/// Uniform comonoid structure in `(M, ⊗, I)` (CDL Thm G.10):
/// `δ` = [`comultiply`](Self::comultiply) (bound `P: Clone`), `ε` =
/// [`counit`](Self::counit). Laws, with `α`, `λ`, `ρ` the coherence maps of `M`:
///
/// - coassociativity `α ∘ (δ ⊗ id_P) ∘ δ = (id_P ⊗ δ) ∘ δ`
/// - left counit `λ ∘ (ε ⊗ id_P) ∘ δ = id_P`
/// - right counit `ρ ∘ (id_P ⊗ ε) ∘ δ = id_P`
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

/// The diagonal comonoid `δ(p) = (p, p)`, `ε(p) = ()` on every object of
/// [`SetMonoidal`]; zero-sized.
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

/// Weight tying (CDL Thm G.10): from `(P × P, f) : X → Y` with action
/// `f(((p1, p2), x))` to `(P, f') : X → Y` with parameter `parameter_tied`
/// and `f'((p, x)) = f(((p, p), x))`, for any `C: Actegory<SetMonoidal>`.
/// `P: Clone` is the comultiplication.
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
