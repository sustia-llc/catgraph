//! Integration tests for `Corel<Lambda>` — constructor validation,
//! equivalence-class extraction, and composition preserving joint surjectivity.

mod common;

use catgraph::{
    category::{Composable, HasIdentity},
    corel::Corel,
    cospan::Cospan,
    errors::CatgraphError,
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
};

#[test]
fn new_rejects_non_surjective_middle_larger_than_boundary() {
    // Two boundary entries, three middle vertices — last one uncovered.
    let c = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']);
    let result = Corel::new(c);
    assert!(matches!(result, Err(CatgraphError::Corel { .. })));
}

#[test]
fn identity_corel_round_trips() {
    let types = vec!['a', 'b', 'c'];
    let id = Corel::<char>::identity(&types);
    let composed = id.compose(&id).unwrap();
    common::assert_corel_eq(&id, &composed);
}

#[test]
fn compose_preserves_joint_surjectivity() {
    let f = Corel::<char>::new(Cospan::new(vec![0], vec![0, 0], vec!['a'])).unwrap();
    let g = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0], vec!['a'])).unwrap();
    let fg = f.compose(&g).unwrap();
    assert!(fg.as_cospan().is_jointly_surjective());
}

#[test]
fn monoidal_product_preserves_joint_surjectivity() {
    let mut a = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['a'])).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['b'])).unwrap();
    a.monoidal(b);
    assert!(a.as_cospan().is_jointly_surjective());
}

#[test]
fn equivalence_classes_count_matches_middle_size() {
    let c = Cospan::new(vec![0, 1, 2], vec![0, 1, 2], vec!['a', 'b', 'c']);
    let corel = Corel::new(c).unwrap();
    assert_eq!(corel.equivalence_classes().len(), 3);
}

#[test]
fn merges_transitive_through_middle() {
    // [0, 1] → [0, 1] with middle ['a', 'a']: two separate classes.
    // Flat layout: dom[0,1] at indices 0,1; middle[0,1] at 2,3; cod[0,1] at 4,5.
    let c = Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a']);
    let corel = Corel::new(c).unwrap();
    assert!(corel.merges(0, 2)); // dom[0] <-> middle[0]
    assert!(corel.merges(0, 4)); // dom[0] <-> cod[0]
    assert!(!corel.merges(0, 1)); // dom[0] != dom[1]
}

#[test]
fn refines_rejects_shape_mismatch() {
    let a = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['a'])).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a'])).unwrap();
    assert!(matches!(a.refines(&b), Err(CatgraphError::Corel { .. })));
}

#[test]
fn ccr_rejects_shape_mismatch() {
    let a = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['a'])).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a'])).unwrap();
    assert!(matches!(
        a.coarsest_common_refinement(&b),
        Err(CatgraphError::Corel { .. })
    ));
}

#[test]
fn symmetric_braiding_preserves_surjectivity() {
    let braid = Corel::<char>::from_permutation(
        permutations::Permutation::transposition(2, 0, 1),
        &['a', 'b'],
        true,
    )
    .unwrap();
    assert!(braid.as_cospan().is_jointly_surjective());
}

#[test]
fn ccr_merges_non_trivial_partition_pair() {
    // fine: [a, a] → [a, a] with each entry in its own class (2 classes total).
    // coarse: [a, a] → [a, a] with everything merged into one class.
    // CCR(fine, coarse) should merge: one class covering all boundary entries.
    let fine = Corel::<char>::new(Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a'])).unwrap();
    let coarse = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a'])).unwrap();

    let ccr = fine.coarsest_common_refinement(&coarse).unwrap();
    // The result should have exactly one class (the coarse merger propagates
    // through the fine partition).
    assert_eq!(ccr.equivalence_classes().len(), 1);
    // And every boundary entry is in that single class — both fine and coarse
    // are refinements of the result.
    assert!(fine.refines(&ccr).unwrap());
    assert!(coarse.refines(&ccr).unwrap());
}

#[test]
fn is_identity_partition_false_for_same_length_non_identity_map() {
    // Same domain/codomain length (2) but legs are [0, 0] → [0, 0] with middle ['a']:
    // everything collapses to one class. Not the identity partition.
    let non_id = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a'])).unwrap();
    assert!(!non_id.is_identity_partition());
}

#[test]
fn corel_clone_and_debug_smoke() {
    // `Corel<Lambda>` derives `Clone` and `Debug`. Downstream coalition work
    // needs both: snapshot APIs clone weighted cospans,
    // tracing emits `Debug` on logged values.
    let c = Corel::<char>::new(Cospan::new(vec![0], vec![1], vec!['a', 'b'])).unwrap();
    let cloned = c.clone();
    common::assert_corel_eq(&c, &cloned);

    let formatted = format!("{c:?}");
    assert!(
        !formatted.is_empty(),
        "Debug should produce non-empty output"
    );
    assert!(
        formatted.contains("Corel"),
        "Debug output should reference the type name, got {formatted}"
    );
}
