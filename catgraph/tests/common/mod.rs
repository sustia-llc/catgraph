//! Shared test helpers for structural equality checks.
//!
//! Core catgraph types intentionally lack `PartialEq` — these helpers compare
//! via public accessors instead.

use catgraph::{
    category::{Composable, ComposableMutating},
    cospan::Cospan,
    cospan_algebra::frobenius_to_cospan,
    frobenius::FrobeniusMorphism,
    named_cospan::NamedCospan,
    span::Span,
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

/// The braiding **wiring** a cospan realizes: domain wire `i` and codomain wire
/// `k` meet when they land on the same apex vertex, i.e. `left[i] == right[k]`.
///
/// The pushout numbers the apex however it likes, so comparing leg vectors
/// cannot say *which* permutation a braiding realizes; this pairing can, and it
/// is what the #258 direction contract is stated in terms of (#286).
///
/// # Panics
///
/// If some domain wire lands on an apex vertex no codomain wire reaches — i.e.
/// the cospan is not a braiding. Callers pass braidings.
#[allow(dead_code)]
pub fn cospan_wiring<L: Eq + Copy + std::fmt::Debug>(c: &Cospan<L>) -> Vec<usize> {
    let (l, r) = (c.left_to_middle(), c.right_to_middle());
    l.iter()
        .map(|li| {
            r.iter()
                .position(|rk| rk == li)
                .expect("a braiding cospan links every domain wire to a codomain wire")
        })
        .collect()
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
// FrobeniusMorphism helpers
// ---------------------------------------------------------------------------

/// Render the public observables of a `FrobeniusMorphism` for a failure message.
///
/// `FrobeniusMorphism` derives `PartialEq` (layer-by-layer presentation
/// equality) but has no `Debug` impl, so `assert_eq!` is unavailable. This
/// stands in for one: the three accessors the crate exposes publicly.
#[allow(dead_code)]
pub fn frobenius_shape<L, BL>(m: &FrobeniusMorphism<L, BL>) -> String
where
    L: Eq + Copy + std::fmt::Debug,
    BL: Eq + Clone,
{
    format!(
        "depth={} dom={:?} cod={:?}",
        m.depth(),
        m.domain(),
        m.codomain()
    )
}

/// Assert two `FrobeniusMorphism`s are equal *by content* (presentation
/// equality), reporting both shapes **and both semantic images** on failure.
///
/// Note this is **presentation** equality, not equality modulo the Frobenius
/// axioms: `δ ; μ` and `id` are semantically equal but compare unequal here.
///
/// `depth`/`domain`/`codomain` cannot tell a connectivity-only regression from
/// a fixture drift (`σ_{a,a}` and `id_{a,a}` print identically), so the message
/// also renders each side's `frobenius_to_cospan(..).canonical_form()` — equal
/// up to apex isomorphism when the two presentations denote the same cospan,
/// away from scalars (`frobenius_to_cospan` is neither sound nor complete on
/// bubbles — see its docs; a spelled `η;ε` and `identity([])` render alike).
#[allow(dead_code)]
pub fn assert_frobenius_eq_msg<L, BL>(
    a: &FrobeniusMorphism<L, BL>,
    b: &FrobeniusMorphism<L, BL>,
    msg: &str,
) where
    L: Eq + Ord + std::hash::Hash + Copy + std::fmt::Debug + Send + Sync,
    BL: Eq + Clone + Send + Sync,
{
    if a == b {
        return;
    }
    let semantic = |m: &FrobeniusMorphism<L, BL>| -> String {
        match frobenius_to_cospan(m) {
            Ok(c) => format!("{:?}", c.canonical_form()),
            Err(e) => format!("<no Cospan image: {e}>"),
        }
    };
    panic!(
        "{msg}: FrobeniusMorphism presentations differ\n  lhs: {} ⟼ {}\n  rhs: {} ⟼ {}",
        frobenius_shape(a),
        semantic(a),
        frobenius_shape(b),
        semantic(b),
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
