//! Acceptance tests for the **typed-magnitude valuation surface** (#211, EQ4).
//!
//! Three layers, each exercised through the public API only:
//!
//! - **T1** — `CoalitionEvaluator::role_shares`: hand-checked per-role values on
//!   the crate's chain fixture, the sum-vs-`base_value()` identity (relative
//!   tolerance — bucketed sums re-associate the row-major accumulation, so
//!   bit-identity is not claimed), the role-mixed skeletal class being *flagged*
//!   rather than split, and the two error/absence cases.
//! - **T2** — `modulate` (hand-checked, including an asymmetric `ρ`) feeding the
//!   ordinary evaluator unchanged, and `role_grid`'s construction-carried
//!   factorization certificate checked against an actual grid evaluation for two
//!   `(ρ, σ, t)` combinations — one of which has a non-trivial closure (a
//!   triangle-inequality-violating fiber pair that the max-product closure
//!   tightens).
//! - **T3** — `ChannelCouplings::collapse`: hand-check, `θ = e_c` exactness, the
//!   monoid-homomorphism property `|v ⊙ w|_θ = |v|_θ·|w|_θ`, the `0^0 = 1` edge,
//!   the rejections, and the collapsed output feeding the evaluator.

use catgraph_magnitude::{
    ChannelCouplings, CoalitionEvaluator, RoleModulation, coalition_magnitude_from_couplings,
    coalition_value, modulate, role_grid,
};
use proptest::prelude::*;

/// Relative tolerance for float-re-association comparisons. Two arithmetic
/// routes to the same real number (bucketed vs row-major sums; grid closure vs
/// per-factor closure) differ only by re-association, which stays far below this.
const REL_TOL: f64 = 1e-9;

fn rel_close(a: f64, b: f64) -> bool {
    (a - b).abs() <= REL_TOL * a.abs().max(b.abs()).max(1.0)
}

/// All indices `0..n`, the "every agent is a member" member list.
fn all(n: usize) -> Vec<usize> {
    (0..n).collect()
}

// ===========================================================================
// T1 — role-share diagnostics
// ===========================================================================

/// The chain fixture `a → b (0.7), b → c (0.5)` has closure `a → c = 0.35`, so
/// its (upper-unitriangular) `ζ` at `t = 1` inverts to a weighting of
/// `[1 − 0.7 + 0, 1 − 0.5, 1] = [0.3, 0.5, 1.0]` and `Mag = 1.8` — the value
/// pinned by `tests/coalition_consumer.rs`. No pair is perfectly coupled, so
/// every member is its own skeletal class and each role's share is a hand-
/// computable sum of those three weights.
#[test]
fn role_shares_exact_partition_hand_checked() {
    let agents = ["a", "b", "c"];
    let couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5)];
    let members = [0usize, 1, 2];
    let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
    assert!(rel_close(ev.base_value(), 1.8), "fixture Mag = 1.8");

    // Partition A: {a, b} = role 0, {c} = role 1.
    let a = ev.role_shares(&[0, 0, 1]).unwrap();
    assert!(a.is_exact(), "no class spans two roles");
    assert!(a.mixed_classes().is_empty());
    assert_eq!(a.n_roles(), 2);
    assert!(rel_close(a.share(0).unwrap(), 0.8), "0.3 + 0.5");
    assert!(rel_close(a.share(1).unwrap(), 1.0));
    assert!(
        rel_close(a.share(0).unwrap() + a.share(1).unwrap(), ev.base_value()),
        "shares partition base_value (up to float re-association)"
    );

    // Partition B: the same weights bucketed differently — {a, c} vs {b}.
    // Role ids are caller-owned and need not be dense: 4 and 9 work.
    let b = ev.role_shares(&[4, 9, 4]).unwrap();
    assert!(b.is_exact());
    assert_eq!(b.n_roles(), 2);
    assert!(rel_close(b.share(4).unwrap(), 1.3), "0.3 + 1.0");
    assert!(rel_close(b.share(9).unwrap(), 0.5));

    // Partition C: one role over everything recovers the whole magnitude.
    let c = ev.role_shares(&[7, 7, 7]).unwrap();
    assert!(c.is_exact());
    assert_eq!(c.n_roles(), 1);
    assert!(rel_close(c.share(7).unwrap(), ev.base_value()));
}

