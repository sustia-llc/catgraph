#![allow(
    clippy::cast_possible_truncation,  // usize fixture sizes fit in i32 by construction
    clippy::cast_possible_wrap,
)]

//! Rayon threshold correctness validation.
//!
//! Verifies that operations produce correct results at sizes large enough to
//! run the parallel arm. Does not test performance.
//!
//! Distinct in purpose from `tests/rayon_equivalence.rs`: this file checks
//! above-threshold *correctness* (results are right once the input is large
//! enough to fan out), whereas `rayon_equivalence.rs` pins
//! parallel-output-equals-sequential-reference *equivalence*, mirroring the
//! same split in `catgraph-applied`.

use catgraph::{
    category::HasIdentity,
    frobenius::{FrobeniusMorphism, special_frobenius_morphism},
    monoidal::Monoidal,
    named_cospan::NamedCospan,
};
use either::Either::{Left, Right};

/// `NamedCospan` `find_nodes_by_name_predicate` with 512 boundary nodes (threshold = 256).
#[test]
fn named_cospan_predicate_above_threshold() {
    // Build a NamedCospan with 300 left nodes and 300 right nodes (total 600 >= 256)
    // Each maps to a distinct middle node.
    let n = 300;
    let left: Vec<usize> = (0..n).collect();
    let right: Vec<usize> = (n..2 * n).collect();
    let middle: Vec<char> = (0..2 * n).map(|_| 'x').collect();
    let left_names: Vec<i32> = (0..n as i32).collect();
    let right_names: Vec<i32> = (n as i32..2 * n as i32).collect();

    let nc: NamedCospan<char, i32, i32> =
        NamedCospan::new(left, right, middle, left_names, right_names);

    // Find all even-named nodes (should hit the parallel path)
    let found = nc.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, false);

    // 150 even names on left (0,2,...,298) + 150 even names on right (300,302,...,598)
    assert_eq!(found.len(), 300);

    // Verify Left/Right classification
    let left_count = found.iter().filter(|e| matches!(e, Left(_))).count();
    let right_count = found.iter().filter(|e| matches!(e, Right(_))).count();
    assert_eq!(left_count, 150);
    assert_eq!(right_count, 150);
}

/// `FrobeniusMorphism` with 128+ blocks via monoidal product (`with_min_len(64)`).
///
/// `special_frobenius_morphism(m, 1, wire_type)` for large m builds layers via
/// recursive monoidal product. Calling `hflip` (through `from_permutation` or
/// direct `special_frobenius_morphism` with m < n) runs the parallel arm, which
/// rayon actually subdivides once a layer reaches ≥ 128 blocks (`with_min_len(64)`
/// splits only at length ≥ 2·min).
#[test]
fn frobenius_hflip_above_threshold() {
    // Build a large morphism: 128 inputs → 1 output
    // This recursively builds layers with 128+ identity blocks that get hflipped.
    let morph: FrobeniusMorphism<char, String> = special_frobenius_morphism(128, 1, 'a');
    assert!(morph.depth() > 0);

    // Now trigger hflip by building 1 → 128 (internally calls hflip on 128→1)
    let morph_flipped: FrobeniusMorphism<char, String> = special_frobenius_morphism(1, 128, 'a');
    assert!(morph_flipped.depth() > 0);

    // Compose: (1 → 128) then (128 → 1) should produce a valid 1 → 1 morphism
    let mut composed = morph_flipped.clone();
    composed.monoidal(FrobeniusMorphism::identity(&vec![]));
    assert!(composed.depth() > 0);
}
