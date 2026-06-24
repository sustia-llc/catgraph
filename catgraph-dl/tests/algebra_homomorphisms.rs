//! Phase DL-2 Agent D — F-algebra/coalgebra homomorphism acceptance tests.
//!
//! CDL Definitions 2.5 and B.2; CDL Example 2.6 (the Geometric-Deep-
//! Learning recovery). These tests exercise the bodies landed in Phase
//! DL-2 for:
//!
//! - [`FAlgebraHom::verify_commutes`] — the F-algebra commuting square
//!   `f ∘ a = b ∘ F(f)`.
//! - [`FCoalgebraHom::verify_commutes`] — the dual coalgebra square.
//! - [`MonadAlgebraHom`] — construction smoke (unit + multiplication
//!   coherence is a documented obligation, not machine-checked at DL-2).
//!
//! Each test consolidates several related assertions in one function (per
//! project TDD convention — quality over quantity).
//!
//! ## CDL Example 2.6 — Geometric Deep Learning recovery
//!
//! Two of the five tests below exhibit the central GDL claim of the CDL
//! paper: when `F = G × −` is the group-action endofunctor and `b` is
//! the *trivial* action `g ▶ y = y`, an F-algebra homomorphism from the
//! `g ▶ x = ±x` *negation* action to the trivial action is exactly a
//! `Z2`-**invariant** map `f(g ▶ x) = f(x)`. This is the simplest
//! Geometric-Deep-Learning shape.
//!
//! - The pointwise absolute value `f(x) = x.iter().map(|v| v.abs())`
//!   **is** invariant — `|−x_i| = |x_i|` — and `verify_commutes`
//!   returns `true`.
//! - The first-coordinate projection `f(x) = vec![x[0]]` between the
//!   *same* (negation, trivial) algebras is **not** invariant —
//!   `(−x)[0] ≠ x[0]` in general — and `verify_commutes` returns
//!   `false`.
//!
//! Together they confirm that `verify_commutes` *can* distinguish
//! homomorphisms (= equivariant / invariant maps) from arbitrary set
//! maps — the CDL Example 2.6 claim, reified in code.

#![allow(
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::needless_pass_by_value,
    clippy::doc_markdown,
    reason = "Test file. type_complexity: the FAlgebraHom<…6 type params…> spelling is exactly what callers see — a `type` alias would still need every parameter and obscures the CDL anchor. items_after_statements: helper `fn`s nested inside tests are scoped to that test by intent. similar_names: `coalg_a2`/`coalg_b2`/`coalg_a3`/`coalg_b3` are the standard `(A, a)` / `(B, b)` algebra names from CDL §2 plus a numeric suffix per fresh construction; renaming would obscure the math. needless_pass_by_value: `fn(Vec<f64>) -> Vec<f64>` is forced by the FAlgebraHom map type. doc_markdown: backtick-wrapping `MonadAlgebraHom` is fine but every doc string already wraps the relevant Rust types; hand-wrapping each prose mention is busywork. Same precedent as Agent A's `para/morphism.rs` module-level allows."
)]

use catgraph_dl::algebra::{
    EndoFunctor, FAlgebra, FAlgebraHom, FCoalgebra, FCoalgebraHom, GroupActionEndo, MonadAlgebra,
    MonadAlgebraHom, Z2Group,
};

/// Type alias for the `Z2` group-action endofunctor used throughout.
type Z2Endo = GroupActionEndo<Z2Group>;

/// The canonical `Z2`-action on `Vec<f64>` by negation.
///
/// `g ▶ x = if g.0 { −x } else { x }` (pointwise). Source algebra in
/// every GDL-recovery test below.
fn negation_action((g, x): (Z2Group, Vec<f64>)) -> Vec<f64> {
    if g.0 {
        x.into_iter().map(|v| -v).collect()
    } else {
        x
    }
}

