//! Serde round-trip tests for the persistence surface — terms (#81), rewrite
//! traces ([#249]), and the content-equality key ([#255]).
//!
//! Runs only under `--features serde`; the default build compiles neither the
//! derives nor this file.
//!
//! The trace and key tests pin the *trust boundary* each type documents, not
//! merely that the derives compile:
//!
//! - a `RewriteStep` is **checked, not trusted** — `replay` re-derives every
//!   step against the running content, so a tampered document is an `Err`;
//! - a `RewriteOutcome`'s cost/count fields have **no validator**, which is
//!   asserted in the honest direction: a forged number survives deserialization;
//! - a `RewriteRule` is rebuilt through `RewriteRule::new` from its persisted
//!   `(lhs, rhs)` pair, because it carries no derives of its own;
//! - a round-tripped `ContentKey` must still *work as a key*: equal to a fresh
//!   key of a `content_eq`-equal content written differently, and separating one
//!   that is genuinely different.
//!
//! [#249]: https://github.com/sustia-llc/catgraph/issues/249
//! [#255]: https://github.com/sustia-llc/catgraph/issues/255
#![cfg(feature = "serde")]

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::content::{
    ContentKey, canonical_key, content_eq, content_of, content_of_colored,
};
use catgraph_applied::prop::presentation::rewrite::{
    RewriteOutcome, RewriteRule, RewriteStep, cost_of, optimize, replay,
};
use catgraph_applied::prop::presentation::{NormalizeEngine, Presentation};
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;

type G = SfgGenerator<i64>;

/// A non-trivial `1 → 1` signal-flow term: `copy ; (scalar(3) ⊗ id) ; add`,
/// exercising every `PropExpr` variant (`Generator`, `Identity`, `Compose`,
/// `Tensor`) and a generator carrying an `R` payload (`Scalar`).
fn sample_term() -> PropExpr<G> {
    let copy = Free::generator(SfgGenerator::Copy); // 1 → 2
    let scaled = Free::tensor(
        Free::generator(SfgGenerator::Scalar(3_i64)), // 1 → 1
        Free::<G>::identity(1),                       // 1 → 1
    ); // 2 → 2
    let add = Free::generator(SfgGenerator::Add); // 2 → 1
    let left = Free::compose(copy, scaled).expect("copy(1→2) ; (2→2)");
    Free::compose(left, add).expect("(1→2) ; add(2→1)")
}

#[test]
fn propexpr_json_round_trip_is_identity() {
    let term = sample_term();
    let json = serde_json::to_string(&term).expect("serialize");
    let back: PropExpr<G> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(term, back, "PropExpr must survive a JSON round-trip");
}

#[test]
fn normalize_engine_round_trips() {
    for engine in [
        NormalizeEngine::Structural,
        NormalizeEngine::CongruenceClosure,
    ] {
        let json = serde_json::to_string(&engine).expect("serialize");
        let back: NormalizeEngine = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(engine, back);
    }
}

#[test]
fn presentation_round_trips_and_still_decides() {
    // A presentation with one equation: copy ; add = id(1).
    let mut pres = Presentation::<G>::new();
    let lhs = Free::compose(
        Free::generator(SfgGenerator::Copy),
        Free::generator(SfgGenerator::Add),
    )
    .expect("copy(1→2) ; add(2→1)");
    let rhs = Free::<G>::identity(1);
    pres.add_equation(lhs.clone(), rhs.clone())
        .expect("arities match");

    // Serialize → deserialize → serialize: the representation is stable.
    let json = serde_json::to_string(&pres).expect("serialize");
    let back: Presentation<G> = serde_json::from_str(&json).expect("deserialize");
    let json2 = serde_json::to_string(&back).expect("re-serialize");
    assert_eq!(json, json2, "Presentation JSON must be round-trip stable");

    // The deserialized presentation still decides the equation it carries.
    assert_eq!(
        back.eq_mod(&lhs, &rhs).expect("eq_mod runs"),
        Some(true),
        "a round-tripped presentation must still prove its own equation",
    );
}

// ---- #249: rewrite traces ----------------------------------------------------

/// Pin `width` monochromatic wires onto `expr` (`SfgGenerator`'s `Color` is `()`).
fn wired(width: usize, expr: PropExpr<G>) -> ColoredExpr<G> {
    ColoredExpr::new(vec![(); width], expr).expect("monochromatic, so only arities can fail")
}

