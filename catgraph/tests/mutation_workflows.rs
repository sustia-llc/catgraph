#![allow(clippy::doc_link_with_quotes)] // single-quoted chars in docs are char literals, not intra-doc links

//! Integration tests for Cospan and Span mutation methods.
//!
//! Verifies that mutating a morphism (adding/deleting boundary nodes, connecting
//! pairs, adding middle nodes, mapping labels) produces structures that still
//! compose correctly and preserve expected invariants.

mod common;
use common::assert_span_eq;

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    errors::CatgraphError,
    span::Span,
};
use either::Either::{Left, Right};

// ===========================================================================
// Helpers
// ===========================================================================

/// Identity cospan on types ['a', 'b'].
fn id_ab() -> Cospan<char> {
    Cospan::<char>::identity(&vec!['a', 'b'])
}

/// Cospan f: {a,b} -> {b,c} with merge in the middle.
/// left=[0,1], right=[1,2], middle=['a','b','c'].
fn cospan_f() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['a', 'b', 'c'])
}

/// Cospan g: {b,c} -> {c,d}.
fn cospan_g() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['b', 'c', 'd'])
}

/// Span f: left=['a','b'], right=['a','b'], pairs=[(0,0),(1,1)].
fn span_f() -> Span<char> {
    Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![(0, 0), (1, 1)])
}

/// Span g: swap — left=['a','b'], right=['b','a'], pairs=[(0,1),(1,0)].
fn span_g() -> Span<char> {
    Span::new(vec!['a', 'b'], vec!['b', 'a'], vec![(0, 1), (1, 0)])
}

// ===========================================================================
// Cospan mutation tests (12)
// ===========================================================================

#[test]
fn cospan_add_boundary_known_target_then_compose_with_identity() {
    // Start with f: {a,b} -> {b,c}, add a left boundary pointing to middle[2]='c'.
    // New domain becomes {a,b,c}. Compose with identity on {a,b,c}.
    let mut f = cospan_f();
    let added = f.add_boundary_node_known_target(Left(2));
    assert_eq!(added, Left(2)); // new left index = 2

    assert_eq!(f.domain(), vec!['a', 'b', 'c']);
    assert_eq!(f.codomain(), vec!['b', 'c']); // codomain unchanged

    let id = Cospan::<char>::identity(&f.domain());
    let id_f = id.compose(&f).expect("id;f after mutation");
    assert_eq!(id_f.domain(), f.domain());
    assert_eq!(id_f.codomain(), f.codomain());
    assert_eq!(id_f.middle().len(), f.middle().len());
}

#[test]
fn cospan_add_boundary_unknown_target_then_compose() {
    // Start with f: {a,b} -> {b,c}. Add a right boundary with new label 'z'.
    // This creates a new middle node with label 'z' and a right boundary pointing to it.
    let mut f = cospan_f();
    let added = f.add_boundary_node_unknown_target(Right('z'));
    assert_eq!(added, Right(2)); // new right index = 2

    assert_eq!(f.codomain(), vec!['b', 'c', 'z']);
    assert_eq!(f.middle().len(), 4); // was 3, now 4

    // Compose with identity on new codomain.
    let id = Cospan::<char>::identity(&f.codomain());
    let f_id = f.compose(&id).expect("f;id after adding right boundary");
    assert_eq!(f_id.domain(), f.domain());
    assert_eq!(f_id.codomain(), f.codomain());
    assert_eq!(f_id.middle().len(), f.middle().len());
}

#[test]
fn cospan_delete_boundary_then_compose() {
    // Start with f: {a,b} -> {b,c}. Delete left boundary 0.
    // Domain shrinks: swap_remove(0) moves the last element to position 0.
    let mut f = cospan_f();
    f.delete_boundary_node(Left(0));

    // After swap_remove(0): left was [0,1] -> becomes [1] (last moved to pos 0).
    assert_eq!(f.left_to_middle().len(), 1);
    assert_eq!(f.domain(), vec!['b']); // domain is now just 'b'

    // Compose with identity on the smaller domain.
    let id = Cospan::<char>::identity(&f.domain());
    let id_f = id.compose(&f).expect("id;f after delete");
    assert_eq!(id_f.domain(), vec!['b']);
    assert_eq!(id_f.codomain(), f.codomain());
}

