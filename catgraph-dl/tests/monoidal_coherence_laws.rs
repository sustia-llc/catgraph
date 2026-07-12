//! Pentagon / triangle coherence-law acceptance tests for the monoidal surface.
//!
//! Mac Lane's coherence theorem; CDL §3.1 (the parameter category `M` of
//! `Para(M, C)` is monoidal). The `(Set, ×, 1)` blanket supplied by
//! [`SetCategoryDefaults`] fixes `Tensor<A, B> = (A, B)` and `Unit = ()`, so
//! the associator and unitors are exact tuple bijections. This file
//! machine-checks that the blanket bodies satisfy the pentagon, the triangle,
//! and the unitor sanity equations — for `SetMonoidal` **and** a fresh
//! test-local ZST opting in via the dual-impl soft-seal, proving the laws hold
//! for the blanket bodies any downstream `(Set, ×, 1)`-flavoured ZST inherits,
//! not just `SetMonoidal` (issue #40).
//!
//! The shared checker [`common::assert_monoidal_coherence`] is generic over
//! [`MonoidalCategory`] and expresses the `α ⊗ id` / `id ⊗ α` legs through
//! [`MonoidalCategory::tensor_morphisms`] (issue #65), so the same driver
//! serves this tuple carrier and the `DirectSum` carrier of
//! `module_actegory_laws.rs`.

mod common;

use common::assert_monoidal_coherence;

use catgraph_dl::para::{Sealed, SetCategoryDefaults, SetMonoidal};

use proptest::prelude::*;

/// A fresh `(Set, ×, 1)`-flavoured ZST opting into the canonical blanket via
/// the dual-impl soft-seal (`Sealed` then `SetCategoryDefaults`) — mirrors the
/// `SetCategoryDefaults` doctest. Exists to prove the coherence laws hold for
/// the blanket bodies, not just for `SetMonoidal`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TestMonoidal;

impl Sealed for TestMonoidal {}
impl SetCategoryDefaults for TestMonoidal {}

/// **Pentagon + triangle + unitor sanity — deterministic.** Both `SetMonoidal`
/// and the downstream-style `TestMonoidal` ZST satisfy the coherence laws on a
/// hand-picked spread of values (including sign extremes and both booleans).
#[test]
fn monoidal_coherence_deterministic() {
    let set = SetMonoidal::new();
    let test = TestMonoidal;

    for (a, b, c, d) in [
        (1_i32, 2_u8, 3_i64, true),
        (-5_i32, 200_u8, -9999_i64, false),
        (i32::MIN, u8::MAX, i64::MAX, true),
        (0_i32, 0_u8, 0_i64, false),
    ] {
        assert_monoidal_coherence(&set, a, b, c, d);
        assert_monoidal_coherence(&test, a, b, c, d);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// The pentagon, triangle, and unitor laws hold for `SetMonoidal` and the
    /// downstream-style `TestMonoidal` ZST across arbitrary value samples.
    #[test]
    fn monoidal_coherence_proptest(
        a in any::<i32>(),
        b in any::<u8>(),
        c in any::<i64>(),
        d in any::<bool>(),
    ) {
        assert_monoidal_coherence(&SetMonoidal::new(), a, b, c, d);
        assert_monoidal_coherence(&TestMonoidal, a, b, c, d);
    }
}
