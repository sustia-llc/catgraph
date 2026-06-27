//! Integration tests for cross-type interactions between catgraph types.
//!
//! Tests exercise interactions across `Span`, `Cospan`, `NamedCospan`, and
//! `LinearCombination` using only the public API. Each test verifies a property
//! that spans multiple type boundaries rather than testing a single type in isolation.

mod common;
use common::*;

use catgraph::{
    category::Composable, cospan::Cospan, monoidal::SymmetricMonoidalMorphism,
    named_cospan::NamedCospan, span::Span,
};

// ---------------------------------------------------------------------------
// 1. Span dagger involution on a non-trivial span
// ---------------------------------------------------------------------------

#[test]
fn span_dagger_involution_non_trivial() {
    // A span with asymmetric structure:
    //   left  = [a, b, c, a]  (4 boundary nodes)
    //   right = [a, b, c]     (3 boundary nodes)
    //   middle pairs map source elements to (left_idx, right_idx)
    // All middle pairs must have matching types at their positions.
    let left = vec!['a', 'b', 'c', 'a'];
    let right = vec!['a', 'b', 'c'];
    let middle = vec![(0, 0), (1, 1), (2, 2), (3, 0)];
    let s = Span::new(left, right, middle);

    // dagger flips domain/codomain and swaps each pair
    let d = s.dagger();
    assert_eq!(d.left(), s.right());
    assert_eq!(d.right(), s.left());

    // double dagger must recover the original span exactly
    let dd = d.dagger();
    assert!(
        spans_eq(&s, &dd),
        "dagger(dagger(s)) should equal s:\n  s.left={:?}, dd.left={:?}\n  s.right={:?}, dd.right={:?}\n  s.middle={:?}, dd.middle={:?}",
        s.left(),
        dd.left(),
        s.right(),
        dd.right(),
        s.middle_pairs(),
        dd.middle_pairs(),
    );
}

// ---------------------------------------------------------------------------
// 2. NamedCospan port preservation through composition
// ---------------------------------------------------------------------------

#[test]
fn named_cospan_port_names_survive_composition() {
    // nc1: left_names = ["in_x", "in_y"], right_names = ["mid_a", "mid_b"]
    //   middle has 2 nodes of type 'A', left/right both map [0->0, 1->1]
    let nc1: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1], // left -> middle
        vec![0, 1], // right -> middle
        vec!['A', 'A'],
        vec!["in_x", "in_y"],
        vec!["mid_a", "mid_b"],
    );

    // nc2: left_names = ["mid_a", "mid_b"], right_names = ["out_p", "out_q"]
    //   middle has 2 nodes of type 'A', identity-like mapping
    let nc2: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],
        vec![0, 1],
        vec!['A', 'A'],
        vec!["mid_a", "mid_b"],
        vec!["out_p", "out_q"],
    );

    let composed = nc1.compose(&nc2).expect("should compose");

    // Composition keeps left_names from the first and right_names from the second.
    assert_eq!(composed.left_names(), &vec!["in_x", "in_y"]);
    assert_eq!(composed.right_names(), &vec!["out_p", "out_q"]);

    // Domain and codomain types are preserved.
    assert_eq!(composed.domain(), vec!['A', 'A']);
    assert_eq!(composed.codomain(), vec!['A', 'A']);
}

// ---------------------------------------------------------------------------
// 3. NamedCospan underlying cospan consistency
// ---------------------------------------------------------------------------

