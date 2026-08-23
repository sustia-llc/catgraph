//! #289 regression pins for the checked boundary-node mutators.
//!
//! Four separable claims, pinned separately:
//!
//! 1. The `add_boundary_node` family **rejects** an out-of-bounds apex index in
//!    every build profile and leaves the value untouched when it does.
//! 2. `Cospan::is_left_identity` / `is_right_identity` answer
//!    `leg.len() == middle.len() && represents_id(leg)` — including the length
//!    conjunct, which is the half that keeps being dropped — and they answer it
//!    about the value in hand rather than about how it was built.
//!
//!    ⚠ **This claim shrank twice.** #289 opened with two cached `bool` fields
//!    that four mutators left a stale `true`, and `perform_pushout` selected
//!    its fast paths from them, so a stale flag was a *wrong composition*. The
//!    r4 review made composition derive the predicate from the legs; the r5
//!    review then deleted the cache outright. What is left is a pair of
//!    accessors that compute the predicate on demand, so the *stale-flag* tests
//!    that used to live here are gone (they would compare a function with
//!    itself) and the surviving ones assert legs, apexes and composites —
//!    hand-written expectations that never went through the cache. Docstrings
//!    below still record composites measured against the pre-fix code; read
//!    them as history, not as a claim about what today's code would do.
//! 3. The remaining panicking preconditions name the invariant they enforce,
//!    in every build profile.
//! 4. `connect_pair`'s leg remap merges the two ports in **every** argument
//!    order — a pre-existing defect surfaced by #289's review left both legs
//!    out of bounds whenever node 1's apex vertex was the last index.
//!
//! Every `#[should_panic]` below is on a plain `assert!`, not a
//! `debug_assert!`, so this file states release behaviour too — and CI runs it
//! under `--release` as well as debug (`.github/workflows/ci.yml`, the
//! `release-test` job's `cargo test -p catgraph --test checked_mutators
//! --release` step). Keep the two in step: a claim about release behaviour that
//! only ever runs in debug is unpinned.

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    errors::{BoundaryLeg, CatgraphError},
    finset::from_cycle,
    monoidal::SymmetricMonoidalMorphism,
    named_cospan::NamedCospan,
    span::Span,
    utils::remove_multiple,
};
use either::Either::{Left, Right};
use permutations::Permutation;

// ===========================================================================
// 1. Bounds: the mutator rejects, and does not half-mutate
// ===========================================================================

/// `Cospan::add_boundary_node_known_target` refuses an out-of-bounds apex index
/// on either leg, naming the leg, the position the node would have taken, the
/// target and the apex size — and leaves the cospan byte-for-byte as it was.
///
/// **What this ranges over.** The `target` axis is swept across the whole
/// interesting range (`middle.len()`, one past it, a large finite index, and
/// `usize::MAX`) on *both* legs; it does **not** sweep apex sizes, leg
/// contents, or the `Right(label)` arm — that arm mints its own index and has
/// no precondition to violate.
///
/// Before #289 every one of these returned `Ok` and pushed the index into the
/// leg: `add_boundary_node` had no check at all, weaker than `new_unchecked`,
/// which at least `debug_assert!`s the same invariant.
#[test]
fn cospan_add_boundary_node_rejects_out_of_bounds_targets_on_both_legs() {
    let apex_len = 2;
    for target in [apex_len, apex_len + 1, 7, usize::MAX] {
        for (leg, arrow) in [
            (BoundaryLeg::Domain, Left(target)),
            (BoundaryLeg::Codomain, Right(target)),
        ] {
            let mut c = Cospan::<char>::new(vec![0, 1], vec![0, 1], vec!['a', 'b'])
                .expect("the fixture legs are in bounds");
            let err = c
                .add_boundary_node_known_target(arrow)
                .expect_err("an apex index at or beyond middle.len() must be refused");
            assert_eq!(
                err,
                CatgraphError::ConstructionIndexOutOfBounds {
                    leg,
                    position: 2,
                    target,
                    target_len: apex_len,
                },
                "target {target} on the {leg} leg reported the wrong location"
            );

            // No half-mutation: neither leg grew and the apex did not grow.
            assert_eq!(
                c.left_to_middle(),
                &[0, 1],
                "the domain leg must be untouched on Err (pre-#289 it grew to {:?})",
                {
                    let mut grown = vec![0, 1];
                    if leg == BoundaryLeg::Domain {
                        grown.push(target);
                    }
                    grown
                }
            );
            assert_eq!(
                c.right_to_middle(),
                &[0, 1],
                "the codomain leg must be untouched on Err"
            );
            assert_eq!(c.middle(), &['a', 'b'], "the apex must be untouched on Err");
        }
    }
}

/// The exact message, once, so a reword is a visible diff rather than a silent
/// change to what a consumer reads.
#[test]
fn cospan_add_boundary_node_error_renders_the_leg_position_target_and_size() {
    let mut c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).expect("in-bounds fixture");
    let err = c
        .add_boundary_node_known_target(Right(4))
        .expect_err("apex index 4 is out of bounds for a 1-vertex apex");
    assert_eq!(
        err.to_string(),
        "construction error: codomain leg entry 1 targets index 4, but the target set has 1 element(s)"
    );
}

// ===========================================================================
// 2. The identity accessors
// ===========================================================================

