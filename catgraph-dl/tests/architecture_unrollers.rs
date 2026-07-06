//! Architecture-as-(co)algebra **unroller** acceptance tests.
//!
//! CDL Appendix I + Appendix J. The five neural-network architectures
//! `FoldingRnn`, `RecursiveNn`, `UnfoldingRnn`, `MealyCell`, `MooreCell`
//! — typed wrappers over ParaMorphism / FreeMnd / FAlgebra surfaces —
//! carry **unrolling methods**: the unique algebra (resp. coalgebra)
//! homomorphism between the relevant inductive (resp. coinductive)
//! carrier and the cell's structure map.
//!
//! Each test consolidates several related assertions in a single
//! function (per project TDD convention — quality over quantity).
//!
//! ## Tests in this file
//!
//! 1. `folding_rnn_sum_with_bias` — right-fold semantics; arithmetic
//!    cell `(p, a, s) ↦ a + s + p`. Exercises both the empty-input
//!    base case and a hand-computed length-3 case.
//! 2. `folding_rnn_length` — degenerate cell `(_p, _a, s) ↦ s + 1`;
//!    asserts `unroll([…n elems]) = n` for several lengths.
//! 3. `recursive_nn_constant_leaf_and_combine` — post-order walk of a
//!    depth-3 [`BinaryTree`] with arithmetic combine; hand-computed
//!    expected values across three trees.
//! 4. `unfolding_rnn_counter_bounded_depth` — counter `cell_o = id,
//!    cell_n = +1`; asserts depth-`n` prefix and `depth = 0`.
//! 5. `mealy_passthrough` — `cell((_p, s)) = |i| (i, s)`; output =
//!    input, state ignored.
//! 6. `mealy_stateful_counter` — emits `s + i`, increments state per
//!    step; hand-computed `[10, 21, 32]`.
//! 7. `moore_output_then_step` — `cell_o(s) = 2s`, `cell_n(s, _i) =
//!    s + 1`; asserts `[0, 2, 4]` (Moore output-then-step).
//! 8. `gdl_recovery_via_z2_invariant_folding` — the headline GDL test:
//!    a `Z2`-invariant aggregator `(p, a, s) ↦ s + |a|` makes
//!    `FoldingRnn::unroll` invariant under the negation action on
//!    `Vec<i64>` — assertion of equality between `unroll([1, -2, 3])`
//!    and `unroll([-1, 2, -3])`.
//! 9. `folding_rnn_equivalent_to_free_mnd_unroller` — the
//!    `FreeMnd`-equivalence test: `unroll(cell, vec) ==
//!    unroll_via_free_mnd(cell, vec_to_free_mnd(vec, ()))`. Local
//!    helper `unroll_via_free_mnd` walks the
//!    `FreeMnd<ListEndo<i64>, ()>` cons-cell tower — proves the
//!    unroller IS the unique algebra hom from the initial algebra of
//!    the free monad on `1 + A × −`.
//! 10. `recursive_nn_equivalent_to_free_mnd_unroller` — same,
//!     dual-direction check for trees: `unroll(cell, tree) ==
//!     unroll_via_free_mnd(cell, tree_to_free_mnd(tree))`.
//!
//! Total: 10 tests (the spec asked for at least 8).

#![allow(
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::items_after_statements,
    clippy::doc_markdown,
    reason = "Test file. type_complexity: the FoldingRnn<…5 type params…> spelling is exactly what callers see — a `type` alias would still need every parameter. items_after_statements: helper `fn`s nested inside tests are scoped to that test by intent. doc_markdown: backtick-wrapping every CDL type name in module-level prose is busywork; the in-line doc comments on individual tests already use backticks where load-bearing. Same precedent as Agent D's tests/algebra_homomorphisms.rs module-level allows."
)]