/// The trivial `Z2`-action on `Vec<f64>` — every group element acts as
/// the identity.
///
/// `g ▶ y = y` for all `g`. Target algebra of the GDL-invariance shape.
fn trivial_action((_g, y): (Z2Group, Vec<f64>)) -> Vec<f64> {
    y
}

/// **Identity is a homomorphism.** CDL Definition 2.5 corollary.
///
/// For any F-algebra `(A, a)`, the identity map `id_A : A → A` makes the
/// square trivially commute (both legs are `a`). This is the categorical
/// "every algebra has an endomorphism by itself".
///
/// Asserts `verify_commutes` returns `true` for the identity hom on the
/// `Z2`-negation algebra `(Vec<f64>, negation_action)`, sampled at
/// several `(g, x)` pairs. Bonus assertion: re-checks that `Z2Endo`'s
/// `fmap` is well-formed (preserves the group element across the
/// re-export at `catgraph_dl::algebra::EndoFunctor`).
#[test]
fn identity_is_an_f_algebra_homomorphism() {
    let alg_a: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0, 3.0], negation_action);
    let alg_b: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0, 3.0], negation_action);

    let id_hom: FAlgebraHom<
        Z2Endo,
        Vec<f64>,
        Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn(Vec<f64>) -> Vec<f64>,
    > = FAlgebraHom::new(alg_a, alg_b, |x| x);

    // Sweep representative samples — both group elements, several vectors.
    for (g, x) in [
        (Z2Group(false), vec![1.0_f64, 2.0, 3.0]),
        (Z2Group(true), vec![1.0_f64, 2.0, 3.0]),
        (Z2Group(false), vec![]),
        (Z2Group(true), vec![]),
        (Z2Group(true), vec![-7.5_f64, 0.0, 4.25]),
    ] {
        assert!(
            id_hom.verify_commutes((g, x.clone())),
            "identity must satisfy the commuting square at (g={g:?}, x={x:?})"
        );
    }

    // fmap sanity through the public re-export.
    let lifted: <Z2Endo as EndoFunctor>::Apply<Vec<f64>> =
        Z2Endo::fmap((Z2Group(true), vec![1.0_f64, 2.0]), |x| x);
    assert_eq!(lifted, (Z2Group(true), vec![1.0_f64, 2.0]));
}

/// **Non-equivariant map fails the commuting square.** CDL Example 2.6,
/// negative case.
///
/// `f(x) = vec![x[0]]` is the first-coordinate projection (wrapped in a
/// singleton `Vec<f64>` so source and target carriers agree). Between
/// the algebras
///
/// - source `(Vec<f64>, negation_action)`,
/// - target `(Vec<f64>, trivial_action)`,
///
/// `f` is **not** a homomorphism. Concretely, for `g = true`:
///
/// ```text
/// LHS: f(g ▶ x) = f(−x) = vec![−x[0]]
/// RHS: g ▶ f(x) = trivial(true, vec![x[0]]) = vec![x[0]]
/// ```
///
/// — these differ whenever `x[0] ≠ 0`. Asserts `verify_commutes` returns
/// `false` on samples with non-zero first coordinate, *and* returns
/// `true` on the trivial diagonal `g = false` and on inputs with
/// `x[0] = 0` (where the failure happens to vanish).
///
/// The "happens-to-vanish" assertion is the key dual: it shows that
/// `verify_commutes` is a *necessary* but not sufficient check —
/// individual samples may satisfy the square without the full map being
/// a homomorphism. The acceptance harness in CDL §3 (and our DL-3 work)
/// would lift this to property-tested form.
#[test]
fn non_equivariant_projection_fails_commuting_square() {
    let source: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, 2.0, 3.0], negation_action);
    let target: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64], trivial_action);

    fn first_coord(x: Vec<f64>) -> Vec<f64> {
        vec![x[0]]
    }
    let hom: FAlgebraHom<
        Z2Endo,
        Vec<f64>,
        Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn(Vec<f64>) -> Vec<f64>,
    > = FAlgebraHom::new(source, target, first_coord);

    // Failing samples — non-zero first coordinate under the non-trivial
    // group element. `verify_commutes` MUST report `false`.
    for (g, x) in [
        (Z2Group(true), vec![1.0_f64, 2.0, 3.0]),
        (Z2Group(true), vec![5.0_f64, -7.0]),
        (Z2Group(true), vec![-1.0_f64, 0.0]),
    ] {
        assert!(
            !hom.verify_commutes((g, x.clone())),
            "non-equivariant projection MUST fail the square at (g={g:?}, x={x:?}) — x[0]={}",
            x[0]
        );
    }

    // Diagonal: g = false makes any map satisfy the square (no negation
    // happens). This is the well-known "passive" zero of the equivariance
    // failure space.
    for x in [
        vec![1.0_f64, 2.0, 3.0],
        vec![-3.0_f64, 4.0],
        vec![100.0_f64],
    ] {
        assert!(
            hom.verify_commutes((Z2Group(false), x.clone())),
            "identity group element makes any hom commute trivially at x={x:?}"
        );
    }

    // Incidental: g = true with x[0] = 0 also passes — the failure
    // vanishes coordinate-wise. Bit-exact zero so the f64 comparison is
    // sound.
    for x in [vec![0.0_f64, 1.0], vec![0.0_f64, -7.0]] {
        assert!(
            hom.verify_commutes((Z2Group(true), x.clone())),
            "incidentally-commuting sample (x[0] = 0) at x={x:?}"
        );
    }
}

