//! Tests for `chain_count_signed_graded` (per-grade signed chain-count
//! diagnostic; renamed v0.4.0 §1.19 from `mobius_chains_graded`) and
//! `is_mobius_invertible_at` (Leinster 2013 §2.1 scatteredness threshold
//! check). Task 22 of the v0.3.0 magnitude-homology plan.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::F64Rig;
use catgraph_magnitude::magnitude::is_mobius_invertible_at;
use catgraph_magnitude::mobius_chains::chain_count_signed_graded;

#[test]
fn mobius_invertible_at_t_above_threshold() {
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    // log(4 - 1) ≈ 1.0986; t = 2 should be invertible.
    assert!(is_mobius_invertible_at(&space, 2.0));
    // t = 0.5 should NOT be invertible.
    assert!(!is_mobius_invertible_at(&space, 0.5));
}

#[test]
fn graded_chain_sum_partitions_total() {
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let graded: Vec<(f64, F64Rig)> = chain_count_signed_graded(&space, 3).unwrap();
    // At ell=0: 4 (k=0 chains), at ell=2: -12 (k=1), at ell=4: 36 (k=2),
    // at ell=6: -108 (k=3). Sum: 4 - 12 + 36 - 108 = -80.
    let sum: f64 = graded.iter().map(|(_, q)| q.0).sum();
    assert!((sum - (-80.0)).abs() < 1e-9);
}
