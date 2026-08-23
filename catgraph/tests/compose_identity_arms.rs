//! What `perform_pushout`'s two identity fast paths compute.
//!
//! `Cospan::compose` is a function of the two operands' `(left, right, middle)`
//! — which, since [#289](https://github.com/sustia-llc/catgraph/issues/289)
//! deleted the cached identity flags, is the whole of a `Cospan`. Two claims
//! about the fast paths, pinned separately because they can fail
//! independently:
//!
//! 1. **Which answer the `left_leg_id` arm gives.** It numbers the composite
//!    apex by the *right* operand's own indexing, where the union-find body
//!    numbers it by left-leg discovery order. That choice is strict left
//!    unitality — `id ; f == f` on the nose — and it is only visible on an `f`
//!    whose left leg does not first-visit its apex in increasing order. Three
//!    tests below: two unital (`n = 2`, `n = 3`), one not.
//! 2. **Which arm wins when both predicates hold.** The `left_leg_id` arm is
//!    tried first and tags the composite apex's representatives `Right(..)`,
//!    so the composite keeps the *right* operand's labels. `composable` has
//!    already forced the two apexes equal under `Lambda`'s `Eq`, so for every
//!    label type in this workspace there is nothing to see — but `Cospan` only
//!    requires `Eq`, never that `Eq` be identity, and under a coarser `Eq` the
//!    two arms give visibly different answers.
//!
//! **Provenance.** This file was written as `compose_flag_independence.rs`,
//! for a third claim — that the arms are entered on the legs and not on
//! `Cospan`'s cached `is_left_id` / `is_right_id` — and its four tests for it
//! manufactured an operand whose cached flag disagreed with its legs, then
//! composed both and compared. #289 removed the cache outright, so that
//! disagreement cannot be built and the claim has no content left to pin:
//! `is_left_identity()` *is* `leg_is_identity(left, middle.len())`. Three of
//! those four tests are kept here for their **absolute** expectations, which
//! were always hand-written and never depended on the cache; the fourth
//! duplicated the `n = 2` unitality pin and is gone. The file was renamed with
//! the cache, so that its name states what it pins rather than a property
//! nothing can now violate.
//!
//! **What this file ranges over**, taken together: five fixtures, apex sizes 2
//! and 3, `char` labels plus one deliberately-coarse `Eq`, and one partner per
//! fixture. It does not sweep apex sizes past 3, does not touch the
//! right-unitality mirror, and has no fixture whose partner's left leg merges
//! apex vertices. **All five** go red when the `left_leg_id` arm is disabled —
//! measured, and each docstring carries the value its fixture then returns.
//! That one perturbation does not falsify claim 2 on its own, though: with the
//! first arm disabled the second takes over, so the label-provenance test
//! cannot tell "arm 1 gone" from "arms swapped". Swapping them is its own
//! perturbation, and the one its docstring names.

mod common;
use common::assert_cospan_eq_msg;

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
};
use either::Either::Left;

/// Strict left unitality on a left leg that separates the two numberings.
///
/// `pushout_correctness.rs`'s `compose_with_left_identity_preserves_structure`
/// asserts this same law, but only for `f.left = [0, 1]` — a fixture on which
/// the `left_leg_id` fast path and the union-find body agree element for
/// element, so it cannot tell which one ran. `property_laws.rs` compares
/// composites up to connectivity equivalence, so it cannot see apex
/// renumbering at all. This fixture can: `f.left = [1, 0]` visits `f`'s apex in
/// reverse, so numbering the composite by left-leg discovery order gives
/// `['b', 'a']` where numbering it by `f`'s own apex gives `['a', 'b']`.
///
/// **What this ranges over.** Exactly one fixture, `n = 2`, `char` labels,
/// left-hand identity only. That is deliberate — its whole job is to separate
/// two candidate implementations of the `left_leg_id` arm, and one point of the
/// space where they differ suffices for that. It is **not** a general unitality
/// pin: it does not sweep apex sizes beyond the `n = 3` sibling below, does not
/// touch the right-unitality mirror, and says nothing about non-identity
/// operands.
#[test]
fn strict_left_unitality_with_a_non_monotone_left_leg() {
    let f = Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b']).expect("in-bounds fixture");
    let id = Cospan::identity(&f.domain());

    let result = id.compose(&f).expect("id ; f must compose");

    assert_eq!(
        result.middle(),
        f.middle(),
        "apex order: id ; f must return f's apex verbatim, got {:?} want {:?} \
         (the union-find numbering gives ['b', 'a'] here)",
        result.middle(),
        f.middle()
    );
    assert_eq!(
        result.left_to_middle(),
        f.left_to_middle(),
        "left leg: got {:?} want {:?} (the union-find numbering gives [0, 1])",
        result.left_to_middle(),
        f.left_to_middle()
    );
    assert_eq!(
        result.right_to_middle(),
        f.right_to_middle(),
        "right leg: got {:?} want {:?} (the union-find numbering gives [1, 0])",
        result.right_to_middle(),
        f.right_to_middle()
    );
}

