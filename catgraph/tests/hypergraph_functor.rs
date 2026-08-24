//! Integration tests for `HypergraphFunctor` trait (Fong-Spivak §2.3).
//!
//! Verifies Frobenius preservation (Eq. 12), functoriality, identity,
//! monoidal preservation, and derived cup/cap for `RelabelingFunctor`.

mod common;

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    cospan::Cospan,
    cospan_algebra::frobenius_to_cospan,
    frobenius::{FrobeniusMorphism, special_frobenius_morphism},
    hypergraph_category::HypergraphCategory,
    hypergraph_functor::{CospanToFrobeniusFunctor, HypergraphFunctor, RelabelingFunctor},
    monoidal::Monoidal,
};
use common::{assert_cospan_eq_msg, assert_frobenius_eq_msg, frobenius_shape};

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

    let original = Cospan::new(vec![0, 0], vec![0, 1], vec!['a', 'b']).unwrap();
    let there = forward.map_mor(&original).unwrap();
    let back = backward.map_mor(&there).unwrap();

    assert_cospan_eq_msg(&original, &back, "roundtrip preserves structure");
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Frobenius preservation (Eq. 12)
//
// These four compare the *whole morphism* (`FrobeniusMorphism: PartialEq` is
// layer-by-layer presentation equality), not just the boundary types — see
// `assert_frobenius_eq_msg`. Boundary-only versions could not see
// connectivity: under a merge-everything-into-one-spider implementation of the
// functor (#285) all four stayed green (each uses one label); the three
// mixed-label tests further down that the mutant reddened
// (`ctf_functoriality_identity`, `ctf_monoidal_preservation`,
// `ctf_multi_type_cospan`) went red for their labels, not their wiring.
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
    assert_frobenius_eq_msg(&mapped, &tgt_unit, "F(η_x) = η_{F(x)}");
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
    assert_frobenius_eq_msg(&mapped, &tgt_counit, "F(ε_x) = ε_{F(x)}");
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
    assert_frobenius_eq_msg(&mapped, &tgt_mul, "F(μ_x) = μ_{F(x)}");
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
    assert_frobenius_eq_msg(&mapped, &tgt_comul, "F(δ_x) = δ_{F(x)}");
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Functoriality
// ---------------------------------------------------------------------------

/// `F(g;h)` and `F(g);F(h)` agree at the boundary for the `δ ; μ` witness.
///
/// Deliberately boundary-only: `δ ; μ` is the one pair where the two routes
/// land on *different presentations* of the same morphism. `F(g;h)` is the
/// identity (depth 1, because `g;h` is the identity cospan), while `F(g);F(h)`
/// is the two-layer `δ ; μ` — equal only modulo the specialness axiom, which
/// `FrobeniusMorphism: PartialEq` does not quotient by. Content-level
/// functoriality is pinned on other witnesses by
/// `ctf_functoriality_composition_content`.
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

/// `F(id_x) = id_{F(x)}` — as whole morphisms, not just at the boundary.
///
/// Scope of the claim: the three concrete objects listed in `cases`. The
/// assertions compare full presentation equality plus `depth()`; they say
/// nothing about objects with more than three wires, and nothing about
/// non-identity cospans (those are pinned by
/// `ctf_single_apex_cospan_is_the_spider` and
/// `ctf_disconnected_cospan_is_the_tensor_not_a_spider`).
///
/// Regression pinned (#285): under an implementation mapping every `m → n`
/// cospan to `special_frobenius_morphism(m, n, z)`, and with this test only
/// comparing domain/codomain, its `['a','b','c']` case went red on its labels
/// alone, while the `['a','a']` case (added here) would have stayed green —
/// `F(id_['a','a'])` came out as the 2→2 spider (depth 2) with the identity's
/// boundary, while the identity has depth 1. Presentation equality sees both.
#[test]
fn ctf_functoriality_identity() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    let cases: Vec<Vec<char>> = vec![vec!['a'], vec!['a', 'a'], vec!['a', 'b', 'c']];
    for types in cases {
        let src_id = Cospan::<char>::identity(&types);
        let mapped: FM = f.map_mor(&src_id).unwrap();
        let tgt_id: FM = HasIdentity::identity(&types);
        assert_eq!(mapped.domain(), tgt_id.domain(), "F(id) domain");
        assert_eq!(mapped.codomain(), tgt_id.codomain(), "F(id) codomain");
        assert!(
            mapped == tgt_id,
            "F(id_{types:?}) must be the identity morphism, not a spider: \
             got {}, want {}",
            frobenius_shape(&mapped),
            frobenius_shape(&tgt_id),
        );
        assert_eq!(
            mapped.depth(),
            1,
            "F(id_{types:?}) is a single layer; the 2→2 spider that the \
             merge-everything implementation produced has depth 2"
        );
    }
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: Monoidal preservation
// ---------------------------------------------------------------------------

