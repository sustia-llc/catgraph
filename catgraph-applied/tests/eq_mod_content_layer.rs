//! `eq_mod` and `eq_colored` on the SMC layer, after the content wiring
//! ([#57](https://github.com/sustia-llc/catgraph/issues/57), a1 PR2).
//!
//! The equality of record at the SMC layer is now content equality, not normal-
//! form equality. Content decides SMC-equality exactly (Lemma 4.1,
//! `docs/SMC-NF-RECONCILIATION.md` §4.2) while `nf` does not — §4.4 and §4.6
//! catalogue SMC-equal pairs it separates — so the observable change is that
//! those pairs now come back `Ok(Some(true))`.
//!
//! Each witness below is recorded with the verdict it returned *before* the
//! wiring, so the tests read as a behavior diff rather than as bare expectations.

use catgraph_applied::prop::PropExpr;
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;

type Sfg = SfgGenerator<BoolRig>;
type E = PropExpr<Sfg>;

fn prim(g: Sfg) -> E {
    PropExpr::Generator(g)
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
fn strue() -> E {
    prim(SfgGenerator::Scalar(BoolRig(true)))
}
fn sfalse() -> E {
    prim(SfgGenerator::Scalar(BoolRig(false)))
}

/// An empty presentation: no user equations, so `eq_mod` reports the SMC layer
/// alone and the CC arm has nothing to add.
fn smc_only() -> Presentation<Sfg> {
    Presentation::<Sfg>::new()
}

// ---- the named divergence witnesses, previously Some(false) -----------------

/// §4.4's withdrawal witness — `η` placement slack, the mechanism behind all of
/// the then-253 published divergences (2026-07-28 classification; the tracker
/// reads 183 since #185). `eq_mod` returned `Ok(Some(false))` before the wiring.
#[test]
fn eta_slack_pair_is_now_decided_equal() {
    let a = par(
        seq(
            prim(SfgGenerator::Copy),
            par(id1(), prim(SfgGenerator::Discard)),
        ),
        prim(SfgGenerator::Zero),
    );
    let b = seq(
        prim(SfgGenerator::Copy),
        par(
            par(id1(), prim(SfgGenerator::Zero)),
            prim(SfgGenerator::Discard),
        ),
    );
    assert_eq!(smc_only().eq_mod(&a, &b).expect("total"), Some(true));
}

/// §4.4 F2 — a braid prefix the diagram carries but the content does not. NF
/// braid-freeness is per-writing, so no NF-level fix reaches this pair; it
/// returned `Ok(Some(false))` before.
#[test]
fn dead_braid_prefix_pair_is_now_decided_equal() {
    let a = seq(
        PropExpr::Braid(1, 1),
        par(prim(SfgGenerator::Discard), prim(SfgGenerator::Discard)),
    );
    let b = par(prim(SfgGenerator::Discard), prim(SfgGenerator::Discard));
    assert_eq!(smc_only().eq_mod(&a, &b).expect("total"), Some(true));
}

/// §4.4 F1 / [#185](https://github.com/sustia-llc/catgraph/issues/185) — the
/// split-presence column nesting, inside `𝔉′`. Returned `Ok(Some(false))` under
/// the old `nf`-equality path; the content path decided it equal while `nf` was
/// still separating it, and #185's symmetric Step-6½ cuts (2026-08-02) have
/// since closed the `nf` side too — so this now pins agreement between the two
/// layers rather than the content layer rescuing an engine defect.
#[test]
fn cut_asymmetry_pair_is_now_decided_equal() {
    let a = seq(
        seq(
            prim(SfgGenerator::Copy),
            par(par(id1(), prim(SfgGenerator::Zero)), sfalse()),
        ),
        par(par(id1(), strue()), prim(SfgGenerator::Discard)),
    );
    let b = par(
        seq(
            seq(prim(SfgGenerator::Copy), par(id1(), sfalse())),
            par(id1(), prim(SfgGenerator::Discard)),
        ),
        seq(prim(SfgGenerator::Zero), strue()),
    );
    assert_eq!(smc_only().eq_mod(&a, &b).expect("total"), Some(true));
}

// ---- the layer boundary is unmoved ------------------------------------------

/// The content layer decides SMC-equality and nothing more. `Copy ; Add` and
/// `Copy ; σ ; Add` are not SMC-equal, so with no user equations present they
/// stay `Ok(Some(false))` — cocommutativity is a Thm 5.60 *user* equation, and
/// wiring content into `eq_mod` must not have smuggled it in.
#[test]
fn cocommutativity_is_still_a_user_equation() {
    let a = seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add));
    let b = seq(
        prim(SfgGenerator::Copy),
        seq(PropExpr::Braid(1, 1), prim(SfgGenerator::Add)),
    );
    assert_eq!(smc_only().eq_mod(&a, &b).expect("total"), Some(false));
}

/// A genuinely different morphism stays unequal.
#[test]
fn distinct_morphisms_stay_unequal() {
    let a = seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add));
    assert_eq!(smc_only().eq_mod(&a, &id1()).expect("total"), Some(false));
}