use catgraph_dl::Either;
use catgraph_dl::architectures::{FoldingRnn, MealyCell, MooreCell, RecursiveNn, UnfoldingRnn};
use catgraph_dl::free_monad::FreeMnd;
use catgraph_dl::free_monad::list_endo::{ListEndo, free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{BinaryTree, TreeEndo, tree_to_free_mnd};

use proptest::prelude::*;

/// List-direction cell types (`FoldingRnn` over `1 + A × −`). Module-level so
/// both the deterministic test and the proptest variant share the
/// `FreeMnd`-walk helper below.
type ListCell0 = fn(()) -> i64;
type ListCell1 = fn(((), i64, i64)) -> i64;

/// Tree-direction cell types (`RecursiveNn` over `A + (−)²`).
type TreeCell0 = fn(i64) -> i64;
type TreeCell1 = fn((i64, u8, i64, i64)) -> i64;

/// Walk the cons-cell tower of `FreeMnd<ListEndo<A>, ()>`, applying the
/// folding cell — the unique algebra hom from `(FreeMnd, structure_map)` into
/// `(S, [cell_0, cell_1])`. CDL Remark 2.13 / Prop B.18.
///
/// Destructs through the canonical [`free_mnd_to_vec`] (which panics loudly on
/// a non-canonical `Roll(None)` cell — the src contract; all inputs here come
/// from `vec_to_free_mnd`, which never emits one), then right-folds the
/// cons-order items through `cell_1` against the `cell_0` seed.
fn unroll_list_via_free_mnd(
    cell: &FoldingRnn<(), i64, ListCell0, ListCell1, i64>,
    free_mnd: FreeMnd<ListEndo<i64>, ()>,
) -> i64 {
    let (items, ()) = free_mnd_to_vec(free_mnd);
    let seed = (cell.cell_0)(());
    items
        .into_iter()
        .rev()
        .fold(seed, |s, a| (cell.cell_1)(((), a, s)))
}

/// Walk `FreeMnd<TreeEndo<A>, Infallible>` directly, applying the recursive
/// cell — the unique algebra hom for the tree direction. Recursive (matches
/// `tree_endo::free_mnd_to_tree`'s discipline). `Roll(Left(a))` (leaves) →
/// `cell_0`; `Roll(Right((l, r)))` (internal nodes) → recurse both subtrees,
/// then `cell_1`. CDL Remark 2.13 / Prop B.18.
fn unroll_tree_via_free_mnd(
    cell: &RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8>,
    free_mnd: FreeMnd<TreeEndo<u8>, core::convert::Infallible>,
) -> i64 {
    match free_mnd {
        FreeMnd::Pure(z) => match z {}, // Infallible: unreachable.
        FreeMnd::Roll(boxed) => match *boxed {
            Either::Left(_a) => (cell.cell_0)(cell.parameter),
            Either::Right((l, r)) => {
                let leftmost = leftmost_leaf_payload(&l);
                let l_val = unroll_tree_via_free_mnd(cell, l);
                let r_val = unroll_tree_via_free_mnd(cell, r);
                (cell.cell_1)((cell.parameter, leftmost, l_val, r_val))
            }
        },
    }
}

/// Find the leftmost leaf payload of a `FreeMnd<TreeEndo<u8>, Infallible>`.
/// Mirrors `RecursiveNn::leftmost_leaf` on the `BinaryTree` carrier.
fn leftmost_leaf_payload(t: &FreeMnd<TreeEndo<u8>, core::convert::Infallible>) -> u8 {
    let mut current = t;
    loop {
        match current {
            FreeMnd::Pure(z) => match *z {},
            FreeMnd::Roll(boxed) => match boxed.as_ref() {
                Either::Left(a) => return *a,
                Either::Right((l, _r)) => current = l,
            },
        }
    }
}

/// Bounded-depth `BinaryTree<u8>` strategy for the tree-direction proptest.
/// Depth ≤ 4, ≤ 16 nodes — matches the `with_cases(64)` budget.
fn arb_binary_tree() -> impl Strategy<Value = BinaryTree<u8>> {
    any::<u8>().prop_map(BinaryTree::leaf).prop_recursive(
        4,  // max recursion depth
        16, // max total nodes
        2,  // expected branching factor
        |inner| (inner.clone(), inner).prop_map(|(l, r)| BinaryTree::node(l, r)),
    )
}

/// **Test 1 — `FoldingRnn::unroll` right-fold semantics.**
///
/// CDL Example 2.12. With `cell_0(p) = p`, `cell_1((p, a, s)) = a + s + p`,
/// and `inputs = [1, 2, 3]`, `p = 10`:
///
/// ```text
/// s_0 = cell_0(10)              = 10
/// s_1 = cell_1((10, 3, 10))     = 3 + 10 + 10 = 23
/// s_2 = cell_1((10, 2, 23))     = 2 + 23 + 10 = 35
/// s_3 = cell_1((10, 1, 35))     = 1 + 35 + 10 = 46
/// ```
///
/// Total = `(initial_p) + n * p + sum(inputs) = 10 + 30 + 6 = 46`.
///
/// Empty-input edge case: `unroll([]) = cell_0(p) = 10`.
#[test]
fn folding_rnn_sum_with_bias() {
    type Cell0 = fn(i64) -> i64;
    type Cell1 = fn((i64, i64, i64)) -> i64;
    let cell: FoldingRnn<i64, i64, Cell0, Cell1, i64> =
        FoldingRnn::new(10_i64, |p| p, |(p, a, s)| a + s + p);

    // Hand-computed length-3 case.
    let result = FoldingRnn::unroll(&cell, vec![1_i64, 2, 3]);
    assert_eq!(
        result, 46,
        "right-fold sum-with-bias on [1, 2, 3] with p = 10"
    );

    // Empty input: only `cell_0(p)` fires.
    let empty = FoldingRnn::unroll(&cell, Vec::<i64>::new());
    assert_eq!(empty, 10, "empty input collapses to cell_0(p)");

    // Singleton: cell_1((10, 5, 10)) = 5 + 10 + 10 = 25.
    let single = FoldingRnn::unroll(&cell, vec![5_i64]);
    assert_eq!(single, 25, "singleton input: cell_1(10, 5, cell_0(10))");
}

/// **Test 2 — `FoldingRnn::unroll` length-counter.**
///
/// `cell_0(_p) = 0`, `cell_1((_p, _a, s)) = s + 1`. Unroll on a `Vec`
/// of length `n` gives `n`. Sweeps several lengths.
#[test]
fn folding_rnn_length() {
    type Cell0 = fn(()) -> usize;
    type Cell1 = fn(((), i32, usize)) -> usize;
    let cell: FoldingRnn<(), usize, Cell0, Cell1, i32> =
        FoldingRnn::new((), |()| 0_usize, |((), _a, s)| s + 1);

    for n in [0_usize, 1, 5, 17, 100] {
        let inputs: Vec<i32> = (0..i32::try_from(n).expect("test length fits i32")).collect();
        let result = FoldingRnn::unroll(&cell, inputs);
        assert_eq!(result, n, "length-counter unroll on {n}-element vec");
    }
}

/// **Test 3 — `RecursiveNn::unroll` post-order walk on `BinaryTree`.**
///
/// CDL Example J.3. `cell_0(p) = p`, `cell_1((_p, _a, l, r)) = l + r + 1`
/// — count internal nodes (number of `Node`s in the tree).
///
/// Verified shapes:
///
/// - `Leaf(_)` → 1 (just `cell_0(1)`).
/// - `Node(Leaf, Leaf)` → `1 + 1 + 1 = 3` (two leaves, one combine).
/// - Depth-3 tree `Node(Node(Leaf, Leaf), Leaf)` → `(1 + 1 + 1) + 1 + 1
///   = 5` (three leaves, two combines).
#[test]
fn recursive_nn_constant_leaf_and_combine() {
    type Cell0 = fn(i64) -> i64;
    type Cell1 = fn((i64, u8, i64, i64)) -> i64;
    let cell: RecursiveNn<i64, i64, Cell0, Cell1, u8> =
        RecursiveNn::new(1_i64, |p| p, |(_p, _a, l, r)| l + r + 1);

    // Single leaf.
    let leaf = BinaryTree::leaf(7_u8);
    assert_eq!(
        RecursiveNn::unroll(&cell, leaf),
        1,
        "leaf collapses to cell_0(p)"
    );

    // Two leaves, one combine.
    let two_leaves = BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8));
    assert_eq!(
        RecursiveNn::unroll(&cell, two_leaves),
        3,
        "Node(Leaf, Leaf): 1 + 1 (cell_0 each) + 1 (cell_1) = 3"
    );

    // Depth-3 left-skewed tree: Node(Node(Leaf, Leaf), Leaf).
    let depth3 = BinaryTree::node(
        BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
        BinaryTree::leaf(3_u8),
    );
    assert_eq!(
        RecursiveNn::unroll(&cell, depth3),
        5,
        "Node(Node(L, L), L): 3 leaves + 2 combines = 5"
    );

    // Depth-3 right-skewed tree: Node(Leaf, Node(Leaf, Leaf)). Symmetry check.
    let depth3_right = BinaryTree::node(
        BinaryTree::leaf(4_u8),
        BinaryTree::node(BinaryTree::leaf(5_u8), BinaryTree::leaf(6_u8)),
    );
    assert_eq!(
        RecursiveNn::unroll(&cell, depth3_right),
        5,
        "Node(L, Node(L, L)): same shape sum"
    );
}