/// `F(g⊗h)` and `F(g)⊗F(h)` agree at the boundary for the `η ⊗ ε` witness.
///
/// Deliberately boundary-only, for the same reason as
/// `ctf_functoriality_composition`: for `η_a ⊗ ε_b` the epi-mono route through
/// the tensored cospan produces a two-layer presentation while the tensor of
/// the two images is one layer. Content-level monoidality is pinned on a
/// different witness by `ctf_disconnected_cospan_is_the_tensor_not_a_spider`.
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
    let types = vec!['a', 'b'];
    let id = Cospan::<char>::identity(&types);
    let mapped: FM = f.map_mor(&id).unwrap();
    assert_eq!(mapped.domain(), vec!['a', 'b']);
    assert_eq!(mapped.codomain(), vec!['a', 'b']);
    let tgt: FM = HasIdentity::identity(&types);
    assert_frobenius_eq_msg(&mapped, &tgt, "F(id_[a,b])");
}

#[test]
fn ctf_asymmetric_cospan() {
    let split3 = Cospan::new(vec![0], vec![0, 0, 0], vec!['a']).unwrap();
    let f = CospanToFrobeniusFunctor::<String>::new();
    let mapped: FM = f.map_mor(&split3).unwrap();
    assert_eq!(mapped.domain(), vec!['a']);
    assert_eq!(mapped.codomain(), vec!['a', 'a', 'a']);
    let spider: FM = special_frobenius_morphism(1, 3, 'a');
    assert_frobenius_eq_msg(&mapped, &spider, "F(1→3 single-apex cospan) = spider(1,3)");
}

// ---------------------------------------------------------------------------
// CospanToFrobeniusFunctor: content-level pins (#285)
//
// Everything above this line that compares only `domain()` / `codomain()`
// cannot see connectivity: under an implementation that mapped *every* `m → n`
// cospan to `special_frobenius_morphism(m, n, z)` (label read off the cospan),
// 7 of the 10 `ctf_*` tests stayed green and the other three went red for
// their mixed boundary labels, not their wiring. The tests below compare whole
// morphisms, and each states the space its claim ranges over.
// ---------------------------------------------------------------------------

/// Every single-apex ("all-merged") cospan `m → {•} ← n` maps to the `(m,n)`
/// spider, by content.
///
/// Scope of the claim: exactly the 25 cospans `(m, n) ∈ {0,…,4}²` over the
/// single label `'a'` with a one-element apex. It ranges over no multi-apex
/// and no multi-label cospan. The reference side is
/// `special_frobenius_morphism`, which is **not** independent of the code
/// under test: `from_decomposition` builds each surjection block by calling
/// it, and outside the base cases (`(1,0)` is `Counit` itself) it is, for
/// `m ≥ n`, `n ≠ 1`, literally `sfm(m,1) ; sfm(1,n)` — the same shape the
/// general route composes. A defect inside
/// `special_frobenius_morphism` is therefore invisible here; what this pin
/// does see is the guard regression below, and
/// `ctf_single_apex_cospan_round_trips_up_to_canonical_form` is the
/// `sfm`-independent companion.
///
/// Regression pinned (#285): the identity fast path in `cospan_to_frobenius`
/// fired on `domain == codomain && left == right`, which the all-merged
/// `m → {•} ← m` cospan also satisfies (both legs are `[0; m]`). Before the
/// fix `(2,2)`, `(3,3)` and `(4,4)` all returned the identity (depth 1)
/// instead of the spider (depth 2, 4, 4).
#[test]
fn ctf_single_apex_cospan_is_the_spider() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    for m in 0usize..=4 {
        for n in 0usize..=4 {
            let c = Cospan::new(vec![0; m], vec![0; n], vec!['a'])
                .expect("single-apex cospan is well formed for every (m, n)");
            let mapped: FM = f.map_mor(&c).unwrap();
            let spider: FM = special_frobenius_morphism(m, n, 'a');
            assert!(
                mapped == spider,
                "F(single-apex {m}→{n}) must be spider({m},{n}): got {}, want {}",
                frobenius_shape(&mapped),
                frobenius_shape(&spider),
            );
        }
    }
}

