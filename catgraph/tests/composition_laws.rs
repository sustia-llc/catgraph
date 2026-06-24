#![allow(clippy::doc_link_with_quotes)] // single-quoted chars in docs are char literals

//! Integration tests for algebraic composition laws across all composable types.
//!
//! Verifies associativity, left/right identity neutrality, empty-boundary edge cases,
//! and large-boundary composition using only the public API.

mod common;
use common::*;

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    named_cospan::NamedCospan,
    span::Span,
};

// ---------------------------------------------------------------------------
// Test morphism builders
// ---------------------------------------------------------------------------

/// Cospan f: {a,b} -> {b,c} with a merge in the middle.
///
/// Left boundary [a,b] maps to middle nodes [0,1].
/// Right boundary [b,c] maps to middle nodes [1,2].
/// Middle labels: ['a','b','c'].
fn cospan_f() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['a', 'b', 'c'])
}

/// Cospan g: {b,c} -> {c,d} with a merge in the middle.
fn cospan_g() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['b', 'c', 'd'])
}

/// Cospan h: {c,d} -> {d,e}.
fn cospan_h() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['c', 'd', 'e'])
}

/// Span f: {a,b} <- S -> {b,c} where S links matching boundary elements.
fn span_f() -> Span<char> {
    // left = ['a','b'], right = ['b','c']
    // middle pairs: (0,0) means left[0]='a' paired with right[0]='b' -- NO, types must match!
    // We need left[i] == right[j] for pair (i,j).
    // left = ['a','b'], right = ['a','b'] -- both boundaries typed the same.
    // Pair (0,0): left[0]='a'==right[0]='a', (1,1): left[1]='b'==right[1]='b'.
    Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)])
}

/// Span g: {a,b} <- S -> {a,b}.
fn span_g() -> Span<char> {
    // A non-trivial span: pairs (0,1) and (1,0) -- swap.
    // left[0]='a'==right[1]='a', left[1]='b'==right[0]='b'.
    Span::new(vec!['a', 'b'], vec!['b', 'a'], vec![(0, 1), (1, 0)])
}

/// Span h: {b,a} <- S -> {b,a} (identity-like).
fn span_h() -> Span<char> {
    Span::new(vec!['b', 'a'], vec!['b', 'a'], vec![(0, 0), (1, 1)])
}

/// Namer: char -> (String, String) for left/right port names.
fn namer(c: char) -> (String, String) {
    (format!("L_{c}"), format!("R_{c}"))
}

fn named_cospan_f() -> NamedCospan<char, String, String> {
    // types: ['a','b'], names: ['x','y']
    // identity-like structure but we just need composable morphisms.
    // domain = ['a','b'], codomain = ['a','b']
    NamedCospan::new(
        vec![0, 1],
        vec![0, 1],
        vec!['a', 'b'],
        vec!["Lx".into(), "Ly".into()],
        vec!["Rx".into(), "Ry".into()],
    )
}

fn named_cospan_g() -> NamedCospan<char, String, String> {
    NamedCospan::new(
        vec![0, 1],
        vec![0, 1],
        vec!['a', 'b'],
        vec!["Gx".into(), "Gy".into()],
        vec!["Hx".into(), "Hy".into()],
    )
}

fn named_cospan_h() -> NamedCospan<char, String, String> {
    NamedCospan::new(
        vec![0, 1],
        vec![0, 1],
        vec!['a', 'b'],
        vec!["Hx".into(), "Hy".into()],
        vec!["Jx".into(), "Jy".into()],
    )
}

// ===========================================================================
// Cospan composition laws
// ===========================================================================

