//! The catgraph-dl claim, end to end.
//!
//! Every shipped `HKT` witness — `ListEndo`, `TreeEndo`, `OptionWitness`,
//! `GroupActionEndo`, `FreeWitness` and `CofreeWitness`
//! (`rg -n 'impl.*HKT for' catgraph-dl/src` → 7 lines, the seventh a
//! `#[cfg(test)]` witness named under `# Reach`) — satisfies the CDL Def 1.4
//! functor identity and composition laws, and the two carrier witnesses'
//! `fmap` carries hand-built values to hand-built values; each of the four
//! that present as containers (`ListEndo`, `TreeEndo`, `OptionWitness`,
//! `GroupActionEndo`; `rg -n 'impl.*Container for' catgraph-dl/src` → 5, the
//! fifth the same `#[cfg(test)]` witness) satisfies the four
//! Abbott–Altenkirch–Ghani container laws at one sample per constructor of its
//! shape set; the CDL Example B.19 / B.20 bijections
//! round-trip in both directions; each of `FoldingRnn`, `RecursiveNn`,
//! `UnfoldingRnn`, `MealyCell` and `MooreCell` unrolls to what a `Free` /
//! `Cofree` walker written in this file computes from the same cell; and
//! `UnfoldingRnn::unroll_iter`, `MealyCell::run_iter` and `MooreCell::run_iter`
//! call their cells exactly once per pulled item, the two `run_iter`s pulling a
//! non-`Vec` source exactly once per item too.
//!
//! # Input space
//!
//! The functor and container laws run on one sample per constructor of each
//! witness's hole — both `Z2` elements for `GroupActionEndo`, whose shape set
//! is the group; for `ListEndo` and `TreeEndo` also on 64 proptest samples of
//! the full `Option<(u32, i32)>` / `Either<u32, (i32, i32)>` hole; and on the
//! two carrier witnesses on hand-built spines of depth ≤ 3 plus 64 proptest
//! samples of cons towers up to 16 cells and `Cofree` caterpillars up to 8
//! levels. The B.19 round trip is a 64-case proptest over `Vec<u32>` up to 24
//! elements plus the empty and two-cell corners; B.20 is three hand-built
//! trees. The five architecture rows compare against the walker on the
//! caller-sampled fixtures listed at each arm and on 64 proptest samples each:
//! `Vec<i64>` up to 16 elements, `BinaryTree<u8>` at depth ≤ 4 and ≤ 16 nodes,
//! and seed/depth or seed/input pairs with depth and length up to 16. The
//! call counters run at `n = 0`, `1`, `5` on `unroll_iter`'s unbounded stream
//! and at `k = 0`, `1`, `3` against a five-item input on the two `run_iter`s;
//! the non-`Vec` source row runs at the same `k` against a five-item counting
//! iterator.
//! The depth rows are a left caterpillar at `MAX_TREE_DEPTH`, one cell deeper,
//! and the `common::DEEP` (32 768) spine.
//!
//! # References
//!
//! The two functor laws and the four container laws are equations between two
//! expressions built from the witness's own operations — the reference is the
//! law. Beside them each carrier `fmap` row compares against a hand-built
//! target value written cell by cell — a target neither law leg supplies. The
//! architecture rows compare
//! against `unroll_list_via_free_mnd` / `unroll_tree_via_free_mnd` (the
//! algebra direction, walking the `Free` tower — the tree one deliberately
//! recursive, unlike the crate's own walks) and against `unfold_stream` /
//! `unfold_driven` (the coalgebra direction, `Cofree::unfold` from the same
//! seed), plus hand-computed closed-form values at each arm. The bijection and
//! depth rows compare against hand-built carriers and closed-form counts.
//!
//! # Reach
//!
//! `StrayWitness` (`free_monad/mod.rs:1003`) implements `HKT`, `Functor` and
//! `Container` inside a `#[cfg(test)]` module, so an integration test cannot
//! name it; its laws are not reachable from here. The ten root re-exports
//! `EnrichedCategory`, `HomMap`, `LawvereMetricSpace`, `BoolRig`, `F64Rig`,
//! `One`, `Rig`, `Tropical`, `UnitInterval` and `Zero`
//! (`src/lib.rs:80-82`) are `catgraph-applied` types,
//! covered by `catgraph-applied/tests/canonical.rs`, and are not declared under
//! `catgraph-dl/src`. The `Para` / actegory / module surface and the
//! F-(co)algebra and monad-algebra newtypes are exercised by the other files
//! under `tests/`, not here. `Free`'s and `Cofree`'s `fmap` bounds on
//! `Container`, so the two carrier witnesses are functors exactly over the
//! operation functors that present as containers — a `Functor` witness with no
//! `Container` impl is outside this file's reach.
//!
//! # covers:
//!
//! `BinaryTree` `Cofree` `CofreeWitness` `Container` `DebugFunctor`
//! `DepthError` `Either` `EndoWitness` `EqFunctor` `FoldingRnn` `Free`
//! `FreeView` `FreeWitness` `Functor` `GroupActionEndo` `HKT` `ListEndo`
//! `MealyCell` `MooreCell` `OptionWitness` `RecursiveNn` `TreeEndo`
//! `UnfoldingRnn` `Z2Group`
//!
//! # not-covered:
//!
//! `Actegory` `Comonoid` `DiagonalComonoid` `DirectSum` `Dual`
//! `DualF64Module` `F64Actegory` `F64Module` `F64Monoidal` `F64Morphism`
//! `F64Object` `FAlgebra` `FAlgebraHom` `FCoalgebra` `FCoalgebraHom` `Group`
//! `IsoBackward` `IsoForward` `Monad` `MonadAlgebra` `MonadAlgebraHom`
//! `MonoidalCategory` `MonoidalTag` `NaturalIso` `NaturalTransformation`
//! `Para` `ParaMorphism` `Pointed` `Pure` `RActegory` `RModule` `RMonoidal`
//! `RMorphism` `RObject` `Reparameterization` `Sealed` `SetActegory`
//! `SetCategoryDefaults` `SetMonoidal` `SetMorphism` `SetObject` `TreeView`

#![allow(
    clippy::type_complexity,
    reason = "The FoldingRnn<…5 type params…> spelling is exactly what callers see; a `type` alias would still need every parameter."
)]

mod common;

use core::convert::Infallible;

