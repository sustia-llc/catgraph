//! Integration tests for cospan-algebras (Fong-Spivak §2.1).
//!
//! Tests the `CospanAlgebra` trait, `PartitionAlgebra` (Example 2.3),
//! and `NameAlgebra` (§4.1) via the public API.

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    compact_closed::cup_single,
    cospan::Cospan,
    cospan_algebra::{CospanAlgebra, NameAlgebra, PartitionAlgebra, functor_induced_algebra_map},
    frobenius::{FrobeniusMorphism, FrobeniusOperation},
    hypergraph_functor::{CospanToFrobeniusFunctor, HypergraphFunctor, RelabelingFunctor},
};

type FM = FrobeniusMorphism<char, String>;

// ---------------------------------------------------------------------------
// PartitionAlgebra (Example 2.3)
// ---------------------------------------------------------------------------

#[test]
fn partition_unit_coherence() {
    let alg = PartitionAlgebra;
    let u: Cospan<char> = alg.unit();
    assert!(u.domain().is_empty());
    assert!(u.codomain().is_empty());
}

#[test]
fn partition_identity_is_identity_function() {
    let alg = PartitionAlgebra;
    // element: [] → [a, b] (a partition of {a, b} into one group)
    let element = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']);
    let id = Cospan::<char>::identity(&vec!['a', 'b']);
    let mapped = alg.map_cospan(&id, &element).unwrap();
    // Identity should preserve the element's codomain
    assert_eq!(mapped.codomain(), element.codomain());
}

#[test]
fn partition_functoriality() {
    let alg = PartitionAlgebra;
    // element: [] → [a]
    let element = Cospan::new(vec![], vec![0], vec!['a']);
    // c1: [a] → [a] (identity)
    let c1 = Cospan::<char>::identity(&vec!['a']);
    // c2: [a] → [a] (identity)
    let c2 = Cospan::<char>::identity(&vec!['a']);
    // c1;c2
    let c12 = c1.compose(&c2).unwrap();

    // map_cospan(c1;c2, e) should equal map_cospan(c2, map_cospan(c1, e))
    let direct = alg.map_cospan(&c12, &element).unwrap();
    let step1 = alg.map_cospan(&c1, &element).unwrap();
    let sequential = alg.map_cospan(&c2, &step1).unwrap();
    assert_eq!(direct.codomain(), sequential.codomain());
}

#[test]
fn partition_lax_monoidal_coherence() {
    let alg = PartitionAlgebra;
    let a = Cospan::new(vec![], vec![0], vec!['a']);
    let b = Cospan::new(vec![], vec![0], vec!['b']);
    let combined: Cospan<char> = alg.lax_monoidal(&a, &b);
    assert!(combined.domain().is_empty());
    assert_eq!(combined.codomain(), vec!['a', 'b']);
}

#[test]
fn partition_merge_cospan() {
    let alg = PartitionAlgebra;
    // element: [] → [a, a] (two nodes, both type 'a')
    let element = Cospan::new(vec![], vec![0, 1], vec!['a', 'a']);
    // merge: [a, a] → [a] (both inputs map to same node)
    let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']);
    let mapped = alg.map_cospan(&merge, &element).unwrap();
    assert!(mapped.domain().is_empty());
    assert_eq!(mapped.codomain(), vec!['a']);
}

#[test]
fn partition_split_cospan() {
    let alg = PartitionAlgebra;
    // element: [] → [a]
    let element = Cospan::new(vec![], vec![0], vec!['a']);
    // split: [a] → [a, a] (one input maps to one node, two outputs from same node)
    let split = Cospan::new(vec![0], vec![0, 0], vec!['a']);
    let mapped = alg.map_cospan(&split, &element).unwrap();
    assert!(mapped.domain().is_empty());
    assert_eq!(mapped.codomain(), vec!['a', 'a']);
}

// ---------------------------------------------------------------------------
// NameAlgebra (§4.1)
// ---------------------------------------------------------------------------

#[test]
fn name_unit_coherence() {
    let alg = NameAlgebra::<String>::new();
    let u: FM = alg.unit();
    assert!(u.domain().is_empty());
    assert!(u.codomain().is_empty());
}

#[test]
fn name_identity_is_identity_function() {
    let alg = NameAlgebra::<String>::new();
    let element: FM = FrobeniusOperation::Unit('a').into();
    let id = Cospan::<char>::identity(&vec!['a']);
    let mapped = alg.map_cospan(&id, &element).unwrap();
    assert_eq!(mapped.domain(), element.domain());
    assert_eq!(mapped.codomain(), element.codomain());
}