/// **Equivariant (invariant) map satisfies the commuting square.** CDL
/// Example 2.6, **positive case — the GDL recovery**.
///
/// The pointwise absolute value `f(x) = x.iter().map(|v| v.abs())` is
/// `Z2`-**invariant** under negation: `|−x_i| = |x_i|`. As an F-algebra
/// homomorphism between
///
/// - source `(Vec<f64>, negation_action)`,
/// - target `(Vec<f64>, trivial_action)`,
///
/// the commuting square reads, for every `g`:
///
/// ```text
/// LHS: f(g ▶ x) = | g ▶ x | = |x|       (absolute value erases the sign)
/// RHS: g ▶ f(x) = trivial(g, |x|) = |x|  (trivial action ignores g)
/// ```
///
/// — both equal `|x|`, so the square commutes. This is the categorical
/// recovery of the GDL claim that "an invariant feature map is an
/// F-algebra homomorphism between the original action and the trivial
/// action". Asserts on a sweep of samples covering both group elements,
/// signs, zeros, and the empty vector.
///
/// Bonus: also asserts the **constant-zero** map `f(x) = vec![]` is
/// invariant (vacuously — the empty vector is `Z2`-fixed).
#[test]
fn absolute_value_is_z2_equivariant_homomorphism() {
    let neg_alg_src: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0, 3.5], negation_action);
    let triv_alg_dst: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, 2.0, 3.5], trivial_action);

    fn abs_map(x: Vec<f64>) -> Vec<f64> {
        x.into_iter().map(f64::abs).collect()
    }
    let abs_hom: FAlgebraHom<
        Z2Endo,
        Vec<f64>,
        Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn(Vec<f64>) -> Vec<f64>,
    > = FAlgebraHom::new(neg_alg_src, triv_alg_dst, abs_map);

    for (g, x) in [
        (Z2Group(false), vec![1.0_f64, -2.0, 3.5]),
        (Z2Group(true), vec![1.0_f64, -2.0, 3.5]),
        (Z2Group(true), vec![-7.0_f64, 0.0, 4.25]),
        (Z2Group(false), vec![]),
        (Z2Group(true), vec![]),
        (Z2Group(true), vec![-100.0_f64, 100.0, -1.0, 1.0]),
        (Z2Group(false), vec![0.0_f64, 0.0, 0.0]),
    ] {
        assert!(
            abs_hom.verify_commutes((g, x.clone())),
            "abs map is Z2-invariant — commuting square must hold at (g={g:?}, x={x:?})"
        );
    }

    // Bonus: empty-output map `f(x) = vec![]` is vacuously invariant.
    let neg_alg_src2: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0], negation_action);
    let triv_alg_dst2: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![], trivial_action);
    fn empty_map(_x: Vec<f64>) -> Vec<f64> {
        vec![]
    }
    let empty_hom: FAlgebraHom<
        Z2Endo,
        Vec<f64>,
        Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn(Vec<f64>) -> Vec<f64>,
    > = FAlgebraHom::new(neg_alg_src2, triv_alg_dst2, empty_map);

    for (g, x) in [
        (Z2Group(false), vec![1.0_f64, -2.0]),
        (Z2Group(true), vec![1.0_f64, -2.0]),
        (Z2Group(true), vec![]),
    ] {
        assert!(
            empty_hom.verify_commutes((g, x.clone())),
            "empty-output map is vacuously Z2-invariant at (g={g:?}, x={x:?})"
        );
    }
}

