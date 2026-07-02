//! Semantic text comparison and clustering over the BTV 2021 Yoneda embedding.
//!
//! Bradley–Terilla–Vlassopoulos 2021 (*An enriched category theory of language*,
//! arXiv:2106.07890) models each text `x` as its representable copresheaf
//! `L(x, −) = π(− | x)` — the meaning of `x` *is* its distribution of
//! extension-probabilities over every other text ("meaning is context-change
//! potential"). This module is the *consumer* layer over that embedding
//! ([`crate::yoneda`]): it ranks and groups whole texts by their meanings,
//! reusing the pinned BTV internal hom / distance without re-deriving them.
//!
//! # What lives here
//!
//! [`k_nearest_from`] / [`k_nearest_to`] and [`cluster_semantic_sym`] (see each
//! item's docs); the batch embedding
//! [`LmCategory::yoneda_all`](crate::lm_category::LmCategory::yoneda_all) lives
//! beside [`LmCategory::yoneda`](crate::lm_category::LmCategory::yoneda) in
//! [`crate::yoneda`].
//!
//! # Asymmetry (BTV 2021 §5)
//!
//! The BTV semantic hom `L̂(f, g) = inf_c min{1, g(c) / f(c)}` (Lemma 2, Eq 11)
//! and its distance `d̂(f, g) = −ln L̂(f, g)` (§5) are **asymmetric** — BTV keep
//! the Lawvere generalized metric ("symmetry is not required"). Concretely
//! `d̂(a, b) < ∞` iff `support(a) ⊆ support(b)` (every extension of `a` is also
//! an extension of `b`), so `d̂(a, b) ≠ d̂(b, a)` in general and "the meanings a
//! query most extends into" (`k_nearest_from`) is a different set from "the
//! meanings that most extend the query" (`k_nearest_to`). Both directions are
//! therefore exposed; neither is derivable from the other. See
//! [`crate::semantic_distance`] for the pinned definition.

use std::collections::HashMap;

use crate::weighted_cospan::NodeId;
use crate::yoneda::{Copresheaf, semantic_distance};

/// The `k` candidates `c` minimizing `d̂(query, c)`, ascending — the meanings the
/// query most **extends into** (candidates whose support contains the query's).
///
/// Distance is the BTV-canonical asymmetric [`semantic_distance`] in the
/// `(query, candidate)` direction. This is *not* interchangeable with
/// [`k_nearest_to`]: the hom is asymmetric (BTV 2021 §5), so "the query's
/// nearest" and "nearest to the query" rank different candidates. See the
/// module docs.
///
/// The returned [`NodeId`] is each candidate's [`Copresheaf::base`]. `query`
/// and `candidates` must all come from the **same**
/// [`LmCategory`](crate::lm_category::LmCategory) — a
/// `base()` is only an identity within one category's object indexing.
/// Candidates with the same `base()` as `query` are skipped (self-distance is
/// a trivial `0`). `f64::INFINITY` distances are legitimate
/// (mutually-unreachable meanings) and sort last via [`f64::total_cmp`]; ties
/// break by `NodeId` ascending for determinism. `k` beyond the candidate count
/// returns all.
///
/// # Panics
///
/// Propagates [`semantic_hom`](crate::semantic_hom)'s panic if `query` and any
/// candidate come from different [`LmCategory`](crate::lm_category::LmCategory)
/// instances (mismatched object counts).
#[must_use]
pub fn k_nearest_from(
    query: &Copresheaf,
    candidates: &[Copresheaf],
    k: usize,
) -> Vec<(NodeId, f64)> {
    // Direction: d̂(query, candidate).
    k_nearest_by(query, candidates, k, semantic_distance)
}

/// The `k` candidates `c` minimizing `d̂(c, query)`, ascending — the meanings the
/// query most **extends** (candidates whose support is contained in the query's).
///
/// Distance is the BTV-canonical asymmetric [`semantic_distance`] in the
/// `(candidate, query)` direction — the mirror of [`k_nearest_from`]. Because
/// the hom is asymmetric (BTV 2021 §5) the two rankings genuinely differ; both
/// are provided. Return / tie-break / self-exclusion / `∞` / `k`-overflow /
/// panic semantics are identical to [`k_nearest_from`].
#[must_use]
pub fn k_nearest_to(query: &Copresheaf, candidates: &[Copresheaf], k: usize) -> Vec<(NodeId, f64)> {
    // Direction: d̂(candidate, query).
    k_nearest_by(query, candidates, k, |q, c| semantic_distance(c, q))
}

