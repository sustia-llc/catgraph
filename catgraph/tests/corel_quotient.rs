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
//! 228 cospans, of which 139 carry at least one bubble. Those two figures are
//! measured and printed by [`corpus_is_not_vacuous`], which asserts the bubbly
//! count non-zero so that no other test in this file can be about nothing.
//! The sweeps below name the offending value on failure, and the pair sweeps
//! their own pair counts — no sweep reports these two corpus-level figures.
//! A single wire type makes every arity-matching pair composable, which is what
//! lets the pair sweeps range over all 25 616 of them.
//!
//! What the corpus does **not** reach: heterogeneous `Lambda`, apexes above 3,
//! boundaries above 2, and any `Lambda` whose `Eq` is coarser than its
//! identity. Label handling and apex-order preservation are pinned separately,
//! on explicit heterogeneous witnesses, in
//! [`quotient_keeps_surviving_labels_in_their_original_order`].
//!
//! ⚠ **One wire type makes the label-level assertions in the *sweeps* weak.**
//! `Cospan::domain` and `Cospan::codomain` read leg entries *through* the apex,
//! so with every apex vertex labelled `'a'` they are `vec!['a'; n]` whatever
//! the legs do, and the lengths are leg lengths the quotient never touches: a
//! `domain() == domain()` assertion over this corpus cannot fail. The claim
//! those assertions are shorthand for is the *partition* on `domain ⊔
//! codomain`, and that is what [`boundary_partition`] measures and the sweeps
//! assert. The heterogeneous witnesses in
//! [`quotient_keeps_surviving_labels_in_their_original_order`] are where the
//! label-level reading is earned.

use catgraph::{
    category::{Composable, HasIdentity},
    corel::Corel,
    cospan::Cospan,
    hypergraph_category::HypergraphCategory,
};

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
    corpus_up_to(3)
}

