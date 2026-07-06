//! Containers — the shape/position presentation of a polynomial endofunctor.
//!
//! Abbott–Altenkirch–Ghani 2003, *Categories of Containers*, reached via the
//! CDL Discussion §"Containers and Type-Safe Design" (issue #41). A container
//! `S ◁ P` has a set of *shapes* `S` and, for each shape `s`, a set of
//! *positions* `P(s)`; its extension is the polynomial functor
//!
//! ```text
//! ⟦S ◁ P⟧(X) = Σ_{s : S} X^{P(s)}
//! ```
//!
//! We ship the **finitary** presentation: positions at a shape `s` are the
//! ordinals `0..arity(s)`, so the contents `X^{P(s)}` are carried in position
//! order as a `Vec<X>`. This is exactly the observation that a polynomial
//! functor's `fmap` acts on the *contents* only, leaving the shape fixed.
//!
//! # The shipped endofunctors as containers
//!
//! | Endofunctor | Witness | `Shape` | `arity` |
//! |---|---|---|---|
//! | `1 + A × −` | [`crate::free_monad::list_endo::ListEndo<A>`] | `Option<A>` | `None → 0`, `Some(_) → 1` |
//! | `A + (−)²` | [`crate::free_monad::tree_endo::TreeEndo<A>`] | `Either<A, ()>` | `Left(_) → 0`, `Right(()) → 2` |
//! | `G × −` | [`crate::algebra::GroupActionEndo<G>`] | `G` | `_ → 1` |
//!
//! Each instance lives next to its witness definition (in `free_monad/` and
//! `algebra/`), keeping the object-map, morphism-map, and container
//! presentation of a witness in one module.

use crate::endofunctor::EndoWitness;

/// The **container** (shape/position) presentation of a polynomial endofunctor
/// (Abbott–Altenkirch–Ghani 2003, via CDL).
///
/// The witness is an [`EndoWitness`]; [`Container`] equips it with a `Shape`
/// set, a per-shape [`arity`](Self::arity), and the [`decompose`](Self::decompose)
/// / [`recompose`](Self::recompose) pair witnessing
/// `F(X) ≅ Σ_{s : Shape} X^{arity(s)}` in the finitary (`Vec`-of-contents)
/// presentation.
///
/// # Laws
///
/// For every `fx : F(X)` with `(s, xs) = decompose(fx)`, and every pure
/// morphism `f : X → Y`:
///
/// 1. **Round-trip**: `recompose(s, xs) == Some(fx)`.
/// 2. **Arity coherence**: `xs.len() == arity(&s)`, and `recompose(s', ys)`
///    returns `Some` **iff** `ys.len() == arity(&s')`.
/// 3. **`fmap` coherence**: `decompose(F::fmap(fx, f)) == (s, xs.map(f))` — the
///    shape is fixed and the contents are mapped in position order.
///
/// These are documented obligations, machine-checked for the shipped instances
/// in `tests/container_laws.rs`.
pub trait Container: EndoWitness {
    /// The shape set `S` of the container. `PartialEq + Debug` so the laws can
    /// be machine-checked (shape equality under `fmap`, decompose round-trip).
    type Shape: PartialEq + core::fmt::Debug;

    /// The number of positions at `shape` — the arity `|P(shape)|` of the
    /// finitary presentation. This is the exact length of the contents `Vec`
    /// that [`decompose`](Self::decompose) produces and that
    /// [`recompose`](Self::recompose) requires.
    fn arity(shape: &Self::Shape) -> usize;

    /// Split `fx : F(X)` into its shape and its contents in position order.
    ///
    /// The contents `Vec<X>` has length `arity(&shape)` (law 2).
    fn decompose<X>(fx: Self::Type<X>) -> (Self::Shape, Vec<X>);

    /// Reassemble `F(X)` from a shape and its position-ordered contents.
    ///
    /// Returns `None` when `contents.len() != arity(&shape)` — the only way the
    /// finitary presentation can fail to reconstruct a value (law 2).
    fn recompose<X>(shape: Self::Shape, contents: Vec<X>) -> Option<Self::Type<X>>;
}