fn scalar(r: i64) -> PropExpr<G> {
    Free::generator(SfgGenerator::Scalar(r))
}

/// `scalar(3) ; scalar(2) ⇒ scalar(6)`: parallel `1 → 1` sides, a two-hyperedge
/// left-hand side, a mono left interface. The `(lhs, rhs)` pair is what a store
/// persists, since `RewriteRule` itself carries no derives.
fn fusion_pair() -> (ColoredExpr<G>, ColoredExpr<G>) {
    let lhs = wired(
        1,
        Free::compose(scalar(3), scalar(2)).expect("1 → 1 twice over"),
    );
    let rhs = wired(1, scalar(6));
    (lhs, rhs)
}

/// `scalar(3) ; scalar(2) ; scalar(2)` — three hyperedges, exactly one match of
/// the fusion rule (the lone `scalar(3)` and the `scalar(2)` it feeds), so the
/// trace is a single step and the leftover third edge is available as a
/// tampering target that is *labelled right and positioned wrong*.
fn fusion_start() -> ColoredExpr<G> {
    wired(
        1,
        Free::compose(
            Free::compose(scalar(3), scalar(2)).expect("1 → 1"),
            scalar(2),
        )
        .expect("1 → 1"),
    )
}

fn fusion_setup() -> (ColoredExpr<G>, Vec<RewriteRule<G>>, RewriteOutcome<G>) {
    let (lhs, rhs) = fusion_pair();
    let rules =
        vec![RewriteRule::new(lhs, rhs).expect("parallel, mono-interfaced, two hyperedges")];
    let start = fusion_start();
    let outcome = optimize(&start, &rules, 64, |_| 1).expect("well-formed start");
    (start, rules, outcome)
}

#[test]
fn rewrite_outcome_json_round_trip_is_identity() {
    let (_, _, outcome) = fusion_setup();

    // The fixture is the one the later tests read, so pin what it *is* first —
    // otherwise a round trip of nothing would pass every assertion below.
    assert_eq!(outcome.initial_cost(), 3);
    assert_eq!(outcome.best_cost(), 2);
    assert_eq!(outcome.steps().len(), 1);
    assert_eq!(outcome.steps()[0].rule(), 0);
    assert_eq!(outcome.steps()[0].matched_edges().len(), 2);
    assert!(!outcome.fuel_exhausted());
    assert_eq!(outcome.states_explored(), 2);

    let json = serde_json::to_string(&outcome).expect("serialize");
    let back: RewriteOutcome<G> = serde_json::from_str(&json).expect("deserialize");

    // Every accessor, not a sample of them.
    assert_eq!(back.best(), outcome.best());
    assert_eq!(back.initial_cost(), outcome.initial_cost());
    assert_eq!(back.best_cost(), outcome.best_cost());
    assert_eq!(back.steps(), outcome.steps());
    assert_eq!(back.fuel_exhausted(), outcome.fuel_exhausted());
    assert_eq!(back.states_explored(), outcome.states_explored());

    // Re-serializing the deserialized value reproduces the document, so the
    // representation is stable and not merely accessor-equal.
    let json2 = serde_json::to_string(&back).expect("re-serialize");
    assert_eq!(json, json2, "RewriteOutcome JSON must be round-trip stable");
}