/// A perfectly-coupled pair collapses into one skeletal class. When the two
/// members of that class carry *different* roles, the class weight belongs to
/// neither: it must be reported, never split.
///
/// Fixture: `a → b (0.5)`, `b ⇄ c (1.0)`. Members `b, c` merge (rep = local
/// index 1), leaving the skeletal space `{a} → {b,c}` at coupling `0.5`, so the
/// weighting is `[0.5, 1.0]` and `Mag = 1.5`.
#[test]
fn role_shares_mixed_class_is_flagged_not_split() {
    let agents = ["a", "b", "c"];
    let couplings = [(0usize, 1usize, 0.5f64), (1, 2, 1.0), (2, 1, 1.0)];
    let members = [0usize, 1, 2];
    let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
    assert!(rel_close(ev.base_value(), 1.5), "skeletal Mag = 0.5 + 1.0");

    // Roles cut ACROSS the {b, c} class.
    let shares = ev.role_shares(&[0, 0, 1]).unwrap();
    assert!(!shares.is_exact(), "the {{b,c}} class spans roles 0 and 1");
    assert_eq!(shares.mixed_classes().len(), 1);
    let mixed = &shares.mixed_classes()[0];
    assert_eq!(mixed.rep(), 1, "class rep is member-local index 1 (= b)");
    assert_eq!(mixed.roles(), &[0, 1], "the spanned roles, ascending");

    // The mixed class's weight (1.0) is attributed to NO role: role 0 keeps only
    // {a}'s 0.5, and role 1 — present but pure nowhere — is Some(0.0), not None.
    assert!(rel_close(shares.share(0).unwrap(), 0.5));
    assert_eq!(shares.share(1), Some(0.0));
    assert_eq!(shares.n_roles(), 2);
    assert!(
        shares.share(0).unwrap() + shares.share(1).unwrap() < ev.base_value(),
        "a mixed class is withheld from the decomposition, so shares under-sum"
    );

    // Roles ALIGNED with the class are exact again on the same coalition.
    let aligned = ev.role_shares(&[0, 1, 1]).unwrap();
    assert!(aligned.is_exact());
    assert!(rel_close(aligned.share(0).unwrap(), 0.5));
    assert!(rel_close(aligned.share(1).unwrap(), 1.0));
    assert!(rel_close(
        aligned.share(0).unwrap() + aligned.share(1).unwrap(),
        ev.base_value()
    ));
}

/// `roles` is member-local, so its length must match the member count exactly;
/// and an id that was never supplied is `None`, distinct from a supplied role
/// that happens to have accumulated `0.0`.
#[test]
fn role_shares_length_mismatch_and_unknown_role() {
    let agents = ["a", "b", "c", "d"];
    let couplings = [(0usize, 1usize, 0.7f64), (1, 2, 0.5)];
    // Coalition of THREE of the four agents — the length check is against the
    // member count, not the agent count.
    let ev = CoalitionEvaluator::new(&agents, &couplings, &[0, 1, 2], 1.0).unwrap();
    assert_eq!(ev.members().len(), 3);

    assert!(ev.role_shares(&[0, 1]).is_err(), "too few roles");
    assert!(ev.role_shares(&[0, 1, 0, 1]).is_err(), "too many roles");
    assert!(
        ev.role_shares(&[]).is_err(),
        "empty roles on a non-empty coalition"
    );

    let shares = ev.role_shares(&[0, 0, 1]).unwrap();
    assert_eq!(shares.share(2), None, "role id never supplied");
    assert_eq!(shares.share(usize::MAX), None);
    assert!(shares.share(0).is_some());
}

// ===========================================================================
// T2 — role-modulated couplings
// ===========================================================================

/// `π′ = ρ(r_from, r_to)·π`, hand-checked on a 3-agent example with an
/// **asymmetric** `ρ` so the two directions of the same pair get different
/// factors.
#[test]
fn modulate_hand_checked_with_asymmetric_rho() {
    let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.25, 1.0]]).unwrap();
    let roles = [0usize, 1, 1]; // agent-indexed
    let couplings = [(0usize, 1usize, 0.8f64), (1, 2, 0.4), (1, 0, 0.6)];

    let out = modulate(&couplings, &roles, &rho).unwrap();
    assert_eq!(
        out.couplings(),
        &[
            (0, 1, 0.5 * 0.8),  // role 0 → role 1
            (1, 2, 1.0 * 0.4),  // role 1 → role 1 (unit diagonal)
            (1, 0, 0.25 * 0.6), // role 1 → role 0 — the OTHER direction
        ],
        "one output per input, in input order"
    );

    // Errors: an out-of-range role, an uncovered agent index, a bad probability.
    assert!(
        modulate(&couplings, &[0, 1, 2], &rho).is_err(),
        "role 2 out of range for a 2-role modulation"
    );
    assert!(
        modulate(&couplings, &[0, 1], &rho).is_err(),
        "agent 2 has no role"
    );
    assert!(
        modulate(&[(0, 1, 1.5)], &roles, &rho).is_err(),
        "probability outside [0, 1]"
    );
}

