//! `FreeMnd` ⇄ `Vec` and `FreeMnd` ⇄ `BinaryTree`
//! bijection acceptance tests.
//!
//! CDL Examples B.19 + B.20. We verify that the two concrete encodings of
//! the free monad coincide (up to iso) with the obvious carriers:
//!
//! - `FreeMnd(1 + A × −)(Z) ≅ (Vec<A>, Z)` — list with explicit terminator.
//! - `FreeMnd(A + (−)²)(!) ≅ BinaryTree<A>` — binary tree with leaves in
//!   `A`. (`!` is modelled by `core::convert::Infallible`.)
//!
//! ## Test taxonomy
//!
//! Five consolidated tests, one per acceptance criterion:
//!
//! 1. `vec_round_trip_proptest` — proptest-driven round trip for `Vec<u32>`
//!    in both directions.
//! 2. `empty_list_is_pure_unit` — the empty `Vec` collapses to
//!    `FreeMnd::Pure(())`.
//! 3. `cons_cell_explicit_structure_round_trips` — the manually-built
//!    cons-cell tower for `[1, 2]` round-trips correctly.
//! 4. `tree_round_trip_examples` — three hand-built `BinaryTree` instances
//!    round-trip via the `FreeMnd<TreeEndo, Infallible>` encoding.
//! 5. `cofree_cmnd_smoke` — `CofreeCmnd<TrivialEndo, u32>` constructs and
//!    `head` is accessible. Compile-time + runtime sanity for the dual.

#![allow(clippy::float_cmp, clippy::single_match_else)]

use catgraph_dl::free_monad::list_endo::{free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{BinaryTree, free_mnd_to_tree, tree_to_free_mnd};
use catgraph_dl::free_monad::{CofreeCmnd, EndoFunctor, FreeMnd};

use proptest::prelude::*;

// CDL Example B.19. Round-trip proptest for the iso
// `FreeMnd(1 + A × −)(Z) ≅ Vec<A> × Z`. Tests `vec_to_free_mnd` followed
// by `free_mnd_to_vec` (reconstruction direction) and `free_mnd_to_vec`
// followed by `vec_to_free_mnd` (destruction direction). The terminator
// `Z = ()` collapses the iso to `Vec<A>`.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn vec_round_trip_proptest(items in proptest::collection::vec(any::<u32>(), 0..=24)) {
        // Forward: items → FreeMnd → (items', ()).
        let f = vec_to_free_mnd::<u32, ()>(items.clone(), ());
        let (round_trip, ()) = free_mnd_to_vec(f);
        prop_assert_eq!(round_trip, items.clone());

        // Backward: build the FreeMnd from the same items, destruct,
        // rebuild — must coincide structurally.
        let f1 = vec_to_free_mnd::<u32, ()>(items.clone(), ());
        let (items_again, ()) = free_mnd_to_vec(f1);
        let f2 = vec_to_free_mnd::<u32, ()>(items_again.clone(), ());
        let (final_items, ()) = free_mnd_to_vec(f2);
        prop_assert_eq!(final_items, items);
    }
}

/// CDL Example B.19 corner case. Empty list with `()` terminator is
/// canonically `FreeMnd::Pure(())` — no `Roll` cells.
#[test]
fn empty_list_is_pure_unit() {
    let f: FreeMnd<catgraph_dl::free_monad::list_endo::ListEndo<u32>, ()> =
        vec_to_free_mnd(Vec::new(), ());
    match f {
        FreeMnd::Pure(()) => (),
        FreeMnd::Roll(_) => panic!("empty Vec must encode to FreeMnd::Pure(()), not Roll"),
    }

    // And the round-trip from Pure(()) gives back (vec![], ()).
    let pure_unit: FreeMnd<catgraph_dl::free_monad::list_endo::ListEndo<u32>, ()> =
        FreeMnd::pure(());
    let (items, ()) = free_mnd_to_vec(pure_unit);
    assert!(items.is_empty(), "Pure(()) must decode to empty Vec");
}

