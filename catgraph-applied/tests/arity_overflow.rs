//! Overflowing arities across the deeper applied passes
//! ([#196](https://github.com/sustia-llc/catgraph/issues/196)).
//!
//! `PropExpr`'s variants are public and raw construction is documented-legal, so
//! `Braid(usize::MAX, 1)` is a value a caller can hand to any of them. #180
//! hardened the surface — `check` / `infer` / `source` / `target` — by
//! *saturating*, which works there because those sums are only ever compared
//! against a real slice length and `usize::MAX` matches none.
//!
//! The passes here consume the sum as a **magnitude** instead — sizing a node
//! vector, a range bound, a matrix dimension — where `usize::MAX` is not a
//! rejectable sentinel but an allocation abort or a non-terminating loop. Each
//! test below pins the rejection that replaces it, and the verdict the equality
//! APIs return in place of the panic.

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::PropExpr;
#[cfg(feature = "serde")]
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::content::is_arity_well_formed;
use catgraph_applied::prop::presentation::{NormalizeEngine, Presentation};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::{SfgGenerator, SignalFlowGraph};
use catgraph_applied::sfg_to_mat::sfg_to_mat;

type Sfg = SfgGenerator<BoolRig>;
type E = PropExpr<Sfg>;

/// `σ_{usize::MAX, 1}` — width `usize::MAX + 1`.
fn over() -> E {
    PropExpr::Braid(usize::MAX, 1)
}

fn id1() -> E {
    PropExpr::Identity(1)
}

fn seq(a: E, b: E) -> E {
    PropExpr::Compose(Box::new(a), Box::new(b))
}

fn par(a: E, b: E) -> E {
    PropExpr::Tensor(Box::new(a), Box::new(b))
}

// ---- the domain predicates ---------------------------------------------------

/// `arities_fit` is the overflow clause on its own; `is_arity_well_formed` is
/// that clause **and** composability. Before #196 the second one answered `true`
/// on an overflowing braid — it looked at `Compose` joins and nothing else — so
/// the gate at both equality call sites waved the value straight into
/// `content_of`, which then panicked sizing its node vector.
#[test]
fn overflowing_widths_are_outside_both_domains() {
    assert!(!over().arities_fit());
    assert!(!is_arity_well_formed(&over()));

    // Buried under a well-formed top level: `id₁ ⊗ σ` and `id₀ ; (id₀ ; σ)` both
    // hide it from a boundary-only reading.
    assert!(!par(id1(), over()).arities_fit());
    assert!(
        !seq(
            PropExpr::Identity(0),
            seq(PropExpr::Identity(0), par(over(), PropExpr::Identity(0)))
        )
        .arities_fit()
    );

    // A `Tensor` whose halves each fit but whose sum does not.
    let wide = par(PropExpr::Identity(usize::MAX), id1());
    assert!(PropExpr::<Sfg>::Identity(usize::MAX).arities_fit());
    assert!(!wide.arities_fit());
    assert!(!is_arity_well_formed(&wide));

    // The two clauses are genuinely distinct: this one composes badly but every
    // width fits, and it must keep reaching the `nf` fallback rather than the
    // new screen.
    let mismatched = seq(id1(), PropExpr::Identity(2));
    assert!(mismatched.arities_fit());
    assert!(!is_arity_well_formed(&mismatched));

    // Exact arities where they fit.
    assert_eq!(
        par(id1(), PropExpr::Braid(1, 2)).checked_arities(),
        Some((4, 4))
    );
    assert_eq!(over().checked_arities(), None);
}

// ---- eq_mod ------------------------------------------------------------------

/// Both engines are screened before they run, so the verdict is the same under
/// each: `Some(true)` on structurally identical trees (the one claim still
/// soundly available) and `None` — undecided — otherwise. `Some(false)` is never
/// returned: no layer established a disproof.
#[test]
fn eq_mod_is_undecided_on_an_overflowing_width() {
    for engine in [
        NormalizeEngine::CongruenceClosure,
        NormalizeEngine::Structural,
    ] {
        let p = Presentation::<Sfg>::with_engine(engine);

        assert_eq!(
            p.eq_mod(&over(), &id1()).expect("total on overflow input"),
            None,
            "{engine:?}: distinct trees are undecided, not disproved"
        );
        assert_eq!(
            p.eq_mod(&id1(), &over()).expect("total on overflow input"),
            None,
            "{engine:?}: the screen is symmetric"
        );
        assert_eq!(
            p.eq_mod(&over(), &over()).expect("total on overflow input"),
            Some(true),
            "{engine:?}: structural identity still decides"
        );
        // Buried under an otherwise ordinary term.
        assert_eq!(
            p.eq_mod(&par(id1(), over()), &par(id1(), id1()))
                .expect("total on overflow input"),
            None,
            "{engine:?}: buried overflow is screened too"
        );
    }
}

