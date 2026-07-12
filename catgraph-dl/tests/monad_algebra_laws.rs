//! Machine-checked monad-algebra coherence-law acceptance tests.
//!
//! CDL Definition 2.3. A monad algebra `(A, a : M(A) → A)` for a monad
//! `(M, η, μ)` must satisfy the **unit law** `a ∘ η_A = id_A` and the
//! **associativity law** `a ∘ M(a) = a ∘ μ_A`. These tests exercise the four
//! sample-based verifiers added in issue #40 against the group-action monad
//! `Z2 × −` on `Vec<f64>` (`η = ` haft's `Pure`, `μ = ` haft's `Monad::join`):
//!
//! - [`MonadAlgebra::verify_unit_law`] / [`MonadAlgebra::verify_assoc_law`]
//! - [`MonadAlgebraHom::verify_unit_coherence`] /
//!   [`MonadAlgebraHom::verify_mult_coherence`]
//!
//! Coverage is three-sided:
//!
//! 1. **Positive** — the `Z2`-negation action is a genuine monad algebra
//!    (`g1 ▶ (g2 ▶ x) = (g1 · g2) ▶ x`), so all four verifiers hold; the
//!    pointwise absolute-value map is the CDL Example 2.6 GDL-recovery
//!    homomorphism between the negation and trivial actions.
//! 2. **Negative** — unlawful structure maps make [`MonadAlgebra`]'s verifiers
//!    return `false`, so a verifier regressing to a tautology fails this suite.
//! 3. **Boundary** — the hom-side coherence verifiers hold even for a
//!    **non**-homomorphism (they probe the ambient monad/algebra structure,
//!    not `f` — see the ⚠️ scope note on [`MonadAlgebraHom`]); only
//!    `verify_commutes` discriminates. Demonstrated with the `first_coord`
//!    projection.
//!
//! The writer-monad `bind`/`join` accumulation *order* (`g · g2`, CDL Def 2.1 /
//! Ex 2.2) is invisible to every `Z2` test (XOR is commutative), so a
//! test-local **non-abelian** `S3` instance pins it exhaustively.
//!
//! The verifiers are **caller-sampled**, not exhaustive — same honesty as
//! `FAlgebraHom::verify_commutes`.

#![allow(
    clippy::float_cmp,
    clippy::type_complexity,
    reason = "Test file. float_cmp: samples are finite (no NaN), so f64 `PartialEq` is meaningful. type_complexity: the MonadAlgebraHom<…6 type params…> spelling is exactly what callers see — a `type` alias would still need every parameter and obscures the CDL anchor. Same precedent as tests/algebra_homomorphisms.rs."
)]

mod common;

use common::{
    VecMap, Z2Action, Z2Endo, abs_map, finite_f64, first_coord, negation_action, trivial_action,
};

use catgraph_dl::algebra::{
    FAlgebra, FAlgebraHom, Group, GroupActionEndo, MonadAlgebra, MonadAlgebraHom, Z2Group,
};
use catgraph_dl::{Monad, Pure};

use proptest::prelude::*;

/// Build the `(Vec<f64>, negation)` monad algebra. The carrier value is
/// irrelevant to the verifiers (they sample the structure map).
fn negation_algebra() -> MonadAlgebra<Z2Endo, Vec<f64>, Z2Action> {
    MonadAlgebra::new(FAlgebra::new(vec![0.0_f64], negation_action as Z2Action))
}

/// Build a monad-algebra homomorphism candidate
/// `(Vec<f64>, negation) → (Vec<f64>, trivial)` carrying `f`.
fn z2_monad_hom(
    f: VecMap,
) -> MonadAlgebraHom<Z2Endo, Vec<f64>, Vec<f64>, Z2Action, Z2Action, VecMap> {
    let from = FAlgebra::new(vec![0.0_f64], negation_action as Z2Action);
    let to = FAlgebra::new(vec![0.0_f64], trivial_action as Z2Action);
    MonadAlgebraHom::new(FAlgebraHom::new(from, to, f))
}