/// [`corpus`] with the apex bound as a parameter.
///
/// The triple sweep ([`new_composition_is_associative_up_to_apex_isomorphism`])
/// takes `2` rather than `3`: triples grow as the cube of the corpus, and at
/// apex ≤ 3 the composable-triple count is 261 625 against 14 473 at apex ≤ 2.
/// Both were measured, and both give the same verdict (0 mismatches up to apex
/// isomorphism); the smaller one is what the suite runs.
fn corpus_up_to(max_apex: usize) -> Vec<Cospan<char>> {
    let mut out = Vec::new();
    for apex in 0..=max_apex {
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

/// The equivalence relation a cospan induces on its **boundary** —
/// `domain ⊔ codomain` — as sorted classes of flat indices: `0..dom_len` is the
/// domain, `dom_len..` the codomain.
///
/// Apex vertices are deliberately **absent**, and that is the whole point: it
/// makes the value comparable across cospans whose apexes differ in size, which
/// is exactly the comparison step (iii) needs.
/// [`Corel::equivalence_classes`] cannot serve here — it interleaves `mid_len`
/// flat indices *between* the two boundaries, so dropping a bubble shifts every
/// codomain index even when the relation is untouched (that shift is itself
/// pinned, as the breaking change it is, by
/// [`compose_shifts_the_flat_index_layout`]).
///
/// Read off the argument's own legs, so it is an independent reading of a
/// result rather than a second copy of the drop-bubbles logic it is used to
/// check.
fn boundary_partition(c: &Cospan<char>) -> Vec<Vec<usize>> {
    let dom_len = c.left_to_middle().len();
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); c.middle().len()];
    for (i, &m) in c.left_to_middle().iter().enumerate() {
        buckets[m].push(i);
    }
    for (k, &m) in c.right_to_middle().iter().enumerate() {
        buckets[m].push(dom_len + k);
    }
    let mut classes: Vec<Vec<usize>> = buckets.into_iter().filter(|b| !b.is_empty()).collect();
    for class in &mut classes {
        class.sort_unstable();
    }
    classes.sort();
    classes
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
/// bubble, would make all of them pass for free. This measures the four
/// quantities the rest of the file leans on — `bubbly`, `total_bubbles`,
/// `bubble_born`, `js_pairs_with_bubbly_pushout` — and asserts non-zero the
/// **three** of them that can independently be zero. `total_bubbles` is
/// reported, not asserted: it is positive whenever `bubbly` is, so an assertion
/// on it would be a restatement rather than a check.
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
/// Checked on every corpus cospan, in the order the body asserts them:
///
/// 1. the image is jointly surjective;
/// 2. `Corel::new` *accepts* it — the codomain claim checked against the
///    validator, not assumed;
/// 3. its canonical form has `scalar_count() == 0`;
/// 4. `domain()` and `codomain()` are untouched. ⚠ **Weak on this corpus**:
///    every apex label is `'a'` and `Cospan::domain` reads leg entries through
///    the apex, so both sides are `vec!['a'; n]` whatever the legs do — this
///    pair cannot fail here. It is kept as the label-level shape check and
///    earned on heterogeneous witnesses in
///    [`quotient_keeps_surviving_labels_in_their_original_order`]; the claim
///    that carries weight here is (5).
/// 5. the equivalence relation induced on `domain ⊔ codomain`
///    ([`boundary_partition`]) is **unchanged** — step (iii) proper, and the
///    claim a leg reindexed onto the wrong survivor breaks;
/// 6. the apex shrinks by exactly the bubble count;
/// 7. `q` is idempotent;
/// 8. and, on the jointly-surjective members only, `q` is the identity.
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
        // What claim (4) is shorthand for, and what this corpus can actually
        // falsify: dropping bubbles must not move which boundary wires share a
        // class.
        assert_eq!(
            boundary_partition(image),
            boundary_partition(&c),
            "quotient of {c:?} moved the partition it induces on domain ⊔ codomain; \
             image {image:?}"
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
    let q = Corel::from_cospan_dropping_bubbles(c.clone());
    assert_eq!(q.as_cospan().middle(), &['p', 'q']);
    assert_eq!(q.as_cospan().left_to_middle(), &[0]);
    assert_eq!(q.as_cospan().right_to_middle(), &[1]);
    // Distinct labels, so *these* domain/codomain assertions can fail — the
    // same pair over the uniform-label corpus in
    // `quotient_is_total_and_lands_in_corel` cannot.
    assert_eq!(q.as_cospan().domain(), c.domain());
    assert_eq!(q.as_cospan().domain(), vec!['p']);
    assert_eq!(q.as_cospan().codomain(), c.codomain());
    assert_eq!(q.as_cospan().codomain(), vec!['q']);

    // Bubble in the middle: apex ['p', 'z', 'q'].
    let c = Cospan::new(vec![0], vec![2], vec!['p', 'z', 'q']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c.clone());
    assert_eq!(q.as_cospan().middle(), &['p', 'q']);
    assert_eq!(q.as_cospan().left_to_middle(), &[0]);
    assert_eq!(q.as_cospan().right_to_middle(), &[1]);
    assert_eq!(q.as_cospan().domain(), c.domain());
    assert_eq!(q.as_cospan().domain(), vec!['p']);
    assert_eq!(q.as_cospan().codomain(), c.codomain());
    assert_eq!(q.as_cospan().codomain(), vec!['q']);

    // Two bubbles at the end, and a leg with a repeated target:
    // apex ['p', 'q', 'z', 'z'], domain hits q then p.
    let c = Cospan::new(vec![1, 0], vec![1], vec!['p', 'q', 'z', 'z']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c.clone());
    assert_eq!(q.as_cospan().middle(), &['p', 'q']);
    assert_eq!(q.as_cospan().left_to_middle(), &[1, 0]);
    assert_eq!(q.as_cospan().right_to_middle(), &[1]);
    // Order-sensitive: `['q', 'p']`, not `['p', 'q']`. A leg reindexed onto
    // the wrong survivor swaps these two entries.
    assert_eq!(q.as_cospan().domain(), c.domain());
    assert_eq!(q.as_cospan().domain(), vec!['q', 'p']);
    assert_eq!(q.as_cospan().codomain(), c.codomain());
    assert_eq!(q.as_cospan().codomain(), vec!['q']);

    // Every vertex a bubble: the image is the empty corelation.
    let c = Cospan::new(vec![], vec![], vec!['z', 'y']).unwrap();
    let q = Corel::from_cospan_dropping_bubbles(c);
    assert!(q.as_cospan().middle().is_empty());
    assert!(q.as_cospan().is_jointly_surjective());
}

// ---------------------------------------------------------------------------
// Half 2 — the composition it repairs
// ---------------------------------------------------------------------------

/// `η ; ε == id_I` in `Corel` — the **extra-special axiom**, and the paper-level
/// payoff of #351 rather than a bug fix that happens to shrink an apex.
///
/// The witness the issue names *is* the unit and the counit:
/// `η : 0 → {m} ← 1` is `Corel::unit('m')`, `ε : 1 → {m} ← 0` is
/// `Corel::counit('m')` (asserted below, so the axiom reading is not a claim
/// about a lookalike pair). Both are jointly surjective, so `Corel::new`
/// accepts both. The pushout glues `η`'s right-leg-only vertex to `ε`'s
/// left-leg-only vertex into a class no *outer* leg reaches, so the raw
/// composite carries one scalar and `Corel::new` would reject it — which is
/// what the comment deleted at #351 asserted could not happen. Step (iii) of
/// F&S 2018 Ex 4.61 fn. 2 drops it, and what is left is `id_I`.
///
/// # Why the axiom, and why it matters here
///
/// Baez–Erbele 2015 (*Categories in Control*, arXiv:1405.6881 §2, p. 11) call a
/// special Frobenius monoid **extra-special** when "the unit followed by the
/// counit is the identity", and identify the free symmetric monoidal category
/// on a commutative extra-special Frobenius monoid as the one whose morphisms
/// `X → Y` are equivalence relations on `X ⊔ Y`, composed "by letting f and g
/// generate an equivalence relation on `X ⊔ Y ⊔ Z` and then restricting this to
/// `X ⊔ Z`" — that description is `Corel` with the composition #351 installed.
/// `Cospan` is deliberately the **special**, not extra-special, theory (#350
/// made `FrobeniusMorphism` match it by deleting the rule that cancelled `η;ε`),
/// and this test is what makes #350's "the extra-special reading remains
/// available as a quotient" a measured fact rather than an aspiration: the two
/// theories now sit on opposite sides of exactly this equation, `Cospan` keeping
/// the bubble as a genuine non-identity and `Corel` cancelling it.
///
/// Ranges over exactly this one pair — one wire type, arities ≤ 1. It is the
/// *existence* witness for the composition break; the universal claim it
/// falsifies is pinned over the whole corpus by
/// [`compose_result_is_always_a_corelation`]. It says nothing about the other
/// extra-special-vs-special consequences, and nothing in-tree proves the
/// Baez–Erbele identification itself — that stays a match of descriptions.
#[test]
fn extra_special_axiom_unit_then_counit_is_id_i() {
    let a = Cospan::<char>::unit('m');
    let b = Cospan::<char>::counit('m');
    assert_eq!(
        a,
        Cospan::<char>::new(vec![], vec![0], vec!['m']).unwrap(),
        "η is not the 0 → {{m}} ← 1 cospan #351 names"
    );
    assert_eq!(
        b,
        Cospan::<char>::new(vec![0], vec![], vec!['m']).unwrap(),
        "ε is not the 1 → {{m}} ← 0 cospan #351 names"
    );
    assert!(a.is_jointly_surjective() && b.is_jointly_surjective());

    // (i) + (ii): the raw pushout. Not a corelation — and in `Cospan`, the
    // *special* theory, this bubble is a genuine non-identity that stays.
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
    assert_ne!(
        raw,
        Cospan::<char>::identity(&Vec::<char>::new()),
        "`Cospan` must keep η;ε apart from id_I — it is the special, not the \
         extra-special, theory (#350)"
    );

    // (iii): Corel's own composition drops it, and the result is id_I on the
    // nose — ε ∘ η = id_I, the extra-special axiom.
    let ca = Corel::<char>::unit('m');
    let cb = Corel::<char>::counit('m');
    let composed = ca.compose(&cb).unwrap();
    assert_eq!(
        composed.as_cospan(),
        Corel::<char>::identity(&Vec::<char>::new()).as_cospan(),
        "η ; ε is not id_I in Corel — the extra-special axiom fails: apex {:?} \
         (raw pushout apex was {:?})",
        composed.as_cospan().middle(),
        raw.middle()
    );
    // Implied by the equality above, kept because they say *what* id_I is here:
    // a legitimate corelation with no scalar left over.
    assert!(composed.as_cospan().is_jointly_surjective());
    assert_eq!(composed.as_cospan().canonical_form().scalar_count(), 0);
}

/// What the #351 break moves on the **public surface** beyond the apex count:
/// the flat-index layout of [`Corel::equivalence_classes`], and every predicate
/// read off it.
///
/// The CHANGELOG entry has to enumerate the blast radius, so the enumeration is
/// pinned here rather than asserted in prose. `equivalence_classes` lays the
/// flat indices out as `0..dom_len` │ `dom_len..dom_len + mid_len` │
/// `dom_len + mid_len..`, so **shrinking the apex shifts every codomain index**
/// — and `merges`, `is_identity_partition` and `equivalence_classes().len()`
/// shift with it. "A dropped class contains no boundary element by definition"
/// is true, but it is a statement about the *partition*, not about its
/// *encoding*, and the encoding is the public surface. The true half is
/// asserted last and on its own, so the two cannot be conflated again.
///
/// `a : 1 → {a,a} ← 2` and `b : 2 → {a,a} ← 1`, both jointly surjective. Their
/// raw pushout is `([0], [0], ['a','a'])` — apex vertex 1 reached by neither
/// outer leg. The pre-#351 value is reconstructed with `Corel::new_unchecked`,
/// which is exactly what `compose` used to wrap the pushout in.
///
/// Ranges over this one pair. It is the enumeration's witness, not a sweep.
#[test]
fn compose_shifts_the_flat_index_layout() {
    /// Sorted flat-index classes, so the comparison is deterministic.
    fn classes(c: &Corel<char>) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = c
            .equivalence_classes()
            .into_iter()
            .map(|class| {
                let mut members: Vec<usize> = class.into_iter().collect();
                members.sort_unstable();
                members
            })
            .collect();
        out.sort();
        out
    }

    let a = Cospan::<char>::new(vec![0], vec![0, 1], vec!['a', 'a']).unwrap();
    let b = Cospan::<char>::new(vec![0, 1], vec![0], vec!['a', 'a']).unwrap();
    assert!(a.is_jointly_surjective() && b.is_jointly_surjective());

    let raw = a.compose(&b).unwrap();
    assert_eq!(
        raw.middle().len(),
        2,
        "raw pushout apex: {:?}",
        raw.middle()
    );
    assert_eq!(
        bubble_count(&raw),
        1,
        "the raw pushout carries no bubble — this witness witnesses nothing"
    );

    // Pre-#351: `compose` was `pushout(…).map(Self::new_unchecked)`.
    let before = Corel::new_unchecked(raw.clone());
    // Post-#351.
    let after = Corel::new(a)
        .unwrap()
        .compose(&Corel::new(b).unwrap())
        .unwrap();
    assert_eq!(after.as_cospan().middle().len(), 1);

    // dom_len 1, mid_len 2 → codomain wire 0 sits at flat index 3.
    assert_eq!(classes(&before), vec![vec![0, 1, 3], vec![2]]);
    assert_eq!(before.equivalence_classes().len(), 2);
    assert!(before.merges(0, 3));
    assert!(!before.is_identity_partition());

    // dom_len 1, mid_len 1 → the same codomain wire now sits at flat index 2.
    assert_eq!(classes(&after), vec![vec![0, 1, 2]]);
    assert_eq!(after.equivalence_classes().len(), 1);
    assert!(!after.merges(0, 3));
    assert!(after.merges(0, 2));
    assert!(after.is_identity_partition());

    // …while the relation on domain ⊔ codomain is the same throughout. That is
    // the claim "nothing else moves" was reaching for, and it is this one only.
    assert_eq!(
        boundary_partition(before.as_cospan()),
        boundary_partition(after.as_cospan()),
        "the restriction moved the boundary relation, not merely its encoding"
    );
}

