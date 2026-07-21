#![allow(
    clippy::similar_names,             // seq_/par_ and left_/right_ pairs are intentional
    clippy::cast_possible_truncation,  // usize fixture sizes fit in i32 by construction
    clippy::cast_possible_wrap,
)]

//! Parallel-vs-sequential equivalence tests for catgraph's core rayon sites.
//!
//! Two `parallel`-feature call sites fan work across rayon workers via
//! `with_min_len`: `NamedCospan::find_nodes_by_name_predicate`
//! (`with_min_len(256)`) and `FrobeniusLayer::hflip` (`with_min_len(64)`,
//! reached through `FrobeniusMorphism`). Rayon's `LengthSplitter` only
//! subdivides a task when its length is at least `2·min` — so `with_min_len(m)`
//! begins fanning across workers at length ≥ 2·m (≥ 512 for find_nodes, ≥ 128
//! for hflip), and runs as a single sequential task below that. Each site's
//! per-element work is independent, so the fan-out must reproduce a plain
//! sequential computation exactly. These tests run each operation at inputs
//! below the single-task ceiling and at inputs wide enough that rayon genuinely
//! subdivides, asserting the output equals a hand-rolled sequential reference
//! computed in-test — divergence would signal a determinism bug. The same tests
//! run under both the default (`parallel`) build and `--no-default-features`, so
//! CI exercises both arms against the same reference.
//!
//! Content-level guards for the `hflip` rayon site (sequential-reference
//! equality, involution) live as `#[cfg(test)]` unit tests in
//! `src/frobenius/operations.rs`, where `FrobeniusLayer::hflip` (module-private)
//! and `FrobeniusMorphism::hflip` (`pub(crate)`) are directly reachable. The
//! public-API guards here go through `special_frobenius_morphism` and
//! `cospan_algebra::cospan_to_frobenius`, both of which call `hflip` internally.
//!
//! Distinct in purpose from `tests/rayon_parallel.rs`: this file pins
//! parallel-output-equals-sequential-reference *equivalence*, whereas
//! `rayon_parallel.rs` checks above-threshold *correctness* (that results are
//! right once the input is large enough to run the parallel arm), mirroring the
//! same split in `catgraph-applied`.
//!
//! Pattern borrowed from the rayon crate's own test suite — the
//! "deterministic parallel-vs-sequential equivalence" idiom.

use catgraph::{
    category::ComposableMutating,
    frobenius::{FrobeniusMorphism, special_frobenius_morphism},
    named_cospan::NamedCospan,
};
use either::Either;

// ── find_nodes_by_name_predicate (threshold 256) ────────────────────────────

/// `n` left ports named `0..n` and `n` right ports named `n..2n`, each mapping
/// to its own middle vertex. As `n` varies the name lists straddle the 256
/// threshold, so the same fixture exercises the sequential and parallel arms.
fn build_named_cospan(n: usize) -> NamedCospan<char, i32, i32> {
    let left: Vec<usize> = (0..n).collect();
    let right: Vec<usize> = (n..2 * n).collect();
    let middle: Vec<char> = (0..2 * n).map(|_| 'x').collect();
    let left_names: Vec<i32> = (0..n as i32).collect();
    let right_names: Vec<i32> = (n as i32..2 * n as i32).collect();
    NamedCospan::new(left, right, middle, left_names, right_names)
}

/// Sequential reference reimplementation of `find_nodes_by_name_predicate`,
/// matching its documented output order: in the non-short-circuit case, all
/// left matches by ascending index followed by all right matches by ascending
/// index; in the `at_most_one` case, the first left match, else the first right
/// match, else empty.
fn predicate_reference(
    left_names: &[i32],
    right_names: &[i32],
    left_pred: impl Fn(i32) -> bool,
    right_pred: impl Fn(i32) -> bool,
    at_most_one: bool,
) -> Vec<Either<usize, usize>> {
    if at_most_one {
        if let Some(i) = left_names.iter().position(|&n| left_pred(n)) {
            return vec![Either::Left(i)];
        }
        if let Some(j) = right_names.iter().position(|&n| right_pred(n)) {
            return vec![Either::Right(j)];
        }
        return Vec::new();
    }
    let mut out: Vec<Either<usize, usize>> = left_names
        .iter()
        .enumerate()
        .filter_map(|(i, &n)| left_pred(n).then_some(Either::Left(i)))
        .collect();
    out.extend(
        right_names
            .iter()
            .enumerate()
            .filter_map(|(j, &n)| right_pred(n).then_some(Either::Right(j))),
    );
    out
}