#[test]
fn cospan_associativity() {
    let f = cospan_f();
    let g = cospan_g();
    let h = cospan_h();

    // (f;g);h
    let fg = f.compose(&g).expect("f;g");
    let fg_h = fg.compose(&h).expect("(f;g);h");

    // f;(g;h)
    let gh = g.compose(&h).expect("g;h");
    let f_gh = f.compose(&gh).expect("f;(g;h)");

    // Structural comparison: domain, codomain, and middle size must match.
    // The internal pushout may renumber middle nodes differently, so we compare
    // the domain/codomain label sequences and the overall connectivity.
    assert_eq!(fg_h.domain(), f_gh.domain(), "domains differ");
    assert_eq!(fg_h.codomain(), f_gh.codomain(), "codomains differ");
    assert_eq!(
        fg_h.middle().len(),
        f_gh.middle().len(),
        "middle sizes differ"
    );
}

#[test]
fn cospan_left_identity() {
    let f = cospan_f();
    let types = f.domain();
    let id = Cospan::<char>::identity(&types);

    let id_f = id.compose(&f).expect("id;f");

    // id;f should have the same domain, codomain, and middle as f.
    assert_eq!(id_f.domain(), f.domain());
    assert_eq!(id_f.codomain(), f.codomain());
    assert_eq!(id_f.middle().len(), f.middle().len());
}

#[test]
fn cospan_right_identity() {
    let f = cospan_f();
    let types = f.codomain();
    let id = Cospan::<char>::identity(&types);

    let f_id = f.compose(&id).expect("f;id");

    assert_eq!(f_id.domain(), f.domain());
    assert_eq!(f_id.codomain(), f.codomain());
    assert_eq!(f_id.middle().len(), f.middle().len());
}

#[test]
fn cospan_identity_compose_identity() {
    // id;id == id on the same boundary.
    let types = vec!['x', 'y', 'z'];
    let id = Cospan::<char>::identity(&types);
    let id2 = id.compose(&id).expect("id;id");

    assert_cospan_eq(&id, &id2);
}

// ===========================================================================
// Span composition laws
// ===========================================================================

#[test]
fn span_associativity() {
    let f = span_f();
    let g = span_g();
    let h = span_h();

    // (f;g);h
    let fg = f.compose(&g).expect("f;g");
    let fg_h = fg.compose(&h).expect("(f;g);h");

    // f;(g;h)
    let gh = g.compose(&h).expect("g;h");
    let f_gh = f.compose(&gh).expect("f;(g;h)");

    assert_eq!(fg_h.left(), f_gh.left(), "left (domain) labels differ");
    assert_eq!(fg_h.right(), f_gh.right(), "right (codomain) labels differ");
    // Middle pairs encode the same relation (possibly reordered).
    let mut pairs_lhs: Vec<_> = fg_h.middle_pairs().to_vec();
    let mut pairs_rhs: Vec<_> = f_gh.middle_pairs().to_vec();
    pairs_lhs.sort_unstable();
    pairs_rhs.sort_unstable();
    assert_eq!(pairs_lhs, pairs_rhs, "middle pair sets differ");
}

#[test]
fn span_left_identity() {
    let f = span_f();
    let types = f.domain();
    let id = Span::<char>::identity(&types);

    let id_f = id.compose(&f).expect("id;f");

    assert_eq!(id_f.left(), f.left());
    assert_eq!(id_f.right(), f.right());
    let mut id_f_pairs: Vec<_> = id_f.middle_pairs().to_vec();
    let mut f_pairs: Vec<_> = f.middle_pairs().to_vec();
    id_f_pairs.sort_unstable();
    f_pairs.sort_unstable();
    assert_eq!(id_f_pairs, f_pairs);
}

#[test]
fn span_right_identity() {
    let f = span_f();
    let types = f.codomain();
    let id = Span::<char>::identity(&types);

    let f_id = f.compose(&id).expect("f;id");

    assert_eq!(f_id.left(), f.left());
    assert_eq!(f_id.right(), f.right());
    let mut f_id_pairs: Vec<_> = f_id.middle_pairs().to_vec();
    let mut f_pairs: Vec<_> = f.middle_pairs().to_vec();
    f_id_pairs.sort_unstable();
    f_pairs.sort_unstable();
    assert_eq!(f_id_pairs, f_pairs);
}

