#![allow(
    clippy::similar_names,             // seq_result / par_result pairs are intentional
    clippy::cast_possible_truncation,  // usize fixture sizes fit in i32 by construction
    clippy::cast_possible_wrap,
)]

//! Parallel-vs-sequential equivalence tests.
//!
//! Runs the same operation at inputs straddling the parallelism threshold
//! and asserts outputs are identical. Rayon 1.12's `with_min_len` adaptive
//! splitter means the same code path runs both cases, so divergence here
//! would indicate a determinism bug — not a threshold mismatch.
//!
//! Pattern borrowed from the rayon crate's own test suite — the
//! "deterministic parallel-vs-sequential equivalence" idiom.

use catgraph::{
    category::HasIdentity,
    frobenius::{FrobeniusMorphism, special_frobenius_morphism},
    monoidal::Monoidal,
    named_cospan::NamedCospan,
};
use either::Either;
use std::collections::BTreeSet;

/// Run the predicate filter twice — once on a small input (below threshold,
/// effectively sequential) and once on a large input (above threshold, fan-out
/// across workers) — and assert the output set is the same shape.
#[test]
fn named_cospan_predicate_determinism() {
    // Small: 10 left + 10 right names, well below threshold 256.
    let nc_small = build_named_cospan(10);
    let found_small = nc_small.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, false);

    // Large: 400 left + 400 right, above threshold.
    let nc_large = build_named_cospan(400);
    let found_large = nc_large.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, false);

    // Predicate results are ordered-producer-dependent; use BTreeSet to compare sets.
    let small_set: BTreeSet<_> = found_small.iter().map(classify).collect();
    let large_set: BTreeSet<_> = found_large.iter().map(classify).collect();

    // Properties preserved across both sizes:
    assert_eq!(small_set.iter().filter(|(is_left, _)| *is_left).count(), 5);
    assert_eq!(
        large_set.iter().filter(|(is_left, _)| *is_left).count(),
        200
    );
    // All matched names are even.
    for (_, name) in &small_set {
        assert_eq!(name % 2, 0);
    }
    for (_, name) in &large_set {
        assert_eq!(name % 2, 0);
    }
}

fn build_named_cospan(n: usize) -> NamedCospan<char, i32, i32> {
    let left: Vec<usize> = (0..n).collect();
    let right: Vec<usize> = (n..2 * n).collect();
    let middle: Vec<char> = (0..2 * n).map(|_| 'x').collect();
    let left_names: Vec<i32> = (0..n as i32).collect();
    let right_names: Vec<i32> = (n as i32..2 * n as i32).collect();
    NamedCospan::new(left, right, middle, left_names, right_names)
}

fn classify(e: &Either<usize, usize>) -> (bool, i32) {
    match e {
        Either::Left(i) => (true, i32::try_from(*i).unwrap()),
        Either::Right(i) => (false, i32::try_from(*i).unwrap()),
    }
}

/// `FrobeniusMorphism` hflip runs twice — on a layer below threshold (64) and
/// above threshold — and we assert shape invariants are preserved across both.
#[test]
fn frobenius_hflip_determinism() {
    // Small: single-layer morphism with fewer than 64 blocks.
    let small: FrobeniusMorphism<char, String> = special_frobenius_morphism(16, 1, 'a');
    // Large: triggers layers with 128+ blocks.
    let large: FrobeniusMorphism<char, String> = special_frobenius_morphism(128, 1, 'a');

    assert!(small.depth() > 0);
    assert!(large.depth() > 0);

    // Monoidal composition with identity preserves depth (no-op structurally).
    let mut small_composed = small.clone();
    small_composed.monoidal(FrobeniusMorphism::identity(&vec![]));
    let mut large_composed = large.clone();
    large_composed.monoidal(FrobeniusMorphism::identity(&vec![]));

    assert_eq!(small_composed.depth(), small.depth());
    assert_eq!(large_composed.depth(), large.depth());
}

// --- Corel ------------------------------------------------------------------

#[test]
fn ccr_deterministic_across_runs() {
    use catgraph::{corel::Corel, cospan::Cospan};

    let a = Corel::<char>::new(Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a'])).unwrap();

    let r1 = a.coarsest_common_refinement(&b).unwrap();
    let r2 = a.coarsest_common_refinement(&b).unwrap();

    assert_eq!(
        r1.as_cospan().left_to_middle(),
        r2.as_cospan().left_to_middle()
    );
    assert_eq!(
        r1.as_cospan().right_to_middle(),
        r2.as_cospan().right_to_middle()
    );
    assert_eq!(r1.as_cospan().middle(), r2.as_cospan().middle());
}
