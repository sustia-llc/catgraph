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
//! One consolidated test per acceptance criterion:
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
//! 7. `deep_spine_survives_carrier_operations` — the #200 regression pin.
//!    A `common::DEEP` fixture (**32 768** — 128× the retired
//!    `MAX_TREE_DEPTH`, 4× the deepest spine a recursive walk survived on a
//!    2 MiB test thread and 2× the shallowest that aborted; see `DEEP`'s own
//!    docs for the bisected table) is built on each of the three carriers and
//!    put through: `BinaryTree` `tree_depth`, `Clone`, `==`, `!=` and `{:?}`;
//!    `Free` `free_mnd_depth`, `==`, `{:?}` and `fold` on the branching
//!    witness, and `==`, `!=`, `{:?}` and `fold` on the list one; `Cofree`
//!    `unfold`, `head`, `tail`, the witness's `contents`, `==`, `!=` and
//!    `{:?}`; the B.19 and B.20 bijections in both directions; and drops —
//!    explicit for the `BinaryTree` and `Cofree` values, via consumption for
//!    the `Free` ones. `Clone` is `BinaryTree`'s alone — the other two
//!    carriers ship none.
//! 8. `free_mnd_to_vec_panics_on_bare_suspend_none` — the documented panic
//!    contract (#312) on a non-canonical `Free::suspend(None)` reaching
//!    `free_mnd_to_vec` with no `Pure` terminator above it. Engineering drift
//!    guard, not a CDL law: the canonical encoding never emits this shape, so
//!    the pin's value is in the panic message wording and the signature, not
//!    a live regression.

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

        // Backward: rebuild from the decoded items and compare structurally
        // (Free's own PartialEq). `Free` has no `Clone` (#93/#222), so `f1`
        // is built twice from `items` rather than cloned.
        let f1 = vec_to_free_mnd::<u32, ()>(items.clone(), ());
        let (items_again, ()) = free_mnd_to_vec(vec_to_free_mnd::<u32, ()>(items.clone(), ()));
        let f2 = vec_to_free_mnd::<u32, ()>(items_again, ());
        prop_assert_eq!(f2, f1);
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

    // The canonical encoding from `vec![1, 2]` must coincide with the
    // hand-built tower structurally (Free's own PartialEq), compared before
    // either side is consumed by decoding.
    let canonical = vec_to_free_mnd::<u32, ()>(vec![1, 2], ());
    assert_eq!(canonical, outer);

    let (items, ()) = free_mnd_to_vec(outer);
    assert_eq!(items, vec![1_u32, 2_u32]);

    let (items_canon, ()) = free_mnd_to_vec(canonical);
    assert_eq!(items_canon, vec![1_u32, 2_u32]);
}