/// `sfm`-independent companion to the grid pin: interpret `F(c)` back into
/// `Cospan` through `frobenius_to_cospan` and compare canonical forms with the
/// original. `frobenius_to_cospan`'s only `special_frobenius_morphism` call
/// sits in the general `Spider` arm of its generator interpreter (the `(0,0)`
/// arm builds the bubble without it), which `cospan_to_frobenius`'s image
/// never reaches — no production path introduces a `Spider` block; rule 4
/// (fusion) and `hflip` only rewrite ones already present — so on this input
/// space the reference side shares no code with `special_frobenius_morphism`.
///
/// Scope of the claim: all **25** single-apex cospans `(m, n) ∈ {0,…,4}²` over
/// `'a'`. `(0,0)` — the bubble — was excluded until #350 and asserted
/// separately at `apex 0`, because `two_layer_simplify`'s rule 3 cancelled the
/// `η;ε` on the way back; with that rule deleted the bubble survives and the
/// grid is uniform. Restoring rule 3 turns the `(0,0)` iteration red (back apex
/// 0 against the original's 1).
///
/// Falsified: under the pre-#285 guard the round trip disagreed at `(2,2)`,
/// `(3,3)` and `(4,4)` (back apex 2, 3, 4 against the original's 1).
#[test]
fn ctf_single_apex_cospan_round_trips_up_to_canonical_form() {
    let f = CospanToFrobeniusFunctor::<String>::new();
    for m in 0usize..=4 {
        for n in 0usize..=4 {
            let c = Cospan::new(vec![0; m], vec![0; n], vec!['a'])
                .expect("single-apex cospan is well formed for every (m, n)");
            let mapped: FM = f.map_mor(&c).unwrap();
            let back = frobenius_to_cospan(&mapped).unwrap().canonical_form();
            let original = c.canonical_form();
            assert_eq!(
                back,
                original,
                "F(single-apex {m}→{n}) does not round-trip: back apex {}, original apex {}",
                back.apex_len(),
                original.apex_len()
            );
        }
    }
}