/// **Test 4 — `UnfoldingRnn::unroll_to_vec` counter.**
///
/// CDL Example J.2. `cell_o((_p, s)) = s`, `cell_n((_p, s)) = s + 1`.
/// `initial = 0, depth = 5` produces `[0, 1, 2, 3, 4]`.
///
/// Edge cases: `depth = 0` gives `[]`; `depth = 1` gives `[s_0]` only.
#[test]
fn unfolding_rnn_counter_bounded_depth() {
    type CellO = fn((i64, i64)) -> i64;
    type CellN = fn((i64, i64)) -> i64;
    let cell: UnfoldingRnn<i64, i64, CellO, CellN, i64> =
        UnfoldingRnn::new(0_i64, |(_p, s)| s, |(_p, s)| s + 1);

    // Headline: depth = 5 from initial = 0.
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, 0_i64, 5),
        vec![0_i64, 1, 2, 3, 4],
        "counter unroll [0..5] from initial state 0"
    );

    // depth = 0 → empty.
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, 0_i64, 0),
        Vec::<i64>::new(),
        "depth = 0 returns empty vec"
    );

    // depth = 1 from initial = 7 → [7].
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, 7_i64, 1),
        vec![7_i64],
        "depth = 1 returns just initial-state output"
    );

    // depth = 4 from initial = -2 → [-2, -1, 0, 1].
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, -2_i64, 4),
        vec![-2_i64, -1, 0, 1],
        "depth = 4 from negative seed"
    );
}