/// The same law one apex size up, where the partner's left leg is a 3-cycle.
///
/// A numbering bug that happened to be an involution could hide in the `n = 2`
/// fixture above; `g.left = [2, 0, 1]` differs from the apex order by more than
/// a transposition, so it cannot. Same oracle: the left operand is an identity,
/// so the answer must be `g` itself, independently of what `perform_pushout`
/// does internally.
///
/// **What this ranges over.** One fixture, `n = 3`, `char` labels, left-hand
/// identity only. Measured falsification: disabling the `left_leg_id` arm makes
/// the composite come back as `left = [0, 1, 2]`, `right = [1, 2, 0]`,
/// `middle = ['a', 'b', 'c']` — the union-find numbering — against `g`'s
/// `[2, 0, 1]` / `[0, 1, 2]` / `['b', 'c', 'a']`.
#[test]
fn strict_left_unitality_at_three_vertices_with_a_cyclic_left_leg() {
    let g =
        Cospan::new(vec![2, 0, 1], vec![0, 1, 2], vec!['b', 'c', 'a']).expect("in-bounds fixture");
    let id = Cospan::identity(&g.domain());

    let result = id.compose(&g).expect("id ; g must compose");

    assert_cospan_eq_msg(&result, &g, "id ; g must be g on the nose");
}

/// The `left_leg_id` arm on an operand that is **not** an identity.
///
/// Unitality is unavailable as an oracle here — the composite is not the
/// partner — so the expectation is hand-derived from the arm's definition
/// (renumber `f`'s codomain-side apex by `g`'s indexing via `g.left`) rather
/// than read off a second run of the code under test. This is the shape that
/// says claim 1 is about `compose` generally and not only about units.
///
/// **What this ranges over.** One fixture, `n = 2`, `char` labels, with `f`'s
/// domain leg reversed so the renumbering is visible in the answer. Measured
/// falsification: disabling the `left_leg_id` arm gives `left = [1, 0]`,
/// `right = [1, 0]`, `middle = ['a', 'b']`.
#[test]
fn the_left_leg_id_arm_renumbers_by_the_right_operands_apex() {
    let f = Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b']).expect("in-bounds fixture");
    let g = Cospan::new(vec![1, 0], vec![0, 1], vec!['b', 'a']).expect("in-bounds partner");

    let composite = f.compose(&g).expect("f ; g composes");

    assert_eq!(
        (
            composite.left_to_middle(),
            composite.right_to_middle(),
            composite.middle()
        ),
        (&[0, 1][..], &[0, 1][..], &['b', 'a'][..]),
        "the fast path renumbers by g's apex, so f's reversed domain leg lands \
         back in order: got left = {:?}, right = {:?}, middle = {:?}",
        composite.left_to_middle(),
        composite.right_to_middle(),
        composite.middle(),
    );
}

/// Composing after a `connect_pair` merge: the CHANGELOG's retired "observable
/// side-effect", now a fixed value.
///
/// `Cospan`'s CHANGELOG carried this fixture (review R3-03) as evidence that a
/// merge is observable downstream — that composing after one could return a
/// different, isomorphic apex order, `structurally_equal` false and
/// `canonical_form` equal, so byte-level consumers had to compare canonical
/// forms. That held while a cached flag chose `perform_pushout`'s fast path.
/// The cache is gone, and this test is why both CHANGELOGs now say the claim
/// was retired: the merge produces `([1, 1], [0, 1], ['b', 'a'])`, and
/// composing that with `g` has exactly one answer.
///
/// **What this ranges over.** One fixture, one merge, one partner. It does not
/// sweep apex sizes or merge arguments, and it does not cover `connect_pair`'s
/// two no-op arms. Measured falsification: disabling `perform_pushout`'s
/// `left_leg_id` arm returns `left = [1, 1]`, `right = [1, 0]`,
/// `middle = ['b', 'a']` — the value the retired entry attributed to the flag
/// being **off**.
#[test]
fn a_connect_pair_merge_composes_to_the_merged_apex() {
    // The R3-03 fixture, merged for real: left [1, 2] -> [1, 1], and the apex
    // loses a vertex.
    let mut merged =
        Cospan::new(vec![1, 2], vec![0, 1], vec!['b', 'a', 'a']).expect("in-bounds fixture");
    merged.connect_pair(Left(0), Left(1));
    assert_eq!(
        (
            merged.left_to_middle(),
            merged.right_to_middle(),
            merged.middle()
        ),
        (&[1, 1][..], &[0, 1][..], &['b', 'a'][..]),
        "the merge must produce the triple the composition below is derived from"
    );

    let g = Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b']).expect("in-bounds partner");
    let composite = merged.compose(&g).expect("merged ; g composes");

    assert_eq!(
        (
            composite.left_to_middle(),
            composite.right_to_middle(),
            composite.middle()
        ),
        (&[0, 0][..], &[0, 1][..], &['a', 'b'][..]),
        "got left = {:?}, right = {:?}, middle = {:?}",
        composite.left_to_middle(),
        composite.right_to_middle(),
        composite.middle(),
    );
}

