//! Unfolding RNN — coalgebra of `Para(O × −)`.
//!
//! CDL Example I.3. Carrier `S`; parametric coalgebra
//! `(P, ⟨cell_o, cell_n⟩) : S → O × S` (under `Para`):
//!
//! - `cell_o : P × S → O` — output projection.
//! - `cell_n : P × S → S` — next-state.
//!
//! Unrolling into `Stream(O)` (CDL Ex J.2 / App I.3): [`UnfoldingRnn::unroll_to_vec`]
//! materialises `[cell_o(p, s_0), …, cell_o(p, s_{n−1})]` with
//! `s_{k+1} = cell_n(p, s_k)`; [`UnfoldingRnn::unroll_iter`] is the same
//! sequence as an infinite `Iterator`.

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
    /// The first `depth` outputs: `[cell_o(p, s_0), …, cell_o(p, s_{depth−1})]`
    /// with `s_{k+1} = cell_n(p, s_k)` (CDL Remark H.6 / Ex J.2); empty for
    /// `depth = 0`.
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

    /// Infinite `Iterator` over `cell_o(p, s_k)`, `s_{k+1} = cell_n(p, s_k)`
    /// (CDL Ex J.2 / Remark H.6), borrowing `cell`; `.take(n)` equals
    /// [`unroll_to_vec`]`(s_0, n)`.
    ///
    /// # Panics
    ///
    /// After a caught panic in `cell_o`/`cell_n`, every further `.next()` panics.
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
        // an `Option` we `take` from and re-seed. On every successful step it
        // is re-seeded `Some` (the coalgebra is total), so the iterator never
        // terminates; `None` is reachable only after a caught panic — poisoned,
        // handled loudly below.
        let mut state = Some(initial_state);
        core::iter::from_fn(move || {
            // `state` is re-seeded `Some` at the end of every successful step,
            // so `None` here means a previous `cell_o`/`cell_n` call panicked
            // and the unwind was caught — the iterator is poisoned. Panic
            // loudly rather than masquerade as a cleanly exhausted stream.
            let s = state.take().expect(
                "UnfoldingRnn::unroll_iter poisoned: a previous cell_o/cell_n call panicked",
            );
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
