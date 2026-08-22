//! `Cospan::compose` is a function of `(left, right, middle)` — of nothing else.
//!
//! Three separable claims about `perform_pushout`'s two identity fast paths,
//! pinned separately because they can fail independently:
//!
//! 1. **Which answer the `left_leg_id` arm gives.** It numbers the composite
//!    apex by the *right* operand's own indexing, where the union-find body
//!    numbers it by left-leg discovery order. That choice is strict left
//!    unitality — `id ; f == f` on the nose — and it is only visible on an `f`
//!    whose left leg does not first-visit its apex in increasing order.
//! 2. **That the arm is entered on the legs, not on a cache.** `Cospan` carries
//!    `is_left_id` / `is_right_id`, and they are *conservative*: the `&=`
//!    mutators only ever clear, so a leg that is the identity can carry
//!    `false`. Until the r4 review of
//!    [#289](https://github.com/sustia-llc/catgraph/issues/289),
//!    `compose_with_quotient` handed those cached flags to `perform_pushout`,
//!    which made the composite a function of each operand's **mutation
//!    history** as well as its public state: two structurally equal cospans
//!    composed two different ways. `perform_pushout` now re-derives the
//!    predicate with `leg_is_identity`.
//! 3. **Which arm wins when both predicates hold.** The `left_leg_id` arm is
//!    tried first and tags the composite apex's representatives `Right(..)`,
//!    so the composite keeps the *right* operand's labels. `composable` has
//!    already forced the two apexes equal under `Lambda`'s `Eq`, so for every
//!    label type in this workspace there is nothing to see — but `Cospan` only
//!    requires `Eq`, never that `Eq` be identity, and under a coarser `Eq` the
//!    two arms give visibly different answers.
//!
//! Claim 2 has exactly one observable shape, and its tests below are confined
//! to it on purpose. A stale `false` on the *right* operand's `is_left_id`
//! (`perform_pushout`'s `right_leg_id`) changes nothing at all: that arm returns
//! field-for-field what the union-find body returns for the same input, so
//! whether it is taken is unobservable. Only a stale `false` on the **left**
//! operand's `is_right_id`, over a right leg that really is the identity, can
//! move a composite — and only when the partner's left leg is not the identity
//! and does not first-visit in increasing order.

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
};
use either::Either::{Left, Right};

/// Build a cospan from slices, twice: once through `Cospan::new`, so its flags
/// are computed from the legs, and once through a mutation round trip that
/// leaves `is_right_id` a conservative `false` on a codomain leg that *is* the
/// identity.
///
/// The round trip is `delete_boundary_node(Right(last))` — an unconditional
/// clear since #289 — followed by `add_boundary_node_known_target(Right(last))`,
/// whose existing-target arm also clears unconditionally and so never restores
/// the `true`. `swap_remove` on the last port is a plain pop, so the leg comes
/// back byte-identical; only the flag differs. This is the codomain-side twin
/// of the route `checked_mutators.rs`'s
/// `cospan_identity_flags_are_conservative_in_the_false_direction` documents.
///
/// # Panics
///
/// If the fixture's codomain leg is not already the identity on its apex, or if
/// the round trip fails to reproduce it — either would make the returned pair
/// differ in more than the flag, and the tests below would be measuring
/// something other than the flag.
fn fresh_and_stale(
    left: &[usize],
    right: &[usize],
    middle: &[char],
) -> (Cospan<char>, Cospan<char>) {
    let build = || {
        Cospan::new(left.to_vec(), right.to_vec(), middle.to_vec())
            .expect("fixture legs are in bounds")
    };

    let fresh = build();
    assert!(
        fresh.is_right_identity(),
        "fixture must start with a genuine codomain identity, else there is no \
         stale `false` to make: right = {:?} over a {}-vertex apex",
        right,
        middle.len()
    );

    let mut stale = build();
    let last = right.len() - 1;
    stale.delete_boundary_node(Right(last));
    stale
        .add_boundary_node_known_target(Right(last))
        .expect("re-adding the port that was just removed targets an in-bounds apex vertex");
    assert_eq!(
        stale.right_to_middle(),
        right,
        "the round trip must restore the leg exactly — only the flag may differ"
    );
    assert!(
        !stale.is_right_identity(),
        "the round trip must leave the flag stale-`false`; if the mutators ever \
         become exact in this direction this helper stops testing anything and \
         must be replaced, not deleted"
    );

    (fresh, stale)
}

