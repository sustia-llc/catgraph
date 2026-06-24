//! Integration tests for `HypergraphFunctor` trait (Fong-Spivak §2.3).
//!
//! Verifies Frobenius preservation (Eq. 12), functoriality, identity,
//! monoidal preservation, and derived cup/cap for `RelabelingFunctor`.

mod common;

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    cospan::Cospan,
    frobenius::FrobeniusMorphism,
    hypergraph_category::HypergraphCategory,
    hypergraph_functor::{CospanToFrobeniusFunctor, HypergraphFunctor, RelabelingFunctor},
    monoidal::Monoidal,
};
use common::assert_cospan_eq_msg;

fn char_to_u32(c: char) -> u32 {
    c as u32
}

type FM = FrobeniusMorphism<char, String>;

// ---------------------------------------------------------------------------
// Frobenius preservation (Eq. 12)
// ---------------------------------------------------------------------------

#[test]
fn frobenius_unit_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let z = 'a';
    let src_unit = Cospan::<char>::unit(z);
    let mapped = f.map_mor(&src_unit).unwrap();
    let tgt_unit = Cospan::<u32>::unit(f.map_ob(z));
    assert_cospan_eq_msg(&mapped, &tgt_unit, "F(η_x) = η_{F(x)}");
}

#[test]
fn frobenius_counit_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let z = 'b';
    let src_counit = Cospan::<char>::counit(z);
    let mapped = f.map_mor(&src_counit).unwrap();
    let tgt_counit = Cospan::<u32>::counit(f.map_ob(z));
    assert_cospan_eq_msg(&mapped, &tgt_counit, "F(ε_x) = ε_{F(x)}");
}

#[test]
fn frobenius_multiplication_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let z = 'c';
    let src_mul = Cospan::<char>::multiplication(z);
    let mapped = f.map_mor(&src_mul).unwrap();
    let tgt_mul = Cospan::<u32>::multiplication(f.map_ob(z));
    assert_cospan_eq_msg(&mapped, &tgt_mul, "F(μ_x) = μ_{F(x)}");
}

#[test]
fn frobenius_comultiplication_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let z = 'd';
    let src_comul = Cospan::<char>::comultiplication(z);
    let mapped = f.map_mor(&src_comul).unwrap();
    let tgt_comul = Cospan::<u32>::comultiplication(f.map_ob(z));
    assert_cospan_eq_msg(&mapped, &tgt_comul, "F(δ_x) = δ_{F(x)}");
}

// ---------------------------------------------------------------------------
// Functoriality
// ---------------------------------------------------------------------------

#[test]
fn functoriality_composition() {
    let f = RelabelingFunctor::new(char_to_u32);
    // g: [a] → [a, a] (comultiplication), h: [a, a] → [a] (multiplication)
    let g = Cospan::<char>::comultiplication('a');
    let h = Cospan::<char>::multiplication('a');

    // map_mor(g ; h) should equal map_mor(g) ; map_mor(h)
    let composed_then_mapped = f.map_mor(&g.compose(&h).unwrap()).unwrap();
    let mapped_g = f.map_mor(&g).unwrap();
    let mapped_h = f.map_mor(&h).unwrap();
    let mapped_then_composed = mapped_g.compose(&mapped_h).unwrap();

    assert_cospan_eq_msg(
        &composed_then_mapped,
        &mapped_then_composed,
        "F(g;h) = F(g);F(h)",
    );
}

#[test]
fn functoriality_identity() {
    let f = RelabelingFunctor::new(char_to_u32);
    let types = vec!['a', 'b', 'c'];
    let src_id = Cospan::<char>::identity(&types);
    let mapped = f.map_mor(&src_id).unwrap();
    let tgt_types: Vec<u32> = types.iter().map(|c| f.map_ob(*c)).collect();
    let tgt_id = Cospan::<u32>::identity(&tgt_types);
    assert_cospan_eq_msg(&mapped, &tgt_id, "F(id_x) = id_{F(x)}");
}