/// The `Structural` engine is out of scope for the wiring: bounded rewriting and
/// compare is its documented contract, and it still separates the `η`-slack pair.
#[test]
fn structural_engine_is_untouched() {
    use catgraph_applied::prop::presentation::NormalizeEngine;

    let a = par(
        seq(
            prim(SfgGenerator::Copy),
            par(id1(), prim(SfgGenerator::Discard)),
        ),
        prim(SfgGenerator::Zero),
    );
    let b = seq(
        prim(SfgGenerator::Copy),
        par(
            par(id1(), prim(SfgGenerator::Zero)),
            prim(SfgGenerator::Discard),
        ),
    );
    let pres = Presentation::<Sfg>::with_engine(NormalizeEngine::Structural);
    assert_eq!(pres.eq_mod(&a, &b).expect("total"), Some(false));
}

// ---- totality on arity-ill-formed input --------------------------------------

/// `content_of` panics outside its arity-well-formed domain, so `eq_mod` gates
/// the content arm and falls back. These are the verdicts the pre-wiring engine
/// returned, recorded by running it before the change and asserted here
/// unchanged — the point is not that `Some(true)` is *right* for an ill-formed
/// tree (it is garbage in, garbage out), but that wiring content in changed
/// nothing about it.
#[test]
fn arity_ill_formed_input_keeps_its_previous_verdicts() {
    // `Identity(1) ; Identity(2)` — accepted by the raw variant, rejected by
    // `Free::compose`.
    let ill = PropExpr::<Sfg>::Compose(
        Box::new(PropExpr::Identity(1)),
        Box::new(PropExpr::Identity(2)),
    );
    let nested = PropExpr::<Sfg>::Compose(
        Box::new(PropExpr::Identity(1)),
        Box::new(PropExpr::Compose(
            Box::new(PropExpr::Identity(2)),
            Box::new(PropExpr::Identity(1)),
        )),
    );
    let pres = smc_only();

    for (name, a, b) in [
        ("ill vs itself", &ill, &ill),
        ("ill vs id₁", &ill, &id1()),
        ("id₁ vs ill", &id1(), &ill),
        ("nested ill vs id₁", &nested, &id1()),
    ] {
        assert_eq!(
            pres.eq_mod(a, b).expect("total on ill-formed input"),
            Some(true),
            "{name}: the pre-wiring verdict was Some(true)"
        );
    }
}

/// The domain predicate the gate reads, checked on the same shapes.
#[test]
fn arity_predicate_matches_the_gate() {
    use catgraph_applied::prop::presentation::content::is_arity_well_formed;

    let ill = PropExpr::<Sfg>::Compose(
        Box::new(PropExpr::Identity(1)),
        Box::new(PropExpr::Identity(2)),
    );
    assert!(!is_arity_well_formed(&ill));
    // Ill-formedness nested under a well-formed top level is still caught: the
    // top-level arities of `id₁ ; (id₂ ; id₁)` agree.
    let nested = PropExpr::<Sfg>::Compose(
        Box::new(PropExpr::Identity(1)),
        Box::new(PropExpr::Compose(
            Box::new(PropExpr::Identity(2)),
            Box::new(PropExpr::Identity(1)),
        )),
    );
    assert_eq!(nested.source(), nested.target());
    assert!(!is_arity_well_formed(&nested));

    assert!(is_arity_well_formed(&seq(
        prim(SfgGenerator::Copy),
        prim(SfgGenerator::Add)
    )));
    assert!(is_arity_well_formed(&par(id1(), PropExpr::Braid(1, 1))));
}

// ---- the colored surface -----------------------------------------------------

fn colored(expr: E) -> ColoredExpr<Sfg> {
    let word = vec![(); expr.source()];
    ColoredExpr::new(word, expr).expect("word-well-formed")
}

/// `eq_colored` was normal-form equality plus boundary words, so it separated the
/// same pairs `eq_mod` did. It now decides them.
#[test]
fn eq_colored_decides_the_eta_slack_pair() {
    let a = colored(par(
        seq(
            prim(SfgGenerator::Copy),
            par(id1(), prim(SfgGenerator::Discard)),
        ),
        prim(SfgGenerator::Zero),
    ));
    let b = colored(seq(
        prim(SfgGenerator::Copy),
        par(
            par(id1(), prim(SfgGenerator::Zero)),
            prim(SfgGenerator::Discard),
        ),
    ));
    assert!(a.eq_colored(&b));
}

#[test]
fn eq_colored_decides_the_dead_braid_prefix_pair() {
    let a = colored(seq(
        PropExpr::Braid(1, 1),
        par(prim(SfgGenerator::Discard), prim(SfgGenerator::Discard)),
    ));
    let b = colored(par(
        prim(SfgGenerator::Discard),
        prim(SfgGenerator::Discard),
    ));
    assert!(a.eq_colored(&b));
}

/// The layer boundary again, on the colored surface.
#[test]
fn eq_colored_keeps_cocommutativity_out() {
    let a = colored(seq(prim(SfgGenerator::Copy), prim(SfgGenerator::Add)));
    let b = colored(seq(
        prim(SfgGenerator::Copy),
        seq(PropExpr::Braid(1, 1), prim(SfgGenerator::Add)),
    ));
    assert!(!a.eq_colored(&b));
}

/// Non-parallel morphisms are unequal whatever their contents do.
#[test]
fn eq_colored_still_requires_parallelism() {
    let a = colored(prim(SfgGenerator::Copy)); // 1 → 2
    let b = colored(id1()); // 1 → 1
    assert!(!a.eq_colored(&b));
}
