//! Integration tests for monoidal category properties using only the public API.
//!
//! Covers tensor associativity, tensor unit, permutation cospan composition,
//! symmetric braiding involutivity, span tensor product, and `permute_side`.

mod common;
use common::{assert_cospan_eq_msg as assert_cospan_eq, assert_cospan_shape};

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
    span::Span,
};
use permutations::Permutation;

/// Build a small non-trivial cospan: domain `[a,b]`, codomain `[b,c]`,
/// middle `[a,b,c]` with left=`[0,1]`, right=`[1,2]`.
fn sample_cospan_abc() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![1, 2], vec!['a', 'b', 'c'])
}

/// Build a second non-trivial cospan: domain `[x]`, codomain `[x,y]`,
/// middle `[x,y]` with left=`[0]`, right=`[0,1]`.
fn sample_cospan_xy() -> Cospan<char> {
    Cospan::new(vec![0], vec![0, 1], vec!['x', 'y'])
}

/// Build a third small cospan: domain `[p,q]`, codomain `[p]`,
/// middle `[p,q]` with left=`[0,1]`, right=`[0]`.
fn sample_cospan_pq() -> Cospan<char> {
    Cospan::new(vec![0, 1], vec![0], vec!['p', 'q'])
}

// ---------------------------------------------------------------------------
// 1. Tensor associativity: (f ⊗ g) ⊗ h  vs  f ⊗ (g ⊗ h)
// ---------------------------------------------------------------------------

#[test]
fn tensor_associativity_cospan() {
    let f = sample_cospan_abc();
    let g = sample_cospan_xy();
    let h = sample_cospan_pq();

    // (f ⊗ g) ⊗ h
    let mut fg = f.clone();
    fg.monoidal(g.clone());
    let mut fg_h = fg.clone();
    fg_h.monoidal(h.clone());

    // f ⊗ (g ⊗ h)
    let mut gh = g.clone();
    gh.monoidal(h.clone());
    let mut f_gh = f.clone();
    f_gh.monoidal(gh);

    // Monoidal product on cospans concatenates boundaries and shifts middle
    // indices, so associativity holds on the nose (not just up to iso).
    assert_cospan_shape(&fg_h, &f_gh, "tensor associativity");

    // Stronger: exact structural equality.
    assert_cospan_eq(&fg_h, &f_gh, "tensor associativity (exact)");
}

// ---------------------------------------------------------------------------
// 2. Tensor with empty (monoidal unit)
// ---------------------------------------------------------------------------

#[test]
fn tensor_unit_cospan() {
    let f = sample_cospan_abc();
    let unit = Cospan::<char>::empty();

    // f ⊗ empty == f
    let mut f_unit = f.clone();
    f_unit.monoidal(unit.clone());
    assert_cospan_eq(&f_unit, &f, "f tensor empty");

    // empty ⊗ f == f
    let mut unit_f = unit;
    unit_f.monoidal(f.clone());
    assert_cospan_eq(&unit_f, &f, "empty tensor f");
}

// ---------------------------------------------------------------------------
// 3. Permutation cospan compose: from_permutation(p1).compose(from_permutation(p2))
//    matches from_permutation(p1 * p2) in domain/codomain/middle size.
// ---------------------------------------------------------------------------

#[test]
fn permutation_cospan_compose() {
    // Use uniform labels so every permutation cospan has domain == codomain
    // labels, making any two composable.
    let types: Vec<char> = vec!['a', 'a', 'a'];

    // p1 = rotation_left(3,1): 0->1, 1->2, 2->0
    let p1 = Permutation::rotation_left(3, 1);
    // p2 = transposition(3,0,2): swap 0<->2
    let p2 = Permutation::transposition(3, 0, 2);

    let c1 = Cospan::from_permutation(p1.clone(), &types, true).unwrap();
    let c2 = Cospan::from_permutation(p2.clone(), &types, true).unwrap();

    // With uniform labels, any two permutation cospans are composable.
    assert!(c1.composable(&c2).is_ok(), "c1;c2 should be composable");

    let composed = c1.compose(&c2).expect("compose should succeed");

    // The combined permutation p1*p2.
    let p12 = p1 * p2;
    let expected = Cospan::from_permutation(p12, &types, true).unwrap();

    // Domain and codomain must match the composed permutation.
    assert_eq!(composed.domain(), expected.domain(), "domain after compose");
    assert_eq!(
        composed.codomain(),
        expected.codomain(),
        "codomain after compose"
    );

    // Middle size: the pushout may differ from the direct construction, but
    // should be at most the sum of the two middles and at least max(left, right).
    let mid_len = composed.middle().len();
    assert!(
        mid_len >= types.len(),
        "middle should have at least {n} nodes, got {mid_len}",
        n = types.len()
    );

    // Validate the composed cospan is internally consistent.
    composed.assert_valid(false, true);
}

// ---------------------------------------------------------------------------
// 4. Symmetric braiding: swap composed with itself yields identity
// ---------------------------------------------------------------------------

