//! End-to-end **K1 → K2 consumer path** (#23): drive
//! `catgraph_applied::Hypergraph` (the K1 CRUD container) into
//! `catgraph_magnitude::coalition_value` (the K2 stable diversity scalar).
//!
//! catgraph-magnitude depends on catgraph-applied, so this is the crate where
//! the cross-crate seam is actually exercised: build a coalition as a
//! hyperedge, read its ordered members with `get_hyperedge_vertices`, map
//! `VertexIndex` → agent-list indices, assemble a `(from, to, prob)` couplings
//! table, and call `coalition_value`. The three tests pin the documented
//! contract points of that seam:
//!
//! 1. the chain fixture value (`a→b 0.7, b→c 0.5 ⇒ Mag(1) = 1.8`) and the
//!    `coalition_value == coalition_magnitude_from_couplings(.., 1.0)` identity;
//! 2. the **dedup-before-magnitude** contract collision — a hyperedge legally
//!    carries duplicate members (applied semantic #2), but the magnitude layer
//!    rejects them, so a consumer must deduplicate first;
//! 3. the **skeletalization** seam — two agents mutually coupled at `1.0`
//!    collapse to one effective agent, so `Mag = 1.0` (matches #22's
//!    `mutual_one_pair` unit test through the plain-data path).

use std::collections::HashMap;

use catgraph_applied::{HyperedgeIndex, Hypergraph, VertexIndex};
use catgraph_magnitude::{coalition_magnitude_from_couplings, coalition_value};

const EPS: f64 = 1e-9;

/// Map a hyperedge's member `VertexIndex` list to indices into an `agents`
/// slice, using the vertex-index → position table built at registration time.
/// This is exactly the `VertexIndex → agent-list index` step a consumer
/// performs between `get_hyperedge_vertices` and `coalition_value`.
fn member_indices(
    g: &Hypergraph<&str, u8>,
    edge: HyperedgeIndex,
    vid_to_idx: &HashMap<VertexIndex, usize>,
) -> Vec<usize> {
    g.get_hyperedge_vertices(edge)
        .expect("edge exists")
        .into_iter()
        .map(|v| vid_to_idx[&v])
        .collect()
}

/// Order-preserving dedup of member indices (first occurrence wins) — the
/// mandatory step before feeding a hyperedge's members to the magnitude layer.
fn dedup_preserving_order(members: &[usize]) -> Vec<usize> {
    let mut seen = HashMap::new();
    let mut out = Vec::with_capacity(members.len());
    for &m in members {
        if seen.insert(m, ()).is_none() {
            out.push(m);
        }
    }
    out
}