#[test]
fn named_cospan_operations_match_inner_cospan() {
    // Build two NamedCospans with a non-trivial middle that forces a pushout.
    // nc1: left=[0,1] right=[0] middle=['X','Y'] -- two left ports into two middle nodes,
    //   one right port into middle node 0
    let nc1: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],
        vec![0],
        vec!['X', 'Y'],
        vec!["a", "b"],
        vec!["c"],
    );

    let c1 = nc1.cospan();

    // nc2: left=[0] right=[0,1] middle=['X','Y'] -- one left port into middle 0,
    //   two right ports into middle nodes 0 and 1
    let nc2: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0],
        vec![0, 1],
        vec!['X', 'Y'],
        vec!["c"],
        vec!["d", "e"],
    );

    let c2 = nc2.cospan();

    // Compose the named cospans and the bare cospans independently.
    let named_composed = nc1.compose(&nc2).expect("named compose");
    let bare_composed = c1.compose(c2).expect("bare compose");

    // The underlying cospan of the named composition must match the bare composition.
    let inner = named_composed.cospan();
    assert!(
        cospan_eq(inner, &bare_composed),
        "NamedCospan composition inner cospan differs from bare Cospan composition:\n  \
         inner.left={:?}, bare.left={:?}\n  inner.right={:?}, bare.right={:?}\n  \
         inner.middle={:?}, bare.middle={:?}",
        inner.left_to_middle(),
        bare_composed.left_to_middle(),
        inner.right_to_middle(),
        bare_composed.right_to_middle(),
        inner.middle(),
        bare_composed.middle(),
    );

    // Domain/codomain consistency.
    assert_eq!(named_composed.domain(), bare_composed.domain());
    assert_eq!(named_composed.codomain(), bare_composed.codomain());
}

// ---------------------------------------------------------------------------
// 4. Permutation cospan via from_permutation
// (LinearCombination ring axioms moved to catgraph-applied::tests::linear_combination_coverage)
// ---------------------------------------------------------------------------

#[test]
fn cospan_from_permutation_structure() {
    use permutations::Permutation;

    let types = &['A', 'B', 'C'];

    // Permutation (0->1, 1->2, 2->0): a 3-cycle (rotation left by 1).
    let p = Permutation::rotation_left(3, 1);

    // types_as_on_domain = true: left is identity, right is permuted.
    let c_dom = Cospan::<char>::from_permutation(p.clone(), types, true).unwrap();

    // Middle should equal the types (the shared set).
    assert_eq!(c_dom.middle(), types, "middle should equal the type labels");

    // Left leg should be identity: [0, 1, 2].
    assert_eq!(
        c_dom.left_to_middle(),
        &[0, 1, 2],
        "left leg should be identity when types_as_on_domain=true"
    );
    assert!(
        c_dom.is_left_identity(),
        "left should be flagged as identity"
    );

    // Right leg is p.inv().permute([0,1,2]).
    // rotation_left(3,1): p(i) = (i+1)%3, so p_inv(i) = (i+2)%3.
    // permute rearranges [0,1,2] by p_inv, yielding [2, 0, 1].
    assert_eq!(
        c_dom.right_to_middle(),
        &[2, 0, 1],
        "right leg should be inverse-permuted indices"
    );

    // Domain types follow the left leg: middle[left[i]] for each i.
    assert_eq!(c_dom.domain(), vec!['A', 'B', 'C']);
    // Codomain types follow the right leg: middle[right[i]] for each i.
    // right = [2, 0, 1] => middle[2]='C', middle[0]='A', middle[1]='B'
    assert_eq!(c_dom.codomain(), vec!['C', 'A', 'B']);

    // types_as_on_domain = false: right is identity, left is permuted.
    let c_cod = Cospan::<char>::from_permutation(p, types, false).unwrap();
    assert_eq!(
        c_cod.right_to_middle(),
        &[0, 1, 2],
        "right leg should be identity when types_as_on_domain=false"
    );
    assert!(
        c_cod.is_right_identity(),
        "right should be flagged as identity"
    );
    assert_eq!(c_cod.middle(), types);
    // Left leg = p.permute([0,1,2]).
    // For rotation_left(3,1): p(i)=(i+1)%3, permute yields [1, 2, 0].
    assert_eq!(c_cod.left_to_middle(), &[1, 2, 0]);
    // Domain follows left: middle[1]='B', middle[2]='C', middle[0]='A'
    assert_eq!(c_cod.domain(), vec!['B', 'C', 'A']);
    assert_eq!(c_cod.codomain(), vec!['A', 'B', 'C']);
}