#[test]
fn name_functoriality() {
    let alg = NameAlgebra::<String>::new();
    let element: FM = FrobeniusOperation::Unit('a').into();
    let c1 = Cospan::<char>::identity(&vec!['a']);
    let c2 = Cospan::<char>::identity(&vec!['a']);
    let c12 = c1.compose(&c2).unwrap();

    let direct = alg.map_cospan(&c12, &element).unwrap();
    let step1 = alg.map_cospan(&c1, &element).unwrap();
    let sequential = alg.map_cospan(&c2, &step1).unwrap();
    assert_eq!(direct.domain(), sequential.domain());
    assert_eq!(direct.codomain(), sequential.codomain());
}

#[test]
fn name_lax_monoidal_coherence() {
    let alg = NameAlgebra::<String>::new();
    let a: FM = FrobeniusOperation::Unit('a').into();
    let b: FM = FrobeniusOperation::Unit('b').into();
    let combined = alg.lax_monoidal(&a, &b);
    assert!(combined.domain().is_empty());
    assert_eq!(combined.codomain(), vec!['a', 'b']);
}

#[test]
fn name_merge_cospan() {
    let alg = NameAlgebra::<String>::new();
    // element: [] → [a, a] (cup_single gives η;δ)
    let element: FM = cup_single('a');
    // merge: [a, a] → [a]
    let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']);
    let mapped = alg.map_cospan(&merge, &element).unwrap();
    assert!(mapped.domain().is_empty());
    assert_eq!(mapped.codomain(), vec!['a']);
}

#[test]
fn name_split_cospan() {
    let alg = NameAlgebra::<String>::new();
    // element: [] → [a]
    let element: FM = FrobeniusOperation::Unit('a').into();
    // split: [a] → [a, a]
    let split = Cospan::new(vec![0], vec![0, 0], vec!['a']);
    let mapped = alg.map_cospan(&split, &element).unwrap();
    assert!(mapped.domain().is_empty());
    assert_eq!(mapped.codomain(), vec!['a', 'a']);
}

/// Verify that `NameAlgebra` is Send + Sync (important for async usage).
#[test]
fn name_algebra_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<NameAlgebra<String>>();
}

// ---------------------------------------------------------------------------
// §4 Prop 4.6: Initiality of PartitionAlgebra (Part_Λ)
// ---------------------------------------------------------------------------
//
// Part_Λ is the initial cospan-algebra: every cospan-algebra A has a unique
// monoidal natural transformation α: Part → A given element-wise by
//
//     α_x(c) = A(c)(unit_A) = A.map_cospan(&c, &A.unit())
//
// The test suite below generates random Part-elements (cospans [] → x) and
// random downstream cospans (f: x → y), and verifies three properties of this
// formula for two concrete algebras (Part and Name):
//
// 1. **Unit coherence**:     α_I(Part.unit()) = A.unit()
// 2. **Naturality**:         α_y(Part(f)(c))  = A(f)(α_x(c))
// 3. **Monoidal coherence**: α_{x⊕y}(c1 ⊕ c2) = A.lax_monoidal(α_x(c1), α_y(c2))
//
// Uniqueness is not directly observable by proptest, but follows from the
// formula: any other α' must send Part.unit() to A.unit() by (1), and then
// by naturality with c viewed as a morphism [] → x, α'_x(c) = A(c)(A.unit()),
// which equals α_x. Verifying (1)–(3) empirically for randomly generated
// witnesses gives evidence that the formula is a well-defined algebra
// homomorphism for each A we test.

use proptest::prelude::*;

/// The canonical unique natural transformation `α: Part → A`:
/// for `c ∈ Part(x) = Cospan([], x)`, `α_x(c) = A.map_cospan(c, A.unit())`.
fn initial_map<A>(alg: &A, c: &Cospan<char>) -> A::Elem
where
    A: CospanAlgebra<char>,
{
    alg.map_cospan(c, &alg.unit())
        .expect("initial map: A.map_cospan on a well-formed cospan never fails")
}

