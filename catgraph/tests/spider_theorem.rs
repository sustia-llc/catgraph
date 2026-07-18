//! Explicit verification of Fong-Spivak Thm 6.55 (spider theorem):
//! any connected Frobenius diagram on `m` inputs and `n` outputs equals
//! the spider `s_{m,n}`.
//!
//! These tests build connected diagrams from the generators (η, ε, μ, δ)
//! and assert **full structural equality** (`FrobeniusMorphism` derives `Eq`)
//! against the canonical spider `s_{m,n}` produced by
//! `special_frobenius_morphism(m, n, z)`, plus domain/codomain agreement.
//! They close the ⚠️ PARTIAL status for Thm 6.55 in
//! `catgraph-applied/docs/FS18-AUDIT.md`.

use catgraph::{
    category::{ComposableMutating, HasIdentity},
    frobenius::{FrobeniusMorphism, special_frobenius_morphism},
    hypergraph_category::HypergraphCategory,
    monoidal::Monoidal,
};

/// Morphisms with char-labelled wires and String black-box labels.
type FM = FrobeniusMorphism<char, String>;

/// s_{2,2} constructed as μ;δ. The resulting diagram is connected
/// (single internal wire from μ to δ) with domain `[z,z]` and
/// codomain `[z,z]`. The spider theorem states this must equal s_{2,2}.
#[test]
fn spider_2_2_via_mu_delta() {
    let z = 'z';

    // Build μ;δ : [z,z] -> [z,z] via in-place composition.
    let mut mu_then_delta = FM::multiplication(z);
    let delta = FM::comultiplication(z);
    ComposableMutating::compose(&mut mu_then_delta, delta).unwrap();

    assert_eq!(mu_then_delta.domain(), vec![z, z]);
    assert_eq!(mu_then_delta.codomain(), vec![z, z]);

    // Canonical spider s_{2,2}.
    let spider: FM = special_frobenius_morphism(2, 2, z);
    assert_eq!(spider.domain(), mu_then_delta.domain());
    assert_eq!(spider.codomain(), mu_then_delta.codomain());
    assert!(spider == mu_then_delta, "s_2_2: composite != spider");
}

/// s_{3,1} constructed as (μ ⊗ id) ; μ : [z,z,z] -> [z].
/// Connected trinary fold; must equal the spider s_{3,1}.
#[test]
fn spider_3_1_via_double_mu() {
    let z = 'z';

    // (μ ⊗ id) ; μ — the outer μ merges the codomain of (μ ⊗ id), which is
    // [z, z], into a single wire.
    let mut mu_id = FM::multiplication(z);
    mu_id.monoidal(<FM as HasIdentity<Vec<char>>>::identity(&vec![z]));
    let mu_outer = FM::multiplication(z);
    ComposableMutating::compose(&mut mu_id, mu_outer).unwrap();

    assert_eq!(mu_id.domain(), vec![z, z, z]);
    assert_eq!(mu_id.codomain(), vec![z]);

    // Canonical spider s_{3,1}.
    let spider: FM = special_frobenius_morphism(3, 1, z);
    assert_eq!(spider.domain(), mu_id.domain());
    assert_eq!(spider.codomain(), mu_id.codomain());
    assert!(spider == mu_id, "s_3_1: composite != spider");
}

/// s_{1,3} constructed as (δ ⊗ id) ; δ : [z] -> [z,z,z].
/// Connected trinary split; must equal the spider s_{1,3}.
#[test]
fn spider_1_3_via_double_delta() {
    let z = 'z';

    // Start with δ : [z] -> [z,z], then (δ ⊗ id) gives [z,z] -> [z,z,z].
    // So the composite is δ ; (δ ⊗ id) : [z] -> [z,z,z].
    let mut delta_first = FM::comultiplication(z);
    let mut delta_id = FM::comultiplication(z);
    delta_id.monoidal(<FM as HasIdentity<Vec<char>>>::identity(&vec![z]));
    ComposableMutating::compose(&mut delta_first, delta_id).unwrap();

    assert_eq!(delta_first.domain(), vec![z]);
    assert_eq!(delta_first.codomain(), vec![z, z, z]);

    // Canonical spider s_{1,3}.
    let spider: FM = special_frobenius_morphism(1, 3, z);
    assert_eq!(spider.domain(), delta_first.domain());
    assert_eq!(spider.codomain(), delta_first.codomain());
    assert!(spider == delta_first, "s_1_3: composite != spider");
}

/// s_{0,0}: the η;ε loop. A connected diagram on zero inputs and zero
/// outputs must equal the spider s_{0,0}.
#[test]
fn spider_0_0_via_eta_epsilon() {
    let z = 'z';

    let mut eta = FM::unit(z);
    let eps = FM::counit(z);
    ComposableMutating::compose(&mut eta, eps).unwrap();

    assert!(eta.domain().is_empty());
    assert!(eta.codomain().is_empty());

    // Canonical spider s_{0,0}.
    let spider: FM = special_frobenius_morphism(0, 0, z);
    assert_eq!(spider.domain(), eta.domain());
    assert_eq!(spider.codomain(), eta.codomain());
    assert!(spider == eta, "s_0_0: composite != spider");
}

/// Sanity check on a direct generator: μ alone is a connected diagram
/// [z,z] -> [z] and must share the shape of the canonical spider
/// s_{2,1}. This exercises the shortest possible "reduction" path — a
/// diagram that already *is* a spider must match `special_frobenius_morphism`.
#[test]
fn connected_diagrams_reduce_to_same_spider() {
    let z = 'z';

    let mu: FM = FM::multiplication(z);
    assert_eq!(mu.domain(), vec![z, z]);
    assert_eq!(mu.codomain(), vec![z]);

    let spider: FM = special_frobenius_morphism(2, 1, z);
    assert_eq!(spider.domain(), mu.domain());
    assert_eq!(spider.codomain(), mu.codomain());
    assert!(spider == mu, "s_2_1: generator != spider");
}
