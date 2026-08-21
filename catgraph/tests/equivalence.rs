//! Integration tests for Fong-Spivak §4 equivalence (Theorem 1.2/4.13/4.16).
//!
//! Tests the `CospanAlgebraMorphism` construction (§4.2) and
//! roundtrip verification (§4.3, Theorem 4.13).

use std::sync::Arc;

mod common;

use common::assert_cospan_eq_msg;

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    cospan::Cospan,
    cospan_algebra::{CospanAlgebra, NameAlgebra, PartitionAlgebra, cospan_to_frobenius},
    equivalence::{CospanAlgebraMorphism, comp_cospan, functor_from_algebra_morphism},
    frobenius::FrobeniusMorphism,
    hypergraph_category::HypergraphCategory,
    monoidal::Monoidal,
};

type PartMorph = CospanAlgebraMorphism<PartitionAlgebra, char>;

fn alg() -> Arc<PartitionAlgebra> {
    Arc::new(PartitionAlgebra)
}

// ---------------------------------------------------------------------------
// comp_cospan correctness
// ---------------------------------------------------------------------------

#[test]
fn comp_cospan_merges_y_copies() {
    let comp = comp_cospan(&['a'], &['b'], &['c']);
    let left = comp.left_to_middle();
    // X: [0], first Y: [1], second Y: [1] (merged!), Z: [2]
    assert_eq!(left, &[0, 1, 1, 2]);
    let right = comp.right_to_middle();
    // X: [0], Z: [2]
    assert_eq!(right, &[0, 2]);
}

// ---------------------------------------------------------------------------
// Identity laws
// ---------------------------------------------------------------------------

#[test]
fn identity_compose_left() {
    let a = alg();
    let id = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let f = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let result = id.compose(&f).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
}

#[test]
fn identity_compose_right() {
    let a = alg();
    let f = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let id = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let result = f.compose(&id).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
}

#[test]
fn identity_multi_type() {
    let a = alg();
    let id = PartMorph::identity_in(Arc::clone(&a), &['a', 'b', 'c']);
    let id2 = PartMorph::identity_in(Arc::clone(&a), &['a', 'b', 'c']);
    let result = id.compose(&id2).unwrap();
    assert_eq!(result.domain(), vec!['a', 'b', 'c']);
    assert_eq!(result.codomain(), vec!['a', 'b', 'c']);
}

// ---------------------------------------------------------------------------
// Composition associativity
// ---------------------------------------------------------------------------

#[test]
fn composition_associativity() {
    let a = alg();
    let f = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let g = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let h = PartMorph::identity_in(Arc::clone(&a), &['a']);
    // (f;g);h
    let fg = f.compose(&g).unwrap();
    let fgh_left = fg.compose(&h).unwrap();
    // f;(g;h)
    let gh = g.compose(&h).unwrap();
    let fgh_right = f.compose(&gh).unwrap();
    assert_eq!(fgh_left.domain(), fgh_right.domain());
    assert_eq!(fgh_left.codomain(), fgh_right.codomain());
}

// ---------------------------------------------------------------------------
// Monoidal product
// ---------------------------------------------------------------------------

#[test]
fn monoidal_product_domains() {
    let a = alg();
    let mut f = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let g = PartMorph::identity_in(Arc::clone(&a), &['b']);
    f.monoidal(g);
    assert_eq!(f.domain(), vec!['a', 'b']);
    assert_eq!(f.codomain(), vec!['a', 'b']);
}

#[test]
fn monoidal_with_empty() {
    let a = alg();
    let mut f = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let g = PartMorph::identity_in(Arc::clone(&a), &[]);
    f.monoidal(g);
    assert_eq!(f.domain(), vec!['a']);
    assert_eq!(f.codomain(), vec!['a']);
}

// ---------------------------------------------------------------------------
// Frobenius axiom ARITIES in H_Part
//
// ⚠ The three tests below assert domains and codomains only. The equations
// themselves are decided in `tests/frobenius_axioms.rs`, where this carrier
// needs its bubbles quotiented first — `(η ⊗ id) ; μ` and `id` differ by two
// scalar classes, so an equality here would not even have been true.
// ---------------------------------------------------------------------------

