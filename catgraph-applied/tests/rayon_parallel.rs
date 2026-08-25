//! Rayon threshold correctness validation for catgraph-applied modules.
//!
//! Both tests run at an input size above their module's rayon threshold — 32
//! terms for `linear_combination`'s `CondIterator` sites, 8 for the
//! `CondIterator` sites in `temperley_lieb::non_crossing` (those two were
//! `par_bridge` before they were unified onto `rayon_cond`). They are not
//! performance tests, and the two differ in strength:
//!
//! - `linear_combination_above_threshold` is a **value oracle**: the whole
//!   product is compared term by term against an independent nested-loop
//!   reference. It really does take the parallel arm — measured, by reddening
//!   under a doubling confined to that arm. Its fixture is the only `Mul` test
//!   fixture in the crate whose basis contains the absorbing element `0`.
//! - `temperley_lieb_above_threshold` is mostly a **shape check**: it asserts a
//!   generator count and the domain/codomain of `e_1 ; e_2` — never that
//!   composite's value — and one value assertion, the identity law
//!   `id ; e_8 = e_8`. Nothing here compares a non-trivial Brauer composite
//!   against a reference.

mod common;

use catgraph::category::{Composable, HasIdentity};
use catgraph_applied::{linear_combination::LinearCombination, temperley_lieb::BrauerMorphism};
use common::{assert_matches_mul_reference, mul_reference, product_multiplicities};

