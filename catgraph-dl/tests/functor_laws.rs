//! Functor-law acceptance tests for the three shipped endofunctor witnesses.
//!
//! Identity + composition (CDL Def 1.4 morphism-map laws; `deep_causality_haft`
//! `Functor` law docs) for `ListEndo`, `TreeEndo`, and `GroupActionEndo`. Each
//! feeds per-witness sample values into the single generic
//! [`common::assert_functor_laws`] helper — previously these were documented
//! obligations only. Kept here (not in `free_monad_bijections.rs`, whose scope
//! is the `Vec`/`BinaryTree` bijections) so the law coverage lives in one place.

mod common;

use common::assert_functor_laws;

use catgraph_dl::Either;
use catgraph_dl::algebra::{GroupActionEndo, Z2Group};
use catgraph_dl::free_monad::list_endo::ListEndo;
use catgraph_dl::free_monad::tree_endo::TreeEndo;

use proptest::prelude::*;

// `ListEndo<u32>` (`1 + A × −`, object map `Option<(A, X)>`). Random payloads
// over the full `Option<(u32, i32)>` shape.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn list_endo_functor_laws(fx in proptest::option::of((any::<u32>(), any::<i32>()))) {
        assert_functor_laws::<ListEndo<u32>>(fx);
    }
}

// `TreeEndo<u32>` (`A + (−)²`, object map `Either<A, (X, X)>`). The `Right` arm
// calls the morphism twice, exercising the multi-call path.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn tree_endo_functor_laws(
        fx in prop_oneof![
            any::<u32>().prop_map(Either::Left),
            (any::<i32>(), any::<i32>()).prop_map(Either::Right),
        ]
    ) {
        assert_functor_laws::<TreeEndo<u32>>(fx);
    }
}

/// `GroupActionEndo<Z2Group>` (`G × −`, object map `(G, X)`). Finite tuple
/// carrier, so a handful of `(g, x)` samples suffice — plain unit test through
/// the same generic law helper.
#[test]
fn group_action_endo_functor_laws() {
    for fx in [
        (Z2Group(false), 0_i32),
        (Z2Group(true), 5),
        (Z2Group(false), -7),
        (Z2Group(true), 42),
    ] {
        assert_functor_laws::<GroupActionEndo<Z2Group>>(fx);
    }
}