/// A label whose `Eq` is deliberately coarser than its identity: two `Tagged`
/// values compare equal when their `sort` agrees, whatever their `origin`.
///
/// `Cospan` only ever requires `Lambda: Eq`, never that `Eq` be identity, so
/// this is a legal label — and it is the only way to *see* which operand's
/// apex a both-legs-identity compose keeps.
#[derive(Clone, Copy, Debug)]
struct Tagged {
    sort: char,
    origin: &'static str,
}

impl PartialEq for Tagged {
    fn eq(&self, other: &Self) -> bool {
        self.sort == other.sort
    }
}

impl Eq for Tagged {}

/// When both legs are the identity, `perform_pushout` enters the `left_leg_id`
/// arm and the composite keeps the **right** operand's apex labels.
///
/// The two arms tag `representative` differently (`Right(i)` vs `Left(i)`),
/// and that tag is what decides whose labels the composite carries. Under
/// every label type in this workspace the choice is invisible, because
/// `composable` has already forced `self.middle[i] == other.middle[i]` and
/// their `Eq` is identity. It is invisible only that far: with a `Lambda`
/// whose `Eq` is coarser than its identity the two arms give visibly different
/// answers, so the arm order is a real decision and not an implementation
/// detail. This pins which way it currently goes.
///
/// **What this ranges over.** One fixture, `n = 2`, both legs the identity on
/// both operands — the only configuration in which both predicates hold at
/// once. It observes provenance through a field `Eq` ignores; it does not
/// claim anything about label types whose `Eq` *is* identity, for which the
/// question has no observable answer. Swapping the two arms in
/// `perform_pushout` turns `["right", "right"]` into `["left", "left"]` here,
/// which is the falsification — measured by disabling the `left_leg_id` arm,
/// which for a both-identity fixture is the same perturbation (the second arm
/// takes over and tags `Left(..)`).
#[test]
fn both_legs_identity_keeps_the_right_operands_labels() {
    let l = Cospan::new(
        vec![0, 1],
        vec![0, 1],
        vec![
            Tagged {
                sort: 'a',
                origin: "left",
            },
            Tagged {
                sort: 'b',
                origin: "left",
            },
        ],
    )
    .expect("in-bounds fixture");
    let r = Cospan::new(
        vec![0, 1],
        vec![0, 1],
        vec![
            Tagged {
                sort: 'a',
                origin: "right",
            },
            Tagged {
                sort: 'b',
                origin: "right",
            },
        ],
    )
    .expect("in-bounds fixture");

    assert!(
        l.is_right_identity() && r.is_left_identity(),
        "both predicates must hold, else only one arm is reachable and this \
         test is measuring nothing"
    );

    let composite = l
        .compose(&r)
        .expect("both operands' interfaces agree under Tagged's Eq");

    assert_eq!(
        composite
            .middle()
            .iter()
            .map(|t| t.origin)
            .collect::<Vec<_>>(),
        vec!["right", "right"],
        "the `left_leg_id` arm runs first and tags `Right(..)`, so the right \
         operand's apex is the one kept; got {:?}",
        composite.middle(),
    );
    assert_eq!(
        composite
            .middle()
            .iter()
            .map(|t| t.sort)
            .collect::<Vec<_>>(),
        vec!['a', 'b'],
        "the sorts are equal either way — that is why this needed a coarse `Eq` \
         to observe at all"
    );
    assert_eq!(composite.left_to_middle(), &[0, 1]);
    assert_eq!(composite.right_to_middle(), &[0, 1]);
}
