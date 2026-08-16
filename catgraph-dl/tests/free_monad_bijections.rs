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
//! 6. `opt_in_depth_guard_boundary` — `catgraph_dl::depth`'s guard, which is a
//!    caller-facing service since #200 rather than something the bijection
//!    helpers call, accepts a carrier at `MAX_TREE_DEPTH` and rejects one cell
//!    deeper. Engineering, not a CDL law.
//! 7. `deep_spine_survives_every_carrier_operation` — the #200 regression pin.
//!    A `common::DEEP`-deep caterpillar (4 096, sixteen times the old ceiling
//!    and ~2× what a 2 MiB test thread can recurse through) is constructed,
//!    embedded, projected back, folded, cloned, compared, formatted and
//!    dropped. Every one of those aborted before the carriers' walks became
//!    iterative.

#![allow(clippy::float_cmp, clippy::single_match_else)]

mod common;

use catgraph_dl::DepthError;
use catgraph_dl::depth::{MAX_TREE_DEPTH, free_mnd_depth, guard_tree_depth, tree_depth};
use catgraph_dl::free_monad::list_endo::{ListEndo, free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{
    BinaryTree, TreeEndo, free_mnd_to_tree, tree_to_free_mnd,
};
use catgraph_dl::free_monad::{Cofree, Free, FreeView};
use catgraph_dl::{Container, Either};

use common::{DEEP, UnitEndo, spine_free_mnd, spine_tree};

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
/// canonically `Free::pure(())` — no `Suspend` cells.
#[test]
fn empty_list_is_pure_unit() {
    let f: Free<ListEndo<u32>, ()> = vec_to_free_mnd(Vec::new(), ());
    match f.as_view() {
        FreeView::Pure(()) => (),
        FreeView::Suspend(_) => panic!("empty Vec must encode to Free::pure(()), not Suspend"),
    }

    // And the round-trip from Pure(()) gives back (vec![], ()).
    let pure_unit: Free<ListEndo<u32>, ()> = Free::pure(());
    let (items, ()) = free_mnd_to_vec(pure_unit);
    assert!(items.is_empty(), "Pure(()) must decode to empty Vec");
}

/// CDL Example B.19. The explicit cons-cell tower for `[1, 2]` written by
/// hand using `Free::suspend` must decode to `vec![1, 2]`.
#[test]
fn cons_cell_explicit_structure_round_trips() {
    // Free::suspend(Some((1, Box(Free::suspend(Some((2, Box(Free::pure(())))))))))
    let inner: Free<ListEndo<u32>, ()> = Free::suspend(Some((2_u32, Box::new(Free::pure(())))));
    let outer: Free<ListEndo<u32>, ()> = Free::suspend(Some((1_u32, Box::new(inner))));

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

/// Issue #231's guard, in its post-#200 role: an **opt-in** service the
/// bijection helpers no longer call.
///
/// Not a CDL law — a depth check is engineering. What is asserted is the
/// boundary, and the fact that the helpers themselves are indifferent to it:
///
/// - a left caterpillar at exactly [`MAX_TREE_DEPTH`] passes the guard, and the
///   bijection **preserves depth** (`tree_depth` and `free_mnd_depth` agree);
/// - one cell deeper is refused with [`DepthError::TreeDepthExceeded`] carrying
///   the exact `{ depth, limit }` — while the same over-limit carrier still
///   round-trips through the (now infallible) helpers, which is the difference
///   #200 made.
///
/// `guard_tree_depth` borrows, so the rejected value is still usable afterwards
/// — the rejection-path drop hazard #231 had to document is gone.
#[test]
fn opt_in_depth_guard_boundary() {
    // --- at the limit: accepted, and depth-preserving --------------------
    let at_limit = spine_tree(MAX_TREE_DEPTH);
    assert_eq!(tree_depth(&at_limit), MAX_TREE_DEPTH);
    assert!(guard_tree_depth(&at_limit).is_ok());

    let encoded = tree_to_free_mnd(at_limit.clone());
    assert_eq!(
        free_mnd_depth(&encoded),
        MAX_TREE_DEPTH,
        "the B.20 bijection preserves structural depth"
    );
    let decoded = free_mnd_to_tree(encoded);
    assert_eq!(decoded, at_limit, "round trip survives at the limit");

    // --- one deeper: the guard rejects, the helpers do not ---------------
    let over = MAX_TREE_DEPTH + 1;
    let too_deep = spine_tree(over);
    match guard_tree_depth(&too_deep) {
        Err(DepthError::TreeDepthExceeded { depth, limit }) => {
            assert_eq!(depth, over);
            assert_eq!(limit, MAX_TREE_DEPTH);
        }
        other => panic!("guard_tree_depth: expected TreeDepthExceeded, got {other:?}"),
    }
    // The guard borrowed, so `too_deep` is still ours — and the helper takes it
    // happily, because it no longer recurses.
    assert_eq!(
        free_mnd_to_tree(tree_to_free_mnd(too_deep)),
        spine_tree(over),
        "an over-limit carrier still round-trips: the helpers are infallible since #200"
    );
    assert_eq!(free_mnd_depth(&spine_free_mnd(over)), over);
}

/// **The #200 regression pin.** Every carrier operation over a spine deep enough
/// to have aborted the process before the walks became iterative.
///
/// The fixture is a `common::DEEP` (32 768) left caterpillar — 128× the retired
/// `MAX_TREE_DEPTH`, and comfortably past every measured abort threshold (see
/// `DEEP`'s own docs for the bisected table: recursive drop glue survives 8 192
/// and aborts at 16 384; recursive `Debug` survives 4 096 and aborts at 8 192).
/// So each operation below is a stack overflow under the previous
/// implementation and a plain value here; the test deliberately runs on the
/// default 2 MiB test thread, since a fat-stacked thread would erase exactly
/// the margin it exists to pin.
///
/// A stack overflow aborts the whole harness rather than failing an assertion,
/// so the *failing* direction shows up as "test binary died / SIGSEGV", never a
/// red assertion. The *passing* direction therefore asserts real values —
/// depths, fold results, structural equality, formatted length — rather than
/// merely reaching the end of the function.
#[test]
fn deep_spine_survives_every_carrier_operation() {
    // Construction, and the two depth measures (already iterative pre-#200).
    let tree = spine_tree(DEEP);
    assert_eq!(tree_depth(&tree), DEEP);

    // `Clone` — derived and recursive before #200.
    let cloned = tree.clone();
    assert_eq!(tree_depth(&cloned), DEEP);

    // `PartialEq` — likewise.
    assert_eq!(
        tree, cloned,
        "a deep caterpillar compares equal to its clone"
    );
    assert_ne!(
        tree,
        spine_tree(DEEP - 1),
        "and unequal to a caterpillar one shorter"
    );

    // `Debug` — likewise. Pin the shape, not just the fact that it returned:
    // `DEEP` leaves means `DEEP - 1` `Node(`s and `DEEP` `Leaf(`s.
    let shown = format!("{tree:?}");
    assert_eq!(shown.matches("Node(").count(), DEEP - 1);
    assert_eq!(shown.matches("Leaf(").count(), DEEP);

    // The bijection, both directions — recursive and `Result`-returning before.
    let encoded = tree_to_free_mnd(cloned);
    assert_eq!(free_mnd_depth(&encoded), DEEP);

    // The carriers' own `==` and `{:?}`, on the `Free` side.
    assert_eq!(
        encoded,
        spine_free_mnd(DEEP),
        "the embedded spine equals the hand-built one"
    );
    assert_eq!(
        format!("{encoded:?}").matches("Suspend(").count(),
        2 * DEEP - 1
    );

    // `Free::fold` — the catamorphism, recursive before #200. Count the leaves.
    let leaves: usize = spine_free_mnd(DEEP).fold(
        // `Infallible` has no values; the `Pure` arm is statically unreachable.
        &|z: core::convert::Infallible| match z {},
        &|node: Either<u8, (usize, usize)>| match node {
            Either::Left(_) => 1,
            Either::Right((l, r)) => l + r,
        },
    );
    assert_eq!(leaves, DEEP, "the caterpillar has DEEP leaves");

    // Back to the tree carrier, and structural equality with the original.
    let decoded = free_mnd_to_tree(encoded);
    assert_eq!(decoded, tree, "the B.20 round trip survives at depth DEEP");

    // `Cofree::unfold` — the anamorphism, recursive before #200, and the one
    // carrier entry that never had a guard in front of it at all. Grow the same
    // caterpillar shape on the `Cofree` side, then let it drop.
    let grown: Cofree<TreeEndo<u8>, usize> = Cofree::unfold(DEEP, &|n: usize| {
        if n <= 1 {
            (n, Either::Left(0_u8))
        } else {
            (n, Either::Right((n - 1, 1)))
        }
    });
    assert_eq!(*grown.head(), DEEP);
    assert_eq!(
        TreeEndo::<u8>::contents(grown.tail()).len(),
        2,
        "the root of the grown spine is a node"
    );

    // Finally: drop. Each of these was a recursive `Box`-chain drop before #200
    // — the residual no pre-flight guard could reach, because rejecting a
    // by-value input did not save that input's own drop.
    drop(grown);
    drop(decoded);
    drop(tree);
}

/// A trivial endofunctor with `Type<X> = ()` — collapses to "no recursive
/// slot at all". Aliased onto the shared `common::UnitEndo` witness; the cofree
/// comonad over it degenerates to a single `head` value followed by trivial
/// `tail = ()`.
struct TrivialTag;
type TrivialEndo = UnitEndo<TrivialTag>;

/// CDL Proposition B.18 dual smoke test. Confirms `Cofree<TrivialEndo,
/// u32>` constructs cleanly under the GAT bound and that `head()` is
/// accessible. Compile-time check: the recursive `F::Type<Box<Self>>` field
/// works through the GAT projection without workaround.
#[test]
fn cofree_cmnd_smoke() {
    let c: Cofree<TrivialEndo, u32> = Cofree::new(42_u32, ());
    assert_eq!(*c.head(), 42);

    // The carriers deliberately ship no `Clone` (#93 owner decision, kept by
    // #222), so construct an equal value and compare structurally through the
    // opt-in `PartialEq` (`UnitEndo: EqFunctor`, `u32: PartialEq`) — which is
    // what this test certifies; the clone was never part of it.
    let c2: Cofree<TrivialEndo, u32> = Cofree::new(42_u32, ());
    assert_eq!(*c2.head(), 42);
    assert_eq!(c, c2);
}
