//! Folding RNN — algebra of `Para(1 + A × −)`.
//!
//! CDL Example I.1. Carrier `S` (hidden state); parametric map
//! `(P, cell) ∈ Para(Set)(1 + A × S, S)`. Via the iso
//! `P × (1 + A × S) ≅ P + P × A × S`, splits as:
//!
//! - `cell_0 : P → S` — initial hidden state.
//! - `cell_1 : P × A × S → S` — the recurrent cell.
//!
//! ## Phase DL-2 Agent E — `unroll`
//!
//! [`FoldingRnn::unroll`] is the unique algebra homomorphism
//! `(P, List(A)) → S` from the *initial* algebra of the free monad
//! `FreeMnd(1 + A × −) ≅ List(A)` (CDL Remark 2.13 / Example 2.12) into
//! the cell's algebra `(S, [cell_0, cell_1])`. Right-fold semantics:
//!
//! ```text
//! unroll([a_1, …, a_n]) = cell_1(p, a_1, cell_1(p, a_2, … cell_1(p, a_n, cell_0(p)) …))
//! ```
//!
//! Implemented as `inputs.into_iter().rev().fold(cell_0(p), step)` so the
//! *rightmost* input is consumed first (innermost call); the *leftmost*
//! becomes the outermost. This matches Haskell's standard `foldr`.

use core::marker::PhantomData;

/// A folding-RNN cell: algebra of `Para(1 + A × −)` on hidden-state `S`.
///
/// CDL Example I.1.
///
/// **Phase DL-1 scaffold:** opaque struct holding the parameter and the
/// pair of cell maps.
#[derive(Debug, Clone)]
pub struct FoldingRnn<P, S, Cell0, Cell1, A> {
    /// The parameter object `P`.
    pub parameter: P,
    /// The initial-hidden-state map `cell_0 : P → S`.
    pub cell_0: Cell0,
    /// The recurrent cell `cell_1 : P × A × S → S`.
    pub cell_1: Cell1,
    _phantom: PhantomData<(S, A)>,
}

impl<P, S, Cell0, Cell1, A> FoldingRnn<P, S, Cell0, Cell1, A> {
    /// Build a folding-RNN cell from its parameter and cell maps.
    pub fn new(parameter: P, cell_0: Cell0, cell_1: Cell1) -> Self {
        Self {
            parameter,
            cell_0,
            cell_1,
            _phantom: PhantomData,
        }
    }
}

impl<P, S, Cell0, Cell1, A> FoldingRnn<P, S, Cell0, Cell1, A>
where
    P: Clone,
    Cell0: Fn(P) -> S,
    Cell1: Fn((P, A, S)) -> S,
{
    /// Unroll the cell over a list of inputs, threading the parameter `p`.
    ///
    /// CDL Remark 2.13 / Example 2.12. This is the unique algebra
    /// homomorphism `(P, List(A)) → S` from the initial algebra of the
    /// free monad on `1 + A × −` into the cell's algebra. Concretely it is
    /// a right-fold:
    ///
    /// ```text
    /// unroll([a_1, …, a_n]) = cell_1(p, a_1, cell_1(p, a_2, … cell_1(p, a_n, cell_0(p)) …))
    /// ```
    ///
    /// The `rev()` on the input iterator is the implementation detail that
    /// realises right-fold semantics from `Iterator::fold`'s left-fold
    /// shape: with the input reversed, the rightmost CDL element `a_n` is
    /// consumed first against the seed `cell_0(p)` (i.e. lands in the
    /// innermost call); the leftmost `a_1` is consumed last (outermost).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // sum-with-bias: cell_0(p) = p, cell_1((p, a, s)) = a + s + p.
    /// let cell: FoldingRnn<i64, i64, fn(i64) -> i64, fn((i64, i64, i64)) -> i64, i64> =
    ///     FoldingRnn::new(10, |p| p, |(p, a, s)| a + s + p);
    /// assert_eq!(FoldingRnn::unroll(&cell, vec![1, 2, 3]), 46);
    /// ```
    pub fn unroll(cell: &FoldingRnn<P, S, Cell0, Cell1, A>, inputs: Vec<A>) -> S {
        let p = cell.parameter.clone();
        let seed = (cell.cell_0)(p);
        inputs
            .into_iter()
            .rev()
            .fold(seed, |s, a| (cell.cell_1)((cell.parameter.clone(), a, s)))
    }
}
