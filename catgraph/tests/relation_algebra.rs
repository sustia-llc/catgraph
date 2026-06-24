#![allow(clippy::similar_names)] // algebraic identities pair up `foo` / `foo_bar` bindings by design

//! Integration tests for the `Span` and `Rel` relation algebra.
//!
//! Tests exercise the public API of `catgraph::span`:
//! - `Span::new`, `Span::dagger`, `Span::is_jointly_injective`
//! - `Span::left`, `Span::right`, `Span::middle_pairs` (accessors)
//! - `HasIdentity::identity`, `Composable::compose`, `Composable::domain`, `Composable::codomain`
//! - `Monoidal::monoidal`
//! - `Rel::new`, `Rel::new_unchecked`, `Rel::identity`, `Rel::compose`, `Rel::as_span`
//! - `Rel` relation properties: `is_reflexive`, `is_irreflexive`, `is_symmetric`,
//!   `is_antisymmetric`, `is_transitive`, `is_homogeneous`, `is_equivalence_rel`, `is_partial_order`
//! - `Rel` set operations: `subsumes`, `union`, `intersection`, `complement`

mod common;
use common::{spans_eq, spans_eq_unordered};

use catgraph::category::{Composable, HasIdentity};
use catgraph::errors::CatgraphError;
use catgraph::monoidal::Monoidal;
use catgraph::span::{Rel, Span};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// 1. Identity span: self-composition yields identity
// ---------------------------------------------------------------------------

#[test]
fn identity_composed_with_itself_is_identity() {
    let types = vec!['a', 'b', 'c'];
    let id = Span::<char>::identity(&types);

    let id_id = id.compose(&id).expect("identity;identity should compose");

    assert_eq!(id_id.domain(), types);
    assert_eq!(id_id.codomain(), types);
    // Pullback of two identity functions produces identity middle pairs.
    assert!(spans_eq(&id, &id_id));
}

// ---------------------------------------------------------------------------
// 2. Dagger involution: dagger(dagger(S)) == S
// ---------------------------------------------------------------------------

#[test]
fn dagger_involution_on_identity() {
    let types = vec!['x', 'y'];
    let id = Span::<char>::identity(&types);
    let double_dagger = id.dagger().dagger();
    assert!(spans_eq(&id, &double_dagger));
}

#[test]
fn dagger_involution_on_general_span() {
    // A non-trivial span: left = [a, b, c], right = [a, b], middle maps some pairs.
    // All middle pairs must have matching types at their indices.
    let left = vec!['a', 'b', 'a'];
    let right = vec!['a', 'b'];
    let middle = vec![(0, 0), (1, 1), (2, 0)];
    let s = Span::new(left, right, middle);

    let dd = s.dagger().dagger();
    assert!(spans_eq(&s, &dd));
}

// ---------------------------------------------------------------------------
// 3. Composition associativity: (R;S);T == R;(S;T)
// ---------------------------------------------------------------------------

#[test]
fn composition_is_associative() {
    // Three composable spans on the same type-set ['a','a','a']
    // so every middle pair is type-compatible.
    let t = vec!['a', 'a', 'a'];

    // R: some subset of {0,1,2} x {0,1,2}
    let r = Span::new(t.clone(), t.clone(), vec![(0, 1), (1, 2)]);
    // S: another subset
    let s = Span::new(t.clone(), t.clone(), vec![(1, 0), (2, 2)]);
    // T: another subset
    let tt = Span::new(t.clone(), t.clone(), vec![(0, 0), (2, 1)]);

    let rs = r.compose(&s).expect("R;S");
    let rs_t = rs.compose(&tt).expect("(R;S);T");

    let st = s.compose(&tt).expect("S;T");
    let r_st = r.compose(&st).expect("R;(S;T)");

    assert!(spans_eq_unordered(&rs_t, &r_st));
}

// ---------------------------------------------------------------------------
// 4. Identity neutrality: id;R == R == R;id
// ---------------------------------------------------------------------------