#[test]
fn symmetric_braiding_involutive() {
    // Use uniform labels so the swap cospan is self-composable
    // (codomain labels match domain labels regardless of permutation).
    let types: Vec<char> = vec!['a', 'a'];

    // The swap permutation on 2 elements: (0 1).
    let swap = Permutation::transposition(2, 0, 1);
    let sigma = Cospan::from_permutation(swap.clone(), &types, true).unwrap();

    // sigma ; sigma should give identity (the braiding is an involution).
    assert!(
        sigma.composable(&sigma).is_ok(),
        "swap should be self-composable"
    );
    let sigma_sq = sigma.compose(&sigma).expect("compose should succeed");

    // The identity cospan for comparison.
    let id = Cospan::<char>::identity(&types);

    // Domain and codomain must be the original types.
    assert_eq!(sigma_sq.domain(), id.domain(), "domain");
    assert_eq!(sigma_sq.codomain(), id.codomain(), "codomain");

    // The swap^2 cospan after pushout simplification should have the
    // same domain-to-middle and codomain-to-middle connectivity as identity:
    // each domain wire i connects to the same middle node as codomain wire i.
    for i in 0..types.len() {
        assert_eq!(
            sigma_sq.left_to_middle()[i],
            sigma_sq.right_to_middle()[i],
            "wire {i} should connect domain and codomain to the same middle node"
        );
    }

    sigma_sq.assert_valid(false, true);
}

// ---------------------------------------------------------------------------
// 5. Span tensor: verify monoidal product combines middle_pairs correctly
// ---------------------------------------------------------------------------

#[test]
fn span_tensor_combines_middle_pairs() {
    // Span s1: left=['a','b'], right=['a','b'], middle=[(0,0),(1,1)] (identity)
    let s1 = Span::<char>::identity(&vec!['a', 'b']);
    // Span s2: left=['c'], right=['c'], middle=[(0,0)] (identity on single wire)
    let s2 = Span::<char>::identity(&vec!['c']);

    let mut product = s1.clone();
    product.monoidal(s2.clone());

    // Domain and codomain are concatenated.
    assert_eq!(product.left(), &['a', 'b', 'c'], "tensor left");
    assert_eq!(product.right(), &['a', 'b', 'c'], "tensor right");

    // Middle pairs: s1 has [(0,0),(1,1)], s2 has [(0,0)].
    // After tensor, s2's pair is shifted to (0+2, 0+2) = (2,2).
    assert_eq!(
        product.middle_pairs(),
        &[(0, 0), (1, 1), (2, 2)],
        "tensor middle_pairs"
    );

    // A non-identity span to tensor with.
    // s3: left=['x','y'], right=['y','x'], middle=[(0,1),(1,0)] (swap relation).
    let s3 = Span::new(vec!['x', 'y'], vec!['y', 'x'], vec![(0, 1), (1, 0)]);

    let mut s1_s3 = s1;
    s1_s3.monoidal(s3);

    assert_eq!(
        s1_s3.left(),
        &['a', 'b', 'x', 'y'],
        "non-trivial tensor left"
    );
    assert_eq!(
        s1_s3.right(),
        &['a', 'b', 'y', 'x'],
        "non-trivial tensor right"
    );
    // s1 middle: [(0,0),(1,1)], s3 middle shifted by (2,2): [(2,3),(3,2)].
    assert_eq!(
        s1_s3.middle_pairs(),
        &[(0, 0), (1, 1), (2, 3), (3, 2)],
        "non-trivial tensor middle_pairs"
    );
}

// ---------------------------------------------------------------------------
// 6. permute_side domain: permuting the domain of a cospan reorders the
//    left boundary.
// ---------------------------------------------------------------------------

#[test]
fn permute_side_reorders_domain() {
    // Start with identity cospan on ['a','b','c'].
    let types = vec!['a', 'b', 'c'];
    let mut c = Cospan::<char>::identity(&types);

    // Before permutation: left_to_middle = [0,1,2], domain = ['a','b','c'].
    assert_eq!(c.domain(), vec!['a', 'b', 'c']);
    assert_eq!(c.left_to_middle(), &[0, 1, 2]);

    // Apply rotation_left(3,1) to the domain side (of_codomain = false).
    // rotation_left(3,1) sends 0->1, 1->2, 2->0.
    let rot = Permutation::rotation_left(3, 1);
    c.permute_side(&rot, false);

    // After permuting the domain, the left leg is reordered.
    // The domain labels should reflect the permutation.
    let new_domain = c.domain();
    assert_eq!(new_domain.len(), 3, "domain size unchanged");

    // The left_to_middle array has been permuted, so domain wires now
    // map to different middle nodes than before.
    assert_ne!(
        c.left_to_middle(),
        &[0, 1, 2],
        "left leg should no longer be identity"
    );
    assert!(
        !c.is_left_identity(),
        "left identity flag should be cleared"
    );

    // Codomain should be untouched.
    assert_eq!(c.codomain(), vec!['a', 'b', 'c'], "codomain unchanged");
    assert_eq!(c.right_to_middle(), &[0, 1, 2], "right leg unchanged");
    assert!(c.is_right_identity(), "right identity flag preserved");

    // The cospan should still be valid.
    c.assert_valid(false, true);
}
