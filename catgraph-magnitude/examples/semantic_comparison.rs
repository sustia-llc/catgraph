//! Comparing and clustering text MEANINGS over the BTV 2021 Yoneda embedding.
//!
//! Bradley–Terilla–Vlassopoulos 2021 (*An enriched category theory of language*,
//! arXiv:2106.07890) models each text `x` as its representable copresheaf
//! `L(x, −) = π(− | x)` — the meaning of `x` *is* its distribution of
//! extension-probabilities over every other text ("meaning is context-change
//! potential"). This example builds a small BYO-LM whose objects are texts, then:
//!
//! 1. materializes every meaning at once with `LmCategory::yoneda_all`;
//! 2. ranks the meanings nearest a chosen text in **both** directions of the BTV
//!    asymmetric distance (`k_nearest_from` vs `k_nearest_to`) and shows they
//!    disagree — the hom is asymmetric (BTV 2021 §5);
//! 3. clusters the meanings with the symmetric convenience metric
//!    (`cluster_semantic_sym`).
//!
//! ## The fixture (meaning-as-context-change-potential)
//!
//! Two "topic" families of interchangeable (synonymous) short texts, plus a
//! general prompt that can continue into either family:
//!
//! ```text
//!   "sky is clear"  ⇄  "sun is out"     both → "good weather"      (fair-weather family)
//!   "sky is grey"   ⇄  "rain is coming" both → "bad weather"       (foul-weather family)
//!   "the weather"   → each of the four short texts (0.25 each)     (general prompt)
//! ```
//!
//! **Why the synonym loops.** The BTV symmetric distance
//! `max(d̂(a,b), d̂(b,a))` is finite only when `support(a) = support(b)` — i.e.
//! when the two texts are **mutually reachable** and share every continuation.
//! That is exactly synonymy: interchangeable texts with the same context-change
//! potential. Distinct texts in a strict prompt→extension DAG are never mutually
//! reachable, so a purely acyclic fixture would symmetric-cluster into
//! singletons. The semantic layer (Yoneda / clustering) does not require the
//! acyclic poset hypothesis that `magnitude()` needs, so modelling synonymy with
//! a two-way transition is well-defined here.

use catgraph_magnitude::lm_category::LmCategory;
use catgraph_magnitude::{cluster_semantic_sym, k_nearest_from, k_nearest_to};

/// Build the two-family synonym fixture described in the module docs.
///
/// Object order (the `NodeId` indexing used throughout):
///   0 "the weather"   1 "sky is clear"  2 "sun is out"   3 "good weather"
///   4 "sky is grey"   5 "rain is coming" 6 "bad weather"
fn build_texts() -> LmCategory {
    let mut m = LmCategory::new(vec![
        "the weather".into(),    // 0 — general prompt
        "sky is clear".into(),   // 1 ⇄ 2  fair-weather family
        "sun is out".into(),     // 2
        "good weather".into(),   // 3 — fair continuation
        "sky is grey".into(),    // 4 ⇄ 5  foul-weather family
        "rain is coming".into(), // 5
        "bad weather".into(),    // 6 — foul continuation
    ]);

    // General prompt continues into any of the four short texts, uniformly.
    m.add_transition("the weather", "sky is clear", 0.25)
        .unwrap();
    m.add_transition("the weather", "sun is out", 0.25).unwrap();
    m.add_transition("the weather", "sky is grey", 0.25)
        .unwrap();
    m.add_transition("the weather", "rain is coming", 0.25)
        .unwrap();

    // Fair-weather synonyms: mutually reachable, shared continuation.
    m.add_transition("sky is clear", "sun is out", 0.5).unwrap();
    m.add_transition("sky is clear", "good weather", 0.5)
        .unwrap();
    m.add_transition("sun is out", "sky is clear", 0.5).unwrap();
    m.add_transition("sun is out", "good weather", 0.5).unwrap();

    // Foul-weather synonyms: mutually reachable, shared continuation.
    m.add_transition("sky is grey", "rain is coming", 0.5)
        .unwrap();
    m.add_transition("sky is grey", "bad weather", 0.5).unwrap();
    m.add_transition("rain is coming", "sky is grey", 0.5)
        .unwrap();
    m.add_transition("rain is coming", "bad weather", 0.5)
        .unwrap();

    m.mark_terminating("good weather");
    m.mark_terminating("bad weather");
    m
}

