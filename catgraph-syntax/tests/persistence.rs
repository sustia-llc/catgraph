//! Textual-persistence round-trip (integration): the pre-serde answer to #73 /
//! #81 — persist a presentation and standalone terms to text, reload them, and
//! assert the reloaded artifacts are **decision-procedure-equivalent**, not
//! merely byte-equal.
//!
//! # What the text format carries (the engine-config caveat)
//!
//! A presentation file carries **only the equation list `E`** — one `lhs = rhs`
//! per line (Seven Sketches Def 5.33). It does **not** carry the
//! [`NormalizeEngine`] choice or the rewrite/congruence depth bound:
//! [`parse_presentation`] always returns a **default-configured**
//! [`Presentation`] (`CongruenceClosure`, depth 32). So a round-trip is faithful
//! on the *equations* and on *decision behaviour over those equations* — it is
//! **not** expected to preserve engine config. Every assertion here compares
//! [`equations()`](Presentation::equations) and `eq_mod` decisions, never the
//! engine. This is exactly why serde persistence of the engine is its own issue
//! (#81): the text surface is deliberately equation-only.
//!
//! # What the three tests verify (and what they deliberately do not)
//!
//! - `presentation_text_round_trip_preserves_equations_and_decisions` builds the
//!   original with a **non-default** engine to make the caveat concrete, then
//!   checks the equation list survives and that *both* engines prove the axioms.
//!   Because the two sides run **different** engines (Structural vs
//!   CongruenceClosure), this test is NOT a round-trip decision-equivalence check
//!   — it only witnesses that neither engine loses the axioms; the engine
//!   mismatch is the point.
//! - `round_trip_preserves_decisions_on_a_derived_consequence` is the genuine
//!   round-trip decision-equivalence check: **same engine** (default
//!   CongruenceClosure) on both sides, asserting a *derived* consequence — one
//!   that provably needs the axiom, not an SMC tautology — decides identically
//!   before and after the round-trip.
//! - `standalone_terms_round_trip_and_eval_agrees` covers the term (not
//!   presentation) surface via `eval` agreement.
//!
//! # The #15 boundary
//!
//! `eq_mod` is **sound but syntactically incomplete**: `Ok(Some(true))` is a
//! proof of equality, but `None` / `Ok(Some(false))` is *not* a disproof.
//! Every equality assertion below relies only on the `Some(true)` direction.
//! Complete decisions come only via `eq_mod_functorial` +
//! `MatrixNFFunctor` (Thm 5.60), which this persistence story does not touch.

mod common;

use catgraph_applied::prop::presentation::{NormalizeEngine, Presentation};
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::eval::eval;
use catgraph_syntax::text::{parse, parse_presentation, print, print_presentation};
use common::{Sig, g, sfg_model};

/// Two arity-valid equations over the demo [`Sig`] (the `presentation.rs`
/// fixture shape): `copy ; add = id(1)` (both `1 → 1`) and
/// `braid(1,1) = id(2)` (both `2 → 2`).
///
/// These are **formal relations of the presented quotient prop**, not
/// SfgModel-valid identities: under the [`sfg_model`] used by
/// `standalone_terms_round_trip_and_eval_agrees` below, `copy ; add` doubles and
/// `braid(1,1)` swaps, so neither holds *semantically*. A presentation is
/// model-free — `add_equation` only arity-checks — so declaring them here is
/// exactly the point: we exercise persistence of a quotient's defining relations,
/// independent of any model that might satisfy or refute them.
fn demo_equations() -> Vec<(PropExpr<Sig>, PropExpr<Sig>)> {
    vec![
        (
            Free::compose(g(Sig::Copy), g(Sig::Add)).expect("copy:1→2 ; add:2→1"),
            Free::<Sig>::identity(1),
        ),
        (Free::<Sig>::braid(1, 1), Free::<Sig>::identity(2)),
    ]
}

