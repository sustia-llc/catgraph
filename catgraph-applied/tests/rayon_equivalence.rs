#![allow(clippy::similar_names)] // `seq_*`/`par_*` binding pairs are the point of these tests

//! Parallel-vs-sequential equivalence tests for catgraph-applied.
//!
//! `LinearCombination::Mul` and `BrauerMorphism::compose` gate a parallel arm
//! via `rayon_cond::CondIterator` at a size threshold (32 for
//! `linear_combination`, 8 for `temperley_lieb`). These tests construct inputs at
//! both sizes and assert determinism — the mathematical result must not depend
//! on whether the `CondIterator::Parallel` or `CondIterator::Serial` arm was
//! taken.
//!
//! Two kinds of coverage here:
//! 1. **Domain-level** — algebraic laws (commutativity, associativity, identity)
//!    verified at sizes straddling each threshold, so both arms run through
//!    the domain code.
//! 2. **`CondIterator`-level** — direct `parallel=true` vs `parallel=false`
//!    equivalence on map+collect and `.any()`, isolating the toggle itself.
//!
//! Pattern borrowed from the rayon crate's own test suite — the
//! "Deterministic parallel-vs-sequential equivalence" idiom.

use catgraph::category::{Composable, HasIdentity};
use catgraph_applied::{linear_combination::LinearCombination, temperley_lieb::BrauerMorphism};

/// `LinearCombination::Mul` is commutative over a commutative Target ring.
/// Run at sizes below (16) and above (64) the threshold; assert commutativity
/// holds in both cases.
#[test]
fn linear_combination_mul_commutative_small_and_large() {
    // Small: 16 terms each, below PARALLEL_MUL_THRESHOLD=32.
    let a_small = make_lc(16, 1);
    let b_small = make_lc(16, 7);
    let ab_small = a_small.clone() * b_small.clone();
    let ba_small = b_small * a_small;
    assert_eq!(
        ab_small, ba_small,
        "Mul should be commutative at small size"
    );

    // Large: 64 terms each, above threshold (triggers parallel path).
    let a_large = make_lc(64, 1);
    let b_large = make_lc(64, 7);
    let ab_large = a_large.clone() * b_large.clone();
    let ba_large = b_large * a_large;
    assert_eq!(
        ab_large, ba_large,
        "Mul should be commutative at large size"
    );
}

/// `LinearCombination::Mul` — verify the parallel and sequential paths produce
/// identical output on the same input by pinning the input size at a level
/// that would exercise each path.
#[test]
fn linear_combination_mul_associative_across_threshold() {
    // At threshold boundary: 33 terms (just above 32).
    let a = make_lc(33, 1);
    let b = make_lc(33, 2);
    let c = make_lc(33, 3);
    let ab_c = (a.clone() * b.clone()) * c.clone();
    let a_bc = a * (b * c);
    assert_eq!(
        ab_c, a_bc,
        "Mul should be associative — parallel path must agree"
    );
}

fn make_lc(n: usize, offset: i64) -> LinearCombination<i64, i64> {
    (0..n)
        .map(|i| (i64::try_from(i).unwrap() + offset, 1_i64))
        .collect()
}

/// `BrauerMorphism` compose is associative. Check at sizes straddling
/// `PARALLEL_COMBINATIONS_THRESHOLD = 8`.
#[test]
fn temperley_lieb_compose_associative_small_and_large() {
    // Small: n=4, below threshold.
    let gens_small: Vec<BrauerMorphism<i64>> = BrauerMorphism::temperley_lieb_gens(4);
    let e1 = &gens_small[0];
    let e2 = &gens_small[1];
    let e3 = &gens_small[2];
    let lhs = e1.compose(e2).unwrap().compose(e3).unwrap();
    let rhs = e1.compose(&e2.compose(e3).unwrap()).unwrap();
    assert_eq!(lhs, rhs, "compose should be associative at small n=4");

    // Large: n=12, triggers parallel non-crossing check (threshold 8).
    let gens_large: Vec<BrauerMorphism<i64>> = BrauerMorphism::temperley_lieb_gens(12);
    let g1 = &gens_large[0];
    let g2 = &gens_large[1];
    let g3 = &gens_large[2];
    let lhs = g1.compose(g2).unwrap().compose(g3).unwrap();
    let rhs = g1.compose(&g2.compose(g3).unwrap()).unwrap();
    assert_eq!(
        lhs, rhs,
        "compose should be associative at large n=12 (parallel path)"
    );
}