/// **Test 5 — `MealyCell::run` passthrough.**
///
/// `cell((_p, s)) = |i| (i, s)` — output the input as-is, ignore state
/// (state never changes since the inner closure returns the captured
/// `s`). Run on `[1, 2, 3]` gives `[1, 2, 3]`.
///
/// Empty-input edge: empty input → empty output.
#[test]
fn mealy_passthrough() {
    let cell: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (i, s));

    assert_eq!(
        MealyCell::run(&cell, 0_i64, vec![1_i64, 2, 3]),
        vec![1_i64, 2, 3],
        "passthrough Mealy: output = input"
    );
    assert_eq!(
        MealyCell::run(&cell, 999_i64, vec![-7_i64, 0, 42]),
        vec![-7_i64, 0, 42],
        "passthrough preserves arbitrary inputs"
    );
    assert_eq!(
        MealyCell::run(&cell, 0_i64, Vec::<i64>::new()),
        Vec::<i64>::new(),
        "empty input → empty output"
    );
}

/// **Test 6 — `MealyCell::run` stateful counter.**
///
/// `cell((_p, s)) = |i| (s + i, s + 1)`. Run on `[10, 20, 30]` with
/// `initial = 0`:
///
/// ```text
/// step 1: s = 0, i = 10 → (0 + 10, 0 + 1) = (10, 1)
/// step 2: s = 1, i = 20 → (1 + 20, 1 + 1) = (21, 2)
/// step 3: s = 2, i = 30 → (2 + 30, 2 + 1) = (32, 3)
/// ```
///
/// Output sequence: `[10, 21, 32]`.
#[test]
fn mealy_stateful_counter() {
    let cell: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (s + i, s + 1));

    assert_eq!(
        MealyCell::run(&cell, 0_i64, vec![10_i64, 20, 30]),
        vec![10_i64, 21, 32],
        "Mealy stateful counter: emit s+i, increment s"
    );

    // From a non-zero initial state.
    assert_eq!(
        MealyCell::run(&cell, 5_i64, vec![1_i64, 1, 1]),
        vec![6_i64, 7, 8],
        "From initial = 5: outputs shift accordingly"
    );
}

