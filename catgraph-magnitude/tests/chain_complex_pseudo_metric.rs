//! LS 2017 Ex 2.9 — pseudo-metric `[0, ∞]`-category before skeletal collapse.
//! v0.3.x's `is_finite_in` gate rejects `d == 0.0` between distinct points;
//! v0.4.0 widens to LS 2017 Def 3.3 simplicity (point inequality only).

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_magnitude::chain_complex::{Chain, ChainIndex, enumerate_chains};

#[test]
fn pseudo_metric_d_zero_distinct_points_enumerate() {
    // 3 distinct points all at d = 0 from each other.
    let space = LawvereMetricSpace::from_distance_fn(3, |_a, _b| 0.0);

    // v0.3.x would reject all (i, j) pairs with i != j because d == 0.0 fails
    // the `d > 0.0` clause in enumerate_chains' extension filter
    //   ⇒ only degree-0 (single-point) chains would be emitted.
    // v0.4.0 widens: distinct points ARE accepted at d == 0.0
    //   ⇒ enumerate_chains emits length-0-graded chains of all degrees 0..=3.
    let chains = enumerate_chains(&space, 3);
    // For n = 3 distinct points + max_degree = 3, the simple-chain count is
    // n + n(n−1) + n(n−1)² + n(n−1)³ = 3 + 6 + 12 + 24 = 45 chains total
    // (degree 0 = 3 single points; degree 1 = 6; degree 2 = 12; degree 3 = 24).
    // Sharpened post-T6 M-3 review: assert the exact full count, not just non-empty.
    assert_eq!(
        chains.len(),
        45,
        "post-§1.18: pseudo-metric d == 0 should enumerate all 45 simple chains \
         for n=3, max_degree=3; v0.3.x's d > 0 clause silently dropped degree ≥ 1 entries"
    );
    let degree_ge_1: Vec<&Chain> = chains.iter().filter(|c| c.degree() >= 1).collect();
    assert_eq!(
        degree_ge_1.len(),
        42,
        "post-§1.18: degree ≥ 1 count should be 6 + 12 + 24 = 42 for n=3, max_degree=3"
    );

    // Also verify Chain::is_finite_in directly accepts the d=0 pseudo-metric edge.
    let pair = Chain::new(vec![0, 1]);
    assert!(
        pair.is_finite_in(&space),
        "post-§1.18: Chain::is_finite_in should accept (0, 1) at d = 0 in pseudo-metric"
    );

    // ChainIndex must enumerate the (k, ℓ=0) buckets for all k ≤ 3.
    let idx = ChainIndex::new(&space, 3);
    assert!(!idx.grades().is_empty());
    // At least one bucket holds a degree ≥ 1 chain.
    let any_deg1 = idx
        .grades()
        .iter()
        .any(|&ell| !idx.chains_at(1, ell).is_empty());
    assert!(
        any_deg1,
        "post-§1.18: ChainIndex must materialise degree-1 buckets when d == 0 \
         between distinct points"
    );
}
