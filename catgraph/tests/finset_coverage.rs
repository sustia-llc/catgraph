//! Integration tests for `catgraph::finset` public API items that lack
//! dedicated coverage elsewhere. Focuses on:
//!
//! - `OrderPresSurj::preimage_cardinalities()`
//! - `OrderPresInj::iden_unit_counts()`
//! - `from_cycle()` — permutation construction from cycle notation
//! - `Decomposition::get_parts()` — accessor for the three-way factorization
//! - Error types: `TryFromSurjError`, `TryFromInjError`, `TryFromFinSetError`

use catgraph::category::{Composable, HasIdentity};
use catgraph::finset::*;
use permutations::Permutation;

// ---------------------------------------------------------------------------
// OrderPresSurj::preimage_cardinalities
// ---------------------------------------------------------------------------

#[test]
fn preimage_cardinalities_identity() {
    // Identity on 4: each element maps to itself, preimage size = 1 each.
    let id = OrderPresSurj::identity(&4);
    assert_eq!(id.preimage_cardinalities(), vec![1, 1, 1, 1]);
}

#[test]
fn preimage_cardinalities_nontrivial() {
    // Surjection [0,1,1,2,3,3,3,4] has preimage sizes [1,2,1,3,1].
    let surj = OrderPresSurj::try_from((vec![0, 1, 1, 2, 3, 3, 3, 4], 0)).unwrap();
    assert_eq!(surj.preimage_cardinalities(), vec![1, 2, 1, 3, 1]);
    // Sum of preimage sizes should equal domain size.
    let total: usize = surj.preimage_cardinalities().iter().sum();
    assert_eq!(total, surj.domain());
}

#[test]
fn preimage_cardinalities_all_collapse() {
    // Everything maps to 0: [0,0,0,0] — one codomain element with preimage size 4.
    let surj = OrderPresSurj::try_from((vec![0, 0, 0, 0], 0)).unwrap();
    assert_eq!(surj.preimage_cardinalities(), vec![4]);
    assert_eq!(surj.domain(), 4);
    assert_eq!(surj.codomain(), 1);
}

#[test]
fn preimage_cardinalities_empty() {
    let surj = OrderPresSurj::default();
    assert_eq!(surj.preimage_cardinalities(), Vec::<usize>::new());
    assert_eq!(surj.domain(), 0);
    assert_eq!(surj.codomain(), 0);
}

// ---------------------------------------------------------------------------
// OrderPresInj::iden_unit_counts
// ---------------------------------------------------------------------------

#[test]
fn iden_unit_counts_identity() {
    // Identity on 5: all elements are consecutive starting at 0.
    let id = OrderPresInj::identity(&5);
    assert_eq!(id.iden_unit_counts(), vec![5]);
    assert_eq!(id.domain(), 5);
    assert_eq!(id.codomain(), 5);
}

#[test]
fn iden_unit_counts_with_gaps() {
    // Injection [0,1,2, gap of 1, 4,5, gap of 2, 8,9,11] with leftovers=23
    // Expected alternating: [3, 1, 2, 2, 2, 1, 1, 23]
    let inj = OrderPresInj::try_from((vec![0, 1, 2, 4, 5, 8, 9, 11], 23)).unwrap();
    assert_eq!(inj.iden_unit_counts(), vec![3, 1, 2, 2, 2, 1, 1, 23]);
    assert_eq!(inj.domain(), 8);
    assert_eq!(inj.codomain(), 12 + 23);
}

#[test]
fn iden_unit_counts_single_shifted() {
    // Injection [2] — element at position 2, nothing before it.
    let inj = OrderPresInj::try_from((vec![2], 0)).unwrap();
    let counts = inj.iden_unit_counts();
    // Should be [0, 2, 1]: 0 identities, gap of 2, then 1 identity.
    assert_eq!(counts, vec![0, 2, 1]);
    assert_eq!(inj.domain(), 1);
    assert_eq!(inj.codomain(), 3);
}

// ---------------------------------------------------------------------------
// from_cycle — permutation from cycle notation
// ---------------------------------------------------------------------------

#[test]
fn from_cycle_transposition() {
    // (0 2) in S_4: swaps 0 and 2, fixes 1 and 3.
    let p = from_cycle(4, &[0, 2]);
    assert_eq!(p.apply(0), 2);
    assert_eq!(p.apply(2), 0);
    assert_eq!(p.apply(1), 1);
    assert_eq!(p.apply(3), 3);
}

#[test]
fn from_cycle_three_cycle() {
    // (0 1 2) in S_4: 0->1, 1->2, 2->0, 3 fixed.
    let p = from_cycle(4, &[0, 1, 2]);
    assert_eq!(p.apply(0), 1);
    assert_eq!(p.apply(1), 2);
    assert_eq!(p.apply(2), 0);
    assert_eq!(p.apply(3), 3);
}

