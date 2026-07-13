//! Law tests for the Cospan-valued [`CompleteFunctor`] on the pure-spider
//! fragment (issue #80, F&S 2019 Prop 3.8).
//!
//! Coverage:
//! - the nine `E_frob` equations all **decide equal** (completeness soundness);
//! - registry integration through `Presentation::eq_mod_functorial`;
//! - scalars are **kept** (η;ε ≠ id₀, bubble multiplicity counted) — the
//!   special-vs-extra-special distinction that makes `Cospan` (not `Corel`) the
//!   right target;
//! - the functor is **strictly finer** than `to_mat_kron` over an idempotent rig
//!   (which loses the scalar), while **agreeing** on the sound direction;
//! - `User` generators fall **outside the fragment**.

mod common;
use common::Sig;

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::presentation::functorial::CompleteFunctor;
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::rig::BoolRig;
use catgraph_syntax::cospan_functor::{CospanFunctor, to_cospan};
use catgraph_syntax::frobenius::{
    FrobeniusOr, hypergraph_presentation, scfm_equations, to_mat_kron,
};

type Term = PropExpr<FrobeniusOr<Sig>>;

fn mu() -> Term {
    Free::generator(FrobeniusOr::Mu)
}
fn eta() -> Term {
    Free::generator(FrobeniusOr::Eta)
}
fn delta() -> Term {
    Free::generator(FrobeniusOr::Delta)
}
fn epsilon() -> Term {
    Free::generator(FrobeniusOr::Epsilon)
}
fn id(n: usize) -> Term {
    Free::<FrobeniusOr<Sig>>::identity(n)
}
fn compose(f: Term, g: Term) -> Term {
    Free::compose(f, g).expect("arity-matched by construction in these tests")
}

/// Every one of the nine `E_frob` equations is decided **equal** by the functor
/// — the completeness-soundness direction (the functor respects all SCFM laws).
#[test]
fn nine_scfm_equations_decide_equal() {
    let f = CospanFunctor::new();
    for (lhs, rhs) in scfm_equations::<Sig>() {
        let fa = f.apply(&lhs).expect("spider fragment is User-free");
        let fb = f.apply(&rhs).expect("spider fragment is User-free");
        assert_eq!(fa, fb, "functor failed to equate an E_frob equation");
    }
}

/// The same nine equations, decided through the presentation registry path
/// `eq_mod_functorial` — a definite `Some(true)`, no depth bound, no `None`.
#[test]
fn registry_integration_via_eq_mod_functorial() {
    let pres = hypergraph_presentation::<Sig>([]).expect("no user equations to lift");
    let f = CospanFunctor::new();
    for (lhs, rhs) in scfm_equations::<Sig>() {
        assert_eq!(
            pres.eq_mod_functorial(&lhs, &rhs, &f)
                .expect("functor applies on User-free terms"),
            Some(true),
        );
    }
}

/// The completeness payoff, and the whole point of #80: a genuine SCFM equality
/// that the default congruence-closure engine **fails to decide** but the
/// complete cospan functor decides `Some(true)`. The witness is a scalar
/// commuting past a tensor — `(η;ε) ⊗ μ = μ ⊗ (η;ε)` (scalars are central in a
/// symmetric monoidal category). `eq_mod` (sound but syntactically incomplete,
/// #15) does not return `Some(true)` here; `eq_mod_functorial` does.
#[test]
fn complete_where_congruence_closure_is_not() {
    let pres = hypergraph_presentation::<Sig>([]).expect("no user equations");
    let f = CospanFunctor::new();
    let bubble = compose(eta(), epsilon());
    let a = Free::tensor(bubble.clone(), mu());
    let b = Free::tensor(mu(), bubble);

    // The syntactic CC engine does not prove the equality (no definite Some(true)).
    assert_ne!(
        pres.eq_mod(&a, &b).expect("eq_mod runs"),
        Some(true),
        "if CC now decides this, pick a harder witness — the test must show a gap",
    );
    // The complete functor does.
    assert_eq!(
        pres.eq_mod_functorial(&a, &b, &f).expect("functor applies"),
        Some(true)
    );
}