#[test]
fn cospan_connect_pair_same_type_verify_map_to_same() {
    // Cospan with 3 middle nodes all labeled 'x'. Left=[0,1], right=[2].
    // Connect left[0] (->mid 0) and left[1] (->mid 1): both label 'x', merge works.
    let mut c = Cospan::new(vec![0, 1], vec![2], vec!['x', 'x', 'x']);
    assert!(!c.map_to_same(Left(0), Left(1)));

    c.connect_pair(Left(0), Left(1));

    // After merge, left[0] and left[1] should map to the same middle node.
    assert!(c.map_to_same(Left(0), Left(1)));
    // Middle should have 2 nodes (one was removed).
    assert_eq!(c.middle().len(), 2);
}

#[test]
fn cospan_connect_pair_different_type_no_change() {
    // Middle nodes with different labels: merging should produce no change.
    let mut c = Cospan::new(vec![0, 1], vec![2], vec!['x', 'y', 'x']);
    let middle_before = c.middle().len();

    c.connect_pair(Left(0), Left(1)); // 'x' vs 'y' — no merge

    assert_eq!(c.middle().len(), middle_before);
    assert!(!c.map_to_same(Left(0), Left(1)));
}

#[test]
fn cospan_add_middle_then_add_boundary_pointing_to_it() {
    // Start empty, build up manually.
    let mut c: Cospan<char> = Cospan::empty();
    let m0 = c.add_middle('p');
    assert_eq!(m0, 0);

    let m1 = c.add_middle('q');
    assert_eq!(m1, 1);

    // Add left boundary pointing to m0, right boundary pointing to m1.
    c.add_boundary_node_known_target(Left(m0));
    c.add_boundary_node_known_target(Right(m1));

    assert_eq!(c.domain(), vec!['p']);
    assert_eq!(c.codomain(), vec!['q']);
    assert_eq!(c.middle().len(), 2);

    // Compose with another manually-built cospan.
    // Need codomain of c == domain of d.
    let mut d: Cospan<char> = Cospan::empty();
    let dm0 = d.add_middle('q');
    let dm1 = d.add_middle('r');
    d.add_boundary_node_known_target(Left(dm0));
    d.add_boundary_node_known_target(Right(dm1));

    let result = c.compose(&d).expect("manual cospans compose");
    assert_eq!(result.domain(), vec!['p']);
    assert_eq!(result.codomain(), vec!['r']);
}

#[test]
fn cospan_identity_flags_preserved_and_broken_by_mutations() {
    let mut id = id_ab();
    assert!(id.is_left_identity());
    assert!(id.is_right_identity());

    // Adding a left boundary with unknown target 'c':
    //   left becomes [0,1,2], middle becomes ['a','b','c'].
    //   Left leg is still [0,1,...,n-1] on a middle of size n — identity is PRESERVED.
    id.add_boundary_node_unknown_target(Left('c'));
    assert!(id.is_left_identity()); // left=[0,1,2], middle len=3 — still identity
    // Right is [0,1] on a middle of size 3 — NOT a bijection onto all of middle,
    // but the flag is only updated by the Right branch, so it's still true here
    // (conservative hint — the Left branch doesn't touch is_right_id).

    // Break left identity: add a left boundary pointing to an EXISTING middle node.
    // This makes left=[0,1,2,0] which is not [0,1,...,n-1].
    id.add_boundary_node_known_target(Left(0));
    // is_left_id &= (left.len()-1 == tgt_idx) => (3 == 0) => false.
    assert!(!id.is_left_identity());

    // Break right identity explicitly by adding a right boundary with unknown target.
    // right becomes [0,1,3], middle becomes ['a','b','c','d'].
    // is_right_id &= (right.len() == middle.len()) => (3 == 4) => false.
    id.add_boundary_node_unknown_target(Right('d'));
    assert!(!id.is_right_identity());
}

#[test]
fn cospan_map_then_compose() {
    let f = cospan_f(); // domain ['a','b'], codomain ['b','c']
    let mapped = f.map(|c| c as u32);

    // mapped has u32 labels. Create a composable partner.
    let g = cospan_g(); // domain ['b','c'], codomain ['c','d']
    let g_mapped = g.map(|c| c as u32);

    let result = mapped.compose(&g_mapped).expect("mapped compose");
    assert_eq!(result.domain(), vec!['a' as u32, 'b' as u32]);
    assert_eq!(result.codomain(), vec!['c' as u32, 'd' as u32]);
}

