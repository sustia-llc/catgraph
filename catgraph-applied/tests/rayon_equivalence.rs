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
//! Three kinds of coverage here:
//! 1. **Domain-level** — algebraic laws (commutativity, associativity, identity)
//!    verified at sizes straddling each threshold, so both arms run through
//!    the domain code. A law test is invariant under any perturbation that
//!    preserves the law, so it constrains the shape of the answer, not its
//!    value.
//! 2. **`CondIterator`-level** — direct `parallel=true` vs `parallel=false`
//!    equivalence on map+collect and `.any()`, isolating the toggle itself.
//! 3. **Value oracles** — the answer compared term by term against an
//!    independent nested-loop reference, at sizes on both sides of the
//!    threshold. This is what a law test cannot do: under the coefficient
//!    doubling of [#293](https://github.com/sustia-llc/catgraph/issues/293)
//!    both law tests here stayed green and the value oracles reddened.
//!
//! Pattern borrowed from the rayon crate's own test suite — the
//! "Deterministic parallel-vs-sequential equivalence" idiom.

mod common;

use catgraph::category::{Composable, HasIdentity};
use catgraph_applied::{linear_combination::LinearCombination, temperley_lieb::BrauerMorphism};
use common::{assert_matches_mul_reference, mul_reference, product_multiplicities};
use std::ops::Mul;

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

// ---------------------------------------------------------------------------
// LinearCombination::linear_combine — a SECOND `rayon_cond::CondIterator`
// dispatch point, independent of `Mul::mul` (it re-implements the dispatch
// rather than delegating). Its parallel arm is taken only when BOTH operands
// have >= PARALLEL_MUL_THRESHOLD (32) terms. These tests compare its output
// against an independent nested-loop sequential reference at sizes below and
// above the threshold, including a non-injective combiner (coefficient
// collisions) — the same "parallel-output-equals-sequential-reference" idiom
// as the core crate's rayon_equivalence.rs.
// ---------------------------------------------------------------------------

/// `n` distinct terms `(key_offset + i, coeff = i + 1)` — a deterministic,
/// collision-free input side.
fn make_terms(n: usize, key_offset: i64) -> Vec<(i64, i64)> {
    (0..n)
        .map(|i| {
            let i = i64::try_from(i).unwrap();
            (key_offset + i, i + 1)
        })
        .collect()
}

/// Independent sequential reference for `linear_combine`: the generalized
/// convolution `Σ combiner(k1, k2) · (c1 · c2)` over all term pairs, built with
/// a plain double loop and public `LinearCombination` operations only — it never
/// calls `linear_combine`, so it is a genuine cross-check of that method.
fn linear_combine_reference<V, F>(
    lhs_terms: &[(i64, i64)],
    rhs_terms: &[(i64, i64)],
    combiner: F,
) -> LinearCombination<i64, V>
where
    V: Eq + std::hash::Hash + Clone + Default,
    F: Fn(i64, i64) -> V,
{
    let mut acc: LinearCombination<i64, V> = LinearCombination::default();
    for &(k1, c1) in lhs_terms {
        for &(k2, c2) in rhs_terms {
            acc += LinearCombination::singleton(combiner(k1, k2)) * (c1 * c2);
        }
    }
    acc
}

/// Injective combiner `(k1, k2)` (no coefficient collisions): `linear_combine`
/// must equal the sequential reference below (16) and above (40) the 32-term
/// threshold. The parallel arm is taken only when BOTH sides have >= 32 terms,
/// so 40/40 exercises it and 16/16 the serial arm.
#[test]
fn linear_combine_matches_sequential_reference_small_and_large() {
    let pair = |a: i64, b: i64| (a, b);
    for n in [16_usize, 40] {
        let lhs_terms = make_terms(n, 1);
        let rhs_terms = make_terms(n, 1000);
        let lhs: LinearCombination<i64, i64> = lhs_terms.iter().copied().collect();
        let rhs: LinearCombination<i64, i64> = rhs_terms.iter().copied().collect();

        let got = lhs.linear_combine(rhs, pair);
        let expected = linear_combine_reference(&lhs_terms, &rhs_terms, pair);
        assert_eq!(
            got, expected,
            "linear_combine must match the sequential reference at n={n}"
        );
    }
}

