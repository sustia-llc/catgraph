//! Integration tests for pushout (coequalizer) correctness via the public Cospan API.
//!
//! Tests verify that `compose` (which internally uses union-find pushout)
//! produces correct middle sets, boundary maps, and label preservation.

mod common;
use common::assert_cospan_eq;

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    monoidal::Monoidal,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a cospan from slices for brevity.
fn cospan(left: &[usize], right: &[usize], middle: &[char]) -> Cospan<char> {
    Cospan::new(left.to_vec(), right.to_vec(), middle.to_vec())
}

// ---------------------------------------------------------------------------
// 1. Identity short-circuit: composing with identity preserves structure
// ---------------------------------------------------------------------------

#[test]
fn compose_with_left_identity_preserves_structure() {
    // f : {a,b} -> {a,b,c}
    //   left  = [0,1]  (boundary maps into middle)
    //   right = [1,2]
    //   middle = [a, b, c]
    let f = cospan(&[0, 1], &[1, 2], &['a', 'b', 'c']);

    // id on domain of f
    let id = Cospan::identity(&f.domain());

    // id ; f  should equal f  (left identity law)
    let result = id.compose(&f).expect("id;f should compose");
    assert_eq!(result.left_to_middle(), f.left_to_middle(), "left leg");
    assert_eq!(result.right_to_middle(), f.right_to_middle(), "right leg");
    assert_eq!(result.middle(), f.middle(), "middle set");
}

#[test]
fn compose_with_right_identity_preserves_structure() {
    let f = cospan(&[0, 1], &[1, 2], &['a', 'b', 'c']);

    // id on codomain of f
    let id = Cospan::identity(&f.codomain());

    // f ; id  should equal f  (right identity law)
    let result = f.compose(&id).expect("f;id should compose");
    assert_eq!(result.left_to_middle(), f.left_to_middle(), "left leg");
    assert_eq!(result.right_to_middle(), f.right_to_middle(), "right leg");
    assert_eq!(result.middle(), f.middle(), "middle set");
}

// ---------------------------------------------------------------------------
// 2. Full identification: all boundary nodes map to one middle node
// ---------------------------------------------------------------------------

#[test]
fn full_identification_collapses_to_minimal_middle() {
    // f : 2 -> 2  with everything mapping to a single middle node 'x'
    //   left = [0, 0], right = [0, 0], middle = ['x']
    let f = cospan(&[0, 0], &[0, 0], &['x']);

    // g : 2 -> 2  same structure
    let g = cospan(&[0, 0], &[0, 0], &['x']);

    // f;g pushout: right-of-f and left-of-g both map everything to index 0,
    // so the pushout merges the single middle node of f with the single
    // middle node of g. Result should still have exactly 1 middle node.
    let result = f.compose(&g).expect("full identification should compose");
    assert_eq!(result.middle().len(), 1, "should collapse to 1 middle node");
    assert_eq!(result.middle()[0], 'x', "label should be preserved");
    assert_eq!(result.left_to_middle(), &[0, 0]);
    assert_eq!(result.right_to_middle(), &[0, 0]);
}

// ---------------------------------------------------------------------------
// 3. Disjoint: legs share no targets => pushout middle = union of both
// ---------------------------------------------------------------------------

#[test]
fn disjoint_composition_produces_union_of_middles() {
    // f : {a,b} -> {c,d}
    //   Each boundary node maps to its own distinct middle node.
    //   left = [0, 1], right = [2, 3], middle = [a, b, c, d]
    let f = cospan(&[0, 1], &[2, 3], &['a', 'b', 'c', 'd']);

    // g : {c,d} -> {e,f}
    //   left = [0, 1], right = [2, 3], middle = [c, d, e, f]
    let g = cospan(&[0, 1], &[2, 3], &['c', 'd', 'e', 'f']);

    // f's right maps to middle indices 2,3 (labels c,d).
    // g's left maps to middle indices 0,1 (labels c,d).
    // The pushout identifies f.middle[2] with g.middle[0] (both 'c')
    // and f.middle[3] with g.middle[1] (both 'd').
    // Middle nodes f.middle[0]='a', f.middle[1]='b' have no counterpart in g,
    // and g.middle[2]='e', g.middle[3]='f' have no counterpart in f.
    // So pushout middle has 4+4 - 2 = 6 nodes: {a, b, c, d, e, f}.
    let result = f.compose(&g).expect("disjoint should compose");
    assert_eq!(result.middle().len(), 6, "union of non-shared nodes");

    // Verify all labels are present.
    let mut labels: Vec<char> = result.middle().to_vec();
    labels.sort_unstable();
    assert_eq!(labels, vec!['a', 'b', 'c', 'd', 'e', 'f']);

    // Left boundary should have 2 nodes, right boundary should have 2 nodes.
    assert_eq!(result.left_to_middle().len(), 2);
    assert_eq!(result.right_to_middle().len(), 2);
}

// ---------------------------------------------------------------------------
// 4. Label preservation through pushout
// ---------------------------------------------------------------------------

#[test]
fn labels_survive_pushout_correctly() {
    // f : {x} -> {y}  with middle = [x, m, y]
    //   left = [0], right = [2]
    let f = cospan(&[0], &[2], &['x', 'm', 'y']);

    // g : {y} -> {z}  with middle = [y, n, z]
    //   left = [0], right = [2]
    let g = cospan(&[0], &[2], &['y', 'n', 'z']);

    // Pushout identifies f.middle[2]='y' with g.middle[0]='y'.
    // Result middle should contain all unique labels from both middles,
    // with the shared 'y' appearing exactly once.
    let result = f.compose(&g).expect("should compose");

    let labels: Vec<char> = result.middle().to_vec();
    let count_y = labels.iter().filter(|&&c| c == 'y').count();
    assert_eq!(count_y, 1, "'y' should appear exactly once after merge");

    // Total middle: f has 3, g has 3, 1 pair merged => 5.
    assert_eq!(labels.len(), 5);

    // All original labels present.
    for expected in &['x', 'm', 'y', 'n', 'z'] {
        assert!(
            labels.contains(expected),
            "label '{expected}' missing from pushout middle"
        );
    }

    // Boundary nodes map to correct label types.
    let left_label = result.middle()[result.left_to_middle()[0]];
    assert_eq!(left_label, 'x', "left boundary should point to 'x'");

    let right_label = result.middle()[result.right_to_middle()[0]];
    assert_eq!(right_label, 'z', "right boundary should point to 'z'");
}