/// Assert two composites agree field for field, reporting all three fields.
fn assert_same_composite(got: &Cospan<char>, want: &Cospan<char>, what: &str) {
    assert!(
        got.structurally_equal(want),
        "{what}\n  got  left = {:?}, right = {:?}, middle = {:?}\
         \n  want left = {:?}, right = {:?}, middle = {:?}",
        got.left_to_middle(),
        got.right_to_middle(),
        got.middle(),
        want.left_to_middle(),
        want.right_to_middle(),
        want.middle(),
    );
}

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
/// pin: it does not sweep apex sizes, does not touch the right-unitality
/// mirror, and says nothing about non-identity operands. Both operands here
/// have honest flags, so it is also blind to claim 2 above — that is the next
/// test's job.
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
/// which is the falsification.
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

// ---------------------------------------------------------------------------
// Claim 2: the arm is entered on the legs, not on the cache.
//
// Four shapes, kept as four tests rather than one so that a regression reports
// every shape it breaks in a single run — the shapes differ in what oracle is
// available to them, so a failure in one says something different from a
// failure in another.
//
// Each builds the same three vectors more than once — by `Cospan::new`, by the
// round trip that stales `is_right_id`, and in shape D also by a real
// `connect_pair` merge — and composes each with the same partner. The
// differential assertion (the composites agree) is the pin. Each shape also
// carries an **absolute** expectation, so it cannot pass by both sides
// agreeing on a wrong answer; that is not decoration, and it is what catches
// the `left_leg_id` arm being disabled, a perturbation the differential alone
// survives.
//
// Measured with the flag parameters restored (the fix reverted). Every row was
// observed, not derived-and-assumed:
//
// | shape | stale operand, pre-fix | fresh operand, and both post-fix |
// |-------|------------------------|----------------------------------|
// | A | `left = [0, 1], right = [1, 0], middle = ['a', 'b']` | `left = [1, 0], right = [0, 1], middle = ['b', 'a']` |
// | B | `left = [0, 1, 2], right = [1, 2, 0], middle = ['a', 'b', 'c']` | `left = [2, 0, 1], right = [0, 1, 2], middle = ['b', 'c', 'a']` |
// | C | `left = [1, 0], right = [1, 0], middle = ['a', 'b']` | `left = [0, 1], right = [0, 1], middle = ['b', 'a']` |
// | D | `left = [1, 1], right = [1, 0], middle = ['b', 'a']` | `left = [0, 0], right = [0, 1], middle = ['a', 'b']` |
//
// The canonical forms are equal in every row, before and after: both answers
// are legitimate pushouts and the difference is only apex numbering. Those
// assertions are therefore **not** the pin — they pass with the fix reverted —
// and are here to say what did *not* change.
//
// Between them the four shapes cover the whole *observable* shape of the
// change, as the module docs argue, at four points — not exhaustively, and
// with two of the three known routes to a flag disagreement (delete-then-re-add
// in A/B/C, a `connect_pair` merge in D; the third, a composite's own
// unconditionally-cleared flags, is not exercised here, nor is `permute_side`
// with an identity permutation). None sweeps apex sizes past `n = 3` or label
// types past `char`, and none uses a partner whose left leg merges apex
// vertices. A `perform_pushout` that special-cased on apex size could pass all
// four and still be history-dependent.
// ---------------------------------------------------------------------------

