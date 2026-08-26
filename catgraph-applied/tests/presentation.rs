//! Integration tests for `Presentation<G>` and the SMC-axiom term rewriter.

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::{
    Free, PropExpr, PropSignature, mono_word,
    presentation::{NormalizeEngine, Presentation},
};
use std::borrow::Cow;

// ---- Tiny signature for testing ----
//
// Three generators A, B, C, all arity 1→1, encoded directly as enum variants
// (the signature trait requires the generator type itself to implement
// `PropSignature` with its own `source()` / `target()` methods).

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum TestGen {
    A,
    B,
    C,
}

impl PropSignature for TestGen {
    type Color = ();

    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
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
    // A is 1→1; (A ⊗ A) is 2→2. Can't equate them. Since #79 P2 the rejection
    // comes from the boundary-word pass, so the error is the shared
    // size-mismatch variant rather than `Presentation`: threading the single
    // inferred source letter through `A ⊗ A` leaves the right factor with none.
    let mut pres = Presentation::<TestGen>::new();
    let a_tensor_a = Free::<TestGen>::tensor(g(TestGen::A), g(TestGen::A));
    let result = pres.add_equation(g(TestGen::A), a_tensor_a);
    assert!(
        matches!(
            result,
            Err(CatgraphError::CompositionSizeMismatch {
                expected: 1,
                actual: 0
            })
        ),
        "expected a size mismatch, got {result:?}"
    );
}

#[test]
fn mono_equation_with_matching_words_still_accepted() {
    // Regression: the ordinary mono path is untouched by P2. `A ; A` and `B`
    // are both 1 → 1 and both well-composed.
    let mut pres = Presentation::<TestGen>::new();
    let a_semi_a = Free::<TestGen>::compose(g(TestGen::A), g(TestGen::A)).unwrap();
    pres.add_equation(a_semi_a, g(TestGen::B))
        .expect("1 → 1 on both sides");
}

#[test]
fn ill_composed_tree_rejected_even_though_top_arities_match() {
    // The P2 strengthening witness. `PropExpr`'s variants are public, so a tree
    // can be assembled without going through `Free::compose`'s check:
    // `Identity(1) ; (Identity(2) ; Identity(1))` reads 1 → 1 at the top
    // (source from the outermost left, target from the innermost right), which
    // the pre-P2 top-level-arity check accepted — but the inner `Identity(2)`
    // is handed a one-letter word.
    let ill = PropExpr::Compose(
        Box::new(PropExpr::Identity(1)),
        Box::new(PropExpr::Compose(
            Box::new(PropExpr::Identity(2)),
            Box::new(PropExpr::Identity(1)),
        )),
    );
    assert_eq!(
        ill.source(),
        1,
        "top-level arities are what the old check saw"
    );
    assert_eq!(ill.target(), 1);

    let mut pres = Presentation::<TestGen>::new();
    let result = pres.add_equation(ill, Free::<TestGen>::identity(1));
    assert!(
        matches!(
            result,
            Err(CatgraphError::CompositionSizeMismatch {
                expected: 2,
                actual: 1
            })
        ),
        "expected the inner Identity(2) to reject a 1-letter word, got {result:?}"
    );
}