/// The modulated triples are ordinary couplings: they construct a
/// `CoalitionEvaluator`, answer `value_with`, and agree with the untyped entry
/// points on the same table. Modulation can only *lengthen* distances (`ρ ≤ 1`),
/// so the modulated coalition is at least as diverse as the unmodulated one.
#[test]
fn modulated_couplings_feed_the_evaluator_unchanged() {
    let agents = ["a", "b", "c", "x"];
    let couplings = [
        (0usize, 1usize, 0.7f64),
        (1, 2, 0.5),
        (2, 3, 0.2),
        (3, 2, 0.2),
    ];
    let roles = [0usize, 0, 1, 1];
    let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.25, 1.0]]).unwrap();
    let modulated = modulate(&couplings, &roles, &rho).unwrap();

    let members = [0usize, 1, 2];
    let ev = CoalitionEvaluator::new(&agents, modulated.couplings(), &members, 1.0).unwrap();

    // base_value is the contract-point-1 bit-identical mirror of a fresh eval.
    let fresh = coalition_value(&agents, modulated.couplings(), &members).unwrap();
    assert_eq!(ev.base_value(), fresh, "#31 contract point 1 still holds");

    // The incremental join still matches a fresh evaluation on S ∪ {x}.
    let with_x = ev.value_with(3).unwrap();
    let fresh_with_x = coalition_value(&agents, modulated.couplings(), &[0, 1, 2, 3]).unwrap();
    assert!(
        rel_close(with_x, fresh_with_x),
        "{with_x} vs {fresh_with_x}"
    );

    // ρ ≤ 1 weakens couplings, i.e. lengthens d = −ln π ⇒ more diversity.
    let unmodulated = coalition_value(&agents, &couplings, &members).unwrap();
    assert!(
        fresh >= unmodulated - REL_TOL,
        "modulated {fresh} should not be below unmodulated {unmodulated}"
    );

    // The reporting surface is unaffected too.
    let report = ev.value_with_report(3).unwrap();
    assert_eq!(report.value(), with_x);
}

/// The role-grid certificate: an actual evaluation of `RoleGrid::couplings()`
/// through the ordinary entry point equals `proof(t).expected_magnitude()`
/// within relative tolerance, for two `(ρ, σ, t)` combinations.
///
/// Combination 1 has a **non-trivial closure**: the fiber's `σ(0,2) = 0.1` is
/// beaten by the two-hop `σ(0,1)·σ(1,2) = 0.81`, so the max-product closure
/// genuinely tightens the table — on both factors *and* on the grid. Any
/// factorization argument that quietly assumed a pre-closed table would fail
/// here.
///
/// Combination 2 uses an **asymmetric** `ρ` and `σ` at `t = 2`, exercising the
/// non-symmetric route at a non-canonical scale.
#[test]
fn role_grid_factorization_certificate() {
    let combos: Vec<(&str, RoleModulation, RoleModulation, f64)> = vec![
        (
            "symmetric roles × triangle-violating fiber, t = 1",
            RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap(),
            RoleModulation::new(vec![
                vec![1.0, 0.9, 0.1], // 0.1 < 0.9·0.9 = 0.81 — the closure tightens it
                vec![0.0, 1.0, 0.9],
                vec![0.0, 0.0, 1.0],
            ])
            .unwrap(),
            1.0,
        ),
        (
            "asymmetric roles × asymmetric fiber, t = 2",
            RoleModulation::new(vec![vec![1.0, 0.3], vec![0.7, 1.0]]).unwrap(),
            RoleModulation::new(vec![vec![1.0, 0.5], vec![0.4, 1.0]]).unwrap(),
            2.0,
        ),
    ];

    for (label, rho, sigma, t) in combos {
        let grid = role_grid(&rho, &sigma).unwrap();
        assert_eq!(grid.n_agents(), rho.n_roles() * sigma.n_roles());

        let agents = all(grid.n_agents());
        let actual =
            coalition_magnitude_from_couplings(&agents, grid.couplings(), &agents, t).unwrap();

        let proof = grid.proof(t).unwrap();
        assert_eq!(proof.t(), t);
        assert!(
            rel_close(
                proof.expected_magnitude(),
                proof.role_magnitude() * proof.fiber_magnitude()
            ),
            "expected_magnitude is the product of the factors"
        );
        assert!(
            rel_close(actual, proof.expected_magnitude()),
            "{label}: grid {actual} vs certificate {} (|ρ| = {}, |σ| = {})",
            proof.expected_magnitude(),
            proof.role_magnitude(),
            proof.fiber_magnitude()
        );
    }
}