/// Non-injective combiner `k1 + k2` at above-threshold size (40/40 → parallel
/// arm): distinct `(k1, k2)` pairs collide onto the same sum, so coefficients
/// must be summed identically on the parallel and sequential paths. Collisions
/// are guaranteed by construction — 40×40 = 1600 pairs map into the 79 sums
/// `0..=78`.
#[test]
fn linear_combine_non_injective_combiner_above_threshold() {
    let add = |a: i64, b: i64| a + b;
    let n = 40_usize;
    let lhs_terms = make_terms(n, 0);
    let rhs_terms = make_terms(n, 0);
    let lhs: LinearCombination<i64, i64> = lhs_terms.iter().copied().collect();
    let rhs: LinearCombination<i64, i64> = rhs_terms.iter().copied().collect();

    let got = lhs.linear_combine(rhs, add);
    let expected = linear_combine_reference(&lhs_terms, &rhs_terms, add);
    assert_eq!(
        got, expected,
        "non-injective combiner (coefficient collisions) must match the reference"
    );
    // Domain sanity: every result key is a sum of two keys in 0..40.
    assert!(
        got.all_terms_satisfy(|k| (0..=78).contains(k)),
        "result basis elements must lie in the summed-key range"
    );
}

// ---------------------------------------------------------------------------
// LinearCombination::Mul — the other of the two `rayon_cond::CondIterator`
// dispatch points in `linear_combination.rs`, and until #293 the one with no
// value oracle above the threshold. `Mul::mul` re-implements the dispatch
// rather than delegating to `linear_combine`, and its parallel arm is taken
// only when BOTH operands have >= PARALLEL_MUL_THRESHOLD (32) terms.
//
// The law tests at the top of this file reach that arm by size (64 and 33 terms
// against a threshold of 32) — but both laws are invariant under a uniform
// doubling of the result, the factor appearing identically on each side of the
// equation, so both stayed green under the mutant of #293. The serial path was
// never in that position: the same doubling confined to it reddened 12 tests,
// `linear_combination::test::multiplication` (2x2) among them, because
// `BrauerMorphism::compose` routes through this very impl over the
// `ExtendedPerfectMatching` basis. (That 12 is from a manual perturbation run
// and no gate re-checks it.)
//
// THE MUTANT, once, since three docstrings below quote runs of it. Both arms of
// the `CondIterator` map the SAME closure (`linear_combination.rs:158-164`), so
// there is no "parallel branch" to edit and no `else` to edit either — the
// second `#[cfg(not(feature = "parallel"))]` binding is not even compiled under
// the default feature set. Confining a defect to one arm means *introducing* the
// split:
//
// ```text
//     CondIterator::new(self.0, enable_parallel)
//         .map(process)
//         .map(|p| if enable_parallel { p.clone() + p } else { p })   // parallel-confined
//         .collect()
// ```
//
// with the two branches exchanged for the serial-confined variant. Every
// figure quoted below comes from running the applied package with
// `--no-fail-fast` under one of those two edits, or under a rewrite of the gate
// itself; without `--no-fail-fast` cargo stops at the first failing binary and
// the totals do not reproduce.
//
// These tests close the parallel half by comparing the answer against an
// independent nested-loop reference at every dispatch state. The reference,
// its multiplicity counter and the failure-message helper live in
// `tests/common/mod.rs` because `tests/rayon_parallel.rs` checks the same
// convolution against them.
// ---------------------------------------------------------------------------

/// Terms with keys `1..=n` and coefficients `c_k = k + coeff_shift`.
///
/// The shift is what keeps the two operands of a same-size cell distinct. With
/// identical operands `self * rhs` is `self * self`, so an impl that read one
/// operand twice would still produce the right answer; the shift removes that
/// blind spot without touching the keys, and therefore without moving any of
/// the measured collision figures, which are functions of the keys alone.
fn mul_terms(n: usize, coeff_shift: i64) -> Vec<(i64, i64)> {
    (1..=n)
        .map(|k| {
            let k = i64::try_from(k).unwrap();
            (k, k + coeff_shift)
        })
        .collect()
}