/// Scalars are kept: the closed bubble `η ; ε` (a `0 → 0` term) is a genuine
/// non-identity, distinct from `id₀`, and two bubbles differ from one. This is
/// the special-Frobenius property that rules out `Corel` (extra-special) as the
/// target.
#[test]
fn scalars_are_kept() {
    let f = CospanFunctor::new();
    let bubble = compose(eta(), epsilon()); // 0 → 0, one apex-only vertex
    let id0 = id(0);
    let two_bubbles = Free::tensor(bubble.clone(), bubble.clone());

    let b = f.apply(&bubble).unwrap();
    assert_eq!(b.scalar_count(), 1);
    assert_eq!(f.apply(&id0).unwrap().scalar_count(), 0);
    assert_ne!(b, f.apply(&id0).unwrap(), "η;ε must not equal id₀");
    assert_eq!(f.apply(&two_bubbles).unwrap().scalar_count(), 2);
    assert_ne!(
        b,
        f.apply(&two_bubbles).unwrap(),
        "one bubble ≠ two bubbles"
    );
}

/// The functor is **strictly finer** than `to_mat_kron` over an idempotent rig:
/// over `BoolRig` the scalar `η;ε` collapses to the same 1×1 matrix as `id₀`
/// (`to_mat_kron` is only *sound*), but the cospan functor separates them —
/// concretely why a complete decision needs the finer `Cospan` target.
#[test]
fn finer_than_mat_kron_over_idempotent_rig() {
    let f = CospanFunctor::new();
    let bubble = compose(eta(), epsilon());
    let id0 = id(0);

    // Cospan functor: separated.
    assert_ne!(f.apply(&bubble).unwrap(), f.apply(&id0).unwrap());

    // to_mat_kron over BoolRig at dim 2: identified (both the trivial 1×1 scalar).
    let mb = to_mat_kron::<Sig, BoolRig>(&bubble, 2).unwrap();
    let mi = to_mat_kron::<Sig, BoolRig>(&id0, 2).unwrap();
    assert_eq!(
        mb, mi,
        "sanity: BoolRig is idempotent so the bubble scalar is invisible to to_mat_kron"
    );
}

/// Sound agreement: on the nine genuine equalities, `to_mat_kron` (the
/// independently-implemented Prop-3.8 checker) also equates both sides — the two
/// functors cannot disagree on things that are actually equal.
#[test]
fn sound_agreement_with_to_mat_kron() {
    for (lhs, rhs) in scfm_equations::<Sig>() {
        let ml = to_mat_kron::<Sig, BoolRig>(&lhs, 2).unwrap();
        let mr = to_mat_kron::<Sig, BoolRig>(&rhs, 2).unwrap();
        assert_eq!(ml, mr, "to_mat_kron disagrees with an E_frob equation");
    }
}

/// Genuinely different spiders are decided **distinct** — both across boundary
/// shape (`μ` vs `δ`) and within a fixed boundary (`μ` merges; `id ⊗ ε`
/// discards).
#[test]
fn distinct_spiders_are_separated() {
    let f = CospanFunctor::new();
    assert_ne!(f.apply(&mu()).unwrap(), f.apply(&delta()).unwrap());

    // Both 2 → 1, but different wirings.
    let discard_right = Free::tensor(id(1), epsilon());
    assert_ne!(f.apply(&mu()).unwrap(), f.apply(&discard_right).unwrap());
}

/// `User` generators are opaque and lie **outside** the pure-spider fragment:
/// `apply` fails with [`CatgraphError::Presentation`].
#[test]
fn user_generator_is_outside_the_fragment() {
    let f = CospanFunctor::new();
    let term: Term = Free::generator(FrobeniusOr::User(Sig::Copy));
    let err = f
        .apply(&term)
        .expect_err("User generators must be rejected");
    assert!(matches!(err, CatgraphError::Presentation { .. }));

    // Also rejected when buried inside a composite.
    let buried = Free::tensor(mu(), Free::generator(FrobeniusOr::User(Sig::Add)));
    assert!(to_cospan::<Sig>(&buried).is_err());
}

/// Arity mismatch in a `Compose` surfaces transparently as
/// [`CatgraphError::Composition`] from the cospan pushout (mirroring
/// `to_mat_kron`).
#[test]
fn arity_mismatch_surfaces_as_composition_error() {
    // η : 0 → 1 composed with μ : 2 → 1 — interface 1 ≠ 2.
    let bad = PropExpr::Compose(Box::new(eta()), Box::new(mu()));
    let err = to_cospan::<Sig>(&bad).expect_err("interface mismatch");
    assert!(matches!(err, CatgraphError::Composition { .. }));
}