/// **F-coalgebra homomorphism — identity smoke.** CDL Definition B.2 dual.
///
/// Constructs a trivial F-coalgebra `(u32, a : u32 → (Z2Group, u32))`
/// where `a(n) = (Z2Group(false), n)` (constant identity-group output),
/// and the identity map `id : u32 → u32` as its endo-homomorphism. The
/// dual square `F(f) ∘ a = b ∘ f` collapses to `a = a` since `F(id) = id`
/// — `verify_commutes` must return `true` on every sample.
///
/// Bonus: a non-identity `f(n) = n + 1` paired with the same structure
/// map on both sides also commutes — `F(f)(a(n)) = (false, n+1) =
/// b(f(n))` — exercising the dual square with a non-trivial morphism.
#[test]
fn coalgebra_hom_identity_smoke() {
    fn structure_map_a(n: u32) -> (Z2Group, u32) {
        (Z2Group(false), n)
    }

    let coalg_a: FCoalgebra<Z2Endo, u32, fn(u32) -> (Z2Group, u32)> =
        FCoalgebra::new(0_u32, structure_map_a);
    let coalg_b: FCoalgebra<Z2Endo, u32, fn(u32) -> (Z2Group, u32)> =
        FCoalgebra::new(0_u32, structure_map_a);

    let id_hom: FCoalgebraHom<
        Z2Endo,
        u32,
        u32,
        fn(u32) -> (Z2Group, u32),
        fn(u32) -> (Z2Group, u32),
        fn(u32) -> u32,
    > = FCoalgebraHom::new(coalg_a, coalg_b, |n| n);

    for n in [0_u32, 1, 7, 42, 1024] {
        assert!(
            id_hom.verify_commutes(n),
            "identity coalgebra hom must satisfy the dual square at n = {n}"
        );
    }

    // `shift(n) = n + 1` paired with structure maps that emit the same
    // group element on both sides — also commutes by direct calculation.
    fn shift(n: u32) -> u32 {
        n + 1
    }
    let coalg_a2: FCoalgebra<Z2Endo, u32, fn(u32) -> (Z2Group, u32)> =
        FCoalgebra::new(0_u32, structure_map_a);
    let coalg_b2: FCoalgebra<Z2Endo, u32, fn(u32) -> (Z2Group, u32)> =
        FCoalgebra::new(0_u32, structure_map_a);
    let shift_hom: FCoalgebraHom<
        Z2Endo,
        u32,
        u32,
        fn(u32) -> (Z2Group, u32),
        fn(u32) -> (Z2Group, u32),
        fn(u32) -> u32,
    > = FCoalgebraHom::new(coalg_a2, coalg_b2, shift);

    for n in [0_u32, 1, 7, 42] {
        assert!(
            shift_hom.verify_commutes(n),
            "shift coalgebra hom commutes when both structure maps emit the same group element at n = {n}"
        );
    }

    // Non-commuting sample: structure maps that emit *different* group
    // elements on the two algebras — square fails.
    fn structure_map_b(n: u32) -> (Z2Group, u32) {
        (Z2Group(true), n)
    }
    let coalg_a3: FCoalgebra<Z2Endo, u32, fn(u32) -> (Z2Group, u32)> =
        FCoalgebra::new(0_u32, structure_map_a);
    let coalg_b3: FCoalgebra<Z2Endo, u32, fn(u32) -> (Z2Group, u32)> =
        FCoalgebra::new(0_u32, structure_map_b);
    let mismatch_hom: FCoalgebraHom<
        Z2Endo,
        u32,
        u32,
        fn(u32) -> (Z2Group, u32),
        fn(u32) -> (Z2Group, u32),
        fn(u32) -> u32,
    > = FCoalgebraHom::new(coalg_a3, coalg_b3, |n| n);

    for n in [0_u32, 1, 7, 42] {
        assert!(
            !mismatch_hom.verify_commutes(n),
            "mismatched structure-map group elements MUST break the dual square at n = {n}"
        );
    }
}