use catgraph_dl::algebra::{GroupActionEndo, Z2Group};
use catgraph_dl::architectures::{FoldingRnn, MealyCell, MooreCell, RecursiveNn, UnfoldingRnn};
use catgraph_dl::depth::{MAX_TREE_DEPTH, free_mnd_depth, guard_tree_depth, tree_depth};
use catgraph_dl::endofunctor::OptionWitness;
use catgraph_dl::free_monad::list_endo::{ListEndo, free_mnd_to_vec, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{
    BinaryTree, TreeEndo, free_mnd_to_tree, tree_to_free_mnd,
};
use catgraph_dl::free_monad::{Cofree, CofreeWitness, Free, FreeView, FreeWitness};
use catgraph_dl::{Container, DepthError, Either, Functor};

use common::{
    DEEP, UnitEndo, assert_container_laws, assert_functor_laws, spine_free_mnd, spine_tree,
};

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Fixtures: hand-built carrier values, cell by cell.
// ---------------------------------------------------------------------------

/// A `Free<TreeEndo<u8>, i32>` with two `Pure` leaves at distinct payloads and
/// one `Suspend(Left(_))` leaf, so a `fmap` that visits one payload and not the
/// other, or that touches the operation functor's own label, is separable from
/// one that visits both and leaves the label alone.
fn free_two_pure_leaves(first: i32, second: i32) -> Free<TreeEndo<u8>, i32> {
    Free::suspend(Either::Right((
        Box::new(Free::pure(first)),
        Box::new(Free::suspend(Either::Right((
            Box::new(Free::pure(second)),
            Box::new(Free::suspend(Either::Left(9_u8))),
        )))),
    )))
}

/// A `Cofree<TreeEndo<u8>, i32>` of three nodes with distinct heads and two
/// distinct leaf labels: a `fmap` that maps a head twice, or that maps only the
/// root, is separable from one that maps each head once.
fn cofree_three_heads(root: i32, left: i32, right: i32) -> Cofree<TreeEndo<u8>, i32> {
    Cofree::new(
        root,
        Either::Right((
            Box::new(Cofree::new(left, Either::Left(7_u8))),
            Box::new(Cofree::new(right, Either::Left(8_u8))),
        )),
    )
}

/// A `Cofree<OptionWitness, i32>` prefix of `heads`, in order; `None` tail at
/// the last. Panics on an empty slice — a `Cofree` node always carries a head.
fn cofree_prefix(heads: &[i32]) -> Cofree<OptionWitness, i32> {
    let (&last, rest) = heads.split_last().expect("a Cofree prefix has a head");
    let mut acc = Cofree::new(last, None);
    for &h in rest.iter().rev() {
        acc = Cofree::new(h, Some(Box::new(acc)));
    }
    acc
}

/// The `head`s of a `Cofree<OptionWitness, A>` prefix, root first. Iterative,
/// like every fixture helper here.
fn cofree_heads(prefix: Cofree<OptionWitness, i32>) -> Vec<i32> {
    let mut out = Vec::new();
    let mut cur = Some(prefix);
    while let Some(node) = cur {
        let (head, tail) = node.into_parts();
        out.push(head);
        cur = tail.map(|boxed| *boxed);
    }
    out
}

/// The structural `(depth, node count)` of a `Cofree<TreeEndo<u8>, _>` spine.
///
/// Iterative: a recursive measure would abort on exactly the values it exists
/// to measure.
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

/// A constant-head `Cofree<TreeEndo<u8>, i32>` left caterpillar of `depth`
/// levels, grown through [`Cofree::unfold`]; `head` labels every node.
fn cofree_spine(depth: usize, head: i32) -> Cofree<TreeEndo<u8>, i32> {
    Cofree::unfold(depth, &|n: usize| {
        if n == 0 {
            (head, Either::Left(0_u8))
        } else {
            (head, Either::Right((n - 1, 0)))
        }
    })
}

// ---------------------------------------------------------------------------
// Functor laws (CDL Def 1.4) — every shipped `HKT` witness.
// ---------------------------------------------------------------------------

/// The identity and composition laws on all six shipped witnesses, and a
/// hand-built `fmap` target value for each carrier witness beside them.
///
/// Each carrier row also asserts where a concrete `fmap` sends a concrete
/// value: every `Pure` payload (resp. every `head`) mapped exactly once, every
/// operation-functor label untouched.
#[test]
fn functor_laws_hold_on_every_shipped_witness() {
    // `1 + A × −`, both summands.
    assert_functor_laws::<ListEndo<u32>>(|| None);
    assert_functor_laws::<ListEndo<u32>>(|| Some((7_u32, 42_i32)));

    // `A + (−)²`, both summands; the `Right` arm calls the morphism twice.
    assert_functor_laws::<TreeEndo<u32>>(|| Either::Left(9_u32));
    assert_functor_laws::<TreeEndo<u32>>(|| Either::Right((3_i32, 4_i32)));

    // `1 + −`, both summands (issue #315: the battery reached the container
    // laws but not this one).
    assert_functor_laws::<OptionWitness>(|| None);
    assert_functor_laws::<OptionWitness>(|| Some(7_i32));

    // `G × −`, both group elements.
    for fx in [
        (Z2Group(false), 0_i32),
        (Z2Group(true), 5),
        (Z2Group(false), -7),
        (Z2Group(true), 42),
    ] {
        assert_functor_laws::<GroupActionEndo<Z2Group>>(move || fx);
    }

    // `Free` over the branching operation functor: a `Pure`-free spine, and one
    // with two `Pure` leaves.
    assert_functor_laws::<FreeWitness<TreeEndo<u8>>>(|| Free::suspend(Either::Left(9_u8)));
    assert_functor_laws::<FreeWitness<TreeEndo<u8>>>(|| free_two_pure_leaves(1, 2));
    // …and over the cons operation functor, where every cell has arity 1.
    assert_functor_laws::<FreeWitness<ListEndo<u8>>>(|| vec_to_free_mnd(vec![1_u8, 2, 3], 7_i32));

    // `Cofree` over both a branching and a linear operation functor.
    assert_functor_laws::<CofreeWitness<TreeEndo<u8>>>(|| cofree_three_heads(1, 2, 3));
    assert_functor_laws::<CofreeWitness<OptionWitness>>(|| cofree_prefix(&[1, 2, 3]));

    // --- concrete `fmap` targets, which the two laws cannot supply ----------

    // Every `Pure` payload mapped once; the `Left(9)` label and the node shape
    // unchanged.
    assert_eq!(
        FreeWitness::<TreeEndo<u8>>::fmap(free_two_pure_leaves(1, 2), |x| x * 10),
        free_two_pure_leaves(10, 20),
        "Free fmap maps every Pure payload once and preserves the F-structure"
    );
    // The cons tower: labels are the operation functor's, the terminator is the
    // payload — only the latter moves.
    let (labels, terminator) = free_mnd_to_vec(FreeWitness::<ListEndo<u8>>::fmap(
        vec_to_free_mnd(vec![1_u8, 2, 3], 7_i32),
        |x| x + 1,
    ));
    assert_eq!(
        (labels, terminator),
        (vec![1_u8, 2, 3], 8_i32),
        "Free fmap over ListEndo moves the terminator and leaves the cons labels"
    );

    // Every `head` mapped once — a double map would give (100, 200, 300).
    assert_eq!(
        CofreeWitness::<TreeEndo<u8>>::fmap(cofree_three_heads(1, 2, 3), |x| x * 10),
        cofree_three_heads(10, 20, 30),
        "Cofree fmap maps every head exactly once and preserves the F-structure"
    );
    assert_eq!(
        cofree_heads(CofreeWitness::<OptionWitness>::fmap(
            cofree_prefix(&[1, 2, 3]),
            |x| x * 10
        )),
        vec![10, 20, 30],
        "Cofree fmap over OptionWitness maps every head of the prefix once"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// `ListEndo<u32>` over the full `Option<(u32, i32)>` hole.
    #[test]
    fn list_endo_functor_laws_proptest(
        fx in proptest::option::of((any::<u32>(), any::<i32>())),
    ) {
        assert_functor_laws::<ListEndo<u32>>(move || fx);
    }

    /// `TreeEndo<u32>` over the full `Either<u32, (i32, i32)>` hole; the
    /// `Right` arm calls the morphism twice.
    #[test]
    fn tree_endo_functor_laws_proptest(
        fx in prop_oneof![
            any::<u32>().prop_map(Either::Left),
            (any::<i32>(), any::<i32>()).prop_map(Either::Right),
        ],
    ) {
        assert_functor_laws::<TreeEndo<u32>>(move || fx);
    }

    /// The two carrier witnesses on generated spines: a cons tower of up to 16
    /// cells, and a `Cofree` caterpillar of up to 8 levels.
    #[test]
    fn free_and_cofree_functor_laws_proptest(
        items in prop::collection::vec(any::<u8>(), 0..=16),
        terminator in any::<i32>(),
        depth in 0..=8_usize,
        head in any::<i32>(),
    ) {
        assert_functor_laws::<FreeWitness<ListEndo<u8>>>(
            || vec_to_free_mnd(items.clone(), terminator),
        );
        assert_functor_laws::<CofreeWitness<TreeEndo<u8>>>(|| cofree_spine(depth, head));

        // The terminator moves by exactly one application of the morphism, and
        // the cons labels do not move at all.
        let mapped = FreeWitness::<ListEndo<u8>>::fmap(
            vec_to_free_mnd(items.clone(), terminator),
            |x: i32| x.wrapping_mul(3),
        );
        let (labels, moved) = free_mnd_to_vec(mapped);
        prop_assert_eq!(labels, items.clone());
        prop_assert_eq!(moved, terminator.wrapping_mul(3));

        // Every head of the grown spine moves by exactly one application.
        let mapped = CofreeWitness::<TreeEndo<u8>>::fmap(
            cofree_spine(depth, head),
            |x: i32| x.wrapping_mul(3),
        );
        prop_assert!(mapped == cofree_spine(depth, head.wrapping_mul(3)));
    }
}

// ---------------------------------------------------------------------------
// Container laws (Abbott–Altenkirch–Ghani 2003, via CDL §4).
// ---------------------------------------------------------------------------

/// Round-trip, arity coherence in both directions, `fmap` coherence and borrow
/// coherence, on every shape of the four shipped container witnesses.
#[test]
fn container_laws_hold_on_every_container_witness() {
    // `1 + A × −`: `None` shape (arity 0) and `Some` shape (arity 1).
    assert_container_laws::<ListEndo<i32>>(None);
    assert_container_laws::<ListEndo<i32>>(Some((7, 42)));

    // `A + (−)²`: `Left` leaf shape (arity 0) and `Right` node shape (arity 2).
    assert_container_laws::<TreeEndo<i32>>(Either::Left(9));
    assert_container_laws::<TreeEndo<i32>>(Either::Right((3, 4)));

    // `G × −`: one shape per group element, all arity 1.
    assert_container_laws::<GroupActionEndo<Z2Group>>((Z2Group(false), 5));
    assert_container_laws::<GroupActionEndo<Z2Group>>((Z2Group(true), -5));

    // `1 + −`: `Shape = bool`; `false` is the unit summand (arity 0), `true`
    // the singleton (arity 1 — the one recursive slot).
    assert_container_laws::<OptionWitness>(None);
    assert_container_laws::<OptionWitness>(Some(7));
}

// ---------------------------------------------------------------------------
// CDL Example B.19 / B.20 — the carrier bijections.
// ---------------------------------------------------------------------------

/// `FreeMnd(1 + A × −)(Z) ≅ (Vec<A>, Z)` and
/// `FreeMnd(A + (−)²)(!) ≅ BinaryTree<A>`, on the corners and three hand-built
/// trees: the empty list is `Pure`, a hand-written cons tower equals the
/// canonical encoding, and each tree survives the round trip.
#[test]
fn b19_b20_bijections_round_trip() {
    // The empty list is canonically `Free::pure(())` — no `Suspend` cells.
    let empty: Free<ListEndo<u32>, ()> = vec_to_free_mnd(Vec::new(), ());
    match empty.as_view() {
        FreeView::Pure(()) => (),
        FreeView::Suspend(_) => panic!("empty Vec must encode to Free::pure(()), not Suspend"),
    }
    let (items, ()) = free_mnd_to_vec(Free::<ListEndo<u32>, ()>::pure(()));
    assert!(items.is_empty(), "Pure(()) must decode to an empty Vec");

    // The explicit cons-cell tower for `[1, 2]` equals the canonical encoding
    // structurally, and both decode to `[1, 2]`.
    let inner: Free<ListEndo<u32>, ()> = Free::suspend(Some((2_u32, Box::new(Free::pure(())))));
    let outer: Free<ListEndo<u32>, ()> = Free::suspend(Some((1_u32, Box::new(inner))));
    let canonical = vec_to_free_mnd::<u32, ()>(vec![1, 2], ());
    assert_eq!(canonical, outer);
    let (hand, ()) = free_mnd_to_vec(outer);
    assert_eq!(hand, vec![1_u32, 2]);
    let (canon, ()) = free_mnd_to_vec(canonical);
    assert_eq!(canon, vec![1_u32, 2]);

    // B.20: a leaf, a single node, and a depth-3 tree.
    for tree in [
        BinaryTree::leaf(7_u32),
        BinaryTree::node(BinaryTree::leaf(1_u32), BinaryTree::leaf(2_u32)),
        BinaryTree::node(
            BinaryTree::node(BinaryTree::leaf(1_u32), BinaryTree::leaf(2_u32)),
            BinaryTree::node(
                BinaryTree::leaf(3_u32),
                BinaryTree::node(BinaryTree::leaf(4_u32), BinaryTree::leaf(5_u32)),
            ),
        ),
    ] {
        let back = free_mnd_to_tree(tree_to_free_mnd(tree.clone()));
        assert_eq!(back, tree, "B.20 round trip on {tree:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// CDL Example B.19 in both directions: `items → Free → items`, and the
    /// re-encoding of the decoded items equals the original carrier under
    /// `Free`'s own `PartialEq`. `Free` ships no `Clone`, so each leg builds
    /// its operand afresh.
    #[test]
    fn vec_round_trip_proptest(items in proptest::collection::vec(any::<u32>(), 0..=24)) {
        let (round_trip, ()) = free_mnd_to_vec(vec_to_free_mnd::<u32, ()>(items.clone(), ()));
        prop_assert_eq!(round_trip, items.clone());

        let f1 = vec_to_free_mnd::<u32, ()>(items.clone(), ());
        let (items_again, ()) = free_mnd_to_vec(vec_to_free_mnd::<u32, ()>(items.clone(), ()));
        let f2 = vec_to_free_mnd::<u32, ()>(items_again, ());
        prop_assert_eq!(f2, f1);
    }
}

/// Issue #312. The documented panic contract on a non-canonical
/// `Free::suspend(None)` reaching `free_mnd_to_vec` with no `Pure` terminator
/// above it. The canonical encoding never emits this shape, so what is pinned
/// is the panic message wording and the infallible signature.
#[test]
#[should_panic(expected = "non-canonical Free value")]
fn free_mnd_to_vec_panics_on_bare_suspend_none() {
    let bare: Free<ListEndo<u32>, ()> = Free::suspend(None);
    let _ = free_mnd_to_vec(bare);
}

/// CDL Proposition B.18, dual side: `Cofree<UnitEndo<_>, u32>` constructs under
/// the GAT bound, `head()` reads back, and two identically built values compare
/// equal through the opt-in `PartialEq`. The carriers ship no `Clone`, so the
/// second value is built rather than cloned.
#[test]
fn cofree_cmnd_constructs_and_reads_back() {
    struct TrivialTag;
    type TrivialEndo = UnitEndo<TrivialTag>;

    let c: Cofree<TrivialEndo, u32> = Cofree::new(42_u32, ());
    assert_eq!(*c.head(), 42);
    let c2: Cofree<TrivialEndo, u32> = Cofree::new(42_u32, ());
    assert_eq!(*c2.head(), 42);
    assert_eq!(c, c2);
}

// ---------------------------------------------------------------------------
// Depth: the opt-in guard boundary and the #200 deep-spine pin.
// ---------------------------------------------------------------------------

/// Issue #231's guard in its post-#200 role — an opt-in service no helper in
/// the crate calls.
///
/// A left caterpillar at exactly `MAX_TREE_DEPTH` passes the guard and the
/// bijection preserves its depth; one cell deeper is refused with
/// `DepthError::TreeDepthExceeded` carrying the exact `{ depth, limit }`, while
/// the same over-limit carrier still round-trips through the infallible
/// helpers.
#[test]
fn opt_in_depth_guard_boundary() {
    let at_limit = spine_tree(MAX_TREE_DEPTH);
    assert_eq!(tree_depth(&at_limit), MAX_TREE_DEPTH);
    assert!(guard_tree_depth(&at_limit).is_ok());

    let encoded = tree_to_free_mnd(at_limit.clone());
    assert_eq!(
        free_mnd_depth(&encoded),
        MAX_TREE_DEPTH,
        "the B.20 bijection preserves structural depth"
    );
    assert_eq!(
        free_mnd_to_tree(encoded),
        at_limit,
        "round trip survives at the limit"
    );

    let over = MAX_TREE_DEPTH + 1;
    let too_deep = spine_tree(over);
    match guard_tree_depth(&too_deep) {
        Err(DepthError::TreeDepthExceeded { depth, limit }) => {
            assert_eq!(depth, over);
            assert_eq!(limit, MAX_TREE_DEPTH);
        }
        other => panic!("guard_tree_depth: expected TreeDepthExceeded, got {other:?}"),
    }
    // The guard borrowed, so the value is still ours — and the helper takes it,
    // because it no longer recurses.
    assert_eq!(
        free_mnd_to_tree(tree_to_free_mnd(too_deep)),
        spine_tree(over),
        "an over-limit carrier still round-trips: the helpers are infallible since #200"
    );
    assert_eq!(free_mnd_depth(&spine_free_mnd(over)), over);
}

/// **The #200 regression pin.** Carrier operations run over a `common::DEEP`
/// (32 768) left caterpillar — past every abort threshold `DEEP`'s own docs
/// tabulate for a recursive walk on a 2 MiB test thread.
///
/// A stack overflow aborts the harness rather than failing an assertion, so the
/// passing direction asserts real values — depths, node counts, fold results,
/// structural equality, rendered text — and the `Cofree` half asserts the whole
/// grown spine, since an `unfold` that stopped after one level would satisfy a
/// root-shaped check. `FreeWitness`'s and `CofreeWitness`'s `fmap` join the
/// list: both are explicit-worklist walks, so both must survive this fixture.
#[test]
fn deep_spine_survives_carrier_operations() {
    // Construction and the two depth measures (already iterative pre-#200).
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

    // The B.20 bijection, forward — recursive and `Result`-returning before.
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
        &|z: Infallible| match z {},
        &|node: Either<u8, (usize, usize)>| match node {
            Either::Left(_) => 1,
            Either::Right((l, r)) => l + r,
        },
    );
    assert_eq!(leaves, DEEP, "the caterpillar has DEEP leaves");

    // The same depth on the crate's other `Free` witness. `ListEndo`'s hole has
    // one recursion slot, so a DEEP-element list is a DEEP-cell tower; a
    // constant label makes `==` walk all of it instead of stopping at the root.
    // `assert!` rather than `assert_eq!`/`assert_ne!` here and below: a failure
    // would otherwise `Debug`-dump megabytes per side.
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
    let cells: usize =
        vec_to_free_mnd::<u8, ()>(vec![0_u8; DEEP], ()).fold(&terminator, &count_cell);
    assert_eq!(cells, DEEP, "the catamorphism counted every cons cell");

    // `FreeWitness::fmap` at the same depth: the terminator moves, every cons
    // label stays, and the mapped tower still has DEEP cells.
    let mapped = FreeWitness::<ListEndo<u8>>::fmap(
        vec_to_free_mnd::<u8, i32>(vec![0_u8; DEEP], 1_i32),
        |x| x + 1,
    );
    assert!(
        mapped == vec_to_free_mnd::<u8, i32>(vec![0_u8; DEEP], 2_i32),
        "fmap over a DEEP cons tower moves the terminator and nothing else"
    );

    // The B.19 bijection, destruction direction, at the same depth. Split into
    // a length and an all-labels check so a failure prints a verdict rather
    // than 32 768 elements.
    let (items, ()) = free_mnd_to_vec(list);
    assert_eq!(items.len(), DEEP, "the B.19 round trip survives at DEEP");
    assert!(items.iter().all(|&a| a == 0), "…with every label intact");

    // Back to the tree carrier, and structural equality with the original.
    let decoded = free_mnd_to_tree(encoded);
    assert_eq!(decoded, tree, "the B.20 round trip survives at depth DEEP");

    // `Cofree::unfold` — the anamorphism, recursive before #200. Asserted on
    // the *whole* spine, not the root.
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

    // `Cofree`'s own `==` and `{:?}` at depth. A constant label makes the
    // comparison walk the entire spine instead of stopping at the root.
    let grown = cofree_spine(DEEP, 0);
    assert_eq!(cofree_shape(&grown), (DEEP + 1, 2 * DEEP + 1));
    assert!(
        grown == cofree_spine(DEEP, 0),
        "two identically grown deep spines must compare equal — a full-depth `==` walk"
    );
    assert!(
        grown != cofree_spine(DEEP - 1, 0),
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

    // `CofreeWitness::fmap` at the same depth: every head of the deep spine
    // moves by one application of the morphism.
    assert!(
        CofreeWitness::<TreeEndo<u8>>::fmap(cofree_spine(DEEP, 0), |x| x + 1)
            == cofree_spine(DEEP, 1),
        "fmap over a DEEP Cofree spine maps every head exactly once"
    );

    // Finally: drop. Each of these was a recursive `Box`-chain drop before #200.
    drop(labelled);
    drop(grown);
    drop(decoded);
    drop(tree);
}

// ---------------------------------------------------------------------------
// The architecture (co)algebra walkers — the references the five unrollers are
// compared against.
// ---------------------------------------------------------------------------

/// List-direction cell types (`FoldingRnn` over `1 + A × −`).
type ListCell0 = fn(()) -> i64;
type ListCell1 = fn(((), i64, i64)) -> i64;

/// Tree-direction cell types (`RecursiveNn` over `A + (−)²`).
type TreeCell0 = fn(i64) -> i64;
type TreeCell1 = fn((i64, u8, i64, i64)) -> i64;

/// Walk the cons-cell tower of `Free<ListEndo<A>, ()>`, applying the folding
/// cell — the unique algebra hom from `(Free, structure_map)` into
/// `(S, [cell_0, cell_1])`. CDL Remark 2.13 / Prop B.18.
fn unroll_list_via_free_mnd(
    cell: &FoldingRnn<(), i64, ListCell0, ListCell1, i64>,
    free_mnd: Free<ListEndo<i64>, ()>,
) -> i64 {
    let (items, ()) = free_mnd_to_vec(free_mnd);
    let seed = (cell.cell_0)(());
    items
        .into_iter()
        .rev()
        .fold(seed, |s, a| (cell.cell_1)(((), a, s)))
}

/// Walk `Free<TreeEndo<A>, Infallible>` directly, applying the recursive cell —
/// the unique algebra hom for the tree direction. CDL Remark 2.13 / Prop B.18.
///
/// Deliberately **recursive**, unlike the crate's own walks since #200: an
/// oracle written the same way as the code under test proves less. Its fixtures
/// are hand-built and depth ≤ 4; the deep regime is pinned separately against a
/// closed-form value rather than against this walker.
fn unroll_tree_via_free_mnd(
    cell: &RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8>,
    free_mnd: Free<TreeEndo<u8>, Infallible>,
) -> i64 {
    match free_mnd.into_view() {
        FreeView::Pure(z) => match z {}, // Infallible: unreachable.
        FreeView::Suspend(node) => match node {
            Either::Left(_a) => (cell.cell_0)(cell.parameter),
            Either::Right((l, r)) => {
                let leftmost = leftmost_leaf_payload(&l);
                let l_val = unroll_tree_via_free_mnd(cell, *l);
                let r_val = unroll_tree_via_free_mnd(cell, *r);
                (cell.cell_1)((cell.parameter, leftmost, l_val, r_val))
            }
        },
    }
}

/// The leftmost leaf payload of a `Free<TreeEndo<u8>, Infallible>`. Mirrors
/// `RecursiveNn::leftmost_leaf` on the `BinaryTree` carrier.
fn leftmost_leaf_payload(t: &Free<TreeEndo<u8>, Infallible>) -> u8 {
    let mut current = t;
    loop {
        match current.as_view() {
            FreeView::Pure(z) => match *z {},
            FreeView::Suspend(node) => match node {
                Either::Left(a) => return *a,
                Either::Right((l, _r)) => current = l.as_ref(),
            },
        }
    }
}

/// A bounded stream prefix over `O` — `Cofree<OptionWitness, O>` is the
/// bounded, non-empty prefix of the terminal `(O × −)`-coalgebra (CDL Remark
/// H.6); the empty (depth-0) case is the top-level `Option`.
type StreamPrefix<O> = Cofree<OptionWitness, O>;

/// Walk a bounded prefix into its observed output sequence — the
/// counit-then-tail projection, collected left to right. `None` yields `[]`.
fn cofree_prefix_to_vec<O>(prefix: Option<StreamPrefix<O>>) -> Vec<O> {
    let mut out = Vec::new();
    let mut cur = prefix;
    while let Some(node) = cur {
        let (head, tail) = node.into_parts();
        out.push(head);
        cur = tail.map(|boxed| *boxed);
    }
    out
}

/// Unfold a **state-driven** stream prefix (the `UnfoldingRnn` shape): emit
/// `step(s).0` at each state, advance to `step(s).1`, for `depth` layers.
fn unfold_stream<S, O>(
    seed: S,
    step: impl Fn(S) -> (O, S),
    depth: usize,
) -> Option<StreamPrefix<O>> {
    if depth == 0 {
        return None;
    }
    let coalg = |(s, remaining): (S, usize)| {
        let (head, next) = step(s);
        let tail_seed = (remaining > 1).then_some((next, remaining - 1));
        (head, tail_seed)
    };
    Some(Cofree::unfold((seed, depth), &coalg))
}

/// Unfold an **input-driven** stream prefix (the `MealyCell` / `MooreCell`
/// shape): consume the inputs left to right, emitting `step(s, i).0` and
/// advancing to `step(s, i).1`. Prefix length = `inputs.len()`.
fn unfold_driven<S, I, O>(
    seed: S,
    inputs: Vec<I>,
    step: impl Fn(S, I) -> (O, S),
) -> Option<StreamPrefix<O>> {
    let mut iter = inputs.into_iter();
    let first = iter.next()?; // empty inputs → top-level `None`.
    let coalg = |(s, input, mut rest): (S, I, std::vec::IntoIter<I>)| {
        let (head, next) = step(s, input);
        let tail_seed = rest.next().map(|i| (next, i, rest));
        (head, tail_seed)
    };
    Some(Cofree::unfold((seed, first, iter), &coalg))
}

/// Bounded-depth `BinaryTree<u8>` strategy: depth ≤ 4, ≤ 16 nodes — which
/// keeps the deliberately-recursive tree oracle inside its own stack budget.
fn arb_binary_tree() -> impl Strategy<Value = BinaryTree<u8>> {
    any::<u8>().prop_map(BinaryTree::leaf).prop_recursive(
        4,  // max recursion depth
        16, // max total nodes
        2,  // expected branching factor
        |inner| (inner.clone(), inner).prop_map(|(l, r)| BinaryTree::node(l, r)),
    )
}

// ---------------------------------------------------------------------------
// The five architectures against the walkers (CDL App I / App J).
// ---------------------------------------------------------------------------

/// **`FoldingRnn::unroll` is the algebra hom** (CDL Remark 2.13 / Prop B.18,
/// Example 2.12): it equals the `Free<ListEndo<i64>, ()>` tower walk on five
/// sampled `Vec<i64>`s including the empty one, and carries the hand-computed
/// sum-with-bias, fold-direction and length-counter values.
#[test]
fn folding_rnn_unroll_equals_the_free_walker() {
    // Sum-with-bias: cell_0(p) = p, cell_1((p, a, s)) = a + s + p, p = 10.
    type Cell0 = fn(i64) -> i64;
    type Cell1 = fn((i64, i64, i64)) -> i64;
    let biased: FoldingRnn<i64, i64, Cell0, Cell1, i64> =
        FoldingRnn::new(10_i64, |p| p, |(p, a, s)| a + s + p);
    assert_eq!(
        FoldingRnn::unroll(&biased, vec![1_i64, 2, 3]),
        46,
        "right-fold sum-with-bias on [1, 2, 3] with p = 10: 10 + 3·10 + 6"
    );
    assert_eq!(
        FoldingRnn::unroll(&biased, Vec::<i64>::new()),
        10,
        "empty input collapses to cell_0(p)"
    );
    assert_eq!(
        FoldingRnn::unroll(&biased, vec![5_i64]),
        25,
        "singleton input: cell_1((10, 5, cell_0(10)))"
    );

    // Fold direction: cell_1((p, a, s)) = s·2 + a, neither commutative nor
    // associative in (a, s).
    let doubling: FoldingRnn<i64, i64, Cell0, Cell1, i64> =
        FoldingRnn::new(0_i64, |p| p, |(_p, a, s)| s * 2 + a);
    assert_eq!(
        FoldingRnn::unroll(&doubling, vec![1_i64, 2, 3]),
        17,
        "right fold on [1, 2, 3] from seed 0: ((0·2 + 3)·2 + 2)·2 + 1 = 17; \
         a left fold gives 11"
    );

    // Length counter: cell_0(_) = 0, cell_1((_, _, s)) = s + 1.
    type LenCell0 = fn(()) -> usize;
    type LenCell1 = fn(((), i32, usize)) -> usize;
    let counter: FoldingRnn<(), usize, LenCell0, LenCell1, i32> =
        FoldingRnn::new((), |()| 0_usize, |((), _a, s)| s + 1);
    for n in [0_usize, 1, 5, 17, 100] {
        let inputs: Vec<i32> = (0..i32::try_from(n).expect("test length fits i32")).collect();
        assert_eq!(
            FoldingRnn::unroll(&counter, inputs),
            n,
            "length-counter unroll on a {n}-element vec"
        );
    }

    // Against the walker.
    let cell: FoldingRnn<(), i64, ListCell0, ListCell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| a + s);
    for vec in [
        Vec::<i64>::new(),
        vec![1_i64],
        vec![1_i64, 2, 3],
        vec![5_i64, -7, 11, -13, 17],
        vec![0_i64; 10],
    ] {
        assert_eq!(
            FoldingRnn::unroll(&cell, vec.clone()),
            unroll_list_via_free_mnd(&cell, vec_to_free_mnd(vec.clone(), ())),
            "FoldingRnn::unroll(cell, {vec:?}) must equal the Free tower walk"
        );
    }
}

/// **`RecursiveNn::unroll` is the algebra hom, tree direction** (CDL Remark
/// 2.13 / Prop B.18, Example J.3): it equals the recursive
/// `Free<TreeEndo<u8>, Infallible>` walk on four hand-built trees under two
/// cells, carries the hand-computed node counts and the asymmetric-cell value
/// that reads the leftmost-leaf payload, and walks the `common::DEEP`
/// caterpillar to its
/// closed-form value (issue #200 — that walk was the crate's only recursive
/// unroller).
#[test]
fn recursive_nn_unroll_equals_the_free_walker() {
    // Node-counting cell: cell_0(p) = p = 1, cell_1((_, _, l, r)) = l + r + 1.
    let cell: RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8> =
        RecursiveNn::new(1_i64, |p| p, |(_p, _a, l, r)| l + r + 1);

    assert_eq!(
        RecursiveNn::unroll(&cell, BinaryTree::leaf(7_u8)),
        1,
        "a leaf collapses to cell_0(p)"
    );

    let trees = [
        BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
        BinaryTree::node(
            BinaryTree::node(BinaryTree::leaf(1_u8), BinaryTree::leaf(2_u8)),
            BinaryTree::leaf(3_u8),
        ),
        BinaryTree::node(
            BinaryTree::leaf(4_u8),
            BinaryTree::node(BinaryTree::leaf(5_u8), BinaryTree::leaf(6_u8)),
        ),
    ];
    // 2 leaves + 1 combine; 3 leaves + 2 combines, left- and right-skewed.
    for (tree, expected) in trees.iter().zip([3_i64, 5, 5]) {
        assert_eq!(
            RecursiveNn::unroll(&cell, tree.clone()),
            expected,
            "node count of {tree:?}"
        );
    }
    // Asymmetric cell: cell_1((p, a, l, r)) = 2·l + r + a weights the two
    // subtrees differently and reads the leftmost-leaf payload.
    let asymmetric: RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8> =
        RecursiveNn::new(1_i64, |p| p, |(_p, a, l, r)| 2 * l + r + i64::from(a));
    assert_eq!(
        RecursiveNn::unroll(&asymmetric, trees[1].clone()),
        10,
        "node(node(leaf 1, leaf 2), leaf 3): inner = 2·1 + 1 + 1 = 4, \
         root = 2·4 + 1 + 1 = 10; l/r swapped gives 7, a rightmost-leaf \
         payload gives 11"
    );

    // And against the recursive oracle, on the leaf and all three nodes, under
    // both cells.
    for tree in core::iter::once(BinaryTree::leaf(7_u8)).chain(trees.iter().cloned()) {
        assert_eq!(
            RecursiveNn::unroll(&cell, tree.clone()),
            unroll_tree_via_free_mnd(&cell, tree_to_free_mnd(tree.clone())),
            "RecursiveNn::unroll(cell, {tree:?}) must equal the recursive oracle"
        );
        assert_eq!(
            RecursiveNn::unroll(&asymmetric, tree.clone()),
            unroll_tree_via_free_mnd(&asymmetric, tree_to_free_mnd(tree.clone())),
            "RecursiveNn::unroll(asymmetric, {tree:?}) must equal the recursive oracle"
        );
    }

    // Issue #200: a caterpillar of `d` leaves has `d − 1` internal nodes, so
    // the value is `2d − 1`. Under the pre-#200 recursive walk this fixture
    // aborted the harness rather than failing an assertion.
    let deep = i64::try_from(DEEP).expect("DEEP fits in i64");
    assert_eq!(
        RecursiveNn::unroll(&cell, spine_tree(DEEP)),
        2 * deep - 1,
        "a DEEP caterpillar unrolls with the same semantics as a shallow one"
    );
}

/// **`UnfoldingRnn::unroll_to_vec` is the finite prefix of the coalgebra hom**
/// (CDL Remark H.6, App I.3, Example J.2): the bounded unroll equals the
/// `Cofree<OptionWitness, i64>` prefix unfolded from the same seed and step, on
/// four seed/depth pairs including `depth = 0`, and carries the hand-computed
/// counter prefixes.
#[test]
fn unfolding_rnn_unroll_equals_the_cofree_walker() {
    type CellO = fn((i64, i64)) -> i64;
    type CellN = fn((i64, i64)) -> i64;
    let cell: UnfoldingRnn<i64, i64, CellO, CellN, i64> =
        UnfoldingRnn::new(0_i64, |(_p, s)| s, |(_p, s)| s + 1);

    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, 0_i64, 5),
        vec![0_i64, 1, 2, 3, 4],
        "counter unroll [0..5] from initial state 0"
    );
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, 0_i64, 0),
        Vec::<i64>::new(),
        "depth = 0 returns an empty vec"
    );
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, 7_i64, 1),
        vec![7_i64],
        "depth = 1 returns just the initial-state output"
    );
    assert_eq!(
        UnfoldingRnn::unroll_to_vec(&cell, -2_i64, 4),
        vec![-2_i64, -1, 0, 1],
        "depth = 4 from a negative seed"
    );

    let step = |s: i64| {
        (
            (cell.cell_o)((cell.parameter, s)),
            (cell.cell_n)((cell.parameter, s)),
        )
    };
    for (seed, depth) in [(0_i64, 5_usize), (7, 1), (-2, 4), (0, 0)] {
        assert_eq!(
            UnfoldingRnn::unroll_to_vec(&cell, seed, depth),
            cofree_prefix_to_vec(unfold_stream(seed, step, depth)),
            "unroll_to_vec(cell, {seed}, {depth}) must equal the Cofree prefix walk"
        );
    }
}

