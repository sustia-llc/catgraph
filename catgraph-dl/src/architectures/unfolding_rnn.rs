//! Unfolding RNN — coalgebra of `Para(O × −)`.
//!
//! CDL Example I.3. Carrier `S`; parametric coalgebra
//! `(P, ⟨cell_o, cell_n⟩) : S → O × S` (under `Para`):
//!
//! - `cell_o : P × S → O` — output projection.
//! - `cell_n : P × S → S` — next-state.
//!
//! Unrolling produces a `Stream(O)` (CDL Example J.2).
//!
//! ## `unroll_to_vec`
//!
//! [`UnfoldingRnn::unroll_to_vec`] is a *bounded-depth* approximation of
//! the unique coalgebra homomorphism into the *final* coalgebra
//! `Stream(O)`. We materialise the first `depth` elements:
//!
//! ```text
//! unroll_to_vec(s_0, n) = [cell_o(p, s_0), cell_o(p, s_1), …, cell_o(p, s_{n−1})]
//! where s_{k+1} = cell_n(p, s_k)
//! ```
//!
//! Infinite (lazy) unrolling is deferred and would need a `Lazy`
//! / `Thunk` carrier (or `tokio_stream::Stream`); see the closing CDL §3.2
//! remark on streams as final coalgebras.

use core::marker::PhantomData;

/// An unfolding-RNN cell: coalgebra of `Para(O × −)` on hidden-state `S`.
///
/// CDL Example I.3.
///
/// Opaque struct.
#[derive(Debug, Clone)]
pub struct UnfoldingRnn<P, S, CellO, CellN, O> {
    /// The parameter object `P`.
    pub parameter: P,
    /// The output map `cell_o : P × S → O`.
    pub cell_o: CellO,
    /// The next-state map `cell_n : P × S → S`.
    pub cell_n: CellN,
    _phantom: PhantomData<(S, O)>,
}

impl<P, S, CellO, CellN, O> UnfoldingRnn<P, S, CellO, CellN, O> {
    /// Build an unfolding-RNN cell from its parameter and cell maps.
    pub fn new(parameter: P, cell_o: CellO, cell_n: CellN) -> Self {
        Self {
            parameter,
            cell_o,
            cell_n,
            _phantom: PhantomData,
        }
    }
}

impl<P, S, CellO, CellN, O> UnfoldingRnn<P, S, CellO, CellN, O>
where
    P: Clone,
    S: Clone,
    CellO: Fn((P, S)) -> O,
    CellN: Fn((P, S)) -> S,
{
    /// Bounded-depth unroll into a `Vec<O>` of length `depth`.
    ///
    /// CDL Remark 2.13 dual / Example J.2. The unique coalgebra
    /// homomorphism into the final coalgebra `Stream(O)` is conceptually
    /// infinite; this method materialises a finite prefix. Semantics:
    ///
    /// ```text
    /// unroll_to_vec(s_0, n) = [cell_o(p, s_0), cell_o(p, s_1), …, cell_o(p, s_{n−1})]
    /// where s_{k+1} = cell_n(p, s_k)
    /// ```
    ///
    /// `depth = 0` returns the empty vector (no states observed).
    ///
    /// # Why bounded?
    ///
    /// Rust eagerly evaluates `Vec<O>`; the *true* final-coalgebra
    /// homomorphism would land in a lazy carrier. A future addition may add an
    /// `unroll_into_iter` returning `impl Iterator<Item = O>` for true
    /// streamy semantics, or a `tokio_stream::Stream` adapter once the
    /// crate gains async dependencies (currently it has none).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Counter: cell_o = identity, cell_n = +1.
    /// let cell: UnfoldingRnn<i64, i64, fn((i64, i64)) -> i64, fn((i64, i64)) -> i64, i64> =
    ///     UnfoldingRnn::new(0, |(_p, s)| s, |(_p, s)| s + 1);
    /// assert_eq!(UnfoldingRnn::unroll_to_vec(&cell, 0, 5), vec![0, 1, 2, 3, 4]);
    /// ```
    pub fn unroll_to_vec(
        cell: &UnfoldingRnn<P, S, CellO, CellN, O>,
        initial_state: S,
        depth: usize,
    ) -> Vec<O> {
        let mut out = Vec::with_capacity(depth);
        let mut state = initial_state;
        for _ in 0..depth {
            let p = cell.parameter.clone();
            let s_for_o = state.clone();
            let o = (cell.cell_o)((p, s_for_o));
            out.push(o);
            // Advance: s_{k+1} = cell_n(p, s_k).
            let p_n = cell.parameter.clone();
            state = (cell.cell_n)((p_n, state));
        }
        out
    }
}
