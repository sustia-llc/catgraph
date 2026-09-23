//! `MultiwayEvolutionGraph::{to_parts, from_parts}`, the graph's `serde`
//! round trip, and `add_sequential_step` called twice from one node.
//!
//! The round-trip fixture is built through the builders: two roots, a
//! two-way fork, a sequential step, a merge edge, a further sequential step,
//! and two `add_sequential_step` calls from one root. Each `from_parts`
//! rejection runs on a minimal hand-built `MultiwayParts`.

use catgraph::canonical_fingerprint;
use catgraph_physics::multiway::{
    BranchId, MultiwayEdge, MultiwayEdgeKind, MultiwayEvolutionGraph, MultiwayNode, MultiwayNodeId,
    MultiwayParts, MultiwayPartsError,
};

type Graph = MultiwayEvolutionGraph<String, String>;

fn s(text: &str) -> String {
    text.to_string()
}

/// Two roots; root 0 forks into `A`/`B`, `A → M`, `B` merges into `M`,
/// `M → Z`; root 1 steps to `U` and then, from the same root, to `V`.
fn fixture() -> Graph {
    let mut graph = Graph::new();
    let r0 = graph.add_root(s("S"));
    let r1 = graph.add_root(s("T"));
    let kids = graph.add_fork(r0, vec![(s("A"), s("s->a"), 0), (s("B"), s("s->b"), 1)]);
    let m = graph.add_sequential_step(kids[0], s("M"), s("a->m"));
    graph.add_merge_edge(kids[1], m, s("b->m"));
    graph.add_sequential_step(m, s("Z"), s("m->z"));
    graph.add_sequential_step(r1, s("U"), s("t->u"));
    graph.add_sequential_step(r1, s("V"), s("t->v"));
    graph
}

fn key(id: MultiwayNodeId) -> (usize, usize) {
    (id.step, id.branch_id.0)
}

fn sorted_keys(ids: &[MultiwayNodeId]) -> Vec<(usize, usize)> {
    let mut keys: Vec<(usize, usize)> = ids.iter().copied().map(key).collect();
    keys.sort_unstable();
    keys
}

/// Every derived observable the round-trip pin compares, labelled.
fn observables(graph: &Graph) -> Vec<(&'static str, Vec<usize>)> {
    let stats = graph.statistics();
    let flat = |keys: Vec<(usize, usize)>| keys.into_iter().flat_map(|(a, b)| [a, b]).collect();
    vec![
        ("node_count", vec![graph.node_count()]),
        ("edge_count", vec![graph.edge_count()]),
        ("max_step", vec![graph.max_step()]),
        ("branch_count", vec![graph.branch_count()]),
        ("leaves", flat(sorted_keys(graph.leaves()))),
        (
            "find_fork_points",
            flat(sorted_keys(&graph.find_fork_points())),
        ),
        (
            "find_merge_points",
            flat(sorted_keys(&graph.find_merge_points())),
        ),
        (
            "statistics",
            vec![
                stats.total_nodes,
                stats.total_edges,
                stats.max_branches,
                stats.max_depth,
                stats.merge_count,
                stats.fork_count,
                stats.leaf_count,
                stats.root_count,
            ],
        ),
    ]
}

fn assert_same_graph(reloaded: &Graph, original: &Graph) {
    assert_eq!(
        reloaded.to_parts(),
        original.to_parts(),
        "to_parts of the reloaded graph (left) differs from the original's (right)"
    );
    for ((name, got), (_, want)) in observables(reloaded).into_iter().zip(observables(original)) {
        assert_eq!(got, want, "{name}: reloaded {got:?}, original {want:?}");
    }
}

fn node(step: usize, branch: usize, state: &str) -> MultiwayNode<String> {
    let state = s(state);
    let fingerprint = canonical_fingerprint(&state);
    MultiwayNode::new(
        MultiwayNodeId::new(BranchId(branch), step),
        state,
        fingerprint,
    )
}

fn id(step: usize, branch: usize) -> MultiwayNodeId {
    MultiwayNodeId::new(BranchId(branch), step)
}

fn edge(from: MultiwayNodeId, to: MultiwayNodeId, kind: MultiwayEdgeKind) -> MultiwayEdge<String> {
    MultiwayEdge {
        from,
        to,
        kind,
        transition_data: s("t"),
    }
}

fn parts(
    nodes: Vec<MultiwayNode<String>>,
    edges: Vec<MultiwayEdge<String>>,
    roots: Vec<MultiwayNodeId>,
) -> MultiwayParts<String, String> {
    MultiwayParts {
        nodes,
        edges,
        roots,
        active_states: Vec::new(),
    }
}

fn rejection(parts: MultiwayParts<String, String>) -> MultiwayPartsError {
    match Graph::from_parts(parts) {
        Ok(graph) => panic!(
            "from_parts accepted the payload, expected a rejection; built {} nodes",
            graph.node_count()
        ),
        Err(error) => error,
    }
}

// ---------------------------------------------------------------------------
// add_sequential_step from one node twice
// ---------------------------------------------------------------------------

