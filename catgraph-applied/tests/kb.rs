//! Tests for the congruence-closure engine in `prop::presentation::kb`.

use catgraph_applied::prop::presentation::kb::CongruenceClosure;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum G {
    A,
    B,
    C,
}

impl PropSignature for G {
    fn source(&self) -> usize {
        1
    }
    fn target(&self) -> usize {
        1
    }
}

fn g(x: G) -> PropExpr<G> {
    Free::<G>::generator(x)
}

#[test]
fn cc_empty_is_reflexive() {
    let mut cc = CongruenceClosure::<G>::new(&[]);
    assert!(cc.are_equal(&g(G::A), &g(G::A)));
}

#[test]
fn cc_distinct_generators_not_equal() {
    let mut cc = CongruenceClosure::<G>::new(&[]);
    assert!(!cc.are_equal(&g(G::A), &g(G::B)));
}

#[test]
fn cc_seeded_equivalence_direct() {
    let mut cc = CongruenceClosure::<G>::new(&[(g(G::A), g(G::B))]);
    assert!(cc.are_equal(&g(G::A), &g(G::B)));
    assert!(cc.are_equal(&g(G::B), &g(G::A))); // symmetry
}

#[test]
fn cc_seeded_transitivity() {
    let mut cc = CongruenceClosure::<G>::new(&[(g(G::A), g(G::B)), (g(G::B), g(G::C))]);
    assert!(cc.are_equal(&g(G::A), &g(G::C))); // transitivity
}

#[test]
fn cc_congruence_through_compose() {
    // If A = B, then A ; A ~ B ; B via congruence.
    let mut cc = CongruenceClosure::<G>::new(&[(g(G::A), g(G::B))]);
    let aa = Free::<G>::compose(g(G::A), g(G::A)).unwrap();
    let bb = Free::<G>::compose(g(G::B), g(G::B)).unwrap();
    assert!(cc.are_equal(&aa, &bb));
}

#[test]
fn cc_congruence_through_tensor() {
    // If A = B, then A ⊗ C ~ B ⊗ C via congruence.
    let mut cc = CongruenceClosure::<G>::new(&[(g(G::A), g(G::B))]);
    let ac = Free::<G>::tensor(g(G::A), g(G::C));
    let bc = Free::<G>::tensor(g(G::B), g(G::C));
    assert!(cc.are_equal(&ac, &bc));
}

#[test]
fn cc_overlapping_equations_converge() {
    // The v0.5.0 killer case: overlapping scalar equations must join.
    // Simulate with: A ; A = B, A = C. Then A ; C = A ; A = B, so A ; C = B.
    let mut cc = CongruenceClosure::<G>::new(&[
        (Free::<G>::compose(g(G::A), g(G::A)).unwrap(), g(G::B)),
        (g(G::A), g(G::C)),
    ]);
    let ac = Free::<G>::compose(g(G::A), g(G::C)).unwrap();
    assert!(
        cc.are_equal(&ac, &g(G::B)),
        "overlapping equations should join under congruence closure"
    );
}

#[test]
fn cc_handles_deep_tensor_nesting() {
    // Left-associated 5-fold tensor vs right-associated: structurally different,
    // NOT equal in congruence closure (which doesn't apply associativity
    // unless given the equation). Verify cc correctly distinguishes these.
    let mut cc = CongruenceClosure::<G>::new(&[]);
    let mut lhs = g(G::A);
    for _ in 0..4 {
        lhs = Free::<G>::tensor(lhs, g(G::A));
    }
    let inner = Free::<G>::tensor(g(G::A), g(G::A));
    let mid = Free::<G>::tensor(g(G::A), inner);
    let rhs = Free::<G>::tensor(g(G::A), mid);
    // Same shape (5 A's tensored) but structurally different parenthesization
    // → NOT equal without assoc equation.
    assert!(!cc.are_equal(&lhs, &rhs));
}

#[test]
fn cc_with_assoc_equation_joins_nesting() {
    // Given (A ⊗ A) ⊗ A = A ⊗ (A ⊗ A), the direct equation holds.
    let aa = Free::<G>::tensor(g(G::A), g(G::A));
    let left_3 = Free::<G>::tensor(aa.clone(), g(G::A));
    let right_3 = Free::<G>::tensor(g(G::A), aa.clone());
    let mut cc = CongruenceClosure::<G>::new(&[(left_3.clone(), right_3.clone())]);
    assert!(cc.are_equal(&left_3, &right_3));
}

#[test]
fn cc_mixed_compose_tensor_congruence() {
    // Seed A = B. The test term layers BOTH tags in a single expression:
    //
    //   lhs = (A ⊗ C) ; (C ⊗ C)           -- outer Compose, inner Tensor
    //   rhs = (B ⊗ C) ; (C ⊗ C)
    //
    // Arities (all G generators are 1→1):
    //   A ⊗ C : 2 → 2,  C ⊗ C : 2 → 2,   so (A ⊗ C) ; (C ⊗ C) : 2 → 2.
    //
    // For lhs = rhs the engine must propagate A = B congruence through:
    //   (1) the inner Tensor:   A ⊗ C  =  B ⊗ C     (Tag::Tensor)
    //   (2) the outer Compose:  (A⊗C);(C⊗C) = (B⊗C);(C⊗C)   (Tag::Compose)
    //
    // If propagation were one-tag-only, the assertion would fail.
    let mut cc = CongruenceClosure::<G>::new(&[(g(G::A), g(G::B))]);
    let cc_tensor = Free::<G>::tensor(g(G::C), g(G::C)); // 2 → 2

    let ac = Free::<G>::tensor(g(G::A), g(G::C)); // 2 → 2
    let bc = Free::<G>::tensor(g(G::B), g(G::C)); // 2 → 2
    let lhs = Free::<G>::compose(ac, cc_tensor.clone()).unwrap();
    let rhs = Free::<G>::compose(bc, cc_tensor).unwrap();

    assert!(
        cc.are_equal(&lhs, &rhs),
        "A = B must propagate through both Tensor and Compose in a mixed term"
    );
}