/// **MonadAlgebraHom — construction smoke.** CDL Definition 2.3.
///
/// Phase DL-2 does not machine-check unit + multiplication coherence
/// (documented obligation on `MonadAlgebraHom::new`). This test
/// instantiates a `MonadAlgebraHom` for the group-action monad
/// `Z2 × −` on `Vec<f64>` and:
///
/// 1. Confirms construction via the public surface
///    (`MonadAlgebra::new(FAlgebra::new(...))` and
///    `MonadAlgebraHom::new(FAlgebraHom::new(...))`).
/// 2. Calls `verify_commutes` through the embedded `algebra_hom`
///    field — exercising the only law that *is* machine-checked at DL-2.
/// 3. Confirms the wrapped `FAlgebra` carrier is preserved through
///    construction.
#[test]
fn monad_algebra_hom_construction_and_commuting_square() {
    let alg_a: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0], negation_action);
    let monad_alg_a: MonadAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        MonadAlgebra::new(alg_a);
    assert_eq!(monad_alg_a.algebra.carrier, vec![1.0_f64, -2.0]);

    // Build two fresh underlying FAlgebras (we already moved `alg_a` into
    // `monad_alg_a`). Both use negation_action — so the identity hom
    // satisfies the F-algebra commuting square.
    let from: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0], negation_action);
    let to: FAlgebra<Z2Endo, Vec<f64>, fn((Z2Group, Vec<f64>)) -> Vec<f64>> =
        FAlgebra::new(vec![1.0_f64, -2.0], negation_action);
    let f_hom: FAlgebraHom<
        Z2Endo,
        Vec<f64>,
        Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn(Vec<f64>) -> Vec<f64>,
    > = FAlgebraHom::new(from, to, |x| x);

    let monad_hom: MonadAlgebraHom<
        Z2Endo,
        Vec<f64>,
        Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn((Z2Group, Vec<f64>)) -> Vec<f64>,
        fn(Vec<f64>) -> Vec<f64>,
    > = MonadAlgebraHom::new(f_hom);

    // Identity always satisfies the F-algebra square (the only law
    // machine-checked at DL-2). Sweep both group elements.
    for (g, x) in [
        (Z2Group(false), vec![1.0_f64, -2.0, 3.0]),
        (Z2Group(true), vec![1.0_f64, -2.0, 3.0]),
        (Z2Group(true), vec![]),
    ] {
        assert!(
            monad_hom.algebra_hom.verify_commutes((g, x.clone())),
            "MonadAlgebraHom (identity, neg, neg) F-algebra square must commute at (g={g:?}, x={x:?})"
        );
    }
}