/// **Test 7 — `MooreCell::run` output-then-step.**
///
/// `cell_o((_p, s)) = s * 2`, `cell_n((_p, s, _i)) = s + 1`. Run with
/// `initial = 0`, `inputs = [(); 3]`:
///
/// ```text
/// step 0: emit cell_o(0) = 0;   advance s = 0 → 1.
/// step 1: emit cell_o(1) = 2;   advance s = 1 → 2.
/// step 2: emit cell_o(2) = 4;   advance s = 2 → 3.
/// ```
///
/// Output: `[0, 2, 4]`. The first output `0` is emitted from the
/// *initial* state — this is the Moore-vs-Mealy distinction (output is
/// a function of state alone, emitted before consuming the next input).
#[test]
fn moore_output_then_step() {
    type CellO = fn(((), i64)) -> i64;
    type CellN = fn(((), i64, ())) -> i64;
    let cell: MooreCell<(), i64, CellO, CellN, (), i64> =
        MooreCell::new((), |((), s)| s * 2, |((), s, ())| s + 1);

    assert_eq!(
        MooreCell::run(&cell, 0_i64, vec![(); 3]),
        vec![0_i64, 2, 4],
        "Moore output-then-step from initial 0 over 3 inputs"
    );

    // From initial = 10 over 4 inputs.
    assert_eq!(
        MooreCell::run(&cell, 10_i64, vec![(); 4]),
        vec![20_i64, 22, 24, 26],
        "From initial = 10: outputs are 2s for s = 10, 11, 12, 13"
    );

    // Empty input → empty output.
    assert_eq!(
        MooreCell::run(&cell, 7_i64, Vec::<()>::new()),
        Vec::<i64>::new(),
        "empty input → empty output"
    );
}