/// **`MealyCell::run` is the input-driven coalgebra prefix** (CDL Remark H.6,
/// App I.4): it equals the `Cofree<OptionWitness, i64>` walk of length
/// `inputs.len()` on three seed/input pairs including the empty one, and
/// carries the passthrough and stateful-counter values.
#[test]
fn mealy_cell_run_equals_the_cofree_walker() {
    // Passthrough: cell((_, s)) = |i| (i, s) — output is the input, state fixed.
    let passthrough: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (i, s));
    assert_eq!(
        MealyCell::run(&passthrough, 0_i64, vec![1_i64, 2, 3]),
        vec![1_i64, 2, 3],
        "passthrough Mealy: output = input"
    );
    assert_eq!(
        MealyCell::run(&passthrough, 999_i64, vec![-7_i64, 0, 42]),
        vec![-7_i64, 0, 42],
        "passthrough preserves arbitrary inputs"
    );
    assert_eq!(
        MealyCell::run(&passthrough, 0_i64, Vec::<i64>::new()),
        Vec::<i64>::new(),
        "empty input → empty output"
    );

    // Stateful counter: emit s + i, increment s.
    let cell: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (s + i, s + 1));
    assert_eq!(
        MealyCell::run(&cell, 0_i64, vec![10_i64, 20, 30]),
        vec![10_i64, 21, 32],
        "Mealy stateful counter from initial 0"
    );
    assert_eq!(
        MealyCell::run(&cell, 5_i64, vec![1_i64, 1, 1]),
        vec![6_i64, 7, 8],
        "…and from initial 5"
    );

    let step = |s: i64, i: i64| ((cell.cell)((cell.parameter, s)))(i);
    for (seed, inputs) in [
        (0_i64, vec![10_i64, 20, 30]),
        (5, vec![1, 1, 1]),
        (0, Vec::<i64>::new()),
    ] {
        assert_eq!(
            MealyCell::run(&cell, seed, inputs.clone()),
            cofree_prefix_to_vec(unfold_driven(seed, inputs.clone(), step)),
            "MealyCell::run(cell, {seed}, {inputs:?}) must equal the Cofree prefix walk"
        );
    }
}

