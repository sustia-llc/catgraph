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
//! Its dual, the **cofree comonad** `Cofree`, models `F`-branching streams
//! (`head` + an `F`-shape of tails). This example builds each by hand, checks the
//! bijections with their obvious carriers, and folds an [`FAlgebra`] over a free
//! monad (a *catamorphism*) — the operation the architecture unrollers generalise.
//!
//! Run with `cargo run -p catgraph-dl --example free_monad_basics`.

use catgraph_dl::algebra::FAlgebra;
use catgraph_dl::endofunctor::OptionWitness;
use catgraph_dl::free_monad::list_endo::{ListEndo, free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{BinaryTree, free_mnd_to_tree, tree_to_free_mnd};
use catgraph_dl::free_monad::{Cofree, Free};

fn main() {
    // ---- 1. The list free monad `FreeMnd(1 + A × −)` --------------------
    //
    // `Pure(z)` is the terminator; `Suspend(Some((a, Box(rest))))` is a cons cell
    // (haft boxes the recursion inside the functor hole). The empty list is
    // exactly `Pure(())`.
    let empty: Free<ListEndo<u32>, ()> = Free::Pure(());
    let (items, ()) = free_mnd_to_vec(empty);
    assert!(items.is_empty());

    // Build `[1, 2]` as an explicit cons tower and decode it.
    let inner: Free<ListEndo<u32>, ()> = Free::Suspend(Some((2_u32, Box::new(Free::Pure(())))));
    let tower: Free<ListEndo<u32>, ()> = Free::Suspend(Some((1_u32, Box::new(inner))));
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
    println!("tree free monad: BinaryTree ⇄ Free<TreeEndo> bijection round-trips");

    // ---- 3. The dual: cofree comonad (a bounded stream prefix) ----------
    //
    // `Cofree<OptionWitness, O>` is `head : O` + `tail : Option<Box<Self>>`; a
    // `None` tail terminates. haft's `Cofree` has private fields, so build with
    // `Cofree::new` and walk through `into_parts()`. Build the 2-element prefix
    // `1, 2` and walk it.
    let stream: Cofree<OptionWitness, i64> =
        Cofree::new(1_i64, Some(Box::new(Cofree::new(2_i64, None))));
    let mut observed = Vec::new();
    let mut cursor = Some(stream);
    while let Some(node) = cursor {
        let (head, tail) = node.into_parts();
        observed.push(head);
        cursor = tail.map(|boxed| *boxed);
    }
    assert_eq!(observed, vec![1_i64, 2]);
    println!("cofree comonad: Cofree<OptionWitness, _> prefix 1,2 walks to [1, 2]");

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

/// Fold a `Free<ListEndo<i64>, ()>` cons tower through an `FAlgebra`'s
/// structure map — the catamorphism (unique algebra hom out of the initial
/// algebra). Re-expressed via haft's [`Free::fold`]: the `pure_case` handles the
/// `Pure(())` terminator (the algebra's `None` case), and the `algebra` is the
/// structure map itself — `fold` walks the tree, threading the recursive result
/// into the `Option<(i64, i64)>` hole functorially. This is the payoff of the
/// haft carrier: the hand-written recursion collapses to the library fold.
fn cata_list<S>(free: Free<ListEndo<i64>, ()>, alg: &FAlgebra<ListEndo<i64>, i64, S>) -> i64
where
    S: Fn(Option<(i64, i64)>) -> i64,
{
    // `pure_case`: the `Pure(())` terminator maps to the algebra's `None` case.
    let pure_case = |()| (alg.structure_map)(None);
    // `algebra`: the `Suspend` node — an `Option<(i64, i64)>` with the recursive
    // result already folded into the hole — is exactly the structure map's input.
    let algebra = |shape: Option<(i64, i64)>| (alg.structure_map)(shape);
    free.fold(&pure_case, &algebra)
}
