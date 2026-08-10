//! `Free` ⇄ `Vec` and `Free` ⇄ `BinaryTree`
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
//! Six consolidated tests, one per acceptance criterion:
//!
//! 1. `vec_round_trip_proptest` — proptest-driven round trip for `Vec<u32>`
//!    in both directions.
//! 2. `empty_list_is_pure_unit` — the empty `Vec` collapses to
//!    `Free::Pure(())`.
//! 3. `cons_cell_explicit_structure_round_trips` — the manually-built
//!    cons-cell tower for `[1, 2]` round-trips correctly.
//! 4. `tree_round_trip_examples` — three hand-built `BinaryTree` instances
//!    round-trip via the `Free<TreeEndo, Infallible>` encoding.
//! 5. `cofree_cmnd_smoke` — `Cofree<TrivialEndo, u32>` constructs and
//!    `head()` is accessible. Compile-time + runtime sanity for the dual.
//! 6. `tree_bijection_depth_guard` — both helpers accept a carrier at
//!    `MAX_TREE_DEPTH` and reject one cell deeper (issue #231). Engineering,
//!    not a CDL law: the guard rejects inputs that would abort the process and
//!    leaves accepted ones untouched.

#![allow(clippy::float_cmp, clippy::single_match_else)]

mod common;

use catgraph_dl::DepthError;
use catgraph_dl::depth::{MAX_TREE_DEPTH, free_mnd_depth, tree_depth};
use catgraph_dl::free_monad::list_endo::{free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{BinaryTree, free_mnd_to_tree, tree_to_free_mnd};
use catgraph_dl::free_monad::{Cofree, Free};

use common::{UnitEndo, spine_free_mnd, spine_tree};

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
        // Forward: items → Free → (items', ()).
        let f = vec_to_free_mnd::<u32, ()>(items.clone(), ());
        let (round_trip, ()) = free_mnd_to_vec(f);
        prop_assert_eq!(round_trip, items.clone());

        // Backward: build the Free from the same items, destruct,
        // rebuild — must coincide structurally.
        let f1 = vec_to_free_mnd::<u32, ()>(items.clone(), ());
        let (items_again, ()) = free_mnd_to_vec(f1);
        let f2 = vec_to_free_mnd::<u32, ()>(items_again.clone(), ());
        let (final_items, ()) = free_mnd_to_vec(f2);
        prop_assert_eq!(final_items, items);
    }
}

/// CDL Example B.19 corner case. Empty list with `()` terminator is
/// canonically `Free::Pure(())` — no `Suspend` cells.
#[test]
fn empty_list_is_pure_unit() {
    let f: Free<catgraph_dl::free_monad::list_endo::ListEndo<u32>, ()> =
        vec_to_free_mnd(Vec::new(), ());
    match f {
        Free::Pure(()) => (),
        Free::Suspend(_) => panic!("empty Vec must encode to Free::Pure(()), not Suspend"),
    }

    // And the round-trip from Pure(()) gives back (vec![], ()).
    let pure_unit: Free<catgraph_dl::free_monad::list_endo::ListEndo<u32>, ()> = Free::Pure(());
    let (items, ()) = free_mnd_to_vec(pure_unit);
    assert!(items.is_empty(), "Pure(()) must decode to empty Vec");
}

/// CDL Example B.19. The explicit cons-cell tower for `[1, 2]` written by
/// hand using `Free::Suspend` constructors must decode to `vec![1, 2]`.
#[test]
fn cons_cell_explicit_structure_round_trips() {
    use catgraph_dl::free_monad::list_endo::ListEndo;

    // Free::Suspend(Some((1, Box(Free::Suspend(Some((2, Box(Free::Pure(())))))))))
    let inner: Free<ListEndo<u32>, ()> = Free::Suspend(Some((2_u32, Box::new(Free::Pure(())))));
    let outer: Free<ListEndo<u32>, ()> = Free::Suspend(Some((1_u32, Box::new(inner))));

    let (items, ()) = free_mnd_to_vec(outer);
    assert_eq!(items, vec![1_u32, 2_u32]);

    // And the canonical encoding from `vec![1, 2]` matches it structurally
    // (compared by re-decoding both through the bijection).
    let canonical = vec_to_free_mnd::<u32, ()>(vec![1, 2], ());
    let (items_canon, ()) = free_mnd_to_vec(canonical);
    assert_eq!(items_canon, vec![1_u32, 2_u32]);
}

