//! Property-based tests for algebraic laws using proptest.
//!
//! Verifies identity, associativity, dagger involution, monoidal interchange,
//! and relation algebra laws hold for randomly generated cospans, spans, and relations.

mod common;
use common::*;

use catgraph::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    monoidal::Monoidal,
    span::{Rel, Span},
};
use catgraph_testutil::all_perm_indices;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::strategy::ValueTree;
use proptest::test_runner::{RngAlgorithm, TestRng, TestRunner};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Deterministic strategy sampling (for the generator meta-tests)
// ---------------------------------------------------------------------------

/// Draw `n` values from `strategy` under a *deterministic* RNG.
///
/// The generator meta-tests below assert measured coverage counts, and a count
/// is only worth printing in a failure message if rerunning the test reproduces
/// it — so these samples do not use proptest's per-run seed.
fn sample_n<S: Strategy>(strategy: &S, n: usize) -> Vec<S::Value> {
    let mut runner = TestRunner::new_with_rng(
        ProptestConfig::default(),
        TestRng::deterministic_rng(RngAlgorithm::ChaCha),
    );
    (0..n)
        .map(|_| {
            strategy
                .new_tree(&mut runner)
                .expect("strategy is total on this runner")
                .current()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Debug wrapper for Span (Span doesn't derive Debug)
// ---------------------------------------------------------------------------

/// Wrapper that gives `Span<char>` a `Debug` impl for proptest shrinking output.
#[derive(Clone)]
struct DebugSpan(Span<char>);

impl std::fmt::Debug for DebugSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("left", &self.0.left())
            .field("right", &self.0.right())
            .field("middle_pairs", &self.0.middle_pairs())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Connectivity checker (used by identity and associativity tests)
// ---------------------------------------------------------------------------

/// Compare two cospans for "connectivity equivalence": same domain, codomain,
/// and for every pair of boundary nodes, they share a middle node iff they do
/// in the other cospan.
fn assert_connectivity_eq(
    a: &Cospan<char>,
    b: &Cospan<char>,
    label: &str,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(a.domain(), b.domain(), "{}: domain mismatch", label);
    prop_assert_eq!(a.codomain(), b.codomain(), "{}: codomain mismatch", label);
    prop_assert_eq!(
        a.left_to_middle().len(),
        b.left_to_middle().len(),
        "{}: left leg size",
        label,
    );
    prop_assert_eq!(
        a.right_to_middle().len(),
        b.right_to_middle().len(),
        "{}: right leg size",
        label,
    );

    let n_left = a.left_to_middle().len();
    let n_right = a.right_to_middle().len();

    // left-left
    for i in 0..n_left {
        for j in 0..n_left {
            let a_same = a.left_to_middle()[i] == a.left_to_middle()[j];
            let b_same = b.left_to_middle()[i] == b.left_to_middle()[j];
            prop_assert!(
                a_same == b_same,
                "{}: left-left connectivity at ({}, {}): {} vs {}",
                label,
                i,
                j,
                a_same,
                b_same,
            );
        }
    }
    // right-right
    for i in 0..n_right {
        for j in 0..n_right {
            let a_same = a.right_to_middle()[i] == a.right_to_middle()[j];
            let b_same = b.right_to_middle()[i] == b.right_to_middle()[j];
            prop_assert!(
                a_same == b_same,
                "{}: right-right connectivity at ({}, {}): {} vs {}",
                label,
                i,
                j,
                a_same,
                b_same,
            );
        }
    }
    // left-right cross
    for i in 0..n_left {
        for j in 0..n_right {
            let a_same = a.left_to_middle()[i] == a.right_to_middle()[j];
            let b_same = b.left_to_middle()[i] == b.right_to_middle()[j];
            prop_assert!(
                a_same == b_same,
                "{}: left-right connectivity at ({}, {}): {} vs {}",
                label,
                i,
                j,
                a_same,
                b_same,
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Generate a valid `Cospan<char>` with small boundaries.
///
/// - `domain_size` and `codomain_size` in 0..=5
/// - `middle_size` >= max(domain, codomain), at least 1 when either boundary is non-empty
/// - `left_to_middle`: each index in `0..middle_size`
/// - `right_to_middle`: each index in `0..middle_size`
/// - middle labels drawn from `'a'..'f'`
fn arb_cospan() -> impl Strategy<Value = Cospan<char>> {
    (0_usize..=5, 0_usize..=5)
        .prop_flat_map(|(domain_size, codomain_size)| {
            let min_middle = if domain_size == 0 && codomain_size == 0 {
                0
            } else {
                domain_size.max(codomain_size).max(1)
            };
            let max_middle = min_middle + 3;
            (
                Just(domain_size),
                Just(codomain_size),
                min_middle..=max_middle,
            )
        })
        .prop_flat_map(|(domain_size, codomain_size, middle_size)| {
            let labels = prop::collection::vec(
                prop::sample::select(vec!['a', 'b', 'c', 'd', 'e', 'f']),
                middle_size,
            );
            let left = if middle_size > 0 {
                prop::collection::vec(0..middle_size, domain_size).boxed()
            } else {
                Just(vec![]).boxed()
            };
            let right = if middle_size > 0 {
                prop::collection::vec(0..middle_size, codomain_size).boxed()
            } else {
                Just(vec![]).boxed()
            };
            (left, right, labels)
        })
        .prop_map(|(left, right, middle)| Cospan::new(left, right, middle).unwrap())
}

/// A left leg for a boundary word `boundary`, into an apex labelled `apex`.
///
/// Every slot `i` is sent to some apex vertex `j` with
/// `apex[j] == boundary[i]`, chosen uniformly among the vertices carrying that
/// label — so the leg is **label-aware** (the generated cospan's `domain()` is
/// `boundary` on the nose, which is what keeps it composable) but **not forced
/// to be the identity injection** `i ↦ i`, which is what the composability
/// generators used to emit.
///
/// # Panics
///
/// Panics if some slot's label is absent from `apex`. Both call sites build
/// `apex` as `boundary ++ extras`, so slot `i` always has at least vertex `i`.
fn arb_label_preserving_leg(
    boundary: &[char],
    apex: &[char],
    // `use<>`: the returned strategy owns its candidate table, so it must not
    // capture either input's lifetime — both call sites build `apex` as a local.
) -> impl Strategy<Value = Vec<usize>> + use<> {
    let candidates: Vec<Vec<usize>> = boundary
        .iter()
        .map(|&label| {
            let targets: Vec<usize> = apex
                .iter()
                .enumerate()
                .filter(|&(_, &a)| a == label)
                .map(|(j, _)| j)
                .collect();
            assert!(
                !targets.is_empty(),
                "apex {apex:?} carries no vertex labelled {label:?}"
            );
            targets
        })
        .collect();

    prop::collection::vec(any::<Index>(), candidates.len()).prop_map(move |choices| {
        choices
            .iter()
            .zip(candidates.iter())
            .map(|(choice, targets)| targets[choice.index(targets.len())])
            .collect()
    })
}

/// Generate two composable cospans: f: A -> B and g: B -> C.
///
/// We generate `f` first, then build `g` so that `g.domain() == f.codomain()`.
fn arb_composable_cospans() -> impl Strategy<Value = (Cospan<char>, Cospan<char>)> {
    arb_cospan().prop_flat_map(|f| {
        let codomain = f.codomain();
        let b_size = codomain.len();

        (0_usize..=5,)
            .prop_flat_map(move |(codomain_g_size,)| {
                let b_size = b_size;
                let codomain = codomain.clone();

                let min_middle_g = if b_size == 0 && codomain_g_size == 0 {
                    0
                } else {
                    b_size.max(codomain_g_size).max(1)
                };
                let max_middle_g = min_middle_g + 3;

                (
                    Just(b_size),
                    Just(codomain_g_size),
                    Just(codomain.clone()),
                    min_middle_g..=max_middle_g,
                )
            })
            .prop_flat_map(move |(b_size, codomain_g_size, codomain, middle_g_size)| {
                let extra_count = middle_g_size - b_size.min(middle_g_size);
                let extra_labels = prop::collection::vec(
                    prop::sample::select(vec!['a', 'b', 'c', 'd', 'e', 'f']),
                    extra_count,
                );
                let right_g = if middle_g_size > 0 {
                    prop::collection::vec(0..middle_g_size, codomain_g_size).boxed()
                } else {
                    Just(vec![]).boxed()
                };
                (Just(codomain), right_g, extra_labels)
            })
            // The left leg is drawn *after* the apex labels are known, so it can
            // land on any equally-labelled vertex rather than only on `i ↦ i`.
            .prop_flat_map(|(codomain, right_g, extra_labels)| {
                let mut middle_g: Vec<char> = codomain.clone();
                middle_g.extend(extra_labels);
                let left_g = arb_label_preserving_leg(&codomain, &middle_g);
                (Just(middle_g), left_g, Just(right_g))
            })
            .prop_map(move |(middle_g, left_g, right_g)| {
                let g = Cospan::new(left_g, right_g, middle_g).unwrap();
                (f.clone(), g)
            })
    })
}

/// Generate three composable cospans: f: A -> B, g: B -> C, h: C -> D.
fn arb_three_composable_cospans()
-> impl Strategy<Value = (Cospan<char>, Cospan<char>, Cospan<char>)> {
    arb_composable_cospans().prop_flat_map(|(f, g)| {
        let codomain_g = g.codomain();
        let c_size = codomain_g.len();

        (0_usize..=4,)
            .prop_flat_map(move |(codomain_h_size,)| {
                let c_size = c_size;
                let codomain_g = codomain_g.clone();
                let min_middle_h = if c_size == 0 && codomain_h_size == 0 {
                    0
                } else {
                    c_size.max(codomain_h_size).max(1)
                };
                let max_middle_h = min_middle_h + 2;
                (
                    Just(c_size),
                    Just(codomain_h_size),
                    Just(codomain_g.clone()),
                    min_middle_h..=max_middle_h,
                )
            })
            .prop_flat_map(
                move |(c_size, codomain_h_size, codomain_g, middle_h_size)| {
                    let extra_count = middle_h_size - c_size.min(middle_h_size);
                    let extra_labels = prop::collection::vec(
                        prop::sample::select(vec!['a', 'b', 'c', 'd', 'e', 'f']),
                        extra_count,
                    );
                    let right_h = if middle_h_size > 0 {
                        prop::collection::vec(0..middle_h_size, codomain_h_size).boxed()
                    } else {
                        Just(vec![]).boxed()
                    };
                    (Just(codomain_g), right_h, extra_labels)
                },
            )
            // As in `arb_composable_cospans`: label-aware, not identity-only.
            .prop_flat_map(|(codomain_g, right_h, extra_labels)| {
                let mut middle_h: Vec<char> = codomain_g.clone();
                middle_h.extend(extra_labels);
                let left_h = arb_label_preserving_leg(&codomain_g, &middle_h);
                (Just(middle_h), left_h, Just(right_h))
            })
            .prop_map(move |(middle_h, left_h, right_h)| {
                let h = Cospan::new(left_h, right_h, middle_h).unwrap();
                (f.clone(), g.clone(), h)
            })
    })
}

/// Generate a valid `Span<char>` with small boundaries, wrapped in `DebugSpan`.
///
/// Both left and right use the *same* label vector so every `(i, j)` middle pair
/// satisfies the type-matching invariant `left[i] == right[j]`.
fn arb_span() -> impl Strategy<Value = DebugSpan> {
    (0_usize..=5,)
        .prop_flat_map(|(size,)| {
            // Pick one label — every boundary node gets the same label, so all
            // index pairs are valid.
            let label = prop::sample::select(vec!['a', 'b', 'c', 'd', 'e', 'f']);
            (Just(size), label, 0_usize..=5)
        })
        .prop_flat_map(|(size, label, n_pairs)| {
            let labels: Vec<char> = vec![label; size];
            let pairs = if size > 0 {
                prop::collection::vec((0..size, 0..size), n_pairs).boxed()
            } else {
                Just(vec![]).boxed()
            };
            (Just(labels.clone()), Just(labels), pairs)
        })
        .prop_map(|(left, right, middle)| DebugSpan(Span::new(left, right, middle).unwrap()))
}

// ---------------------------------------------------------------------------
// Cospan property tests
// ---------------------------------------------------------------------------

proptest! {
    /// Left identity: id_A ; f == f (up to connectivity equivalence).
    #[test]
    fn cospan_left_identity(f in arb_cospan()) {
        let id_a = Cospan::identity(&f.domain());
        let result = id_a.compose(&f).expect("id;f must compose");
        assert_connectivity_eq(&result, &f, "left identity")?;
    }

    /// Right identity: f ; id_B == f (up to connectivity equivalence).
    #[test]
    fn cospan_right_identity(f in arb_cospan()) {
        let id_b = Cospan::identity(&f.codomain());
        let result = f.compose(&id_b).expect("f;id must compose");
        assert_connectivity_eq(&result, &f, "right identity")?;
    }

    /// Associativity: (f;g);h and f;(g;h) have the same connectivity.
    #[test]
    fn cospan_associativity((f, g, h) in arb_three_composable_cospans()) {
        let fg = f.compose(&g).expect("f;g must compose");
        let gh = g.compose(&h).expect("g;h must compose");
        let fg_h = fg.compose(&h).expect("(f;g);h must compose");
        let f_gh = f.compose(&gh).expect("f;(g;h) must compose");
        assert_connectivity_eq(&fg_h, &f_gh, "associativity")?;
    }
}

// ---------------------------------------------------------------------------
// Span property tests
// ---------------------------------------------------------------------------

proptest! {
    /// Dagger involution: dagger(dagger(s)) == s.
    #[test]
    fn span_dagger_involution(ds in arb_span()) {
        let s = &ds.0;
        let dd = s.dagger().dagger();
        prop_assert!(
            span_eq(s, &dd),
            "dagger(dagger(s)) should equal s.\n  \
             left:   {:?} vs {:?}\n  right:  {:?} vs {:?}\n  middle: {:?} vs {:?}",
            s.left(), dd.left(),
            s.right(), dd.right(),
            s.middle_pairs(), dd.middle_pairs(),
        );
    }

    /// Dagger reverses domain and codomain.
    #[test]
    fn span_dagger_swaps_boundaries(ds in arb_span()) {
        let s = &ds.0;
        let d = s.dagger();
        prop_assert_eq!(s.domain(), d.codomain(), "dagger should swap domain to codomain");
        prop_assert_eq!(s.codomain(), d.domain(), "dagger should swap codomain to domain");
    }
}

// ---------------------------------------------------------------------------
// Monoidal property tests
// ---------------------------------------------------------------------------

proptest! {
    /// Monoidal product is associative: (f tensor g) tensor h == f tensor (g tensor h).
    #[test]
    fn cospan_monoidal_associativity(
        f in arb_cospan(),
        g in arb_cospan(),
        h in arb_cospan(),
    ) {
        let mut fg = f.clone();
        fg.monoidal(g.clone());
        let mut fg_h = fg;
        fg_h.monoidal(h.clone());

        let mut gh = g;
        gh.monoidal(h);
        let mut f_gh = f;
        f_gh.monoidal(gh);

        prop_assert_eq!(fg_h.domain(), f_gh.domain(), "monoidal assoc: domain");
        prop_assert_eq!(fg_h.codomain(), f_gh.codomain(), "monoidal assoc: codomain");
        prop_assert!(
            cospan_eq(&fg_h, &f_gh),
            "monoidal product is not associative:\n  \
             left:   {:?} vs {:?}\n  right:  {:?} vs {:?}\n  middle: {:?} vs {:?}",
            fg_h.left_to_middle(), f_gh.left_to_middle(),
            fg_h.right_to_middle(), f_gh.right_to_middle(),
            fg_h.middle(), f_gh.middle(),
        );
    }

    /// Monoidal right unit: f tensor empty == f.
    #[test]
    fn cospan_monoidal_right_unit(f in arb_cospan()) {
        let mut result = f.clone();
        result.monoidal(Cospan::empty());
        prop_assert!(cospan_eq(&f, &result), "f tensor empty should equal f");
    }

    /// Monoidal left unit: empty tensor f == f.
    #[test]
    fn cospan_monoidal_left_unit(f in arb_cospan()) {
        let mut result = Cospan::<char>::empty();
        result.monoidal(f.clone());
        prop_assert!(cospan_eq(&f, &result), "empty tensor f should equal f");
    }
}

// ---------------------------------------------------------------------------
// Debug wrapper for Rel (Rel doesn't derive Debug)
// ---------------------------------------------------------------------------

/// Wrapper that gives `Rel<char>` a `Debug` impl for proptest shrinking output.
///
/// Manual `Clone` because `Rel<char>` doesn't derive `Clone` — we reconstruct
/// from the underlying span's public accessors.
struct DebugRel(Rel<char>);

impl Clone for DebugRel {
    fn clone(&self) -> Self {
        let s = self.0.as_span();
        let span = Span::new(
            s.left().to_vec(),
            s.right().to_vec(),
            s.middle_pairs().to_vec(),
        )
        .unwrap();
        Self(Rel::new_unchecked(span))
    }
}

impl std::fmt::Debug for DebugRel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.0.as_span();
        f.debug_struct("Rel")
            .field("left", &s.left())
            .field("right", &s.right())
            .field("middle_pairs", &s.middle_pairs())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Rel strategy
// ---------------------------------------------------------------------------

/// Build the homogeneous `Rel<char>` on `n` elements whose pair set is the
/// subset of the row-major Cartesian product `0..n × 0..n` that `keep` selects
/// by flat index.
///
/// Uniform label: every node is `'a'`, so all `(i, j)` pairs are
/// type-compatible. Pairs are distinct by construction, so the span is jointly
/// injective and `new_unchecked` is safe.
///
/// The single construction shared by [`arb_homogeneous_rel`] and
/// [`arb_three_homogeneous_rels`] (through [`rel_from_bools`]) and by
/// `rel_predicates_decided_exhaustively_on_small_carriers` (through
/// [`rel_from_mask`]). Sharing one function is not on its own evidence that
/// the proptest corpus and the exhaustive corpus contain the same relation for
/// a given mask, and it says nothing about which pair a given bit denotes —
/// both the flat-index convention and the mask-vs-bool-vec parity are
/// asserted by `rel_from_selector_matches_its_definition`.
fn rel_from_selector(n: usize, keep: impl Fn(usize) -> bool) -> Rel<char> {
    let types: Vec<char> = vec!['a'; n];
    let pairs: Vec<(usize, usize)> = (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .enumerate()
        .filter(|&(bit, _)| keep(bit))
        .map(|(_, pair)| pair)
        .collect();
    Rel::new_unchecked(Span::new(types.clone(), types, pairs).unwrap())
}

/// The bool-vec view of [`rel_from_selector`] — the construction the proptest
/// strategies use, `bits[bit]` deciding pair `bit` of the row-major product.
///
/// A named function rather than a closure inside each strategy, so that
/// `rel_from_selector_matches_its_definition` exercises the strategies'
/// actual path instead of a re-spelling of it.
fn rel_from_bools(n: usize, bits: &[bool]) -> Rel<char> {
    rel_from_selector(n, |bit| bits[bit])
}

/// Generate a homogeneous `Rel<char>` on a small type set (1–4 elements).
///
/// All boundary nodes share a single label so every index pair is type-compatible.
/// Pairs are drawn from the full Cartesian product via a boolean mask, guaranteeing
/// joint injectivity (no duplicates), so `new_unchecked` is safe.
fn arb_homogeneous_rel() -> impl Strategy<Value = DebugRel> {
    (1_usize..=4,)
        .prop_flat_map(|(size,)| {
            let n_possible = size * size;
            // Boolean mask: one bit per potential (i, j) pair
            let mask = prop::collection::vec(prop::bool::ANY, n_possible);
            (Just(size), mask)
        })
        .prop_map(|(size, mask)| DebugRel(rel_from_bools(size, &mask)))
}

/// Generate three homogeneous `Rel<char>` instances on the *same* type set.
///
/// Picks one shared size, then generates three independent boolean masks.
fn arb_three_homogeneous_rels() -> impl Strategy<Value = (DebugRel, DebugRel, DebugRel)> {
    (1_usize..=4,)
        .prop_flat_map(|(size,)| {
            let n_possible = size * size;
            let mask = || prop::collection::vec(prop::bool::ANY, n_possible);
            (Just(size), mask(), mask(), mask())
        })
        .prop_map(|(size, m1, m2, m3)| {
            let build = |mask: &[bool]| DebugRel(rel_from_bools(size, mask));
            (build(&m1), build(&m2), build(&m3))
        })
}

/// Extract middle pairs of a `Rel` as a `HashSet` for order-independent comparison.
fn rel_pair_set(r: &Rel<char>) -> HashSet<(usize, usize)> {
    r.as_span().middle_pairs().iter().copied().collect()
}

// ---------------------------------------------------------------------------
// Relation algebra property tests
// ---------------------------------------------------------------------------

proptest! {
    /// Union is commutative: R ∪ S == S ∪ R.
    #[test]
    fn rel_union_commutative(dr in arb_homogeneous_rel(), ds in arb_homogeneous_rel()) {
        let r = &dr.0;
        let s = &ds.0;
        prop_assume!(r.domain() == s.domain());

        let rs = r.union(s).expect("R ∪ S");
        let sr = s.union(r).expect("S ∪ R");
        prop_assert_eq!(
            rel_pair_set(&rs),
            rel_pair_set(&sr),
            "union should be commutative"
        );
    }

    /// Complement is an involution: complement(complement(R)) == R.
    #[test]
    fn rel_complement_involution(dr in arb_homogeneous_rel()) {
        let r = &dr.0;
        let comp = r.complement().expect("complement(R)");
        let double = comp.complement().expect("complement(complement(R))");
        prop_assert_eq!(
            rel_pair_set(r),
            rel_pair_set(&double),
            "complement should be an involution"
        );
    }

    /// Intersection distributes over union: R ∩ (S ∪ T) == (R ∩ S) ∪ (R ∩ T).
    ///
    /// Uses a shared-size strategy to avoid excessive `prop_assume!` rejects when
    /// three independently generated relations rarely match in dimension.
    #[test]
    fn rel_intersection_distributes_over_union(
        (dr, ds, dt) in arb_three_homogeneous_rels(),
    ) {
        let r = &dr.0;
        let s = &ds.0;
        let t = &dt.0;

        let s_union_t = s.union(t).expect("S ∪ T");
        let lhs = r.intersection(&s_union_t).expect("R ∩ (S ∪ T)");

        let r_inter_s = r.intersection(s).expect("R ∩ S");
        let r_inter_t = r.intersection(t).expect("R ∩ T");
        let rhs = r_inter_s.union(&r_inter_t).expect("(R ∩ S) ∪ (R ∩ T)");

        prop_assert_eq!(
            rel_pair_set(&lhs),
            rel_pair_set(&rhs),
            "intersection should distribute over union"
        );
    }

    /// Every `Rel` order-theoretic predicate agrees with a *direct* decision on
    /// the relation's pair set.
    ///
    /// Replaces the former `rel_equivalence_iff_rst`, which compared
    /// `is_equivalence_rel()` against `is_reflexive() && is_symmetric() &&
    /// is_transitive()` — the production body's own definition on both sides of
    /// the assertion, so all three predicates could be rewritten to `return
    /// true` with this file still green (#287). The oracles below never call a
    /// `Rel` predicate: they quantify over `middle_pairs()` directly.
    ///
    /// # What this ranges over
    ///
    /// Homogeneous relations on a **1–4 element** carrier, every element
    /// labelled `'a'`, with pairs drawn from the full Cartesian product by an
    /// independent boolean mask — so any of the `2^(n²)` relations is
    /// reachable, but only a random sample of them is seen per run. It says
    /// nothing about heterogeneous relations, and nothing about carriers of 5
    /// or more. `rel_predicates_decided_exhaustively_on_small_carriers` closes
    /// the sampling gap for `n ≤ 3` by deciding *every* relation there, and
    /// `rel_composites_require_homogeneity` covers the non-homogeneous case.
    #[test]
    fn rel_predicates_match_a_direct_pair_set_oracle(dr in arb_homogeneous_rel()) {
        let r = &dr.0;
        let n = r.domain().len();
        let pairs = rel_pair_set(r);

        prop_assert_eq!(r.is_reflexive(), oracle_reflexive(n, &pairs), "is_reflexive");
        prop_assert_eq!(r.is_irreflexive(), oracle_irreflexive(n, &pairs), "is_irreflexive");
        prop_assert_eq!(r.is_symmetric(), oracle_symmetric(&pairs), "is_symmetric");
        prop_assert_eq!(r.is_antisymmetric(), oracle_antisymmetric(&pairs), "is_antisymmetric");
        prop_assert_eq!(r.is_transitive(), oracle_transitive(&pairs), "is_transitive");
        prop_assert_eq!(
            r.is_equivalence_rel(),
            oracle_reflexive(n, &pairs) && oracle_symmetric(&pairs) && oracle_transitive(&pairs),
            "is_equivalence_rel"
        );
        prop_assert_eq!(
            r.is_partial_order(),
            oracle_reflexive(n, &pairs)
                && oracle_antisymmetric(&pairs)
                && oracle_transitive(&pairs),
            "is_partial_order"
        );
    }
}

// ---------------------------------------------------------------------------
// Pair-set oracles for the `Rel` predicates
//
// Deliberately spelled out from the textbook definitions over `middle_pairs()`.
// None of them calls a `Rel` predicate, so a bug shared between a predicate and
// its oracle would have to be introduced here twice, in two different spellings
// — and the literature counts in
// `rel_predicates_decided_exhaustively_on_small_carriers` would still catch it,
// so long as the shared bug changed how many relations the predicate accepts.
// ---------------------------------------------------------------------------

/// `∀i < n. (i, i) ∈ R`.
fn oracle_reflexive(n: usize, pairs: &HashSet<(usize, usize)>) -> bool {
    (0..n).all(|i| pairs.contains(&(i, i)))
}

/// `∀i < n. (i, i) ∉ R`.
fn oracle_irreflexive(n: usize, pairs: &HashSet<(usize, usize)>) -> bool {
    (0..n).all(|i| !pairs.contains(&(i, i)))
}

/// `∀(i, j) ∈ R. (j, i) ∈ R`.
fn oracle_symmetric(pairs: &HashSet<(usize, usize)>) -> bool {
    pairs.iter().all(|&(i, j)| pairs.contains(&(j, i)))
}

/// `∀(i, j) ∈ R. (j, i) ∈ R → i = j`.
fn oracle_antisymmetric(pairs: &HashSet<(usize, usize)>) -> bool {
    pairs
        .iter()
        .all(|&(i, j)| i == j || !pairs.contains(&(j, i)))
}

/// `∀(i, j), (j, k) ∈ R. (i, k) ∈ R`.
fn oracle_transitive(pairs: &HashSet<(usize, usize)>) -> bool {
    pairs.iter().all(|&(i, j)| {
        pairs
            .iter()
            .filter(|&&(from, _)| from == j)
            .all(|&(_, k)| pairs.contains(&(i, k)))
    })
}

/// Build the homogeneous `Rel<char>` on `n` elements whose pair set is the
/// bitmask `mask` over the row-major Cartesian product `0..n × 0..n`.
///
/// A `u32`-bitmask view of [`rel_from_selector`], the same construction the
/// proptest strategies use.
fn rel_from_mask(n: usize, mask: u32) -> Rel<char> {
    rel_from_selector(n, |bit| mask & (1 << bit) != 0)
}

/// [`rel_from_selector`]'s row-major flat-index convention, decided against an
/// independent reference, and the mask-vs-bool-vec parity that #287's
/// docstring for the helper claimed.
///
/// #287's docstring said the proptest corpus and the exhaustive corpus
/// "contain the *same* relation for a given mask by construction" — a
/// structural claim no test asserted (this is the #287 follow-up). Both halves
/// are now checked for every mask over `n ∈ {1, 2, 3}` — 2 + 16 + 512 = 530
/// relations, exactly the corpus
/// `rel_predicates_decided_exhaustively_on_small_carriers` decides — in this
/// order:
///
/// 1. **parity** — the strategies' own path, [`rel_from_bools`]`(n, &bits)`
///    with `bits[k]` = bit `k` of `mask`, yields the same pair set as
///    `rel_from_mask(n, mask)`. This is the literal "same relation for a given
///    mask", the half that a divergence between the *two views* (not the shared
///    helper) would break — checked first, so such a divergence reports
///    through it instead of being masked by (2).
/// 2. **definition** — `rel_from_mask(n, mask)`'s middle pairs equal
///    `{ (i, j) : bit i*n + j of mask is set }`, built here from the row-major
///    convention itself. The reference never calls [`rel_from_selector`], so a
///    convention change inside the helper cannot move both sides together.
///
/// # What this ranges over
///
/// Every mask for `n ∈ {1, 2, 3}` exhaustively, and **nothing about `n ≥ 4`**
/// — where [`arb_homogeneous_rel`] does draw `n = 4`. `n = 4` is not
/// enumerated because it is `2^16` = 65 536 masks; the convention
/// `bit ↦ (bit / n, bit % n)` is `n`-independent, so a break in it is already
/// visible at `n = 2` (at `n = 1` a single bit has no order to get wrong).
/// Homogeneous, single-label relations only, like the corpora it pins.
///
/// Falsified three ways, each mutation measured and reverted; each assertion
/// has a mutation that reports through it:
///
/// - the Cartesian product built column-major inside [`rel_from_selector`]
///   (flat index `j*n + i`): both views drift together, so (1) stays green
///   and (2) reddens at `n = 2, mask = 0b10` — `{(1, 0)}` vs `{(0, 1)}`;
/// - [`rel_from_mask`]'s bit order reversed (`1 << (n*n - 1 - bit)`): (1)
///   reddens at `n = 2, mask = 0b1` — bool-vec `{(0, 0)}`, bitmask `{(1, 1)}`
///   — and (2) would too, behind it;
/// - [`rel_from_bools`]'s bit order reversed (the strategies' own path): (1)
///   reddens at `n = 2, mask = 0b1` — bool-vec `{(1, 1)}`, bitmask `{(0, 0)}`
///   — with (2) untouched, which is what makes (1) an independent check of
///   the corpus the strategies actually draw.
#[test]
fn rel_from_selector_matches_its_definition() {
    for n in [1_usize, 2, 3] {
        for mask in 0..(1_u32 << (n * n)) {
            // Independent reference: the row-major convention, spelled out.
            let mut reference: HashSet<(usize, usize)> = HashSet::new();
            for i in 0..n {
                for j in 0..n {
                    if mask & (1 << (i * n + j)) != 0 {
                        reference.insert((i, j));
                    }
                }
            }

            let from_mask = rel_pair_set(&rel_from_mask(n, mask));

            // (1) parity, first: the strategies' path vs the exhaustive test's.
            let bits: Vec<bool> = (0..n * n).map(|k| ((mask >> k) & 1) == 1).collect();
            let from_bools = rel_pair_set(&rel_from_bools(n, &bits));
            assert_eq!(
                from_bools, from_mask,
                "n = {n}, mask = {mask:#b}: the strategies' bool-vec path \
                 (rel_from_bools) built {from_bools:?}, the exhaustive test's \
                 u32-bitmask path (rel_from_mask) built {from_mask:?} — \
                 not the same relation for a given mask"
            );

            // (2) definition: the u32 path against the row-major convention.
            assert_eq!(
                from_mask, reference,
                "n = {n}, mask = {mask:#b}: rel_from_mask built {from_mask:?}, \
                 the row-major definition (bit i*n + j set ⟺ (i, j) ∈ R) \
                 gives {reference:?}"
            );
        }
    }
}

/// Every `Rel` predicate decided against its oracle on **every** relation over
/// carriers of size 1, 2, and 3 — all 530 of them — with the per-predicate
/// totals cross-checked against published enumerations.
///
/// Two independent claims, and the second is the one that survives an oracle
/// written by the same reasoning as the predicate:
///
/// 1. predicate == oracle, relation by relation (530 × 7 decisions);
/// 2. the number of relations each predicate accepts equals the count the
///    combinatorics literature gives for that `n` — reflexive/irreflexive
///    `2^(n²−n)`, symmetric `2^(n(n+1)/2)`, antisymmetric `2^n · 3^(n(n−1)/2)`,
///    transitive [OEIS A006905] `2, 13, 171`, equivalence = Bell(n) `1, 2, 5`,
///    partial order [OEIS A001035] `1, 3, 19`.
///
/// A bug present in both a predicate and its oracle passes (1) and fails (2)
/// whenever the shared bug changes the acceptance count. A shared bug that
/// accepted a *different* set of the same cardinality would pass both.
///
/// # What this ranges over
///
/// The *complete* relation space for `n ∈ {1, 2, 3}`, homogeneous and
/// single-label. Not `n = 0` (`arb_homogeneous_rel` excludes it too, and
/// `is_reflexive` on an empty carrier is vacuously true), not `n ≥ 4`, and not
/// heterogeneous relations.
///
/// [OEIS A006905]: https://oeis.org/A006905
/// [OEIS A001035]: https://oeis.org/A001035
#[test]
fn rel_predicates_decided_exhaustively_on_small_carriers() {
    // (n, [reflexive, irreflexive, symmetric, antisymmetric, transitive,
    //      equivalence, partial_order])
    let expected: [(usize, [usize; 7]); 3] = [
        (1, [1, 1, 2, 2, 2, 1, 1]),
        (2, [4, 4, 8, 12, 13, 2, 3]),
        (3, [64, 64, 64, 216, 171, 5, 19]),
    ];

    for (n, counts) in expected {
        let mut seen = [0_usize; 7];
        for mask in 0..(1_u32 << (n * n)) {
            let r = rel_from_mask(n, mask);
            let pairs = rel_pair_set(&r);

            let decided = [
                (r.is_reflexive(), oracle_reflexive(n, &pairs), "reflexive"),
                (
                    r.is_irreflexive(),
                    oracle_irreflexive(n, &pairs),
                    "irreflexive",
                ),
                (r.is_symmetric(), oracle_symmetric(&pairs), "symmetric"),
                (
                    r.is_antisymmetric(),
                    oracle_antisymmetric(&pairs),
                    "antisymmetric",
                ),
                (r.is_transitive(), oracle_transitive(&pairs), "transitive"),
                (
                    r.is_equivalence_rel(),
                    oracle_reflexive(n, &pairs)
                        && oracle_symmetric(&pairs)
                        && oracle_transitive(&pairs),
                    "equivalence",
                ),
                (
                    r.is_partial_order(),
                    oracle_reflexive(n, &pairs)
                        && oracle_antisymmetric(&pairs)
                        && oracle_transitive(&pairs),
                    "partial_order",
                ),
            ];

            for (slot, (production, oracle, name)) in decided.into_iter().enumerate() {
                assert_eq!(
                    production, oracle,
                    "n = {n}, mask = {mask:#b} (pairs {pairs:?}): is_{name} said \
                     {production}, the pair-set oracle said {oracle}"
                );
                if production {
                    seen[slot] += 1;
                }
            }
        }

        assert_eq!(
            seen, counts,
            "n = {n}: predicate acceptance counts {seen:?} do not match the \
             published enumeration {counts:?} (order: reflexive, irreflexive, \
             symmetric, antisymmetric, transitive, equivalence, partial_order)"
        );
    }
}

/// The two composite predicates screen for homogeneity *first*, and answer
/// `false` on a relation whose domain and codomain words differ.
///
/// Not decoration: every leaf predicate `unwrap()`s something that returns
/// `Err` on a boundary mismatch — `is_reflexive` and `is_symmetric` a
/// `subsumes`, `is_antisymmetric` an `intersection` and then a `subsumes`,
/// `is_transitive` the self-composition `self.0.compose(&self.0)`, which
/// `Span::composable` rejects before `subsumes` is reached. `is_reflexive` is
/// the first leaf in BOTH chains (`is_equivalence_rel`: reflexive, symmetric,
/// transitive; `is_partial_order`: reflexive, antisymmetric, transitive), so
/// dropping the `is_homogeneous() &&` guard from either composite turns this
/// case from `false` into a panic at that first `unwrap`. The exhaustive test
/// above cannot see it — every relation it builds is homogeneous by
/// construction.
#[test]
fn rel_composites_require_homogeneity() {
    // 1 → 1 with differently-labelled boundaries and no pairs: legal as a span
    // (the label check only fires on an actual middle pair), not homogeneous.
    let heterogeneous = Rel::new_unchecked(Span::new(vec!['a'], vec!['b'], vec![]).unwrap());
    assert!(!heterogeneous.is_homogeneous());
    assert!(!heterogeneous.is_equivalence_rel());
    assert!(!heterogeneous.is_partial_order());

    // The same shape with agreeing labels *is* homogeneous, and the empty
    // relation on one element is neither — so the `false`s above are not just
    // "empty relations are never equivalences".
    let homogeneous = Rel::new_unchecked(Span::new(vec!['a'], vec!['a'], vec![]).unwrap());
    assert!(homogeneous.is_homogeneous());
    assert!(!homogeneous.is_equivalence_rel());
    let diagonal = Rel::new_unchecked(Span::new(vec!['a'], vec!['a'], vec![(0, 0)]).unwrap());
    assert!(diagonal.is_equivalence_rel());
    assert!(diagonal.is_partial_order());
}

// ---------------------------------------------------------------------------
// Generator meta-test: the composability generators
// ---------------------------------------------------------------------------

/// The composability generators emit **label-aware, not identity-only** left
/// legs.
///
/// A generator is a claim, and the proptests that consume it cannot check that
/// claim: associativity of cospan composition holds over identity-injection
/// left legs exactly as well as over general ones, so `cospan_associativity`
/// stays green whichever the generator emits (#287 — the legs used to be
/// `(0..b_size).collect()` unconditionally, quotienting the property outside
/// the generated space). Both halves are therefore asserted here, where a drift
/// in [`arb_label_preserving_leg`] is actually visible:
///
/// - **label-aware** — every sampled pair/triple composes. A leg that picked
///   apex vertices without regard to labels would land a `'b'` slot on an `'a'`
///   vertex and `composable` would reject it.
/// - **non-identity** — the number of samples whose left leg differs from
///   `i ↦ i` is counted, reported, and asserted positive.
///
/// # What this ranges over
///
/// 256 deterministic samples per generator. The count assertion is a claim
/// about *those samples*, not that every draw is non-identity: an apex whose
/// only matching vertex is `i` leaves the leg nowhere else to go, and the
/// identity is a legitimate draw whenever the apex does offer a choice.
///
/// Measured at the commit that introduced this test: **139 of 256** for `g`,
/// **138 of 256** for `h`. Only `> 0` is asserted (a proptest RNG change would
/// move the exact figures), but a run reporting a number far from those is
/// drift in the generator, not noise.
#[test]
fn composability_generators_emit_label_aware_non_identity_left_legs() {
    const SAMPLES: usize = 256;

    let is_identity_injection = |leg: &[usize]| leg.iter().enumerate().all(|(i, &m)| i == m);

    let pairs = sample_n(&arb_composable_cospans(), SAMPLES);
    let mut non_identity_g = 0_usize;
    for (f, g) in &pairs {
        f.compose(g).unwrap_or_else(|e| {
            panic!(
                "generated pair does not compose, so the left leg is not label-aware: {e}\n  \
                 f.codomain() = {:?}\n  g.domain()   = {:?}\n  g.left       = {:?}\n  \
                 g.middle     = {:?}",
                f.codomain(),
                g.domain(),
                g.left_to_middle(),
                g.middle(),
            )
        });
        if !is_identity_injection(g.left_to_middle()) {
            non_identity_g += 1;
        }
    }
    assert!(
        non_identity_g > 0,
        "arb_composable_cospans: {non_identity_g} of {SAMPLES} samples had a \
         non-identity left leg on g — the generator is back to the identity \
         injection and `cospan_associativity` is quotiented outside its own space"
    );

    let triples = sample_n(&arb_three_composable_cospans(), SAMPLES);
    let mut non_identity_h = 0_usize;
    for (f, g, h) in &triples {
        f.compose(g).expect("f;g must compose");
        g.compose(h).unwrap_or_else(|e| {
            panic!(
                "generated triple does not compose at g;h: {e}\n  \
                 g.codomain() = {:?}\n  h.domain()   = {:?}",
                g.codomain(),
                h.domain(),
            )
        });
        if !is_identity_injection(h.left_to_middle()) {
            non_identity_h += 1;
        }
    }
    assert!(
        non_identity_h > 0,
        "arb_three_composable_cospans: {non_identity_h} of {SAMPLES} samples had \
         a non-identity left leg on h"
    );
}

// ---------------------------------------------------------------------------
// CospanCanon: canonical form decides apex isomorphism
// ---------------------------------------------------------------------------

/// A small `Cospan<char>`: apex of 0–5 vertices, boundaries of 0–3 wires,
/// labels drawn from `'a'..'c'` so label ties (and hence non-trivial apex
/// symmetries) are common.
///
/// Kept small deliberately: [`exists_apex_iso`] brute-forces `S_apex`, so the
/// apex bound is what keeps `5! = 120` permutations per case affordable.
fn arb_small_cospan() -> impl Strategy<Value = Cospan<char>> {
    (0_usize..=5)
        .prop_flat_map(|apex_size| {
            let max_boundary: usize = if apex_size == 0 { 0 } else { 3 };
            (Just(apex_size), 0..=max_boundary, 0..=max_boundary)
        })
        .prop_flat_map(|(apex_size, domain_size, codomain_size)| {
            let labels =
                prop::collection::vec(prop::sample::select(vec!['a', 'b', 'c']), apex_size);
            // `max(1)` only to keep the range non-empty; both counts are 0 when
            // the apex is, so no index is ever drawn from it.
            let bound = apex_size.max(1);
            (
                prop::collection::vec(0..bound, domain_size),
                prop::collection::vec(0..bound, codomain_size),
                labels,
            )
        })
        .prop_map(|(left, right, middle)| Cospan::new(left, right, middle).unwrap())
}

/// A cospan together with a perturbation of it: the apex relabelled by a random
/// permutation, then — about half the time — a single leg entry rewired to a
/// random apex vertex.
///
/// The permutation arm alone would only ever produce isomorphic pairs, making
/// the `iff` in `canonical_form_decides_apex_isomorphism` half vacuous; the
/// rewire arm supplies the other half. It is *not* filtered to non-isomorphic
/// rewires, because some single rewires happen to be apex permutations in
/// disguise (see `a_single_rewire_changes_the_form_unless_it_is_a_relabelling`)
/// — the brute-force oracle decides which, so both outcomes are legal here.
fn arb_cospan_and_perturbation() -> impl Strategy<Value = (Cospan<char>, Cospan<char>)> {
    arb_small_cospan()
        .prop_flat_map(|a| {
            let perms = all_perm_indices(a.middle().len());
            (
                Just(a),
                prop::sample::select(perms),
                prop::option::of((any::<bool>(), any::<Index>(), any::<Index>())),
            )
        })
        .prop_map(|(a, perm, rewire)| {
            let mut left: Vec<usize> = a.left_to_middle().iter().map(|&m| perm[m]).collect();
            let mut right: Vec<usize> = a.right_to_middle().iter().map(|&m| perm[m]).collect();
            let mut middle: Vec<char> = a.middle().to_vec();
            for (m, &label) in a.middle().iter().enumerate() {
                middle[perm[m]] = label;
            }

            if let Some((on_right_leg, slot, target)) = rewire {
                let apex_size = middle.len();
                let leg = if on_right_leg { &mut right } else { &mut left };
                if !leg.is_empty() {
                    let position = slot.index(leg.len());
                    // Move the wire *somewhere else*: a rewire onto the vertex
                    // it already sits on is a no-op, and no-ops only pad the
                    // isomorphic side of the corpus. A non-empty leg implies a
                    // non-empty apex, and `elsewhere` is empty exactly when the
                    // apex has a single vertex — then there is nowhere to go.
                    let elsewhere: Vec<usize> =
                        (0..apex_size).filter(|&v| v != leg[position]).collect();
                    if !elsewhere.is_empty() {
                        leg[position] = elsewhere[target.index(elsewhere.len())];
                    }
                }
            }

            let b = Cospan::new(left, right, middle).unwrap();
            (a, b)
        })
}

/// Apex isomorphism decided the long way: brute force over `S_apex`, straight
/// from the definition (F&S 2019 §3 — a bijection of apexes commuting with both
/// legs, boundaries fixed).
///
/// This is the *definition*, not a second copy of `canonical_form`'s
/// sort-the-signatures algorithm — which is what makes it usable as that
/// method's oracle.
fn exists_apex_iso(a: &Cospan<char>, b: &Cospan<char>) -> bool {
    if a.middle().len() != b.middle().len()
        || a.left_to_middle().len() != b.left_to_middle().len()
        || a.right_to_middle().len() != b.right_to_middle().len()
    {
        return false;
    }
    all_perm_indices(a.middle().len()).into_iter().any(|perm| {
        a.middle()
            .iter()
            .enumerate()
            .all(|(m, &label)| b.middle()[perm[m]] == label)
            && a.left_to_middle()
                .iter()
                .zip(b.left_to_middle())
                .all(|(&m, &image)| perm[m] == image)
            && a.right_to_middle()
                .iter()
                .zip(b.right_to_middle())
                .all(|(&m, &image)| perm[m] == image)
    })
}

proptest! {
    /// `canonical_form()` equality **is** apex isomorphism — both directions,
    /// against a brute-force search over `S_apex`.
    ///
    /// The module's one hand-written iso⇒equal fixture covers a single apex
    /// swap on `id(2)` (#287); this ranges over random permutations of apexes
    /// up to 5 vertices, and over single-leg rewires, which supply the
    /// non-isomorphic side that a permutation-only generator could never reach.
    ///
    /// # What this ranges over
    ///
    /// Apexes of 0–5 vertices, boundaries of 0–3 wires, three labels — pairs
    /// related by an apex permutation and at most **one** rewired leg entry. It
    /// does not range over pairs that differ in more than one leg entry; over
    /// pairs with different boundary sizes (those are separated by `dom_len` /
    /// `cod_len` and pinned in the module's own tests); over pairs with
    /// different apex sizes or different apex label multisets (both arms
    /// preserve `middle.len()` and the label multiset); or over larger apexes,
    /// where the brute-force oracle stops being affordable.
    ///
    /// Consequence of the equal-size (apex and both boundaries),
    /// equal-label-multiset corpus, by construction: the oracle's size guard —
    /// three length checks, one early `return false` — is dead here, and equal
    /// non-bubble classes force equal
    /// bubble-label multisets (a bubble class is its label and two empty
    /// preimages) and hence equal forms, so a `canonical_form` that dropped
    /// bubble classes would pass this test on every seed. Confirmed on the
    /// meta-test's 256 deterministic samples: `apex_len` differs in 0 of 256;
    /// `scalar_count` differs in 33 of 256, all non-isomorphic (a rewire that
    /// empties a vertex frees a bubble; one that lands on a bubble fills it);
    /// the non-bubble classes differ in all 64 of the 64 non-isomorphic pairs;
    /// and all 18 tests in this file stay green under that mutant. The mutant
    /// is caught by `cospan_canon.rs`'s own scalar pins and the bubble ledgers
    /// in `cospan_algebra`, `frobenius` and `tests/frobenius_axioms.rs` — 11
    /// tests crate-wide — not here.
    #[test]
    fn canonical_form_decides_apex_isomorphism((a, b) in arb_cospan_and_perturbation()) {
        let by_canonical_form = a.canonical_form() == b.canonical_form();
        let by_definition = exists_apex_iso(&a, &b);
        prop_assert_eq!(
            by_canonical_form, by_definition,
            "canonical_form said {}, the brute-force apex search said {}\n  \
             a = (left {:?}, right {:?}, middle {:?})\n  \
             b = (left {:?}, right {:?}, middle {:?})",
            by_canonical_form, by_definition,
            a.left_to_middle(), a.right_to_middle(), a.middle(),
            b.left_to_middle(), b.right_to_middle(), b.middle(),
        );
    }
}

/// The perturbation generator reaches **both** sides of the `iff` above.
///
/// Without this, dropping the rewire arm from [`arb_cospan_and_perturbation`]
/// would leave `canonical_form_decides_apex_isomorphism` green on a corpus in
/// which `by_definition` is `true` every single time — the exact shape of
/// vacuity #287 was filed for. The measured split is reported either way.
///
/// Measured at the commit that introduced this test: **192 isomorphic, 64
/// non-isomorphic** of 256. Only "both sides non-empty" is asserted; the
/// figures are here so a later run can tell generator drift from RNG noise.
#[test]
fn perturbation_generator_reaches_isomorphic_and_non_isomorphic_pairs() {
    const SAMPLES: usize = 256;
    let samples = sample_n(&arb_cospan_and_perturbation(), SAMPLES);
    let isomorphic = samples
        .iter()
        .filter(|(a, b)| exists_apex_iso(a, b))
        .count();
    let non_isomorphic = SAMPLES - isomorphic;
    assert!(
        isomorphic > 0 && non_isomorphic > 0,
        "perturbation split over {SAMPLES} samples: {isomorphic} isomorphic, \
         {non_isomorphic} non-isomorphic — the `iff` in \
         canonical_form_decides_apex_isomorphism is one-sided on this corpus"
    );
}

/// The single-rewire negative, and the case that keeps it honest.
///
/// Rewiring one leg entry generally changes the canonical form — but not
/// always, and asserting that it always does would be wrong rather than merely
/// narrow: a rewire that moves a wire onto an equally-labelled vertex can be
/// exactly an apex transposition, which the form is *designed* to forget. Both
/// cases are pinned, and the brute-force oracle agrees with the form on each.
#[test]
fn a_single_rewire_changes_the_form_unless_it_is_a_relabelling() {
    // ---- the negative: one rewire, genuinely different wiring ----
    // id(2), then domain slot 1 rewired from vertex 1 to vertex 0. Both apex
    // labels agree, so the change is purely in the wiring — and no permutation
    // undoes it, because it merges two domain wires onto one vertex where id(2)
    // gives each its own.
    let identity_two = Cospan::<char>::new(vec![0, 1], vec![0, 1], vec!['a', 'a']).unwrap();
    let merged = Cospan::<char>::new(vec![0, 0], vec![0, 1], vec!['a', 'a']).unwrap();
    assert!(
        !exists_apex_iso(&identity_two, &merged),
        "fixture is not the negative case it is named for"
    );
    assert_ne!(
        identity_two.canonical_form(),
        merged.canonical_form(),
        "a single rewire that merges both domain wires must change the form"
    );
    // Visible in the exposed representation: one class now holds both domain
    // indices, where id(2) splits them one apiece.
    assert_eq!(
        merged.canonical_form().classes()[1].dom_preimage(),
        [0_usize, 1].as_slice()
    );

    // ---- and the case where a single rewire is an apex transposition ----
    // 1 → apex{'a', 'a'} ← 0: which of the two identically-labelled vertices the
    // wire lands on is exactly the labelling the canonical form discards.
    let on_first = Cospan::<char>::new(vec![0], vec![], vec!['a', 'a']).unwrap();
    let on_second = Cospan::<char>::new(vec![1], vec![], vec!['a', 'a']).unwrap();
    assert!(exists_apex_iso(&on_first, &on_second));
    assert_eq!(on_first.canonical_form(), on_second.canonical_form());
    assert!(!on_first.structurally_equal(&on_second));
}