/// `is_left_identity` / `is_right_identity` require the leg to cover the
/// **whole** apex, not merely to start `[0, 1, …]`.
///
/// `represents_id` tests only `leg[i] == i` for the entries present, so without
/// the `leg.len() == middle.len()` conjunct a leg strictly shorter than the
/// apex passes. Three of the four #289 flag defects were exactly that — a
/// hand-written re-spelling missing the conjunct — and `Cospan::assert_valid`'s
/// retired strong arm made the same omission in the other direction, rejecting
/// `Cospan::new(vec![0], vec![0, 1], vec!['a', 'b'])`, a perfectly valid
/// cospan, for having a correctly-`false` left flag.
///
/// **What this ranges over.** One 2-vertex apex, both legs, both answers, plus
/// an out-of-order leg and the empty cospan as the degenerate case. It does not
/// sweep apex sizes. It says nothing about legs *longer* than the apex — legal
/// (`Cospan::new(vec![0, 0], vec![], vec!['a'])`) but a case where the length
/// conjunct is not what decides the answer, so it would not exercise this
/// claim; `Cospan::add_middle`'s docs carry that shape.
///
/// Both short-leg fixtures are short **and in order**, which is the whole
/// point: only such a leg separates the two conjuncts. This mirror read
/// `right == [1]` until the sixth review — an assertion that held for an
/// unrelated reason, since `represents_id([1])` fails on its own, so the
/// conjunct was never what it tested.
///
/// Three measured falsifications:
///
/// 1. Deleting `leg.len() == apex_len` from the private `leg_is_identity`
///    reddens **9 of this file's 27 tests** — `perform_pushout` reads the same
///    predicate — this one first, at its opening assertion. The count is 9 with
///    either mirror fixture; the mirror is not what makes the file sensitive.
/// 2. That is because the *first* assertion masks the mirror. Under the same
///    perturbation, with the domain assertion neutralised, the run fails on the
///    mirror instead ("right == [0] misses apex vertex 1") — so the mirror is
///    load-bearing rather than decorative. With the old `right == [1]` fixture
///    in its place, that same run **passes**: the pre-r6 mirror was vacuous
///    with respect to the conjunct.
/// 3. Sharper still, dropping the conjunct from `is_right_identity` **alone**
///    (leaving `leg_is_identity` and hence `is_left_identity` and
///    `perform_pushout` intact) reddens **4** of the 27 with the old fixture and
///    **5** with this one — this test being the fifth. So the file was never
///    blind to a conjunct-less codomain accessor;
///    `cospan_delete_boundary_node_keeps_composition_correct`,
///    `cospan_unknown_target_add_grows_the_apex_past_the_partner_leg` and the
///    two `connect_pair` tests all carry a short-and-in-order codomain leg
///    incidentally. What was missing was a test that fails *for that reason*,
///    which is what this one is for.
#[test]
fn cospan_identity_accessors_need_the_leg_to_cover_the_whole_apex() {
    // The CHANGELOG's own witness: `represents_id([0])` is true, the leg is
    // still not an identity on a 2-vertex apex.
    let short_left = Cospan::<char>::new(vec![0], vec![0, 1], vec!['a', 'b'])
        .expect("a leg may be shorter than the apex");
    assert!(
        !short_left.is_left_identity(),
        "left == [0] misses apex vertex 1; without the length conjunct this \
         reports true"
    );
    assert!(
        short_left.is_right_identity(),
        "right == [0, 1] does cover the 2-vertex apex"
    );

    // The mirror, so the two accessors cannot be checked by one expression —
    // and, like the domain case above, short but *in order*. `right == [1]`
    // would be the easier fixture to write and would be worthless here:
    // `represents_id([1])` already fails, so it would redden for the wrong
    // reason and a conjunct-less `is_right_identity` would still pass it.
    let short_right = Cospan::<char>::new(vec![0, 1], vec![0], vec!['a', 'b'])
        .expect("a leg may be shorter than the apex");
    assert!(short_right.is_left_identity());
    assert!(
        !short_right.is_right_identity(),
        "right == [0] misses apex vertex 1; without the length conjunct this \
         reports true"
    );

    // A leg that covers the apex out of order is not an identity either.
    let reversed =
        Cospan::<char>::new(vec![1, 0], vec![0, 1], vec!['a', 'b']).expect("in-bounds fixture");
    assert!(
        !reversed.is_left_identity(),
        "left == [1, 0] is not in order"
    );

    // Degenerate: an empty leg over an empty apex covers it vacuously.
    let empty = Cospan::<char>::empty();
    assert!(empty.is_left_identity() && empty.is_right_identity());
}

/// The identity accessors are a function of the value, not of how it was built.
///
/// Until #289 deleted the cache, the round trip below returned a cospan whose
/// domain leg *was* the identity while `is_left_identity()` said `false` — the
/// mutators could only ever clear the flag, never restore it, so the answer
/// depended on the value's history. The retired test that pinned that
/// behaviour (`cospan_identity_flags_are_conservative_in_the_false_direction`)
/// asserted exactly the negation of the last assertion below.
///
/// **What this ranges over.** Three routes by which the old cache reported a
/// conservative `false` over a leg that was the identity — a delete-then-add
/// round trip, the two permutation constructors' hard-coded flags, and
/// `permute_side` with an identity permutation — on one apex size each, and
/// only that direction of the history-independence claim, which is the one
/// #289 changed. The *other* direction (a writer leaving a stale `true`) has
/// no fixture here because it has no reachable state: the accessor reads the
/// leg. It does not sweep other mutator sequences or non-identity
/// permutations.
///
/// Two measured falsifications, which is what separates this from a
/// restatement of the `assert_eq!`s beside it: dropping the length conjunct
/// from the private `leg_is_identity` reddens the **first** assertion (`[0]`
/// over a 2-vertex apex starts reporting `true`), and any accessor that goes
/// back to reading a clear-only cache reddens all four `true` assertions —
/// which is the state this branch started from, where the three routes below
/// reported `false`, `false` and `false`.
#[test]
fn cospan_identity_accessors_ignore_how_the_value_was_built() {
    let mut c = Cospan::<char>::identity(&vec!['a', 'b']);
    c.delete_boundary_node(Left(1));
    assert_eq!(c.left_to_middle(), &[0]);
    assert!(
        !c.is_left_identity(),
        "left == [0] on a 2-vertex apex is not the identity"
    );

    c.add_boundary_node_known_target(Left(1))
        .expect("apex index 1 is in bounds for a 2-vertex apex");
    assert_eq!(
        (c.left_to_middle(), c.middle()),
        (&[0, 1][..], &['a', 'b'][..]),
        "`swap_remove` on the last port is a plain pop, so the round trip must \
         restore the leg exactly"
    );
    assert!(
        c.is_left_identity(),
        "the leg is the identity again, so the accessor says so; before #289 \
         deleted the cache this reported `false` — the mutators' `&=` could \
         only clear"
    );

    // The two permutation constructors hard-coded the flag of the leg they do
    // not build, so for `p == identity` both lied `false`.
    let types = ['a', 'b', 'c'];
    let dom = Cospan::<char>::from_permutation_on_domain(Permutation::identity(3), &types)
        .expect("the permutation's length matches the type list");
    assert_eq!(dom.right_to_middle(), &[0, 1, 2]);
    assert!(
        dom.is_right_identity(),
        "the identity permutation leaves the codomain leg [0, 1, 2] over a \
         3-vertex apex; the constructor used to hard-code `is_right_id: false`"
    );
    let cod = Cospan::<char>::from_permutation_on_codomain(Permutation::identity(3), &types)
        .expect("the permutation's length matches the type list");
    assert_eq!(cod.left_to_middle(), &[0, 1, 2]);
    assert!(
        cod.is_left_identity(),
        "the mirror, hard-coded `is_left_id: false`"
    );

    // `permute_side` cleared the permuted leg's flag unconditionally, so an
    // identity permutation — which moves no wire — ended the identity.
    let mut id = Cospan::<char>::identity(&types.to_vec());
    id.permute_side(&Permutation::identity(3), false);
    assert_eq!(id.left_to_middle(), &[0, 1, 2], "no wire moved");
    assert!(
        id.is_left_identity(),
        "an identity permutation cannot end an identity; the unconditional \
         clear used to say it did"
    );
}