/// **Unit + associativity laws for the negation algebra — deterministic.**
/// CDL Definition 2.3.
///
/// - Unit `a(η(x)) == x`: `η(x) = (e, x) = (Z2(false), x)`, and
///   `negation_action((false, x)) = x`.
/// - Associativity `a(M(a)(mma)) == a(μ(mma))`: the group-action axiom
///   `g1 ▶ (g2 ▶ x) == (g1 · g2) ▶ x`, checked over both group elements.
#[test]
fn negation_monad_algebra_unit_and_assoc_laws() {
    let alg = negation_algebra();

    // Unit law across several carriers, including the empty vector.
    for x in [vec![1.0_f64, -2.0, 3.0], vec![-7.5_f64, 0.0, 4.25], vec![]] {
        assert!(
            alg.verify_unit_law(x.clone()),
            "unit law a(η(x)) == x must hold at x = {x:?}"
        );
    }

    // Associativity law across all four (g1, g2) combinations.
    for g1 in [false, true] {
        for g2 in [false, true] {
            let mma = (Z2Group(g1), (Z2Group(g2), vec![1.0_f64, -2.0, 3.0]));
            assert!(
                alg.verify_assoc_law(mma),
                "assoc law a∘M(a) == a∘μ must hold at (g1={g1}, g2={g2})"
            );
        }
    }
}

/// **Negative cases: unlawful structure maps are rejected.** CDL Def 2.3.
///
/// Guards the verifiers themselves against regressing to tautologies (a
/// verifier comparing a leg to itself, or stubbed to `true`, fails here):
///
/// - `drop_action` (constant empty vector) violates the **unit law** on any
///   non-empty carrier: `a(η(x)) = [] ≠ x`.
/// - `broken_action` (acts only on the `false` element, ignoring group
///   composition) violates **associativity** at `g1 = g2 = true`:
///   `a(M(a)((true, (true, x)))) = []` but `a(μ(...)) = a((false, x)) = x`.
#[test]
fn unlawful_structure_maps_fail_the_algebra_laws() {
    fn drop_action((_g, _x): (Z2Group, Vec<f64>)) -> Vec<f64> {
        Vec::new()
    }
    fn broken_action((g, x): (Z2Group, Vec<f64>)) -> Vec<f64> {
        if g.0 { Vec::new() } else { x }
    }

    let dropped: MonadAlgebra<Z2Endo, Vec<f64>, Z2Action> =
        MonadAlgebra::new(FAlgebra::new(vec![0.0_f64], drop_action as Z2Action));
    assert!(
        !dropped.verify_unit_law(vec![1.0_f64, 2.0]),
        "constant-empty action must FAIL the unit law on a non-empty carrier"
    );

    let broken: MonadAlgebra<Z2Endo, Vec<f64>, Z2Action> =
        MonadAlgebra::new(FAlgebra::new(vec![0.0_f64], broken_action as Z2Action));
    let mma = (Z2Group(true), (Z2Group(true), vec![1.0_f64, -2.0]));
    assert!(
        !broken.verify_assoc_law(mma),
        "composition-ignoring action must FAIL associativity at g1 = g2 = true"
    );
}

/// **Unit + multiplication coherence for the abs-value hom — deterministic.**
/// CDL Definition 2.3 / Example 2.6.
///
/// - Unit coherence `M(f)(η(x)) == η(f(x))`: η-naturality of `f` — both sides
///   carry the identity group element and apply `f` to the payload.
/// - Multiplication coherence `f(a(M(a)(mma))) == f(a(μ(mma)))`: follows because
///   the source action is a genuine group action (inner assoc holds), then `f`
///   is applied on both sides.
#[test]
fn abs_hom_unit_and_mult_coherence() {
    let hom = z2_monad_hom(abs_map as VecMap);

    for x in [vec![1.0_f64, -2.0], vec![-3.0_f64, 4.0, -5.0], vec![]] {
        assert!(
            hom.verify_unit_coherence(x.clone()),
            "unit coherence M(f)∘η == η∘f must hold at x = {x:?}"
        );
    }

    for g1 in [false, true] {
        for g2 in [false, true] {
            let mma = (Z2Group(g1), (Z2Group(g2), vec![1.0_f64, -2.0, 3.0]));
            assert!(
                hom.verify_mult_coherence(mma),
                "mult coherence f∘a∘M(a) == f∘a∘μ must hold at (g1={g1}, g2={g2})"
            );
        }
    }
}