/// `Corel::compose` returns something `Corel::new` accepts — over every
/// composable pair of corelations the corpus offers, not one.
///
/// This is the universal claim `tests/corel.rs`'s
/// `compose_of_unfold_then_fold_is_jointly_surjective` used to be *named*
/// after while asserting it of a single input pair. Its name is now honest and
/// the universal reading lives here, swept over 4 803 pairs; the count of
/// pairs whose *raw* pushout is not jointly surjective is measured and asserted
/// non-zero, so the sweep cannot pass by never meeting the case.
///
/// It also pins step (iii)'s *scope*: the composite induces the **same
/// partition on `domain ⊔ codomain`** as the raw pushout does
/// ([`boundary_partition`]), so the restriction is genuinely apex-only. The
/// `domain()` / `codomain()` pair asserted alongside it is the label-level
/// shape check and is **weak here** — one wire type makes both sides
/// `vec!['a'; n]` whatever the legs do (see the module docs); the partition
/// assertion is the one that can fail.
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
            // composite induces on domain ⊔ codomain is the raw pushout's,
            // unchanged. This is the assertion that claim needs — `domain()`
            // and `codomain()` below cannot carry it on a one-label corpus.
            assert_eq!(
                boundary_partition(composed.as_cospan()),
                boundary_partition(&raw),
                "compose({a:?}, {b:?}) moved the boundary partition: raw pushout {raw:?}, \
                 composite {:?}",
                composed.as_cospan()
            );
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