/// The boundary case the pre-#289 `&=` let through with the flag still `true`:
/// `tgt_idx == leg.len()`.
///
/// On an identity cospan `leg.len() == middle.len()`, so the one index that
/// satisfies the old test `leg.len() - 1 == tgt_idx` *after* the push is
/// exactly `middle.len()` — out of bounds. The old code therefore pushed an
/// out-of-range entry and kept the codomain flag `true`, which was the silent
/// half of #289. The out-of-range leg entry is the part that still bites: it
/// outlived both the r4 change and the deletion of the cache, since it is the
/// *leg* that is wrong.
///
/// Measured on the pre-#289 body: the call returned `Ok(Right(2))` and
/// `right_to_middle()` became `[0, 1, 2]`, entry 2 out of range for a 2-vertex
/// apex. Post-fix: `Err`, and the leg stays `[0, 1]`.
///
/// **What this ranges over.** One index — the boundary one — on the codomain
/// leg of one fixture. It is deliberately narrow: the general bounds sweep is
/// the test above. What is unique here is that this index is the *only*
/// out-of-bounds value the old arithmetic did not notice.
#[test]
fn cospan_add_boundary_node_rejects_the_boundary_index() {
    let mut c = Cospan::<char>::identity(&vec!['a', 'b']);
    assert_eq!(c.right_to_middle(), &[0, 1]);
    assert_eq!(c.middle().len(), 2);

    // tgt_idx == right.len() == middle.len() == 2.
    let err = c
        .add_boundary_node_known_target(Right(2))
        .expect_err("apex index 2 is out of bounds for a 2-vertex apex");
    assert_eq!(
        err,
        CatgraphError::ConstructionIndexOutOfBounds {
            leg: BoundaryLeg::Codomain,
            position: 2,
            target: 2,
            target_len: 2,
        }
    );
    assert_eq!(
        c.right_to_middle(),
        &[0, 1],
        "the refused entry must not be pushed (pre-#289: [0, 1, 2], with entry 2 out of range)"
    );
}

/// The `Right(label)` arm grows the **apex** as well as the leg it pushes to,
/// so the partner leg is left one short of the apex.
///
/// This is the half of #289 the first pass missed. While the identity answer
/// was cached, that arm updated only the flag of the leg it pushed to, so
/// `add_boundary_node_unknown_target(Right(_))` left a legitimately-`true`
/// `is_left_id` in place with the domain leg now strictly shorter than the apex
/// (and the mirror for `Left(_)`) — reachable through the fully checked API,
/// with no `_unchecked` call and no malformed input, and while `perform_pushout`
/// still read the flags it was a wrong composition (the test below carries the
/// measurement). Both defects are structurally gone: the accessor reads the
/// leg. What is left to pin is the arm's effect on the value, which is what
/// made the stale flag reachable in the first place.
///
/// Measured before the fix: `Cospan::new(vec![0], vec![], vec!['a'])` followed
/// by `add_boundary_node_unknown_target(Right('b'))` reported
/// `is_left_identity() == true` for the resulting `([0], [1], ['a', 'b'])`.
///
/// **What this ranges over.** Both sides of the outer `Either` — the two arms
/// in the source, which are separate expressions — asserting for each that the
/// apex grew by one, that the pushed leg grew with it, and that the partner leg
/// did not. It uses one apex size, and does not range over the `Left(idx)`
/// (known-target) arm, which does not grow the apex and is covered above.
#[test]
fn cospan_unknown_target_add_grows_the_apex_past_the_partner_leg() {
    // Domain-side push: leg and apex grow together; the codomain leg does not.
    let mut c = Cospan::<char>::identity(&vec!['a']);
    c.add_boundary_node_unknown_target(Left('b'));
    assert_eq!(
        (c.left_to_middle(), c.right_to_middle(), c.middle()),
        (&[0, 1][..], &[0][..], &['a', 'b'][..])
    );
    assert!(
        c.is_left_identity(),
        "left == [0, 1] on a 2-vertex apex is still the identity"
    );
    assert!(
        !c.is_right_identity(),
        "right == [0] on a 2-vertex apex misses apex vertex 1"
    );

    // Codomain-side push: the mirror, a separate expression in the source.
    let mut c = Cospan::<char>::identity(&vec!['a']);
    c.add_boundary_node_unknown_target(Right('b'));
    assert_eq!(
        (c.left_to_middle(), c.right_to_middle(), c.middle()),
        (&[0][..], &[0, 1][..], &['a', 'b'][..])
    );
    assert!(
        c.is_right_identity(),
        "right == [0, 1] on a 2-vertex apex is still the identity"
    );
    assert!(
        !c.is_left_identity(),
        "left == [0] on a 2-vertex apex misses apex vertex 1"
    );
}

/// `NamedCospan` delegates the arm, so the apex grows past the partner leg
/// there too.
///
/// **What this ranges over.** One side (the codomain push) on the named
/// wrapper. The wrapper adds a name list and delegates the leg/apex work to
/// `Cospan`, so the arm-by-arm sweep is the test above; what is unique here is
/// that the delegation reaches the fixed code and keeps the name lists in step.
#[test]
fn named_cospan_unknown_target_add_grows_the_apex_past_the_partner_leg() {
    let mut nc = NamedCospan::<char, &str, &str>::new(
        vec![0],
        vec![0],
        vec!['a'],
        vec!["in0"],
        vec!["out0"],
    )
    .expect("in-bounds fixture with one name per port");
    assert!(nc.cospan().is_left_identity());

    nc.add_boundary_node_unknown_target('b', Right("out1"))
        .expect("fresh name, and the apex index is minted by the call");
    assert_eq!(nc.cospan().middle(), &['a', 'b']);
    assert_eq!(nc.cospan().left_to_middle(), &[0]);
    assert_eq!(nc.right_names(), &vec!["out0", "out1"]);
    assert!(
        !nc.cospan().is_left_identity(),
        "left == [0] on a 2-vertex apex misses apex vertex 1 (measured `true` \
         on the named surface before the fix)"
    );
}

