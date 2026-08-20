#![allow(
    clippy::cast_possible_truncation,  // usize fixture sizes fit in i32 by construction
    clippy::cast_possible_wrap,
)]

//! Rayon threshold correctness validation.
//!
//! Verifies that operations produce correct results at sizes large enough to
//! run the parallel arm. Does not test performance.
//!
//! Distinct in purpose from `tests/rayon_equivalence.rs`: this file checks
//! above-threshold *correctness* (results are right once the input is large
//! enough to fan out), whereas `rayon_equivalence.rs` pins
//! parallel-output-equals-sequential-reference *equivalence*, mirroring the
//! same split in `catgraph-applied`.

use catgraph::{
    category::ComposableMutating,
    frobenius::{FrobeniusMorphism, frobenius_to_cospan, special_frobenius_morphism},
    named_cospan::NamedCospan,
};
use either::Either::{Left, Right};

/// `NamedCospan` `find_nodes_by_name_predicate` with 512 boundary nodes (threshold = 256).
#[test]
fn named_cospan_predicate_above_threshold() {
    // Build a NamedCospan with 300 left nodes and 300 right nodes (total 600 >= 256)
    // Each maps to a distinct middle node.
    let n = 300;
    let left: Vec<usize> = (0..n).collect();
    let right: Vec<usize> = (n..2 * n).collect();
    let middle: Vec<char> = (0..2 * n).map(|_| 'x').collect();
    let left_names: Vec<i32> = (0..n as i32).collect();
    let right_names: Vec<i32> = (n as i32..2 * n as i32).collect();

    let nc: NamedCospan<char, i32, i32> =
        NamedCospan::new(left, right, middle, left_names, right_names).unwrap();

    // Find all even-named nodes (should hit the parallel path)
    let found = nc.find_nodes_by_name_predicate(|n| n % 2 == 0, |n| n % 2 == 0, false);

    // 150 even names on left (0,2,...,298) + 150 even names on right (300,302,...,598)
    assert_eq!(found.len(), 300);

    // Verify Left/Right classification
    let left_count = found.iter().filter(|e| matches!(e, Left(_))).count();
    let right_count = found.iter().filter(|e| matches!(e, Right(_))).count();
    assert_eq!(left_count, 150);
    assert_eq!(right_count, 150);
}

/// `FrobeniusMorphism` `hflip` on a layer rayon genuinely subdivides.
///
/// `special_frobenius_morphism(m, 1, wire_type)` for large m builds a binary μ
/// tree: `m/2` blocks in the first layer, then `m/4`, and so on. Calling `hflip`
/// (through `from_permutation` or a direct `special_frobenius_morphism` with
/// m < n) runs the parallel arm, which rayon actually subdivides once a layer
/// reaches ≥ 128 blocks — `with_min_len(64)` splits only at length ≥ 2·min.
///
/// # The arity, and why it is 256 and not 128
///
/// The widest layer of `special_frobenius_morphism(m, 1, _)` holds `m/2` blocks,
/// so **m = 256** is the first power of two whose widest layer (128) is one
/// rayon subdivides. At m = 128 the widest layer is 64 blocks, exactly
/// `PARALLEL_BLOCK_THRESHOLD` and half of what `with_min_len` needs — this test
/// used to run at 128 and never reached the arm it is named for.
///
/// # What this asserts, and what it used to
///
/// The composition below is a **real** one: `(1 → 256) ; (256 → 1)`, checked
/// against the identity wire through [`frobenius_to_cospan`]. It replaces a
/// `composed.monoidal(FrobeniusMorphism::identity(&vec![]))` — a tensor with the
/// empty identity, i.e. a no-op on the value and no composition at all. That
/// version, plus the two `depth() > 0` checks, stayed green with `hflip` made a
/// complete no-op, while 24 tests in eight other binaries went red.
///
/// The first two assertions alone would catch that mutant (a no-op `hflip`
/// leaves a `[a; 256] → [a]` word where `[a] → [a; 256]` is claimed). The
/// interpretation check adds that the fold denotes the *identity wire* — one
/// connected component on one input and one output — rather than merely having
/// the right interfaces.
///
/// What it does **not** add, measured rather than assumed: block order. A
/// `hflip` rebuilt with each layer's blocks reversed (placements recomputed, so
/// the layer stays well-formed) leaves this test green at 256, because every
/// layer of `special_frobenius_morphism(m, 1, 'a')` is homogeneous — all
/// `Multiplication`, and all `Comultiplication` after the flip — so on this
/// input the block order carries no information at all. The same mutant is
/// caught by `frobenius_layer_hflip_matches_sequential_reference` in
/// `src/frobenius/operations.rs`, which compares against a sequential reference
/// over a *heterogeneous* layer (`wide_frobenius_layer`); block-order
/// sensitivity lives there, not here.
///
/// Scope: one wire type, the single arity pair (1, 256). It is a correctness
/// check on the above-threshold arm, not a survey of `hflip`.
#[test]
fn frobenius_hflip_above_threshold() {
    // 256 inputs → 1 output: the widest layer is 128 μ blocks, which is the
    // first width `with_min_len(64)` subdivides.
    const WIDTH: usize = 256;

    let morph: FrobeniusMorphism<char, String> = special_frobenius_morphism(WIDTH, 1, 'a');
    assert_eq!(morph.domain(), vec!['a'; WIDTH]);
    assert_eq!(morph.codomain(), vec!['a']);

    // Now trigger hflip by building 1 → 256 (internally calls hflip on 256→1)
    let morph_flipped: FrobeniusMorphism<char, String> = special_frobenius_morphism(1, WIDTH, 'a');
    assert_eq!(morph_flipped.domain(), vec!['a']);
    assert_eq!(morph_flipped.codomain(), vec!['a'; WIDTH]);

    // Compose for real: (1 → 256) ; (256 → 1), a connected diagram on one input
    // and one output, which by the spider theorem is the identity wire.
    let mut composed = morph_flipped;
    ComposableMutating::compose(&mut composed, morph).unwrap();
    assert_eq!(composed.domain(), vec!['a']);
    assert_eq!(composed.codomain(), vec!['a']);

    let identity_wire: FrobeniusMorphism<char, String> = special_frobenius_morphism(1, 1, 'a');
    assert_eq!(
        frobenius_to_cospan(&composed).unwrap().canonical_form(),
        frobenius_to_cospan(&identity_wire)
            .unwrap()
            .canonical_form(),
        "the 1 → 256 → 1 fold does not denote the identity wire"
    );
}