/// **`MooreCell::run` is the output-then-step coalgebra prefix** (CDL Remark
/// H.6, App I.5): it equals the `Cofree<OptionWitness, i64>` walk on three
/// seed/input pairs including the empty one, and carries the hand-computed
/// output-before-consume sequences — the first output comes from the *initial*
/// state, which is the Moore-vs-Mealy distinction.
#[test]
fn moore_cell_run_equals_the_cofree_walker() {
    type CellO = fn(((), i64)) -> i64;
    type CellN = fn(((), i64, ())) -> i64;
    let cell: MooreCell<(), i64, CellO, CellN, (), i64> =
        MooreCell::new((), |((), s)| s * 2, |((), s, ())| s + 1);

    assert_eq!(
        MooreCell::run(&cell, 0_i64, vec![(); 3]),
        vec![0_i64, 2, 4],
        "Moore output-then-step from initial 0 over 3 inputs"
    );
    assert_eq!(
        MooreCell::run(&cell, 10_i64, vec![(); 4]),
        vec![20_i64, 22, 24, 26],
        "…from initial 10: outputs are 2s for s = 10, 11, 12, 13"
    );
    assert_eq!(
        MooreCell::run(&cell, 7_i64, Vec::<()>::new()),
        Vec::<i64>::new(),
        "empty input → empty output"
    );

    let step = |s: i64, i: ()| {
        (
            (cell.cell_o)((cell.parameter, s)),
            (cell.cell_n)((cell.parameter, s, i)),
        )
    };
    for (seed, inputs) in [
        (0_i64, vec![(); 3]),
        (10, vec![(); 4]),
        (7, Vec::<()>::new()),
    ] {
        assert_eq!(
            MooreCell::run(&cell, seed, inputs.clone()),
            cofree_prefix_to_vec(unfold_driven(seed, inputs.clone(), step)),
            "MooreCell::run(cell, {seed}, {} inputs) must equal the Cofree prefix walk",
            inputs.len()
        );
    }
}