/// `NamedCospan::connect_pair` resolves the names and delegates to `Cospan`,
/// so it inherited the last-vertex remap defect — and this is the shape
/// `WiringDiagram::connect_pair` exposes: mint a port with
/// `add_boundary_node_unknown_target` (it lands on the **last** apex vertex),
/// then connect it, passing the new port first.
///
/// **What this ranges over.** Both argument orders of one merge on the named
/// wrapper — the order the remap used to get wrong (new port first, so the
/// last apex index is node 1) and its reverse (last apex index in node 2) —
/// on one fixture, one apex size, one leg. The two orders run the same
/// assertions from one loop body, so they cannot drift apart. Legs are checked
/// by hand and the ports by `map_to_same`, because a rebuild of the mutated
/// value cannot see a remap error. Codomain-side merges, mixed legs and larger
/// apexes are swept by the `Cospan` tests above, not here.
#[test]
fn named_cospan_connect_pair_merges_in_either_order() {
    for (first, second) in [("in1", "in0"), ("in0", "in1")] {
        let mut nc = NamedCospan::<char, &str, &str>::new(
            vec![0],
            vec![0],
            vec!['a'],
            vec!["in0"],
            vec!["out0"],
        )
        .expect("in-bounds fixture with one name per port");
        nc.add_boundary_node_unknown_target('a', Left("in1"))
            .expect("fresh name, and the apex index is minted by the call");
        assert_eq!(nc.cospan().left_to_middle(), &[0, 1]);
        assert!(
            !nc.cospan().is_right_identity(),
            "right == [0] over a 2-vertex apex is not the identity"
        );

        // Only the ('in1', 'in0') order exercised the defect: it is the one
        // whose node 1 sits on the last apex vertex. In the other order node
        // 2's vertex is the last, `keep = mid_for_node_1` was already right,
        // and the pre-fix code returned the same answer as today's — so its
        // row must not be annotated with the other row's history.
        let history = if first == "in1" {
            "pre-fix this order gave left = [1, 0], right = [1] over a 1-vertex \
             apex — every entry out of bounds"
        } else {
            "this order's remap was already correct pre-fix; it is here so the \
             two orders run the same assertions"
        };

        nc.connect_pair(Left(first), Left(second));
        assert_eq!(nc.cospan().middle(), &['a'], "order ({first}, {second})");
        assert_eq!(
            (nc.cospan().left_to_middle(), nc.cospan().right_to_middle()),
            (&[0, 0][..], &[0][..]),
            "order ({first}, {second}): {history}"
        );
        assert!(
            nc.map_to_same(Left("in0"), Left("in1")),
            "order ({first}, {second}): the two named ports must share a vertex \
             after the merge ({history})"
        );
        assert!(!nc.cospan().is_left_identity());
        assert!(
            nc.cospan().is_right_identity(),
            "order ({first}, {second}): right == [0] over the merged 1-vertex \
             apex IS the identity again"
        );
    }
}

/// The stale `true` **was** a wrong composition, not a cosmetic lie: while
/// `perform_pushout` selected its fast path from the flag, it sized its
/// reindexing map from the partner's apex. The two shapes below are the two
/// ways that reached `compose_with_quotient` — one indexed out of range (a
/// panic), one silently dropped an apex vertex (an `Ok` with the wrong middle)
/// — and each carries its measured pre-fix value inline.
///
/// Both are pinned by **hand-written** expectations on the composite's three
/// vectors. An earlier draft also compared each composite against
/// `f.compose(&fresh(&g))`, and certified in a second assertion that the
/// comparison was vacuous — the mutated and rebuilt operands carried identical
/// flags, so no `perform_pushout` could tell them apart. With the cache gone
/// they are not merely flag-equal but the *same value*, so both the comparison
/// and its certificate are removed rather than explained.
///
/// **What this ranges over.** Two shapes, one operand pair each, one apex size.
/// It does not sweep apex sizes and does not touch the mirrored fast path.
#[test]
fn cospan_unknown_target_add_keeps_composition_correct() {
    // `g`: one domain port on apex vertex 'a', then a codomain port on a fresh
    // apex vertex 'b'. The domain leg is now [0] over a 2-vertex apex.
    let build_g = || {
        let mut g = Cospan::<char>::new(vec![0], vec![], vec!['a']).expect("in-bounds fixture");
        g.add_boundary_node_unknown_target(Right('b'));
        g
    };

    // Shape 1: `g`'s stale `is_left_id` was `perform_pushout`'s `right_leg_id`,
    // whose fast path sizes `right_to_pushout` from `f`'s one-entry codomain
    // leg; `compose_with_quotient` then indexes it with `g`'s codomain port —
    // `right_to_pushout[*target_in_other_middle]`. Measured with the partner
    // flag left stale, back when the flag chose the arm: `f.compose(&g)`
    // panicked there with `index out of bounds: the len is 1 but the index is
    // 1`. The arm is chosen from the leg now, and `g`'s domain leg `[0]` over a
    // 2-vertex apex fails `leg_is_identity`, so that arm is not entered at all.
    let g = build_g();
    let f = Cospan::<char>::new(vec![0], vec![0], vec!['a', 'x']).expect("in-bounds fixture");
    let composite = f.compose(&g).expect("f ; g composes");
    assert_eq!(composite.middle(), &['a', 'x', 'b']);
    assert_eq!(composite.left_to_middle(), &[0]);
    assert_eq!(composite.right_to_middle(), &[2]);

    // Shape 2: with the codomain port deleted first nothing indexes out of
    // range, and the stale `true` was *silent* — measured middle `['a', 'x']`
    // where the apex below has three vertices, i.e. one silently dropped.
    let mut g = build_g();
    g.delete_boundary_node(Right(0));
    assert_eq!(g.right_to_middle(), &[] as &[usize]);
    let composite = f.compose(&g).expect("f ; g composes");
    assert_eq!(composite.middle(), &['a', 'x', 'b']);
    assert_eq!(composite.left_to_middle(), &[0]);
    assert_eq!(
        composite.right_to_middle(),
        &[] as &[usize],
        "g has no codomain port left, so neither has the composite"
    );
}