#[test]
fn span_identity_compose_identity() {
    let types = vec!['p', 'q'];
    let id = Span::<char>::identity(&types);
    let id2 = id.compose(&id).expect("id;id");

    assert_span_eq(&id, &id2);
}

// ===========================================================================
// NamedCospan composition laws
// ===========================================================================

#[test]
fn named_cospan_associativity() {
    let f = named_cospan_f();
    let g = named_cospan_g();
    let h = named_cospan_h();

    let fg = f.compose(&g).expect("f;g");
    let fg_h = fg.compose(&h).expect("(f;g);h");

    let gh = g.compose(&h).expect("g;h");
    let f_gh = f.compose(&gh).expect("f;(g;h)");

    // Names: compose keeps left names from the first operand and right names from the second.
    // (f;g);h -> left_names = f.left_names, right_names = h.right_names
    // f;(g;h) -> left_names = f.left_names, right_names = h.right_names
    assert_eq!(fg_h.left_names(), f_gh.left_names());
    assert_eq!(fg_h.right_names(), f_gh.right_names());

    // Underlying cospan structure matches.
    assert_eq!(fg_h.cospan().domain(), f_gh.cospan().domain());
    assert_eq!(fg_h.cospan().codomain(), f_gh.cospan().codomain());
    assert_eq!(fg_h.cospan().middle().len(), f_gh.cospan().middle().len());
}

#[test]
fn named_cospan_left_identity() {
    let f = named_cospan_f();
    let types: Vec<char> = vec!['a', 'b'];
    let prenames: Vec<char> = vec!['x', 'y'];
    let id = NamedCospan::<char, String, String>::identity(&types, &prenames, namer);

    // id's right_names must match f's left_names for compose to make sense structurally.
    // The identity namer produces R_x, R_y as right names.
    // f's left names are Lx, Ly. These don't need to match (names are independent of composability).
    // Composability depends only on the cospan's codomain == other's domain labels.
    let id_f = id.compose(&f).expect("id;f");

    // id;f preserves f's right names and id's left names.
    assert_eq!(id_f.left_names(), id.left_names());
    assert_eq!(id_f.right_names(), f.right_names());
    assert_eq!(id_f.cospan().domain(), f.cospan().domain());
    assert_eq!(id_f.cospan().codomain(), f.cospan().codomain());
}

#[test]
fn named_cospan_right_identity() {
    let f = named_cospan_f();
    let types: Vec<char> = vec!['a', 'b'];
    let prenames: Vec<char> = vec!['x', 'y'];
    let id = NamedCospan::<char, String, String>::identity(&types, &prenames, namer);

    let f_id = f.compose(&id).expect("f;id");

    assert_eq!(f_id.left_names(), f.left_names());
    assert_eq!(f_id.right_names(), id.right_names());
    assert_eq!(f_id.cospan().domain(), f.cospan().domain());
    assert_eq!(f_id.cospan().codomain(), f.cospan().codomain());
}

#[test]
fn named_cospan_identity_compose_identity() {
    let types: Vec<char> = vec!['a', 'b'];
    let prenames: Vec<char> = vec!['x', 'y'];
    let id = NamedCospan::<char, String, String>::identity(&types, &prenames, namer);
    let id2 = id.compose(&id).expect("id;id");

    // id;id keeps left_names from the first id and right_names from the second.
    // Since both are the same identity, this should be structurally equal.
    assert_named_cospan_eq(&id, &id2);
}

// ===========================================================================
// Empty boundary edge cases
// ===========================================================================

#[test]
fn cospan_empty_compose_empty() {
    let e1: Cospan<()> = Cospan::empty();
    let e2: Cospan<()> = Cospan::empty();
    let result = e1.compose(&e2).expect("empty;empty");

    assert!(result.left_to_middle().is_empty());
    assert!(result.right_to_middle().is_empty());
    assert!(result.middle().is_empty());
    assert!(result.is_empty());
}