#[test]
fn two_sequential_steps_from_one_node_give_two_distinct_nodes() {
    let mut graph: MultiwayEvolutionGraph<u32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    let first = graph.add_sequential_step(root, 1, ());
    let second = graph.add_sequential_step(root, 2, ());

    let at_step_1 = graph.node_ids_at_step(1);
    assert_eq!(
        (graph.node_count(), at_step_1.len(), graph.leaves().len()),
        (3, 2, 2),
        "(node_count, node_ids_at_step(1) len, leaves len): step-1 ids {at_step_1:?}, leaves {:?}",
        graph.leaves()
    );
    assert_ne!(first, second, "both calls returned {first}");
    assert_eq!(
        at_step_1,
        vec![first, second],
        "node_ids_at_step(1) = {at_step_1:?}, expected [{first}, {second}]"
    );
    assert_eq!(
        graph.leaves(),
        &[first, second],
        "leaves = {:?}, expected [{first}, {second}]",
        graph.leaves()
    );
    assert_eq!(second.branch_id, BranchId(1), "second step on {second}");
    assert_eq!(graph.branch_count(), 2);
    let kinds: Vec<MultiwayEdgeKind> = graph
        .get_forward_edges(&root)
        .expect("invariant: the root has two out-edges")
        .iter()
        .map(|edge| edge.kind.clone())
        .collect();
    assert_eq!(
        kinds,
        vec![
            MultiwayEdgeKind::Sequential,
            MultiwayEdgeKind::Fork { rule_index: 1 }
        ],
        "root out-edge kinds"
    );
}

// ---------------------------------------------------------------------------
// Round trip
// ---------------------------------------------------------------------------

#[test]
fn from_parts_of_to_parts_reproduces_every_observable() {
    let original = fixture();
    let reloaded = Graph::from_parts(original.to_parts()).expect("builder parts are valid");
    assert_same_graph(&reloaded, &original);
    let fingerprint = canonical_fingerprint(&s("Z"));
    assert_eq!(
        reloaded.find_merge_candidate(fingerprint),
        original.find_merge_candidate(fingerprint),
        "find_merge_candidate(Z)"
    );
}

#[test]
fn add_fork_on_a_reloaded_graph_allocates_a_fresh_branch() {
    let original = fixture();
    let mut reloaded = Graph::from_parts(original.to_parts()).expect("builder parts are valid");
    let present: Vec<BranchId> = original
        .to_parts()
        .nodes
        .iter()
        .map(|node| node.id.branch_id)
        .collect();
    let leaf = *reloaded.leaves().first().expect("the fixture has leaves");
    let new_ids = reloaded.add_fork(leaf, vec![(s("X"), s("x"), 0), (s("Y"), s("y"), 1)]);
    for new_id in &new_ids {
        assert!(
            !present.contains(&new_id.branch_id),
            "add_fork allocated {}, already present among {present:?}",
            new_id.branch_id
        );
    }
    assert_eq!(
        reloaded.node_count(),
        original.node_count() + 2,
        "a reused id overwrites a node: {} nodes, expected {}",
        reloaded.node_count(),
        original.node_count() + 2
    );
}

#[test]
fn from_parts_is_independent_of_input_order() {
    let original = fixture();
    let forward = original.to_parts();
    let mut reversed = forward.clone();
    reversed.nodes.reverse();
    reversed.edges.reverse();
    reversed.active_states.reverse();

    let a = Graph::from_parts(forward).expect("builder parts are valid");
    let b = Graph::from_parts(reversed).expect("reversed builder parts are valid");
    assert_eq!(
        b.to_parts(),
        a.to_parts(),
        "to_parts, reversed input vs forward"
    );
    assert_eq!(
        b.leaves(),
        a.leaves(),
        "leaves order: reversed input {:?}, forward {:?}",
        b.leaves(),
        a.leaves()
    );
    for node in &a.to_parts().nodes {
        let targets = |graph: &Graph| -> Vec<MultiwayNodeId> {
            graph
                .get_forward_edges(&node.id)
                .map(|edges| edges.iter().map(|edge| edge.to).collect())
                .unwrap_or_default()
        };
        assert_eq!(
            targets(&b),
            targets(&a),
            "forward edge order at {}: reversed input vs forward",
            node.id
        );
    }
}

// ---------------------------------------------------------------------------
// Rejections — one per variant
// ---------------------------------------------------------------------------

#[test]
fn rejects_duplicate_node() {
    let error = rejection(parts(
        vec![node(0, 0, "a"), node(0, 0, "a")],
        vec![],
        vec![id(0, 0)],
    ));
    assert_eq!(error, MultiwayPartsError::DuplicateNode { id: id(0, 0) });
}

#[test]
fn rejects_fingerprint_mismatch() {
    let mut bad = node(0, 0, "a");
    let computed = bad.fingerprint;
    bad.fingerprint = computed.wrapping_add(1);
    let error = rejection(parts(vec![bad], vec![], vec![id(0, 0)]));
    assert_eq!(
        error,
        MultiwayPartsError::FingerprintMismatch {
            id: id(0, 0),
            stored: computed.wrapping_add(1),
            computed,
        }
    );
}

