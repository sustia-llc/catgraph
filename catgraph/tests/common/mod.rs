//! Shared test helpers for structural equality checks.
//!
//! Core catgraph types intentionally lack `PartialEq` — these helpers compare
//! via public accessors instead.

use catgraph::{category::Composable, cospan::Cospan, named_cospan::NamedCospan, span::Span};

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
// Corel helpers
// ---------------------------------------------------------------------------

use catgraph::corel::Corel;

#[allow(dead_code)]
pub fn corel_eq<L: Eq + Copy + std::fmt::Debug>(a: &Corel<L>, b: &Corel<L>) -> bool {
    cospan_eq(a.as_cospan(), b.as_cospan())
}

#[allow(dead_code)]
pub fn assert_corel_eq<L: Eq + Copy + std::fmt::Debug>(a: &Corel<L>, b: &Corel<L>) {
    assert!(
        corel_eq(a, b),
        "Corels differ — underlying cospans:\n  {:?}\n  vs\n  {:?}",
        a.as_cospan().middle(),
        b.as_cospan().middle(),
    );
}