/// The end-to-end consumer story: persist the trace *and* the rule pairs,
/// reload both, and replay to the same morphism the in-process trace reaches.
///
/// The rules go through `RewriteRule::new` on the way back, which is the whole
/// reason that type carries no derives — its four conditions are checked once
/// and never re-checked, so the rebuild is where they get re-established.
#[test]
fn a_persisted_trace_replays_to_the_same_morphism() {
    let (start, rules, outcome) = fusion_setup();
    let in_process = replay(&start, &rules, outcome.steps()).expect("the trace is legal");

    // ---- persist: the start, the trace, and the rules as `(lhs, rhs)` pairs.
    let start_doc = serde_json::to_string(&start).expect("serialize the start");
    let trace_doc = serde_json::to_string(outcome.steps()).expect("serialize the trace");
    let rules_doc = serde_json::to_string(&[fusion_pair()]).expect("serialize the rule pairs");

    // ---- load, in a scope that shares nothing with the values above.
    let start_back: ColoredExpr<G> = serde_json::from_str(&start_doc).expect("load the start");
    let steps_back: Vec<RewriteStep> = serde_json::from_str(&trace_doc).expect("load the trace");
    let pairs_back: Vec<(ColoredExpr<G>, ColoredExpr<G>)> =
        serde_json::from_str(&rules_doc).expect("load the rule pairs");
    let rules_back: Vec<RewriteRule<G>> = pairs_back
        .into_iter()
        .map(|(lhs, rhs)| RewriteRule::new(lhs, rhs).expect("a rule this crate built and stored"))
        .collect();

    let replayed =
        replay(&start_back, &rules_back, &steps_back).expect("the stored trace is legal");

    assert!(replayed.eq_colored(&in_process));
    assert_eq!(
        canonical_key(&content_of_colored(&replayed)),
        canonical_key(&content_of_colored(outcome.best())),
        "a trace that crossed a process boundary must reach the recorded endpoint",
    );
    // And the cost claim is re-derivable rather than taken from the document.
    assert_eq!(
        cost_of(&content_of_colored(&replayed), |_| 1),
        outcome.best_cost()
    );
}