/// **The lazy surfaces advance exactly once per pulled item** (issue #314;
/// lazy-unroll surface #36).
///
/// For each of `UnfoldingRnn::unroll_iter`, `MealyCell::run_iter` and
/// `MooreCell::run_iter`, a `Cell` counter inside the cell observes the number
/// of calls under `.take(k)`, and the assertion is equality with `k` — the
/// count an eager implementation, one that runs the whole input before handing
/// back an iterator, does not produce for a `k` below the input length.
/// Prefix agreement with the eager surface and with the `Cofree` walk is
/// asserted alongside, and `UnfoldingRnn`'s row adds a `cell_n` that panics
/// past a bound the bounded `.take` never reaches.
#[test]
fn lazy_iterators_advance_exactly_once_per_pulled_item() {
    // --- UnfoldingRnn: prefix agreement, then the counter -------------------
    type CellO = fn((i64, i64)) -> i64;
    type CellN = fn((i64, i64)) -> i64;
    let cell: UnfoldingRnn<i64, i64, CellO, CellN, i64> =
        UnfoldingRnn::new(0_i64, |(_p, s)| s, |(_p, s)| s + 1);
    let step = |s: i64| {
        (
            (cell.cell_o)((cell.parameter, s)),
            (cell.cell_n)((cell.parameter, s)),
        )
    };
    for (seed, n) in [(0_i64, 0_usize), (0, 1), (0, 5), (7, 1), (-2, 4)] {
        let via_iter: Vec<i64> = UnfoldingRnn::unroll_iter(&cell, seed).take(n).collect();
        assert_eq!(
            via_iter,
            UnfoldingRnn::unroll_to_vec(&cell, seed, n),
            "unroll_iter({seed}).take({n}) must agree with unroll_to_vec({seed}, {n})"
        );
        assert_eq!(
            via_iter,
            cofree_prefix_to_vec(unfold_stream(seed, step, n)),
            "unroll_iter({seed}).take({n}) must agree with the Cofree prefix walk"
        );
    }

    // A `cell_n` that asserts the state stays under a bound: a bounded `.take`
    // advances only through states 0..5 and never trips it.
    let guarded = UnfoldingRnn::new(
        0_i64,
        |(_p, s): (i64, i64)| s,
        |(_p, s): (i64, i64)| {
            assert!(
                s < 100,
                "unroll_iter over-evaluated the infinite tail past the bound"
            );
            s + 1
        },
    );
    assert_eq!(
        UnfoldingRnn::unroll_iter(&guarded, 0)
            .take(5)
            .collect::<Vec<i64>>(),
        vec![0_i64, 1, 2, 3, 4],
        "take(5) yields the first five outputs without tripping the bound guard"
    );

    for n in [0_usize, 1, 5] {
        let outputs = core::cell::Cell::new(0_usize);
        let advances = core::cell::Cell::new(0_usize);
        let counting = UnfoldingRnn::new(
            0_i64,
            |(_p, s): (i64, i64)| {
                outputs.set(outputs.get() + 1);
                s
            },
            |(_p, s): (i64, i64)| {
                advances.set(advances.get() + 1);
                s + 1
            },
        );
        let _ = UnfoldingRnn::unroll_iter(&counting, 0)
            .take(n)
            .collect::<Vec<_>>();
        let via_iter = (outputs.get(), advances.get());
        outputs.set(0);
        advances.set(0);
        let _ = UnfoldingRnn::unroll_to_vec(&counting, 0, n);
        let via_vec = (outputs.get(), advances.get());
        assert_eq!(
            via_iter, via_vec,
            "unroll_iter.take({n}) must call cell_o and cell_n exactly as often as \
             unroll_to_vec(_, {n}) (observed {via_iter:?}, expected {via_vec:?})"
        );
        assert_eq!(
            via_iter,
            (n, n),
            "UnfoldingRnn::unroll_iter must call cell_o and cell_n exactly once each per \
             pulled item (observed {via_iter:?}, expected ({n}, {n}))"
        );
    }

    // --- MealyCell: prefix agreement, then the counter ----------------------
    let mealy: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (s + i, s + 1));
    let mealy_step = |s: i64, i: i64| ((mealy.cell)((mealy.parameter, s)))(i);
    for (seed, inputs) in [
        (0_i64, vec![10_i64, 20, 30]),
        (5, vec![1, 1, 1]),
        (0, Vec::<i64>::new()),
    ] {
        let via_run = MealyCell::run(&mealy, seed, inputs.clone());
        let via_iter: Vec<i64> = MealyCell::run_iter(&mealy, seed, inputs.clone()).collect();
        assert_eq!(
            via_iter, via_run,
            "MealyCell::run_iter(cell, {seed}, {inputs:?}).collect() must equal run"
        );
        for k in 0..=inputs.len() {
            let prefix: Vec<i64> = MealyCell::run_iter(&mealy, seed, inputs.clone())
                .take(k)
                .collect();
            assert_eq!(
                prefix,
                via_run.iter().copied().take(k).collect::<Vec<i64>>(),
                "run_iter take({k}) prefix agrees with run"
            );
        }
        assert_eq!(
            via_iter,
            cofree_prefix_to_vec(unfold_driven(seed, inputs.clone(), mealy_step)),
            "run_iter output must equal the Cofree prefix walk"
        );
    }

    // The counter, over a five-item input so a short `.take` is separable from
    // full consumption.
    for k in [0_usize, 1, 3] {
        let calls = core::cell::Cell::new(0_usize);
        let counting: MealyCell<(), i64, _, i64, i64> = MealyCell::new((), |((), s): ((), i64)| {
            calls.set(calls.get() + 1);
            move |i: i64| (s + i, s + 1)
        });
        let pulled: Vec<i64> = MealyCell::run_iter(&counting, 0_i64, vec![1_i64; 5])
            .take(k)
            .collect();
        assert_eq!(pulled.len(), k, "run_iter yielded exactly the pulled items");
        assert_eq!(
            calls.get(),
            k,
            "MealyCell::run_iter must call its cell exactly once per pulled item \
             (observed {}, expected {k} over a 5-item input)",
            calls.get()
        );
    }

    // --- MooreCell: prefix agreement, then the counters ---------------------
    let moore: MooreCell<(), i64, CellO2, CellN2, (), i64> =
        MooreCell::new((), |((), s)| s * 2, |((), s, ())| s + 1);
    let moore_step = |s: i64, i: ()| {
        (
            (moore.cell_o)((moore.parameter, s)),
            (moore.cell_n)((moore.parameter, s, i)),
        )
    };
    for (seed, inputs) in [
        (0_i64, vec![(); 3]),
        (10, vec![(); 4]),
        (7, Vec::<()>::new()),
    ] {
        let via_run = MooreCell::run(&moore, seed, inputs.clone());
        let via_iter: Vec<i64> = MooreCell::run_iter(&moore, seed, inputs.clone()).collect();
        assert_eq!(
            via_iter,
            via_run,
            "MooreCell::run_iter(cell, {seed}, {} inputs).collect() must equal run",
            inputs.len()
        );
        for k in 0..=inputs.len() {
            let prefix: Vec<i64> = MooreCell::run_iter(&moore, seed, inputs.clone())
                .take(k)
                .collect();
            assert_eq!(
                prefix,
                via_run.iter().copied().take(k).collect::<Vec<i64>>(),
                "run_iter take({k}) prefix agrees with run"
            );
        }
        assert_eq!(
            via_iter,
            cofree_prefix_to_vec(unfold_driven(seed, inputs.clone(), moore_step)),
            "run_iter output must equal the Cofree prefix walk"
        );
    }

    // Both maps counted: output-then-step means one of each per pulled item.
    for k in [0_usize, 1, 3] {
        let outputs = core::cell::Cell::new(0_usize);
        let steps = core::cell::Cell::new(0_usize);
        let counting: MooreCell<(), i64, _, _, (), i64> = MooreCell::new(
            (),
            |((), s): ((), i64)| {
                outputs.set(outputs.get() + 1);
                s * 2
            },
            |((), s, ()): ((), i64, ())| {
                steps.set(steps.get() + 1);
                s + 1
            },
        );
        let pulled: Vec<i64> = MooreCell::run_iter(&counting, 0_i64, vec![(); 5])
            .take(k)
            .collect();
        assert_eq!(pulled.len(), k, "run_iter yielded exactly the pulled items");
        assert_eq!(
            (outputs.get(), steps.get()),
            (k, k),
            "MooreCell::run_iter must call cell_o and cell_n exactly once each per \
             pulled item (observed ({}, {}), expected ({k}, {k}) over a 5-item input)",
            outputs.get(),
            steps.get()
        );
    }
}