/// The closure really does tighten combination 1's fiber — a guard so the test
/// above cannot silently degrade into an already-closed fixture.
///
/// The fiber declares `σ(0,2) = 0.1` while the two-hop `σ(0,1)·σ(1,2) = 0.81`
/// beats it, so the closure overwrites the declared entry. Evidence: deleting
/// the direct edge changes nothing (the closure supplies `0.81` either way),
/// whereas a direct edge *above* `0.81` does move the number.
#[test]
fn role_grid_fixture_has_a_nontrivial_closure() {
    let sigma = RoleModulation::new(vec![
        vec![1.0, 0.9, 0.1],
        vec![0.0, 1.0, 0.9],
        vec![0.0, 0.0, 1.0],
    ])
    .unwrap();
    let one_role = RoleModulation::new(vec![vec![1.0]]).unwrap();
    let grid = role_grid(&one_role, &sigma).unwrap();

    // Closed ζ = [[1, 0.9, 0.81], [0, 1, 0.9], [0, 0, 1]] ⇒ weighting
    // [0.1, 0.1, 1.0] ⇒ Mag = 1.2.
    let fiber_mag = grid.proof(1.0).unwrap().fiber_magnitude();
    assert!(
        rel_close(fiber_mag, 1.2),
        "closed fiber magnitude is 1.2, got {fiber_mag}"
    );

    let agents = all(3);
    let without_direct =
        coalition_magnitude_from_couplings(&agents, &[(0, 1, 0.9), (1, 2, 0.9)], &agents, 1.0)
            .unwrap();
    assert!(
        rel_close(without_direct, fiber_mag),
        "the declared 0 → 2 = 0.1 is dominated by the closure's 0.81"
    );

    let stronger = coalition_magnitude_from_couplings(
        &agents,
        &[(0, 1, 0.9), (1, 2, 0.9), (0, 2, 0.95)],
        &agents,
        1.0,
    )
    .unwrap();
    assert!(
        (stronger - fiber_mag).abs() > 1e-6,
        "a direct edge above the two-hop product does move the magnitude: \
         {stronger} vs {fiber_mag}"
    );
}

/// Both factors must have an exact unit diagonal — the precondition that makes
/// `closure(ρ ⊗ σ) = closure(ρ) ⊗ closure(σ)` true under the engine's forced
/// unit diagonal.
#[test]
fn role_grid_rejects_a_non_unit_diagonal_factor() {
    let good = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5, 1.0]]).unwrap();
    let bad = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.5, 0.999]]).unwrap();

    let err = role_grid(&good, &bad).unwrap_err().to_string();
    assert!(
        err.contains("fiber"),
        "error names the offending factor: {err}"
    );
    assert!(role_grid(&bad, &good).is_err(), "role factor checked too");
    assert!(role_grid(&good, &good).is_ok());
}

/// An `n × n` `RoleModulation` with an exact unit diagonal, reading its
/// off-diagonal entries from `off` in row-major order.
fn unit_diagonal_factor(n: usize, off: &[f64]) -> RoleModulation {
    let mut rows = vec![vec![0.0_f64; n]; n];
    let mut k = 0;
    for (i, row) in rows.iter_mut().enumerate() {
        for (j, entry) in row.iter_mut().enumerate() {
            if i == j {
                *entry = 1.0;
            } else {
                *entry = off[k];
                k += 1;
            }
        }
    }
    RoleModulation::new(rows).expect("unit diagonal, entries in [0, 1]")
}