/// `delete_boundary_node` shortens a leg without shrinking the apex, so
/// deleting the **last** port of an identity cospan ends the identity — and
/// the old `is_right_id &= z == right.len() - 1` kept the flag `true`.
///
/// This was the flag defect with teeth, back when `perform_pushout` took its
/// fast path on the flag: that path sizes its reindexing map from the
/// *partner's* apex. Measured then by reverting the clear to the pre-#289
/// `is_right_id &= z == self.right.len() - 1`, the `f.compose(&g)` below
/// **panicked** with `index out of bounds: the len is 1 but the index is 1` at
/// `compose_with_quotient`'s `left_to_pushout[*target_in_self_middle]` (the
/// `left_leg_id` fast path had sized that map from `g`'s one-entry domain leg).
///
/// ⚠ **That is history now.** There is no flag to leave stale, so this is a
/// plain composition pin: a cospan whose codomain leg is one short of its apex,
/// composed with the identity on that leg's image, keeps both its apex vertices.
///
/// **What this ranges over.** One delete-the-last-codomain-port on one
/// 2-vertex fixture, composed with one identity. It does not sweep apex sizes,
/// non-last ports, or the domain-side mirror (the lib test
/// `cospan_delete_boundary_node_states_its_invariant` carries that).
#[test]
fn cospan_delete_boundary_node_keeps_composition_correct() {
    let mut f = Cospan::<char>::identity(&vec!['a', 'b']);

    f.delete_boundary_node(Right(1));
    assert_eq!(f.right_to_middle(), &[0]);
    assert_eq!(f.middle(), &['a', 'b']);
    assert!(
        !f.is_right_identity(),
        "a 1-entry codomain leg over a 2-vertex apex is not the identity \
         (pre-#289 the cached flag said it was, and the composition below \
          panicked with `index out of bounds: the len is 1 but the index is 1`)"
    );

    let g = Cospan::<char>::identity(&vec!['a']);
    let composite = f.compose(&g).expect("f ; id_a composes");
    assert_eq!(composite.domain(), vec!['a', 'b']);
    assert_eq!(composite.codomain(), vec!['a']);
    assert_eq!(composite.middle(), &['a', 'b']);
}

/// `connect_pair` merges two apex vertices, so a leg that was the identity on
/// the old apex is not one on the new — and before #289 the flags said
/// otherwise, with teeth back when `perform_pushout` fast-pathed on them.
///
/// Measured on the branch before the fix (review R2-01): `f` below kept
/// `(true, true)` after the merge where a fresh construction says
/// `(false, false)`, and `f.compose(&identity(&['a', 'a']))` returned
/// `left = [0, 0], right = [0, 1], middle = ['a', 'a']` — the `left_leg_id`
/// fast path took `g`'s apex unmerged — against the reference's
/// `left = [0, 0], right = [0, 0], middle = ['a']`. No panic, the types line
/// up, `structurally_equal` is false.
///
/// ⚠ That composite could only go wrong because the flag chose the arm, so
/// what remains here is a claim about `connect_pair`'s remap and about the
/// composite, both against **hand-written** expectations. An earlier draft also
/// compared the composite against `fresh(&f).compose(&g)`; with no flags left,
/// `fresh(&f)` and `f` are the same value, so that comparison was a value
/// against itself and is gone.
///
/// **What this ranges over.** One merge of two domain ports on a 2-vertex
/// apex, in the argument order whose remap was already correct (node 2's
/// vertex is the last index — the reversed order is the sibling test below),
/// composed with one identity. It does not sweep apex sizes, codomain-side
/// merges, or the same-vertex / label-mismatch no-op arms.
#[test]
fn cospan_connect_pair_merges_so_compose_is_right() {
    let mut f =
        Cospan::<char>::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).expect("in-bounds fixture");
    assert!(
        f.is_left_identity() && f.is_right_identity(),
        "fixture must start as an identity on both legs"
    );

    f.connect_pair(Left(0), Left(1));
    assert_eq!(f.middle(), &['a']);
    assert_eq!(f.left_to_middle(), &[0, 0]);
    assert_eq!(f.right_to_middle(), &[0, 0]);
    assert!(f.map_to_same(Left(0), Left(1)));
    assert!(
        !f.is_left_identity() && !f.is_right_identity(),
        "a 2-entry leg over a 1-vertex apex is not the identity (pre-fix both \
         cached flags stayed true)"
    );

    let g = Cospan::<char>::identity(&vec!['a', 'a']);
    let composite = f.compose(&g).expect("f ; id composes");
    assert_eq!(
        composite.middle(),
        &['a'],
        "pre-fix: right = [0, 1] over ['a', 'a'], i.e. the merge undone by the \
         composite"
    );
    assert_eq!(composite.left_to_middle(), &[0, 0]);
    assert_eq!(composite.right_to_middle(), &[0, 0]);
}

/// `connect_pair`'s remap wrote node 1's **old** apex index after `swap_remove`
/// had moved that vertex into node 2's slot, so whenever node 1's vertex was
/// the last apex index (and node 2's was not) both legs received an
/// out-of-bounds entry and the two ports were never merged at all — a
/// pre-existing defect surfaced by #289's third review (R3-01), invisible to
/// every in-tree caller because each passes the lower apex index first or
/// merges away the last vertex. Reachable unchanged through
/// `NamedCospan::connect_pair` and `WiringDiagram::connect_pair`, where
/// "mint a port with `add_boundary_node_unconnected`, then connect it" hits
/// it whenever the new (last) port is passed first.
///
/// Measured before the fix, on the three shapes below: `left = [1, 0]`,
/// `right = [1, 0]` over a 1-vertex apex (entry 1 out of bounds);
/// `left = [2, 1, 0]` over a 2-vertex apex with `map_to_same(Left(0),
/// Left(2))` false; and the mixed pair likewise `[1, 0]` / `[1, 0]`. A debug
/// build was equally silent: `connect_pair` runs no `assert_valid`.
///
/// **What this ranges over.** Three argument shapes on three fixtures — the
/// reversed order of the sibling test's fixture, a 3-vertex apex merged from
/// its last vertex into its first, and a mixed `Left`/`Right` pair — each
/// compared against **hand-written** legs and `map_to_same`, because a rebuild
/// of the mutated value cannot see a remap error. Codomain-side merges of the
/// same shape and apex sizes above 3 are not swept; the one order the remap
/// always got right is the sibling test's.
#[test]
fn cospan_connect_pair_merges_when_node_1s_vertex_is_the_last_apex_index() {
    // Reversed order of the sibling test's fixture: node 1 sits on vertex 1,
    // the last, so `swap_remove(0)` moves it into slot 0.
    let mut f =
        Cospan::<char>::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).expect("in-bounds fixture");
    f.connect_pair(Left(1), Left(0));
    assert_eq!(f.middle(), &['a']);
    assert_eq!(
        (f.left_to_middle(), f.right_to_middle()),
        (&[0, 0][..], &[0, 0][..]),
        "pre-fix: left = [1, 0], right = [1, 0] over a 1-vertex apex — entry 1 out of bounds"
    );
    assert!(
        f.map_to_same(Left(0), Left(1)),
        "the two ports must share a vertex after the merge"
    );
    assert!(
        !f.is_left_identity() && !f.is_right_identity(),
        "both legs are length 2 over a 1-vertex apex, so neither is the \
         identity; the pre-#289 cache kept both stale-`true` from the \
         pre-merge value"
    );

    // A 3-vertex apex, merging the LAST vertex into the first: the moved
    // vertex (old index 2) lands in slot 0, and the untouched port keeps its own.
    let mut c =
        Cospan::<char>::new(vec![0, 1, 2], vec![], vec!['a', 'a', 'a']).expect("in-bounds fixture");
    c.connect_pair(Left(2), Left(0));
    assert_eq!(c.middle(), &['a', 'a']);
    assert_eq!(
        c.left_to_middle(),
        &[0, 1, 0],
        "pre-fix: left = [2, 1, 0] over a 2-vertex apex — two entries out of bounds"
    );
    assert!(c.map_to_same(Left(0), Left(2)));
    assert!(
        !c.map_to_same(Left(0), Left(1)),
        "the untouched port keeps its own vertex"
    );
    assert!(
        !c.is_left_identity() && !c.is_right_identity(),
        "left is length 3 over a 2-vertex apex and right is empty over a \
         non-empty one, so neither is the identity; the pre-#289 cache kept \
         is_left_id stale-`true` from the pre-merge [0, 1, 2] over 3"
    );

    // Mixed legs: a domain port on the last vertex merged with a codomain port
    // on the first.
    let mut m =
        Cospan::<char>::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).expect("in-bounds fixture");
    m.connect_pair(Left(1), Right(0));
    assert_eq!(m.middle(), &['a']);
    assert_eq!(
        (m.left_to_middle(), m.right_to_middle()),
        (&[0, 0][..], &[0, 0][..]),
        "pre-fix: left = [1, 0], right = [1, 0] over a 1-vertex apex"
    );
    assert!(m.map_to_same(Left(1), Right(0)));
    assert!(
        !m.is_left_identity() && !m.is_right_identity(),
        "both legs are length 2 over a 1-vertex apex after the mixed-leg \
         merge; the pre-#289 cache kept both stale-`true`"
    );
}