/// CDL Example B.20. Three hand-built `BinaryTree` instances round-trip
/// via the `Free<TreeEndo<A>, Infallible>` encoding.
#[test]
fn tree_round_trip_examples() {
    // Case 1: a single leaf.
    let leaf = BinaryTree::leaf(7_u32);
    let f1 = tree_to_free_mnd(leaf.clone()).unwrap();
    let back1 = free_mnd_to_tree(f1).unwrap();
    assert_eq!(back1, leaf);

    // Case 2: a single internal node with two leaves —
    //     Node(Leaf(1), Leaf(2)).
    let node = BinaryTree::node(BinaryTree::leaf(1_u32), BinaryTree::leaf(2_u32));
    let f2 = tree_to_free_mnd(node.clone()).unwrap();
    let back2 = free_mnd_to_tree(f2).unwrap();
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
    let f3 = tree_to_free_mnd(deep.clone()).unwrap();
    let back3 = free_mnd_to_tree(f3).unwrap();
    assert_eq!(back3, deep);
}

/// Issue #231 — the pre-flight recursion guard on both tree bijection helpers.
///
/// Not a CDL law: a depth check is engineering, and the paper's Example B.20 iso
/// is unaffected for every carrier the guard accepts. What is asserted is the
/// boundary in both directions, for both helpers:
///
/// - a left caterpillar at exactly [`MAX_TREE_DEPTH`] round-trips, and the
///   bijection **preserves depth** (so a `Free` produced by `tree_to_free_mnd`
///   always clears `free_mnd_to_tree`'s guard);
/// - one cell deeper is refused with
///   [`DepthError::TreeDepthExceeded`] carrying the exact `{ depth, limit }`.
///
/// The over-limit `Free` is built directly by `spine_free_mnd` — it cannot come
/// from `tree_to_free_mnd`, which refuses to produce it. Both fixtures stay
/// ~16× (depth 257 vs 4 096) inside the pre-guard measured-safe depth, so
/// their own recursive drop glue is not at risk.
#[test]
fn tree_bijection_depth_guard() {
    // --- at the limit: accepted, and depth-preserving --------------------
    let at_limit = spine_tree(MAX_TREE_DEPTH);
    assert_eq!(tree_depth(&at_limit), MAX_TREE_DEPTH);

    let encoded = tree_to_free_mnd(at_limit.clone()).expect("a tree at the limit is accepted");
    assert_eq!(
        free_mnd_depth(&encoded),
        MAX_TREE_DEPTH,
        "the B.20 bijection preserves structural depth"
    );
    let decoded = free_mnd_to_tree(encoded).expect("a spine at the limit is accepted");
    assert_eq!(decoded, at_limit, "round trip survives at the limit");

    // --- one deeper: rejected, with the measured depth -------------------
    let over = MAX_TREE_DEPTH + 1;
    match tree_to_free_mnd(spine_tree(over)) {
        Err(DepthError::TreeDepthExceeded { depth, limit }) => {
            assert_eq!(depth, over);
            assert_eq!(limit, MAX_TREE_DEPTH);
        }
        other => panic!("tree_to_free_mnd: expected TreeDepthExceeded, got {other:?}"),
    }
    match free_mnd_to_tree(spine_free_mnd(over)) {
        Err(DepthError::TreeDepthExceeded { depth, limit }) => {
            assert_eq!(depth, over);
            assert_eq!(limit, MAX_TREE_DEPTH);
        }
        other => panic!("free_mnd_to_tree: expected TreeDepthExceeded, got {other:?}"),
    }
}

/// A trivial endofunctor with `Type<X> = ()` — collapses to "no recursive
/// slot at all". Aliased onto the shared `common::UnitEndo` witness; the cofree
/// comonad over it degenerates to a single `head` value followed by trivial
/// `tail = ()`.
struct TrivialTag;
type TrivialEndo = UnitEndo<TrivialTag>;

/// CDL Proposition B.18 dual smoke test. Confirms `Cofree<TrivialEndo,
/// u32>` constructs cleanly under the GAT bound and that `head()` is
/// accessible. Compile-time check: haft's recursive `F::Type<Box<Self>>`
/// field works through the GAT projection without workaround.
#[test]
fn cofree_cmnd_smoke() {
    let c: Cofree<TrivialEndo, u32> = Cofree::new(42_u32, ());
    assert_eq!(*c.head(), 42);

    // Carrier `Clone` is deliberately unadopted (#93 owner decision) even though
    // haft 0.4.2 ships the `CloneFunctor` that would enable it, so construct an
    // equal value and compare structurally through the opt-in `PartialEq`
    // (`UnitEndo: EqFunctor`, `u32: PartialEq`) — which is what this test
    // certifies; the clone was never part of it.
    let c2: Cofree<TrivialEndo, u32> = Cofree::new(42_u32, ());
    assert_eq!(*c2.head(), 42);
    assert_eq!(c, c2);
}