/// Render a `(NodeId, distance)` ranking against the object names.
fn show(label: &str, names: &[String], ranked: &[(usize, f64)]) {
    println!("  {label}");
    for &(id, d) in ranked {
        let shown = if d.is_infinite() {
            "∞ (mutually unreachable)".to_owned()
        } else {
            format!("{d:.4}")
        };
        println!("    d̂ = {shown:<26}  {:?}", names[id]);
    }
}

fn main() {
    let m = build_texts();
    let names = m.objects().to_vec();

    // 1. All meanings in one enriched-space pass.
    let meanings = m
        .yoneda_all()
        .expect("well-formed fixture yields an enriched space");

    println!(
        "=== BTV 2021 semantic comparison over {} texts ===\n",
        names.len()
    );

    // 2. Bidirectional nearest meanings from the fair-weather text "sky is clear".
    //
    // from-direction d̂(query, c): meanings the query most EXTENDS INTO — its
    //   generalizations (candidates whose support contains the query's).
    // to-direction   d̂(c, query): meanings the query most EXTENDS — its
    //   specializations (candidates whose support is contained in the query's).
    // Because the BTV hom is asymmetric (§5) these are different sets.
    let query_id = 1usize; // "sky is clear"
    let query = &meanings[query_id];
    let k = 3;
    println!("Nearest meanings to {:?} (k = {k}):\n", names[query_id]);

    let from = k_nearest_from(query, &meanings, k);
    let to = k_nearest_to(query, &meanings, k);
    show(
        "k_nearest_from — meanings the query extends INTO (generalizations):",
        &names,
        &from,
    );
    println!();
    show(
        "k_nearest_to   — meanings the query EXTENDS (specializations):",
        &names,
        &to,
    );
    println!();

    let from_ids: Vec<usize> = from.iter().map(|&(id, _)| id).collect();
    let to_ids: Vec<usize> = to.iter().map(|&(id, _)| id).collect();
    println!(
        "  → the two directions disagree: from = {from_ids:?}, to = {to_ids:?}\n    \
         (asymmetry is BTV 2021 §5 — 'query's nearest' ≠ 'nearest to query').\n"
    );

    // 3. Symmetric single-linkage clustering. ε above the within-family
    //    distance (−ln0.5 ≈ 0.693) and below ∞ groups each synonym family;
    //    cross-family pairs sit at ∞ and never merge.
    let epsilon = 0.75;
    let clusters = cluster_semantic_sym(&meanings, epsilon);
    println!("Clusters at ε = {epsilon} (symmetric convenience metric, non-canonical):");
    for cluster in &clusters {
        let texts: Vec<&str> = cluster.iter().map(|&id| names[id].as_str()).collect();
        let tag = if cluster.len() >= 2 {
            "cluster"
        } else {
            "singleton"
        };
        println!("    [{tag}] {texts:?}");
    }
    let nontrivial = clusters.iter().filter(|c| c.len() >= 2).count();
    println!(
        "\n  → {nontrivial} nontrivial clusters (the two synonym families); the \
         general prompt\n    and the two continuations stay singletons (∞-separated)."
    );

    // -----------------------------------------------------------------------
    // Assertions — this example doubles as an integration check.
    // -----------------------------------------------------------------------

    // The asymmetry gap is nonempty: the two nearest-meaning rankings differ.
    assert_ne!(
        from_ids, to_ids,
        "expected the two BTV distance directions to disagree"
    );

    // Exactly the two synonym families cluster (the full-partition assert
    // subsumes the nontrivial-cluster count printed above).
    assert_eq!(
        clusters,
        vec![vec![0], vec![1, 2], vec![3], vec![4, 5], vec![6]],
        "unexpected cluster partition"
    );

    println!("\nAll semantic_comparison assertions hold.");
}
