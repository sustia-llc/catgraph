//! #289 regression pins for the checked boundary-node mutators.
//!
//! Three separable claims, pinned separately:
//!
//! 1. The `add_boundary_node` family **rejects** an out-of-bounds apex index in
//!    every build profile and leaves the value untouched when it does.
//! 2. `Cospan`'s cached `is_left_id` / `is_right_id` are no longer left a stale
//!    `true` by `add_boundary_node` (either arm, on either leg) or
//!    `delete_boundary_node`, where they previously were. The predicate they
//!    claim to cache is `Cospan::new_unchecked`'s, `leg.len() == middle.len()
//!    && represents_id(leg)`; `perform_pushout` fast-paths on the flags, so a
//!    stale `true` is a *wrong composition*, not a cosmetic lie. The `false`
//!    direction stays conservative by design, and is pinned as such below.
//! 3. The remaining panicking preconditions name the invariant they enforce,
//!    in every build profile.
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
    named_cospan::NamedCospan,
    span::Span,
    utils::remove_multiple,
};
use either::Either::{Left, Right};

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

            // No half-mutation: neither leg grew, the apex did not grow, and
            // the identity flags are the ones the fixture was built with.
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
            assert!(c.is_left_identity() && c.is_right_identity());
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
// 2. The identity flags
// ===========================================================================

/// The boundary case the pre-#289 `&=` let through with the flag still `true`:
/// `tgt_idx == leg.len()`.
///
/// On an identity cospan `leg.len() == middle.len()`, so the one index that
/// satisfies the old test `leg.len() - 1 == tgt_idx` *after* the push is
/// exactly `middle.len()` — out of bounds. The old code therefore pushed an
/// out-of-range entry **and kept `is_right_id == true`**, which is the silent
/// half of #289: `perform_pushout` fast-paths on that flag.
///
/// Measured on the pre-#289 body: the call returned `Ok(Right(2))`,
/// `right_to_middle()` became `[0, 1, 2]` (entry 2 out of range for a 2-vertex
/// apex) and `is_right_identity()` stayed `true`. Post-fix: `Err`, the leg
/// stays `[0, 1]`, and the flag stays `true` because nothing happened.
///
/// **What this ranges over.** One index — the boundary one — on the codomain
/// leg of one fixture. It is deliberately narrow: the general bounds sweep is
/// the test above, and the apex-growing arm's flag defect is the test below.
/// What is unique here is that this index is the *only* out-of-bounds value the
/// old arithmetic did not notice.
#[test]
fn cospan_identity_flag_is_not_corrupted_at_the_boundary_index() {
    let mut c = Cospan::<char>::identity(&vec!['a', 'b']);
    assert_eq!(c.right_to_middle(), &[0, 1]);
    assert_eq!(c.middle().len(), 2);
    assert!(c.is_right_identity());

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
    assert!(
        c.is_right_identity(),
        "the flag is still true because the cospan is still the identity it was"
    );
}

/// The reference for a cached flag: what `Cospan::new` computes for the same
/// three vectors, which is the definition the flag claims to summarize.
///
/// Only usable where the mutator under test is expected to be **exact**. The
/// flags are deliberately conservative in the `false` direction — `&=` can only
/// clear, so `delete` then `add` can leave a `false` where a fresh construction
/// says `true` (see the CHANGELOG's `Fixed` entry) — so this is not a general
/// invariant of the type and must not be asserted after a `delete`.
fn assert_flags_agree_with_a_fresh_construction(c: &Cospan<char>, context: &str) {
    let fresh = Cospan::new(
        c.left_to_middle().to_vec(),
        c.right_to_middle().to_vec(),
        c.middle().to_vec(),
    )
    .expect("the mutated cospan's legs are in bounds");
    assert_eq!(
        (c.is_left_identity(), c.is_right_identity()),
        (fresh.is_left_identity(), fresh.is_right_identity()),
        "{context}: the cached (left, right) identity flags disagree with a \
         fresh construction from left = {:?}, right = {:?}, middle = {:?}",
        c.left_to_middle(),
        c.right_to_middle(),
        c.middle(),
    );
}