proptest! {
    /// The factorization certificate over the product-coupling family rather
    /// than two hand-built combinations: for unit-diagonal factors with
    /// off-diagonal entries in `[0.0, 0.9]` at `n_roles, n_fiber ∈ {2, 3}`, an
    /// evaluation of `RoleGrid::couplings()` through the ordinary entry point
    /// equals `proof(t).expected_magnitude()` within `REL_TOL`. The three `t`
    /// comparisons per case are counted, so an `Err` from either route fails
    /// the case instead of skipping.
    #[test]
    fn role_grid_certificate_over_product_couplings(
        n_roles in 2usize..=3,
        n_fiber in 2usize..=3,
        rho_off in prop::collection::vec(0.0f64..=0.9, 6),
        sigma_off in prop::collection::vec(0.0f64..=0.9, 6),
    ) {
        let rho = unit_diagonal_factor(n_roles, &rho_off);
        let sigma = unit_diagonal_factor(n_fiber, &sigma_off);
        let grid = role_grid(&rho, &sigma).expect("both diagonals are exactly 1.0");
        prop_assert_eq!(grid.n_agents(), n_roles * n_fiber);

        let agents = all(grid.n_agents());
        let mut compared = 0usize;
        for &t in &[1.0_f64, 1.5, 2.0] {
            let actual =
                coalition_magnitude_from_couplings(&agents, grid.couplings(), &agents, t);
            let proof = grid.proof(t);
            let (Ok(actual), Ok(proof)) = (actual, proof) else {
                continue;
            };
            compared += 1;
            prop_assert!(
                rel_close(actual, proof.expected_magnitude()),
                "t={t}: grid {actual} vs certificate {} (|ρ| = {}, |σ| = {})",
                proof.expected_magnitude(),
                proof.role_magnitude(),
                proof.fiber_magnitude()
            );
        }
        prop_assert_eq!(
            compared, 3,
            "every t must evaluate on both routes (n_roles={}, n_fiber={})",
            n_roles, n_fiber
        );
    }
}

// ===========================================================================
// T3 — channel-valued couplings + declared collapse
// ===========================================================================

/// `|v|_θ = Π_c v_c^{θ_c}`, hand-checked; `θ = e_c` recovers channel `c`
/// **exactly** (`v^1 = v`, `w^0 = 1`, both exact in IEEE 754).
#[test]
fn collapse_hand_checked_and_unit_theta_is_exact() {
    let mut cc = ChannelCouplings::new(3).unwrap();
    cc.set(0, 1, vec![0.5, 0.8, 0.25]).unwrap();
    cc.set(1, 0, vec![0.2, 1.0, 0.5]).unwrap();
    assert_eq!(cc.n_channels(), 3);

    // Output is in ascending (from, to) order — reproducible.
    let e0 = cc.collapse(&[1.0, 0.0, 0.0]).unwrap();
    assert_eq!(e0, vec![(0, 1, 0.5), (1, 0, 0.2)], "θ = e_0 is exact");
    assert_eq!(
        cc.collapse(&[0.0, 1.0, 0.0]).unwrap(),
        vec![(0, 1, 0.8), (1, 0, 1.0)],
        "θ = e_1 is exact"
    );

    // Even mix over three channels: the geometric mean.
    let mixed = cc.collapse(&[1.0 / 3.0; 3]).unwrap();
    let expect = (0.5_f64 * 0.8 * 0.25).powf(1.0 / 3.0);
    assert!(
        (mixed[0].2 - expect).abs() < 1e-12,
        "{} vs {expect}",
        mixed[0].2
    );

    // A lopsided, off-simplex θ is accepted (the simplex is a convention).
    let lop = cc.collapse(&[2.0, 0.5, 0.0]).unwrap();
    let expect = 0.5_f64.powf(2.0) * 0.8_f64.powf(0.5);
    assert!((lop[0].2 - expect).abs() < 1e-12);
}

