//! Rayon threshold correctness validation for catgraph-applied modules.
//!
//! Verifies that operations produce correct results at sizes above
//! their rayon parallelism thresholds. Does not test performance.


use catgraph::category::{Composable, HasIdentity};
use catgraph_applied::{linear_combination::LinearCombination, temperley_lieb::BrauerMorphism};
use std::collections::HashMap;

/// `LinearCombination` Mul with 64 terms (threshold = 32).
#[test]
fn linear_combination_above_threshold() {
    let terms_a: HashMap<i32, i64> = (0..64).map(|i| (i, (i + 1).into())).collect();
    let terms_b: HashMap<i32, i64> = (0..64).map(|i| (i, 1i64)).collect();
    let lc_a: LinearCombination<i64, i32> = terms_a.into_iter().collect();
    let lc_b: LinearCombination<i64, i32> = terms_b.into_iter().collect();

    // Multiplication distributes over basis: (c1*t1) * (c2*t2) = (c1*c2) * (t1*t2)
    // For i32 basis, Mul<Output=i32> multiplies basis elements.
    let product = lc_a * lc_b;

    // Product should be non-empty (64 * 64 = 4096 cross-terms before simplification)
    let simplified = {
        let mut p = product;
        p.simplify();
        p
    };
    // At minimum, (0 * anything) = 0 basis elements get folded, but nonzero entries remain
    assert_ne!(simplified, LinearCombination::default());
}

/// `BrauerMorphism` compose at n=16 (threshold = 8 for `non_crossing` checks).
#[test]
fn temperley_lieb_above_threshold() {
    let gens: Vec<BrauerMorphism<i64>> = BrauerMorphism::temperley_lieb_gens(16);
    assert_eq!(gens.len(), 15);

    // Compose e_1 * e_2 — triggers diagram stacking with 16 source/target points
    let composed = gens[0].compose(&gens[1]).unwrap();
    assert_eq!(composed.domain(), 16);
    assert_eq!(composed.codomain(), 16);

    // Compose identity with a generator — should equal the generator
    let id: BrauerMorphism<i64> = BrauerMorphism::identity(&16);
    let id_composed = id.compose(&gens[7]).unwrap();
    assert_eq!(id_composed, gens[7]);
}
