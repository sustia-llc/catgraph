//! Chain-sum vs matrix-inversion Möbius equivalence (Leinster 2013 Prop 2.1.3
//! validated against §1.1 Lemma 1.1.4 on scattered spaces).
//!
//! Per Leinster 2013, when ζ is invertible AND the space is scattered
//! (Def 2.1.2), both `mobius_function::<Q>` (Gaussian elimination) and
//! `mobius_function_via_chains::<Q>` (chain-sum) yield the same matrix.
//! The matrix-inversion path is the ground truth; the chain-sum path must
//! agree on every scattered fixture.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::{F64Rig, Tropical};
use catgraph_magnitude::magnitude::{is_scattered, mobius_function};
use catgraph_magnitude::mobius_chains::mobius_function_via_chains;
use catgraph_testutil::{approx_rel, assert_approx_rel};
use proptest::prelude::*;

/// Relative agreement demanded of the two Möbius routes in the **converged**
/// regime (#169).
///
/// Both paths compute the same real matrix by different arithmetic — Gaussian
/// elimination versus a truncated chain sum — so the meaningful question is how
/// many significant digits survive, not how many absolute units. An absolute
/// epsilon conflates the two. At the sites this constant governs, μ entries run
/// from **0.0120** (the proptest corner `n = 5, slack = 3.0`) to **1.1565** (the
/// 2-point diagonal), so the previous absolute `1e-9` ranged from ~83× *looser*
/// than this bound at the small end to ~16% *tighter* at the large end — which
/// is exactly the magnitude-dependence that made it unreadable.
///
/// Headroom is ample: sweeping the whole proptest domain (`n ∈ 2..=5`, `slack`
/// across `[0.5, 3.0]`) the worst relative disagreement measured is **9.99e-14**,
/// four orders under this bound.
const MOBIUS_REL_TOL: f64 = 1e-9;

/// Relative agreement at the **scatteredness boundary** (`slack = 0.1`).
///
/// The chain-sum path is a truncated DFS whose convergence rate is governed by
/// `r = (n − 1)·e^(−d)`; the boundary fixture deliberately sits at `d` just
/// above `log(n − 1)`, so `r → 1⁻` and the truncation tail is far heavier than
/// in the well-separated fixtures. Measured residual on the shipped 4-state
/// boundary fixture is **1.08e-9 relative** (`2.448e-10` on an entry of
/// `0.2267`), so [`MOBIUS_REL_TOL`] does not apply there — this is a property of
/// the method at `r ≈ 1`, not rounding noise, and it was invisible under the old
/// absolute epsilon. One order of headroom above the measurement.
///
/// Applied per-entry, this also relaxes the **diagonal** entries of the same
/// fixture (`μ_ii = 1.2051`, so a bound of `1.205e-8` against a measured
/// relative residual of only `2.03e-10`) — they would have passed at
/// [`MOBIUS_REL_TOL`]. Recorded rather than split into a third constant: the
/// off-diagonals are what the fixture exists to exercise, and a per-entry
/// tolerance switch would obscure that.
const MOBIUS_BOUNDARY_REL_TOL: f64 = 1e-8;

/// Bound for entries that are **exactly** `1.0` by construction (the singleton
/// space's `μ[0][0]`).
///
/// Pure-absolute (`rel = 0.0`): there is no accumulated error to scale here —
/// the measured residual is bit-exactly `0.0` — so the `1e-12` the pre-#169
/// assertion used is earned and must not be silently widened to `1e-9` by a
/// relative bound against a value of `1.0`.
const EXACT_ABS_TOL: f64 = 1e-12;

/// Absolute floor for the same comparisons: entries that are structurally zero
/// have no scale for a relative bound to multiply.
const MOBIUS_ABS_TOL: f64 = 1e-12;

/// Hand-built 4-state scattered space at the boundary `d > log(3)`.
fn scattered_uniform_space(n: usize, slack: f64) -> LawvereMetricSpace<usize> {
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new((0..n).collect());
    #[allow(clippy::cast_precision_loss)]
    let d = ((n - 1) as f64).ln() + slack;
    for a in 0..n {
        for b in 0..n {
            if a == b {
                space.set_distance(a, b, Tropical(0.0));
            } else {
                space.set_distance(a, b, Tropical(d));
            }
        }
    }
    space
}

#[test]
fn chain_sum_equals_matrix_inversion_on_4_state_scattered() {
    let space = scattered_uniform_space(4, 0.1);
    assert!(is_scattered(&space));

    let mu_inv = mobius_function::<F64Rig>(&space).expect("matrix-inversion succeeded");
    let mu_chains = mobius_function_via_chains::<F64Rig>(&space).expect("chain-sum succeeded");

    for i in 0..4 {
        for j in 0..4 {
            let inv_val = mu_inv.entries()[i][j].0;
            let chains_val = mu_chains.entries()[i][j].0;
            assert_approx_rel!(
                inv_val,
                chains_val,
                MOBIUS_BOUNDARY_REL_TOL,
                MOBIUS_ABS_TOL,
                "μ[{i}][{j}]: inversion vs chains"
            );
        }
    }
}