#[test]
fn rejects_edge_endpoint_missing() {
    let error = rejection(parts(
        vec![node(0, 0, "a")],
        vec![edge(id(0, 0), id(1, 0), MultiwayEdgeKind::Sequential)],
        vec![id(0, 0)],
    ));
    assert_eq!(
        error,
        MultiwayPartsError::EdgeEndpointMissing {
            from: id(0, 0),
            to: id(1, 0),
            missing: id(1, 0),
        }
    );
}

#[test]
fn rejects_step_not_advanced_by_one() {
    let kind = MultiwayEdgeKind::Fork { rule_index: 0 };
    let error = rejection(parts(
        vec![node(0, 0, "a"), node(2, 1, "b")],
        vec![edge(id(0, 0), id(2, 1), kind.clone())],
        vec![id(0, 0)],
    ));
    assert_eq!(
        error,
        MultiwayPartsError::StepNotAdvancedByOne {
            from: id(0, 0),
            to: id(2, 1),
            kind,
        }
    );
}

#[test]
fn rejects_sequential_branch_change() {
    let error = rejection(parts(
        vec![node(0, 0, "a"), node(1, 1, "b")],
        vec![edge(id(0, 0), id(1, 1), MultiwayEdgeKind::Sequential)],
        vec![id(0, 0)],
    ));
    assert_eq!(
        error,
        MultiwayPartsError::SequentialBranchChange {
            from: id(0, 0),
            to: id(1, 1),
        }
    );
}

#[test]
fn rejects_root_not_a_node() {
    let error = rejection(parts(vec![node(0, 0, "a")], vec![], vec![id(0, 3)]));
    assert_eq!(error, MultiwayPartsError::RootNotANode { root: id(0, 3) });
}

#[test]
fn rejects_root_not_at_step_zero() {
    let error = rejection(parts(vec![node(1, 0, "a")], vec![], vec![id(1, 0)]));
    assert_eq!(
        error,
        MultiwayPartsError::RootNotAtStepZero { root: id(1, 0) }
    );
}

#[test]
fn rejects_duplicate_root() {
    let error = rejection(parts(
        vec![node(0, 0, "a")],
        vec![],
        vec![id(0, 0), id(0, 0)],
    ));
    assert_eq!(error, MultiwayPartsError::DuplicateRoot { root: id(0, 0) });
}

#[test]
fn rejects_active_state_not_a_node() {
    let mut payload = parts(vec![node(0, 0, "a")], vec![], vec![id(0, 0)]);
    payload.active_states = vec![(7, id(0, 5))];
    let error = rejection(payload);
    assert_eq!(
        error,
        MultiwayPartsError::ActiveStateNotANode {
            fingerprint: 7,
            id: id(0, 5),
        }
    );
}

#[test]
fn rejects_active_state_fingerprint_mismatch() {
    let root = node(0, 0, "a");
    let node_fingerprint = root.fingerprint;
    let mut payload = parts(vec![root], vec![], vec![id(0, 0)]);
    payload.active_states = vec![(node_fingerprint.wrapping_add(1), id(0, 0))];
    let error = rejection(payload);
    assert_eq!(
        error,
        MultiwayPartsError::ActiveStateFingerprintMismatch {
            fingerprint: node_fingerprint.wrapping_add(1),
            id: id(0, 0),
            node_fingerprint,
        }
    );
}

#[test]
fn rejects_duplicate_active_state() {
    let root = node(0, 0, "a");
    let fingerprint = root.fingerprint;
    let mut payload = parts(vec![root], vec![], vec![id(0, 0)]);
    payload.active_states = vec![(fingerprint, id(0, 0)), (fingerprint, id(0, 0))];
    let error = rejection(payload);
    assert_eq!(
        error,
        MultiwayPartsError::DuplicateActiveState { fingerprint }
    );
}

// ---------------------------------------------------------------------------
// serde
// ---------------------------------------------------------------------------

#[cfg(feature = "serde")]
#[test]
fn serde_json_round_trip_reproduces_every_observable() {
    let original = fixture();
    let json = serde_json::to_string(&original).expect("the fixture serializes");
    let reloaded: Graph = serde_json::from_str(&json).expect("the fixture's JSON deserializes");
    assert_same_graph(&reloaded, &original);
}

#[cfg(feature = "serde")]
#[test]
fn serde_rejects_a_malformed_payload_with_the_parts_error() {
    let payload = parts(vec![node(1, 0, "a")], vec![], vec![id(1, 0)]);
    let json = serde_json::to_string(&payload).expect("parts serialize");
    let error = match serde_json::from_str::<Graph>(&json) {
        Ok(graph) => panic!(
            "a root at step 1 deserialized into a graph of {} nodes",
            graph.node_count()
        ),
        Err(error) => error,
    };
    let expected = MultiwayPartsError::RootNotAtStepZero { root: id(1, 0) }.to_string();
    assert!(
        error.to_string().contains(&expected),
        "deserialize error {error:?} does not carry {expected:?}"
    );
}
