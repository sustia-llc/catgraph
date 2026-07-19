//! Moore cell — coalgebra of `Para(O × (I → −))`.
//!
//! CDL Example I.5. Carrier `S`; output independent of current input:
//!
//! - `cell_o : P × S → O` — output (no `I` dependency).
//! - `cell_n : P × S × I → S` — next-state.
//!
//! ## `run`
//!
//! [`MooreCell::run`] projects the unique coalgebra homomorphism into the
//! final Moore coalgebra `Moore_{O,I}`. Distinctive trait vs. Mealy:
//! **output happens BEFORE consuming the next input**. Concretely:
//!
//! ```text
//! run(s_0, [i_1, …, i_n]) = [o_0, o_1, …, o_{n−1}]
//! where o_k = cell_o(p, s_k)            (output is a function of state alone)
//!       s_{k+1} = cell_n(p, s_k, i_{k+1})
//! ```
//!
//! The first output `o_0 = cell_o(p, s_0)` is emitted *before any input
//! is consumed* — exactly the Moore vs. Mealy distinction.

use core::marker::PhantomData;

/// A Moore-cell: coalgebra of `Para(O × (I → −))`.
///
/// CDL Example I.5.
///
/// Opaque struct.
#[derive(Debug, Clone)]
pub struct MooreCell<P, S, CellO, CellN, I, O> {
    /// The parameter object `P`.
    pub parameter: P,
    /// The output map `cell_o : P × S → O` (no `I`).
    pub cell_o: CellO,
    /// The next-state map `cell_n : P × S × I → S`.
    pub cell_n: CellN,
    _phantom: PhantomData<(S, I, O)>,
}

impl<P, S, CellO, CellN, I, O> MooreCell<P, S, CellO, CellN, I, O> {
    /// Build a Moore cell from its parameter and cell maps.
    pub fn new(parameter: P, cell_o: CellO, cell_n: CellN) -> Self {
        Self {
            parameter,
            cell_o,
            cell_n,
            _phantom: PhantomData,
        }
    }
}

impl<P, S, CellO, CellN, I, O> MooreCell<P, S, CellO, CellN, I, O>
where
    P: Clone,
    S: Clone,
    CellO: Fn((P, S)) -> O,
    CellN: Fn((P, S, I)) -> S,
{
    /// Run the cell over a sequence of inputs from `initial_state`.
    ///
    /// CDL Remark H.6 / Example J.5 (iterated Moore; the cell itself is
    /// Example I.5). Moore semantics:
    ///
    /// ```text
    /// run(s_0, [i_1, …, i_n]) = [o_0, o_1, …, o_{n−1}]
    /// where o_k       = cell_o(p, s_k)
    ///       s_{k+1}   = cell_n(p, s_k, i_{k+1})
    /// ```
    ///
    /// **Output-then-step ordering:** at step `k` we read out `o_k =
    /// cell_o(p, s_k)` *before* consuming `i_{k+1}` to advance the state.
    /// This is the Moore signature — the output is a function of state
    /// alone, independent of the next input. Contrast Mealy
    /// ([`crate::architectures::MealyCell::run`]) where each input
    /// produces its own output and the inner closure has access to the
    /// input.
    ///
    /// The returned vector has length `inputs.len()`: there are exactly
    /// as many outputs as inputs, with the first output emitted from the
    /// initial state.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // cell_o(p, s) = s * 2; cell_n(p, s, _i) = s + 1.
    /// let cell: MooreCell<_, _, _, _, (), i64> = MooreCell::new(
    ///     (),
    ///     |((), s): ((), i64)| s * 2,
    ///     |((), s, _i): ((), i64, ())| s + 1,
    /// );
    /// assert_eq!(MooreCell::run(&cell, 0, vec![(); 3]), vec![0, 2, 4]);
    /// ```
    pub fn run(
        cell: &MooreCell<P, S, CellO, CellN, I, O>,
        initial_state: S,
        inputs: Vec<I>,
    ) -> Vec<O> {
        let mut out = Vec::with_capacity(inputs.len());
        let mut state = initial_state;
        for i in inputs {
            // Output FIRST — Moore-distinctive.
            let p_o = cell.parameter.clone();
            let s_for_o = state.clone();
            let o = (cell.cell_o)((p_o, s_for_o));
            out.push(o);
            // Then advance.
            let p_n = cell.parameter.clone();
            state = (cell.cell_n)((p_n, state, i));
        }
        out
    }
}