/// **Test 8 — GDL recovery via Z2-invariant folding.**
///
/// CDL Example 2.6 — Geometric Deep Learning recovery, applied at the
/// architecture level. A `Z2`-invariant cell `(p, a, s) ↦ s + |a|`
/// (absolute value) makes `FoldingRnn::unroll` *invariant under the
/// pointwise `Z2`-negation action on `Vec<i64>`*. Concretely:
///
/// ```text
/// unroll([1, -2, 3])  ==  unroll([-1, 2, -3])
/// ```
///
/// The lax-algebra-coherence acceptance test: the cell respects the
/// action on inputs because `|a| = |-a|`. This is the architecture-level
/// reflection of Agent D's `algebra_homomorphisms::absolute_value_is_z2_equivariant_homomorphism`.
///
/// Bonus: the *non*-invariant cell `(p, a, s) ↦ s + a` does *not*
/// satisfy the invariance — `unroll([1, -2, 3]) ≠ unroll([-1, 2, -3])`
/// in general — confirming the test discriminates.
#[test]
fn gdl_recovery_via_z2_invariant_folding() {
    // Z2-invariant cell: abs(a) is invariant under `a ↦ -a`.
    type Cell0 = fn(()) -> i64;
    type Cell1 = fn(((), i64, i64)) -> i64;
    let invariant_cell: FoldingRnn<(), i64, Cell0, Cell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| s + a.abs());

    let positive = vec![1_i64, -2, 3];
    let negated: Vec<i64> = positive.iter().map(|v| -v).collect();
    assert_eq!(
        FoldingRnn::unroll(&invariant_cell, positive.clone()),
        FoldingRnn::unroll(&invariant_cell, negated.clone()),
        "Z2-invariant cell: unroll([1, -2, 3]) MUST equal unroll([-1, 2, -3])"
    );

    // Concrete value: 0 + |3| + (|3|+|-2|) + (|3|+|-2|+|1|) = …
    // Actually right-fold with cell_1((_, a, s)) = s + |a|, seed 0:
    //   s_1 = cell_1(((), 3, 0)) = 0 + |3|  = 3
    //   s_2 = cell_1(((), -2, 3)) = 3 + |-2| = 5
    //   s_3 = cell_1(((), 1, 5)) = 5 + |1|  = 6
    assert_eq!(
        FoldingRnn::unroll(&invariant_cell, positive.clone()),
        6,
        "concrete Z2-invariant fold value: |1| + |-2| + |3| = 6"
    );

    // Discriminator: non-invariant cell distinguishes the two orderings.
    let non_invariant_cell: FoldingRnn<(), i64, Cell0, Cell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| s + a);
    let pos_result = FoldingRnn::unroll(&non_invariant_cell, positive);
    let neg_result = FoldingRnn::unroll(&non_invariant_cell, negated);
    assert_ne!(
        pos_result, neg_result,
        "non-invariant cell MUST distinguish [1, -2, 3] from [-1, 2, -3]"
    );
    // Sanity: pos = 1 + (-2) + 3 = 2; neg = -1 + 2 + (-3) = -2.
    assert_eq!(pos_result, 2);
    assert_eq!(neg_result, -2);
}

/// **Test 9 — `FreeMnd`-equivalence for `FoldingRnn`.** (List direction.)
///
/// CDL Remark 2.13 / Proposition B.18. The central structural claim of
/// CDL: the architecture unroller IS the unique algebra homomorphism
/// from the initial algebra of the free monad on the corresponding
/// endofunctor. We exhibit this concretely for `FoldingRnn`:
///
/// ```text
/// FoldingRnn::unroll(cell, vec)
///   ==
/// unroll_via_free_mnd(cell, vec_to_free_mnd(vec, ()))
/// ```
///
/// The right-hand side walks the cons-cell tower of
/// `FreeMnd<ListEndo<A>, ()>` directly, applying `cell.cell_1` at each
/// `Roll(Some((a, rest)))` and `cell.cell_0` at the `Pure(())`
/// terminator. It is structurally identical to the unique algebra hom
/// from `(FreeMnd, [Roll, Pure])` into `(S, [cell_1, cell_0])`.
///
/// If this equality holds across several samples, the unroller is
/// (acceptance-tested) the algebra homomorphism. Sweeps several lengths
/// including the empty list.
#[test]
fn folding_rnn_equivalent_to_free_mnd_unroller() {
    let cell: FoldingRnn<(), i64, ListCell0, ListCell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| a + s);

    for vec in [
        Vec::<i64>::new(),
        vec![1_i64],
        vec![1_i64, 2, 3],
        vec![5_i64, -7, 11, -13, 17],
        vec![0_i64, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ] {
        let direct = FoldingRnn::unroll(&cell, vec.clone());
        let via_free_mnd = unroll_list_via_free_mnd(&cell, vec_to_free_mnd(vec.clone(), ()));
        assert_eq!(
            direct, via_free_mnd,
            "FoldingRnn::unroll(cell, {vec:?}) MUST equal unroll_list_via_free_mnd(cell, vec_to_free_mnd({vec:?}, ()))"
        );
    }
}