#[test]
fn special_frobenius_arities() {
    // δ;μ = id (domain/codomain)
    let a = alg();
    let delta = PartMorph::comultiplication_in(Arc::clone(&a), 'a');
    let mu = PartMorph::multiplication_in(Arc::clone(&a), 'a');
    let result = delta.compose(&mu).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
}

#[test]
fn unitality_left_arities() {
    // (η ⊗ id) ; μ = id
    let a = alg();
    let mut eta_id = PartMorph::unit_in(Arc::clone(&a), 'a');
    eta_id.monoidal(PartMorph::identity_in(Arc::clone(&a), &['a']));
    let mu = PartMorph::multiplication_in(Arc::clone(&a), 'a');
    let result = eta_id.compose(&mu).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
}

#[test]
fn counitality_left_arities() {
    // δ ; (ε ⊗ id) = id
    let a = alg();
    let delta = PartMorph::comultiplication_in(Arc::clone(&a), 'a');
    let mut eps_id = PartMorph::counit_in(Arc::clone(&a), 'a');
    eps_id.monoidal(PartMorph::identity_in(Arc::clone(&a), &['a']));
    let result = delta.compose(&eps_id).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
}

#[test]
fn cup_types() {
    let cup = PartMorph::cup('a').unwrap();
    assert!(cup.domain().is_empty());
    assert_eq!(cup.codomain(), vec!['a', 'a']);
}

#[test]
fn cap_types() {
    let cap = PartMorph::cap('a').unwrap();
    assert_eq!(cap.domain(), vec!['a', 'a']);
    assert!(cap.codomain().is_empty());
}

// ---------------------------------------------------------------------------
// Roundtrip: A → H_A → A_{H_A} = A (Direction 2, Theorem 4.13)
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_a_to_h_to_a_objects() {
    // A_{H_{Part}}(X) = H_{Part}(∅, X) = Part(∅ ⊕ X) = Part(X)
    let a = alg();
    let partition = Cospan::new(vec![], vec![0, 0], vec!['a']).unwrap();

    let morph = PartMorph::new(Arc::clone(&a), partition.clone(), vec![], vec!['a', 'b']);
    assert!(morph.domain().is_empty());
    assert_eq!(morph.codomain(), vec!['a', 'b']);
    assert_eq!(morph.element().codomain(), partition.codomain());
}

#[test]
fn roundtrip_a_to_h_to_a_morphisms() {
    let part = PartitionAlgebra;

    // Element e ∈ Part([a]) = Cospan(∅, [a])
    let e = Cospan::new(vec![], vec![0], vec!['a']).unwrap();
    // Cospan c: [a] → [a] (identity)
    let c = Cospan::<char>::identity(&vec!['a']);
    // Part(c)(e) — direct
    let direct = part.map_cospan(&c, &e).unwrap();

    assert_eq!(direct.codomain(), e.codomain());
}

#[test]
fn roundtrip_partition_identity_via_h() {
    // The identity in H_{Part}([a],[a]) should act as identity when composed
    let a = alg();
    // Create a non-trivial morphism: [a] → [a] via a partition
    let s = Cospan::new(vec![], vec![0, 0], vec!['a']).unwrap();
    let f = PartMorph::new(Arc::clone(&a), s, vec!['a'], vec!['a']);

    let id = PartMorph::identity_in(Arc::clone(&a), &['a']);
    let result = id.compose(&f).unwrap();
    assert_eq!(result.domain(), vec!['a']);
    assert_eq!(result.codomain(), vec!['a']);
}

// ---------------------------------------------------------------------------
// Roundtrip: H → A_H → H_{A_H} ≅ H (Direction 1, Theorem 4.13)
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_cospan_to_partition_objects() {
    // A_{Cospan}(X) = Part(X) = Cospan(∅, X) (Remark 4.5)
    let part = PartitionAlgebra;
    let unit: Cospan<char> = part.unit();
    assert!(unit.domain().is_empty());
    assert!(unit.codomain().is_empty());

    // Map through cospan ∅ → [a] to get element of Part([a])
    let s = Cospan::new(vec![], vec![0], vec!['a']).unwrap();
    let elem = part.map_cospan(&s, &unit).unwrap();
    assert!(elem.domain().is_empty());
    assert_eq!(elem.codomain(), vec!['a']);
}

