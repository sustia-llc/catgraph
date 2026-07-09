//! Acceptance tests for the `R`-module actegory `(FinReal, ⊕, R⁰)` — the first
//! non-`(Set, ×, 1)` [`MonoidalCategory`] / [`Actegory`] instance (issue #36).
//!
//! CDL Definition E.2 (actegory coherence) / Example E.4 (self-action) /
//! Example G.3 (the cartesian `⊕` structure of real vector spaces used by
//! gradient-based-learning `Para(…)` constructions).
//!
//! Four surfaces:
//!
//! 1. **Monoidal coherence** — pentagon / triangle / unitor sanity on the
//!    `DirectSum` tensor, via `common::assert_direct_sum_coherence`.
//! 2. **`R`-module axioms** — `Zero` / `One` load-bearing identities on
//!    [`F64Module`], via `common::assert_f64_module_axioms`.
//! 3. **`⊕`-monoid laws** — concrete coordinate concatenation, unit, and
//!    associativity, via `common::assert_direct_sum_monoid`.
//! 4. **Actegory action + multiplicator `µ`** — `act` / `compose_action` shape
//!    and its agreement with [`F64Monoidal`]'s tensor.

#![allow(clippy::float_cmp)]

mod common;

use common::{
    assert_direct_sum_coherence, assert_direct_sum_monoid, assert_f64_module_axioms, finite_f64,
};

use catgraph_dl::para::{
    Actegory, DirectSum, F64Actegory, F64Module, F64Monoidal, MonoidalCategory,
};

use proptest::prelude::*;

/// **Pentagon + triangle + unitor sanity — deterministic.** [`F64Monoidal`]'s
/// `DirectSum` associator/unitors satisfy the Mac Lane coherence laws on a
/// hand-picked spread of object values (sign extremes, both booleans).
#[test]
fn direct_sum_coherence_deterministic() {
    let m = F64Monoidal::new();
    for (a, b, c, d) in [
        (1_i32, 2_u8, 3_i64, true),
        (-5_i32, 200_u8, -9999_i64, false),
        (i32::MIN, u8::MAX, i64::MAX, true),
        (0_i32, 0_u8, 0_i64, false),
    ] {
        assert_direct_sum_coherence(&m, a, b, c, d);
    }
}

/// **`R`-module axioms — deterministic.** Additive identity, scalar unit / zero,
/// and standard-basis coherence for a spread of dimensions (including the empty
/// module `R⁰`) — the identities where `Zero` / `One` are load-bearing.
#[test]
fn f64_module_axioms_deterministic() {
    assert_f64_module_axioms(vec![], 3.5);
    assert_f64_module_axioms(vec![0.0], -2.0);
    assert_f64_module_axioms(vec![1.0, -1.0, 2.5], 4.0);
    assert_f64_module_axioms(vec![-1e6, 1e6, 0.0, 7.25], 0.0);
}

/// **`⊕`-monoid laws — deterministic.** Dimensions add, coordinates
/// concatenate, `R⁰` is the unit, `⊕` is associative on the nose, and
/// `flatten` agrees with the generic action — for several module triples
/// (including empties).
#[test]
fn direct_sum_monoid_deterministic() {
    assert_direct_sum_monoid(vec![1.0, 2.0], vec![3.0], vec![4.0, 5.0, 6.0]);
    assert_direct_sum_monoid(vec![], vec![9.0], vec![]);
    assert_direct_sum_monoid(vec![], vec![], vec![]);
    assert_direct_sum_monoid(vec![-1.5], vec![2.5, -3.5], vec![0.0]);
}

/// **Actegory action + multiplicator `µ` — direct.** `act(p, x) = p ⊕ x` and
/// `compose_action(q, p, x) = (q ⊕ p) ⊕ x`, the exact `DirectSum`
/// re-association `µ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X` matching [`F64Monoidal`]'s
/// tensor. Cross-checks the `(q ⊕ p)` slot against `tensor_objects(q, p)` and
/// realises the concrete concatenation via `flatten`.
#[test]
fn actegory_action_and_multiplicator() {
    let acteg = F64Actegory::new();
    let mono = F64Monoidal::new();

    // act(p, x) == p ⊕ x (abstract), and its flatten is the concatenation.
    let p = F64Module::new(vec![7.0]);
    let x = F64Module::new(vec![13.0, 14.0]);
    let acted: DirectSum<F64Module, F64Module> = acteg.act(p.clone(), x.clone());
    assert_eq!(acted, DirectSum(p.clone(), x.clone()));
    assert_eq!(acted.flatten(), F64Module::new(vec![7.0, 13.0, 14.0]));

    // µ : Q ▶ (P ▶ X) → (Q ⊗ P) ▶ X is ((q ⊕ p) ⊕ x).
    let mu = acteg.compose_action(2_i64, 3_i64, 5_i64);
    assert_eq!(mu, DirectSum(DirectSum(2_i64, 3_i64), 5_i64));

    // The (q ⊕ p) slot equals SetMonoidal-style tensor_objects(q, p) on this
    // monoidal category — i.e. µ's parameter block is the monoidal tensor.
    let DirectSum(qp, _) = mu;
    assert_eq!(qp, mono.tensor_objects(2_i64, 3_i64));

    // Concrete module multiplicator: (q ⊕ p) ⊕ x flattens to full concatenation.
    let q_m = F64Module::new(vec![1.0]);
    let p_m = F64Module::new(vec![2.0, 3.0]);
    let x_m = F64Module::new(vec![4.0]);
    let mu_m: DirectSum<DirectSum<F64Module, F64Module>, F64Module> =
        acteg.compose_action(q_m, p_m, x_m);
    let DirectSum(inner, x_out) = mu_m;
    assert_eq!(
        inner.flatten().direct_sum(x_out),
        F64Module::new(vec![1.0, 2.0, 3.0, 4.0]),
        "µ realises (q ⊕ p) ⊕ x as one concatenated module"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Pentagon / triangle / unitor laws for [`F64Monoidal`] across arbitrary
    /// object samples.
    #[test]
    fn direct_sum_coherence_proptest(
        a in any::<i32>(),
        b in any::<u8>(),
        c in any::<i64>(),
        d in any::<bool>(),
    ) {
        assert_direct_sum_coherence(&F64Monoidal::new(), a, b, c, d);
    }

    /// `R`-module axioms across finite (NaN-free) coordinate vectors and
    /// scalars.
    #[test]
    fn f64_module_axioms_proptest(
        coords in proptest::collection::vec(finite_f64(), 0..8),
        r in finite_f64(),
    ) {
        assert_f64_module_axioms(coords, r);
    }

    /// `⊕`-monoid laws across finite module triples.
    #[test]
    fn direct_sum_monoid_proptest(
        u in proptest::collection::vec(finite_f64(), 0..6),
        v in proptest::collection::vec(finite_f64(), 0..6),
        w in proptest::collection::vec(finite_f64(), 0..6),
    ) {
        assert_direct_sum_monoid(u, v, w);
    }
}