/// The screen is *only* the overflow class. A merely arity-mismatched tree keeps
/// the pre-#57 route — `nf` short-circuit, then congruence closure — and the
/// verdict it always had. `id₁ ; id₂` reaching `Some(true)` against `id₁` is
/// that route being taken: `nf` coalesces the two identity layers, which is
/// exactly the "sound, not complete" behavior the fallback is documented to
/// have. What matters here is that the answer is `Some(_)` at all — the new
/// screen would have returned `None`.
#[test]
fn eq_mod_still_answers_on_a_merely_mismatched_tree() {
    let p = Presentation::<Sfg>::new();
    let ill = seq(id1(), PropExpr::Identity(2));
    assert_eq!(
        p.eq_mod(&ill, &ill).expect("total on ill-formed input"),
        Some(true)
    );
    assert_eq!(
        p.eq_mod(&ill, &id1()).expect("total on ill-formed input"),
        Some(true),
        "the nf fallback, not the #196 screen"
    );
    assert_eq!(
        p.eq_mod(&ill, &PropExpr::Generator(SfgGenerator::Copy))
            .expect("total on ill-formed input"),
        Some(false),
        "still able to separate"
    );
}

// ---- eq_colored ---------------------------------------------------------------

/// `ColoredExpr::new` cannot build one of these: `check` threads a real word
/// through every node, so an overflowing braid demands a `usize::MAX`-long
/// source word and is rejected long before `eq_colored` sees it. The one way in
/// is the documented serde trust boundary — which is exactly the case
/// `eq_colored`'s ill-formed branch exists for — so the test lives in the serde
/// lane.
///
/// `eq_colored` has no "undecided" answer to give, so it falls back to
/// structural equality: sound in the same sense as its arity-ill-formed branch
/// (`true` sound, `false` not a disproof), and reflexive.
#[cfg(feature = "serde")]
#[test]
fn eq_colored_falls_back_to_structural_equality_on_overflow() {
    // `Braid(usize::MAX, 1)` against an empty word — no constructor would accept
    // this pairing, and `Deserialize` does not re-run `check`.
    // `BoolRig` carries no serde derives, so this one case uses the `i64` rig —
    // the payload is irrelevant, the braid width is the subject.
    type SerdeSfg = SfgGenerator<i64>;
    let forged = r#"{"source_word":[],"target_word":[],"expr":{"Braid":[18446744073709551615,1]}}"#;
    let a: ColoredExpr<SerdeSfg> =
        serde_json::from_str(forged).expect("the trust boundary accepts it");
    let b: ColoredExpr<SerdeSfg> =
        serde_json::from_str(forged).expect("the trust boundary accepts it");
    assert!(a.eq_colored(&b), "reflexive on the overflow class");

    let elsewhere =
        r#"{"source_word":[],"target_word":[],"expr":{"Braid":[18446744073709551614,2]}}"#;
    let c: ColoredExpr<SerdeSfg> =
        serde_json::from_str(elsewhere).expect("the trust boundary accepts it");
    assert!(
        !a.eq_colored(&c),
        "structurally distinct overflows are not claimed equal"
    );
}

// ---- nf -----------------------------------------------------------------------

/// `nf` rejects the overflow class at its entry point, in both build profiles.
/// Before #196 this was a debug-build overflow panic inside Step 1 and a
/// release-build wrap onto a small, spuriously valid braid width.
#[test]
#[should_panic(expected = "nf is undefined on a Braid/Tensor width that overflows usize")]
fn nf_rejects_an_overflowing_width() {
    let _ = catgraph_applied::prop::presentation::smc_nf::nf(&over());
}

/// …and only that class. A mismatched `Compose` still normalizes — `eq_mod`'s
/// fallback depends on it.
#[test]
fn nf_still_normalizes_a_merely_mismatched_tree() {
    let ill = seq(id1(), PropExpr::Identity(2));
    let normalized = catgraph_applied::prop::presentation::smc_nf::nf(&ill);
    assert_eq!(
        normalized,
        catgraph_applied::prop::presentation::smc_nf::nf(&ill)
    );
}

// ---- normalize / apply_smc_rules ----------------------------------------------

/// Rule 9 fuses `Identity(m) ⊗ Identity(n)` into `Identity(m + n)`. On overflow
/// the rule declines to fire and the `Tensor` stands — declining a rewrite is
/// always sound, wrapping is not.
#[test]
fn identity_coherence_declines_to_fuse_an_overflowing_pair() {
    let p = Presentation::<Sfg>::new();
    let wide = par(PropExpr::Identity(usize::MAX), id1());
    let out = p
        .normalize(&wide)
        .expect("normalization is infallible today");
    assert_eq!(out.expr, wide, "the pair is left unfused");
    assert!(out.converged);

    // The rule still fires where the sum fits, including at the boundary.
    let fits = par(PropExpr::Identity(usize::MAX - 1), id1());
    assert_eq!(
        p.normalize(&fits).expect("infallible").expr,
        PropExpr::Identity(usize::MAX)
    );
}

// ---- sfg_to_mat ----------------------------------------------------------------

/// `braid_matrix` sizes a `dim × dim` matrix from `m + n`. The functor already
/// returns `Result` for exactly this class of misuse, so the overflow routes
/// through it rather than through a panic.
#[test]
fn sfg_to_mat_rejects_an_overflowing_braid_width() {
    let sfg = SignalFlowGraph::<BoolRig>::from_prop_expr(over());
    match sfg_to_mat(&sfg) {
        Err(CatgraphError::SfgFunctor { message }) => {
            assert!(
                message.contains("overflows usize"),
                "message should name the overflow, got {message:?}"
            );
        }
        other => panic!("expected SfgFunctor, got {other:?}"),
    }
}
