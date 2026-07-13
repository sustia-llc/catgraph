//! Free monads, cofree comonads, and folding an algebra over them
//! (Gavranović et al., ICML 2024, CDL Appendix B).
//!
//! The free monad on an endofunctor `F` is the datatype of finite `F`-shaped
//! trees with variables at the leaves — the *initial* `F`-algebra when the leaf
//! type is empty. Two standard instances:
//!
//! - `FreeMnd(1 + A × −)(Z) ≅ (Vec<A>, Z)` — a list with a `Z` terminator
//!   (`ListEndo`), and
//! - `FreeMnd(A + (−)²)(!) ≅ BinaryTree<A>` — a binary tree with `A`-leaves
//!   (`TreeEndo`; the empty leaf type `!` is `core::convert::Infallible`).
//!
//! Its dual, the **cofree comonad** `CofreeCmnd`, models `F`-branching streams
//! (`head` + an `F`-shape of tails). This example builds each by hand, checks the
//! bijections with their obvious carriers, and folds an [`FAlgebra`] over a free
//! monad (a *catamorphism*) — the operation the architecture unrollers generalise.
//!
//! Run with `cargo run -p catgraph-dl --example free_monad_basics`.

use catgraph_dl::algebra::FAlgebra;
use catgraph_dl::endofunctor::OptionWitness;
use catgraph_dl::free_monad::list_endo::{ListEndo, free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{BinaryTree, free_mnd_to_tree, tree_to_free_mnd};
use catgraph_dl::free_monad::{CofreeCmnd, FreeMnd};

fn main() {
    // ---- 1. The list free monad `FreeMnd(1 + A × −)` --------------------
    //
    // `Pure(z)` is the terminator; `Roll(Some((a, rest)))` is a cons cell. The
    // empty list is exactly `Pure(())`.
    let empty: FreeMnd<ListEndo<u32>, ()> = FreeMnd::pure(());
    let (items, ()) = free_mnd_to_vec(empty);
    assert!(items.is_empty());

    // Build `[1, 2]` as an explicit cons tower and decode it.
    let inner: FreeMnd<ListEndo<u32>, ()> = FreeMnd::roll(Some((2_u32, FreeMnd::pure(()))));
    let tower: FreeMnd<ListEndo<u32>, ()> = FreeMnd::roll(Some((1_u32, inner)));
    let (decoded, ()) = free_mnd_to_vec(tower);
    assert_eq!(decoded, vec![1_u32, 2]);

    // The `Vec` bijection round-trips (CDL Example B.19).
    let round: Vec<u32> = {
        let f = vec_to_free_mnd::<u32, ()>(vec![3, 1, 4, 1, 5], ());
        free_mnd_to_vec(f).0
    };
    assert_eq!(round, vec![3, 1, 4, 1, 5]);
    println!("list free monad: [] = Pure(()), cons tower decodes, Vec bijection round-trips");

    // ---- 2. The tree free monad `FreeMnd(A + (−)²)` ---------------------
    //
    // `BinaryTree<A>` round-trips through the `TreeEndo` encoding (CDL B.20).
    let tree = BinaryTree::node(
        BinaryTree::node(BinaryTree::leaf(1_u32), BinaryTree::leaf(2_u32)),
        BinaryTree::leaf(3_u32),
    );
    let back = free_mnd_to_tree(tree_to_free_mnd(tree.clone()));
    assert_eq!(back, tree);
    println!("tree free monad: BinaryTree ⇄ FreeMnd<TreeEndo> bijection round-trips");

    // ---- 3. The dual: cofree comonad (a bounded stream prefix) ----------
    //
    // `CofreeCmnd<OptionWitness, O>` is `head : O` + `tail : Option<Self>`; a
    // `None` tail terminates. Build the 2-element prefix `1, 2` and walk it.
    let stream: CofreeCmnd<OptionWitness, i64> =
        CofreeCmnd::new(1_i64, Some(CofreeCmnd::new(2_i64, None)));
    let mut observed = Vec::new();
    let mut cursor = Some(stream);
    while let Some(node) = cursor {
        observed.push(node.head);
        cursor = *node.tail;
    }
    assert_eq!(observed, vec![1_i64, 2]);
    println!("cofree comonad: CofreeCmnd<OptionWitness, _> prefix 1,2 walks to [1, 2]");

    // ---- 4. Fold an algebra over the free monad (a catamorphism) --------
    //
    // An `FAlgebra<ListEndo<i64>, i64, _>` is a carrier `i64` plus a structure
    // map `1 + i64 × i64 → i64`. Folding it over a cons tower is the unique
    // algebra homomorphism from the initial algebra (the free monad). Here the
    // sum algebra `None ↦ 0`, `Some((a, s)) ↦ a + s` computes the list sum.
    let sum_algebra =
        FAlgebra::<ListEndo<i64>, i64, _>::new(0_i64, |shape: Option<(i64, i64)>| match shape {
            None => 0,
            Some((a, s)) => a + s,
        });

    let list = vec_to_free_mnd::<i64, ()>(vec![10, 20, 12], ());
    let total = cata_list(list, &sum_algebra);
    assert_eq!(total, 42);
    println!("algebra fold: sum-algebra catamorphism over [10, 20, 12] = {total}");

    println!("free_monad_basics: all assertions passed");
}

/// Fold a `FreeMnd<ListEndo<i64>, ()>` cons tower through an `FAlgebra`'s
/// structure map — the catamorphism (unique algebra hom out of the initial
/// algebra). `Pure`/`Roll(None)` hit the algebra's `None` case; each cons cell
/// applies the structure map to `Some((head, fold(rest)))`.
fn cata_list<S>(free: FreeMnd<ListEndo<i64>, ()>, alg: &FAlgebra<ListEndo<i64>, i64, S>) -> i64
where
    S: Fn(Option<(i64, i64)>) -> i64,
{
    match free {
        FreeMnd::Pure(()) => (alg.structure_map)(None),
        FreeMnd::Roll(boxed) => match *boxed {
            None => (alg.structure_map)(None),
            Some((a, rest)) => (alg.structure_map)(Some((a, cata_list(rest, alg)))),
        },
    }
}
