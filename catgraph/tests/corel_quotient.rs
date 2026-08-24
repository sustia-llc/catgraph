//! The total `Cospan → Corel` quotient
//! ([`Corel::from_cospan_dropping_bubbles`]) and the composition it repairs
//! ([#351](https://github.com/sustia-llc/catgraph/issues/351)).
//!
//! `Corel::new` *rejects* a cospan that is not jointly surjective, so before
//! #351 a bubble-carrying cospan could not enter the corelation world at all —
//! and `Corel::compose` returned values `Corel::new` would have rejected,
//! because pushout composition **creates** bubbles. F&S 2018 (*Seven Sketches*)
//! Example 4.61 fn. 2 makes corelation composition three steps: (i) read both
//! as relations on `A ⊔ B ⊔ C`, (ii) transitive closure of the union, (iii)
//! restrict to `A ⊔ C`. The pushout is (i) + (ii); the quotient is (iii).
//!
//! # The corpus these sweeps range over
//!
//! [`corpus`] enumerates **every** cospan with apex size ≤ 3, domain ≤ 2 and
//! codomain ≤ 2 over a single wire type — every leg map, not a sample. That is
//! 228 cospans, of which 139 carry at least one bubble; every sweep below
//! reports both figures on failure, and [`corpus_is_not_vacuous`] asserts the
//! bubble count directly so that no other test in this file can be about
//! nothing. A single wire type makes every arity-matching pair composable,
//! which is what lets the pair sweeps range over all 25 616 of them.
//!
//! What the corpus does **not** reach: heterogeneous `Lambda`, apexes above 3,
//! boundaries above 2, and any `Lambda` whose `Eq` is coarser than its
//! identity. Label handling and apex-order preservation are pinned separately,
//! on explicit heterogeneous witnesses, in
//! [`quotient_keeps_surviving_labels_in_their_original_order`].

use catgraph::{category::Composable, corel::Corel, cospan::Cospan};

// ---------------------------------------------------------------------------
// Corpus
// ---------------------------------------------------------------------------

/// Every `len`-tuple over `0..radix`, in lexicographic order.
fn tuples(len: usize, radix: usize) -> Vec<Vec<usize>> {
    if radix == 0 {
        // The only leg into an empty apex is the empty leg.
        return if len == 0 { vec![vec![]] } else { vec![] };
    }
    let mut out = vec![Vec::new()];
    for _ in 0..len {
        let mut next = Vec::with_capacity(out.len() * radix);
        for base in &out {
            for v in 0..radix {
                let mut t = base.clone();
                t.push(v);
                next.push(t);
            }
        }
        out = next;
    }
    out
}

/// Every cospan with apex ≤ 3, domain ≤ 2, codomain ≤ 2 over the wire type
/// `'a'` — exhaustive in the leg maps, not sampled.
fn corpus() -> Vec<Cospan<char>> {
    let mut out = Vec::new();
    for apex in 0..=3usize {
        for dom in 0..=2usize {
            for cod in 0..=2usize {
                for left in tuples(dom, apex) {
                    for right in tuples(cod, apex) {
                        out.push(
                            Cospan::new(left.clone(), right, vec!['a'; apex])
                                .expect("invariant: every tuple entry is below the apex size"),
                        );
                    }
                }
            }
        }
    }
    out
}

/// How many apex vertices of `c` are reached by neither leg.
fn bubble_count(c: &Cospan<char>) -> usize {
    (0..c.middle().len())
        .filter(|v| !c.left_to_middle().contains(v) && !c.right_to_middle().contains(v))
        .count()
}

