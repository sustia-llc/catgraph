//! Tests for `catgraph_applied::operad_functor` — F&S *Seven Sketches*
//! §6.5 Rough Def 6.98 (maps between operads preserving substitution
//! and identities) plus the canonical E1 → E2 inclusion.

use catgraph::category::HasIdentity;
use catgraph_applied::e1_operad::E1;
use catgraph_applied::e2_operad::E2;
use catgraph_applied::operad_functor::{E1ToE2, OperadFunctor, check_substitution_preserved};

#[test]
fn e1_to_e2_preserves_arity_on_identity() {
    let id = E1::identity(&());
    let mapped: E2<usize> = E1ToE2::default().map_operation(&id).unwrap();
    assert_eq!(mapped.arity_of(), 1);
}

#[test]
fn e1_to_e2_generic_arity_shadow_holds() {
    // Generic functoriality law in arity form, via the trait-level helper.
    let make_outer = || E1::new(vec![(0.0, 0.5), (0.5, 1.0)], true).expect("outer well-formed");
    let make_inner =
        || E1::new(vec![(0.0, 0.3), (0.3, 0.6), (0.6, 1.0)], true).expect("inner well-formed");
    check_substitution_preserved::<E1ToE2, E1, E2<usize>, usize, _, _>(
        &E1ToE2::default(),
        make_outer,
        0,
        make_inner,
    )
    .expect("arity(F(o ∘ q)) == arity(F(o)) + arity(F(q)) − 1");
}

#[test]
fn e1_to_e2_preserves_substitution_geometrically() {
    // Full functoriality: F(outer ∘_0 inner) and F(outer) ∘_0 F(inner)
    // produce the same E₂ disks up to naming. Uses the offset-aware
    // inherent helper so the RHS gets disjoint disk names.
    let make_outer = || E1::new(vec![(0.0, 0.5), (0.5, 1.0)], true).expect("outer well-formed");
    let make_inner =
        || E1::new(vec![(0.0, 0.3), (0.3, 0.6), (0.6, 1.0)], true).expect("inner well-formed");
    E1ToE2::check_substitution_preserved(make_outer, 0, make_inner)
        .expect("F(outer ∘_0 inner) ≡ F(outer) ∘_0 F(inner) geometrically");
}

#[test]
fn e1_to_e2_preserves_substitution_for_different_slot() {
    // Substitute inner into outer slot 1 (right interval) — different
    // rescaling parameters, same functoriality law.
    let make_outer = || E1::new(vec![(0.0, 0.4), (0.4, 1.0)], true).expect("outer well-formed");
    let make_inner = || E1::new(vec![(0.1, 0.5), (0.5, 0.9)], true).expect("inner well-formed");
    E1ToE2::check_substitution_preserved(make_outer, 1, make_inner)
        .expect("functoriality holds on slot 1 too");
}
