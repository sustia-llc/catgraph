//! Tests for `catgraph-magnitude::chain_complex` (LS 2017 §2 substrate).

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_magnitude::chain_complex::Chain;
use catgraph_magnitude::chain_complex::enumerate_chains;
use catgraph_magnitude::weighted_cospan::NodeId;

#[test]
fn chain_zero_simplex_has_degree_zero() {
    let c = Chain::new(vec![0_usize]);
    assert_eq!(c.degree(), 0);
    assert_eq!(c.points().len(), 1);
}

#[test]
fn chain_with_two_distinct_points_has_degree_one() {
    let c: Chain = Chain::new(vec![0, 1]);
    assert_eq!(c.degree(), 1);
}

#[test]
fn chain_length_sums_consecutive_distances() {
    let space =
        LawvereMetricSpace::<NodeId>::from_distance_fn(2, |a, b| if a == b { 0.0 } else { 1.5 });
    let c = Chain::new(vec![0, 1, 0]);
    assert!((c.length(&space) - 3.0).abs() < 1e-12);
}

#[test]
fn enumerate_chains_4state_scattered_degree_1() {
    // 4-state scattered space fixture: d(i,j) = 2.0 for i ≠ j.
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let chains_k1 = enumerate_chains(&space, 1);
    // For k=1, chains are ordered pairs (x_0, x_1) with x_0 ≠ x_1.
    // 4 * 3 = 12 ordered pairs.
    let degree_1: Vec<_> = chains_k1.iter().filter(|c| c.degree() == 1).collect();
    assert_eq!(degree_1.len(), 12);
    // All have length 2.0
    for c in &degree_1 {
        assert!((c.length(&space) - 2.0).abs() < 1e-12);
    }
}

#[test]
fn enumerate_chains_4state_scattered_degree_2() {
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let chains_k2 = enumerate_chains(&space, 2);
    let degree_2: Vec<_> = chains_k2.iter().filter(|c| c.degree() == 2).collect();
    // For k=2, chains are (x_0, x_1, x_2) with x_0≠x_1, x_1≠x_2.
    // 4 * 3 * 3 = 36 ordered triples.
    assert_eq!(degree_2.len(), 36);
    for c in &degree_2 {
        assert!((c.length(&space) - 4.0).abs() < 1e-12);
    }
}

#[test]
fn enumerate_chains_excludes_infinite_distance_chains() {
    // Two-component space: 0-1 distance 1.0; 0-2 distance ∞.
    let space = LawvereMetricSpace::from_distance_fn(3, |a, b| match (a, b) {
        (0, 0) | (1, 1) | (2, 2) => 0.0,
        (0, 1) | (1, 0) => 1.0,
        _ => f64::INFINITY,
    });
    let chains = enumerate_chains(&space, 1);
    // Degree-1 finite chains: (0,1) and (1,0). All others have ∞.
    let degree_1_finite: Vec<_> = chains
        .iter()
        .filter(|c| c.degree() == 1 && c.is_finite_in(&space))
        .collect();
    assert_eq!(degree_1_finite.len(), 2);
}

#[test]
fn length_buckets_group_chains_at_same_grade() {
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let buckets = catgraph_magnitude::chain_complex::ChainIndex::new(&space, 2);
    // Expect: 4 chains at (k=0, ℓ=0); 12 at (k=1, ℓ=2); 36 at (k=2, ℓ=4).
    assert_eq!(buckets.chains_at(0, 0.0).len(), 4);
    assert_eq!(buckets.chains_at(1, 2.0).len(), 12);
    assert_eq!(buckets.chains_at(2, 4.0).len(), 36);
    // Bucket retrieval at unrepresented grade returns empty.
    assert_eq!(buckets.chains_at(1, 5.0).len(), 0);
}

/// Type alias used by the boundary tests below. `F64Rig` is the only `Ring +
/// From<i64>` rig in the workspace today; once an exact-arithmetic `Rational`
/// rig lands, tests can be re-parameterised on it.
type Q = catgraph_applied::rig::F64Rig;