/// `Corel::compose` is still associative — `(a ; b) ; c == a ; (b ; c)` — up to
/// apex isomorphism, after #351 changed what composition *is*.
///
/// Everything else in this file pins the quotient, or pins composition against
/// the quotient pairwise. Nothing pinned the **category law of the new
/// composition itself**, and it is exactly the kind of law a restriction step
/// can break: step (iii) discards apex vertices, so an inner composite can lose
/// a vertex that the outer composition would have merged, and whether that
/// changes the answer depends on the order the two compositions run in.
///
/// It does not. Measured over the 14 473 composable triples of corelations the
/// apex ≤ 2 corpus offers: **0** differ up to apex isomorphism. That is the
/// assertion. Two riders keep it from being about nothing:
///
/// - **the corpus reaches the case** — the restriction must actually fire
///   somewhere in the sweep, or associativity would be a statement about the
///   raw pushout wearing this test's name;
/// - **the hedge is real** — the two sides differ *structurally* on some
///   triples (456 measured), so `up to apex isomorphism` is load-bearing and
///   not a weaker claim standing in for one that would have held on the nose.
///   That residual is the same `perform_pushout` apex-numbering artefact
///   [`quotient_functoriality_is_not_structural`] characterises, not a second
///   phenomenon — and the **correlate** of that diagnosis is asserted rather
///   than left in prose: all **456 of 456** carry an identity fast-path
///   asymmetry between the two associations, where the fast path fires for one
///   association's composition and not the other's. Of those, **120** have the
///   asymmetry at the outer composition only (`(a;b);c` vs `a;(b;c)`) and not
///   at the inner one; that split is printed rather than asserted, since it
///   would move with a corpus retune while the 456-of-456 claim is the one the
///   docstring above makes.
///
///   ⚠ **That is a necessary condition, not a proof.** The asymmetry holds on
///   far more triples than mismatch (printed below), so a *second* cause
///   confined to asymmetric triples would satisfy the assertion unchanged. It
///   pins the correlation; the diagnosis rests on
///   [`quotient_functoriality_is_not_structural`]'s analysis, which carries the
///   same hedge.
///
/// **Narrow-pin question.** The claim ranges over associativity of `Corel`
/// composition; the assertions touch triples of jointly-surjective cospans with
/// apex ≤ 2 and boundaries ≤ 2 over a single wire type. It says nothing about
/// heterogeneous labels, wider boundaries, or deeper nestings than three. The
/// associativity **verdict** is decided on `canonical_form`s, never on
/// structural equality, deliberately: a structural verdict would be red on the
/// 456 triples counted below and would pin the artefact rather than the law.
/// *Structural* (`==`) comparison of `as_cospan()` appears only where the
/// residual is counted; the verdict compares its `canonical_form`. (`as_cospan()`
/// itself is used throughout — it is the only way to reach the wrapped value —
/// so "never `as_cospan()`" would be, and in one earlier revision was, false.)
///
/// **Context, printed rather than asserted:** the pre-#351 composition (the raw
/// pushout, no restriction) has 0 mismatches up to apex isomorphism and 512
/// structural ones on this same corpus, so #351 left associativity intact and
/// slightly narrowed the structural gap. Not asserted because it is a property
/// of the retired composition, not of this one.
#[test]
fn new_composition_is_associative_up_to_apex_isomorphism() {
    /// Does `perform_pushout`'s identity fast path fire for this composition?
    /// Same predicate [`quotient_functoriality_is_not_structural`] uses.
    ///
    /// ⚠ **Locally recomputed, not observed.** The crate's `leg_is_identity` is
    /// private, so this is a copy of it; a rewrite of the crate's version would
    /// not redden anything here, and the assertion below would go on reporting
    /// "an identity fast-path asymmetry" about a predicate the crate no longer
    /// uses. `cospan.rs` records four writers dropping the `len() == apex`
    /// conjunct from this exact predicate, so the divergence is a live hazard
    /// rather than a theoretical one.
    fn fast_path(left: &Cospan<char>, right: &Cospan<char>) -> bool {
        fn leg_is_identity(leg: &[usize], apex: usize) -> bool {
            leg.len() == apex && leg.iter().enumerate().all(|(i, &v)| v == i)
        }
        leg_is_identity(left.right_to_middle(), left.middle().len())
            || leg_is_identity(right.left_to_middle(), right.middle().len())
    }

    let corels: Vec<Cospan<char>> = corpus_up_to(2)
        .into_iter()
        .filter(Cospan::is_jointly_surjective)
        .collect();

    let mut triples = 0usize;
    let mut iso_mismatches = 0usize;
    let mut structural_mismatches = 0usize;
    let mut triples_where_the_restriction_fired = 0usize;
    let mut structural_with_a_fast_path_asymmetry = 0usize;
    let mut structural_with_an_outer_asymmetry_only = 0usize;
    // The denominator that makes 456-of-456 meaningful: how often the
    // asymmetry holds at all. Without it a reader cannot tell a discriminating
    // predicate from a near-universal one.
    let mut triples_with_a_fast_path_asymmetry = 0usize;

    for a in &corels {
        for b in &corels {
            if a.right_to_middle().len() != b.left_to_middle().len() {
                continue;
            }
            for c in &corels {
                if b.right_to_middle().len() != c.left_to_middle().len() {
                    continue;
                }
                triples += 1;

                let ca = Corel::new(a.clone()).expect("invariant: filtered to jointly surjective");
                let cb = Corel::new(b.clone()).expect("invariant: filtered to jointly surjective");
                let cc = Corel::new(c.clone()).expect("invariant: filtered to jointly surjective");

                let ab = ca.compose(&cb).expect("invariant: arities matched above");
                let bc = cb.compose(&cc).expect("invariant: arities matched above");
                let lhs = ab
                    .compose(&cc)
                    .expect("invariant: composition preserves the boundary");
                let rhs = ca
                    .compose(&bc)
                    .expect("invariant: composition preserves the boundary");

                // Did step (iii) actually discard anything anywhere in this
                // triple? Compare each composite's apex against the raw pushout
                // it was restricted from.
                let raw_ab = a.compose(b).expect("invariant: arities matched above");
                let raw_bc = b.compose(c).expect("invariant: arities matched above");
                let raw_lhs = raw_ab
                    .compose(c)
                    .expect("invariant: composition preserves the boundary");
                let raw_rhs = a
                    .compose(&raw_bc)
                    .expect("invariant: composition preserves the boundary");
                if raw_ab.middle().len() != ab.as_cospan().middle().len()
                    || raw_bc.middle().len() != bc.as_cospan().middle().len()
                    || raw_lhs.middle().len() != lhs.as_cospan().middle().len()
                    || raw_rhs.middle().len() != rhs.as_cospan().middle().len()
                {
                    triples_where_the_restriction_fired += 1;
                }

                if lhs.as_cospan().canonical_form() != rhs.as_cospan().canonical_form() {
                    iso_mismatches += 1;
                }
                if fast_path(ab.as_cospan(), c) != fast_path(a, bc.as_cospan())
                    || fast_path(a, b) != fast_path(b, c)
                {
                    triples_with_a_fast_path_asymmetry += 1;
                }

                if lhs.as_cospan() != rhs.as_cospan() {
                    structural_mismatches += 1;
                    // The two associations run different compositions: LHS
                    // does (a;b) then ·;c, RHS does (b;c) then a;·. Where the
                    // identity fast path fires for one and not the other, the
                    // composite apex is numbered by a different rule — the
                    // artefact `quotient_functoriality_is_not_structural`
                    // characterises for pairs.
                    let outer = fast_path(ab.as_cospan(), c) != fast_path(a, bc.as_cospan());
                    let inner = fast_path(a, b) != fast_path(b, c);
                    if outer || inner {
                        structural_with_a_fast_path_asymmetry += 1;
                    }
                    if outer && !inner {
                        structural_with_an_outer_asymmetry_only += 1;
                    }
                }
            }
        }
    }

    assert_eq!(
        iso_mismatches, 0,
        "{iso_mismatches} of {triples} composable triples break associativity up to apex \
         isomorphism — 0 was measured when this pin was written"
    );
    assert!(
        triples_where_the_restriction_fired > 0,
        "step (iii) never fired across {triples} triples, so this sweep is about the raw pushout \
         and not about #351's composition at all"
    );
    assert!(
        structural_mismatches > 0,
        "the two sides agree structurally on all {triples} triples, so `up to apex isomorphism` \
         is no longer load-bearing here and this pin should be strengthened to `==` on the \
         `Cospan` — 456 structural mismatches were measured when it was written"
    );
    assert_eq!(
        structural_with_a_fast_path_asymmetry,
        structural_mismatches,
        "{} of {structural_mismatches} structural mismatches are NOT explained by an identity \
         fast-path asymmetry between the two associations — the residual has a second cause, and \
         the docstring's claim that it is the same artefact \
         `quotient_functoriality_is_not_structural` characterises no longer holds",
        structural_mismatches - structural_with_a_fast_path_asymmetry
    );
    println!(
        "{triples} composable triples: {iso_mismatches} differ up to apex isomorphism, \
         {structural_mismatches} differ structurally ({structural_with_a_fast_path_asymmetry} \
         with a fast-path asymmetry, {structural_with_an_outer_asymmetry_only} at the outer \
         composition only), {triples_where_the_restriction_fired} have step (iii) firing \
         somewhere, and {triples_with_a_fast_path_asymmetry} of {triples} carry the asymmetry at \
         all — so it is necessary for a structural mismatch and far from sufficient"
    );
}