/// A stale flag does not move `id ; g` at `n = 2`.
///
/// **What this ranges over.** One fixture. The left operand is the identity on
/// `['a', 'b']`, so strict left unitality is available as an oracle
/// independent of `perform_pushout`: the composite must be `g` itself, not
/// merely whatever the fresh operand produced.
#[test]
fn compose_ignores_a_stale_flag_on_an_identity_operand() {
    let (fresh, stale) = fresh_and_stale(&[0, 1], &[0, 1], &['a', 'b']);
    // g's left leg runs backwards, which is what makes the two numberings
    // differ; on a monotone left leg both arms agree and nothing is visible.
    let g = Cospan::new(vec![1, 0], vec![0, 1], vec!['b', 'a']).expect("in-bounds partner");

    let from_fresh = fresh.compose(&g).expect("fresh ; g composes");
    let from_stale = stale.compose(&g).expect("stale ; g composes");

    assert_same_composite(
        &from_stale,
        &from_fresh,
        "the composite must not depend on the cached flag (measured pre-fix, \
         the stale operand gave left = [0, 1], right = [1, 0], middle = \
         ['a', 'b'])",
    );
    assert_same_composite(&from_stale, &g, "id ; g must be g on the nose");
    assert_eq!(
        from_stale.canonical_form(),
        from_fresh.canonical_form(),
        "characterization, not the pin — this held pre-fix too: both answers \
         are legitimate pushouts differing only in apex numbering"
    );
}

/// The same at `n = 3`, where the partner's left leg is a 3-cycle.
///
/// **What this ranges over.** One fixture, one apex size larger than the
/// previous test, with a partner whose left leg differs from the apex order by
/// more than a transposition — so a numbering bug that happened to be an
/// involution could not hide in it. Same oracle: the left operand is an
/// identity, so the answer must be `g`.
#[test]
fn compose_ignores_a_stale_flag_on_a_three_vertex_identity_operand() {
    let (fresh, stale) = fresh_and_stale(&[0, 1, 2], &[0, 1, 2], &['a', 'b', 'c']);
    let g =
        Cospan::new(vec![2, 0, 1], vec![0, 1, 2], vec!['b', 'c', 'a']).expect("in-bounds partner");

    let from_fresh = fresh.compose(&g).expect("fresh ; g composes");
    let from_stale = stale.compose(&g).expect("stale ; g composes");

    assert_same_composite(
        &from_stale,
        &from_fresh,
        "the composite must not depend on the cached flag (measured pre-fix, \
         the stale operand gave left = [0, 1, 2], right = [1, 2, 0], middle = \
         ['a', 'b', 'c'])",
    );
    assert_same_composite(&from_stale, &g, "id ; g must be g on the nose");
    assert_eq!(
        from_stale.canonical_form(),
        from_fresh.canonical_form(),
        "characterization, not the pin — this held pre-fix too"
    );
}