/// Non-short-circuit path: exact ordered-`Vec` equality against the sequential
/// reference, at sizes below (100) and above (600) the 256 threshold.
#[test]
fn find_nodes_matches_sequential_reference() {
    let even = |n: i32| n % 2 == 0;
    for n in [100_usize, 600] {
        let nc = build_named_cospan(n);
        let found = nc.find_nodes_by_name_predicate(even, even, false);
        let expected = predicate_reference(nc.left_names(), nc.right_names(), even, even, false);
        assert_eq!(
            found, expected,
            "find_nodes must match the sequential reference (order included) at n={n}"
        );
        // Guard against a vacuously-empty reference: both sides contribute matches.
        assert!(found.iter().any(|e| matches!(e, Either::Left(_))), "n={n}");
        assert!(found.iter().any(|e| matches!(e, Either::Right(_))), "n={n}");
    }
}

/// A never-matching predicate yields an empty vector on both arms.
#[test]
fn find_nodes_no_matches_is_empty() {
    let never = |_n: i32| false;
    for n in [100_usize, 600] {
        let nc = build_named_cospan(n);
        let found = nc.find_nodes_by_name_predicate(never, never, false);
        assert!(found.is_empty(), "no predicate hits must yield [] at n={n}");
    }
}

/// `at_most_one = true` short-circuits: a left hit returns exactly the first
/// left index; with no left hit it returns the first right index; with neither,
/// empty. Checked against the reference at both sizes.
#[test]
fn find_nodes_at_most_one_short_circuits() {
    let even = |n: i32| n % 2 == 0;
    let never = |_n: i32| false;
    for n in [100_usize, 600] {
        let nc = build_named_cospan(n);

        // Left hit present → first even left name is 0, at index 0.
        let left_hit = nc.find_nodes_by_name_predicate(even, even, true);
        assert_eq!(
            left_hit,
            predicate_reference(nc.left_names(), nc.right_names(), even, even, true),
            "n={n}"
        );
        assert_eq!(left_hit, vec![Either::Left(0)], "n={n}");

        // No left hit, right hit present → first even right name is n (even), at index 0.
        let right_hit = nc.find_nodes_by_name_predicate(never, even, true);
        assert_eq!(
            right_hit,
            predicate_reference(nc.left_names(), nc.right_names(), never, even, true),
            "n={n}"
        );
        assert_eq!(right_hit, vec![Either::Right(0)], "n={n}");

        // Neither side hits → empty.
        let miss = nc.find_nodes_by_name_predicate(never, never, true);
        assert!(miss.is_empty(), "n={n}");
    }
}

// ── hflip (threshold 64) — public-API determinism guards ────────────────────
//
// `FrobeniusMorphism`/`FrobeniusLayer` derive only `Clone, PartialEq, Eq` (no
// `Debug`), so morphisms are compared with `==` inside `assert!`, not
// `assert_eq!`.

/// `special_frobenius_morphism(1, n, _)` builds the `(n, 1)` morphism and
/// `hflip`s it; at n = 256 the flipped layers are wide enough (≥ 128) that
/// rayon's `with_min_len(64)` actually subdivides, so this pins run-to-run
/// determinism of the parallel hflip through the public constructor.
/// Sequential-reference equality and involution are unit-tested in
/// `src/frobenius/operations.rs`.
#[test]
fn frobenius_hflip_construction_deterministic() {
    let a: FrobeniusMorphism<char, String> = special_frobenius_morphism(1, 256, 'a');
    let b: FrobeniusMorphism<char, String> = special_frobenius_morphism(1, 256, 'a');
    assert!(a == b, "hflip-driven construction must be deterministic");
    assert_eq!(a.domain(), vec!['a']);
    assert_eq!(a.codomain(), vec!['a'; 256]);
}

/// `cospan_algebra::cospan_to_frobenius` epi-mono-decomposes each leg and
/// `hflip`s the right leg internally. A 260-wide cospan whose right leg is a
/// reversal permutation yields permutation layers wide enough (≥ 128) that
/// rayon's `with_min_len(64)` actually subdivides, so converting the same
/// cospan twice guards public-API determinism of that internal hflip.
#[test]
fn cospan_to_frobenius_hflip_deterministic() {
    use catgraph::cospan::Cospan;
    use catgraph::cospan_algebra::cospan_to_frobenius;

    let n: usize = 260;
    let middle: Vec<i32> = (0..n as i32).collect();
    let left: Vec<usize> = (0..n).collect();
    let right: Vec<usize> = (0..n).rev().collect();

    let build = || {
        let cospan = Cospan::new(left.clone(), right.clone(), middle.clone());
        cospan_to_frobenius::<i32, ()>(&cospan).expect("epi-mono decomposition should succeed")
    };
    let first = build();
    let second = build();
    assert!(
        first == second,
        "cospan_to_frobenius (internal hflip) must be deterministic"
    );
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
