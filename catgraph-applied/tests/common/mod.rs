//! Shared test helpers for structural equality checks and for the
//! `LinearCombination` convolution reference.
//!
//! `Span` and `NamedCospan` have no `PartialEq`, so `assert_eq!` is unavailable
//! on them and these helpers compare via public accessors instead. `Cospan` is
//! the exception since
//! [#289](https://github.com/sustia-llc/catgraph/issues/289): with the cached
//! identity flags gone it derives `PartialEq`, so the `Cospan` helpers here are
//! `==` plus a per-field failure message, kept for their callers rather than
//! because `==` is unavailable.
//!
//! The `Mul` reference at the bottom lives here rather than in either caller
//! because both `tests/rayon_equivalence.rs` and `tests/rayon_parallel.rs` check
//! the same convolution against it
//! ([#293](https://github.com/sustia-llc/catgraph/issues/293)), and two
//! independently drifting copies of a reference implementation would defeat the
//! point of having one.

use {
    catgraph::{category::Composable, cospan::Cospan, named_cospan::NamedCospan, span::Span},
    catgraph_applied::linear_combination::LinearCombination,
    std::{collections::HashMap, fmt::Debug, hash::Hash, ops::Mul},
};

// ---------------------------------------------------------------------------
// Cospan helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn cospan_eq<L: Eq + Copy + std::fmt::Debug>(a: &Cospan<L>, b: &Cospan<L>) -> bool {
    a.left_to_middle() == b.left_to_middle()
        && a.right_to_middle() == b.right_to_middle()
        && a.middle() == b.middle()
}

#[allow(dead_code)]
pub fn assert_cospan_eq<L: Eq + Copy + std::fmt::Debug>(a: &Cospan<L>, b: &Cospan<L>) {
    assert!(
        cospan_eq(a, b),
        "Cospans differ:\n  left:   {:?} vs {:?}\n  right:  {:?} vs {:?}\n  middle: {:?} vs {:?}",
        a.left_to_middle(),
        b.left_to_middle(),
        a.right_to_middle(),
        b.right_to_middle(),
        a.middle(),
        b.middle(),
    );
}

#[allow(dead_code)]
pub fn assert_cospan_eq_msg<L: Eq + Copy + std::fmt::Debug>(
    a: &Cospan<L>,
    b: &Cospan<L>,
    msg: &str,
) {
    assert_eq!(
        a.left_to_middle(),
        b.left_to_middle(),
        "{msg}: left_to_middle mismatch"
    );
    assert_eq!(
        a.right_to_middle(),
        b.right_to_middle(),
        "{msg}: right_to_middle mismatch"
    );
    assert_eq!(a.middle(), b.middle(), "{msg}: middle mismatch");
}

#[allow(dead_code)]
pub fn assert_cospan_shape<L: Eq + Copy + std::fmt::Debug>(
    a: &Cospan<L>,
    b: &Cospan<L>,
    msg: &str,
) {
    assert_eq!(a.domain(), b.domain(), "{msg}: domain mismatch");
    assert_eq!(a.codomain(), b.codomain(), "{msg}: codomain mismatch");
    assert_eq!(
        a.middle().len(),
        b.middle().len(),
        "{msg}: middle size mismatch"
    );
}

// ---------------------------------------------------------------------------
// Span helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn span_eq<L: Eq + Copy + std::fmt::Debug>(a: &Span<L>, b: &Span<L>) -> bool {
    a.left() == b.left() && a.right() == b.right() && a.middle_pairs() == b.middle_pairs()
}

#[allow(dead_code)]
pub fn spans_eq<L: Eq + Copy + std::fmt::Debug>(a: &Span<L>, b: &Span<L>) -> bool {
    span_eq(a, b)
}

#[allow(dead_code)]
pub fn spans_eq_unordered<L: Eq + Copy + std::fmt::Debug + Ord>(a: &Span<L>, b: &Span<L>) -> bool {
    if a.left() != b.left() || a.right() != b.right() {
        return false;
    }
    let mut a_mid: Vec<_> = a.middle_pairs().to_vec();
    let mut b_mid: Vec<_> = b.middle_pairs().to_vec();
    a_mid.sort_unstable();
    b_mid.sort_unstable();
    a_mid == b_mid
}

#[allow(dead_code)]
pub fn assert_span_eq<L: Eq + Copy + std::fmt::Debug>(a: &Span<L>, b: &Span<L>) {
    assert!(
        span_eq(a, b),
        "Spans differ:\n  left:   {:?} vs {:?}\n  right:  {:?} vs {:?}\n  middle: {:?} vs {:?}",
        a.left(),
        b.left(),
        a.right(),
        b.right(),
        a.middle_pairs(),
        b.middle_pairs(),
    );
}