/// Every arity-matching ordered pair drawn from [`corpus`], with the raw
/// (un-quotiented) `Cospan` pushout of each.
fn composable_pairs() -> Vec<(Cospan<char>, Cospan<char>, Cospan<char>)> {
    let items = corpus();
    let mut out = Vec::new();
    for a in &items {
        for b in &items {
            if a.codomain().len() != b.domain().len() {
                continue;
            }
            let Ok(raw) = a.compose(b) else { continue };
            out.push((a.clone(), b.clone(), raw));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Non-vacuity — measured first, so nothing below is about nothing
// ---------------------------------------------------------------------------

/// The corpus really carries bubbles, and composition really creates them.
///
/// Every sweep in this file is a claim about bubble-dropping. A corpus of
/// jointly-surjective cospans, or a pair sweep whose pushouts never grow a
/// bubble, would make all of them pass for free. This measures all four
/// quantities the rest of the file leans on and asserts each is non-zero,
/// naming the measured value in every message.
///
/// Ranges over exactly the corpus described in the module docs, so the counts
/// are properties of *that* enumeration; they move if it is retuned, and the
/// messages print the value seen so drift reads as drift rather than as a
/// regression.
#[test]
fn corpus_is_not_vacuous() {
    let items = corpus();
    let bubbly = items.iter().filter(|c| bubble_count(c) > 0).count();
    let total_bubbles: usize = items.iter().map(bubble_count).sum();
    assert!(
        bubbly > 0,
        "corpus of {} cospans carries no bubbles at all ({total_bubbles} bubble vertices in \
         total) — every quotient claim in this file would hold vacuously",
        items.len()
    );

    let pairs = composable_pairs();
    let bubble_born = pairs
        .iter()
        .filter(|(a, b, raw)| bubble_count(raw) > bubble_count(a) + bubble_count(b))
        .count();
    let js_pairs_with_bubbly_pushout = pairs
        .iter()
        .filter(|(a, b, raw)| {
            a.is_jointly_surjective() && b.is_jointly_surjective() && bubble_count(raw) > 0
        })
        .count();
    assert!(
        bubble_born > 0,
        "no pair among {} composable pairs grows a bubble the operands did not already carry — \
         the composition sweeps below would be about nothing",
        pairs.len()
    );
    assert!(
        js_pairs_with_bubbly_pushout > 0,
        "no pair of genuine corelations among {} composable pairs has a bubble-carrying raw \
         pushout — `compose_result_is_always_a_corelation` would hold vacuously",
        pairs.len()
    );

    println!(
        "corpus {} cospans, {bubbly} bubbly ({total_bubbles} bubble vertices); \
         {} composable pairs, {bubble_born} grow a new bubble, \
         {js_pairs_with_bubbly_pushout} jointly-surjective pairs have a bubbly raw pushout",
        items.len(),
        pairs.len(),
    );
}

// ---------------------------------------------------------------------------
// Half 1 — the quotient
// ---------------------------------------------------------------------------

/// The quotient is **total**, its image is always a corelation, and it is the
/// identity on inputs that already are one.
///
/// Six claims, each checked on every one of the 228 corpus cospans:
/// the image is jointly surjective; `Corel::new` *accepts* it (the codomain
/// claim checked against the validator, not assumed); its canonical form has
/// `scalar_count() == 0`; domain and codomain are untouched; the apex shrinks
/// by exactly the bubble count; and `q` is idempotent.
///
/// Ranges over the module-doc corpus only. It says nothing about heterogeneous
/// labels (see [`quotient_keeps_surviving_labels_in_their_original_order`]),
/// larger apexes, or `Lambda`s with a coarse `Eq`.
#[test]
fn quotient_is_total_and_lands_in_corel() {
    for c in corpus() {
        let bubbles = bubble_count(&c);
        let q = Corel::from_cospan_dropping_bubbles(c.clone());
        let image = q.as_cospan();

        assert!(
            image.is_jointly_surjective(),
            "quotient of {c:?} (with {bubbles} bubbles) is still not jointly surjective: {image:?}"
        );
        assert!(
            Corel::new(image.clone()).is_ok(),
            "Corel::new rejects the quotient's own image for {c:?}: {image:?}"
        );
        assert_eq!(
            image.canonical_form().scalar_count(),
            0,
            "quotient of {c:?} left a scalar class in {image:?}"
        );
        assert_eq!(
            image.domain(),
            c.domain(),
            "quotient moved the domain of {c:?}"
        );
        assert_eq!(
            image.codomain(),
            c.codomain(),
            "quotient moved the codomain of {c:?}"
        );
        assert_eq!(
            image.middle().len(),
            c.middle().len() - bubbles,
            "quotient of {c:?} should drop exactly its {bubbles} bubble(s), got apex {:?}",
            image.middle()
        );

        // Idempotent: q(q(c)) == q(c), field for field.
        let twice = Corel::from_cospan_dropping_bubbles(image.clone());
        assert_eq!(
            twice.as_cospan(),
            image,
            "quotient is not idempotent on {c:?}: {:?} then {:?}",
            image,
            twice.as_cospan()
        );

        // The identity on an input that is already a corelation. `Cospan`'s
        // `PartialEq` cannot see whether the value was returned or rebuilt, so
        // this pins the *equality*, not the early return that delivers it.
        if c.is_jointly_surjective() {
            assert_eq!(
                image, &c,
                "quotient is not the identity on the jointly-surjective {c:?}"
            );
        }
    }
}

/// The quotient **reindexes** the legs onto the survivors, rather than merely
/// filtering the apex, and it keeps the survivors in their original relative
/// order carrying their original labels.
///
/// Filtering without reindexing is the failure mode the issue names, and it is
/// invisible to a corpus of same-labelled vertices: with every apex vertex
/// labelled `'a'`, a leg pointing at a *wrong but in-bounds* index still lands
/// on an `'a'` and every sweep in this file stays green. Measured, not
/// asserted: renumbering the survivors in reverse order leaves
/// [`quotient_is_total_and_lands_in_corel`],
/// [`compose_result_is_always_a_corelation`] and
/// [`quotient_is_functorial_up_to_apex_isomorphism`] all passing, and this test
/// is the only one in the file that catches it on the label
/// (`['q', 'p']` where `['p', 'q']` was expected). These witnesses use distinct
/// labels for that reason, and cover a bubble before, between and after the
/// survivors, plus an all-bubble apex.
///
/// Ranges over four hand-built witnesses at apex ≤ 4 — one per bubble position.
/// It is not a sweep and claims nothing beyond those four shapes.
#[test]
fn quotient_keeps_surviving_labels_in_their_original_order() {
    // Bubble first: apex ['z', 'p', 'q'], legs reach 1 and 2.
    let c = Cospan::new(vec![1], vec![2], vec!['z', 'p', 'q']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c);
    assert_eq!(q.as_cospan().middle(), &['p', 'q']);
    assert_eq!(q.as_cospan().left_to_middle(), &[0]);
    assert_eq!(q.as_cospan().right_to_middle(), &[1]);

    // Bubble in the middle: apex ['p', 'z', 'q'].
    let c = Cospan::new(vec![0], vec![2], vec!['p', 'z', 'q']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c);
    assert_eq!(q.as_cospan().middle(), &['p', 'q']);
    assert_eq!(q.as_cospan().left_to_middle(), &[0]);
    assert_eq!(q.as_cospan().right_to_middle(), &[1]);

    // Two bubbles at the end, and a leg with a repeated target:
    // apex ['p', 'q', 'z', 'z'], domain hits q then p.
    let c = Cospan::new(vec![1, 0], vec![1], vec!['p', 'q', 'z', 'z']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c);
    assert_eq!(q.as_cospan().middle(), &['p', 'q']);
    assert_eq!(q.as_cospan().left_to_middle(), &[1, 0]);
    assert_eq!(q.as_cospan().right_to_middle(), &[1]);

    // Every vertex a bubble: the image is the empty corelation.
    let c = Cospan::new(vec![], vec![], vec!['z', 'y']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c);
    assert!(q.as_cospan().middle().is_empty());
    assert!(q.as_cospan().is_jointly_surjective());
}

// ---------------------------------------------------------------------------
// Half 2 — the composition it repairs
// ---------------------------------------------------------------------------

/// The witness from #351: two corelations whose pushout is **not** one.
///
/// `a : 0 → {m} ← 1` and `b : 1 → {m} ← 0` are both jointly surjective, so
/// `Corel::new` accepts both. The pushout glues `a`'s right-leg-only vertex to
/// `b`'s left-leg-only vertex into a class no *outer* leg reaches, so the raw
/// composite carries one scalar and `Corel::new` would reject it — which is
/// what the deleted comment at `corel.rs:310` asserted could not happen. Step
/// (iii) of F&S 2018 Ex 4.61 fn. 2 drops it.
///
/// Ranges over exactly this one pair. It is the *existence* witness — the
/// universal claim it falsifies is pinned over the whole corpus by
/// [`compose_result_is_always_a_corelation`].
#[test]
fn compose_restricts_to_the_outer_boundary() {
    let a = Cospan::<char>::new(vec![], vec![0], vec!['m']).unwrap();
    let b = Cospan::<char>::new(vec![0], vec![], vec!['m']).unwrap();
    assert!(a.is_jointly_surjective() && b.is_jointly_surjective());

    // (i) + (ii): the raw pushout. Not a corelation.
    let raw = a.compose(&b).unwrap();
    assert_eq!(
        raw.middle(),
        &['m'],
        "the pushout should keep one apex vertex"
    );
    assert!(raw.left_to_middle().is_empty());
    assert!(raw.right_to_middle().is_empty());
    assert!(
        !raw.is_jointly_surjective(),
        "the raw pushout is jointly surjective — this witness no longer witnesses anything"
    );
    assert_eq!(raw.canonical_form().scalar_count(), 1);
    assert!(Corel::new(raw.clone()).is_err());

    // (iii): Corel's own composition drops it.
    let ca = Corel::new(a).unwrap();
    let cb = Corel::new(b).unwrap();
    let composed = ca.compose(&cb).unwrap();
    assert!(
        composed.as_cospan().middle().is_empty(),
        "Corel::compose kept a mid-born bubble: apex {:?} (raw pushout apex was {:?})",
        composed.as_cospan().middle(),
        raw.middle()
    );
    assert!(composed.as_cospan().is_jointly_surjective());
    assert_eq!(composed.as_cospan().canonical_form().scalar_count(), 0);
}

/// `Corel::compose` returns something `Corel::new` accepts — over every
/// composable pair of corelations the corpus offers, not one.
///
/// This is the universal claim `tests/corel.rs`'s
/// `compose_of_fold_then_unfold_is_jointly_surjective` used to be *named*
/// after while asserting it of a single input pair. Its name is now honest and
/// the universal reading lives here, swept over 4 803 pairs; the count of
/// pairs whose *raw* pushout is not jointly surjective is measured and asserted
/// non-zero, so the sweep cannot pass by never meeting the case.
///
/// Ranges over the module-doc corpus restricted to jointly-surjective members
/// — every ordered arity-matching pair of them. Single wire type, apex ≤ 3,
/// boundary ≤ 2.
#[test]
fn compose_result_is_always_a_corelation() {
    let corels: Vec<Cospan<char>> = corpus()
        .into_iter()
        .filter(Cospan::is_jointly_surjective)
        .collect();

    let mut pairs = 0usize;
    let mut raw_not_jointly_surjective = 0usize;
    for a in &corels {
        for b in &corels {
            if a.codomain().len() != b.domain().len() {
                continue;
            }
            let Ok(raw) = a.compose(b) else { continue };
            pairs += 1;
            if !raw.is_jointly_surjective() {
                raw_not_jointly_surjective += 1;
            }

            let composed = Corel::new(a.clone())
                .unwrap()
                .compose(&Corel::new(b.clone()).unwrap())
                .unwrap();
            assert!(
                composed.as_cospan().is_jointly_surjective(),
                "compose({a:?}, {b:?}) is not jointly surjective: {:?}",
                composed.as_cospan()
            );
            assert!(
                Corel::new(composed.as_cospan().clone()).is_ok(),
                "Corel::new rejects Corel::compose's own output for ({a:?}, {b:?}): {:?}",
                composed.as_cospan()
            );
            assert_eq!(
                composed.as_cospan().canonical_form().scalar_count(),
                0,
                "compose({a:?}, {b:?}) kept a scalar class: {:?}",
                composed.as_cospan()
            );
            // The restriction touches the apex only: the partition the
            // composite induces on domain ⊔ codomain is the pushout's.
            assert_eq!(
                composed.as_cospan().domain(),
                a.domain(),
                "compose moved the domain"
            );
            assert_eq!(
                composed.as_cospan().codomain(),
                b.codomain(),
                "compose moved the codomain"
            );
        }
    }
    assert!(
        raw_not_jointly_surjective > 0,
        "none of the {pairs} corelation pairs had a raw pushout that was not jointly \
         surjective — this sweep never met the case it exists to cover"
    );
    println!(
        "{pairs} corelation pairs, {raw_not_jointly_surjective} of whose raw pushouts are not \
         jointly surjective"
    );
}

// ---------------------------------------------------------------------------
// Functoriality — what holds, and exactly what does not
// ---------------------------------------------------------------------------

/// `q(a ; b) == q(a) ; q(b)` **up to apex isomorphism**, over every composable
/// pair — with `;` the `Cospan` pushout on the left and `Corel::compose` on
/// the right.
///
/// `CospanCanon` is a complete invariant for parallel-cospan equality (equal
/// canonical forms iff isomorphic apex bijection commuting with both legs), so
/// comparing canonical forms is comparing the corelations themselves: two
/// values that agree here induce the same equivalence relation on
/// `domain ⊔ codomain` and differ at most in how the apex is numbered.
///
/// ⚠ This is **not** on-the-nose equality, and the gap is real, not an
/// artefact of how the comparison is written:
/// [`quotient_functoriality_is_not_structural`] measures it, characterises it,
/// and names its cause. Read the two together — this test alone would let a
/// reader believe more than was measured.
///
/// Ranges over all 25 616 ordered arity-matching pairs of the module-doc
/// corpus. Single wire type, apex ≤ 3, boundary ≤ 2; nothing here speaks to
/// larger diagrams or heterogeneous labels.
#[test]
fn quotient_is_functorial_up_to_apex_isomorphism() {
    let pairs = composable_pairs();
    let mut mismatches = 0usize;
    let mut first: Option<String> = None;
    for (a, b, raw) in &pairs {
        let lhs = Corel::from_cospan_dropping_bubbles(raw.clone());
        let rhs = Corel::from_cospan_dropping_bubbles(a.clone())
            .compose(&Corel::from_cospan_dropping_bubbles(b.clone()))
            .unwrap();
        if lhs.as_cospan().canonical_form() != rhs.as_cospan().canonical_form() {
            mismatches += 1;
            if first.is_none() {
                first = Some(format!(
                    "a={a:?} b={b:?}\n  q(a;b) = {:?}\n  q(a);q(b) = {:?}",
                    lhs.as_cospan(),
                    rhs.as_cospan()
                ));
            }
        }
    }
    assert_eq!(
        mismatches,
        0,
        "q is not functorial up to apex isomorphism: {mismatches} of {} pairs differ. First:\n  {}",
        pairs.len(),
        first.unwrap_or_default()
    );
}

/// The residual: functoriality is **not** on the nose, and the reason is
/// `perform_pushout`'s identity fast paths, not the quotient.
///
/// Recorded rather than papered over. Three things are measured over the same
/// 25 616 pairs and asserted:
///
/// 1. structural mismatches exist (so the "up to apex isomorphism" hedge in
///    [`quotient_is_functorial_up_to_apex_isomorphism`] is load-bearing, not
///    defensive wording);
/// 2. **every** mismatch has a bubble-carrying operand — with both operands
///    already jointly surjective, `q` is the identity on them and the two
///    sides are literally the same expression, so the gap can only open where
///    `q` does something;
/// 3. **every** mismatch is a pair where `perform_pushout`'s identity fast
///    path fires on one side of the quotient and not the other. Dropping a
///    bubble can *make* a leg the identity on its apex
///    (`right=[0]` over a 2-vertex apex is not, over the 1-vertex quotient it
///    is), and `cospan.rs`'s `left_leg_id` arm deliberately numbers the
///    composite apex by the right operand's indexing where union-find numbers
///    it by left-leg discovery order. That arm is load-bearing for strict left
///    unitality (`id ; g == g`, pinned in `tests/compose_identity_arms.rs`),
///    so the gap is a property of composition's apex numbering that the
///    quotient *exposes*, not one it introduces.
///
/// Claim 3 is checked against a locally recomputed `leg_is_identity` (the
/// crate's is private), so it pins the *correlation*, not the crate's own
/// predicate — a rewrite of `leg_is_identity` would not redden it. It is
/// evidence for the diagnosis, not a proof of it.
///
/// Exact counts are printed rather than asserted: they are properties of this
/// corpus, and an assertion on them would report a corpus retune as a
/// regression. What is asserted is the shape of the residual.
#[test]
fn quotient_functoriality_is_not_structural() {
    fn leg_is_identity(leg: &[usize], apex: usize) -> bool {
        leg.len() == apex && leg.iter().enumerate().all(|(i, &v)| v == i)
    }

    let pairs = composable_pairs();
    let mut structural_mismatches = 0usize;
    let mut mismatches_with_both_operands_jointly_surjective = 0usize;
    let mut mismatches_with_a_fast_path_flip = 0usize;
    let mut fast_path_flips = 0usize;

    for (a, b, raw) in &pairs {
        let lhs = Corel::from_cospan_dropping_bubbles(raw.clone());
        let qa = Corel::from_cospan_dropping_bubbles(a.clone());
        let qb = Corel::from_cospan_dropping_bubbles(b.clone());
        let rhs = qa.compose(&qb).unwrap();

        let before = leg_is_identity(a.right_to_middle(), a.middle().len())
            || leg_is_identity(b.left_to_middle(), b.middle().len());
        let after = leg_is_identity(
            qa.as_cospan().right_to_middle(),
            qa.as_cospan().middle().len(),
        ) || leg_is_identity(
            qb.as_cospan().left_to_middle(),
            qb.as_cospan().middle().len(),
        );
        let flipped = before != after;
        if flipped {
            fast_path_flips += 1;
        }

        if lhs.as_cospan() != rhs.as_cospan() {
            structural_mismatches += 1;
            if a.is_jointly_surjective() && b.is_jointly_surjective() {
                mismatches_with_both_operands_jointly_surjective += 1;
            }
            if flipped {
                mismatches_with_a_fast_path_flip += 1;
            }
        }
    }

    assert!(
        structural_mismatches > 0,
        "no structural mismatch among {} pairs — functoriality now holds on the nose, and \
         `quotient_is_functorial_up_to_apex_isomorphism` should be strengthened to `==` rather \
         than left hedging about a gap that has closed",
        pairs.len()
    );
    assert_eq!(
        mismatches_with_both_operands_jointly_surjective, 0,
        "{mismatches_with_both_operands_jointly_surjective} of {structural_mismatches} structural \
         mismatches have two already-jointly-surjective operands, where q is the identity and \
         both sides are the same expression — the gap is no longer confined to inputs the \
         quotient actually changes"
    );
    assert_eq!(
        mismatches_with_a_fast_path_flip,
        structural_mismatches,
        "{} of {structural_mismatches} structural mismatches are NOT explained by a \
         perform_pushout identity-fast-path flip ({fast_path_flips} pairs flip in total) — the \
         residual has a second cause this test does not name",
        structural_mismatches - mismatches_with_a_fast_path_flip
    );
    println!(
        "{structural_mismatches} of {} pairs differ structurally; all carry a fast-path flip \
         ({fast_path_flips} pairs flip in total, so the flip is necessary and not sufficient)",
        pairs.len()
    );
}
