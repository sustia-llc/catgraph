//! Chain-sum vs matrix-inversion Möbius equivalence (Leinster 2013 Prop 2.1.3
//! validated against §1.1 Lemma 1.1.4 on scattered spaces).
//!
//! Per Leinster 2013, when ζ is invertible AND the space is scattered
//! (Def 2.1.2), both `mobius_function::<Q>` (Gaussian elimination) and
//! `mobius_function_via_chains::<Q>` (chain-sum) yield the same matrix.
//! v0.1.x's matrix-inversion is the ground truth; v0.2.0's chain-sum must
//! agree on every scattered fixture.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::{F64Rig, Tropical};
use catgraph_magnitude::magnitude::{is_scattered, mobius_function};
use catgraph_magnitude::mobius_chains::mobius_function_via_chains;
use proptest::prelude::*;

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
            assert!(
                (inv_val - chains_val).abs() < 1e-9,
                "μ[{i}][{j}]: inversion={inv_val:.12}, chains={chains_val:.12}, residual={:.2e}",
                (inv_val - chains_val).abs()
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
    // Boundary-near (v0.2.1 reviewer #2 Minor #4): 4-state space with
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
    assert!((mu.entries()[0][0].0 - 1.0).abs() < 1e-12);
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
            assert!(
                (inv_val - chains_val).abs() < 1e-9,
                "μ[{i}][{j}]: inv={inv_val}, chains={chains_val}"
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
                    (inv_val - chains_val).abs() < 1e-9,
                    "n={n} slack={slack} μ[{i}][{j}]: inv={inv_val}, chains={chains_val}"
                );
            }
        }
    }
}
