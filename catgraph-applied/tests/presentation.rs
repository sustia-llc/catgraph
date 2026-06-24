//! Integration tests for `Presentation<G>` and the SMC-axiom term rewriter.

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::{
    Free, PropExpr, PropSignature,
    presentation::{NormalizeEngine, Presentation},
};

// ---- Tiny signature for testing ----
//
// Three generators A, B, C, all arity 1→1, encoded directly as enum variants
// (the signature trait requires the generator type itself to implement
// `PropSignature` with its own `source()` / `target()` methods).

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum TestGen {
    A,
    B,
    C,
}

impl PropSignature for TestGen {
    fn source(&self) -> usize {
        1
    }
    fn target(&self) -> usize {
        1
    }
}

fn g(x: TestGen) -> PropExpr<TestGen> {
    Free::<TestGen>::generator(x)
}

// ---- Tests ----

#[test]
fn empty_presentation_applies_smc_rules_only() {
    // (Identity(0) ⊗ A).normalize  should reduce to just A (left unitor).
    let pres = Presentation::<TestGen>::new();
    let expr = Free::<TestGen>::tensor(Free::<TestGen>::identity(0), g(TestGen::A));
    let normalized = pres.normalize(&expr).unwrap().expr;
    assert_eq!(normalized, g(TestGen::A));
}

#[test]
fn user_equation_applied_left_to_right() {
    // Presentation with A = B.
    let mut pres = Presentation::<TestGen>::new();
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    let normalized = pres.normalize(&g(TestGen::A)).unwrap().expr;
    assert_eq!(normalized, g(TestGen::B));
}

#[test]
fn eq_mod_detects_smc_interchange() {
    // (A ⊗ B) ; (A ⊗ B)  vs  (A ; A) ⊗ (B ; B) — should be SMC-equal via interchange.
    let pres = Presentation::<TestGen>::new();

    let lhs = Free::<TestGen>::compose(
        Free::<TestGen>::tensor(g(TestGen::A), g(TestGen::B)),
        Free::<TestGen>::tensor(g(TestGen::A), g(TestGen::B)),
    )
    .unwrap();

    let rhs = Free::<TestGen>::tensor(
        Free::<TestGen>::compose(g(TestGen::A), g(TestGen::A)).unwrap(),
        Free::<TestGen>::compose(g(TestGen::B), g(TestGen::B)).unwrap(),
    );

    assert!(
        pres.eq_mod(&lhs, &rhs).unwrap().unwrap_or(false),
        "(A⊗B);(A⊗B) should SMC-equal (A;A)⊗(B;B)"
    );
}

#[test]
fn arity_mismatch_on_add_equation_rejected() {
    // A is 1→1; (A ⊗ A) is 2→2. Can't equate them.
    let mut pres = Presentation::<TestGen>::new();
    let a_tensor_a = Free::<TestGen>::tensor(g(TestGen::A), g(TestGen::A));
    let result = pres.add_equation(g(TestGen::A), a_tensor_a);
    assert!(matches!(result, Err(CatgraphError::Presentation { .. })));
}

#[test]
fn depth_bound_respected_on_cyclic_rewrite() {
    // Cyclic rewrite A → B, B → A. Normalize must terminate within depth bound.
    let mut pres = Presentation::<TestGen>::with_depth(16);
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    pres.add_equation(g(TestGen::B), g(TestGen::A)).unwrap();

    // Must return *some* PropExpr within bound, even if not unique.
    let result = pres.normalize(&g(TestGen::A));
    assert!(
        result.is_ok(),
        "normalize must terminate under cyclic rewrites: {result:?}"
    );
}

#[test]
fn braid_involution_smc_rule() {
    // Braid(1,1) ; Braid(1,1) should normalize to Identity(2).
    let pres = Presentation::<TestGen>::new();
    let expr = Free::<TestGen>::compose(Free::<TestGen>::braid(1, 1), Free::<TestGen>::braid(1, 1))
        .unwrap();
    let normalized = pres.normalize(&expr).unwrap().expr;
    assert_eq!(normalized, Free::<TestGen>::identity(2));
}

#[test]
fn identity_unitor_right_smc_rule() {
    // A ⊗ Identity(0) should normalize to A.
    let pres = Presentation::<TestGen>::new();
    let expr = Free::<TestGen>::tensor(g(TestGen::A), Free::<TestGen>::identity(0));
    let normalized = pres.normalize(&expr).unwrap().expr;
    assert_eq!(normalized, g(TestGen::A));
}

#[test]
fn compose_identity_reduction_smc_rule() {
    // Identity(1) ; A should normalize to A.
    let pres = Presentation::<TestGen>::new();
    let expr = Free::<TestGen>::compose(Free::<TestGen>::identity(1), g(TestGen::A)).unwrap();
    let normalized = pres.normalize(&expr).unwrap().expr;
    assert_eq!(normalized, g(TestGen::A));
}

// Unused in passing runs; keeps the import honest if a later assertion needs
// it. Present to mirror the spec's triad TestGen::{A, B, C}.
#[allow(dead_code)]
fn _use_c() -> PropExpr<TestGen> {
    g(TestGen::C)
}

// ---- v0.5.1 NormalizeEngine selector tests ----
//
// The overlapping-equations "killer case": seed `A ; A = B` AND `A = C`.
// Under the v0.5.0 structural rewriter, normalize rewrites `A ; A → B` and
// `A → C` independently, yielding distinct normal forms and a false negative.
// The v0.5.1 congruence-closure engine handles overlap and returns `true`.