/// `Mul` must equal the nested-loop reference in all four states of the
/// `enable_parallel` predicate, with basis collisions present in each.
///
/// Keys are `1..=n` on both sides, so the products `k1 · k2` collide — how much
/// is measured below and emitted as `MEASURED` facts rather than asserted from a
/// hand-written constant. Coefficients are `c_k = k` on the left and `c_k = k+1`
/// on the right, so the two operands are never equal (see [`mul_terms`]).
/// `enable_parallel` is `self.len() >= 32 && rhs.len() >= 32`, a conjunction of
/// two operands, so its truth table has four cells and all four are here — under
/// `--no-default-features` there is no `enable_parallel` at all and every cell
/// runs the one sequential path, which is why the arm column names the
/// default-feature build:
///
/// | operands | `self >= 32` | `rhs >= 32` | arm      |
/// |----------|--------------|-------------|----------|
/// | 16 × 16  | false        | false       | serial   |
/// | 40 × 40  | true         | true        | parallel |
/// | 40 × 16  | true         | false       | serial   |
/// | 16 × 40  | false        | true        | serial   |
///
/// The two mixed cells put a *large* operand through the serial arm — so a bug
/// confined to one arm is caught whichever operand the gate happens to key on.
/// That is what they are for; no claim is made about their being the crate's
/// only unequal-operand pair, since every cell here now has unequal operands.
///
/// **What bounds the whole four-cell claim.** Both arms map the same `process`
/// closure (`linear_combination.rs:158-164`); what differs between them is
/// rayon's iterator and `collect`, not the mapped body. So any single-token
/// mutation of `process` moves both arms, and the three serial cells were not
/// observed to add detection over the existing 2×2
/// `linear_combination::test::multiplication` in any perturbation run recorded
/// for #293 — those are enumerated in the crate CHANGELOG's entry for the issue.
/// Whether a mutation tool could generate one they catch is untested — no such
/// tool was run here. They are insurance against a future bespoke parallel body,
/// not coverage of one that exists.
///
/// The cells do **not** pin the gate, and nothing else does either. Two
/// measurements, both null: rewriting the `&&` to `||` left all 660 tests in
/// `catgraph-applied` green, and so did widening the threshold by 2 so that
/// 33-term operands fall to the serial arm. Both arms compute the same value
/// when neither is broken, and no test in the crate observes which arm ran, so
/// a value oracle cannot see a dispatch decision at all; only an arm-confined
/// defect makes one visible. (Both figures are from manual perturbation runs
/// and no gate re-checks them.)
///
/// Collision density, every figure emitted as a `MEASURED` fact and therefore
/// guarded by `scripts/check_measured_claims.py` rather than restated by hand:
/// 256<!--m:mul.16x16.term_pairs--> term pairs give
/// 97<!--m:mul.16x16.distinct_products--> distinct products, the most-collided
/// of them carrying 6<!--m:mul.16x16.max_multiplicity--> pairs;
/// 1600<!--m:mul.40x40.term_pairs--> pairs give
/// 517<!--m:mul.40x40.distinct_products--> distinct products, top multiplicity
/// 12<!--m:mul.40x40.max_multiplicity-->; each mixed cell gives
/// 287<!--m:mul.40x16.distinct_products--> distinct products, top multiplicity
/// 8<!--m:mul.40x16.max_multiplicity-->.
///
/// **What this pin cannot see.** The basis here is `i64` under integer
/// multiplication, which is commutative, so it is blind to a swap of the basis
/// operands (`k1 * k2` → `k2 * k1`); `mul_on_a_non_commutative_basis_keeps_operand_order`
/// below covers that. Coefficients are `i64`, and every coefficient ring the
/// crate exercises anywhere is commutative — `i64` here, `num::Complex<i32>` in
/// `temperley_lieb`'s own unit tests — so a swap of `c_k1 * c_k2` is invisible
/// crate-wide. This test ranges over one coefficient type, one basis type, one
/// key family (consecutive positive integers) and two affine coefficient
/// families (`c_k = k` and `c_k = k+1`), with no zero and no negative
/// coefficients — the absorbing basis element `0` lives in
/// `tests/rayon_parallel.rs` instead. Sizes are 16 and 40 only: no value oracle
/// anywhere runs at 31, 32 or 33 terms, and per the two null measurements above
/// an off-by-one in the threshold comparison is caught by **nothing**.
/// `linear_combination_mul_associative_across_threshold` runs at 33 but is a law
/// test comparing two dispatch-identical computations, so it is in the same
/// blind class, not an exception to it.
///
/// One structural gap in *this test's* fixtures: their keys are `1..=n`, so for
/// a fixed `k1 ≥ 1` the map `k2 ↦ k1 · k2` is injective and no two products
/// collide *within* one `process` call — every collision measured above is
/// merged later, by the `Add` in the fold, and `AddAssign`'s `and_modify`
/// branch is never taken while a partial is being built. It is not a gap in the
/// crate: doubling that branch reddens
/// `linear_combination::test::add_assign` and
/// `rayon_parallel::linear_combination_above_threshold`, whose basis contains
/// the absorbing `0` — its `k1 = 0` partial merges 63 times, reading 2206 at
/// basis `0` against a reference 2143. This test and the `Word` test stay green
/// under it.
#[test]
fn mul_matches_sequential_reference_across_dispatch_states() {
    for (lhs_n, rhs_n) in [(16_usize, 16_usize), (40, 40), (40, 16), (16, 40)] {
        let lhs_terms = mul_terms(lhs_n, 0);
        let rhs_terms = mul_terms(rhs_n, 1);
        let lhs: LinearCombination<i64, i64> = lhs_terms.iter().copied().collect();
        let rhs: LinearCombination<i64, i64> = rhs_terms.iter().copied().collect();

        let expected = mul_reference(&lhs_terms, &rhs_terms);
        // Discriminating power, measured rather than asserted from the fixture's
        // construction: an impl that read one operand twice would compute
        // `lhs * lhs` or `rhs * rhs`, so both must differ from the answer this
        // cell checks. Comparing the term *lists* would not establish that —
        // distinct inputs can still have equal products.
        assert_ne!(
            mul_reference(&lhs_terms, &lhs_terms),
            expected,
            "the {lhs_n}x{rhs_n} cell cannot see an impl that reads `self` twice: \
             lhs*lhs equals lhs*rhs"
        );
        assert_ne!(
            mul_reference(&rhs_terms, &rhs_terms),
            expected,
            "the {lhs_n}x{rhs_n} cell cannot see an impl that reads `rhs` twice: \
             rhs*rhs equals lhs*rhs"
        );
        let multiplicities = product_multiplicities(&lhs_terms, &rhs_terms);
        let pairs = lhs_n * rhs_n;
        let distinct = multiplicities.len();
        // Most-collided product, ties broken towards the smaller key, so the
        // probe named in a failure message is deterministic.
        let (probe, max_multiplicity) = multiplicities
            .iter()
            .map(|(k, m)| (*k, *m))
            .max_by_key(|(k, m)| (*m, -*k))
            .expect("invariant: both operands are non-empty, so some product exists");

        assert!(
            max_multiplicity > 1,
            "the {lhs_n}x{rhs_n} fixture has NO basis collisions — {pairs} term pairs produced \
             {distinct} distinct products, so this case does not exercise coefficient merging \
             and the docstring's collision claim is false"
        );

        let got = lhs * rhs;
        assert_matches_mul_reference(
            &got,
            &expected,
            &probe,
            &format!("{lhs_n}x{rhs_n} terms ({pairs} pairs, {distinct} distinct products)"),
        );

        // Machine-readable facts for `scripts/check_measured_claims.py`. The
        // leading `\n` is load-bearing under `cargo test -- --nocapture`, which
        // CI runs multi-threaded: the harness writes `... ok` WITHOUT a
        // trailing newline, so a bare `println!` can land as `okMEASURED …` and
        // the guard's `(?:^|\s)MEASURED` then matches nothing. Observed here,
        // not theorised — `mul.word40x40.distinct_products` went missing from a
        // full workspace run exactly that way. `println!` takes the stdout lock
        // for the whole call, so this leading newline cannot itself be split
        // off, and the trailing one keeps the value at end-of-line as the
        // guard's `\s*$` requires.
        println!("\nMEASURED mul.{lhs_n}x{rhs_n}.term_pairs = {pairs}");
        println!("\nMEASURED mul.{lhs_n}x{rhs_n}.distinct_products = {distinct}");
        println!("\nMEASURED mul.{lhs_n}x{rhs_n}.max_multiplicity = {max_multiplicity}");
    }
}

