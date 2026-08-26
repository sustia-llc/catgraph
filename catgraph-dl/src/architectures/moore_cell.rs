//! Moore cell — coalgebra of `Para(O × (I → −))`.
//!
//! CDL Example I.5. Carrier `S`; output independent of current input:
//!
//! - `cell_o : P × S → O` — output (no `I` dependency).
//! - `cell_n : P × S × I → S` — next-state.
//!
//! [`MooreCell::run`]: `run(s_0, [i_1, …, i_n]) = [o_0, …, o_{n−1}]`,
//! `o_k = cell_o(p, s_k)`, `s_{k+1} = cell_n(p, s_k, i_{k+1})` — output
//! before the input is consumed.

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
    /// `[o_0, …, o_{n−1}]` with `o_k = cell_o(p, s_k)`,
    /// `s_{k+1} = cell_n(p, s_k, i_{k+1})` (CDL Remark H.6 / Ex J.5); one
    /// output per input, the first from `initial_state`.
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

    /// Lazy [`run`]: one output per input pulled, `cell_o` before `cell_n`,
    /// borrowing `cell`; `.collect()` equals [`run`]`(s_0, inputs)`.
    ///
    /// # Panics
    ///
    /// After a caught panic in `cell_o`/`cell_n`, every further `.next()` panics.
    ///
    /// [`run`]: MooreCell::run
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
    /// let outputs: Vec<i64> = MooreCell::run_iter(&cell, 0, vec![(); 3]).collect();
    /// assert_eq!(outputs, vec![0, 2, 4]);
    /// ```
    pub fn run_iter<'a, It>(
        cell: &'a MooreCell<P, S, CellO, CellN, I, O>,
        initial_state: S,
        inputs: It,
    ) -> impl Iterator<Item = O> + 'a
    where
        It: IntoIterator<Item = I>,
        It::IntoIter: 'a,
    {
        // State is moved into `cell_n` each step, so it lives behind an
        // `Option` we `take` from and re-seed; it is `Some` on every step
        // where an input is still available.
        let mut state = Some(initial_state);
        let mut iter = inputs.into_iter();
        core::iter::from_fn(move || {
            let i = iter.next()?;
            // `None` state means a previous cell_o/cell_n call panicked and
            // the unwind was caught — poisoned; panic rather than end silently.
            let s = state
                .take()
                .expect("MooreCell::run_iter poisoned: a previous cell_o/cell_n call panicked");
            // Output FIRST — Moore-distinctive.
            let p_o = cell.parameter.clone();
            let s_for_o = s.clone();
            let o = (cell.cell_o)((p_o, s_for_o));
            // Then advance.
            let p_n = cell.parameter.clone();
            state = Some((cell.cell_n)((p_n, s, i)));
            Some(o)
        })
    }
}