// ---------------------------------------------------------------------------
// 5. Determinism: same input always produces the same output
// ---------------------------------------------------------------------------

#[test]
fn pushout_is_deterministic() {
    // f's codomain (right leg labels) must match g's domain (left leg labels).
    // f : right = [1, 2, 0] => codomain labels = [b, c, a]
    // g : left  = [2, 0, 1] => domain labels  = [c, a, b]  -- mismatch!
    //
    // Instead, use a non-trivial pair that IS composable:
    // f : 3 -> 3, with a permuted internal wiring
    //   middle = [a, b, c], left = [0, 1, 2], right = [2, 0, 1]
    //   codomain = [c, a, b]
    // g : 3 -> 3
    //   middle = [c, a, b], left = [0, 1, 2], right = [2, 0, 1]
    //   domain = [c, a, b]  -- matches f's codomain
    let f = cospan(&[0, 1, 2], &[2, 0, 1], &['a', 'b', 'c']);
    let g = cospan(&[0, 1, 2], &[2, 0, 1], &['c', 'a', 'b']);

    // Run composition 10 times and verify identical results.
    let reference = f.compose(&g).expect("should compose");
    for _ in 0..10 {
        let trial = f.compose(&g).expect("should compose");
        assert_cospan_eq(&trial, &reference);
    }
}

// ---------------------------------------------------------------------------
// 6. Wire merging: boundary nodes sharing a middle get merged
// ---------------------------------------------------------------------------

#[test]
fn wire_merging_via_shared_middle_target() {
    // f : {a,a} -> {a}
    //   Both left boundary nodes map to the same middle node as the right.
    //   left = [0, 0], right = [0], middle = ['a']
    let f = cospan(&[0, 0], &[0], &['a']);

    // g : {a} -> {a,a}
    //   left = [0], right = [0, 0], middle = ['a']
    let g = cospan(&[0], &[0, 0], &['a']);

    // f's right has 1 node, g's left has 1 node. They match.
    // Pushout merges f.middle[0] with g.middle[0].
    // Result: 1 middle node, left has 2 boundary, right has 2 boundary.
    let result = f.compose(&g).expect("wire merge should compose");
    assert_eq!(result.middle().len(), 1);
    assert_eq!(result.middle()[0], 'a');
    assert_eq!(result.left_to_middle(), &[0, 0]);
    assert_eq!(result.right_to_middle(), &[0, 0]);
}

// ---------------------------------------------------------------------------
// 7. Associativity: (f;g);h = f;(g;h)
// ---------------------------------------------------------------------------

#[test]
fn composition_is_associative() {
    // Three morphisms forming a pipeline: 2 -> 3 -> 2 -> 3.
    let f = cospan(&[0, 1], &[0, 1, 2], &['a', 'b', 'c']);
    let g = cospan(&[0, 1, 2], &[0, 1], &['a', 'b', 'c']);
    let h = cospan(&[0, 1], &[0, 1, 2], &['a', 'b', 'c']);

    let fg = f.compose(&g).expect("f;g");
    let fg_h = fg.compose(&h).expect("(f;g);h");

    let gh = g.compose(&h).expect("g;h");
    let f_gh = f.compose(&gh).expect("f;(g;h)");

    // Associativity means the resulting domain, codomain, and middle structure
    // are identical (up to the canonical pushout representation).
    assert_eq!(fg_h.domain(), f_gh.domain(), "domains must match");
    assert_eq!(fg_h.codomain(), f_gh.codomain(), "codomains must match");
    assert_eq!(
        fg_h.middle().len(),
        f_gh.middle().len(),
        "middle sizes must match"
    );
    assert_cospan_eq(&fg_h, &f_gh);
}

// ---------------------------------------------------------------------------
// 8. Monoidal + compose interaction: (f⊗g) ; (f'⊗g') = (f;f') ⊗ (g;g')
//    when f,f' and g,g' compose independently
// ---------------------------------------------------------------------------

#[test]
fn monoidal_then_compose_equals_compose_then_monoidal() {
    // f : {a} -> {a},  f2 : {a} -> {a}
    let f = cospan(&[0], &[0], &['a']);
    let f2 = cospan(&[0], &[0], &['a']);

    // g : {b} -> {b},  g' : {b} -> {b}
    let g = cospan(&[0], &[0], &['b']);
    let g2 = cospan(&[0], &[0], &['b']);

    // Path 1: (f ⊗ g) ; (f' ⊗ g')
    let mut tensor_left = f.clone();
    tensor_left.monoidal(g.clone());
    let mut tensor_right = f2.clone();
    tensor_right.monoidal(g2.clone());
    let path1 = tensor_left
        .compose(&tensor_right)
        .expect("monoidal then compose");

    // Path 2: (f;f') ⊗ (g;g')
    let mut composed_f = f.compose(&f2).expect("f;f'");
    let composed_g = g.compose(&g2).expect("g;g'");
    composed_f.monoidal(composed_g);
    let path2 = composed_f;

    assert_cospan_eq(&path1, &path2);
}