/// `RewriteStep`'s derive is safe **because** `replay` re-validates: a document
/// that names a rule out of range, permutes the match, or slides it onto a
/// same-labelled edge elsewhere is rejected, not applied at the wrong place.
///
/// Tampering happens on the JSON — the fields are private, so this is exactly
/// the untrusted-document path the type's serde section describes.
#[test]
fn a_tampered_trace_is_rejected_rather_than_replayed() {
    let (start, rules, outcome) = fusion_setup();
    let honest: Vec<usize> = outcome.steps()[0].matched_edges().to_vec();
    assert_eq!(honest.len(), 2);
    // The one hyperedge the match does not occupy: same label as `honest[1]`
    // (both are `scalar(2)`), wrong place.
    let elsewhere = (0..3)
        .find(|e| !honest.contains(e))
        .expect("three hyperedges, two matched");

    let rejected = |what: &str, doc: String| {
        let steps: Vec<RewriteStep> = serde_json::from_str(&doc)
            .unwrap_or_else(|e| panic!("{what}: the forged document must still deserialize: {e}"));
        match replay(&start, &rules, &steps) {
            Err(CatgraphError::Presentation { message }) => message,
            other => panic!("{what}: a tampered trace must not replay, got {other:?}"),
        }
    };

    // (a) a rule index outside the slice.
    let message = rejected(
        "out-of-range rule",
        format!(r#"[{{"rule":7,"matched_edges":{honest:?}}}]"#),
    );
    assert!(message.contains("names rule 7 of 1"), "got: {message}");

    // (b) the match permuted: `lhs` edge 0 is `scalar(3)`, so pointing it at the
    //     `scalar(2)` image is a label mismatch.
    let swapped = [honest[1], honest[0]];
    let message = rejected(
        "permuted match",
        format!(r#"[{{"rule":0,"matched_edges":{swapped:?}}}]"#),
    );
    assert!(message.contains("convex match"), "got: {message}");

    // (c) the match slid onto the *other* `scalar(2)`: right labels, but the
    //     two hyperedges are not adjacent, so the assignment is no match at all.
    let slid = [honest[0], elsewhere];
    let message = rejected(
        "match slid to a same-labelled edge",
        format!(r#"[{{"rule":0,"matched_edges":{slid:?}}}]"#),
    );
    assert!(message.contains("convex match"), "got: {message}");

    // The honest document, by contrast, replays.
    let honest_doc = format!(r#"[{{"rule":0,"matched_edges":{honest:?}}}]"#);
    let steps: Vec<RewriteStep> = serde_json::from_str(&honest_doc).expect("deserialize");
    assert_eq!(steps, outcome.steps());
    assert!(replay(&start, &rules, &steps).is_ok());
}

/// The other half of the honesty claim, asserted rather than only documented:
/// `RewriteOutcome`'s cost and count fields have **no validator anywhere**, so a
/// forged document carries forged numbers straight through. Nothing in the crate
/// rejects them; a consumer that needs them right recomputes them.
#[test]
fn an_outcome_report_field_is_not_validated_by_anything() {
    let (start, rules, outcome) = fusion_setup();
    let doc = serde_json::to_string(&outcome).expect("serialize");

    // `best_cost` above `initial_cost` — a shape `optimize` cannot produce, since
    // the start is itself a candidate.
    let forged = doc
        .replace("\"best_cost\":2", "\"best_cost\":99")
        .replace("\"states_explored\":2", "\"states_explored\":0")
        .replace("\"fuel_exhausted\":false", "\"fuel_exhausted\":true");
    assert_ne!(forged, doc, "the field names must have actually matched");

    let back: RewriteOutcome<G> = serde_json::from_str(&forged).expect("no validator rejects it");
    assert_eq!(back.best_cost(), 99);
    assert!(back.best_cost() > back.initial_cost());
    assert_eq!(back.states_explored(), 0);
    assert!(back.fuel_exhausted());

    // The trace it carries is the checkable part, and it still checks — which is
    // the exact asymmetry the type documents.
    let replayed = replay(&start, &rules, back.steps()).expect("the trace is still legal");
    assert_eq!(
        cost_of(&content_of_colored(&replayed), |_| 1),
        outcome.best_cost(),
        "recomputing is the remedy: the real cost is unaffected by the forgery",
    );
}

// ---- #255: the content-equality key ------------------------------------------

fn prim(g: SfgGenerator<i64>) -> PropExpr<G> {
    Free::generator(g)
}

fn hash_of(key: &ContentKey<G>) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// A round-tripped `ContentKey` must survive as a *key*, not merely as a value:
/// equal and hash-equal to itself, still equal to a freshly computed key of a
/// `content_eq`-equal content written differently, and still separating one that
/// is genuinely different.
///
/// The equal-by-a-different-writing pair is `content_equality.rs`'s
/// `trapped_closed_block` witness (§4.6(c)), chosen over the shorter candidates
/// for two reasons, both checked rather than assumed:
///
/// - its two writings **disagree on raw indices**, so the anchored renumbering
///   behind `canonical_key` is doing real work. Pairs whose raw index spaces
///   already agree — `dead_braid_prefix`, `eta_layer_slack` — assert nothing
///   about canonicalization here: reseeding the numbering in raw index order
///   leaves both of those equal and separates this one.
/// - its canonical form has a **non-empty closed part**, so the round trip
///   exercises the `ClosedBlock` half of the key rather than only the
///   boundary-attached half.
///
/// The foil differs *only inside that closed component* (a `Scalar` spliced into
/// the bubble), which is the half a round trip is most likely to drop silently.
#[test]
fn content_key_round_trips_and_still_keys() {
    let copy = || prim(SfgGenerator::Copy);
    let add = || prim(SfgGenerator::Add);
    let discard = || prim(SfgGenerator::Discard);
    let zero = || prim(SfgGenerator::Zero);
    let id1 = || Free::<G>::identity(1);
    let seq = |f, g| Free::compose(f, g).expect("the interfaces meet");

    // `copy ; ((id ⊗ zero) ⊗ id) ; ((id ⊗ discard) ⊗ id) ; add` — the bubble is
    // threaded through the middle of a `1 → 1` term.
    let threaded = seq(
        seq(
            seq(copy(), Free::tensor(Free::tensor(id1(), zero()), id1())),
            Free::tensor(Free::tensor(id1(), discard()), id1()),
        ),
        add(),
    );
    // `(zero ; discard) ⊗ (copy ; add)` — the same content, with the bubble
    // written as a free-standing closed component.
    let split = Free::tensor(seq(zero(), discard()), seq(copy(), add()));
    // The same shape, with a `scalar(7)` spliced into the *closed* component.
    let scaled_bubble = Free::tensor(seq(seq(zero(), scalar(7)), discard()), seq(copy(), add()));

    let (threaded, split, scaled_bubble) = (
        content_of(&threaded),
        content_of(&split),
        content_of(&scaled_bubble),
    );
    assert!(
        content_eq(&threaded, &split),
        "the fixture pair must be equal"
    );
    assert!(
        !content_eq(&threaded, &scaled_bubble),
        "the foil must be different"
    );

    let key = canonical_key(&threaded);
    // The fixture must actually reach the closed-block machinery, or the
    // `ClosedBlock` derive would be untested by this test.
    assert!(
        format!("{key:?}").contains("ClosedBlock"),
        "the fixture must have a closed component: {key:?}"
    );

    let json = serde_json::to_string(&key).expect("serialize");
    let back: ContentKey<G> = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back, key, "ContentKey must survive a JSON round-trip");
    assert_eq!(hash_of(&back), hash_of(&key), "and hash to the same bucket");
    assert_eq!(
        serde_json::to_string(&back).expect("re-serialize"),
        json,
        "ContentKey JSON must be round-trip stable"
    );

    // Still a key: it matches an equal content written differently, and refuses
    // one that differs only inside the closed component.
    assert_eq!(back, canonical_key(&split));
    assert_ne!(back, canonical_key(&scaled_bubble));

    // The `Eq + Hash` contract end to end — which is the whole contract, since
    // the type is deliberately not `Ord`.
    let mut table: HashMap<ContentKey<G>, &str> = HashMap::new();
    table.insert(back, "the trapped-bubble class");
    assert_eq!(
        table.get(&canonical_key(&split)),
        Some(&"the trapped-bubble class")
    );
    assert_eq!(table.get(&canonical_key(&scaled_bubble)), None);
}

// ---- the two color regimes a key has to survive ------------------------------

/// `F : A → B`, `H : B → A` — a two-letter `Λ`, so the key's color slots carry
/// real letters rather than `()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Wire {
    A,
    B,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Two {
    F,
    H,
}

impl catgraph_applied::prop::PropSignature for Two {
    type Color = Wire;

    fn source_word(&self) -> std::borrow::Cow<'_, [Wire]> {
        std::borrow::Cow::Borrowed(match self {
            Two::F => &[Wire::A],
            Two::H => &[Wire::B],
        })
    }

    fn target_word(&self) -> std::borrow::Cow<'_, [Wire]> {
        std::borrow::Cow::Borrowed(match self {
            Two::F => &[Wire::B],
            Two::H => &[Wire::A],
        })
    }
}

/// A key's `colors` field is `Vec<Option<Color>>`, and `Option` is **lossy in
/// JSON** whenever `Color` itself serializes as `null` — which is precisely the
/// monochromatic `Color = ()` case that most of this crate uses. `Some(())` and
/// `None` both write `null`; a plain derive brings the key home with every color
/// erased, equal to no freshly computed key at all. Both regimes are pinned
/// here, because the failing one is the *default* one.
#[test]
fn a_key_round_trips_in_both_color_regimes() {
    // Monochromatic: every node typed `Some(())`, plus — via a short source word
    // across `ColoredExpr`'s serde boundary — a genuinely `None` slot to prove
    // the two tags stay apart rather than both collapsing to "typed".
    let mono = content_of_colored(&wired(1, prim(SfgGenerator::Copy)));
    assert_eq!(mono.node_colors(), &[Some(()), Some(()), Some(())]);
    let mono_key = canonical_key(&mono);
    let back: ContentKey<G> =
        serde_json::from_str(&serde_json::to_string(&mono_key).expect("serialize"))
            .expect("deserialize");
    assert_eq!(back, mono_key, "`Some(())` must not come back as `None`");

    let untyped: ColoredExpr<G> =
        serde_json::from_str(r#"{"source_word":[],"target_word":[],"expr":{"Identity":1}}"#)
            .expect("the serde path does not re-run `check`");
    let untyped = content_of_colored(&untyped);
    assert_eq!(untyped.node_colors(), &[None], "an uncovered coordinate");
    let untyped_key = canonical_key(&untyped);
    let back: ContentKey<G> =
        serde_json::from_str(&serde_json::to_string(&untyped_key).expect("serialize"))
            .expect("deserialize");
    assert_eq!(back, untyped_key);
    assert_ne!(
        untyped_key,
        canonical_key(&content_of_colored(&wired(1, PropExpr::Identity(1)))),
        "typed and untyped keys must differ — the distinction serde has to carry",
    );

    // Λ-colored: the slots carry real letters, and the key still separates two
    // contents that differ only in them.
    let f = catgraph_applied::prop::Free::<Two>::generator(Two::F);
    let h = catgraph_applied::prop::Free::<Two>::generator(Two::H);
    let key_f = canonical_key(&content_of(&f));
    let back: ContentKey<Two> =
        serde_json::from_str(&serde_json::to_string(&key_f).expect("serialize"))
            .expect("deserialize");
    assert_eq!(back, key_f);
    assert_eq!(hash_of_two(&back), hash_of_two(&key_f));
    assert_ne!(back, canonical_key(&content_of(&h)));
}

fn hash_of_two(key: &ContentKey<Two>) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}
