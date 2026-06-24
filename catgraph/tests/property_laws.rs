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
use proptest::prelude::*;
use std::collections::HashSet;

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
        .prop_map(|(left, right, middle)| Cospan::new(left, right, middle))
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
                let left_g: Vec<usize> = (0..b_size).collect();
                let right_g = if middle_g_size > 0 {
                    prop::collection::vec(0..middle_g_size, codomain_g_size).boxed()
                } else {
                    Just(vec![]).boxed()
                };
                (Just(codomain), Just(left_g), right_g, extra_labels)
            })
            .prop_map(move |(codomain, left_g, right_g, extra_labels)| {
                let mut middle_g: Vec<char> = codomain;
                middle_g.extend(extra_labels);
                let g = Cospan::new(left_g, right_g, middle_g);
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
                    let left_h: Vec<usize> = (0..c_size).collect();
                    let right_h = if middle_h_size > 0 {
                        prop::collection::vec(0..middle_h_size, codomain_h_size).boxed()
                    } else {
                        Just(vec![]).boxed()
                    };
                    (Just(codomain_g), Just(left_h), right_h, extra_labels)
                },
            )
            .prop_map(move |(codomain_g, left_h, right_h, extra_labels)| {
                let mut middle_h: Vec<char> = codomain_g;
                middle_h.extend(extra_labels);
                let h = Cospan::new(left_h, right_h, middle_h);
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
        .prop_map(|(left, right, middle)| DebugSpan(Span::new(left, right, middle)))
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
        );
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
        .prop_map(|(size, mask)| {
            // Uniform label: every node is 'a', so all (i, j) pairs are type-compatible.
            let types: Vec<char> = vec!['a'; size];
            let pairs: Vec<(usize, usize)> = (0..size)
                .flat_map(|i| (0..size).map(move |j| (i, j)))
                .zip(mask.iter())
                .filter(|&(_, keep)| *keep)
                .map(|(pair, _)| pair)
                .collect();
            let span = Span::new(types.clone(), types, pairs);
            DebugRel(Rel::new_unchecked(span))
        })
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
            let build = |mask: &[bool]| {
                let types: Vec<char> = vec!['a'; size];
                let pairs: Vec<(usize, usize)> = (0..size)
                    .flat_map(|i| (0..size).map(move |j| (i, j)))
                    .zip(mask.iter())
                    .filter(|&(_, keep)| *keep)
                    .map(|(pair, _)| pair)
                    .collect();
                DebugRel(Rel::new_unchecked(Span::new(types.clone(), types, pairs)))
            };
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

    /// Equivalence relation iff reflexive, symmetric, and transitive.
    #[test]
    fn rel_equivalence_iff_rst(dr in arb_homogeneous_rel()) {
        let r = &dr.0;
        let is_equiv = r.is_equivalence_rel();
        let is_rst = r.is_reflexive() && r.is_symmetric() && r.is_transitive();
        prop_assert_eq!(
            is_equiv, is_rst,
            "is_equivalence_rel should equal is_reflexive && is_symmetric && is_transitive"
        );
    }
}