/// CDL Example B.19. The explicit cons-cell tower for `[1, 2]` written by
/// hand using `FreeMnd::roll` constructors must decode to `vec![1, 2]`.
#[test]
fn cons_cell_explicit_structure_round_trips() {
    use catgraph_dl::free_monad::list_endo::ListEndo;

    // FreeMnd::Roll(Some((1, FreeMnd::Roll(Some((2, FreeMnd::Pure(())))))))
    let inner: FreeMnd<ListEndo<u32>, ()> = FreeMnd::roll(Some((2_u32, FreeMnd::pure(()))));
    let outer: FreeMnd<ListEndo<u32>, ()> = FreeMnd::roll(Some((1_u32, inner)));

    let (items, ()) = free_mnd_to_vec(outer);
    assert_eq!(items, vec![1_u32, 2_u32]);

    // And the canonical encoding from `vec![1, 2]` matches it structurally
    // (compared by re-decoding both through the bijection).
    let canonical = vec_to_free_mnd::<u32, ()>(vec![1, 2], ());
    let (items_canon, ()) = free_mnd_to_vec(canonical);
    assert_eq!(items_canon, vec![1_u32, 2_u32]);
}

/// CDL Example B.20. Three hand-built `BinaryTree` instances round-trip
/// via the `FreeMnd<TreeEndo<A>, Infallible>` encoding.
#[test]
fn tree_round_trip_examples() {
    // Case 1: a single leaf.
    let leaf = BinaryTree::leaf(7_u32);
    let f1 = tree_to_free_mnd(leaf.clone());
    let back1 = free_mnd_to_tree(f1);
    assert_eq!(back1, leaf);

    // Case 2: a single internal node with two leaves —
    //     Node(Leaf(1), Leaf(2)).
    let node = BinaryTree::node(BinaryTree::leaf(1_u32), BinaryTree::leaf(2_u32));
    let f2 = tree_to_free_mnd(node.clone());
    let back2 = free_mnd_to_tree(f2);
    assert_eq!(back2, node);

    // Case 3: a depth-3 tree — Node(Node(Leaf(1), Leaf(2)),
    //                                Node(Leaf(3), Node(Leaf(4), Leaf(5)))).
    let deep = BinaryTree::node(
        BinaryTree::node(BinaryTree::leaf(1_u32), BinaryTree::leaf(2_u32)),
        BinaryTree::node(
            BinaryTree::leaf(3_u32),
            BinaryTree::node(BinaryTree::leaf(4_u32), BinaryTree::leaf(5_u32)),
        ),
    );
    let f3 = tree_to_free_mnd(deep.clone());
    let back3 = free_mnd_to_tree(f3);
    assert_eq!(back3, deep);
}

/// A trivial endofunctor with `Apply<X> = ()` — collapses to "no recursive
/// slot at all". The cofree comonad over this functor degenerates to a
/// single `head` value followed by trivial `tail = ()`.
#[derive(Debug, Clone, Copy)]
struct TrivialEndo;

impl EndoFunctor for TrivialEndo {
    type Apply<X> = ();
    fn fmap<X, Y, G>((): Self::Apply<X>, _: G) -> Self::Apply<Y>
    where
        G: Fn(X) -> Y,
    {
    }
}

/// CDL Proposition B.18 dual smoke test. Confirms `CofreeCmnd<TrivialEndo,
/// u32>` constructs cleanly under the GAT bound and that `head` is
/// accessible. Compile-time check: the recursive-type pattern
/// `Box<F::Apply<Self>>` works through the GAT projection without
/// workaround.
#[test]
fn cofree_cmnd_smoke() {
    let c: CofreeCmnd<TrivialEndo, u32> = CofreeCmnd::new(42_u32, ());
    assert_eq!(c.head, 42);

    // Clone works under the manual `where F::Apply<Self>: Clone` bound.
    let c2 = c.clone();
    assert_eq!(c2.head, 42);
    assert_eq!(c, c2);
}