#[test]
fn presentation_text_round_trip_preserves_equations_and_decisions() {
    // Build the ORIGINAL with a deliberately NON-default engine, to make the
    // caveat concrete: the engine choice will *not* survive the text round-trip,
    // yet the decision behaviour over the axioms will.
    let mut original = Presentation::<Sig>::with_engine(NormalizeEngine::Structural);
    for (lhs, rhs) in demo_equations() {
        original
            .add_equation(lhs, rhs)
            .expect("demo equations are arity-matched by construction");
    }

    // Persist to text, then reload. `parse_presentation` returns a
    // default-configured (CongruenceClosure) presentation — engine config does
    // NOT round-trip, by design.
    let text = print_presentation(&original);
    let reloaded = parse_presentation::<Sig>(&text).expect("printed presentation reparses");

    // (1) The equation list E survives verbatim.
    assert_eq!(
        reloaded.equations(),
        original.equations(),
        "equation list E is preserved across the text round-trip"
    );

    // (2) Both engines prove the axioms. Each axiom is provable by its own
    // presentation, so eq_mod returns Some(true) for BOTH the original
    // (Structural engine) and the reloaded (CongruenceClosure engine). NOTE this
    // is a weaker statement than round-trip decision equivalence: the two sides
    // run different engines, and they would legitimately DIVERGE on a derived
    // consequence (Structural can return None where CC returns Some(true) — the
    // #15 gap). True same-engine round-trip equivalence is checked in the next
    // test. #15: we assert only the Some(true) direction throughout.
    for (lhs, rhs) in original.equations() {
        assert_eq!(
            original
                .eq_mod(lhs, rhs)
                .expect("eq_mod is infallible here"),
            Some(true),
            "original proves its own axiom"
        );
        assert_eq!(
            reloaded
                .eq_mod(lhs, rhs)
                .expect("eq_mod is infallible here"),
            Some(true),
            "reloaded proves the same axiom after the round-trip"
        );
    }
}

#[test]
fn round_trip_preserves_decisions_on_a_derived_consequence() {
    // The genuine round-trip decision-equivalence check: DEFAULT engine
    // (CongruenceClosure) on both sides, so before/after is a true apples-to-apples
    // round-trip (default → text → default), and we can safely test a *derived*
    // consequence rather than only the axioms.
    let mut original = Presentation::<Sig>::new(); // CongruenceClosure, depth 32
    for (lhs, rhs) in demo_equations() {
        original
            .add_equation(lhs, rhs)
            .expect("demo equations are arity-matched by construction");
    }
    let reloaded = parse_presentation::<Sig>(&print_presentation(&original))
        .expect("printed presentation reparses");

    // A consequence that is NOT an axiom and NOT an SMC tautology: `copy ; add`
    // twice in a row. It reduces to `id(1)` ONLY via the `copy ; add = id(1)`
    // axiom — verified out-of-band that an axiom-free presentation decides it
    // `Some(false)`, so a `Some(true)` here proves the axiom survived the
    // round-trip functionally, not just syntactically.
    let copy_add = || Free::compose(g(Sig::Copy), g(Sig::Add)).expect("copy:1→2 ; add:2→1");
    let consequence =
        Free::compose(copy_add(), copy_add()).expect("(copy;add):1→1 ; (copy;add):1→1");
    let id1 = Free::<Sig>::identity(1);

    let before = original
        .eq_mod(&consequence, &id1)
        .expect("eq_mod is infallible here");
    let after = reloaded
        .eq_mod(&consequence, &id1)
        .expect("eq_mod is infallible here");

    assert_eq!(
        before,
        Some(true),
        "original (CC) proves the derived consequence"
    );
    assert_eq!(
        after, before,
        "reloaded presentation decides the derived consequence identically — a \
         true same-engine round-trip preserves decisions, not just the axiom list"
    );
}

#[test]
fn standalone_terms_round_trip_and_eval_agrees() {
    // Standalone SFG terms carry runtime semantics, so we can assert eval
    // agreement across the text round-trip (structural equality already implies
    // it, but exercising eval is the persistence-relevant guarantee: a persisted
    // circuit computes the same thing after reload).
    let model = sfg_model();
    // (source term string, an input vector of the right length).
    let cases: [(&str, Vec<i64>); 3] = [
        ("copy ; add", vec![4]),                       // 1 → 1  doubles
        ("copy ; scalar:2 * scalar:3 ; add", vec![6]), // 1 → 1  x ↦ 5x
        ("braid(1,1)", vec![10, 20]),                  // 2 → 2  swaps
    ];
    for (src, input) in cases {
        let original = parse::<SfgGenerator<i64>>(src).expect("term parses");
        let text = print(&original);
        let reloaded = parse::<SfgGenerator<i64>>(&text).expect("printed term reparses");

        // Structural round-trip: same syntax tree (the printer never normalizes).
        assert_eq!(reloaded, original, "structural round-trip for `{src}`");

        // Eval agreement across the round-trip.
        let out_original = eval(&original, &model, input.clone());
        let out_reloaded = eval(&reloaded, &model, input);
        assert_eq!(
            out_original, out_reloaded,
            "persisted term `{src}` evaluates identically after reload"
        );
    }
}
