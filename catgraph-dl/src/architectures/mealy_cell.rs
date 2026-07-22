//! Full RNN / Mealy cell — coalgebra of `Para(I → O × −)`.
//!
//! CDL Example I.4. Carrier `S`; parametric coalgebra
//! `(P, cell) : S → I → O × S`. Veličković: "recurrent neural networks
//! can be thought of as **learnable Mealy machines**, a perspective
//! seldom advocated for in the literature."
//!
//! Unrolling builds a `Mealy_{O,I}` element with shared parameter `P`
//! (CDL Example J.4).
//!
//! ## `run`
//!
//! [`MealyCell::run`] is the *stream-process* projection of the unique
//! coalgebra homomorphism into the final `Mealy_{O,I}` coalgebra. Semantics:
//!
//! ```text
//! run(s_0, [i_1, i_2, …, i_n]) = [o_1, o_2, …, o_n]
//! where (o_k, s_k) = (cell(p, s_{k−1}))(i_k)
//! ```
//!
//! State is threaded left-to-right through the input sequence; outputs
//! are collected in order.

use core::marker::PhantomData;

/// A Mealy-cell / full-RNN cell: coalgebra of `Para(I → O × −)`.
///
/// CDL Example I.4.
///
/// Opaque struct.
#[derive(Debug, Clone)]
pub struct MealyCell<P, S, Cell, I, O> {
    /// The parameter object `P`.
    pub parameter: P,
    /// The cell map `cell : P × S → I → O × S`.
    pub cell: Cell,
    _phantom: PhantomData<(S, I, O)>,
}

impl<P, S, Cell, I, O> MealyCell<P, S, Cell, I, O> {
    /// Build a Mealy cell from its parameter and cell map.
    pub fn new(parameter: P, cell: Cell) -> Self {
        Self {
            parameter,
            cell,
            _phantom: PhantomData,
        }
    }
}

impl<P, S, Cell, I, O> MealyCell<P, S, Cell, I, O>
where
    P: Clone,
{
    /// Run the cell over a sequence of inputs from `initial_state`,
    /// collecting outputs in order.
    ///
    /// CDL Remark H.6 / Example J.4 (iterated Mealy). The Mealy unfolding: each
    /// input step produces an output and a fresh state; state threads
    /// left-to-right through the sequence.
    ///
    /// ```text
    /// run(s_0, [i_1, …, i_n]) = [o_1, …, o_n]
    /// where (o_k, s_k) = (cell(p, s_{k−1}))(i_k)
    /// ```
    ///
    /// # Closure shape
    ///
    /// The cell is two-stage by CDL convention: outer `cell : (P, S) →
    /// (I → O × S)` returns a fresh per-step closure; the inner closure
    /// then consumes one input. We model the inner stage with a separate
    /// generic `Step: FnOnce(I) -> (O, S)` so each call site can use a
    /// fresh closure (the standard Rust workaround for "functions
    /// returning closures" without naming the closure type).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Stateful counter: emit s+i, increment s.
    /// let cell: MealyCell<_, _, _, i64, i64> = MealyCell::new((), |((), s): ((), i64)| {
    ///     move |i: i64| (s + i, s + 1)
    /// });
    /// assert_eq!(MealyCell::run(&cell, 0, vec![10, 20, 30]), vec![10, 21, 32]);
    /// ```
    pub fn run<Step>(cell: &MealyCell<P, S, Cell, I, O>, initial_state: S, inputs: Vec<I>) -> Vec<O>
    where
        Cell: Fn((P, S)) -> Step,
        Step: FnOnce(I) -> (O, S),
    {
        let mut out = Vec::with_capacity(inputs.len());
        let mut state = initial_state;
        for i in inputs {
            let p = cell.parameter.clone();
            let step = (cell.cell)((p, state));
            let (o, s_next) = step(i);
            out.push(o);
            state = s_next;
        }
        out
    }

    /// Lazily run the cell over any input iterable, yielding one output per
    /// input pulled — an `impl Iterator<Item = O>` that consumes `inputs` on
    /// demand.
    ///
    /// CDL Remark H.6 / Example J.4. The pull-based dual of [`run`]: pulling one
    /// item from `inputs` produces exactly one Mealy step (`(cell(p, s))(i)`),
    /// threading the state left-to-right, identical two-stage closure shape.
    /// `run_iter(s_0, inputs).collect()` equals [`run`]`(s_0, inputs)` for any
    /// `inputs` (empty included); the iterator is finite, ending when `inputs`
    /// is exhausted. It borrows `cell` for the lifetime of the returned
    /// iterator.
    ///
    /// [`run`]: MealyCell::run
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Stateful counter: emit s+i, increment s.
    /// let cell: MealyCell<_, _, _, i64, i64> = MealyCell::new((), |((), s): ((), i64)| {
    ///     move |i: i64| (s + i, s + 1)
    /// });
    /// let outputs: Vec<i64> = MealyCell::run_iter(&cell, 0, [10, 20, 30]).collect();
    /// assert_eq!(outputs, vec![10, 21, 32]);
    /// ```
    pub fn run_iter<'a, Step, It>(
        cell: &'a MealyCell<P, S, Cell, I, O>,
        initial_state: S,
        inputs: It,
    ) -> impl Iterator<Item = O> + 'a
    where
        Cell: Fn((P, S)) -> Step,
        Step: FnOnce(I) -> (O, S),
        It: IntoIterator<Item = I>,
        It::IntoIter: 'a,
    {
        // State is moved into each per-step closure, so it lives behind an
        // `Option` we `take` from and re-seed; it is `Some` on every step
        // where an input is still available.
        let mut state = Some(initial_state);
        let mut iter = inputs.into_iter();
        core::iter::from_fn(move || {
            let i = iter.next()?;
            let s = state.take()?;
            let p = cell.parameter.clone();
            let step = (cell.cell)((p, s));
            let (o, s_next) = step(i);
            state = Some(s_next);
            Some(o)
        })
    }
}