/// The CHANGELOG's retired "observable side-effect": a `connect_pair` merge
/// that turns `is_right_id` **on** no longer moves the next composite.
///
/// `Cospan`'s CHANGELOG carried this fixture (review R3-03) as evidence that a
/// merge is observable downstream — that composing after one could return a
/// different, isomorphic apex order, `structurally_equal` false and
/// `canonical_form` equal, so byte-level consumers had to compare canonical
/// forms. That held while the flag chose the fast path. It does not hold in
/// what ships, and this test is why the CHANGELOG now says so.
///
/// The merge is performed for real (`connect_pair`), and the resulting triple
/// `([1, 1], [0, 1], ['b', 'a'])` is *also* built twice by hand — once with
/// honest flags, once staled — so all three operands are byte-equal and differ
/// only in `is_right_id`. All three must compose to the same thing. The
/// absolute expectation is the value the CHANGELOG attributed to the flag being
/// **on**; the value it attributed to the flag being **off**,
/// `left = [1, 1], right = [1, 0], middle = ['b', 'a']`, is now unreachable and
/// is the measured falsification of this test.
///
/// **What this ranges over.** One fixture, and the *second* route to a flag
/// disagreement (a real `connect_pair` merge, where the other tests in this
/// file use delete-then-re-add). It does not sweep apex sizes or merge
/// arguments, and it does not cover `connect_pair`'s two no-op arms, which
/// leave the flags untouched.
#[test]
fn a_connect_pair_merge_does_not_move_the_next_composite() {
    // The R3-03 fixture, merged for real: left [1, 2] -> [1, 1], apex loses a
    // vertex, and the codomain flag comes back on.
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
        "the merge must produce the triple the two hand-built operands use"
    );
    assert!(
        merged.is_right_identity(),
        "the merge turns `is_right_id` on — that is the premise of the retired \
         observable, and if it stops holding this test is measuring nothing"
    );

    // The same triple with honest flags and with a stale `false`.
    let (fresh, stale) = fresh_and_stale(&[1, 1], &[0, 1], &['b', 'a']);
    let g = Cospan::new(vec![1, 0], vec![0, 1], vec!['a', 'b']).expect("in-bounds partner");

    let from_merged = merged.compose(&g).expect("merged ; g composes");
    let from_fresh = fresh.compose(&g).expect("fresh ; g composes");
    let from_stale = stale.compose(&g).expect("stale ; g composes");

    assert_same_composite(
        &from_stale,
        &from_merged,
        "the composite must not depend on how the operand's flag got its value \
         (measured pre-fix, the stale operand gave left = [1, 1], \
         right = [1, 0], middle = ['b', 'a'])",
    );
    assert_same_composite(&from_fresh, &from_merged, "fresh vs merged");
    assert_eq!(
        (
            from_merged.left_to_middle(),
            from_merged.right_to_middle(),
            from_merged.middle()
        ),
        (&[0, 0][..], &[0, 1][..], &['a', 'b'][..]),
        "got left = {:?}, right = {:?}, middle = {:?}",
        from_merged.left_to_middle(),
        from_merged.right_to_middle(),
        from_merged.middle(),
    );
}

/// A stale flag does not move a composite whose left operand is *not* an
/// identity.
///
/// **What this ranges over.** One fixture whose domain leg is `[1, 0]`, so the
/// composite is not the partner and unitality is unavailable as an oracle; the
/// absolute expectation below is hand-derived from the fast path's definition
/// (renumber `f`'s codomain-side apex by `g`'s indexing via `g.left`) rather
/// than read off a second run of the code under test. This is the shape that
/// says the claim is about `compose` generally and not only about units.
#[test]
fn compose_ignores_a_stale_flag_on_a_non_identity_operand() {
    let (fresh, stale) = fresh_and_stale(&[1, 0], &[0, 1], &['a', 'b']);
    let g = Cospan::new(vec![1, 0], vec![0, 1], vec!['b', 'a']).expect("in-bounds partner");

    let from_fresh = fresh.compose(&g).expect("fresh ; g composes");
    let from_stale = stale.compose(&g).expect("stale ; g composes");

    assert_same_composite(
        &from_stale,
        &from_fresh,
        "the composite must not depend on the cached flag (measured pre-fix, \
         the stale operand gave left = [1, 0], right = [1, 0], middle = \
         ['a', 'b'])",
    );
    assert_eq!(
        (
            from_stale.left_to_middle(),
            from_stale.right_to_middle(),
            from_stale.middle()
        ),
        (&[0, 1][..], &[0, 1][..], &['b', 'a'][..]),
        "the fast path renumbers by g's apex, so f's reversed domain leg lands \
         back in order: got left = {:?}, right = {:?}, middle = {:?}",
        from_stale.left_to_middle(),
        from_stale.right_to_middle(),
        from_stale.middle(),
    );
    assert_eq!(
        from_stale.canonical_form(),
        from_fresh.canonical_form(),
        "characterization, not the pin — this held pre-fix too"
    );
}