/// Moore cell-map types, spelled once for the arm above.
type CellO2 = fn(((), i64)) -> i64;
type CellN2 = fn(((), i64, ())) -> i64;

/// An `Iterator` that is not a `Vec` iterator: it yields `remaining` clones of
/// `item` and counts every `next()` call, the exhausting one included, in a
/// borrowed `Cell`.
struct PullCounter<'c, I> {
    pulls: &'c core::cell::Cell<usize>,
    remaining: usize,
    item: I,
}

impl<I: Clone> Iterator for PullCounter<'_, I> {
    type Item = I;

    fn next(&mut self) -> Option<I> {
        self.pulls.set(self.pulls.get() + 1);
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(self.item.clone())
    }
}

/// **The two `run_iter`s pull a non-`Vec` source exactly once per item**
/// (issue #314, the `It: IntoIterator` half).
///
/// `MealyCell::run_iter` and `MooreCell::run_iter` are fed a `PullCounter`
/// instead of a `Vec`, `.take(k)` for `k` in `{0, 1, 3}` against a five-item
/// source, and the assertion is that the source was pulled exactly `k` times —
/// the count an implementation that drains its input at construction does not
/// produce for a `k` below the source length. Neither `run_iter` peeks: the
/// measured count is `k`, not `k + 1`.
#[test]
fn run_iter_pulls_a_non_vec_source_exactly_once_per_item() {
    let mealy: MealyCell<(), i64, _, i64, i64> =
        MealyCell::new((), |((), s): ((), i64)| move |i: i64| (s + i, s + 1));
    let moore: MooreCell<(), i64, CellO2, CellN2, (), i64> =
        MooreCell::new((), |((), s)| s * 2, |((), s, ())| s + 1);

    for k in [0_usize, 1, 3] {
        let mealy_pulls = core::cell::Cell::new(0_usize);
        let pulled: Vec<i64> = MealyCell::run_iter(
            &mealy,
            0_i64,
            PullCounter {
                pulls: &mealy_pulls,
                remaining: 5,
                item: 1_i64,
            },
        )
        .take(k)
        .collect();
        assert_eq!(pulled.len(), k, "run_iter yielded exactly the pulled items");
        assert_eq!(
            mealy_pulls.get(),
            k,
            "MealyCell::run_iter must pull a non-`Vec` source exactly once per pulled \
             item (observed {}, expected {k} over a 5-item source)",
            mealy_pulls.get()
        );

        let moore_pulls = core::cell::Cell::new(0_usize);
        let pulled: Vec<i64> = MooreCell::run_iter(
            &moore,
            0_i64,
            PullCounter {
                pulls: &moore_pulls,
                remaining: 5,
                item: (),
            },
        )
        .take(k)
        .collect();
        assert_eq!(pulled.len(), k, "run_iter yielded exactly the pulled items");
        assert_eq!(
            moore_pulls.get(),
            k,
            "MooreCell::run_iter must pull a non-`Vec` source exactly once per pulled \
             item (observed {}, expected {k} over a 5-item source)",
            moore_pulls.get()
        );
    }
}