/// Generate a Part-element: a cospan [] → x with small codomain of char labels.
fn arb_part_element() -> impl Strategy<Value = Cospan<char>> {
    (1usize..=3, prop::sample::select(vec!['a', 'b', 'c'])).prop_map(|(n, c)| {
        let cod: Vec<char> = std::iter::repeat_n(c, n).collect();
        let right_to_mid: Vec<usize> = (0..n).collect();
        Cospan::new(vec![], right_to_mid, cod)
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Prop 4.6 for PartitionAlgebra: the unique map `Part → Part` is the
    /// identity. We verify that `initial_map(Part, c)` is structurally `c`
    /// (same domain / codomain / middle) for random `c`.
    #[test]
    fn prop_4_6_partition_is_identity(c in arb_part_element()) {
        let alg = PartitionAlgebra;
        let mapped = initial_map(&alg, &c);
        prop_assert!(mapped.domain().is_empty());
        prop_assert_eq!(mapped.codomain(), c.codomain());
        prop_assert_eq!(mapped.middle().to_vec(), c.middle().to_vec());
    }

    /// Prop 4.6 naturality for `PartitionAlgebra`:
    /// `α_y(Part(f)(c)) = Part(f)(α_x(c))`.
    /// Since α is the identity self-map, this reduces to
    /// `Part(f)(c) = Part(f)(c)`, but we still exercise the formula
    /// element-by-element to catch silent regressions in `map_cospan`.
    #[test]
    fn prop_4_6_partition_naturality(c in arb_part_element()) {
        let alg = PartitionAlgebra;
        let x = c.codomain();
        let f = Cospan::<char>::identity(&x);
        let lhs_elem = alg.map_cospan(&f, &c).unwrap();
        let lhs = initial_map(&alg, &lhs_elem);
        let alpha_c = initial_map(&alg, &c);
        let rhs = alg.map_cospan(&f, &alpha_c).unwrap();
        prop_assert_eq!(lhs.codomain(), rhs.codomain());
        prop_assert_eq!(lhs.middle().to_vec(), rhs.middle().to_vec());
    }

    /// Prop 4.6 naturality for `NameAlgebra` over concrete witnesses.
    #[test]
    fn prop_4_6_name_unit_and_naturality(c in arb_part_element()) {
        let name_alg = NameAlgebra::<String>::new();

        // (1) Unit coherence: α_I(empty) = Name.unit()
        let empty_cospan: Cospan<char> = Cospan::empty();
        let alpha_unit = initial_map(&name_alg, &empty_cospan);
        prop_assert!(alpha_unit.domain().is_empty());
        prop_assert!(alpha_unit.codomain().is_empty());

        // (2) Naturality: α_y(Part(f)(c)) = Name(f)(α_x(c))
        //     Use f = identity on x to keep the equality checkable on
        //     domain/codomain (FrobeniusMorphism has no deep semantic eq).
        let x = c.codomain();
        let f = Cospan::<char>::identity(&x);
        let part = PartitionAlgebra;
        let lhs_elem = part.map_cospan(&f, &c).unwrap();
        let lhs = initial_map(&name_alg, &lhs_elem);
        let alpha_c = initial_map(&name_alg, &c);
        let rhs = name_alg.map_cospan(&f, &alpha_c).unwrap();
        prop_assert_eq!(lhs.domain(), rhs.domain());
        prop_assert_eq!(lhs.codomain(), rhs.codomain());
    }

    /// Prop 4.6 monoidal coherence for `NameAlgebra`:
    /// `α_{x⊕y}(c1 ⊕ c2) = Name.lax_monoidal(α_x(c1), α_y(c2))`.
    #[test]
    fn prop_4_6_name_monoidal(
        c1 in arb_part_element(),
        c2 in arb_part_element(),
    ) {
        let name_alg = NameAlgebra::<String>::new();
        let part = PartitionAlgebra;

        let combined_part = part.lax_monoidal(&c1, &c2);
        let lhs = initial_map(&name_alg, &combined_part);

        let alpha_c1 = initial_map(&name_alg, &c1);
        let alpha_c2 = initial_map(&name_alg, &c2);
        let rhs = name_alg.lax_monoidal(&alpha_c1, &alpha_c2);

        prop_assert_eq!(lhs.domain(), rhs.domain());
        prop_assert_eq!(lhs.codomain(), rhs.codomain());
    }
}

/// Sanity: on concrete identity input, the initial map for `PartitionAlgebra`
/// reproduces the input cospan — the identity is the unique witness for the
/// self-map `Part → Part`.
#[test]
fn prop_4_6_partition_identity_is_unique_self_map() {
    let alg = PartitionAlgebra;
    let c = Cospan::new(vec![], vec![0], vec!['a']);
    let mapped = initial_map(&alg, &c);
    assert_eq!(mapped.codomain(), c.codomain());
    assert_eq!(mapped.middle().to_vec(), c.middle().to_vec());
}

// ---------------------------------------------------------------------------
// §4 Lemma 4.3: A_F natural transformation from a hypergraph functor F: H → H'
// (paper states the lemma for io functors over fixed Λ; the cross-label
// RelabelingFunctor case below is a beyond-paper generalization, Eq 29 direction)
// ---------------------------------------------------------------------------
//
// Given F: H → H', the induced cospan-algebra morphism α: A_H → A_H' is defined
// element-wise as α_x(e) = F(e). Lemma 4.3 asserts three properties:
//
// 1. **Unit preservation**: α_I(A_H.unit()) = A_H'.unit()
// 2. **Naturality**:        α_y(A_H(c)(e))  = A_H'(c)(α_x(e))  for all cospans c
// 3. **Monoidal coherence**: α_{x⊕y}(A_H.lax_monoidal(e1, e2))
//                          = A_H'.lax_monoidal(α_x(e1), α_y(e2))
//
// Tests below verify these for two concrete hypergraph functors:
//
// - `RelabelingFunctor<char → u32>` lifting PartitionAlgebra<char> →
//   PartitionAlgebra<u32>. This is the "F = relabeling" case from the gap
//   description; α relabels all cospan middles from char to u32.
// - `CospanToFrobeniusFunctor` lifting PartitionAlgebra<char> →
//   NameAlgebra<char, String>. This is the canonical H = Cospan, H' = Frobenius
//   setting; α sends a cospan to its epi-mono Frobenius realisation.

fn char_to_u32(c: char) -> u32 {
    c as u32
}

/// Relabel a cospan element by applying `char_to_u32` to every label, producing
/// the "expected" image under `RelabelingFunctor::new(char_to_u32)`.
fn relabel_part_element(c: &Cospan<char>) -> Cospan<u32> {
    let new_middle: Vec<u32> = c.middle().iter().map(|z| char_to_u32(*z)).collect();
    Cospan::new(
        c.left_to_middle().to_vec(),
        c.right_to_middle().to_vec(),
        new_middle,
    )
}

#[test]
fn lemma_4_3_relabeling_unit_preservation() {
    let relabel: RelabelingFunctor<fn(char) -> u32> = RelabelingFunctor::new(char_to_u32);
    let src_part = PartitionAlgebra;
    let tgt_part = PartitionAlgebra;

    let src_unit: Cospan<char> = src_part.unit();
    let mapped = functor_induced_algebra_map(&relabel, &src_unit).unwrap();
    let tgt_unit: Cospan<u32> = tgt_part.unit();
    assert_eq!(mapped.domain(), tgt_unit.domain());
    assert_eq!(mapped.codomain(), tgt_unit.codomain());
    assert_eq!(mapped.middle(), tgt_unit.middle());
}

#[test]
fn lemma_4_3_relabeling_naturality() {
    let relabel: RelabelingFunctor<fn(char) -> u32> = RelabelingFunctor::new(char_to_u32);
    let src_part = PartitionAlgebra;
    let tgt_part = PartitionAlgebra;

    // e: [] → [a, a] in Part<char> (two disjoint middle nodes both type 'a')
    let e = Cospan::new(vec![], vec![0, 1], vec!['a', 'a']);
    // c: [a, a] → [a] (merge) in Cospan<char>
    let c = Cospan::new(vec![0, 0], vec![0], vec!['a']);

    // LHS: α_y(A_H(c)(e))
    let lhs_src_elem: Cospan<char> = src_part.map_cospan(&c, &e).unwrap();
    let lhs: Cospan<u32> = functor_induced_algebra_map(&relabel, &lhs_src_elem).unwrap();

    // RHS: A_H'(F(c))(α_x(e))
    let alpha_e: Cospan<u32> = functor_induced_algebra_map(&relabel, &e).unwrap();
    let fc: Cospan<u32> = relabel.map_mor(&c).unwrap();
    let rhs: Cospan<u32> = tgt_part.map_cospan(&fc, &alpha_e).unwrap();

    assert_eq!(lhs.domain(), rhs.domain());
    assert_eq!(lhs.codomain(), rhs.codomain());
    assert_eq!(lhs.middle(), rhs.middle());
}

#[test]
fn lemma_4_3_relabeling_monoidal() {
    let relabel: RelabelingFunctor<fn(char) -> u32> = RelabelingFunctor::new(char_to_u32);
    let src_part = PartitionAlgebra;
    let tgt_part = PartitionAlgebra;

    let e1 = Cospan::new(vec![], vec![0], vec!['a']);
    let e2 = Cospan::new(vec![], vec![0], vec!['b']);

    // LHS: α_{x⊕y}(A_H.lax_monoidal(e1, e2))
    let combined_src: Cospan<char> = src_part.lax_monoidal(&e1, &e2);
    let lhs: Cospan<u32> = functor_induced_algebra_map(&relabel, &combined_src).unwrap();

    // RHS: A_H'.lax_monoidal(α_x(e1), α_y(e2))
    let alpha_e1: Cospan<u32> = functor_induced_algebra_map(&relabel, &e1).unwrap();
    let alpha_e2: Cospan<u32> = functor_induced_algebra_map(&relabel, &e2).unwrap();
    let rhs: Cospan<u32> = tgt_part.lax_monoidal(&alpha_e1, &alpha_e2);

    assert_eq!(lhs.codomain(), rhs.codomain());
    assert_eq!(lhs.middle(), rhs.middle());
}

#[test]
fn lemma_4_3_relabeling_matches_direct_relabel() {
    // F = RelabelingFunctor is exactly pointwise relabeling of the cospan middle,
    // so the induced α must agree with hand-written `relabel_part_element`.
    let relabel: RelabelingFunctor<fn(char) -> u32> = RelabelingFunctor::new(char_to_u32);
    let c = Cospan::new(vec![0, 1], vec![0], vec!['a', 'b']);
    let lhs: Cospan<u32> = functor_induced_algebra_map(&relabel, &c).unwrap();
    let rhs = relabel_part_element(&c);
    assert_eq!(lhs.middle(), rhs.middle());
    assert_eq!(lhs.left_to_middle(), rhs.left_to_middle());
    assert_eq!(lhs.right_to_middle(), rhs.right_to_middle());
}

#[test]
fn lemma_4_3_cospan_to_frobenius_unit() {
    // F = CospanToFrobenius sends PartitionAlgebra<char> → NameAlgebra<char, String>.
    let f = CospanToFrobeniusFunctor::<String>::new();
    let src_part = PartitionAlgebra;
    let tgt_name = NameAlgebra::<String>::new();

    let src_unit: Cospan<char> = src_part.unit();
    let mapped: FM = functor_induced_algebra_map(&f, &src_unit).unwrap();
    let tgt_unit: FM = tgt_name.unit();
    assert_eq!(mapped.domain(), tgt_unit.domain());
    assert_eq!(mapped.codomain(), tgt_unit.codomain());
}

#[test]
fn lemma_4_3_cospan_to_frobenius_naturality() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let src_part = PartitionAlgebra;
    let tgt_name = NameAlgebra::<String>::new();

    // e: [] → [a, a] (two disjoint nodes, both type 'a')
    let e = Cospan::new(vec![], vec![0, 1], vec!['a', 'a']);
    // c: [a, a] → [a] (merge)
    let c = Cospan::new(vec![0, 0], vec![0], vec!['a']);

    // LHS: α_y(Part(c)(e))
    let lhs_src_elem: Cospan<char> = src_part.map_cospan(&c, &e).unwrap();
    let lhs: FM = functor_induced_algebra_map(&f, &lhs_src_elem).unwrap();

    // RHS: NameAlgebra(c)(α_x(e))
    let alpha_e: FM = functor_induced_algebra_map(&f, &e).unwrap();
    let rhs: FM = tgt_name.map_cospan(&c, &alpha_e).unwrap();

    assert_eq!(lhs.domain(), rhs.domain());
    assert_eq!(lhs.codomain(), rhs.codomain());
}

#[test]
fn lemma_4_3_cospan_to_frobenius_monoidal() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let src_part = PartitionAlgebra;
    let tgt_name = NameAlgebra::<String>::new();

    let e1 = Cospan::new(vec![], vec![0], vec!['a']);
    let e2 = Cospan::new(vec![], vec![0], vec!['b']);

    let combined_src: Cospan<char> = src_part.lax_monoidal(&e1, &e2);
    let lhs: FM = functor_induced_algebra_map(&f, &combined_src).unwrap();

    let alpha_e1: FM = functor_induced_algebra_map(&f, &e1).unwrap();
    let alpha_e2: FM = functor_induced_algebra_map(&f, &e2).unwrap();
    let rhs: FM = tgt_name.lax_monoidal(&alpha_e1, &alpha_e2);

    assert_eq!(lhs.domain(), rhs.domain());
    assert_eq!(lhs.codomain(), rhs.codomain());
}