/// Issue #312. The documented panic contract on a non-canonical
/// `Free::suspend(None)` reaching `free_mnd_to_vec` with no `Pure`
/// terminator above it. Engineering pin, not a CDL law: the canonical
/// encoding (`vec_to_free_mnd`) never emits this shape, so the pin's value
/// is in guarding the panic message wording and a future signature change —
/// a silent-return alternative does not compile today.
#[test]
#[should_panic(expected = "non-canonical Free value")]
fn free_mnd_to_vec_panics_on_bare_suspend_none() {
    let bare: Free<ListEndo<u32>, ()> = Free::suspend(None);
    let _ = free_mnd_to_vec(bare);
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

/// **The #200 regression pin.** Carrier operations run over a spine deep enough
/// to have aborted the process before the walks became iterative — enumerated
/// per carrier in this module's docs, item 7.
///
/// The fixture is a `common::DEEP` (32 768) left caterpillar — 128× the retired
/// `MAX_TREE_DEPTH`, and comfortably past every measured abort threshold (see
/// `DEEP`'s own docs for the bisected table: recursive drop glue survives 8 192
/// and aborts at 16 384; recursive `Debug` survives 4 096 and aborts at 8 192).
/// The operations that recursed before #200 abort at this depth under that
/// implementation and are plain values here; construction, the two depth
/// measures and the B.19 bijections were already iterative. The test
/// deliberately runs on the default 2 MiB test thread, since a fat-stacked
/// thread would erase exactly the margin it exists to pin.
///
/// A stack overflow aborts the whole harness rather than failing an assertion,
/// so the *failing* direction shows up as "test binary died / SIGSEGV", never a
/// red assertion. The *passing* direction therefore asserts real values —
/// depths, node counts, fold results, structural equality, rendered text —
/// rather than merely reaching the end of the function. In particular the
/// `Cofree` half asserts the **whole grown spine** (its depth and node count,
/// a full-depth `==` against an identically grown twin, an `!=` whose only
/// witness is at the bottom, and a `{:?}` node count): an `unfold` that
/// stopped after one level would satisfy a root-shaped check.
#[test]
fn deep_spine_survives_carrier_operations() {
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

    // The same depth on the crate's other `Free` witness. `ListEndo`'s hole has
    // one recursion slot, so a DEEP-element list is a DEEP-cell tower; a
    // constant label makes `==` walk all of it instead of stopping at the root.
    // `assert!` rather than `assert_eq!`/`assert_ne!` for the same reason as the
    // `Cofree` pair below: a failure would `Debug`-dump ~0.5 MB per side.
    let list = vec_to_free_mnd::<u8, ()>(vec![0_u8; DEEP], ());
    assert!(
        list == vec_to_free_mnd::<u8, ()>(vec![0_u8; DEEP], ()),
        "two identically built DEEP cons towers must compare equal"
    );
    let mut odd_items = vec![0_u8; DEEP];
    odd_items[DEEP - 1] = 1;
    assert!(
        list != vec_to_free_mnd::<u8, ()>(odd_items, ()),
        "…and unequal to one whose deepest cons label differs: every cell above \
         it agrees and both sides have arity 1 throughout, so nothing but the \
         label comparison separates them"
    );
    let list_shown = format!("{list:?}");
    assert_eq!(
        list_shown.matches("Suspend(").count(),
        DEEP,
        "`{{:?}}` renders one Suspend per cons cell"
    );
    assert_eq!(
        list_shown.matches("Some((").count(),
        DEEP,
        "`{{:?}}` renders every cons cell through ListEndo::fmt_shape"
    );
    assert_eq!(
        list_shown.matches("Pure(").count(),
        1,
        "the terminator renders exactly once"
    );
    let terminator = |(): ()| 0_usize;
    let count_cell = |node: Option<(u8, usize)>| match node {
        None => 0,
        Some((_, rest)) => rest + 1,
    };
    let refolded = vec_to_free_mnd::<u8, ()>(vec![0_u8; DEEP], ());
    let cells: usize = refolded.fold(&terminator, &count_cell);
    assert_eq!(cells, DEEP, "the catamorphism counted every cons cell");
    // The B.19 bijection, destruction direction, at the same depth. Split into
    // a length and an all-labels check so a failure prints a verdict rather
    // than 32 768 elements.
    let (items, ()) = free_mnd_to_vec(list);
    assert_eq!(items.len(), DEEP, "the B.19 round trip survives at DEEP");
    assert!(items.iter().all(|&a| a == 0), "…with every label intact");

    // Back to the tree carrier, and structural equality with the original.
    let decoded = free_mnd_to_tree(encoded);
    assert_eq!(decoded, tree, "the B.20 round trip survives at depth DEEP");

    // `Cofree::unfold` — the anamorphism, recursive before #200, and the one
    // carrier entry that never had a guard in front of it at all. Grow the same
    // caterpillar shape on the `Cofree` side.
    //
    // Asserted on the *whole* spine, not on the root: an `unfold` that stopped
    // after one level would satisfy any shallow shape check.
    let labelled: Cofree<TreeEndo<u8>, usize> = Cofree::unfold(DEEP, &|n: usize| {
        if n <= 1 {
            (n, Either::Left(0_u8))
        } else {
            (n, Either::Right((n - 1, 1)))
        }
    });
    assert_eq!(*labelled.head(), DEEP);
    assert_eq!(
        TreeEndo::<u8>::contents(labelled.tail()).len(),
        2,
        "the root of the grown spine is a node"
    );
    assert_eq!(
        cofree_shape(&labelled),
        (DEEP, 2 * DEEP - 1),
        "unfold grew the full caterpillar: DEEP levels, 2·DEEP−1 nodes"
    );

    // `Cofree`'s own `==` and `{:?}` at depth — recursive through the witness
    // before #200, and untested at depth until now. A constant label makes the
    // comparison walk the entire spine instead of stopping at the root.
    let flat = |n: usize| {
        if n == 0 {
            (0_usize, Either::Left(0_u8))
        } else {
            (0_usize, Either::Right((n - 1, 0)))
        }
    };
    let grown: Cofree<TreeEndo<u8>, usize> = Cofree::unfold(DEEP, &flat);
    assert_eq!(cofree_shape(&grown), (DEEP + 1, 2 * DEEP + 1));
    // `assert!` rather than `assert_eq!`/`assert_ne!`: a failure here would
    // otherwise `Debug`-dump ~1.5 MB per side, which tells a reader nothing.
    assert!(
        grown == Cofree::unfold(DEEP, &flat),
        "two identically grown deep spines must compare equal — a full-depth `==` walk"
    );
    assert!(
        grown != Cofree::unfold(DEEP - 1, &flat),
        "…and unequal to one that bottoms out a level sooner: every label agrees, \
         so the mismatch is only reachable at the bottom"
    );
    let shown = format!("{grown:?}");
    assert_eq!(
        shown.matches("Cofree {").count(),
        2 * DEEP + 1,
        "`{{:?}}` renders every node of the deep spine"
    );
    assert!(
        shown.starts_with("Cofree { head: 0, tail: Right((Cofree {"),
        "unexpected head of the rendering: {:?}",
        &shown[..64.min(shown.len())]
    );
    assert!(
        shown.ends_with("Cofree { head: 0, tail: Left(0) })) }"),
        "unexpected tail of the rendering: {:?}",
        &shown[shown.len().saturating_sub(64)..]
    );

    // Finally: drop. Each of these was a recursive `Box`-chain drop before #200
    // — the residual no pre-flight guard could reach, because rejecting a
    // by-value input did not save that input's own drop.
    drop(labelled);
    drop(grown);
    drop(decoded);
    drop(tree);
}

/// The structural `(depth, node count)` of a `Cofree<TreeEndo<u8>, _>` spine.
///
/// Iterative, like every fixture helper here: a recursive measure would abort
/// on exactly the values it exists to measure.
fn cofree_shape<A>(root: &Cofree<TreeEndo<u8>, A>) -> (usize, usize) {
    let mut work = vec![(root, 1_usize)];
    let (mut deepest, mut nodes) = (0_usize, 0_usize);
    while let Some((node, depth)) = work.pop() {
        deepest = deepest.max(depth);
        nodes += 1;
        for child in TreeEndo::<u8>::contents(node.tail()) {
            work.push((child.as_ref(), depth + 1));
        }
    }
    (deepest, nodes)
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