/// **GDL recovery at the architecture level** (CDL Example 2.6): a
/// `Z2`-invariant aggregator `(p, a, s) ↦ s + |a|` makes `FoldingRnn::unroll`
/// invariant under the pointwise negation action on `Vec<i64>`, while the
/// non-invariant `(p, a, s) ↦ s + a` separates the two orbits — so the
/// invariance assertion is not one every cell would satisfy.
#[test]
fn gdl_recovery_via_z2_invariant_folding() {
    type Cell0 = fn(()) -> i64;
    type Cell1 = fn(((), i64, i64)) -> i64;
    let invariant: FoldingRnn<(), i64, Cell0, Cell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| s + a.abs());

    let positive = vec![1_i64, -2, 3];
    let negated: Vec<i64> = positive.iter().map(|v| -v).collect();
    assert_eq!(
        FoldingRnn::unroll(&invariant, positive.clone()),
        FoldingRnn::unroll(&invariant, negated.clone()),
        "a Z2-invariant cell: unroll([1, -2, 3]) must equal unroll([-1, 2, -3])"
    );
    assert_eq!(
        FoldingRnn::unroll(&invariant, positive.clone()),
        6,
        "the concrete invariant fold value: |1| + |-2| + |3|"
    );

    let non_invariant: FoldingRnn<(), i64, Cell0, Cell1, i64> =
        FoldingRnn::new((), |()| 0_i64, |((), a, s)| s + a);
    let pos = FoldingRnn::unroll(&non_invariant, positive);
    let neg = FoldingRnn::unroll(&non_invariant, negated);
    assert_ne!(
        pos, neg,
        "a non-invariant cell must distinguish [1, -2, 3] from [-1, 2, -3]"
    );
    assert_eq!(pos, 2);
    assert_eq!(neg, -2);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// `FoldingRnn::unroll` equals the `Free` tower walk on every generated
    /// `Vec<i64>` up to 16 elements. `wrapping_add` so arbitrary payloads
    /// cannot overflow; both legs use the same cell.
    #[test]
    fn folding_rnn_free_mnd_equivalence_proptest(
        input in prop::collection::vec(any::<i64>(), 0..=16),
    ) {
        let cell: FoldingRnn<(), i64, ListCell0, ListCell1, i64> =
            FoldingRnn::new((), |()| 0_i64, |((), a, s)| a.wrapping_add(s));
        let direct = FoldingRnn::unroll(&cell, input.clone());
        let via_free_mnd = unroll_list_via_free_mnd(&cell, vec_to_free_mnd(input, ()));
        prop_assert_eq!(direct, via_free_mnd);
    }

    /// `RecursiveNn::unroll` equals the recursive `Free<TreeEndo, Infallible>`
    /// walk on every generated bounded `BinaryTree<u8>`.
    #[test]
    fn recursive_nn_free_mnd_equivalence_proptest(tree in arb_binary_tree()) {
        let cell: RecursiveNn<i64, i64, TreeCell0, TreeCell1, u8> =
            RecursiveNn::new(1_i64, |p| p, |(_p, _a, l, r)| l + r + 1);
        let direct = RecursiveNn::unroll(&cell, tree.clone());
        let via_free_mnd = unroll_tree_via_free_mnd(&cell, tree_to_free_mnd(tree));
        prop_assert_eq!(direct, via_free_mnd);
    }

    /// `UnfoldingRnn::unroll_to_vec` equals the `Cofree` prefix walk on every
    /// seed and depth up to 16.
    #[test]
    fn unfolding_rnn_cofree_equivalence_proptest(
        seed in any::<i64>(),
        depth in 0..=16_usize,
    ) {
        type CellO = fn((i64, i64)) -> i64;
        type CellN = fn((i64, i64)) -> i64;
        let cell: UnfoldingRnn<i64, i64, CellO, CellN, i64> =
            UnfoldingRnn::new(0_i64, |(_p, s)| s, |(_p, s)| s.wrapping_add(1));
        let step = |s: i64| ((cell.cell_o)((cell.parameter, s)), (cell.cell_n)((cell.parameter, s)));
        let direct = UnfoldingRnn::unroll_to_vec(&cell, seed, depth);
        let via_cofree = cofree_prefix_to_vec(unfold_stream(seed, step, depth));
        prop_assert_eq!(direct, via_cofree);
    }

    /// `MealyCell::run` equals the input-driven `Cofree` prefix walk on every
    /// seed and input sequence up to 16 elements.
    #[test]
    fn mealy_cell_cofree_equivalence_proptest(
        seed in any::<i64>(),
        inputs in prop::collection::vec(any::<i64>(), 0..=16),
    ) {
        let cell: MealyCell<(), i64, _, i64, i64> = MealyCell::new(
            (),
            |((), s): ((), i64)| move |i: i64| (s.wrapping_add(i), s.wrapping_add(1)),
        );
        let step = |s: i64, i: i64| ((cell.cell)((cell.parameter, s)))(i);
        let direct = MealyCell::run(&cell, seed, inputs.clone());
        let via_cofree = cofree_prefix_to_vec(unfold_driven(seed, inputs, step));
        prop_assert_eq!(direct, via_cofree);
    }

    /// `MooreCell::run` equals the output-then-step `Cofree` prefix walk on
    /// every seed and input length up to 16.
    #[test]
    fn moore_cell_cofree_equivalence_proptest(
        seed in any::<i64>(),
        len in 0..=16_usize,
    ) {
        let cell: MooreCell<(), i64, CellO2, CellN2, (), i64> =
            MooreCell::new((), |((), s)| s.wrapping_mul(2), |((), s, ())| s.wrapping_add(1));
        let step =
            |s: i64, i: ()| ((cell.cell_o)((cell.parameter, s)), (cell.cell_n)((cell.parameter, s, i)));
        let inputs = vec![(); len];
        let direct = MooreCell::run(&cell, seed, inputs.clone());
        let via_cofree = cofree_prefix_to_vec(unfold_driven(seed, inputs, step));
        prop_assert_eq!(direct, via_cofree);
    }

    /// The lazy `unroll_iter(seed).take(depth)` prefix equals
    /// `unroll_to_vec(seed, depth)` on every seed and depth up to 16.
    #[test]
    fn unfolding_rnn_unroll_iter_equivalence_proptest(
        seed in any::<i64>(),
        depth in 0..=16_usize,
    ) {
        type CellO = fn((i64, i64)) -> i64;
        type CellN = fn((i64, i64)) -> i64;
        let cell: UnfoldingRnn<i64, i64, CellO, CellN, i64> =
            UnfoldingRnn::new(0_i64, |(_p, s)| s, |(_p, s)| s.wrapping_add(1));
        let via_iter: Vec<i64> = UnfoldingRnn::unroll_iter(&cell, seed).take(depth).collect();
        let via_vec = UnfoldingRnn::unroll_to_vec(&cell, seed, depth);
        prop_assert_eq!(via_iter, via_vec);
    }

    /// Full consumption of `MealyCell::run_iter` equals `run` on every seed and
    /// input sequence up to 16 elements.
    #[test]
    fn mealy_cell_run_iter_equivalence_proptest(
        seed in any::<i64>(),
        inputs in prop::collection::vec(any::<i64>(), 0..=16),
    ) {
        let cell: MealyCell<(), i64, _, i64, i64> = MealyCell::new(
            (),
            |((), s): ((), i64)| move |i: i64| (s.wrapping_add(i), s.wrapping_add(1)),
        );
        let via_iter: Vec<i64> = MealyCell::run_iter(&cell, seed, inputs.clone()).collect();
        let via_run = MealyCell::run(&cell, seed, inputs);
        prop_assert_eq!(via_iter, via_run);
    }

    /// Full consumption of `MooreCell::run_iter` equals `run` on every seed and
    /// input length up to 16.
    #[test]
    fn moore_cell_run_iter_equivalence_proptest(
        seed in any::<i64>(),
        len in 0..=16_usize,
    ) {
        let cell: MooreCell<(), i64, CellO2, CellN2, (), i64> =
            MooreCell::new((), |((), s)| s.wrapping_mul(2), |((), s, ())| s.wrapping_add(1));
        let inputs = vec![(); len];
        let via_iter: Vec<i64> = MooreCell::run_iter(&cell, seed, inputs.clone()).collect();
        let via_run = MooreCell::run(&cell, seed, inputs);
        prop_assert_eq!(via_iter, via_run);
    }
}