/// **Test 10 — `FreeMnd`-equivalence for `RecursiveNn`.** (Tree
/// direction.)
///
/// Same shape as test 9 but for the binary-tree direction:
///
/// ```text
/// RecursiveNn::unroll(cell, tree)
///   ==
/// unroll_via_free_mnd(cell, tree_to_free_mnd(tree))
/// ```
///
/// The local helper walks `FreeMnd<TreeEndo<A>, Infallible>` through
/// `Roll(Left(a))` (leaves → `cell_0`) and `Roll(Right((l, r)))`
/// (internal nodes → recurse into both subtrees, then `cell_1`).
#[test]
fn recursive_nn_equivalent_to_free_mnd_unroller() {
    let cell: RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8> =
        RecursiveNn::new(1_i64, |p| p, |(_p, _a, l, r)| l + r + 1);

    for tree in [
        BinaryTree::leaf(7_u8),
        BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
        BinaryTree::node(
            BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
            BinaryTree::leaf(3_u8),
        ),
        BinaryTree::node(
            BinaryTree::leaf(4_u8),
            BinaryTree::node(BinaryTree::leaf(5_u8), BinaryTree::leaf(6_u8)),
        ),
    ] {
        let direct = RecursiveNn::unroll(&cell, tree.clone());
        let via_free_mnd = unroll_tree_via_free_mnd(&cell, tree_to_free_mnd(tree.clone()));
        assert_eq!(
            direct, via_free_mnd,
            "RecursiveNn::unroll(cell, {tree:?}) MUST equal unroll_tree_via_free_mnd(cell, tree_to_free_mnd({tree:?}))"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **`FreeMnd`-equivalence proptest — list direction.** CDL Remark 2.13 /
    /// Proposition B.18. `FoldingRnn::unroll` IS the unique algebra hom from the
    /// initial algebra of the free monad on `1 + A × −`: it agrees with the
    /// direct `FreeMnd`-tower walk on every generated `Vec<i64>` (≤ 16 elems).
    /// Lifts the caller-sampled test 9 to property-based.
    #[test]
    fn folding_rnn_free_mnd_equivalence_proptest(
        input in prop::collection::vec(any::<i64>(), 0..=16),
    ) {
        // `wrapping_add` so arbitrary i64 payloads can't overflow; both legs
        // use the *same* cell, so the equivalence is unaffected.
        let cell: FoldingRnn<(), i64, ListCell0, ListCell1, i64> =
            FoldingRnn::new((), |()| 0_i64, |((), a, s)| a.wrapping_add(s));
        let direct = FoldingRnn::unroll(&cell, input.clone());
        let via_free_mnd = unroll_list_via_free_mnd(&cell, vec_to_free_mnd(input, ()));
        prop_assert_eq!(direct, via_free_mnd);
    }

    /// **`FreeMnd`-equivalence proptest — tree direction.** CDL Remark 2.13 /
    /// Proposition B.18. `RecursiveNn::unroll` agrees with the direct
    /// `FreeMnd<TreeEndo, Infallible>` walk on every generated bounded
    /// `BinaryTree<u8>`. Lifts the caller-sampled test 10 to property-based.
    #[test]
    fn recursive_nn_free_mnd_equivalence_proptest(tree in arb_binary_tree()) {
        let cell: RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8> =
            RecursiveNn::new(1_i64, |p| p, |(_p, _a, l, r)| l + r + 1);
        let direct = RecursiveNn::unroll(&cell, tree.clone());
        let via_free_mnd = unroll_tree_via_free_mnd(&cell, tree_to_free_mnd(tree));
        prop_assert_eq!(direct, via_free_mnd);
    }
}