#[test]
fn boundary_partial_squared_is_zero_on_4state_scattered() {
    use catgraph_magnitude::chain_complex::{ChainIndex, boundary_matrix};
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let idx = ChainIndex::new(&space, 3);
    // For 4-state scattered with d=2.0, the geodesic condition requires
    // d(x_{i-1}, x_{i+1}) = 4.0, but d(any, any) = 2.0, so NO interior
    // omission preserves the length grade. Boundary is identically zero.
    // ∂² = 0 trivially. But the matrix shapes must still be consistent.
    let b1 = boundary_matrix::<Q>(&idx, &space, 1, 2.0).unwrap();
    let b2 = boundary_matrix::<Q>(&idx, &space, 2, 4.0).unwrap();
    // For k=1 with i ranging 1..k-1, the loop is empty: ∂_1 is the zero map.
    let sum1: f64 = b1.entries().iter().flatten().map(|q| q.0).sum();
    assert!(sum1.abs() < 1e-12);
    // For k=2 the geodesic condition fails everywhere so ∂_2 = 0.
    let sum2: f64 = b2.entries().iter().flatten().map(|q| q.0).sum();
    assert!(sum2.abs() < 1e-12);
}

#[test]
fn boundary_geodesic_condition_admits_term_when_distances_add() {
    // 3-point space on a line: d(0,1)=1, d(1,2)=1, d(0,2)=2.
    let space = LawvereMetricSpace::from_distance_fn(3, |a, b| {
        let table = [[0.0, 1.0, 2.0], [1.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
        table[a][b]
    });
    let idx = catgraph_magnitude::chain_complex::ChainIndex::new(&space, 2);
    // At k=2, ℓ=2: chain (0,1,2) — interior point 1 admits omission since
    // d(0,2) = 2 = 1 + 1. So ∂_2(0,1,2) = -1 * (0,2) (sign (-1)^1).
    let b2 = catgraph_magnitude::chain_complex::boundary_matrix::<Q>(&idx, &space, 2, 2.0).unwrap();
    // Expect at least one non-zero entry.
    let nonzero = b2
        .entries()
        .iter()
        .flatten()
        .filter(|q| q.0.abs() > 1e-12)
        .count();
    assert!(
        nonzero >= 1,
        "boundary should have a non-zero geodesic term"
    );
}

#[test]
fn boundary_squared_is_zero_geodesic_line() {
    // 5-point line with d(i,j) = |i-j|.
    let space = LawvereMetricSpace::from_distance_fn(5, |a, b| {
        // Both `a` and `b` are in `0..5`, so the i64 conversions cannot wrap
        // and the difference fits exactly in f64's 52-bit mantissa.
        let ai = i64::try_from(a).expect("|a| ≤ 5 fits in i64");
        let bi = i64::try_from(b).expect("|b| ≤ 5 fits in i64");
        let diff = (ai - bi).abs();
        // diff ∈ {0, 1, 2, 3, 4} — f64 representation is exact.
        #[allow(clippy::cast_precision_loss)]
        let d = diff as f64;
        d
    });
    let idx = catgraph_magnitude::chain_complex::ChainIndex::new(&space, 4);
    for &ell in idx.grades() {
        let b1 =
            catgraph_magnitude::chain_complex::boundary_matrix::<Q>(&idx, &space, 1, ell).unwrap();
        let b2 =
            catgraph_magnitude::chain_complex::boundary_matrix::<Q>(&idx, &space, 2, ell).unwrap();
        if b2.cols() == 0 || b1.cols() == 0 {
            continue;
        }
        let composed = b1.matmul(&b2).unwrap();
        // ∂_1 ∘ ∂_2 = 0
        for row in composed.entries() {
            for cell in row {
                assert!(cell.0.abs() < 1e-12, "∂² ≠ 0 at ell={ell}");
            }
        }
    }
}