/// A free-monoid basis: `Mul` is string concatenation — associative, and
/// crucially NOT commutative, unlike every other basis in this file.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct Word(String);

impl Mul for Word {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        // `push_str` rather than `self.0 + &rhs.0`: clippy's
        // `suspicious_arithmetic_impl` fires on a `+` inside a `Mul` impl, and
        // silencing it with an `allow` would also silence a real slip here.
        let mut concatenated = self.0;
        concatenated.push_str(&rhs.0);
        Self(concatenated)
    }
}

/// `Mul` on a non-commutative basis, at 40 × 40 so the parallel arm runs.
///
/// `LinearCombination::Mul` is public API whose documented contract is
/// convolution over any `Target: Mul<Output = Target>`, and the crate's own
/// production call site — `BrauerMorphism::compose` — instantiates it at
/// `ExtendedPerfectMatching`, whose product is diagram gluing rather than
/// integer multiplication. Every `Mul` fixture in the crate that builds its own
/// `LinearCombination` — rather than reaching the impl through `compose` —
/// nevertheless uses an integer basis, where `k1 * k2 == k2 * k1` makes a swap
/// of the two operands undetectable by construction.
///
/// Measured rather than assumed: with `k1 * k2` rewritten to `k2 * k1` in
/// `Mul::mul`, this was the **only** test in `catgraph-applied` to redden
/// (659 passed / 1 failed on the default feature set) — the `temperley_lieb`
/// fixtures, which reach the same impl through `compose`, all stayed green.
/// That figure comes from a manual perturbation run; no gate re-checks it, so
/// treat it as a record of one measurement and not as a standing invariant.
///
/// The test also proves its own discriminating power before it asserts
/// anything: the swapped-order reference must DIFFER from the true one,
/// otherwise the fixture could not see an operand swap and the equality
/// assertion below would be vacuous with respect to order.
///
/// **What this pin cannot see.** Concatenation on these keys is injective, so
/// all 1600<!--m:mul.word40x40.distinct_products--> products are distinct and
/// this fixture carries no basis collisions at all — that coverage is in
/// `mul_matches_sequential_reference_across_dispatch_states`. It runs one
/// dispatch state (parallel), one size, and one non-commutative basis, and it
/// says nothing about whether `ExtendedPerfectMatching`'s own product is
/// order-sensitive.
#[test]
fn mul_on_a_non_commutative_basis_keeps_operand_order() {
    let n = 40_usize;
    let word_terms = |prefix: char| -> Vec<(Word, i64)> {
        (0..n)
            .map(|i| {
                let i = i64::try_from(i).unwrap();
                (Word(format!("{prefix}{i}")), i + 1)
            })
            .collect()
    };
    let lhs_terms = word_terms('a');
    let rhs_terms = word_terms('b');
    let lhs: LinearCombination<i64, Word> = lhs_terms.iter().cloned().collect();
    let rhs: LinearCombination<i64, Word> = rhs_terms.iter().cloned().collect();

    let expected = mul_reference(&lhs_terms, &rhs_terms);
    // Σ (k2 · k1) · (c2 · c1) — the same pair set with the basis operands
    // swapped. If this equalled `expected`, the fixture would be order-blind.
    let swapped = mul_reference(&rhs_terms, &lhs_terms);
    assert_ne!(
        expected, swapped,
        "the Word fixture is order-blind — swapping the basis operands left the reference \
         unchanged, so this test could not detect an operand swap in Mul::mul"
    );

    let distinct = expected.len();
    assert_eq!(
        distinct,
        n * n,
        "concatenation was expected to be injective on these keys, so all {} pairs should give \
         distinct products; got {distinct}",
        n * n
    );

    let probe = Word("a0b0".to_string());
    let got = lhs * rhs;
    assert_matches_mul_reference(&got, &expected, &probe, "40x40 Word terms (parallel arm)");

    // Leading `\n` for the interleaving reason documented above.
    println!("\nMEASURED mul.word40x40.distinct_products = {distinct}");
}