#[test]
fn cospan_empty_left_identity() {
    // Empty cospan has domain [] and codomain []. Compose with a morphism
    // whose domain is also [].
    let empty: Cospan<char> = Cospan::empty();

    // A morphism with empty domain, non-empty codomain:
    // left = [] (no domain), right = [0] (one codomain node), middle = ['x'].
    let f = Cospan::new(vec![], vec![0], vec!['x']);

    let result = empty.compose(&f).expect("empty;f");
    assert!(result.left_to_middle().is_empty());
    assert_eq!(result.right_to_middle().len(), 1);
    assert_eq!(result.codomain(), f.codomain());
}

#[test]
fn cospan_empty_right_identity() {
    // Morphism with non-empty domain, empty codomain, composed with empty.
    let f = Cospan::new(vec![0], vec![], vec!['y']);
    let empty: Cospan<char> = Cospan::empty();

    let result = f.compose(&empty).expect("f;empty");
    assert_eq!(result.left_to_middle().len(), 1);
    assert!(result.right_to_middle().is_empty());
    assert_eq!(result.domain(), f.domain());
}

// ===========================================================================
// Large boundary: composition with 50+ boundary nodes
// ===========================================================================

#[test]
fn cospan_large_boundary_compose() {
    let n = 60;

    // Build a cospan f: [0..n) -> [0..n) that is essentially an identity
    // but with u32 labels.
    let types: Vec<u32> = (0..n).collect();
    let id_f = Cospan::<u32>::identity(&types);

    // Build a non-trivial morphism g: [0..n) -> [0..n) where each boundary
    // node maps to a single shared middle node per adjacent pair.
    // Middle has n nodes, left[i] -> i, right[i] -> i (another identity).
    let id_g = Cospan::<u32>::identity(&types);

    // Compose two identities: result should be identity.
    let result = id_f.compose(&id_g).expect("large id;id");
    assert_eq!(result.domain(), types);
    assert_eq!(result.codomain(), types);
    assert_eq!(result.middle().len(), n as usize);

    // Now build a non-trivial large cospan: all boundary nodes merge to one middle node.
    let merge_all = Cospan::new(vec![0; n as usize], vec![0; n as usize], vec![0u32]);
    // This has domain = [0,0,...,0] (n copies) and codomain = [0,0,...,0].
    // compose(merge_all, merge_all) should succeed.
    let merged = merge_all.compose(&merge_all).expect("merge;merge");
    assert_eq!(merged.left_to_middle().len(), n as usize);
    assert_eq!(merged.right_to_middle().len(), n as usize);
    // All boundary nodes still point to the single middle node.
    assert_eq!(merged.middle().len(), 1);
    assert!(merged.left_to_middle().iter().all(|&x| x == 0));
    assert!(merged.right_to_middle().iter().all(|&x| x == 0));
}

#[test]
fn span_large_boundary_associativity() {
    let n = 50;
    let types: Vec<u32> = (0..n).collect();

    // Three identity spans on a large boundary.
    let f = Span::<u32>::identity(&types);
    let g = Span::<u32>::identity(&types);
    let h = Span::<u32>::identity(&types);

    let fg = f.compose(&g).expect("f;g");
    let fg_h = fg.compose(&h).expect("(f;g);h");

    let gh = g.compose(&h).expect("g;h");
    let f_gh = f.compose(&gh).expect("f;(g;h)");

    assert_eq!(fg_h.left(), f_gh.left());
    assert_eq!(fg_h.right(), f_gh.right());
    assert_eq!(fg_h.middle_pairs().len(), f_gh.middle_pairs().len());

    // Both should be identity-like: each middle pair is (i, i).
    for (i, pair) in fg_h.middle_pairs().iter().enumerate() {
        assert_eq!(*pair, (i, i), "fg_h pair at index {i}");
    }
    for (i, pair) in f_gh.middle_pairs().iter().enumerate() {
        assert_eq!(*pair, (i, i), "f_gh pair at index {i}");
    }
}