/// The defining property: for each fixed `θ`, `|·|_θ` is a monoid homomorphism
/// `([0,1]^C, ·) → ([0,1], ·)` — `|v ⊙ w|_θ = |v|_θ · |w|_θ` — which is exactly
/// the Leinster 2013 §1.3 "size" datum that makes the magnitude well-defined per
/// choice of θ.
#[test]
fn collapse_is_a_monoid_homomorphism() {
    let v = [0.5_f64, 0.8, 0.6];
    let w = [0.25_f64, 1.0, 0.4];
    let vw: Vec<f64> = v.iter().zip(&w).map(|(a, b)| a * b).collect();

    let build = |vec: &[f64]| {
        let mut cc = ChannelCouplings::new(3).unwrap();
        cc.set(0, 1, vec.to_vec()).unwrap();
        cc
    };

    for theta in [
        vec![1.0 / 3.0; 3],
        vec![0.5, 0.25, 0.25],
        vec![1.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0], // the degenerate all-zero weight
        vec![2.0, 0.5, 1.5], // off-simplex
    ] {
        let lhs = build(&vw).collapse(&theta).unwrap()[0].2;
        let rhs =
            build(&v).collapse(&theta).unwrap()[0].2 * build(&w).collapse(&theta).unwrap()[0].2;
        assert!(
            (lhs - rhs).abs() <= 1e-12 * lhs.abs().max(rhs.abs()).max(1.0),
            "θ = {theta:?}: |v⊙w| = {lhs} vs |v|·|w| = {rhs}"
        );
    }

    // The unit of ([0,1]^C, ·) maps to the unit of ([0,1], ·).
    assert_eq!(
        build(&[1.0, 1.0, 1.0])
            .collapse(&[0.5, 0.25, 0.25])
            .unwrap()[0]
            .2,
        1.0
    );
}

/// `0.0_f64.powf(0.0) == 1.0`: a zero channel carrying zero weight drops out of
/// the product instead of annihilating it — and `θ = 0` collapses everything to
/// `1.0` (degenerate, documented, the caller's responsibility).
#[test]
fn collapse_zero_to_the_zero_edge_cases() {
    let mut cc = ChannelCouplings::new(2).unwrap();
    cc.set(0, 1, vec![0.0, 0.6]).unwrap();

    // Channel 0 is zero but unweighted: 0^0 · 0.6^1 = 1 · 0.6.
    assert_eq!(cc.collapse(&[0.0, 1.0]).unwrap(), vec![(0, 1, 0.6)]);
    // Weighted, it annihilates: 0^1 · 0.6^0 = 0.
    assert_eq!(cc.collapse(&[1.0, 0.0]).unwrap(), vec![(0, 1, 0.0)]);
    // θ = 0 everywhere ⇒ every coupling becomes a perfect 1.0.
    assert_eq!(cc.collapse(&[0.0, 0.0]).unwrap(), vec![(0, 1, 1.0)]);
}

/// Collapsed triples are ordinary couplings — they drive the whole coalition
/// surface, and different θ genuinely give different valuations (the point of
/// θ being an experiment parameter rather than a theorem).
#[test]
fn collapsed_couplings_feed_the_evaluator() {
    let agents = ["a", "b", "c"];
    let mut cc = ChannelCouplings::new(2).unwrap();
    // Channel 0: "capability overlap". Channel 1: "communication".
    cc.set(0, 1, vec![0.9, 0.4]).unwrap();
    cc.set(1, 2, vec![0.3, 0.95]).unwrap();

    let members = all(3);
    let mut values = Vec::new();
    for theta in [vec![1.0, 0.0], vec![0.0, 1.0], vec![0.5, 0.5]] {
        let couplings = cc.collapse(&theta).unwrap();
        assert_eq!(couplings.len(), 2);

        let ev = CoalitionEvaluator::new(&agents, &couplings, &members, 1.0).unwrap();
        let fresh = coalition_value(&agents, &couplings, &members).unwrap();
        assert_eq!(ev.base_value(), fresh);
        assert!(fresh.is_finite() && fresh > 0.0);
        values.push(fresh);
    }

    // θ = e_0 and θ = e_1 pick out different couplings, so different magnitudes.
    assert!(
        (values[0] - values[1]).abs() > 1e-6,
        "θ choice must move the valuation: {values:?}"
    );

    // A channel table can also be modulated by roles before evaluation — the
    // layers compose: collapse (T3) then modulate (T2).
    let rho = RoleModulation::new(vec![vec![1.0, 0.5], vec![0.25, 1.0]]).unwrap();
    let collapsed = cc.collapse(&[0.5, 0.5]).unwrap();
    let typed = modulate(&collapsed, &[0, 1, 1], &rho).unwrap();
    let composed = coalition_value(&agents, typed.couplings(), &members).unwrap();
    assert!(
        composed >= values[2] - REL_TOL,
        "role modulation weakens couplings ⇒ diversity does not drop"
    );
}