// ---------------------------------------------------------------------------
// Monoidal preservation
// ---------------------------------------------------------------------------

#[test]
fn monoidal_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let g = Cospan::<char>::unit('a');
    let h = Cospan::<char>::counit('b');

    // map_mor(g ⊗ h) should equal map_mor(g) ⊗ map_mor(h)
    let mut tensor = g.clone();
    tensor.monoidal(h.clone());
    let mapped_tensor = f.map_mor(&tensor).unwrap();

    let mut mapped_parts = f.map_mor(&g).unwrap();
    mapped_parts.monoidal(f.map_mor(&h).unwrap());

    assert_cospan_eq_msg(&mapped_tensor, &mapped_parts, "F(g⊗h) = F(g)⊗F(h)");
}

// ---------------------------------------------------------------------------
// Derived structure
// ---------------------------------------------------------------------------

#[test]
fn relabeling_cup_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let z = 'x';
    let src_cup = Cospan::<char>::cup(z).unwrap();
    let mapped = f.map_mor(&src_cup).unwrap();
    let tgt_cup = Cospan::<u32>::cup(f.map_ob(z)).unwrap();
    assert_cospan_eq_msg(&mapped, &tgt_cup, "F(cup_x) = cup_{F(x)}");
}

#[test]
fn relabeling_cap_preservation() {
    let f = RelabelingFunctor::new(char_to_u32);
    let z = 'y';
    let src_cap = Cospan::<char>::cap(z).unwrap();
    let mapped = f.map_mor(&src_cap).unwrap();
    let tgt_cap = Cospan::<u32>::cap(f.map_ob(z)).unwrap();
    assert_cospan_eq_msg(&mapped, &tgt_cap, "F(cap_x) = cap_{F(x)}");
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_boundaries() {
    let f = RelabelingFunctor::new(char_to_u32);
    let empty = Cospan::<char>::empty();
    let mapped = f.map_mor(&empty).unwrap();
    assert!(mapped.domain().is_empty());
    assert!(mapped.codomain().is_empty());
    assert!(mapped.middle().is_empty());
}

#[test]
fn relabeling_roundtrip_invertible() {
    // char → u32 → char roundtrip preserves structure
    let forward = RelabelingFunctor::new(char_to_u32);
    let backward = RelabelingFunctor::new(|n: u32| char::from_u32(n).unwrap());

    let original = Cospan::new(vec![0, 0], vec![0, 1], vec!['a', 'b']);
    let there = forward.map_mor(&original).unwrap();
    let back = backward.map_mor(&there).unwrap();

    assert_cospan_eq_msg(&original, &back, "roundtrip preserves structure");
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Frobenius preservation (Eq. 12)
// ---------------------------------------------------------------------------

#[test]
fn ctf_unit_preservation() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let z = 'a';
    let src_unit = Cospan::<char>::unit(z);
    let mapped: FM = f.map_mor(&src_unit).unwrap();
    let tgt_unit: FM = HypergraphCategory::unit(f.map_ob(z));
    assert_eq!(mapped.domain(), tgt_unit.domain(), "F(η) domain");
    assert_eq!(mapped.codomain(), tgt_unit.codomain(), "F(η) codomain");
}

#[test]
fn ctf_counit_preservation() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let z = 'b';
    let src_counit = Cospan::<char>::counit(z);
    let mapped: FM = f.map_mor(&src_counit).unwrap();
    let tgt_counit: FM = HypergraphCategory::counit(f.map_ob(z));
    assert_eq!(mapped.domain(), tgt_counit.domain(), "F(ε) domain");
    assert_eq!(mapped.codomain(), tgt_counit.codomain(), "F(ε) codomain");
}

#[test]
fn ctf_multiplication_preservation() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let z = 'c';
    let src_mul = Cospan::<char>::multiplication(z);
    let mapped: FM = f.map_mor(&src_mul).unwrap();
    let tgt_mul: FM = HypergraphCategory::multiplication(f.map_ob(z));
    assert_eq!(mapped.domain(), tgt_mul.domain(), "F(μ) domain");
    assert_eq!(mapped.codomain(), tgt_mul.codomain(), "F(μ) codomain");
}