#[test]
fn cospan_chain_of_mutations_then_compose() {
    // Start with identity on ['a'].
    let mut c = Cospan::<char>::identity(&vec!['a']);
    assert_eq!(c.domain(), vec!['a']);
    assert_eq!(c.codomain(), vec!['a']);

    // Add middle node 'b'.
    let m = c.add_middle('b');
    // Add left boundary pointing to 'b'.
    c.add_boundary_node_known_target(Left(m));
    // Add right boundary pointing to 'b'.
    c.add_boundary_node_known_target(Right(m));

    assert_eq!(c.domain(), vec!['a', 'b']);
    assert_eq!(c.codomain(), vec!['a', 'b']);

    // Compose with identity on ['a', 'b'].
    let id = Cospan::<char>::identity(&vec!['a', 'b']);
    let result = c.compose(&id).expect("mutated compose identity");
    assert_eq!(result.domain(), vec!['a', 'b']);
    assert_eq!(result.codomain(), vec!['a', 'b']);
}

#[test]
fn cospan_map_to_same_cross_boundary() {
    // Left[0] -> mid 0, Right[0] -> mid 0: same middle node.
    let c = Cospan::new(vec![0], vec![0], vec!['x']);
    assert!(c.map_to_same(Left(0), Right(0)));

    // Left[0] -> mid 0, Right[0] -> mid 1: different middle nodes.
    let c2 = Cospan::new(vec![0], vec![1], vec!['x', 'x']);
    assert!(!c2.map_to_same(Left(0), Right(0)));
}

#[test]
fn cospan_delete_last_vs_non_last_boundary() {
    // Cospan with 3 left boundaries: left=[0,1,2], right=[0], middle=['a','b','c'].
    let mut c = Cospan::new(vec![0, 1, 2], vec![0], vec!['a', 'b', 'c']);

    // Delete last left boundary (index 2): simple pop, no swap.
    c.delete_boundary_node(Left(2));
    assert_eq!(c.left_to_middle().len(), 2);
    assert_eq!(c.domain(), vec!['a', 'b']);

    // Delete non-last left boundary (index 0): swap_remove moves index 1 to position 0.
    c.delete_boundary_node(Left(0));
    assert_eq!(c.left_to_middle().len(), 1);
    // After swap_remove(0): left was [0,1] -> [1] (the value from position 1).
    assert_eq!(c.domain(), vec!['b']);
}

#[test]
fn cospan_add_boundary_to_empty_then_compose() {
    // Start from empty, add boundaries step by step.
    let mut c: Cospan<char> = Cospan::empty();
    assert!(c.is_empty());

    // Add a left boundary with new label 'x'.
    c.add_boundary_node_unknown_target(Left('x'));
    assert!(!c.is_empty());
    assert_eq!(c.domain(), vec!['x']);
    assert!(c.codomain().is_empty());

    // Add a right boundary pointing to the same middle node.
    c.add_boundary_node_known_target(Right(0));
    assert_eq!(c.codomain(), vec!['x']);

    // Should compose with identity on ['x'].
    let id = Cospan::<char>::identity(&vec!['x']);
    let result = c.compose(&id).expect("from-empty compose identity");
    assert_eq!(result.domain(), vec!['x']);
    assert_eq!(result.codomain(), vec!['x']);
    assert_eq!(result.middle().len(), 1);
}

// ===========================================================================
// Span mutation tests (8)
// ===========================================================================

#[test]
fn span_add_boundary_left_then_compose() {
    // Start with span f: left=['a','b'], right=['a','b'], pairs=[(0,0),(1,1)].
    // Add a left boundary 'c'. Domain becomes ['a','b','c'].
    let mut f = span_f();
    let added = f.add_boundary_node(Left('c'));
    assert_eq!(added, Left(2));

    assert_eq!(f.left(), &['a', 'b', 'c']);
    assert_eq!(f.right(), &['a', 'b']); // unchanged
    // The new left boundary has no middle pair connecting to it yet.

    // Compose with identity on f's codomain (['a','b']).
    let id = Span::<char>::identity(&f.codomain());
    let result = f.compose(&id).expect("span left-add compose");
    // Result domain = f's left = ['a','b','c'].
    assert_eq!(result.left(), &['a', 'b', 'c']);
    assert_eq!(result.right(), &['a', 'b']);
    // Only 2 pairs survive (the 'c' boundary has no connection through middle).
    assert_eq!(result.middle_pairs().len(), 2);
}

#[test]
fn span_add_boundary_right_then_compose() {
    let mut f = span_f();
    let added = f.add_boundary_node(Right('c'));
    assert_eq!(added, Right(2));

    assert_eq!(f.right(), &['a', 'b', 'c']);

    // Compose with identity on f's domain (['a','b']).
    let id = Span::<char>::identity(&f.domain());
    let result = id.compose(&f).expect("id compose span right-add");
    assert_eq!(result.left(), &['a', 'b']);
    assert_eq!(result.right(), &['a', 'b', 'c']);
    assert_eq!(result.middle_pairs().len(), 2);
}