/// `Span`'s identity flags are weaker than `Cospan`'s answer, and the `Span`
/// docs now say so — pinned here so the docstring cannot drift from the code.
///
/// `Span::new_unchecked` computes them as `represents_id` over the middle-pair
/// components with no conjunct against the boundary lengths, so appending a
/// boundary label leaves `is_left_identity()` reporting `true` for a span whose
/// middle no longer covers its domain. Nothing mis-composes on it —
/// `Span::compose` does not fast-path on the flags — so this pins the current
/// contract, not a defect being fixed here. Tracked as
/// [#345](https://github.com/sustia-llc/catgraph/issues/345): when that issue
/// tightens the flag this pin must go **red** and be inverted — that is what it
/// is for; do not "fix" it by deleting it.
///
/// `Span` still *caches* both flags; #289 deleted `Cospan`'s cache but left
/// `Span`'s alone, so the two types now differ in two ways rather than one —
/// the missing conjunct #345 names, and whether the answer is recomputed. That
/// widens #345's scope; it does not settle it.
///
/// **What this ranges over.** One domain-side append on one fixture, with one
/// `Cospan` contrast beside it so the two claims cannot be read as one.
///
/// ⚠ **The contrast is `Cospan`'s mirror, not its twin, and it has to be.** The
/// two halves grow in opposite directions: the `Span` gains a *boundary* entry
/// its middle pairs do not reach (boundary **longer** than the middle), while
/// the `Cospan` gains an *apex* vertex its domain leg does not reach (leg
/// **shorter** than the apex). The literal twin — a `Cospan` leg longer than
/// its apex, e.g. `identity(&['a', 'b'])` then
/// `add_boundary_node_known_target(Left(0))`, giving `left == [0, 1, 0]` over
/// two vertices — cannot demonstrate the conjunct at all: a leg longer than the
/// apex has all its entries below `apex_len`, so by pigeonhole it repeats one,
/// so `represents_id` fails first and the answer is `false` with or without
/// `leg.len() == middle.len()`. Only a leg that is short **and in order**
/// leaves the length conjunct deciding. An editor "aligning" the two halves
/// into the same shape would therefore turn the contrast into a vacuous
/// assertion; keep them opposite.
#[test]
fn span_identity_flag_ignores_the_boundary_length() {
    let mut s = Span::<char>::identity(&vec!['a', 'b']);
    assert!(s.is_left_identity());
    s.add_boundary_node(Left('c'));
    assert_eq!(s.left().len(), 3);
    assert_eq!(s.middle_pairs().len(), 2);
    assert!(
        s.is_left_identity(),
        "unlike `Cospan`, `Span` reads only the middle pairs — see the \
         `Span::add_boundary_node` docs"
    );

    // The `Cospan` shape #345 would bring `Span` into line with: a domain leg
    // that no longer covers the apex is not an identity.
    let mut c = Cospan::<char>::identity(&vec!['a', 'b']);
    c.add_boundary_node_unknown_target(Right('c'));
    assert_eq!(c.left_to_middle().len(), 2);
    assert_eq!(c.middle().len(), 3);
    assert!(
        !c.is_left_identity(),
        "`Cospan` carries the boundary-length conjunct `Span` lacks"
    );
}

// ===========================================================================
// 3. The panicking preconditions name their invariant
// ===========================================================================

#[test]
#[should_panic(
    expected = "delete_boundary_node: domain index 0 is out of bounds; the domain has 0 port(s)"
)]
fn cospan_delete_boundary_node_on_an_empty_domain_names_the_invariant() {
    // Pre-#289 this computed `self.left.len() - 1` first: a debug overflow
    // panic, and in release a wrap to usize::MAX followed by a bare
    // `swap_remove` panic. Neither said which precondition was violated.
    Cospan::<char>::empty().delete_boundary_node(Left(0));
}

#[test]
#[should_panic(
    expected = "delete_boundary_node: codomain index 3 is out of bounds; the codomain has 1 port(s)"
)]
fn cospan_delete_boundary_node_past_the_codomain_names_the_invariant() {
    let mut c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).expect("in-bounds fixture");
    c.delete_boundary_node(Right(3));
}

#[test]
#[should_panic(expected = "map_to_same: domain index 5 is out of bounds; the domain has 1 port(s)")]
fn cospan_map_to_same_names_the_invariant() {
    let c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).expect("in-bounds fixture");
    let _ = c.map_to_same(Left(5), Right(0));
}

#[test]
#[should_panic(
    expected = "connect_pair: codomain index 2 is out of bounds; the codomain has 1 port(s)"
)]
fn cospan_connect_pair_names_the_invariant() {
    let mut c = Cospan::<char>::new(vec![0], vec![0], vec!['a']).expect("in-bounds fixture");
    c.connect_pair(Left(0), Right(2));
}