/// **Boundary: the hom coherence verifiers do not discriminate homs.**
///
/// `first_coord` is **not** a monad-algebra homomorphism — it fails the
/// F-algebra square (`verify_commutes`) — yet both point-2 coherence verifiers
/// return `true`: unit coherence is η-naturality (a law of the monad witness,
/// true for every `f`), and mult coherence follows from the *source* algebra's
/// associativity alone. This test pins the documented ⚠️ scope note on
/// [`MonadAlgebraHom`]: certifying a hom requires the square.
#[test]
fn hom_coherence_verifiers_pass_for_a_non_homomorphism() {
    let hom = z2_monad_hom(first_coord as VecMap);
    let x = vec![1.0_f64, 2.0];

    assert!(
        !hom.algebra_hom.verify_commutes((Z2Group(true), x.clone())),
        "the square must FAIL for the projection — it is not a hom"
    );
    assert!(
        hom.verify_unit_coherence(x.clone()),
        "unit coherence holds for ANY f over a lawful monad (η-naturality)"
    );
    assert!(
        hom.verify_mult_coherence((Z2Group(true), (Z2Group(false), x))),
        "mult coherence holds for ANY f over a lawful source algebra"
    );
}

/// **The full monad-algebra-homomorphism certification recipe — end to end.**
/// CDL Def 2.3 (algebra laws) + Def 2.5 (hom square); Mac Lane CWM VI.2.
///
/// Certifying "`f` is a homomorphism between *lawful* Eilenberg–Moore algebras"
/// is a **three-part** conjunction — no single verifier decides it (the ⚠️ scope
/// note on [`MonadAlgebraHom`]: the coherence verifiers probe the ambient
/// structure, not `f`; only the square discriminates):
///
/// 1. **source algebra lawful** — `verify_unit_law` + `verify_assoc_law`;
/// 2. **target algebra lawful** — the same two, after rewrapping the hom's bare
///    [`FAlgebra`] field as a [`MonadAlgebra`] (`MonadAlgebra::new(hom.algebra_hom.to.clone())`
///    — the mildly unergonomic step #67 flags);
/// 3. **the hom square** — `hom.algebra_hom.verify_commutes` (CDL Def 2.5).
///
/// This test runs the composite `(source_lawful, target_lawful, square)` tuple
/// positively (the abs-value hom → `(true, true, true)`) and against three
/// negatives that each fail **exactly one** part while the other two pass —
/// proving the recipe is a genuine three-way conjunction, not any single check
/// masquerading as certification.
#[test]
fn full_monad_algebra_hom_certification_recipe() {
    type Hom = MonadAlgebraHom<Z2Endo, Vec<f64>, Vec<f64>, Z2Action, Z2Action, VecMap>;

    // Non-empty carriers throughout: `drop_action`'s unit-law violation only
    // shows on a non-empty carrier (`a(η([])) = [] = []` holds vacuously), and
    // `first_coord` panics on the empty vector.
    fn carriers() -> [Vec<f64>; 2] {
        [vec![1.0_f64, -2.0, 3.0], vec![-4.5_f64, 6.0]]
    }

    /// Run the three-part recipe, returning `(source_lawful, target_lawful,
    /// square)`. Structure maps are sampled (caller-sampled honesty), so
    /// "lawful"/"commutes" means "holds on every sample".
    fn certify(hom: &Hom) -> (bool, bool, bool) {
        // Rewrap the bare `FAlgebra` fields as `MonadAlgebra`s to reach the
        // unit/assoc verifiers (the ergonomic wrinkle #67 notes).
        let source = MonadAlgebra::new(hom.algebra_hom.from.clone());
        let target = MonadAlgebra::new(hom.algebra_hom.to.clone());

        let lawful = |alg: &MonadAlgebra<Z2Endo, Vec<f64>, Z2Action>| {
            carriers().iter().all(|x| alg.verify_unit_law(x.clone()))
                && [false, true].iter().all(|&g1| {
                    [false, true].iter().all(|&g2| {
                        alg.verify_assoc_law((Z2Group(g1), (Z2Group(g2), carriers()[0].clone())))
                    })
                })
        };
        let square = carriers().iter().all(|x| {
            [false, true]
                .iter()
                .all(|&g| hom.algebra_hom.verify_commutes((Z2Group(g), x.clone())))
        });
        (lawful(&source), lawful(&target), square)
    }

    // Constant-empty maps: `drop_action` is an unlawful algebra structure map;
    // `drop_map` is the object map that makes the square commute against it
    // (both sides collapse to `[]`), isolating a single failing part.
    fn drop_action((_g, _x): (Z2Group, Vec<f64>)) -> Vec<f64> {
        Vec::new()
    }
    fn drop_map(_x: Vec<f64>) -> Vec<f64> {
        Vec::new()
    }
    let lawful_from = || FAlgebra::new(vec![0.0_f64], negation_action as Z2Action);
    let lawful_to = || FAlgebra::new(vec![0.0_f64], trivial_action as Z2Action);
    let unlawful = || FAlgebra::new(vec![0.0_f64], drop_action as Z2Action);

    // Positive: abs-value hom (Example 2.6) — all three parts hold.
    assert_eq!(
        certify(&z2_monad_hom(abs_map as VecMap)),
        (true, true, true),
        "abs-value hom between lawful negation/trivial algebras is fully certified"
    );

    // Negative — square only fails: lawful algebras, non-hom map `first_coord`.
    assert_eq!(
        certify(&z2_monad_hom(first_coord as VecMap)),
        (true, true, false),
        "first_coord: both algebras lawful, but the hom square fails"
    );

    // Negative — source only fails: unlawful source, square-commuting `drop_map`,
    // lawful target.
    let bad_source: Hom = MonadAlgebraHom::new(FAlgebraHom::new(
        unlawful(),
        lawful_to(),
        drop_map as VecMap,
    ));
    assert_eq!(
        certify(&bad_source),
        (false, true, true),
        "unlawful source algebra fails part 1 alone (target lawful, square commutes)"
    );

    // Negative — target only fails: lawful source, square-commuting `drop_map`,
    // unlawful target.
    let bad_target: Hom = MonadAlgebraHom::new(FAlgebraHom::new(
        lawful_from(),
        unlawful(),
        drop_map as VecMap,
    ));
    assert_eq!(
        certify(&bad_target),
        (true, false, true),
        "unlawful target algebra fails part 2 alone (source lawful, square commutes)"
    );
}