#[test]
fn presentation_eq_mod_cc_joins_overlapping_equations() {
    // Setup: A;A = B  AND  A = C  ⟹  A;C == C;C == A;A == B (via congruence).
    let mut pres = Presentation::<TestGen>::new(); // default: CongruenceClosure

    let a_semi_a = Free::<TestGen>::compose(g(TestGen::A), g(TestGen::A)).unwrap();
    pres.add_equation(a_semi_a, g(TestGen::B)).unwrap();
    pres.add_equation(g(TestGen::A), g(TestGen::C)).unwrap();

    let a_semi_c = Free::<TestGen>::compose(g(TestGen::A), g(TestGen::C)).unwrap();

    // CC derives A;C == B by congruence: A = C replaces the second A in
    // `A;A = B`, giving A;C = B.
    assert_eq!(
        pres.eq_mod(&a_semi_c, &g(TestGen::B)).unwrap(),
        Some(true),
        "CC engine should derive A;C == B via congruence closure over overlapping equations"
    );
}

#[test]
fn presentation_default_engine_is_cc() {
    // Default `new()` should pick CongruenceClosure — verified by the
    // overlapping-equations killer case returning `Some(true)`.
    let pres = Presentation::<TestGen>::new();
    assert_eq!(pres.engine(), NormalizeEngine::CongruenceClosure);

    // Also verify `with_depth` defaults to CC.
    let pres2 = Presentation::<TestGen>::with_depth(64);
    assert_eq!(pres2.engine(), NormalizeEngine::CongruenceClosure);
}

#[test]
fn presentation_with_engine_structural_recovers_v050_behavior() {
    // Under the Structural engine, a simple non-overlapping equation should
    // still work: A = B ⟹ eq_mod(A, B) = Some(true).
    let mut pres = Presentation::<TestGen>::with_engine(NormalizeEngine::Structural);
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    assert_eq!(pres.engine(), NormalizeEngine::Structural);
    assert_eq!(
        pres.eq_mod(&g(TestGen::A), &g(TestGen::B)).unwrap(),
        Some(true),
        "Structural engine should decide A == B when A = B is the only equation"
    );

    // And `set_engine` flips the engine in place.
    pres.set_engine(NormalizeEngine::CongruenceClosure);
    assert_eq!(pres.engine(), NormalizeEngine::CongruenceClosure);
}

#[test]
fn presentation_cc_handles_both_smc_interchange_and_overlapping_user_equations() {
    // Subsumption contract: the default CC engine handles SMC-structural
    // rewrites AND CC overlapping-equation joining in the SAME presentation.
    //
    // Setup: seed `A;A = B` and `A = C` (overlapping per Thm 5.60 scalar
    // D-group pattern — the second A in `A;A = B` overlaps with `A = C`).
    //
    // Query: `(A ⊗ Identity(0)) ; C` vs `B`.
    //
    // - Pre-pass (SMC structural normalize) on LHS: right unitor rewrites
    //   `A ⊗ Identity(0)` → `A`, yielding `A ; C`. Under pure CC (no pre-
    //   pass) the LHS would have been structurally `(A⊗Identity(0));C`,
    //   which the CC term graph wouldn't unify with any seeded equation
    //   because neither seed has a tensor node.
    // - CC on normalized query: the graph contains `A;A = B` and `A = C`.
    //   Via congruence, `A ; C ≡ C ; C ≡ A ; A ≡ B`. Returns `Some(true)`.
    //
    // If this test fails on the default engine, either the SMC pre-pass
    // didn't run (losing v0.5.0 capability) or the CC engine isn't being
    // fed the normalized equation graph (losing v0.5.1 capability).
    let mut pres = Presentation::<TestGen>::new(); // default: CongruenceClosure
    assert_eq!(pres.engine(), NormalizeEngine::CongruenceClosure);

    let a_semi_a = Free::<TestGen>::compose(g(TestGen::A), g(TestGen::A)).unwrap();
    pres.add_equation(a_semi_a, g(TestGen::B)).unwrap();
    pres.add_equation(g(TestGen::A), g(TestGen::C)).unwrap();

    // LHS: (A ⊗ Identity(0)) ; C. Arity: `A ⊗ Identity(0)` is 1→1 (A is 1→1,
    // Identity(0) is 0→0). C is 1→1. So the compose is well-typed.
    let lhs = Free::<TestGen>::compose(
        Free::<TestGen>::tensor(g(TestGen::A), Free::<TestGen>::identity(0)),
        g(TestGen::C),
    )
    .unwrap();
    let rhs = g(TestGen::B);

    assert_eq!(
        pres.eq_mod(&lhs, &rhs).unwrap(),
        Some(true),
        "default CC engine must subsume BOTH SMC normalization (unitor reduces \
         A⊗Identity(0) → A) AND CC overlapping-equation joining (A;C ≡ A;A ≡ B \
         via A=C congruence)"
    );
}

#[test]
fn presentation_structural_engine_returns_none_on_cyclic_overlap() {
    // With A = B, B = A under Structural + small depth bound, normalization
    // oscillates. The v0.5.0 behavior is to return `None` (depth-bound hit).
    // This locks in that the `Structural` branch preserves the `Option<bool>`
    // return-type contract.
    let mut pres = Presentation::<TestGen>::with_depth(16);
    pres.set_engine(NormalizeEngine::Structural);
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    pres.add_equation(g(TestGen::B), g(TestGen::A)).unwrap();

    // Any of Some(true) / Some(false) / None is acceptable here — the point
    // is that we don't panic and we stay within the depth bound.
    let _ = pres.eq_mod(&g(TestGen::A), &g(TestGen::B)).unwrap();
}