#[test]
#[should_panic(
    expected = "delete_boundary_node: domain index 0 is out of bounds; the domain has 0 port(s)"
)]
fn named_cospan_delete_boundary_node_on_an_empty_domain_names_the_invariant() {
    // The name list is `swap_remove`d before the cospan is touched, so without
    // a check here the underlying cospan's message is never reached and the
    // name list is left one shorter than the leg.
    NamedCospan::<char, &str, &str>::empty().delete_boundary_node(Left(0));
}

// ===========================================================================
// 4. NamedCospan: one contract for both invariants
// ===========================================================================

/// Both of `NamedCospan::add_boundary_node`'s invariants are now reported the
/// same way, the name is checked first, and neither list is written on `Err`.
///
/// **What this ranges over.** Both legs for the duplicate-name arm, one leg
/// for the index arm (the index check is `Cospan`'s, swept by the first test
/// in this file), and the one ordering case where both are violated at once.
///
/// Pre-#289: the duplicate name hit a bare `assert!(...)` with no message and
/// aborted the process in **every** profile, while the out-of-bounds index was
/// not checked at all — one method, two postures.
#[test]
fn named_cospan_add_boundary_node_reports_both_invariants_name_first() {
    let fixture = || {
        NamedCospan::<char, &str, &str>::new(vec![0], vec![0], vec!['a'], vec!["in0"], vec!["out0"])
            .expect("in-bounds fixture with one name per port")
    };

    // Duplicate domain name.
    let mut nc = fixture();
    let err = nc
        .add_boundary_node_known_target(0, Left("in0"))
        .expect_err("\"in0\" already names domain port 0");
    assert_eq!(
        err,
        CatgraphError::ConstructionDuplicatePortName {
            leg: BoundaryLeg::Domain,
            existing_position: 0,
        }
    );
    assert_eq!(
        err.to_string(),
        "construction error: a domain port at position 0 already carries the requested name; \
         port names must be unique on each boundary"
    );
    assert_eq!(nc.left_names().len(), 1, "no name is pushed on Err");
    assert_eq!(
        nc.cospan().left_to_middle().len(),
        1,
        "and no leg entry either — the two lists cannot go out of step"
    );

    // Duplicate codomain name, through the `_unknown_target` wrapper, which
    // would otherwise have grown the apex.
    let mut nc = fixture();
    let err = nc
        .add_boundary_node_unknown_target('z', Right("out0"))
        .expect_err("\"out0\" already names codomain port 0");
    assert_eq!(
        err,
        CatgraphError::ConstructionDuplicatePortName {
            leg: BoundaryLeg::Codomain,
            existing_position: 0,
        }
    );
    assert_eq!(nc.right_names().len(), 1);
    assert_eq!(
        nc.cospan().middle().len(),
        1,
        "the apex must not grow on Err"
    );

    // Fresh name, out-of-bounds index: the cospan's variant, and still no
    // half-mutation.
    let mut nc = fixture();
    let err = nc
        .add_boundary_node_known_target(9, Left("in1"))
        .expect_err("apex index 9 is out of bounds for a 1-vertex apex");
    assert_eq!(
        err,
        CatgraphError::ConstructionIndexOutOfBounds {
            leg: BoundaryLeg::Domain,
            position: 1,
            target: 9,
            target_len: 1,
        }
    );
    assert_eq!(
        nc.left_names().len(),
        1,
        "the name must not be pushed when the index is refused"
    );

    // Both violated: the name is reported, as documented.
    let mut nc = fixture();
    let err = nc
        .add_boundary_node_known_target(9, Left("in0"))
        .expect_err("both invariants are violated");
    assert_eq!(
        err,
        CatgraphError::ConstructionDuplicatePortName {
            leg: BoundaryLeg::Domain,
            existing_position: 0,
        },
        "the name is checked before the index"
    );

    // Control: a fresh name and an in-bounds index still work.
    let mut nc = fixture();
    let where_ = nc
        .add_boundary_node_known_target(0, Left("in1"))
        .expect("fresh name, in-bounds index");
    assert_eq!(where_, Left(1));
    assert_eq!(nc.left_names().len(), 2);
    assert_eq!(nc.cospan().left_to_middle(), &[0, 0]);
}

// ===========================================================================
// 5. Span::add_middle
// ===========================================================================

/// `Span::add_middle` bounds-checks the pair before reading the labels, and
/// reports it with the variant `Span::new` already raises for the identical
/// input shape.
///
/// **What this ranges over.** Both halves of the pair, with `usize::MAX` and a
/// small out-of-range index, plus the label-mismatch arm as a control that the
/// pre-existing error was not displaced. It does not sweep apex sizes.
///
/// Measured on the pre-#289 body, `add_middle((usize::MAX, 0))` panicked with
/// `index out of bounds: the len is 1 but the index is 18446744073709551615`
/// — a bare slice message, in every profile, from a method that already
/// returns `Result`.
#[test]
fn span_add_middle_rejects_out_of_bounds_pairs_on_both_halves() {
    let fixture = || {
        Span::<char>::new(vec!['a'], vec!['a'], vec![]).expect("empty middle is trivially valid")
    };

    for (leg, pair, target, target_len) in [
        (BoundaryLeg::Domain, (usize::MAX, 0), usize::MAX, 1),
        (BoundaryLeg::Domain, (1, 0), 1, 1),
        (BoundaryLeg::Codomain, (0, usize::MAX), usize::MAX, 1),
        (BoundaryLeg::Codomain, (0, 4), 4, 1),
    ] {
        let mut s = fixture();
        let err = s
            .add_middle(pair)
            .expect_err("an out-of-range pair half must be refused, not panic");
        assert_eq!(
            err,
            CatgraphError::ConstructionMiddlePairOutOfBounds {
                leg,
                pair_position: 0,
                target,
                target_len,
            },
            "pair {pair:?} reported the wrong location"
        );
        assert!(
            s.middle_pairs().is_empty(),
            "the refused pair must not be pushed"
        );
    }

    // Both halves out of range: the domain half is reported, as documented.
    let mut s = fixture();
    let err = s
        .add_middle((3, 3))
        .expect_err("both halves are out of range");
    assert_eq!(
        err,
        CatgraphError::ConstructionMiddlePairOutOfBounds {
            leg: BoundaryLeg::Domain,
            pair_position: 0,
            target: 3,
            target_len: 1,
        }
    );

    // Control: the label-agreement arm still reports `Composition`, and the
    // valid arm still returns the new middle index.
    let mut s = Span::<char>::new(vec!['a', 'b'], vec!['a', 'b'], vec![])
        .expect("empty middle is trivially valid");
    assert!(matches!(
        s.add_middle((0, 1)),
        Err(CatgraphError::Composition { .. })
    ));
    assert_eq!(s.add_middle((0, 0)).expect("labels agree"), 0);
    assert_eq!(
        s.add_middle((1, 1)).expect("labels agree"),
        1,
        "pair_position tracks the growing list"
    );
}

