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
//! ## Phase DL-2 Agent E — `run`
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
/// **Phase DL-1 scaffold:** opaque struct.
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
    /// CDL Remark 2.13 dual / Example J.4. The Mealy unfolding: each
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
}