#[test]
fn span_add_middle_valid_matching_types() {
    // left=['a','b'], right=['a','b']. Add middle pair (0,1) — left[0]='a', right[1]='b'.
    // Wait: 'a' != 'b', so this should fail. Let's use matching types.
    // Add middle pair (0,0): left[0]='a' == right[0]='a'. Valid.
    let mut s = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![]);
    let result = s.add_middle((0, 0));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
    assert_eq!(s.middle_pairs().len(), 1);
    assert_eq!(s.middle_pairs()[0], (0, 0));

    // Add another: (1,1): left[1]='b' == right[1]='b'. Valid.
    let result2 = s.add_middle((1, 1));
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), 1);
}

#[test]
fn span_add_middle_type_mismatch_returns_error() {
    let mut s = Span::new(vec!['a', 'b'], vec!['a', 'b'], vec![]);
    // Add middle pair (0,1): left[0]='a' != right[1]='b'. Should fail.
    let result = s.add_middle((0, 1));
    assert!(result.is_err());
    match result {
        Err(CatgraphError::Composition { message }) => {
            assert!(message.contains("Mismatched"));
        }
        other => panic!("Expected Composition error, got {other:?}"),
    }
    // The middle should not have grown.
    assert!(s.middle_pairs().is_empty());
}

#[test]
fn span_map_then_compose() {
    let f = span_f(); // left=['a','b'], right=['a','b'], pairs=[(0,0),(1,1)]
    let mapped = f.map(|c| c as u32);

    let g = span_g(); // left=['a','b'], right=['b','a'], pairs=[(0,1),(1,0)]
    let g_mapped = g.map(|c| c as u32);

    // f's codomain (mapped) = [97,98], g's domain (mapped) = [97,98]. Composable.
    let result = mapped.compose(&g_mapped).expect("mapped spans compose");
    assert_eq!(result.left(), &['a' as u32, 'b' as u32]);
    assert_eq!(result.right(), &['b' as u32, 'a' as u32]);
}

#[test]
fn span_middle_to_left_right_projections() {
    // Use uniform labels so type checks pass for arbitrary pair patterns.
    let s2 = Span::new(
        vec!['t', 't', 't'],
        vec!['t', 't', 't'],
        vec![(0, 2), (1, 0), (2, 1)],
    );
    assert_eq!(s2.middle_to_left(), vec![0, 1, 2]);
    assert_eq!(s2.middle_to_right(), vec![2, 0, 1]);
}

#[test]
fn span_add_boundary_then_dagger() {
    let mut s = span_f(); // left=['a','b'], right=['a','b'], pairs=[(0,0),(1,1)]
    s.add_boundary_node(Left('c'));
    // left=['a','b','c'], right=['a','b'], pairs=[(0,0),(1,1)]

    let dag = s.dagger();
    // Dagger swaps left/right and flips pairs.
    assert_eq!(dag.left(), &['a', 'b']); // was right
    assert_eq!(dag.right(), &['a', 'b', 'c']); // was left
    assert_eq!(dag.middle_pairs(), &[(0, 0), (1, 1)]); // flipped from (0,0),(1,1)

    // Compose dagger with identity on its codomain.
    let id = Span::<char>::identity(&dag.codomain());
    let result = dag.compose(&id).expect("dagger compose identity");
    assert_eq!(result.left(), dag.left());
    assert_eq!(result.right(), dag.right());
}

#[test]
fn span_multiple_mutations_then_compose_with_identity() {
    // Build a span from scratch with mutations.
    let mut s = Span::new(vec!['a'], vec!['a'], vec![(0, 0)]);

    // Extend domain.
    s.add_boundary_node(Left('b'));
    // Extend codomain.
    s.add_boundary_node(Right('b'));
    // Connect the new boundary nodes through middle.
    let mid_result = s.add_middle((1, 1)); // left[1]='b' == right[1]='b'
    assert!(mid_result.is_ok());

    assert_eq!(s.left(), &['a', 'b']);
    assert_eq!(s.right(), &['a', 'b']);
    assert_eq!(s.middle_pairs(), &[(0, 0), (1, 1)]);

    // Compose with identity on ['a','b'].
    let id = Span::<char>::identity(&vec!['a', 'b']);
    let result = s.compose(&id).expect("mutated span compose identity");
    assert_span_eq(&result, &s);
}