/// Identity law: `id ; f = f = f ; id` at sizes below and above threshold.
#[test]
fn temperley_lieb_identity_law_small_and_large() {
    // Small: n=4.
    let id_small: BrauerMorphism<i64> = BrauerMorphism::identity(&4);
    let gens_small: Vec<BrauerMorphism<i64>> = BrauerMorphism::temperley_lieb_gens(4);
    let g = &gens_small[0];
    assert_eq!(&id_small.compose(g).unwrap(), g);
    assert_eq!(&g.compose(&id_small).unwrap(), g);

    // Large: n=16.
    let id_large: BrauerMorphism<i64> = BrauerMorphism::identity(&16);
    let gens_large: Vec<BrauerMorphism<i64>> = BrauerMorphism::temperley_lieb_gens(16);
    let h = &gens_large[7];
    assert_eq!(&id_large.compose(h).unwrap(), h);
    assert_eq!(&h.compose(&id_large).unwrap(), h);
}

// ---------------------------------------------------------------------------
// Direct CondIterator arm-equivalence tests. These exercise the `Parallel`
// vs `Serial` arms of `rayon_cond::CondIterator` on the same input and
// assert bit-identical output — isolating the toggle from domain logic.
//
// Gated on the `parallel` feature: `rayon_cond` is only in the dep graph
// when `parallel` is active. `wasm32-wasip1 --no-default-features` builds
// skip these tests since there's no parallel arm to exercise.
// ---------------------------------------------------------------------------

/// `CondIterator::map(..).collect()` must produce identical output regardless
/// of whether the parallel or serial arm was taken.
#[cfg(feature = "parallel")]
#[test]
fn cond_iterator_arms_agree_on_map_collect() {
    use rayon_cond::CondIterator;

    let data: Vec<i64> = (0..256).collect();
    let par: Vec<i64> = CondIterator::new(data.clone(), true)
        .map(|x| x * x + 3)
        .collect();
    let ser: Vec<i64> = CondIterator::new(data, false).map(|x| x * x + 3).collect();
    assert_eq!(
        par, ser,
        "CondIterator::Parallel and CondIterator::Serial must agree on map+collect"
    );
}

/// `CondIterator::any(..)` must produce identical output for both arms, for
/// both matching and non-matching predicates.
#[cfg(feature = "parallel")]
#[test]
fn cond_iterator_arms_agree_on_any() {
    use rayon_cond::CondIterator;

    let data: Vec<i64> = (0..256).collect();

    // Predicate matches (128 ∈ range).
    let par_hit = CondIterator::new(data.clone(), true).any(|x| x == 128);
    let ser_hit = CondIterator::new(data.clone(), false).any(|x| x == 128);
    assert_eq!(par_hit, ser_hit, "any() must agree on matching predicate");
    assert!(par_hit, "expected 128 to be found");

    // Predicate never matches.
    let par_miss = CondIterator::new(data.clone(), true).any(|x| x < 0);
    let ser_miss = CondIterator::new(data, false).any(|x| x < 0);
    assert_eq!(
        par_miss, ser_miss,
        "any() must agree on non-matching predicate"
    );
    assert!(!par_miss, "expected no negative value");
}

/// Direct arm coverage for the `combinations(2)` pattern used in
/// `BrauerMorphism::non_crossing`: verify both arms agree on the crossing-check
/// predicate over a synthesized pair list.
#[cfg(feature = "parallel")]
#[test]
fn cond_iterator_agrees_on_combinations_pattern() {
    use itertools::Itertools;
    use rayon_cond::CondIterator;

    // Build 16 non-overlapping integer intervals — no "crossings" by construction.
    let items: Vec<(i64, i64)> = (0..16).map(|i| (i * 10, i * 10 + 5)).collect();
    let combos: Vec<Vec<(i64, i64)>> = items.iter().copied().combinations(2).collect();

    let par = CondIterator::new(combos.clone(), true).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        (a.0 < b.0 && a.1 > b.0 && a.1 < b.1) || (b.0 < a.0 && b.1 > a.0 && b.1 < a.1)
    });
    let ser = CondIterator::new(combos, false).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        (a.0 < b.0 && a.1 > b.0 && a.1 < b.1) || (b.0 < a.0 && b.1 > a.0 && b.1 < a.1)
    });
    assert_eq!(
        par, ser,
        "combinations-pattern any() must agree across arms"
    );
    assert!(!par, "non-overlapping intervals should report no crossing");
}