#[test]
fn roundtrip_frobenius_generators() {
    // Frobenius generators in H_{Part} should compose correctly
    let eta = PartMorph::unit('x');
    let eps = PartMorph::counit('x');
    let mu = PartMorph::multiplication('x');
    let delta = PartMorph::comultiplication('x');

    // η;δ = cup
    let cup = eta.compose(&delta).unwrap();
    assert!(cup.domain().is_empty());
    assert_eq!(cup.codomain(), vec!['x', 'x']);

    // μ;ε = cap
    let cap = mu.compose(&eps).unwrap();
    assert_eq!(cap.domain(), vec!['x', 'x']);
    assert!(cap.codomain().is_empty());
}

// ---------------------------------------------------------------------------
// Lemma 4.9: F_α io functor from a morphism α: A → B of cospan-algebras
// ---------------------------------------------------------------------------

type NameMorph = CospanAlgebraMorphism<NameAlgebra<String>, char>;

fn name_alg() -> Arc<NameAlgebra<String>> {
    Arc::new(NameAlgebra::<String>::new())
}

/// Lemma 4.9 — `F_id: H_A → H_A` for `α = id` is the identity functor.
///
/// Verifies that `functor_from_algebra_morphism` with the identity closure
/// leaves the domain, codomain, and the underlying algebra element of the
/// image morphism unchanged (up to Clone).
#[test]
fn lemma_4_9_identity_functor_on_part_morph() {
    let alg = alg();
    let id_x: PartMorph = PartMorph::identity_in(Arc::clone(&alg), &['a', 'b']);

    let id_alpha = |e: &Cospan<char>| e.clone();
    let image: PartMorph = functor_from_algebra_morphism(&id_alpha, Arc::clone(&alg), &id_x);

    assert_eq!(image.domain(), vec!['a', 'b']);
    assert_eq!(image.codomain(), vec!['a', 'b']);
    // The underlying algebra element is preserved as a cospan: domain, codomain,
    // middle all match the original identity's structural image.
    assert_eq!(image.element().domain(), id_x.element().domain());
    assert_eq!(image.element().codomain(), id_x.element().codomain());
    assert_eq!(image.element().middle(), id_x.element().middle());
}

/// Lemma 4.9 — `F_id` preserves composition: `F_id(f ; g) = F_id(f) ; F_id(g)`.
#[test]
fn lemma_4_9_identity_functor_preserves_composition() {
    let alg = alg();
    let f: PartMorph = PartMorph::identity_in(Arc::clone(&alg), &['a']);
    let g: PartMorph = PartMorph::identity_in(Arc::clone(&alg), &['a']);

    let id_alpha = |e: &Cospan<char>| e.clone();

    // F_α(f ; g)
    let fg = f.compose(&g).unwrap();
    let lhs: PartMorph = functor_from_algebra_morphism(&id_alpha, Arc::clone(&alg), &fg);

    // F_α(f) ; F_α(g)
    let ff = functor_from_algebra_morphism(&id_alpha, Arc::clone(&alg), &f);
    let fg_direct = functor_from_algebra_morphism(&id_alpha, Arc::clone(&alg), &g);
    let rhs = ff.compose(&fg_direct).unwrap();

    assert_eq!(lhs.domain(), rhs.domain());
    assert_eq!(lhs.codomain(), rhs.codomain());
    assert_eq!(lhs.element().middle(), rhs.element().middle());
}