/// The symmetric group `S₃` as index-permutation arrays: `self.0[i]` is the
/// image of `i`. Composition is right-to-left (`(g1 · g2)(i) = g1(g2(i))`),
/// matching the group-action convention `g1 ▶ (g2 ▶ x) = (g1 · g2) ▶ x`.
/// **Non-abelian** — exists to pin the writer monad's accumulation order,
/// which every `Z2` test is blind to (XOR is commutative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct S3([usize; 3]);

impl Group for S3 {
    fn compose(g1: Self, g2: Self) -> Self {
        Self([g1.0[g2.0[0]], g1.0[g2.0[1]], g1.0[g2.0[2]]])
    }

    fn identity() -> Self {
        Self([0, 1, 2])
    }
}

/// All six elements of `S₃`.
const S3_ELEMENTS: [S3; 6] = [
    S3([0, 1, 2]),
    S3([0, 2, 1]),
    S3([1, 0, 2]),
    S3([1, 2, 0]),
    S3([2, 0, 1]),
    S3([2, 1, 0]),
];

/// **Writer-monad laws over non-abelian `S₃` — exhaustive.** CDL Def 2.1 /
/// Ex 2.2.
///
/// `Z2` cannot distinguish `g · g2` from `g2 · g`, so this test pins the
/// accumulation order of `GroupActionEndo`'s `bind`/`join` against a
/// non-abelian group, exhaustively over all 6 elements (36 pairs, 216
/// triples):
///
/// - **Order**: `bind((g1, x), |x| (g2, x)) == (g1 · g2, x)` and
///   `join((g1, (g2, x))) == (g1 · g2, x)` — the documented μ; a regression
///   swapping the compose arguments fails at any non-commuting pair.
/// - **Left/right unit** via the two-sided `S3` identity.
/// - **Bind associativity** via `S3`'s associativity.
///
/// A sanity assertion first confirms `S₃` really is non-abelian, so the test
/// cannot silently weaken if the fixture changes.
#[test]
fn writer_monad_laws_pinned_by_non_abelian_s3() {
    type S3Endo = GroupActionEndo<S3>;

    // Sanity: the fixture is genuinely non-abelian.
    assert!(
        S3_ELEMENTS.iter().any(|&g1| S3_ELEMENTS
            .iter()
            .any(|&g2| S3::compose(g1, g2) != S3::compose(g2, g1))),
        "S3 fixture must be non-abelian for this test to have teeth"
    );

    let x = 7_i32;

    for &g in &S3_ELEMENTS {
        // Left unit: bind(pure(x), f) == f(x), with f(x) = (g, x).
        assert_eq!(
            S3Endo::bind(S3Endo::pure(x), |v: i32| (g, v)),
            (g, x),
            "writer monad left-unit law"
        );
        // Right unit: bind((g, x), pure) == (g, x).
        assert_eq!(
            S3Endo::bind((g, x), S3Endo::pure),
            (g, x),
            "writer monad right-unit law"
        );
    }

    for &g1 in &S3_ELEMENTS {
        for &g2 in &S3_ELEMENTS {
            // Accumulation order: g1 · g2, never g2 · g1.
            assert_eq!(
                S3Endo::bind((g1, x), |v: i32| (g2, v)),
                (S3::compose(g1, g2), x),
                "bind must accumulate the group slot as g1 · g2"
            );
            assert_eq!(
                S3Endo::join((g1, (g2, x))),
                (S3::compose(g1, g2), x),
                "join((g1, (g2, x))) must be (g1 · g2, x) — the documented μ"
            );

            for &g3 in &S3_ELEMENTS {
                // Associativity: bind(bind(m, f), h) == bind(m, |v| bind(f(v), h)).
                let f = |v: i32| (g2, v);
                let h = |v: i32| (g3, v);
                assert_eq!(
                    S3Endo::bind(S3Endo::bind((g1, x), f), h),
                    S3Endo::bind((g1, x), |v: i32| S3Endo::bind(f(v), h)),
                    "writer monad bind-associativity law"
                );
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// All four monad-algebra coherence verifiers hold for the `Z2`-negation
    /// algebra and the abs-value homomorphism across arbitrary finite samples.
    /// CDL Definition 2.3 / Example 2.6.
    #[test]
    fn monad_algebra_coherence_holds_proptest(
        g1 in any::<bool>(),
        g2 in any::<bool>(),
        x in prop::collection::vec(finite_f64(), 0..=16),
    ) {
        let alg = negation_algebra();
        let hom = z2_monad_hom(abs_map as VecMap);
        let mma = (Z2Group(g1), (Z2Group(g2), x.clone()));

        prop_assert!(alg.verify_unit_law(x.clone()), "unit law");
        prop_assert!(alg.verify_assoc_law(mma.clone()), "assoc law");
        prop_assert!(hom.verify_unit_coherence(x), "unit coherence");
        prop_assert!(hom.verify_mult_coherence(mma), "mult coherence");
    }
}