/// The `Right(label)` arm grows the **apex**, so the leg it does not touch
/// stops being the identity — and the flag has to say so.
///
/// This is the half of #289 the first pass missed: that arm updated only the
/// flag of the leg it pushed to, so `add_boundary_node_unknown_target(Right(_))`
/// left a legitimately-`true` `is_left_id` in place with the domain leg now
/// strictly shorter than the apex (and the mirror for `Left(_)` /
/// `is_right_id`). It is reachable through the fully checked API — no
/// `_unchecked` call, no malformed input — and `perform_pushout` fast-paths on
/// exactly that flag, so it is a wrong composition (pinned by the test below).
///
/// Measured before the fix: `Cospan::new(vec![0], vec![], vec!['a'])` followed
/// by `add_boundary_node_unknown_target(Right('b'))` reported
/// `is_left_identity() == true` while a fresh `Cospan::new` of the resulting
/// `([0], [1], ['a', 'b'])` reported `false`.
///
/// **What this ranges over.** Both sides of the outer `Either` — the two arms
/// in the source, which are separate expressions — and for each, *both* flags:
/// the pushed leg's, which must stay `true`, and the partner's, which must
/// clear. Each is checked against a fresh `Cospan::new` of the same three
/// vectors, an independent computation of the predicate rather than a
/// transcription of the mutator's arithmetic. It uses one apex size, and does
/// not range over the `Left(idx)` (known-target) arm, which does not grow the
/// apex and is covered above.
#[test]
fn cospan_unknown_target_add_clears_the_partner_legs_identity_flag() {
    // Domain-side push: leg and apex grow together, so `is_left_id` survives;
    // the codomain leg falls behind the apex, so `is_right_id` must clear.
    let mut c = Cospan::<char>::identity(&vec!['a']);
    assert!(c.is_left_identity() && c.is_right_identity());
    c.add_boundary_node_unknown_target(Left('b'));
    assert_eq!(
        (c.left_to_middle(), c.right_to_middle()),
        (&[0, 1][..], &[0][..])
    );
    assert_flags_agree_with_a_fresh_construction(&c, "after unknown_target(Left)");
    assert!(
        c.is_left_identity(),
        "left == [0, 1] on a 2-vertex apex is still the identity"
    );
    assert!(
        !c.is_right_identity(),
        "right == [0] on a 2-vertex apex misses apex vertex 1 (this read `true` \
         before the fix — the `Left(label)` arm never touched `is_right_id`)"
    );

    // Codomain-side push: the mirror, a separate expression in the source.
    let mut c = Cospan::<char>::identity(&vec!['a']);
    c.add_boundary_node_unknown_target(Right('b'));
    assert_eq!(
        (c.left_to_middle(), c.right_to_middle()),
        (&[0][..], &[0, 1][..])
    );
    assert_flags_agree_with_a_fresh_construction(&c, "after unknown_target(Right)");
    assert!(
        c.is_right_identity(),
        "right == [0, 1] on a 2-vertex apex is still the identity"
    );
    assert!(
        !c.is_left_identity(),
        "left == [0] on a 2-vertex apex misses apex vertex 1 (this read `true` \
         before the fix)"
    );
}

/// `NamedCospan` inherits the same arm, so it inherited the same stale `true`.
///
/// **What this ranges over.** One side (the codomain push, clearing the domain
/// flag) on the named wrapper. The wrapper adds a name list and delegates the
/// leg/apex work to `Cospan`, so the arm-by-arm sweep is the test above; what
/// is unique here is that the delegation reaches the *fixed* code.
#[test]
fn named_cospan_unknown_target_add_clears_the_partner_legs_identity_flag() {
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
    assert_eq!(nc.cospan().middle().len(), 2);
    assert!(
        !nc.cospan().is_left_identity(),
        "left == [0] on a 2-vertex apex misses apex vertex 1 (measured `true` \
         on the named surface before the fix)"
    );
}

/// The stale `true` is a **wrong composition**, not a cosmetic lie:
/// `perform_pushout` fast-paths on the flag and sizes its reindexing map from
/// the partner's apex.
///
/// **What this ranges over.** Both shapes the stale `is_left_id` reached in
/// `compose_with_quotient` — one that indexed out of range (a panic) and one
/// that silently dropped an apex vertex (an `Ok` with the wrong middle). Each
/// composite is compared against the reference composition, i.e. the same
/// operand rebuilt with `Cospan::new` so that its flags are computed from
/// scratch, rather than against a transcribed expectation. One pair of
/// operands per shape; it does not sweep apex sizes or the mirrored
/// (`is_right_id`) fast path.
#[test]
fn cospan_unknown_target_add_keeps_composition_correct() {
    // `g`: one domain port on apex vertex 'a', then a codomain port on a fresh
    // apex vertex 'b'. The domain leg is now [0] over a 2-vertex apex.
    let build_g = || {
        let mut g = Cospan::<char>::new(vec![0], vec![], vec!['a']).expect("in-bounds fixture");
        g.add_boundary_node_unknown_target(Right('b'));
        g
    };
    // The reference operand: the same three vectors, flags computed from scratch.
    let refresh = |g: &Cospan<char>| {
        Cospan::new(
            g.left_to_middle().to_vec(),
            g.right_to_middle().to_vec(),
            g.middle().to_vec(),
        )
        .expect("the mutated legs are in bounds")
    };

    // Shape 1: the fast path indexes `left_to_pushout` past its end.
    // Measured with the partner flag left stale: `f.compose(&g)` panicked at
    // `catgraph/src/cospan.rs:653` with
    // `index out of bounds: the len is 1 but the index is 1`.
    let g = build_g();
    let f = Cospan::<char>::new(vec![0], vec![0], vec!['a', 'x']).expect("in-bounds fixture");
    let composite = f.compose(&g).expect("f ; g composes");
    let reference = f.compose(&refresh(&g)).expect("f ; g composes");
    assert!(
        composite.structurally_equal(&reference),
        "the composite must not depend on whether `g`'s flags were cached or \
         recomputed: got left = {:?}, right = {:?}, middle = {:?}",
        composite.left_to_middle(),
        composite.right_to_middle(),
        composite.middle(),
    );
    assert_eq!(composite.middle(), &['a', 'x', 'b']);
    assert_eq!(composite.left_to_middle(), &[0]);
    assert_eq!(composite.right_to_middle(), &[2]);

    // Shape 2: with the codomain port deleted first nothing indexes out of
    // range, and the stale `true` was *silent* — measured middle `['a', 'x']`
    // against the reference's `['a', 'x', 'b']`, i.e. an apex vertex dropped.
    let mut g = build_g();
    g.delete_boundary_node(Right(0));
    assert_eq!(g.right_to_middle(), &[] as &[usize]);
    let composite = f.compose(&g).expect("f ; g composes");
    let reference = f.compose(&refresh(&g)).expect("f ; g composes");
    assert!(
        composite.structurally_equal(&reference),
        "the composite silently dropped an apex vertex: middle = {:?} against \
         the reference's {:?}",
        composite.middle(),
        reference.middle(),
    );
    assert_eq!(composite.middle(), &['a', 'x', 'b']);
}

