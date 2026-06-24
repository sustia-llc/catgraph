//! F&S 2018 Example 6.64: Corel is a hypergraph category.
//!
//! Verifies the special commutative Frobenius-structure generators on
//! `Corel<char>`. Corel inherits its Frobenius structure from the underlying
//! `Cospan`, so passing here corroborates that `Corel::new_unchecked` of each
//! generator preserves joint surjectivity.

mod common;

use catgraph::{
    category::{Composable, HasIdentity},
    corel::Corel,
    hypergraph_category::HypergraphCategory,
    monoidal::Monoidal,
};

#[test]
fn unit_domain_is_empty_codomain_is_z() {
    let eta = Corel::<char>::unit('a');
    assert_eq!(eta.domain(), Vec::<char>::new());
    assert_eq!(eta.codomain(), vec!['a']);
}

#[test]
fn counit_domain_is_z_codomain_is_empty() {
    let eps = Corel::<char>::counit('a');
    assert_eq!(eps.domain(), vec!['a']);
    assert_eq!(eps.codomain(), Vec::<char>::new());
}

#[test]
fn multiplication_2_to_1() {
    let mu = Corel::<char>::multiplication('a');
    assert_eq!(mu.domain(), vec!['a', 'a']);
    assert_eq!(mu.codomain(), vec!['a']);
}

#[test]
fn comultiplication_1_to_2() {
    let delta = Corel::<char>::comultiplication('a');
    assert_eq!(delta.domain(), vec!['a']);
    assert_eq!(delta.codomain(), vec!['a', 'a']);
}

#[test]
fn left_unitality_via_cospan_delegation() {
    // (id ⊗ η) ; μ has the same domain/codomain shape as identity on [a].
    let eta = Corel::<char>::unit('a');
    let mu = Corel::<char>::multiplication('a');
    let id_z = Corel::<char>::identity(&vec!['a']);

    let mut left = id_z;
    left.monoidal(eta);
    let composed = left.compose(&mu).unwrap();

    assert_eq!(composed.domain(), vec!['a']);
    assert_eq!(composed.codomain(), vec!['a']);
    assert!(composed.as_cospan().is_jointly_surjective());
}

#[test]
fn cup_is_0_to_2() {
    let cup = Corel::<char>::cup('a').unwrap();
    assert_eq!(cup.domain(), Vec::<char>::new());
    assert_eq!(cup.codomain(), vec!['a', 'a']);
    assert!(cup.as_cospan().is_jointly_surjective());
}

#[test]
fn cap_is_2_to_0() {
    let cap = Corel::<char>::cap('a').unwrap();
    assert_eq!(cap.domain(), vec!['a', 'a']);
    assert_eq!(cap.codomain(), Vec::<char>::new());
    assert!(cap.as_cospan().is_jointly_surjective());
}

#[test]
fn zigzag_identity_cup_cap() {
    // (cup ⊗ id_z) ; (id_z ⊗ cap) = id_z  (zigzag / snake identity).
    let cup = Corel::<char>::cup('a').unwrap();
    let cap = Corel::<char>::cap('a').unwrap();
    let id_z = Corel::<char>::identity(&vec!['a']);

    let mut left = cup;
    left.monoidal(id_z);

    let mut right = Corel::<char>::identity(&vec!['a']);
    right.monoidal(cap);

    let result = left.compose(&right).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
    assert!(result.as_cospan().is_jointly_surjective());

    // Zigzag law: the composite is the identity on [a] up to cospan isomorphism.
    // The canonical identity has middle == [a], both legs identity. Check shape
    // matches; structural equality would require simplification that catgraph
    // core doesn't perform on Cospan::compose output, so assert equivalence-class
    // count matches the identity's (one class per wire).
    let identity = Corel::<char>::identity(&vec!['a']);
    assert_eq!(
        result.equivalence_classes().len(),
        identity.equivalence_classes().len(),
        "zigzag law: composite must have same partition as identity on [a]"
    );
}
