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
//! Lazy (on-demand) unrolling is provided by [`UnfoldingRnn::unroll_iter`],
//! which returns a genuinely infinite `impl Iterator<Item = O>`. A plain
//! pull-based Rust `Iterator` is the pragmatic lazy carrier for the
//! conceptually-infinite `Stream(O)` — no `Lazy` / `Thunk` carrier and no
//! async `tokio_stream::Stream` dependency are needed; callers bound the
//! stream with `.take(n)`. See the closing CDL §3.2 remark on streams as
//! final coalgebras.

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
    /// homomorphism lands in a lazy carrier. That carrier is
    /// [`UnfoldingRnn::unroll_iter`], which returns a genuinely infinite
    /// `impl Iterator<Item = O>` — the pragmatic lazy carrier for the
    /// conceptually-infinite `Stream(O)` (a pull-based Rust `Iterator`, no
    /// async dependency). `unroll_iter(s_0).take(n)` agrees with
    /// `unroll_to_vec(s_0, n)` elementwise.
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

    /// Lazily unroll into a **genuinely infinite** `impl Iterator<Item = O>`.
    ///
    /// CDL Example J.2 / Remark H.6. This is the lazy carrier the *true*
    /// final-coalgebra homomorphism into `Stream(O)` lands in: a pull-based
    /// Rust `Iterator` that steps the coalgebra on demand, emitting one output
    /// per `.next()` and threading the state exactly as [`unroll_to_vec`]
    /// does — same `(cell_o, cell_n)` output-then-advance sequencing:
    ///
    /// ```text
    /// unroll_iter(s_0) = [cell_o(p, s_0), cell_o(p, s_1), cell_o(p, s_2), …]
    /// where s_{k+1} = cell_n(p, s_k)
    /// ```
    ///
    /// The iterator never terminates on its own; callers **must** bound it —
    /// `unroll_iter(s_0).take(n)` yields the same sequence as
    /// [`unroll_to_vec`]`(s_0, n)` elementwise, for every `n` (`n = 0`
    /// included). It borrows `cell` for the lifetime of the returned iterator.
    ///
    /// [`unroll_to_vec`]: UnfoldingRnn::unroll_to_vec
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Counter: cell_o = identity, cell_n = +1.
    /// let cell: UnfoldingRnn<i64, i64, fn((i64, i64)) -> i64, fn((i64, i64)) -> i64, i64> =
    ///     UnfoldingRnn::new(0, |(_p, s)| s, |(_p, s)| s + 1);
    /// let first_five: Vec<i64> = UnfoldingRnn::unroll_iter(&cell, 0).take(5).collect();
    /// assert_eq!(first_five, vec![0, 1, 2, 3, 4]);
    /// assert_eq!(first_five, UnfoldingRnn::unroll_to_vec(&cell, 0, 5));
    /// ```
    pub fn unroll_iter(
        cell: &UnfoldingRnn<P, S, CellO, CellN, O>,
        initial_state: S,
    ) -> impl Iterator<Item = O> + '_ {
        // The state is moved out (into `cell_n`) each step, so it lives behind
        // an `Option` we `take` from and re-seed; it is `Some` on every call
        // (the coalgebra is total), so the iterator never terminates.
        let mut state = Some(initial_state);
        core::iter::from_fn(move || {
            let s = state.take()?;
            let p = cell.parameter.clone();
            let s_for_o = s.clone();
            let o = (cell.cell_o)((p, s_for_o));
            // Advance: s_{k+1} = cell_n(p, s_k).
            let p_n = cell.parameter.clone();
            state = Some((cell.cell_n)((p_n, s)));
            Some(o)
        })
    }
}