#[test]
fn identity_is_neutral_for_composition() {
    let types = vec!['a', 'b', 'a'];
    let id_left = Span::<char>::identity(&types);

    // R: a span from types -> types (all 'a'/'b' compatible)
    let r = Span::new(types.clone(), types.clone(), vec![(0, 2), (1, 1), (2, 0)]);

    let id_r = id_left.compose(&r).expect("id;R");
    let r_id = r.compose(&Span::identity(&types)).expect("R;id");

    assert!(spans_eq_unordered(&id_r, &r));
    assert!(spans_eq_unordered(&r_id, &r));
}

// ---------------------------------------------------------------------------
// 5. Dagger of composition: dagger(R;S) == dagger(S);dagger(R)
// ---------------------------------------------------------------------------

#[test]
fn dagger_reverses_composition() {
    let t = vec!['a', 'a', 'a'];
    let r = Span::new(t.clone(), t.clone(), vec![(0, 1), (1, 2), (2, 0)]);
    let s = Span::new(t.clone(), t.clone(), vec![(0, 0), (1, 1), (2, 2)]);

    let rs = r.compose(&s).expect("R;S");
    let dagger_rs = rs.dagger();

    let dagger_s = s.dagger();
    let dagger_r = r.dagger();
    let dagger_s_dagger_r = dagger_s.compose(&dagger_r).expect("dagger(S);dagger(R)");

    assert!(spans_eq_unordered(&dagger_rs, &dagger_s_dagger_r));
}

// ---------------------------------------------------------------------------
// 6. Symmetric span: dagger(R) == R
// ---------------------------------------------------------------------------

#[test]
fn symmetric_span_equals_its_dagger() {
    // A span where swapping left/right and flipping pairs yields the same span.
    // Types must be uniform for this to work: all 'a'.
    let t = vec!['a', 'a'];
    // Symmetric set of pairs: {(0,1), (1,0)} -- flipping each gives the same set.
    let s = Span::new(t.clone(), t.clone(), vec![(0, 1), (1, 0)]);
    let d = s.dagger();

    assert!(spans_eq_unordered(&s, &d));
}

// ---------------------------------------------------------------------------
// 7. Rel identity through public trait API
// ---------------------------------------------------------------------------

#[test]
fn rel_identity_roundtrips_through_as_span() {
    let types = vec!['x', 'y', 'z'];
    let rel_id = Rel::<char>::identity(&types);

    assert_eq!(rel_id.domain(), types);
    assert_eq!(rel_id.codomain(), types);

    let span = rel_id.as_span();
    assert_eq!(span.left(), types.as_slice());
    assert_eq!(span.right(), types.as_slice());
    assert!(span.is_jointly_injective());
    // Identity relation has diagonal pairs.
    assert_eq!(span.middle_pairs(), &[(0, 0), (1, 1), (2, 2)]);
}

// ---------------------------------------------------------------------------
// 8. Rel composition preserves joint injectivity
// ---------------------------------------------------------------------------

#[test]
fn rel_compose_identity_preserves_structure() {
    let types = vec!['a', 'b'];
    let id = Rel::<char>::identity(&types);

    // id;id should give back the identity relation.
    let composed = id.compose(&id).expect("rel id;id should compose");
    let span = composed.as_span();

    assert_eq!(span.left(), types.as_slice());
    assert_eq!(span.right(), types.as_slice());
    assert!(span.is_jointly_injective());
    assert_eq!(span.middle_pairs(), &[(0, 0), (1, 1)]);
}

// ---------------------------------------------------------------------------
// 9. Rel monoidal (tensor product of identity relations)
// ---------------------------------------------------------------------------

#[test]
fn rel_monoidal_tensors_identities() {
    let mut r1 = Rel::<char>::identity(&vec!['a']);
    let r2 = Rel::<char>::identity(&vec!['b']);

    r1.monoidal(r2);

    assert_eq!(r1.domain(), vec!['a', 'b']);
    assert_eq!(r1.codomain(), vec!['a', 'b']);

    let span = r1.as_span();
    assert!(span.is_jointly_injective());
    assert_eq!(span.middle_pairs(), &[(0, 0), (1, 1)]);
}