#[test]
fn paired_inverse_equations_converge_in_one_iteration() {
    // A = B, B = A at depth 16; normalize(A): converged, expr A, steps_taken 1.
    let mut pres = Presentation::<TestGen>::with_depth(16);
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    pres.add_equation(g(TestGen::B), g(TestGen::A)).unwrap();

    let result = pres.normalize(&g(TestGen::A)).unwrap();
    assert!(result.converged);
    assert_eq!(result.expr, g(TestGen::A));
    assert_eq!(result.steps_taken, 1);
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

// ---- NormalizeEngine selector tests ----
//
// The overlapping-equations "killer case": seed `A ; A = B` AND `A = C`.
// Under the structural rewriter, normalize rewrites `A ; A → B` and
// `A → C` independently, yielding distinct normal forms and a false negative.
// The congruence-closure engine handles overlap and returns `true`.

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
fn presentation_exposes_its_configured_rewrite_depth() {
    // The sibling of `engine()`, and needed for the same reason: a consumer that
    // stores a presentation as its parts and rebuilds it later has to be able to
    // read the depth back. Without this accessor a rebuild through
    // `new()` + `add_equation` silently restored the default 32.
    assert_eq!(Presentation::<TestGen>::new().rewrite_depth(), 32);
    assert_eq!(Presentation::<TestGen>::default().rewrite_depth(), 32);
    assert_eq!(Presentation::<TestGen>::with_depth(7).rewrite_depth(), 7);
    assert_eq!(Presentation::<TestGen>::with_depth(0).rewrite_depth(), 0);

    // `with_engine` is the other constructor that takes no depth, and it lands on
    // the same default `new()` does above.
    assert_eq!(
        Presentation::<TestGen>::with_engine(NormalizeEngine::Structural).rewrite_depth(),
        32
    );

    // And the documented rebuild really does need both calls: `with_depth`
    // carries the depth alone, so the engine has to be restored separately or it
    // silently reverts to the default — the same silent-default bug the depth
    // accessor exists to prevent, one slot over.
    assert_eq!(
        Presentation::<TestGen>::with_depth(7).engine(),
        NormalizeEngine::CongruenceClosure
    );
    let mut rebuilt = Presentation::<TestGen>::with_depth(7);
    rebuilt.set_engine(NormalizeEngine::Structural);
    assert_eq!(rebuilt.rewrite_depth(), 7);
    assert_eq!(rebuilt.engine(), NormalizeEngine::Structural);

    // Adding equations does not disturb it.
    let mut pres = Presentation::<TestGen>::with_depth(7);
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    assert_eq!(pres.rewrite_depth(), 7);
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
    // didn't run (losing SMC-normalization capability) or the CC engine isn't
    // being fed the normalized equation graph (losing overlap-joining capability).
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
fn presentation_structural_engine_decides_true_on_paired_inverse_equations() {
    // A = B, B = A under Structural at depth 16: normalize(A) = (A, 1 step),
    // normalize(B) = (A, 2 steps), eq_mod(A, B) = Some(true).
    let mut pres = Presentation::<TestGen>::with_depth(16);
    pres.set_engine(NormalizeEngine::Structural);
    pres.add_equation(g(TestGen::A), g(TestGen::B)).unwrap();
    pres.add_equation(g(TestGen::B), g(TestGen::A)).unwrap();

    let na = pres.normalize(&g(TestGen::A)).unwrap();
    assert!(na.converged);
    assert_eq!(na.expr, g(TestGen::A));
    assert_eq!(na.steps_taken, 1);

    let nb = pres.normalize(&g(TestGen::B)).unwrap();
    assert!(nb.converged);
    assert_eq!(nb.expr, g(TestGen::A));
    assert_eq!(nb.steps_taken, 2);

    assert_eq!(
        pres.eq_mod(&g(TestGen::A), &g(TestGen::B)).unwrap(),
        Some(true)
    );
}

#[test]
fn presentation_structural_engine_returns_none_only_when_a_side_hits_the_bound() {
    // A = A;A under Structural at depth 4: normalize(A) converged false,
    // steps_taken 4; normalize(B) = (B, 1 step); eq_mod(A, B) = None;
    // eq_mod(B ⊗ Identity(0), B) = Some(true); eq_mod(B, C) = Some(false).
    let mut pres = Presentation::<TestGen>::with_depth(4);
    pres.set_engine(NormalizeEngine::Structural);
    let a_then_a = Free::<TestGen>::compose(g(TestGen::A), g(TestGen::A)).unwrap();
    pres.add_equation(g(TestGen::A), a_then_a).unwrap();

    let na = pres.normalize(&g(TestGen::A)).unwrap();
    assert!(!na.converged);
    assert_eq!(na.steps_taken, 4);

    let nb = pres.normalize(&g(TestGen::B)).unwrap();
    assert!(nb.converged);
    assert_eq!(nb.expr, g(TestGen::B));
    assert_eq!(nb.steps_taken, 1);

    assert_eq!(pres.eq_mod(&g(TestGen::A), &g(TestGen::B)).unwrap(), None);

    let b_tensor_id0 = Free::<TestGen>::tensor(g(TestGen::B), Free::<TestGen>::identity(0));
    let n_unitor = pres.normalize(&b_tensor_id0).unwrap();
    assert!(n_unitor.converged);
    assert_eq!(n_unitor.expr, g(TestGen::B));
    assert_eq!(n_unitor.steps_taken, 2);

    assert_eq!(
        pres.eq_mod(&b_tensor_id0, &g(TestGen::B)).unwrap(),
        Some(true)
    );
    assert_eq!(
        pres.eq_mod(&g(TestGen::B), &g(TestGen::C)).unwrap(),
        Some(false)
    );
}

#[test]
fn presentation_cc_engine_returns_none_when_the_smc_pre_pass_hits_the_bound() {
    // Depth 0, default CC engine: eq_mod(A, B) = None, eq_mod(A, A) =
    // Some(true); the same pair at the default depth 32 = Some(false).
    let pres = Presentation::<TestGen>::with_depth(0);
    assert_eq!(pres.engine(), NormalizeEngine::CongruenceClosure);
    assert_eq!(pres.eq_mod(&g(TestGen::A), &g(TestGen::B)).unwrap(), None);
    assert_eq!(
        pres.eq_mod(&g(TestGen::A), &g(TestGen::A)).unwrap(),
        Some(true)
    );

    assert_eq!(
        Presentation::<TestGen>::new()
            .eq_mod(&g(TestGen::A), &g(TestGen::B))
            .unwrap(),
        Some(false)
    );
}
