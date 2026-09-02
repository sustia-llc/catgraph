//! Full RNN / Mealy cell — coalgebra of `Para(I → O × −)`.
//!
//! CDL Ex I.4 / Ex J.4. Carrier `S`; `cell : (P, S) → (I → (O, S))`.
//! [`MealyCell::run`]: `run(s_0, [i_1, …, i_n]) = [o_1, …, o_n]` with
//! `(o_k, s_k) = (cell(p, s_{k−1}))(i_k)`.

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
    /// `[o_1, …, o_n]` with `(o_k, s_k) = (cell(p, s_{k−1}))(i_k)` (CDL Remark
    /// H.6 / Ex J.4); `Step` is the per-step inner closure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use catgraph_dl::architectures::MealyCell;
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

    /// Lazy [`run`]: one Mealy step per input pulled, ending with `inputs`,
    /// borrowing `cell`; `.collect()` equals [`run`]`(s_0, inputs)`.
    ///
    /// # Panics
    ///
    /// After a caught panic in the cell or step, every further `.next()` panics.
    ///
    /// [`run`]: MealyCell::run
    ///
    /// # Examples
    ///
    /// ```
    /// # use catgraph_dl::architectures::MealyCell;
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
            // `None` state means a previous cell/step call panicked and the
            // unwind was caught — poisoned; panic rather than end silently.
            let s = state
                .take()
                .expect("MealyCell::run_iter poisoned: a previous cell/step call panicked");
            let p = cell.parameter.clone();
            let step = (cell.cell)((p, s));
            let (o, s_next) = step(i);
            state = Some(s_next);
            Some(o)
        })
    }
}