// ---------------------------------------------------------------------------
// 10. Dagger of identity is identity
// ---------------------------------------------------------------------------

#[test]
fn dagger_of_identity_is_identity() {
    let types = vec!['p', 'q', 'r', 's'];
    let id = Span::<char>::identity(&types);
    let d = id.dagger();

    // Identity span has (i,i) pairs; dagger flips to (i,i) -- same span.
    assert!(spans_eq(&id, &d));
}

// ---------------------------------------------------------------------------
// 11. Rel::new validates joint injectivity
// ---------------------------------------------------------------------------

#[test]
fn rel_new_validates_joint_injectivity() {
    // Build a non-jointly-injective span: duplicate (0, 0) pair
    let left = vec!['a', 'a'];
    let right = vec!['a', 'a'];
    let span = Span::new(left, right, vec![(0, 0), (0, 0)]);
    assert!(!span.is_jointly_injective());

    let result = Rel::new(span);
    assert!(matches!(result, Err(CatgraphError::Relation { .. })));
}

// ---------------------------------------------------------------------------
// 12. Rel::new_unchecked bypasses validation
// ---------------------------------------------------------------------------

#[test]
fn rel_new_unchecked_bypasses() {
    // Same non-jointly-injective span
    let left = vec!['a', 'a'];
    let right = vec!['a', 'a'];
    let span = Span::new(left, right, vec![(0, 0), (0, 0)]);
    assert!(!span.is_jointly_injective());

    // new_unchecked should succeed without checking
    let rel = Rel::new_unchecked(span);
    assert_eq!(rel.as_span().middle_pairs(), &[(0, 0), (0, 0)]);
}

// ---------------------------------------------------------------------------
// 13. Rel equivalence relation
// ---------------------------------------------------------------------------

#[test]
fn rel_equivalence_relation() {
    // Partition equivalence on ['a','a','a','a']:
    // Block {0,1} and block {2,3}
    // Pairs: (0,0),(0,1),(1,0),(1,1),(2,2),(2,3),(3,2),(3,3)
    let types = vec!['a', 'a', 'a', 'a'];
    let pairs = vec![
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (2, 2),
        (2, 3),
        (3, 2),
        (3, 3),
    ];
    let rel = Rel::new(Span::new(types.clone(), types, pairs)).unwrap();

    assert!(rel.is_equivalence_rel());
    // An equivalence relation is reflexive, symmetric, transitive
    assert!(rel.is_reflexive());
    assert!(rel.is_symmetric());
    assert!(rel.is_transitive());
    assert!(rel.is_homogeneous());
}

// ---------------------------------------------------------------------------
// 14. Rel partial order
// ---------------------------------------------------------------------------

#[test]
fn rel_partial_order() {
    // Total order on 3 elements: 0 <= 1 <= 2
    // Pairs: (0,0),(0,1),(0,2),(1,1),(1,2),(2,2)
    let types = vec!['a', 'a', 'a'];
    let pairs = vec![(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (2, 2)];
    let rel = Rel::new(Span::new(types.clone(), types, pairs)).unwrap();

    assert!(rel.is_partial_order());
    assert!(rel.is_reflexive());
    assert!(rel.is_antisymmetric());
    assert!(rel.is_transitive());
}

// ---------------------------------------------------------------------------
// 15. Rel not transitive
// ---------------------------------------------------------------------------

#[test]
fn rel_not_transitive() {
    // Reflexive + symmetric but NOT transitive:
    // {(0,0),(1,1),(2,2),(0,1),(1,0),(1,2),(2,1)} but NOT (0,2) or (2,0)
    let types = vec!['a', 'a', 'a'];
    let pairs = vec![(0, 0), (1, 1), (2, 2), (0, 1), (1, 0), (1, 2), (2, 1)];
    let rel = Rel::new(Span::new(types.clone(), types, pairs)).unwrap();

    assert!(rel.is_reflexive());
    assert!(rel.is_symmetric());
    assert!(!rel.is_transitive());
    // Not an equivalence relation because not transitive
    assert!(!rel.is_equivalence_rel());
}

// ---------------------------------------------------------------------------
// 16. Rel union and intersection
// ---------------------------------------------------------------------------

#[test]
fn rel_union_intersection() {
    let types = vec!['a', 'a', 'a'];
    let r1 = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 0), (0, 1), (1, 1)],
    ))
    .unwrap();
    let r2 = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 1), (1, 1), (2, 2)],
    ))
    .unwrap();

    let u = r1.union(&r2).unwrap();
    let u_pairs: HashSet<(usize, usize)> = u.as_span().middle_pairs().iter().copied().collect();
    // Union should have all pairs from both: (0,0),(0,1),(1,1),(2,2)
    assert_eq!(u_pairs.len(), 4);
    assert!(u_pairs.contains(&(0, 0)));
    assert!(u_pairs.contains(&(0, 1)));
    assert!(u_pairs.contains(&(1, 1)));
    assert!(u_pairs.contains(&(2, 2)));

    let i = r1.intersection(&r2).unwrap();
    let i_pairs: HashSet<(usize, usize)> = i.as_span().middle_pairs().iter().copied().collect();
    // Intersection should have common pairs: (0,1),(1,1)
    assert_eq!(i_pairs.len(), 2);
    assert!(i_pairs.contains(&(0, 1)));
    assert!(i_pairs.contains(&(1, 1)));
}

