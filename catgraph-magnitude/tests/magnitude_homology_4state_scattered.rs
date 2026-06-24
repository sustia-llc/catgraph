//! H_{k,ℓ} ranks for the 4-state-scattered fixture.

use catgraph_applied::lawvere_metric::LawvereMetricSpace;
use catgraph_applied::rig::F64Rig;
use catgraph_magnitude::chain_complex::{ChainIndex, magnitude_homology_rank};

#[test]
fn rank_h00_for_4state_scattered() {
    // For a discrete space with no geodesic compositions, ∂ is identically
    // zero, so H_{k,ℓ} = C_{k,ℓ} ⊗ ℤ. At (k=0, ℓ=0): 4 points → rank 4.
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let idx = ChainIndex::new(&space, 3);
    let r = magnitude_homology_rank::<F64Rig>(&idx, &space, 0, 0.0).unwrap();
    assert_eq!(r, 4);
}

#[test]
fn rank_h1_2_for_4state_scattered() {
    // At (k=1, ℓ=2): C_{1,2} has 12 ordered pairs (i ≠ j). ∂_1 has empty
    // interior (1-chains have no interior indices to omit). ∂_2 is trivial
    // because no triple (a, c, b) satisfies d(a, c) + d(c, b) = 2 in a
    // discrete-distance-2 metric — the smallest mid-step sum is 2 + 2 = 4.
    // So rank H_{1, 2} = 12.
    let space = LawvereMetricSpace::from_distance_fn(4, |a, b| if a == b { 0.0 } else { 2.0 });
    let idx = ChainIndex::new(&space, 3);
    let r = magnitude_homology_rank::<F64Rig>(&idx, &space, 1, 2.0).unwrap();
    assert_eq!(r, 12);
}