#[test]
fn ctf_comultiplication_preservation() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let z = 'd';
    let src_comul = Cospan::<char>::comultiplication(z);
    let mapped: FM = f.map_mor(&src_comul).unwrap();
    let tgt_comul: FM = HypergraphCategory::comultiplication(f.map_ob(z));
    assert_eq!(mapped.domain(), tgt_comul.domain(), "F(δ) domain");
    assert_eq!(mapped.codomain(), tgt_comul.codomain(), "F(δ) codomain");
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Functoriality
// ---------------------------------------------------------------------------

#[test]
fn ctf_functoriality_composition() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let g = Cospan::<char>::comultiplication('a');
    let h = Cospan::<char>::multiplication('a');

    // F(g ; h)
    let composed = g.compose(&h).unwrap();
    let mapped_composed: FM = f.map_mor(&composed).unwrap();

    // F(g) ; F(h)
    let mut mapped_g: FM = f.map_mor(&g).unwrap();
    let mapped_h: FM = f.map_mor(&h).unwrap();
    ComposableMutating::compose(&mut mapped_g, mapped_h).unwrap();

    assert_eq!(
        mapped_composed.domain(),
        mapped_g.domain(),
        "F(g;h) vs F(g);F(h) domain"
    );
    assert_eq!(
        mapped_composed.codomain(),
        mapped_g.codomain(),
        "F(g;h) vs F(g);F(h) codomain"
    );
}

#[test]
fn ctf_functoriality_identity() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let types = vec!['a', 'b', 'c'];
    let src_id = Cospan::<char>::identity(&types);
    let mapped: FM = f.map_mor(&src_id).unwrap();
    let tgt_id: FM = HasIdentity::identity(&types);
    assert_eq!(mapped.domain(), tgt_id.domain(), "F(id) domain");
    assert_eq!(mapped.codomain(), tgt_id.codomain(), "F(id) codomain");
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Monoidal preservation
// ---------------------------------------------------------------------------

#[test]
fn ctf_monoidal_preservation() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let g = Cospan::<char>::unit('a');
    let h = Cospan::<char>::counit('b');

    // F(g ⊗ h)
    let mut tensor = g.clone();
    tensor.monoidal(h.clone());
    let mapped_tensor: FM = f.map_mor(&tensor).unwrap();

    // F(g) ⊗ F(h)
    let mut mapped_parts: FM = f.map_mor(&g).unwrap();
    mapped_parts.monoidal(f.map_mor(&h).unwrap());

    assert_eq!(
        mapped_tensor.domain(),
        mapped_parts.domain(),
        "F(g⊗h) vs F(g)⊗F(h) domain"
    );
    assert_eq!(
        mapped_tensor.codomain(),
        mapped_parts.codomain(),
        "F(g⊗h) vs F(g)⊗F(h) codomain"
    );
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Edge cases
// ---------------------------------------------------------------------------

#[test]
fn ctf_empty_cospan() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let empty = Cospan::<char>::empty();
    let mapped: FM = f.map_mor(&empty).unwrap();
    assert!(mapped.domain().is_empty());
    assert!(mapped.codomain().is_empty());
}

#[test]
fn ctf_multi_type_cospan() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let id = Cospan::<char>::identity(&vec!['a', 'b']);
    let mapped: FM = f.map_mor(&id).unwrap();
    assert_eq!(mapped.domain(), vec!['a', 'b']);
    assert_eq!(mapped.codomain(), vec!['a', 'b']);
}

#[test]
fn ctf_asymmetric_cospan() {
    let split3 = Cospan::new(vec![0], vec![0, 0, 0], vec!['a']);
    let f = CospanToFrobeniusFunctor::<String>::new();
    let mapped: FM = f.map_mor(&split3).unwrap();
    assert_eq!(mapped.domain(), vec!['a']);
    assert_eq!(mapped.codomain(), vec!['a', 'a', 'a']);
}