/// The reported `pair_position` is the position the pair *would* have taken,
/// so it keeps naming the offending element as the list grows.
#[test]
fn span_add_middle_reports_the_position_the_pair_would_have_taken() {
    let mut s = Span::<char>::new(vec!['a'], vec!['a'], vec![]).expect("valid fixture");
    s.add_middle((0, 0)).expect("labels agree");
    s.add_middle((0, 0)).expect("labels agree");
    let err = s
        .add_middle((0, 6))
        .expect_err("codomain index 6 is out of range");
    assert_eq!(
        err,
        CatgraphError::ConstructionMiddlePairOutOfBounds {
            leg: BoundaryLeg::Codomain,
            pair_position: 2,
            target: 6,
            target_len: 1,
        }
    );
}

// ===========================================================================
// 6. finset::from_cycle
// ===========================================================================

/// A cycle of distinct in-range elements still means exactly what it did, and
/// the two malformed cases now say so instead of failing obscurely.
///
/// **What this ranges over.** The control compares `from_cycle` against an
/// independently written cycle oracle over **every** cycle of every length
/// `0..=n` on `n = 5` whose elements are distinct and in range (a permutation
/// of each subset, generated here rather than taken from the implementation),
/// so it is not a single-input pin. The panic arms are two inputs each.
#[test]
fn from_cycle_agrees_with_an_independent_oracle_on_every_valid_cycle() {
    const N: usize = 5;

    // Independent oracle: a cycle [c_0, .., c_{k-1}] sends c_i to c_{i+1 mod k}
    // and fixes everything else. Written from the docstring, not from the
    // implementation (which builds a product of transpositions).
    fn oracle(n: usize, cycle: &[usize]) -> Vec<usize> {
        let mut image: Vec<usize> = (0..n).collect();
        if cycle.len() >= 2 {
            for (i, &from) in cycle.iter().enumerate() {
                image[from] = cycle[(i + 1) % cycle.len()];
            }
        }
        image
    }

    // Every distinct-element cycle over 0..N, all lengths, all orders.
    fn each_cycle(n: usize, prefix: &mut Vec<usize>, f: &mut impl FnMut(&[usize])) {
        f(prefix);
        for candidate in 0..n {
            if !prefix.contains(&candidate) {
                prefix.push(candidate);
                each_cycle(n, prefix, f);
                prefix.pop();
            }
        }
    }

    let mut checked = 0_usize;
    each_cycle(N, &mut Vec::new(), &mut |cycle| {
        let p = from_cycle(N, cycle);
        let got: Vec<usize> = (0..N).map(|i| p.apply(i)).collect();
        assert_eq!(
            got,
            oracle(N, cycle),
            "from_cycle({N}, {cycle:?}) disagrees with the cycle-notation oracle"
        );
        checked += 1;
    });
    // sum_{k=0..5} 5!/(5-k)! = 1 + 5 + 20 + 60 + 120 + 120 = 326.
    assert_eq!(
        checked, 326,
        "the enumeration itself must not silently shrink"
    );
}

#[test]
#[should_panic(expected = "from_cycle: every cycle element must be less than n = 3")]
fn from_cycle_rejects_an_out_of_range_element() {
    // Pre-#289 this reached `assert!(i < n && j < n)` inside `permutations`,
    // from a recursive call, naming neither this function nor the element.
    let _ = from_cycle(3, &[0, 7]);
}

#[test]
#[should_panic(expected = "from_cycle: every cycle element must be less than n = 3")]
fn from_cycle_rejects_an_out_of_range_singleton() {
    // Pre-#289 a cycle shorter than 2 short-circuited to the identity before
    // any check, so this returned a permutation instead of reporting the
    // nonsense.
    let _ = from_cycle(3, &[7]);
}

#[test]
#[should_panic(expected = "from_cycle: cycle elements must be pairwise distinct")]
fn from_cycle_rejects_a_repeated_element() {
    // Pre-#289 this returned, silently, the identity on 3 elements — not the
    // documented "sends a to b, b to c, c to a", and no 3-cycle exists on the
    // two distinct elements named.
    let _ = from_cycle(3, &[0, 1, 0]);
}

// ===========================================================================
// 7. utils::remove_multiple
// ===========================================================================

/// Repeated indices name one element and remove it once, and every index is
/// read against the vector as it was on entry.
///
/// Measured on the pre-#289 body: `[0, 1, 2, 3, 4]` with `[3, 3]` came back as
/// `[0, 1, 2]` — index 3 was removed, then index 3 *of the shortened vector*,
/// silently deleting the element that had been at 4. Post-fix: `[0, 1, 2, 4]`.
/// With the repeat at the tail (`[0, 1, 2, 3]` with `[3, 3]`) the second
/// removal panicked instead.
///
/// **What this ranges over.** Repeats at the tail and in the middle, a
/// three-fold repeat, an unsorted list, and the empty list; it does not sweep
/// element types (the function is generic and never inspects `T`).
#[test]
fn remove_multiple_dedups_and_reads_every_index_against_the_original() {
    let mut middle_repeat = vec![0, 1, 2, 3, 4];
    remove_multiple(&mut middle_repeat, vec![3, 3]);
    assert_eq!(
        middle_repeat,
        vec![0, 1, 2, 4],
        "a repeated index removes one element (pre-#289: [0, 1, 2])"
    );

    let mut tail_repeat = vec![0, 1, 2, 3];
    remove_multiple(&mut tail_repeat, vec![3, 3]);
    assert_eq!(
        tail_repeat,
        vec![0, 1, 2],
        "a repeat at the tail is a no-op, not a panic (pre-#289: panicked)"
    );

    let mut thrice = vec![10, 11, 12, 13];
    remove_multiple(&mut thrice, vec![1, 1, 1]);
    assert_eq!(thrice, vec![10, 12, 13]);

    let mut unsorted = vec!['a', 'b', 'c', 'd', 'e'];
    remove_multiple(&mut unsorted, vec![3, 0, 3, 1]);
    assert_eq!(
        unsorted,
        vec!['c', 'e'],
        "indices are read against the entry vector, in any order"
    );

    let mut untouched = vec![1, 2, 3];
    remove_multiple(&mut untouched, vec![]);
    assert_eq!(untouched, vec![1, 2, 3]);
}

#[test]
#[should_panic(expected = "remove_multiple: index 5 is out of bounds for a vector of 2 element(s)")]
fn remove_multiple_names_the_bounds_invariant() {
    let mut v = vec!['a', 'b'];
    remove_multiple(&mut v, vec![0, 5]);
}