/// Shared ranking core for [`k_nearest_from`] / [`k_nearest_to`]. `dist(query,
/// candidate)` selects the distance direction; everything else (self-exclusion,
/// `total_cmp` + `NodeId` tie-break, `k`-truncation) is common.
fn k_nearest_by(
    query: &Copresheaf,
    candidates: &[Copresheaf],
    k: usize,
    dist: impl Fn(&Copresheaf, &Copresheaf) -> f64,
) -> Vec<(NodeId, f64)> {
    let mut scored: Vec<(NodeId, f64)> = candidates
        .iter()
        .filter(|c| c.base() != query.base()) // drop the query itself (self-distance 0 is noise)
        .map(|c| (c.base(), dist(query, c)))
        .collect();
    // `total_cmp` gives a total order over f64 including +∞ (which sorts last,
    // as intended for mutually-unreachable meanings); NodeId breaks ties.
    scored.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    scored.truncate(k); // k > len is a no-op ⇒ returns all
    scored
}

/// Single-linkage threshold clustering of meanings: the connected components of
/// the graph with an edge between `a, b` wherever `semantic_distance_sym(a, b)
/// <= epsilon`.
///
/// **Symmetric convenience, not the BTV enriched hom.** The edge test is
/// equivalent to thresholding [`semantic_distance_sym`](crate::semantic_distance_sym)
/// — the labelled, *non-canonical* symmetric variant `max(d̂(a, b), d̂(b, a))` —
/// evaluated direction-by-direction with a short-circuit. BTV 2021 §5 keeps the
/// asymmetric Lawvere generalized metric; symmetrizing is a caller convenience
/// for clustering only (see the [`crate::yoneda`] module docs, and
/// [`k_nearest_from`] / [`k_nearest_to`] for the paper-faithful asymmetric
/// rankings).
///
/// **`∞` semantics.** `semantic_distance_sym(a, b) < ∞` requires `support(a) =
/// support(b)` — the meanings must be **mutually reachable** (each an extension
/// of the other; e.g. interchangeable / synonymous texts). Meanings that are
/// not mutually reachable sit at distance `∞` and therefore never merge at any
/// finite `epsilon`. This clustering does *not* require the acyclic poset
/// hypothesis that [`magnitude`](crate::lm_category::LmCategory::magnitude)
/// needs — see [`enriched_space`](crate::lm_category::LmCategory::enriched_space)'s
/// documented best-path semantics on mutually-reachable tables.
///
/// # Determinism and cost
///
/// Each cluster's members are the candidates' [`Copresheaf::base`] ids, sorted
/// ascending; clusters are ordered by their smallest member id. O(n²) pairwise
/// distance evaluations, each itself an O(n) inf-fold over the contexts —
/// Θ(n³) total, sized for small BYO-LM tables. Plain union-find, no new
/// dependencies.
///
/// # Panics
///
/// Propagates [`semantic_hom`](crate::semantic_hom)'s panic if the copresheaves
/// come from different [`LmCategory`](crate::lm_category::LmCategory) instances
/// (mismatched object counts).
#[must_use]
pub fn cluster_semantic_sym(copresheaves: &[Copresheaf], epsilon: f64) -> Vec<Vec<NodeId>> {
    let n = copresheaves.len();
    let mut parent: Vec<usize> = (0..n).collect();

    // Union every pair within the epsilon threshold (upper triangle; the metric
    // is symmetric here and self-distance is 0, so i < j suffices).
    // `max(d1, d2) <= ε  ⟺  d1 <= ε ∧ d2 <= ε` (distances are in [0, ∞], never
    // NaN), so evaluate the forward direction first and skip the reverse fold
    // whenever it already disqualifies the merge — support-disjoint pairs (∞)
    // never pay for the second direction.
    for i in 0..n {
        for j in (i + 1)..n {
            if semantic_distance(&copresheaves[i], &copresheaves[j]) > epsilon {
                continue;
            }
            if semantic_distance(&copresheaves[j], &copresheaves[i]) <= epsilon {
                let (ri, rj) = (uf_find(&mut parent, i), uf_find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }

    // Bucket indices by representative, labelling members with base() NodeIds.
    let mut buckets: HashMap<usize, Vec<NodeId>> = HashMap::new();
    for (i, cp) in copresheaves.iter().enumerate() {
        let root = uf_find(&mut parent, i);
        buckets.entry(root).or_default().push(cp.base());
    }

    let mut clusters: Vec<Vec<NodeId>> = buckets.into_values().collect();
    for cluster in &mut clusters {
        cluster.sort_unstable(); // members ascending
    }
    // Clusters ordered by smallest member (each cluster is non-empty and sorted).
    clusters.sort_unstable_by_key(|c| c[0]);
    clusters
}

/// Iterative union-find `find` with path halving.
fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lm_category::LmCategory;

    const EPS: f64 = 1e-9;

    /// Acyclic toy LM `root → {a (0.6), b (0.4)}`, `a → leaf (1.0)`.
    ///
    /// Copresheaf rows (index root=0, a=1, b=2, leaf=3), `π = exp(−d)`:
    ///   root = [1, 0.6, 0.4, 0.6]   a = [0, 1, 0, 1]   b = [0, 0, 1, 0]   leaf = [0, 0, 0, 1]
    fn dag() -> LmCategory {
        let mut m = LmCategory::new(vec!["root".into(), "a".into(), "b".into(), "leaf".into()]);
        m.add_transition("root", "a", 0.6).unwrap();
        m.add_transition("root", "b", 0.4).unwrap();
        m.add_transition("a", "leaf", 1.0).unwrap();
        m.mark_terminating("b");
        m.mark_terminating("leaf");
        m
    }

    /// Two synonym pairs, each mutually reachable and sharing one continuation:
    ///   g1 ↔ g2, both → gc ;   h1 ↔ h2, both → hc.
    /// Mutual reachability makes support(g1) = support(g2) (= {g1, g2, gc}), so
    /// the symmetric distance is finite *within* a pair and `∞` across pairs /
    /// to a continuation. (Cyclic on purpose — the semantic layer does not need
    /// the acyclic poset hypothesis; only magnitude() does.)
    fn synonyms() -> LmCategory {
        let mut m = LmCategory::new(vec![
            "g1".into(),
            "g2".into(),
            "gc".into(),
            "h1".into(),
            "h2".into(),
            "hc".into(),
        ]);
        m.add_transition("g1", "g2", 0.5).unwrap();
        m.add_transition("g1", "gc", 0.5).unwrap();
        m.add_transition("g2", "g1", 0.5).unwrap();
        m.add_transition("g2", "gc", 0.5).unwrap();
        m.add_transition("h1", "h2", 0.5).unwrap();
        m.add_transition("h1", "hc", 0.5).unwrap();
        m.add_transition("h2", "h1", 0.5).unwrap();
        m.add_transition("h2", "hc", 0.5).unwrap();
        m.mark_terminating("gc");
        m.mark_terminating("hc");
        m
    }

    #[test]
    fn yoneda_all_agrees_with_per_object_yoneda() {
        // Cover both the acyclic and the mutually-reachable substrates.
        for m in [dag(), synonyms()] {
            let all = m.yoneda_all().unwrap();
            assert_eq!(all.len(), m.objects().len());
            for (i, name) in m.objects().iter().enumerate() {
                let one = m.yoneda(name).unwrap();
                assert_eq!(
                    all[i], one,
                    "row {i} ({name:?}) disagrees with yoneda({name:?})"
                );
            }
        }
    }

    #[test]
    fn k_nearest_both_directions_differ() {
        let m = dag();
        let all = m.yoneda_all().unwrap();
        let root = &all[0];

        // to-direction: d̂(c, root). support(c) ⊆ support(root)=all for every c,
        // so all finite. d̂(a,root)=d̂(leaf,root)=−ln0.6 (tie ⇒ NodeId order a<leaf);
        // d̂(b,root)=−ln0.4.
        let to = k_nearest_to(root, &all, 3);
        assert_eq!(
            to.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
            vec![1, 3, 2]
        );
        assert!((to[0].1 - (-0.6_f64.ln())).abs() < EPS);
        assert!((to[1].1 - (-0.6_f64.ln())).abs() < EPS);
        assert!((to[2].1 - (-0.4_f64.ln())).abs() < EPS);

        // from-direction: d̂(root, c). No candidate's support contains root's
        // (all of root), so every distance is ∞ ⇒ NodeId order 1,2,3.
        let from = k_nearest_from(root, &all, 3);
        assert_eq!(
            from.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(from.iter().all(|&(_, d)| d.is_infinite()));

        // The two directions genuinely disagree (order and finiteness).
        assert_ne!(
            to.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
            from.iter().map(|&(id, _)| id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn k_nearest_infinite_ranked_last_and_self_excluded() {
        let m = dag();
        let all = m.yoneda_all().unwrap();
        let a = &all[1];

        // to-direction from `a`: d̂(leaf, a)=0 (leaf extends a); d̂(root,a)=d̂(b,a)=∞.
        let to = k_nearest_to(a, &all, 10);
        assert_eq!(to.len(), 3); // self excluded ⇒ 3 of 4
        assert!(to.iter().all(|&(id, _)| id != a.base()));
        assert_eq!(to[0].0, 3); // leaf first, finite
        assert_eq!(to[0].1, 0.0);
        assert!(to[1].1.is_infinite() && to[2].1.is_infinite()); // ∞ ranked last
        assert_eq!(vec![to[1].0, to[2].0], vec![0, 2]); // NodeId tie-break among ∞
    }

    #[test]
    fn k_nearest_k_zero_and_overflow() {
        let m = dag();
        let all = m.yoneda_all().unwrap();
        let root = &all[0];
        assert!(k_nearest_from(root, &all, 0).is_empty()); // k = 0
        assert_eq!(k_nearest_from(root, &all, 999).len(), 3); // k > len ⇒ all (minus self)
    }

    #[test]
    fn cluster_fine_epsilon_gives_singletons() {
        let m = synonyms();
        let all = m.yoneda_all().unwrap();
        // Within-pair distance is −ln0.5 ≈ 0.693; ε = 0.5 is below it ⇒ no edges.
        let clusters = cluster_semantic_sym(&all, 0.5);
        assert_eq!(
            clusters,
            vec![vec![0], vec![1], vec![2], vec![3], vec![4], vec![5]]
        );
    }

    #[test]
    fn cluster_coarse_epsilon_merges_synonym_pairs_and_infinity_never_merges() {
        let m = synonyms();
        let all = m.yoneda_all().unwrap();
        // ε = 0.7 > 0.693 ⇒ each synonym pair merges; continuations stay
        // singletons. The same partition must hold at an enormous ε: cross-pair
        // meanings are support-disjoint (⇒ ∞) and never merge at any finite ε.
        for eps in [0.7, 1e6] {
            let clusters = cluster_semantic_sym(&all, eps);
            assert_eq!(
                clusters,
                vec![vec![0, 1], vec![2], vec![3, 4], vec![5]],
                "unexpected partition at ε = {eps}"
            );
        }
    }

    #[test]
    fn cluster_is_deterministic_and_a_partition() {
        let m = synonyms();
        let all = m.yoneda_all().unwrap();
        let n = all.len();
        for &eps in &[0.5_f64, 0.7, 1e6] {
            let a = cluster_semantic_sym(&all, eps);
            let b = cluster_semantic_sym(&all, eps);
            assert_eq!(a, b, "clustering not deterministic at ε = {eps}");

            // Partition: every NodeId 0..n appears exactly once, members sorted,
            // clusters ordered by smallest member.
            let mut flat: Vec<NodeId> = a.iter().flatten().copied().collect();
            flat.sort_unstable();
            assert_eq!(
                flat,
                (0..n).collect::<Vec<_>>(),
                "not a partition at ε = {eps}"
            );
            for cluster in &a {
                let mut sorted = cluster.clone();
                sorted.sort_unstable();
                assert_eq!(*cluster, sorted, "members not ascending at ε = {eps}");
            }
            let firsts: Vec<NodeId> = a.iter().map(|c| c[0]).collect();
            let mut firsts_sorted = firsts.clone();
            firsts_sorted.sort_unstable();
            assert_eq!(
                firsts, firsts_sorted,
                "clusters not ordered by min member at ε = {eps}"
            );
        }
    }
}
