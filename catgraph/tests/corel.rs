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
    let c = Cospan::new(vec![0], vec![1], vec!['a', 'b', 'c']).unwrap();
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
    let f = Corel::<char>::new(Cospan::new(vec![0], vec![0, 0], vec!['a']).unwrap()).unwrap();
    let g = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0], vec!['a']).unwrap()).unwrap();
    let fg = f.compose(&g).unwrap();
    assert!(fg.as_cospan().is_jointly_surjective());
}

/// The partition a composite *induces*, as sorted flat-index classes, so the
/// comparison is deterministic.
///
/// Flat layout is [`Corel::equivalence_classes`]'s: `0..dom` for the domain,
/// then the apex vertices, then the codomain.
fn partition(c: &Corel<char>) -> Vec<Vec<usize>> {
    let mut classes: Vec<Vec<usize>> = c
        .equivalence_classes()
        .into_iter()
        .map(|class| {
            let mut members: Vec<usize> = class.into_iter().collect();
            members.sort_unstable();
            members
        })
        .collect();
    classes.sort();
    classes
}

/// Composition in `Corel` produces the right **partition**, not merely a
/// jointly-surjective one.
///
/// [`compose_preserves_joint_surjectivity`] above and the rest of this file's
/// composition coverage assert only the invariant `Corel::new` checks, which a
/// composite that merged the wrong wires would satisfy just as happily — every
/// composite here is jointly surjective under a wrong μ too. These name the
/// whole class structure.
///
/// Ranges over three composites on one wire type at arities ≤ 3. It says
/// nothing about heterogeneous labels or about the tensor.
#[test]
fn composites_induce_the_expected_partition() {
    use catgraph::hypergraph_category::HypergraphCategory;

    let mu = || Corel::<char>::multiplication('a');
    let delta = || Corel::<char>::comultiplication('a');
    let id = || Corel::<char>::identity(&vec!['a']);

    // δ ; μ : [a] → [a]. One apex vertex; flat indices dom 0, apex 1, cod 2.
    let special = delta().compose(&mu()).unwrap();
    assert_eq!(
        partition(&special),
        vec![vec![0, 1, 2]],
        "delta ; mu should leave one class joining the single domain wire, the apex vertex and \
         the single codomain wire"
    );
    assert!(
        special.is_identity_partition(),
        "delta ; mu is the identity partition on one wire"
    );

    // (μ ⊗ id) ; μ : [a, a, a] → [a]. Everything lands on one apex vertex;
    // flat indices dom 0,1,2, apex 3, cod 4.
    let mut mu_id = mu();
    mu_id.monoidal(id());
    let fold = mu_id.compose(&mu()).unwrap();
    assert_eq!(
        partition(&fold),
        vec![vec![0, 1, 2, 3, 4]],
        "(mu (x) id) ; mu should merge all three domain wires with the codomain wire"
    );

    // μ ; δ : [a, a] → [a, a] — the Frobenius "H", still one class:
    // flat indices dom 0,1, apex 2, cod 3,4.
    let h = mu().compose(&delta()).unwrap();
    assert_eq!(
        partition(&h),
        vec![vec![0, 1, 2, 3, 4]],
        "mu ; delta should join both domain wires to both codomain wires"
    );

    // id ⊗ id keeps the two wires apart — the control that says the assertions
    // above are not just "everything merges".
    let mut two = id();
    two.monoidal(id());
    assert_eq!(
        partition(&two),
        vec![vec![0, 2, 4], vec![1, 3, 5]],
        "id (x) id must keep the two wires in separate classes"
    );
}

#[test]
fn monoidal_product_preserves_joint_surjectivity() {
    let mut a = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['a']).unwrap()).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['b']).unwrap()).unwrap();
    a.monoidal(b);
    assert!(a.as_cospan().is_jointly_surjective());
}

#[test]
fn equivalence_classes_count_matches_middle_size() {
    let c = Cospan::new(vec![0, 1, 2], vec![0, 1, 2], vec!['a', 'b', 'c']).unwrap();
    let corel = Corel::new(c).unwrap();
    assert_eq!(corel.equivalence_classes().len(), 3);
}

#[test]
fn merges_transitive_through_middle() {
    // [0, 1] → [0, 1] with middle ['a', 'a']: two separate classes.
    // Flat layout: dom[0,1] at indices 0,1; middle[0,1] at 2,3; cod[0,1] at 4,5.
    let c = Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).unwrap();
    let corel = Corel::new(c).unwrap();
    assert!(corel.merges(0, 2)); // dom[0] <-> middle[0]
    assert!(corel.merges(0, 4)); // dom[0] <-> cod[0]
    assert!(!corel.merges(0, 1)); // dom[0] != dom[1]
}

#[test]
fn refines_rejects_shape_mismatch() {
    let a = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['a']).unwrap()).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a']).unwrap()).unwrap();
    assert!(matches!(a.refines(&b), Err(CatgraphError::Corel { .. })));
}

#[test]
fn ccr_rejects_shape_mismatch() {
    let a = Corel::<char>::new(Cospan::new(vec![0], vec![0], vec!['a']).unwrap()).unwrap();
    let b = Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a']).unwrap()).unwrap();
    assert!(matches!(
        a.coarsest_common_refinement(&b),
        Err(CatgraphError::Corel { .. })
    ));
}

#[test]
fn symmetric_braiding_preserves_surjectivity() {
    let braid = Corel::<char>::from_permutation_on_domain(
        permutations::Permutation::transposition(2, 0, 1),
        &['a', 'b'],
    )
    .unwrap();
    assert!(braid.as_cospan().is_jointly_surjective());
}

#[test]
fn ccr_merges_non_trivial_partition_pair() {
    // fine: [a, a] → [a, a] with each entry in its own class (2 classes total).
    // coarse: [a, a] → [a, a] with everything merged into one class.
    // CCR(fine, coarse) should merge: one class covering all boundary entries.
    let fine =
        Corel::<char>::new(Cospan::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).unwrap()).unwrap();
    let coarse =
        Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a']).unwrap()).unwrap();

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
    let non_id =
        Corel::<char>::new(Cospan::new(vec![0, 0], vec![0, 0], vec!['a']).unwrap()).unwrap();
    assert!(!non_id.is_identity_partition());
}

#[test]
fn corel_clone_and_debug_smoke() {
    // `Corel<Lambda>` derives `Clone` and `Debug`. Downstream coalition work
    // needs both: snapshot APIs clone weighted cospans,
    // tracing emits `Debug` on logged values.
    let c = Corel::<char>::new(Cospan::new(vec![0], vec![1], vec!['a', 'b']).unwrap()).unwrap();
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