#[test]
fn from_cycle_empty_and_singleton() {
    // Empty cycle and singleton cycle both yield identity.
    let id_empty = from_cycle(3, &[]);
    let id_single = from_cycle(3, &[1]);
    let id = Permutation::identity(3);
    assert_eq!(id_empty, id);
    assert_eq!(id_single, id);
}

#[test]
fn from_cycle_full_rotation() {
    // (0 1 2 3 4) is a full 5-cycle.
    let p = from_cycle(5, &[0, 1, 2, 3, 4]);
    assert_eq!(p.apply(0), 1);
    assert_eq!(p.apply(1), 2);
    assert_eq!(p.apply(2), 3);
    assert_eq!(p.apply(3), 4);
    assert_eq!(p.apply(4), 0);
}

// ---------------------------------------------------------------------------
// Decomposition::get_parts
// ---------------------------------------------------------------------------

#[test]
fn get_parts_identity() {
    let decomp = Decomposition::identity(&3);
    let (perm, surj, inj) = decomp.get_parts();
    assert_eq!(*perm, Permutation::identity(3));
    assert_eq!(surj.preimage_cardinalities(), vec![1, 1, 1]);
    assert_eq!(inj.iden_unit_counts(), vec![3]);
}

#[test]
fn get_parts_nontrivial() {
    // Map [2, 0, 0, 1, 1] with 0 leftovers: sorts to [0,0,1,1,2].
    // Decomposition = permutation ; surjection ; injection.
    let decomp = Decomposition::try_from((vec![2, 0, 0, 1, 1], 0)).unwrap();
    let (perm, surj, inj) = decomp.get_parts();

    // Permutation part has the right size.
    assert_eq!(perm.len(), 5);

    // Surjection's preimage sizes sum to its domain.
    let surj_preimages = surj.preimage_cardinalities();
    assert_eq!(surj_preimages.iter().sum::<usize>(), surj.domain());

    // Injection's iden_unit_counts is non-empty.
    assert!(!inj.iden_unit_counts().is_empty());

    // Overall decomposition domain and codomain.
    assert_eq!(decomp.domain(), 5);
    assert_eq!(decomp.codomain(), 3);

    // Compose with identity preserves domain/codomain.
    let composed = decomp
        .compose(&Decomposition::identity(&decomp.codomain()))
        .unwrap();
    assert_eq!(composed.domain(), 5);
    assert_eq!(composed.codomain(), 3);
}

// ---------------------------------------------------------------------------
// Error paths: TryFromSurjError
// ---------------------------------------------------------------------------

#[test]
fn try_from_surj_error_not_sorted() {
    // [1, 0] is not order-preserving.
    let result = OrderPresSurj::try_from((vec![1, 0], 0));
    assert_eq!(result, Err(TryFromSurjError));
}

#[test]
fn try_from_surj_error_not_surjective() {
    // [0, 2] skips 1 — not surjective.
    let result = OrderPresSurj::try_from((vec![0, 2], 0));
    assert_eq!(result, Err(TryFromSurjError));
}

#[test]
fn try_from_surj_error_nonzero_leftover() {
    // A surjection must have 0 leftovers (nothing unreachable in codomain).
    let result = OrderPresSurj::try_from((vec![0, 1, 2], 1));
    assert_eq!(result, Err(TryFromSurjError));
}

// ---------------------------------------------------------------------------
// Error paths: TryFromInjError
// ---------------------------------------------------------------------------

#[test]
fn try_from_inj_error_not_sorted() {
    // [2, 0] is not order-preserving.
    let result = OrderPresInj::try_from((vec![2, 0], 0));
    assert_eq!(result, Err(TryFromInjError));
}

#[test]
fn try_from_inj_error_not_injective() {
    // [0, 0] has duplicates — not injective.
    let result = OrderPresInj::try_from((vec![0, 0], 0));
    assert_eq!(result, Err(TryFromInjError));
}

// ---------------------------------------------------------------------------
// Error paths: TryFromFinSetError (Decomposition)
// ---------------------------------------------------------------------------

// Note: Decomposition::try_from accepts any function [n] -> [m] via the
// permutation-sort + epi-mono factorization, so the error path is hard to
// trigger with well-formed input. We verify that valid edge cases succeed.

#[test]
fn decomposition_empty_map() {
    // Empty map: domain 0, codomain 0.
    let decomp = Decomposition::try_from((vec![], 0));
    assert!(decomp.is_ok());
    let d = decomp.unwrap();
    assert_eq!(d.domain(), 0);
    assert_eq!(d.codomain(), 0);
}

#[test]
fn decomposition_with_leftovers() {
    // Map [0] with 5 leftovers: domain 1, codomain 6.
    let decomp = Decomposition::try_from((vec![0], 5)).unwrap();
    assert_eq!(decomp.domain(), 1);
    assert_eq!(decomp.codomain(), 6);
    let (_, _, inj) = decomp.get_parts();
    // Injection covers [0] with 5 extra unreachable codomain elements.
    assert_eq!(inj.iden_unit_counts(), vec![1, 5]);
}