/// Lemma 4.9 — functoriality of the construction: `F_{α;β} = F_α ; F_β`.
///
/// Using `α = id` and `β = id` (both on `PartitionAlgebra` → `PartitionAlgebra`)
/// gives `F_{id;id}(f) = F_id(F_id(f)) = f`.
///
/// The witness is deliberately **not** an identity morphism: `μ ; δ` on `'a'`.
/// The comparison is **leg by leg** on the underlying element — an
/// identity witness compared on `middle()` alone left a wrong leg invisible
/// (#285).
///
/// Scope of the claim, stated honestly: `Part` is the **initial**
/// cospan-algebra, so its only self-morphism is the identity — `α` and `β`
/// cannot be anything else here, and the two routes are therefore the same
/// expression. What this pins is `functor_from_algebra_morphism` transporting
/// the element and both boundaries faithfully, **not** functoriality of a
/// non-trivial `α`. The non-trivial `α` (`cospan_to_frobenius`, `Part → Name`)
/// is covered by `lemma_4_9_cospan_to_name_preserves_composition`.
#[test]
fn lemma_4_9_functor_composition_of_morphisms() {
    let alg = alg();
    let mu: PartMorph = PartMorph::multiplication_in(Arc::clone(&alg), 'a');
    let delta: PartMorph = PartMorph::comultiplication_in(Arc::clone(&alg), 'a');
    let f: PartMorph = mu.compose(&delta).unwrap();
    assert_eq!(f.domain(), vec!['a', 'a'], "witness μ;δ domain");
    assert_eq!(f.codomain(), vec!['a', 'a'], "witness μ;δ codomain");

    let alpha = |e: &Cospan<char>| e.clone();
    let beta = |e: &Cospan<char>| e.clone();
    // Composed α;β
    let alpha_then_beta = |e: &Cospan<char>| beta(&alpha(e));

    let combined: PartMorph = functor_from_algebra_morphism(&alpha_then_beta, Arc::clone(&alg), &f);
    let two_step_first: PartMorph = functor_from_algebra_morphism(&alpha, Arc::clone(&alg), &f);
    let two_step: PartMorph =
        functor_from_algebra_morphism(&beta, Arc::clone(&alg), &two_step_first);

    assert_eq!(combined.domain(), two_step.domain());
    assert_eq!(combined.codomain(), two_step.codomain());
    assert_cospan_eq_msg(
        combined.element(),
        two_step.element(),
        "F_{α;β}(μ;δ) vs F_α(F_β(μ;δ))",
    );
    // …and the round trip really is the input element, not just self-consistent.
    assert_cospan_eq_msg(
        combined.element(),
        f.element(),
        "F_{id;id}(μ;δ) reproduces μ;δ",
    );
}

/// Lemma 4.9 — non-trivial α: `PartitionAlgebra` → `NameAlgebra` via
/// `cospan_to_frobenius`. This is the natural transformation whose existence
/// follows from `CospanToFrobeniusFunctor` being a hypergraph functor.
///
/// Verifies `F_α(id_x)` sits in `H_{NameAlgebra}` with the expected domain/codomain.
#[test]
fn lemma_4_9_cospan_to_name_functor_preserves_identity() {
    let part = alg();
    let name = name_alg();

    let id_x: PartMorph = PartMorph::identity_in(Arc::clone(&part), &['a']);
    let alpha = |c: &Cospan<char>| -> FrobeniusMorphism<char, String> {
        cospan_to_frobenius(c).expect("cospan_to_frobenius is total on well-formed cospans")
    };
    let image: NameMorph = functor_from_algebra_morphism(&alpha, Arc::clone(&name), &id_x);

    assert_eq!(image.domain(), vec!['a']);
    assert_eq!(image.codomain(), vec!['a']);
    // The underlying element is a FrobeniusMorphism, not a Cospan — its
    // own `domain()` / `codomain()` are the concatenation X⊕Y of the H_A
    // morphism, which for id_X on ['a'] is [a, a] (cup cospan realised as
    // a Frobenius morphism).
    let elem = image.element();
    assert!(elem.domain().is_empty());
    assert_eq!(elem.codomain(), vec!['a', 'a']);
}