/// A *disconnected* cospan maps to the tensor of its components' images, not
/// to one spider that merges everything.
///
/// Scope of the claim: three concrete two-component witnesses —
/// `μ_a ⊗ id_a` (`[a,a,a] → [a,a]`, **uniform label**, two apex nodes),
/// `μ_a ⊗ id_b` (`[a,a,b] → [a,b]`) and `μ_a ⊗ μ_b` (`[a,a,b,b] → [a,b]`).
/// Each is compared as a whole morphism against the tensor of the component
/// images, and against the single spider a merge-everything implementation
/// would produce. It claims nothing about components with empty boundaries
/// (scalars), nor about three or more components.
///
/// The uniform-label witness is the one that carries the claim: for the
/// mixed-label witnesses a wrong implementation is already visible in the
/// boundary *labels*, so they cannot tell a connectivity bug from a labelling
/// bug. `μ_a ⊗ id_a` and `spider(3,2,'a')` have byte-identical domains and
/// codomains, so only the connectivity separates them.
///
/// Regression pinned (#285): mapping every `m → n` cospan to
/// `special_frobenius_morphism(m, n, z)` is wrong for exactly this shape, and
/// the boundary-only tests could not see it — `spider(3,2,'a')` has depth 3
/// against the correct depth 1.
#[test]
fn ctf_disconnected_cospan_is_the_tensor_not_a_spider() {
    let f = CospanToFrobeniusFunctor::<String>::new();

    /// `(name, g, h, m, n)`: the two components and the arity of `g ⊗ h`.
    type TensorWitness = (&'static str, Cospan<char>, Cospan<char>, usize, usize);

    let witnesses: Vec<TensorWitness> = vec![
        (
            "μ_a ⊗ id_a",
            Cospan::<char>::multiplication('a'),
            Cospan::<char>::identity(&vec!['a']),
            3,
            2,
        ),
        (
            "μ_a ⊗ id_b",
            Cospan::<char>::multiplication('a'),
            Cospan::<char>::identity(&vec!['b']),
            3,
            2,
        ),
        (
            "μ_a ⊗ μ_b",
            Cospan::<char>::multiplication('a'),
            Cospan::<char>::multiplication('b'),
            4,
            2,
        ),
    ];

    for (name, g, h, m, n) in witnesses {
        let mut tensor = g.clone();
        tensor.monoidal(h.clone());
        assert_eq!(
            tensor.middle().len(),
            2,
            "{name}: the witness must be disconnected (two apex nodes)"
        );

        let mapped_tensor: FM = f.map_mor(&tensor).unwrap();
        let mut mapped_parts: FM = f.map_mor(&g).unwrap();
        mapped_parts.monoidal(f.map_mor(&h).unwrap());

        assert!(
            mapped_tensor == mapped_parts,
            "{name}: F(g⊗h) must equal F(g)⊗F(h): got {}, want {}",
            frobenius_shape(&mapped_tensor),
            frobenius_shape(&mapped_parts),
        );

        let merge_all: FM = special_frobenius_morphism(m, n, 'a');
        assert!(
            mapped_tensor != merge_all,
            "{name}: F(g⊗h) must NOT be the connected spider({m},{n}) \
             (both are {m}→{n}, so the arities alone cannot tell them apart): \
             got {}, spider is {}",
            frobenius_shape(&mapped_tensor),
            frobenius_shape(&merge_all),
        );
    }
}

/// `F(g ; h) = F(g) ; F(h)`, by content.
///
/// Scope of the claim: two concrete composable pairs — `μ_a ; δ_a` (a merge
/// followed by a split, both connected) and
/// `(η_a ⊗ id_b) ; (id_a ⊗ ε_b)` (disconnected on both sides). It does **not**
/// range over all composable pairs: `FrobeniusMorphism: PartialEq` is
/// presentation equality, not equality modulo the Frobenius axioms, so pairs
/// whose two routes normalize differently are out of reach — `δ_a ; μ_a` is
/// exactly such a pair (`F(g;h)` is the depth-1 identity, `F(g);F(h)` is the
/// depth-2 `δ ; μ`), and is covered at the boundary by
/// `ctf_functoriality_composition` instead.
///
/// Regression pinned (#285): the `μ_a ; δ_a` witness composes to the all-merged
/// `[a,a] → {•} ← [a,a]` cospan, which the old identity fast path mapped to
/// the identity (depth 1) while `F(μ) ; F(δ)` is depth 2.
#[test]
fn ctf_functoriality_composition_content() {
    let f = CospanToFrobeniusFunctor::<String>::new();

    let mut disconnected_g = Cospan::<char>::unit('a');
    disconnected_g.monoidal(Cospan::<char>::identity(&vec!['b']));
    let mut disconnected_h = Cospan::<char>::identity(&vec!['a']);
    disconnected_h.monoidal(Cospan::<char>::counit('b'));

    let witnesses: Vec<(&str, Cospan<char>, Cospan<char>)> = vec![
        (
            "μ_a ; δ_a",
            Cospan::<char>::multiplication('a'),
            Cospan::<char>::comultiplication('a'),
        ),
        (
            "(η_a ⊗ id_b) ; (id_a ⊗ ε_b)",
            disconnected_g,
            disconnected_h,
        ),
    ];

    for (name, g, h) in witnesses {
        let composed = g.compose(&h).unwrap();
        let mapped_composed: FM = f.map_mor(&composed).unwrap();

        let mut mapped_g: FM = f.map_mor(&g).unwrap();
        ComposableMutating::compose(&mut mapped_g, f.map_mor(&h).unwrap()).unwrap();

        assert!(
            mapped_composed == mapped_g,
            "{name}: F(g;h) must equal F(g);F(h): got {}, want {}",
            frobenius_shape(&mapped_composed),
            frobenius_shape(&mapped_g),
        );
    }
}
