//! Containers `S ◁ P` (Abbott–Altenkirch–Ghani 2003, via CDL §4):
//! `⟦S ◁ P⟧(X) = Σ_{s : S} X^{P(s)}`, finitary presentation with positions
//! `0..arity(s)` and contents as a `Vec<X>` in position order.
//!
//! | Endofunctor | Witness | `Shape` | `arity` |
//! |---|---|---|---|
//! | `1 + A × −` | [`crate::free_monad::list_endo::ListEndo<A>`] | `Option<A>` | `None → 0`, `Some(_) → 1` |
//! | `A + (−)²` | [`crate::free_monad::tree_endo::TreeEndo<A>`] | `Either<A, ()>` | `Left(_) → 0`, `Right(()) → 2` |
//! | `G × −` | [`crate::algebra::GroupActionEndo<G>`] | `G` | `_ → 1` |
//! | `1 + −` | [`crate::endofunctor::OptionWitness`] | `bool` (`is_some`) | `false → 0`, `true → 1` |

use crate::endofunctor::EndoWitness;

/// Shape/position presentation of an [`EndoWitness`]: `Shape`,
/// [`arity`](Self::arity), [`decompose`](Self::decompose) /
/// [`recompose`](Self::recompose) witnessing `F(X) ≅ Σ_{s} X^{arity(s)}`, and
/// [`contents`](Self::contents) by reference. The carriers' recursion schemes,
/// `==` and `{:?}` bound on it.
///
/// Laws, for every `fx : F(X)` with `(s, xs) = decompose(fx)` and pure
/// `f : X → Y`:
///
/// 1. `recompose(s, xs) == Some(fx)`.
/// 2. `xs.len() == arity(&s)`; `recompose(s', ys)` is `Some` iff
///    `ys.len() == arity(&s')`.
/// 3. `decompose(F::fmap(fx, f)) == (s, xs.map(f))`.
/// 4. `contents(&fx)` yields references to the values of `decompose(fx)`, in
///    the same order.
pub trait Container: EndoWitness {
    /// The shape set `S` of the container.
    ///
    /// Deliberately **unbounded** since the v0.14.0 window: the law helper
    /// asks for `PartialEq + Debug` where it needs them, so a witness whose
    /// shape carries a label (`ListEndo<A>`'s `Option<A>`, `TreeEndo<A>`'s
    /// `Either<A, ()>`) stays a container for *every* `A` rather than only for
    /// comparable, printable ones. That matters because `Free`/`Cofree` bound
    /// their walks on this trait.
    type Shape;

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

    /// Borrow the contents of `fx` in position order, leaving `fx` intact.
    ///
    /// The non-consuming half of [`decompose`](Self::decompose): same values,
    /// same order, no shape (law 4). It is what lets a *borrowing* walk — the
    /// carriers' `==` and `{:?}` — visit a spine iteratively, where `decompose`
    /// would have to take ownership.
    fn contents<X>(fx: &Self::Type<X>) -> Vec<&X>;
}