// ---------------------------------------------------------------------------
// 17. Rel complement involution
// ---------------------------------------------------------------------------

#[test]
fn rel_complement_involution() {
    let types = vec!['a', 'a', 'a'];
    let r = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 0), (1, 2), (2, 1)],
    ))
    .unwrap();

    let comp = r.complement().expect("complement should succeed");
    let double_comp = comp.complement().expect("double complement should succeed");

    // complement(complement(r)) == r (as sets of pairs)
    let original: HashSet<(usize, usize)> = r.as_span().middle_pairs().iter().copied().collect();
    let roundtrip: HashSet<(usize, usize)> = double_comp
        .as_span()
        .middle_pairs()
        .iter()
        .copied()
        .collect();
    assert_eq!(original, roundtrip);

    // Also verify complement has the right size: 3*3 - 3 = 6 pairs
    assert_eq!(comp.as_span().middle_pairs().len(), 6);
}

// ---------------------------------------------------------------------------
// 18. Rel subsumes
// ---------------------------------------------------------------------------

#[test]
fn rel_subsumes() {
    let types = vec!['a', 'a', 'a'];
    let full = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 0), (0, 1), (1, 1), (2, 2)],
    ))
    .unwrap();
    let partial = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 0), (1, 1)],
    ))
    .unwrap();

    assert!(full.subsumes(&partial).unwrap());
    assert!(!partial.subsumes(&full).unwrap());
    // Every relation subsumes itself
    assert!(full.subsumes(&full).unwrap());
}

// ---------------------------------------------------------------------------
// 19. Rel is_irreflexive
// ---------------------------------------------------------------------------

#[test]
fn rel_is_irreflexive() {
    // Strict order: no diagonal pairs
    let types = vec!['a', 'a', 'a'];
    let strict = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 1), (0, 2), (1, 2)],
    ))
    .unwrap();

    assert!(strict.is_irreflexive());
    assert!(!strict.is_reflexive());
    // Adding a diagonal pair breaks irreflexivity
    let not_strict = Rel::new(Span::new(
        types.clone(),
        types.clone(),
        vec![(0, 0), (0, 1), (0, 2), (1, 2)],
    ))
    .unwrap();
    assert!(!not_strict.is_irreflexive());
}

// ---------------------------------------------------------------------------
// 20. Rel heterogeneous (not homogeneous)
// ---------------------------------------------------------------------------

#[test]
fn rel_heterogeneous() {
    // Different domain and codomain labels
    let domain = vec!['a', 'b'];
    let codomain = vec!['x', 'y'];
    // Empty middle — no pairs to check type compatibility
    let rel = Rel::new(Span::new(domain, codomain, vec![])).unwrap();

    assert!(!rel.is_homogeneous());
}