/// `LinearCombination` Mul with 64 terms on each side (threshold = 32, so the
/// parallel arm runs), checked against an independent nested-loop reference.
///
/// Until [#293](https://github.com/sustia-llc/catgraph/issues/293) this
/// asserted only `assert_ne!(simplified, LinearCombination::default())` — that
/// the answer was not empty. Every wrong answer with at least one term passes
/// that, including the coefficient-doubling mutant of the parallel arm that
/// motivated the issue.
///
/// The fixture's inputs are unchanged rather than folded into or replaced by
/// `tests/rayon_equivalence.rs` (the terms are now built as a `Vec` so the
/// reference can iterate them twice, and the coefficient expression is spelled
/// `i64::from(i) + 1`; the values are the same ones the test has always used),
/// because it covers two things the `1..=n` fixtures there do not. The basis
/// contains `0`, which is **absorbing** under integer multiplication and so
/// gathers a collision class of 127<!--m:mul_absorbing.pairs_at_zero--> term
/// pairs onto the single product `0` — verified against every other `Mul` test
/// fixture in the crate, none of whose bases contain `0`. And the instantiation is
/// `LinearCombination<i64, i32>`: the basis products are formed in `i32`, a
/// narrower type than the `i64` coefficients, where every other directly-built
/// `Mul` fixture computes its basis products in the coefficient type or in
/// `String`.
///
/// **What this pin cannot see.** One size and one dispatch state (64 × 64,
/// parallel). Both basis multiplication and coefficient multiplication are
/// commutative here, so an operand swap in either is invisible — see
/// `rayon_equivalence::mul_on_a_non_commutative_basis_keeps_operand_order` for
/// the basis half; the coefficient half is uncovered crate-wide. All
/// coefficients are positive, so nothing cancels: the absorbing class sums to
/// 2143<!--m:mul_absorbing.coeff_at_zero--> rather than
/// vanishing, exactly 0<!--m:mul_absorbing.zero_coefficient_terms--> of the
/// 1238<!--m:mul_absorbing.distinct_products--> product terms carry a zero
/// coefficient, and this fixture therefore does not exercise zero-coefficient
/// removal after a collision — `simplify` is asserted here to be the identity
/// on this input, not to remove anything.
#[test]
fn linear_combination_above_threshold() {
    let terms_a: Vec<(i32, i64)> = (0..64).map(|i| (i, i64::from(i) + 1)).collect();
    let terms_b: Vec<(i32, i64)> = (0..64).map(|i| (i, 1_i64)).collect();
    let lc_a: LinearCombination<i64, i32> = terms_a.iter().copied().collect();
    let lc_b: LinearCombination<i64, i32> = terms_b.iter().copied().collect();

    // Independent reference: multiplication distributes over the basis, so
    // (c1·t1)·(c2·t2) = (c1·c2)·(t1·t2). `mul_reference` accumulates in a plain
    // HashMap, sharing no arithmetic with `LinearCombination`; it is the same
    // one `tests/rayon_equivalence.rs` checks against.
    let term_pairs = terms_a.len() * terms_b.len();
    let expected = mul_reference(&terms_a, &terms_b);
    let pairs_at_zero = product_multiplicities(&terms_a, &terms_b)[&0];
    let coeff_at_zero = expected[&0];
    let distinct_products = expected.len();
    let zero_coefficient_terms = expected.values().filter(|c| **c == 0).count();

    // Closed form for the same count: the pairs reaching product 0 are those
    // with k1 == 0 (one per rhs term) or k2 == 0 (one per lhs term), counted
    // once. Asserting the counted multiplicity against inclusion-exclusion pins
    // the collision class exactly, rather than with a floor that a fixture edit
    // could slip under.
    assert_eq!(
        pairs_at_zero,
        terms_a.len() + terms_b.len() - 1,
        "the absorbing class at basis 0 is not the one this fixture is supposed to have — \
         {pairs_at_zero} of the {term_pairs} term pairs reach it, and both sides are supposed to \
         contain exactly one zero key"
    );
    assert_eq!(
        zero_coefficient_terms, 0,
        "a coefficient cancelled to zero in this fixture — every coefficient is supposed to be \
         positive here, and the docstring's claim that `simplify` removes nothing is now false"
    );

    // Probe the absorbing element: it carries the largest collision class here,
    // so it is the coefficient a failure message is most likely to be about.
    let product = lc_a * lc_b;
    assert_matches_mul_reference(&product, &expected, &0, "64x64 terms (parallel arm)");

    // `simplify` drops zero coefficients. Every coefficient here is positive,
    // so it must drop nothing — asserted against the count measured from the
    // reference rather than against a hand-written constant.
    let mut simplified = product;
    simplified.simplify();
    let mut expected_nonzero = expected;
    expected_nonzero.retain(|_, c| *c != 0);
    let expected_nonzero_lc: LinearCombination<i64, i32> = expected_nonzero.into_iter().collect();
    assert!(
        simplified == expected_nonzero_lc,
        "simplify() at 64x64 disagrees with the reference after zero-coefficient removal \
         ({zero_coefficient_terms} zero-coefficient term(s) were expected to be removed)"
    );

    // Machine-readable facts for `scripts/check_measured_claims.py`. The
    // leading `\n` is load-bearing: under the multi-threaded
    // `cargo test -- --nocapture` that CI captures, the harness writes `... ok`
    // with no trailing newline, so a bare `println!` can land as `okMEASURED …`
    // and the guard sees no fact at all. Same reasoning as
    // `tests/rayon_equivalence.rs`, where it was observed.
    println!("\nMEASURED mul_absorbing.term_pairs = {term_pairs}");
    println!("\nMEASURED mul_absorbing.pairs_at_zero = {pairs_at_zero}");
    println!("\nMEASURED mul_absorbing.coeff_at_zero = {coeff_at_zero}");
    println!("\nMEASURED mul_absorbing.zero_coefficient_terms = {zero_coefficient_terms}");
    println!("\nMEASURED mul_absorbing.distinct_products = {distinct_products}");
}

/// `BrauerMorphism` compose at n=16 (threshold = 8 for `non_crossing` checks).
#[test]
fn temperley_lieb_above_threshold() {
    let gens: Vec<BrauerMorphism<i64>> = BrauerMorphism::temperley_lieb_gens(16);
    assert_eq!(gens.len(), 15);

    // Compose e_1 * e_2 — triggers diagram stacking with 16 source/target points
    let composed = gens[0].compose(&gens[1]).unwrap();
    assert_eq!(composed.domain(), 16);
    assert_eq!(composed.codomain(), 16);

    // Compose identity with a generator — should equal the generator
    let id: BrauerMorphism<i64> = BrauerMorphism::identity(&16);
    let id_composed = id.compose(&gens[7]).unwrap();
    assert_eq!(id_composed, gens[7]);
}
