//! Neural architectures as (co)algebra unrollers (Gavranović et al., ICML 2024,
//! CDL Appendices I & J).
//!
//! Each recurrent architecture is a *cell* (a `Para` structure map) plus an
//! **unrolling** — the unique (co)algebra homomorphism between an
//! (co)inductive carrier and that cell. This example runs the five wrappers:
//!
//! - **algebra direction** (fold an inductive structure into a value):
//!   [`FoldingRnn`] over lists, [`RecursiveNn`] over binary trees;
//! - **coalgebra direction** (unfold a state into an observed sequence):
//!   [`UnfoldingRnn`] (state-driven), [`MealyCell`] and [`MooreCell`]
//!   (input-driven).
//!
//! It closes with the CDL Example 2.6 headline: a `Z2`-invariant folding cell
//! makes `unroll` invariant under the negation action — Geometric Deep Learning
//! recovered at the architecture level.
//!
//! Run with `cargo run -p catgraph-dl --example architecture_unrollers`.

use catgraph_dl::architectures::{FoldingRnn, MealyCell, MooreCell, RecursiveNn, UnfoldingRnn};
use catgraph_dl::free_monad::tree_endo::BinaryTree;

fn main() {
    // ---- FoldingRnn: right-fold a list (algebra on `1 + A × −`) ----------
    //
    // cell_0(p) = p (the seed), cell_1((p, a, s)) = a + s + p (bias-added sum).
    type Cell0 = fn(i64) -> i64;
    type Cell1 = fn((i64, i64, i64)) -> i64;
    let folder: FoldingRnn<i64, i64, Cell0, Cell1, i64> =
        FoldingRnn::new(10_i64, |p| p, |(p, a, s)| a + s + p);
    assert_eq!(FoldingRnn::unroll(&folder, vec![1, 2, 3]), 46); // 10 + 3*10 + 6
    assert_eq!(FoldingRnn::unroll(&folder, vec![]), 10); // empty → seed
    println!("FoldingRnn: sum-with-bias unroll([1,2,3]) = 46, unroll([]) = 10");

    // ---- RecursiveNn: post-order walk of a tree (algebra on `A + (−)²`) --
    //
    // cell_0(p) = p at leaves, cell_1((_p, _a, l, r)) = l + r + 1 counts internal
    // nodes. Node(Node(Leaf, Leaf), Leaf) → 3 leaves + 2 combines = 5.
    type TCell0 = fn(i64) -> i64;
    type TCell1 = fn((i64, u8, i64, i64)) -> i64;
    let tree_net: RecursiveNn<i64, i64, TCell0, TCell1, u8> =
        RecursiveNn::new(1_i64, |p| p, |(_p, _a, l, r)| l + r + 1);
    let tree = BinaryTree::node(
        BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
        BinaryTree::leaf(3_u8),
    );
    assert_eq!(RecursiveNn::unroll(&tree_net, tree), 5);
    println!("RecursiveNn: node-count unroll(Node(Node(L,L),L)) = 5");

    // ---- UnfoldingRnn: state-driven unfold (coalgebra) ------------------
    //
    // cell_o((_p, s)) = s emits the state, cell_n((_p, s)) = s + 1 advances it.
    // A bounded counter from a seed.
    type UCellO = fn((i64, i64)) -> i64;
    type UCellN = fn((i64, i64)) -> i64;
    let counter: UnfoldingRnn<i64, i64, UCellO, UCellN, i64> =
        UnfoldingRnn::new(0_i64, |(_p, s)| s, |(_p, s)| s + 1);
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&counter, 0, 5),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&counter, 0, 0),
        Vec::<i64>::new()
    );
    println!("UnfoldingRnn: counter unroll(seed=0, depth=5) = [0,1,2,3,4]");

    // ---- MealyCell: input-driven, output depends on input --------------
    //
    // cell((_p, s)) = |i| (s + i, s + 1): emit s+i, then advance s. Mealy output
    // is a function of state *and* the current input.
    let mealy: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (s + i, s + 1));
    assert_eq!(
        MealyCell::run(&mealy, 0, vec![10, 20, 30]),
        vec![10, 21, 32]
    );
    println!("MealyCell: stateful counter run(0, [10,20,30]) = [10,21,32]");

    // ---- MooreCell: input-driven, output depends on state only ----------
    //
    // cell_o((_p, s)) = 2s emits *before* stepping; cell_n((_p, s, _i)) = s + 1.
    // The first output comes from the initial state — the Moore-vs-Mealy tell.
    type MCellO = fn(((), i64)) -> i64;
    type MCellN = fn(((), i64, ())) -> i64;
    let moore: MooreCell<(), i64, MCellO, MCellN, (), i64> =
        MooreCell::new((), |((), s)| s * 2, |((), s, ())| s + 1);
    assert_eq!(MooreCell::run(&moore, 0, vec![(); 3]), vec![0, 2, 4]);
    println!("MooreCell: output-then-step run(0, 3 inputs) = [0,2,4]");

    // ---- GDL recovery: a Z2-invariant fold is negation-invariant --------
    //
    // CDL Example 2.6. The aggregator (p, a, s) ↦ s + |a| is invariant under the
    // pointwise Z2 action a ↦ -a, so the whole unroll is too.
    type ZCell0 = fn(()) -> i64;
    type ZCell1 = fn(((), i64, i64)) -> i64;
    let invariant: FoldingRnn<(), i64, ZCell0, ZCell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| s + a.abs());
    let signed = vec![1_i64, -2, 3];
    let negated: Vec<i64> = signed.iter().map(|v| -v).collect();
    assert_eq!(
        FoldingRnn::unroll(&invariant, signed.clone()),
        FoldingRnn::unroll(&invariant, negated),
    );
    // A non-invariant cell (s + a) discriminates the two, confirming the point.
    let plain: FoldingRnn<(), i64, ZCell0, ZCell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| s + a);
    assert_ne!(
        FoldingRnn::unroll(&plain, signed.clone()),
        FoldingRnn::unroll(&plain, signed.iter().map(|v| -v).collect()),
    );
    println!(
        "GDL recovery: |·|-invariant unroll([1,-2,3]) == unroll([-1,2,-3]); plain sum differs"
    );

    println!("architecture_unrollers: all assertions passed");
}
