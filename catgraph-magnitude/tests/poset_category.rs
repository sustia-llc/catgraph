//! Leinster 2008 Cor 1.5 substrate — `PosetCategory<NodeId>` input type.
//!
//! Tests the four constructor / validator paths:
//! - `from_partial_order` over a 3-chain `0 ≤ 1 ≤ 2`,
//! - `from_partial_order` over the `2²` Boolean diamond,
//! - `from_arrow_counts` rejecting a 2-cycle (not circuit-free),
//! - `from_arrow_counts` accepting an upper-triangular skeletal category.

use catgraph::errors::CatgraphError;
use catgraph_magnitude::PosetCategory;

#[test]
fn poset_3_chain_from_partial_order() {
    let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
    assert_eq!(cat.size(), 3);
    assert_eq!(cat.zeta(&0, &1), 1, "0 ≤ 1");
    assert_eq!(cat.zeta(&1, &0), 0, "1 not ≤ 0");
    assert_eq!(cat.zeta(&0, &0), 1, "identity");
}

#[test]
fn poset_diamond_from_partial_order() {
    // 2² Boolean lattice: ⊥, {a}, {b}, ⊤.
    let cat = PosetCategory::<u32>::from_partial_order(
        vec![0, 1, 2, 3],
        |a, b| (*a & *b) == *a, // bitwise: a ≤ b iff a ⊆ b.
    );
    assert_eq!(cat.zeta(&0, &3), 1, "⊥ ≤ ⊤");
    assert_eq!(cat.zeta(&1, &2), 0, "{{a}} incomparable to {{b}}");
    assert_eq!(cat.zeta(&3, &0), 0, "⊤ not ≤ ⊥");
}

#[test]
fn poset_from_arrow_counts_validates_circuit_free() {
    // 2-object cycle: a → b, b → a, no identities; not circuit-free.
    let cat = PosetCategory::<u32>::from_arrow_counts(vec![0, 1], vec![vec![0, 1], vec![1, 0]]);
    assert!(
        matches!(
            &cat,
            Err(CatgraphError::Composition { message })
                if message.contains("circuit-free") || message.contains("cycle")
        ),
        "expected CatgraphError::Composition mentioning 'cycle'/'circuit-free'; got {cat:?}"
    );
}

#[test]
fn poset_from_arrow_counts_accepts_skeletal_with_identities() {
    let cat = PosetCategory::<u32>::from_arrow_counts(
        vec![0, 1, 2],
        vec![vec![1, 1, 0], vec![0, 1, 1], vec![0, 0, 1]],
    );
    assert!(cat.is_ok());
}