/// `delete_boundary_node` shortens a leg without shrinking the apex, so
/// deleting the **last** port of an identity cospan ends the identity — and
/// the old `is_right_id &= z == right.len() - 1` kept the flag `true`.
///
/// This is the flag defect with teeth: `perform_pushout` takes a fast path
/// when a leg is flagged as the identity, and that path sizes its reindexing
/// map from the *partner's* apex. Measured by reverting only the
/// `&& self.right.len() == self.middle.len()` conjunct: the `f.compose(&g)`
/// below **panicked** at `catgraph/src/cospan.rs:648` with `index out of
/// bounds: the len is 1 but the index is 1` — `compose_with_quotient`'s
/// `left_to_pushout[*target_in_self_middle]`. Post-fix it returns the correct
/// composite `{a,b} -> {a}`.
#[test]
fn cospan_delete_boundary_node_clears_the_identity_flag_so_compose_is_right() {
    let mut f = Cospan::<char>::identity(&vec!['a', 'b']);
    assert!(f.is_right_identity());

    f.delete_boundary_node(Right(1));
    assert_eq!(f.right_to_middle(), &[0]);
    assert_eq!(f.middle().len(), 2);
    assert!(
        !f.is_right_identity(),
        "a 1-entry codomain leg over a 2-vertex apex is not the identity \
         (pre-#289 this was true, and the composition below panicked with \
          `index out of bounds: the len is 1 but the index is 1`)"
    );

    let g = Cospan::<char>::identity(&vec!['a']);
    let composite = f.compose(&g).expect("f ; id_a composes");
    assert_eq!(composite.domain(), vec!['a', 'b']);
    assert_eq!(composite.codomain(), vec!['a']);
    assert_eq!(composite.middle(), &['a', 'b']);
}

/// The other direction is **not** fixed, deliberately, and the CHANGELOG says
/// so: `&=` can only clear, so a flag that a delete correctly cleared is never
/// restored by a later add that makes the leg an identity again.
///
/// Stale `false` costs `perform_pushout` its fast path and nothing else, which
/// is why it is left alone — but it is the reason
/// `assert_flags_agree_with_a_fresh_construction` is not a general invariant of
/// the type, and the reason `Cospan` does not derive `PartialEq`. Pinned so
/// that "conservative in one direction" stays a measured claim rather than a
/// remembered one.
///
/// **What this ranges over.** One delete-then-add round trip on the domain leg.
#[test]
fn cospan_identity_flags_are_conservative_in_the_false_direction() {
    let mut c = Cospan::<char>::identity(&vec!['a', 'b']);
    c.delete_boundary_node(Left(1));
    assert!(
        !c.is_left_identity(),
        "left == [0] on a 2-vertex apex is not the identity"
    );

    c.add_boundary_node_known_target(Left(1))
        .expect("apex index 1 is in bounds for a 2-vertex apex");
    assert_eq!(c.left_to_middle(), &[0, 1]);
    let fresh = Cospan::new(
        c.left_to_middle().to_vec(),
        c.right_to_middle().to_vec(),
        c.middle().to_vec(),
    )
    .expect("the legs are in bounds");
    assert!(
        fresh.is_left_identity(),
        "a fresh construction from the same three vectors says identity"
    );
    assert!(
        !c.is_left_identity(),
        "the cached flag stays false — conservative, not wrong: a stale false \
         only costs `perform_pushout` its fast path"
    );
}

/// `Span`'s identity flags are weaker than `Cospan`'s, and the `Span`
/// docs now say so — pinned here so the docstring cannot drift from the code.
///
/// `Span::new_unchecked` computes them as `represents_id` over the middle-pair
/// components with no conjunct against the boundary lengths, so appending a
/// boundary label leaves `is_left_identity()` reporting `true` for a span whose
/// middle no longer covers its domain. Nothing mis-composes on it —
/// `Span::compose` does not fast-path on the flags — so this pins the current
/// contract, not a defect being fixed here.
///
/// **What this ranges over.** One domain-side append on one fixture.
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