#[test]
fn non_scattered_returns_err_on_chain_sum() {
    // 4-state space with d = 0.1 < log(3) ≈ 1.099 ⇒ not scattered.
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0, 1, 2, 3]);
    for a in 0..4 {
        for b in 0..4 {
            if a == b {
                space.set_distance(a, b, Tropical(0.0));
            } else {
                space.set_distance(a, b, Tropical(0.1));
            }
        }
    }
    assert!(!is_scattered(&space));

    let result = mobius_function_via_chains::<F64Rig>(&space);
    assert!(
        result.is_err(),
        "chain-sum on non-scattered space should Err"
    );
}

#[test]
fn boundary_near_non_scattered_returns_err_on_chain_sum() {
    // Boundary-near: 4-state space with
    // d = 1.05 < log(3) ≈ 1.0986 ⇒ not scattered, but only barely. Verifies
    // the scatteredness check is tight (no off-by-epsilon — the strict
    // `>` in Def 2.1.2 should reject this).
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0, 1, 2, 3]);
    for a in 0..4 {
        for b in 0..4 {
            if a == b {
                space.set_distance(a, b, Tropical(0.0));
            } else {
                space.set_distance(a, b, Tropical(1.05));
            }
        }
    }
    assert!(
        !is_scattered(&space),
        "d = 1.05 < log(3) ≈ 1.0986 ⇒ must classify non-scattered"
    );
    let result = mobius_function_via_chains::<F64Rig>(&space);
    assert!(
        result.is_err(),
        "chain-sum on boundary-near non-scattered space should Err"
    );
}

#[test]
fn chain_sum_empty_space() {
    let space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![]);
    let mu = mobius_function_via_chains::<F64Rig>(&space).expect("empty space ok");
    assert_eq!(mu.rows(), 0);
    assert_eq!(mu.cols(), 0);
}

#[test]
fn chain_sum_one_point_space() {
    let mut space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0]);
    space.set_distance(0, 0, Tropical(0.0));
    let mu = mobius_function_via_chains::<F64Rig>(&space).expect("singleton ok");
    assert_eq!(mu.rows(), 1);
    assert_approx_rel!(mu.entries()[0][0].0, 1.0, 0.0, EXACT_ABS_TOL);
}

#[test]
fn chain_sum_two_point_space_matches_inversion() {
    // n = 2 ⇒ scatteredness threshold is log(1) = 0; any d > 0 is scattered.
    let space = scattered_uniform_space(2, 1.0);
    assert!(is_scattered(&space));

    let mu_inv = mobius_function::<F64Rig>(&space).expect("inv ok");
    let mu_chains = mobius_function_via_chains::<F64Rig>(&space).expect("chains ok");

    for i in 0..2 {
        for j in 0..2 {
            let inv_val = mu_inv.entries()[i][j].0;
            let chains_val = mu_chains.entries()[i][j].0;
            assert_approx_rel!(
                inv_val,
                chains_val,
                MOBIUS_REL_TOL,
                MOBIUS_ABS_TOL,
                "μ[{i}][{j}]: inv vs chains"
            );
        }
    }
}

proptest! {
    /// Equivalence on random uniform-distance scattered spaces of size 2-5.
    /// Slack range bounded away from 0 to keep r = (n-1)·e^(-d) comfortably
    /// below 1 for the truncated DFS.
    #[test]
    fn chain_sum_equals_matrix_inversion_on_random_scattered(
        n in 2usize..=5,
        slack in 0.5f64..3.0,
    ) {
        let space = scattered_uniform_space(n, slack);
        prop_assume!(is_scattered(&space));

        let mu_inv = mobius_function::<F64Rig>(&space)
            .map_err(|e| TestCaseError::fail(format!("inv: {e:?}")))?;
        let mu_chains = mobius_function_via_chains::<F64Rig>(&space)
            .map_err(|e| TestCaseError::fail(format!("chains: {e:?}")))?;

        for i in 0..n {
            for j in 0..n {
                let inv_val = mu_inv.entries()[i][j].0;
                let chains_val = mu_chains.entries()[i][j].0;
                prop_assert!(
                    approx_rel(inv_val, chains_val, MOBIUS_REL_TOL, MOBIUS_ABS_TOL),
                    "n={n} slack={slack} μ[{i}][{j}]: inv={inv_val}, chains={chains_val}, \
                     residual={:.3e}",
                    (inv_val - chains_val).abs()
                );
            }
        }
    }
}