/// Lemma 4.9 — the same non-trivial α on a **non-identity** morphism: `F_α(μ)`.
///
/// `lemma_4_9_cospan_to_name_functor_preserves_identity` above only exercises
/// `id_['a']`, whose image is symmetric in every component — a functor that
/// dropped or swapped a leg would land on the same answer. `μ: [a,a] → [a]` is
/// asymmetric, so the boundaries below pin the direction as well as the size.
///
/// Scope of the claim: this one morphism, compared on the morphism boundary,
/// the name element's boundary, and the element's `depth()`. It is not a
/// content comparison — see `lemma_4_9_cospan_to_name_preserves_composition`
/// for why presentation equality is out of reach on this side.
#[test]
fn lemma_4_9_cospan_to_name_on_a_non_identity_morphism() {
    let part = alg();
    let name = name_alg();

    let mu: PartMorph = PartMorph::multiplication_in(Arc::clone(&part), 'a');
    let alpha = |c: &Cospan<char>| -> FrobeniusMorphism<char, String> {
        cospan_to_frobenius(c).expect("cospan_to_frobenius is total")
    };
    let image: NameMorph = functor_from_algebra_morphism(&alpha, Arc::clone(&name), &mu);

    assert_eq!(image.domain(), vec!['a', 'a'], "F_α(μ) domain");
    assert_eq!(image.codomain(), vec!['a'], "F_α(μ) codomain");

    // The name of μ lives in H(I, P(X) ⊕ Y): a morphism [] → [a,a,a].
    let elem = image.element();
    assert!(elem.domain().is_empty(), "a name has empty domain");
    assert_eq!(
        elem.codomain(),
        vec!['a', 'a', 'a'],
        "the name of a 2→1 morphism has the X⊕Y interface [a,a,a]"
    );
    assert_eq!(
        elem.depth(),
        3,
        "the epi-mono realisation of μ's name is three layers; the identity \
         witness this test used to rely on is a single layer, so depth alone \
         could not tell the two apart"
    );
}

/// Lemma 4.9 — `F_α` preserves composition for the non-trivial α above:
/// `F_α(f ; g)` agrees with `F_α(f) ; F_α(g)` on domain/codomain.
///
/// The witnesses are **non-identity** (`μ` and `δ` on `'a'`), and the
/// assertions cover the name element's own boundary as well as the morphism's
/// — the identity witnesses this test used to carry made `F_α` indistinguishable
/// from a constant (#285).
///
/// Scope of the claim: this one composable pair, compared on four boundaries
/// (`domain`/`codomain` of the morphism and of its `FrobeniusMorphism`
/// element). It is **not** a content comparison: presentation equality does
/// not hold here — `F_α(μ;δ).element()` is a depth-3 layering and
/// `(F_α(μ);F_α(δ)).element()` a depth-5 one, equal only modulo the Frobenius
/// axioms, which `FrobeniusMorphism: PartialEq` does not quotient by.
#[test]
fn lemma_4_9_cospan_to_name_preserves_composition() {
    let part = alg();
    let name = name_alg();

    let f: PartMorph = PartMorph::multiplication_in(Arc::clone(&part), 'a');
    let g: PartMorph = PartMorph::comultiplication_in(Arc::clone(&part), 'a');

    let alpha = |c: &Cospan<char>| -> FrobeniusMorphism<char, String> {
        cospan_to_frobenius(c).expect("cospan_to_frobenius is total")
    };

    let fg = f.compose(&g).unwrap();
    let lhs: NameMorph = functor_from_algebra_morphism(&alpha, Arc::clone(&name), &fg);

    let ff: NameMorph = functor_from_algebra_morphism(&alpha, Arc::clone(&name), &f);
    let gg: NameMorph = functor_from_algebra_morphism(&alpha, Arc::clone(&name), &g);
    let rhs = ff.compose(&gg).unwrap();

    assert_eq!(lhs.domain(), rhs.domain(), "F_α(μ;δ) domain");
    assert_eq!(lhs.codomain(), rhs.codomain(), "F_α(μ;δ) codomain");
    assert_eq!(lhs.domain(), vec!['a', 'a'], "μ;δ is 2→2, not an identity");
    assert_eq!(
        lhs.element().domain(),
        rhs.element().domain(),
        "F_α(μ;δ) name-element domain"
    );
    assert_eq!(
        lhs.element().codomain(),
        rhs.element().codomain(),
        "F_α(μ;δ) name-element codomain"
    );
    assert_eq!(
        lhs.element().codomain(),
        vec!['a', 'a', 'a', 'a'],
        "the name of a 2→2 morphism has the X⊕Y interface [a,a,a,a]"
    );
}