// ---------------------------------------------------------------------------
// NamedCospan helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn assert_named_cospan_eq<L, LN, RN>(a: &NamedCospan<L, LN, RN>, b: &NamedCospan<L, LN, RN>)
where
    L: Eq + Copy + std::fmt::Debug,
    LN: Eq + Clone + std::fmt::Debug,
    RN: Eq + std::fmt::Debug,
{
    assert!(
        cospan_eq(a.cospan(), b.cospan())
            && a.left_names() == b.left_names()
            && a.right_names() == b.right_names(),
        "NamedCospans differ:\n  left_names:  {:?} vs {:?}\n  right_names: {:?} vs {:?}\n  \
         left_map:    {:?} vs {:?}\n  right_map:   {:?} vs {:?}\n  middle:      {:?} vs {:?}",
        a.left_names(),
        b.left_names(),
        a.right_names(),
        b.right_names(),
        a.cospan().left_to_middle(),
        b.cospan().left_to_middle(),
        a.cospan().right_to_middle(),
        b.cospan().right_to_middle(),
        a.cospan().middle(),
        b.cospan().middle(),
    );
}

// ---------------------------------------------------------------------------
// LinearCombination::Mul reference (#293)
// ---------------------------------------------------------------------------

/// Independent sequential reference for `Mul`: the convolution
/// `Σ (k1 · k2) · (c1 · c2)` over all term pairs, accumulated in a plain
/// `HashMap` with no `LinearCombination` arithmetic in it at all.
///
/// Deliberately stronger than `rayon_equivalence.rs`'s `linear_combine_reference`,
/// which builds its answer out of `singleton`, `* coeff` and `+=` and therefore
/// shares those three impls with the code it is checking. This one shares only
/// `FromIterator` and the derived `PartialEq`: a wrong `Add`/`AddAssign`/
/// `Mul<Coeffs>` would move the value under test without moving the reference.
///
/// It returns the map rather than a `LinearCombination` for a second reason —
/// `LinearCombination`'s public API exposes no way to read the coefficient at a
/// basis element, so a failure message can only name a coefficient when the
/// reference side is a plain map.
#[allow(dead_code)]
pub fn mul_reference<T>(lhs_terms: &[(T, i64)], rhs_terms: &[(T, i64)]) -> HashMap<T, i64>
where
    T: Eq + Hash + Clone + Mul<Output = T>,
{
    let mut acc: HashMap<T, i64> = HashMap::new();
    for (k1, c1) in lhs_terms {
        for (k2, c2) in rhs_terms {
            *acc.entry(k1.clone() * k2.clone()).or_insert(0) += c1 * c2;
        }
    }
    acc
}

/// How many term pairs land on each product basis element, counted from the
/// input keys alone — independent of both the implementation and the reference
/// above, so a "this fixture has collisions" claim is measured rather than
/// asserted by construction.
#[allow(dead_code)]
pub fn product_multiplicities<T>(
    lhs_terms: &[(T, i64)],
    rhs_terms: &[(T, i64)],
) -> HashMap<T, usize>
where
    T: Eq + Hash + Clone + Mul<Output = T>,
{
    let mut counts: HashMap<T, usize> = HashMap::new();
    for (k1, _) in lhs_terms {
        for (k2, _) in rhs_terms {
            *counts.entry(k1.clone() * k2.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// Assert `got` matches the reference, naming the coefficient on BOTH sides
/// when it does not.
///
/// The observed side is read through `linearly_extend(|k| k == probe)`, which
/// collapses the whole combination into at most two terms — the `true` term
/// carrying exactly the coefficient at `probe`. That projection is the only
/// coefficient read-out the public API offers, and it keeps the failure message
/// to two numbers instead of a dump of several hundred unordered entries.
#[allow(dead_code)]
pub fn assert_matches_mul_reference<T>(
    got: &LinearCombination<i64, T>,
    expected: &HashMap<T, i64>,
    probe: &T,
    context: &str,
) where
    T: Eq + Hash + Clone + Debug,
{
    let expected_lc: LinearCombination<i64, T> = expected.clone().into_iter().collect();
    assert!(
        got == &expected_lc,
        "{context}: Mul disagrees with the nested-loop reference.\n  \
         reference coefficient at basis {probe:?}: {}\n  \
         observed, projected onto (basis == {probe:?}): {:?}\n  \
         — the `true` entry of that projection IS the observed coefficient at \
         {probe:?} (no `true` entry means the basis element is absent, i.e. 0), \
         and the `false` entry is every other term summed",
        expected[probe],
        got.linearly_extend(|k| &k == probe),
    );
}