/// Register `agents` as vertices and return the graph plus the
/// `VertexIndex → agent-list index` table.
fn registry(
    agents: &[&'static str],
) -> (Hypergraph<&'static str, u8>, HashMap<VertexIndex, usize>) {
    let mut g: Hypergraph<&'static str, u8> = Hypergraph::new();
    let vid_to_idx = agents
        .iter()
        .enumerate()
        .map(|(i, &a)| (g.add_vertex(a), i))
        .collect();
    (g, vid_to_idx)
}

// -----------------------------------------------------------------------------
// 1. The documented consumer path, end to end: hyperedge members → couplings →
//    coalition_value. Chain a→b→c (0.7, 0.5) ⇒ Mag(1) = 1.8, and the stable
//    entry point equals the explicit-t=1 plain-data call.
// -----------------------------------------------------------------------------
#[test]
fn consumer_path_chain_end_to_end() {
    let agents = ["alice", "bob", "carol", "dave", "erin"];
    let (mut g, vid_to_idx) = registry(&agents);

    // A coalition over the first three agents (the chain participants).
    let coalition = g
        .add_hyperedge(
            vec![VertexIndex(0), VertexIndex(1), VertexIndex(2)],
            1, // coalition tag
        )
        .unwrap();

    // K1 read → VertexIndex → agent-list indices.
    let members = member_indices(&g, coalition, &vid_to_idx);
    assert_eq!(members, vec![0, 1, 2]);

    // koalisi would map member capabilities to these pairwise couplings.
    let couplings = [(0usize, 1usize, 0.7_f64), (1, 2, 0.5)];

    // K2: the stable consumer scalar.
    let value = coalition_value(&agents, &couplings, &members).unwrap();

    // Hand-derived: closed ζ (order a,b,c) is upper-triangular
    //   [[1, 0.7, 0.35], [0, 1, 0.5], [0, 0, 1]]   (a→c closes to 0.7·0.5 = 0.35)
    // back-substitute ζ·w = 1: w_c = 1; w_b = 1 − 0.5 = 0.5;
    //   w_a = 1 − 0.7·0.5 − 0.35·1 = 0.30 ⇒ Mag(1) = 0.30 + 0.5 + 1.0 = 1.80.
    assert!(
        (value - 1.8).abs() < EPS,
        "chain Mag(1) = {value}, expected 1.8"
    );

    // coalition_value is exactly coalition_magnitude_from_couplings at t = 1.
    let via_t1 = coalition_magnitude_from_couplings(&agents, &couplings, &members, 1.0).unwrap();
    assert!((value - via_t1).abs() < EPS);
}

// -----------------------------------------------------------------------------
// 2. Dedup-before-magnitude contract collision. A hyperedge may legally hold a
//    duplicate member (applied semantic #2), but coalition_value rejects a
//    duplicate member index — so the consumer MUST dedup first.
// -----------------------------------------------------------------------------
#[test]
fn duplicate_member_must_be_deduped_before_magnitude() {
    let agents = ["alice", "bob", "carol", "dave", "erin"];
    let (mut g, vid_to_idx) = registry(&agents);

    // Legal in the K1 container: an ordered member list with a duplicate.
    let coalition = g
        .add_hyperedge(vec![VertexIndex(0), VertexIndex(1), VertexIndex(0)], 1)
        .unwrap();

    let raw = member_indices(&g, coalition, &vid_to_idx);
    assert_eq!(
        raw,
        vec![0, 1, 0],
        "the hyperedge kept its duplicate member"
    );

    let couplings = [(0usize, 1usize, 0.5_f64)];

    // Non-deduped list → coalition_value errors (a repeated agent would seed two
    // ∞-separated nodes for one agent).
    assert!(
        coalition_value(&agents, &couplings, &raw).is_err(),
        "the magnitude layer must reject the duplicate member"
    );

    // Deduped list → coalition_value succeeds.
    let deduped = dedup_preserving_order(&raw);
    assert_eq!(deduped, vec![0, 1]);
    assert!(
        coalition_value(&agents, &couplings, &deduped).is_ok(),
        "the deduped member list feeds the magnitude layer cleanly"
    );
}

// -----------------------------------------------------------------------------
// 3. Skeletalization seam. Two agents mutually coupled at 1.0 are at distance 0
//    both ways ⇒ one effective agent ⇒ Mag = 1.0 (matches #22's mutual_one_pair
//    unit test, reached here through the K1 → K2 consumer path).
// -----------------------------------------------------------------------------
#[test]
fn mutual_one_pair_collapses_to_single_effective_agent() {
    let agents = ["x", "y"];
    let (mut g, vid_to_idx) = registry(&agents);

    let coalition = g
        .add_hyperedge(vec![VertexIndex(0), VertexIndex(1)], 1)
        .unwrap();
    let members = member_indices(&g, coalition, &vid_to_idx);
    assert_eq!(members, vec![0, 1]);

    // Perfect mutual coupling: x ⇄ y at 1.0 (distance 0 both directions).
    let couplings = [(0usize, 1usize, 1.0_f64), (1, 0, 1.0)];

    let value = coalition_value(&agents, &couplings, &members).unwrap();
    assert!(
        (value - 1.0).abs() < EPS,
        "mutual-1.0 pair collapses to one effective agent ⇒ Mag = 1.0, got {value}"
    );
}
