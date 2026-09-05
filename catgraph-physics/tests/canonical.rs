//! The catgraph-physics claim, end to end.
//!
//! catgraph-physics is inspiration-anchored, not theorem-anchored
//! (`catgraph-physics/docs/ANCHORS.md`): the Wilson-loop / holonomy / branchial
//! vocabulary is read off \[Gor23\] and lattice gauge theory, and the crate's
//! own claims are the two below.
//!
//! **Causal invariance.** `RewriteRule::wolfram_a_to_bb` on
//! `{{0,1,2},{1,2,3}}` to depth 3 reaches isomorphic states on more than one
//! branch, and every such pair's root-based causal graphs compare
//! `CausalComparison::Isomorphic`, with holonomy `1.0` on every Wilson loop;
//! `RewriteRule::collapse` on `{{0,1},{1,2},{2,3},{3,4}}` to depth 4 has a pair
//! that compares `NotIsomorphic` and loops of holonomy `0.0`. On every distinct
//! causal-graph shape either fixture produces, and on hand-built
//! `CausalGraph::from_events` pairs, `CausalGraph::compare` returns
//! `Isomorphic` exactly when a brute-force permutation search over
//! `catgraph_testutil::all_perm_indices` finds an edge-preserving bijection.
//!
//! **Branchial pipeline.** `run_multiway_bfs` → `BranchialGraph` →
//! `OllivierRicciCurvature` via `wasserstein_1` reproduces hand-computed
//! curvature on a K₄ branchial slice; `wasserstein_1` agrees with exhaustive
//! transport optima on three seeded families and with exact integer transport
//! on the `#163` topologies, carries the Petersen edge curvature −1/3, and is
//! zero on `(mu, mu)` and symmetric in its two marginals over the sampled
//! family below; `to_petgraph` keeps parallel edges and drops an edge with an
//! unregistered endpoint on both carriers.
//!
//! # Input space
//!
//! Construction and rule structure run on `{{0,1,2},{2,3,4}}` and on a
//! two-`add_hyperedge` build. Matching runs on `{{7,8,9}}` and on the empty
//! graph. Deterministic and trivial evolution runs on `{{0,1,2}}` at 3 steps,
//! on the empty graph at 5 steps, and through `run_multiway` at 0 steps.
//! Multiway enumeration runs on `{{0,1,2},{3,4,5}}` at 2 steps and `max_nodes`
//! 100, and on `{{0,1,2}}` under `RewriteRule::wolfram_a_to_bb` and
//! `RewriteRule::edge_split` at 3 steps and `max_nodes` 50. The gauge arm runs
//! on lattices of sizes `[5]` and `[4,4]`.
//! The two rewriting fixtures are the ones named above, at `max_nodes` 50 and
//! 200. The brute-force isomorphism corpus is every distinct
//! `(event_count, causal edge list)` shape those two fixtures produce through
//! `HypergraphEvolution::causal_graph`, plus nine hand-built graphs, over every
//! ordered pair; the arm asserts each graph it holds has at most 8 events, the
//! width of the permutation search.
//! `wasserstein_1` runs on 300 seeded 3x4 instances with margins in twelfths,
//! 400 seeded uniform k x k instances for each `k` in `2..=6`, 300 seeded
//! instances of the 3x4 family embedded in a 6x8 support with zero-mass rows
//! and columns interleaved, the five `#163` topology fixtures, and 256 proptest
//! cases of marginals of length 2 to 6 over symmetric zero-diagonal ground
//! matrices with entries in `0.0..=8.0`. The branchial pipeline runs on the
//! binary-fork evolution to depth 2. The `to_petgraph` fixtures are hand-built
//! at 3 and 2 nodes.
//!
//! # References
//!
//! `compare` is checked against a permutation search written in this file over
//! the graphs' own edge lists. `wasserstein_1` is checked against the minimum
//! over every non-negative integer contingency table at the margins'
//! denominator, against the minimum over the `k!` permutation couplings, and
//! against a branch-and-bound integer transport minimum — three enumerations
//! with no catgraph edge. The K₄ curvature, the Petersen curvature and the
//! `to_petgraph` counts are hand-computed constants written from the
//! definition and the contract sentences at
//! `src/multiway/branchial_analysis.rs:55` and `src/multiway/ollivier_ricci.rs:60-62`.
//!
//! # Reach
//!
//! `CausalComparison::Undecided` is returned past
//! `CausalGraph::MAX_SEARCH_STEPS` (200 000 candidate assignments) and is out
//! of reach on this file's corpus: the brute-force arm asserts every
//! comparison it makes over the two fixtures' causal-graph shapes and the
//! hand-built graphs is `Isomorphic` or `NotIsomorphic`.
//! The two `to_petgraph` arms need the `rustworkx` feature that gates
//! `src/multiway/branchial_analysis.rs` and carry a per-arm
//! `#[cfg(feature = "rustworkx")]`; every other arm runs on
//! `--no-default-features` too.
//! `tests/branchial_analysis.rs` (coloring, k-core, spectra),
//! `tests/catgraph_bridge.rs` (the cospan/span bridge),
//! `tests/gauge_theory.rs` (the lattice's own evolution and Wilson-loop search)
//! and `tests/multiway_evolution.rs` (the evolution-graph surface and the
//! `wasserstein_1` triangle inequality) exercise the rest of the crate, not
//! this file.
//!
//! # covers:
//!
//! `BranchId` `BranchialGraph` `CausalComparison` `CausalEvent` `CausalGraph`
//! `CausalInvarianceResult` `CurvatureFoliation` `DiscreteCurvature` `EdgeId`
//! `EventId` `EvolutionStatistics` `GaugeGroup` `Hyperedge` `Hypergraph`
//! `HypergraphEvolution` `HypergraphLattice` `HypergraphNode`
//! `HypergraphRewriteGroup` `MultiwayEdge` `MultiwayEdgeKind`
//! `MultiwayEvolutionGraph` `MultiwayNode` `MultiwayNodeId` `OllivierFoliation`
//! `OllivierRicciCurvature` `RewriteMatch` `RewriteRule` `WilsonLoop`
//!
//! # not-covered:
//!
//! `BranchialSpectrum` `BranchialStepStats` `BranchialSummary`
//! `ConfluenceDiamond` `ConservationResult` `CospanInvarianceResult`
//! `CospanMergeDetail` `DiscreteInterval` `HypergraphStep` `MergePoint`
//! `MultiwayCospan` `MultiwayCospanExt` `MultiwayCospanGraph` `MultiwayCycle`
//! `MultiwayStatistics` `ParallelIntervals` `RepeatDetection` `RewriteEffect`
//! `RewriteSpan` `RewriteSpanError` `SpanSide` `StepTrace` `TemporalComplex`
//! `TemporalComplexError` `TraceAnalysis`

use std::collections::{HashMap, HashSet, VecDeque};

use catgraph_physics::hypergraph::{
    CausalComparison, CausalEvent, CausalGraph, EdgeId, EventId, GaugeGroup, Hypergraph,
    HypergraphEvolution, HypergraphLattice, HypergraphRewriteGroup, RewriteRule, plaquette_action,
    total_action,
};
use catgraph_physics::multiway::{
    BranchId, BranchialGraph, DiscreteCurvature, MultiwayEdgeKind, MultiwayEvolutionGraph,
    MultiwayNodeId, OllivierFoliation, OllivierRicciCurvature, extract_branchial_foliation,
    run_multiway_bfs, wasserstein_1,
};
use catgraph_testutil::{Lcg, all_perm_indices};

use proptest::prelude::*;

use rustworkx_core::generators::{
    gnp_random_graph, grid_graph, path_graph, petersen_graph, random_regular_graph,
};
use rustworkx_core::petgraph::graph::UnGraph;

// ===========================================================================
// Arm A fixtures — hypergraph rewriting
// ===========================================================================

/// `{{0,1,2},{1,2,3}}` under `A→BB` to depth 3.
fn confluent_fixture() -> HypergraphEvolution {
    HypergraphEvolution::run_multiway(
        &Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]),
        &[RewriteRule::wolfram_a_to_bb()],
        3,
        50,
    )
}

/// `{{0,1},{1,2},{2,3},{3,4}}` under `collapse` to depth 4.
fn non_confluent_fixture() -> HypergraphEvolution {
    HypergraphEvolution::run_multiway(
        &Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]),
        &[RewriteRule::collapse()],
        4,
        200,
    )
}

/// One update event, from raw instance identities.
fn event(rule_index: usize, consumed: &[usize], produced: &[usize]) -> CausalEvent {
    CausalEvent {
        rule_index,
        consumed: consumed.iter().copied().map(EdgeId).collect(),
        produced: produced.iter().copied().map(EdgeId).collect(),
    }
}

/// The unordered pairs of distinct evolution nodes whose states are isomorphic
/// — the pairs `find_wilson_loops` closes a loop over.
fn isomorphic_state_pairs(evolution: &HypergraphEvolution) -> Vec<(usize, usize)> {
    let count = evolution.node_count();
    let mut pairs = Vec::new();
    for left in 0..count {
        for right in (left + 1)..count {
            let a = evolution
                .get_node(left)
                .expect("invariant: left < node_count");
            let b = evolution
                .get_node(right)
                .expect("invariant: right < node_count");
            if a.fingerprint == b.fingerprint && a.state.is_isomorphic_to(&b.state) {
                pairs.push((left, right));
            }
        }
    }
    pairs
}

/// `(event count, causal edges)` — everything `compare` reads off a graph.
fn shape(graph: &CausalGraph) -> (usize, Vec<(usize, usize)>) {
    (
        graph.event_count(),
        graph
            .causal_edges()
            .map(|(EventId(source), EventId(target))| (source, target))
            .collect(),
    )
}

/// One causal graph per distinct shape the evolution's nodes produce.
fn distinct_causal_shapes(evolution: &HypergraphEvolution) -> Vec<CausalGraph> {
    let mut seen: Vec<(usize, Vec<(usize, usize)>)> = Vec::new();
    let mut graphs = Vec::new();
    for id in 0..evolution.node_count() {
        let Some(graph) = evolution.causal_graph(id) else {
            continue;
        };
        let key = shape(&graph);
        if !seen.contains(&key) {
            seen.push(key);
            graphs.push(graph);
        }
    }
    graphs
}

/// True when some bijection of `left`'s events onto `right`'s carries every
/// causal edge to a causal edge, searched over `perms[n]`.
fn brute_force_isomorphic(left: &CausalGraph, right: &CausalGraph, perms: &[Vec<usize>]) -> bool {
    if left.event_count() != right.event_count()
        || left.causal_edge_count() != right.causal_edge_count()
    {
        return false;
    }
    let left_edges: Vec<(usize, usize)> = left
        .causal_edges()
        .map(|(EventId(source), EventId(target))| (source, target))
        .collect();
    let right_edges: HashSet<(usize, usize)> = right
        .causal_edges()
        .map(|(EventId(source), EventId(target))| (source, target))
        .collect();

    perms.iter().any(|p| {
        left_edges
            .iter()
            .all(|&(source, target)| right_edges.contains(&(p[source], p[target])))
    })
}

// ===========================================================================
// Arm A — construction, rules and evolution
// ===========================================================================

/// `Hypergraph` vertex/edge counts, and the two rules' left/right shapes read
/// through `Hyperedge`.
#[test]
fn hypergraph_and_rule_structure() {
    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![2, 3, 4]]);
    assert_eq!(graph.vertex_count(), 5, "vertices 0..=4");
    assert_eq!(graph.edge_count(), 2, "two hyperedges");

    let mut built = Hypergraph::new();
    built.add_hyperedge(vec![0, 1, 2]);
    built.add_hyperedge(vec![2, 3]);
    assert_eq!(built.vertex_count(), 4, "vertices 0..=3");
    assert_eq!(built.edge_count(), 2, "two hyperedges");

    let a_to_bb = RewriteRule::wolfram_a_to_bb();
    assert_eq!(a_to_bb.name(), Some("A\u{2192}BB"));
    assert_eq!(
        (
            a_to_bb.left_arity(),
            a_to_bb.right_arity(),
            a_to_bb.num_variables()
        ),
        (1, 2, 3),
        "A\u{2192}BB is {{{{x,y,z}}}} \u{2192} {{{{x,y}},{{y,z}}}}"
    );
    assert!(a_to_bb.deleted_variables().is_empty());
    assert!(a_to_bb.created_variables().is_empty());
    let left: Vec<Vec<usize>> = a_to_bb
        .left()
        .iter()
        .map(|e| e.vertices().to_vec())
        .collect();
    let right: Vec<Vec<usize>> = a_to_bb
        .right()
        .iter()
        .map(|e| e.vertices().to_vec())
        .collect();
    assert_eq!(left, vec![vec![0, 1, 2]], "one ternary edge on the left");
    assert_eq!(
        right,
        vec![vec![0, 1], vec![1, 2]],
        "two binary edges on the right"
    );
    assert_eq!(
        a_to_bb.left()[0].arity(),
        3,
        "the matched edge is ternary; got {:?}",
        a_to_bb.left()[0].vertices()
    );

    let collapse = RewriteRule::collapse();
    assert_eq!(collapse.name(), Some("collapse"));
    assert_eq!(
        (
            collapse.left_arity(),
            collapse.right_arity(),
            collapse.num_variables()
        ),
        (2, 1, 3),
        "collapse is {{{{x,y}},{{y,z}}}} \u{2192} {{{{x,z}}}}"
    );
    assert_eq!(
        collapse.deleted_variables(),
        vec![1],
        "the shared middle variable is dropped"
    );
    assert!(collapse.created_variables().is_empty());
}

/// `find_matches` on a matching graph, on the empty graph, and the variable
/// binding it reports through `RewriteMatch`.
#[test]
fn rule_matching_binds_variables_and_edges() {
    let rule = RewriteRule::wolfram_a_to_bb();

    let graph = Hypergraph::from_edges(vec![vec![7, 8, 9]]);
    let matches = rule.find_matches(&graph);
    assert_eq!(matches.len(), 1, "one ternary edge, one match site");
    assert_eq!(
        matches[0].matched_edges,
        vec![0],
        "the rule matched host edge 0"
    );
    assert_eq!(
        (
            matches[0].get(0),
            matches[0].get(1),
            matches[0].get(2),
            matches[0].get(3)
        ),
        (Some(7), Some(8), Some(9), None),
        "variables 0,1,2 bind to 7,8,9 and there is no variable 3"
    );

    assert!(
        rule.find_matches(&Hypergraph::new()).is_empty(),
        "no edges, no matches"
    );
}

/// `run` on one ternary edge, on the empty graph, and `run_multiway` at zero
/// steps, read through `HypergraphNode`.
#[test]
fn deterministic_and_trivial_evolution() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let graph = Hypergraph::from_edges(vec![vec![0, 1, 2]]);

    let evolution = HypergraphEvolution::run(&graph, std::slice::from_ref(&rule), 3);
    assert!(
        evolution.node_count() > 1,
        "the rule applies at least once, got {} nodes",
        evolution.node_count()
    );
    assert!(evolution.max_step() >= 1);
    let root = evolution.root();
    assert_eq!((root.id, root.step, root.parent), (0, 0, None));
    assert_eq!(root.state.edge_count(), 1, "the root is the initial graph");

    let empty = HypergraphEvolution::run(&Hypergraph::new(), std::slice::from_ref(&rule), 5);
    assert_eq!(empty.node_count(), 1, "no rule applies, only the root");
    assert_eq!(empty.max_step(), 0);
    assert_eq!(
        (
            empty.root().state.edge_count(),
            empty.root().state.vertex_count()
        ),
        (0, 0)
    );

    let zero = HypergraphEvolution::run_multiway(&graph, &[rule], 0, 100);
    assert_eq!(zero.node_count(), 1, "max_steps 0 expands nothing");
    assert_eq!(zero.max_step(), 0);
    assert_eq!(zero.root().state.edge_count(), 1);
}

/// `run_multiway` opens one node per match site per step, `leaves` are exactly
/// the childless nodes, and `statistics` reports the same counts.
#[test]
fn multiway_evolution_enumerates_every_match() {
    let rule = RewriteRule::wolfram_a_to_bb();
    let two_sites = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![3, 4, 5]]);
    let evolution =
        HypergraphEvolution::run_multiway(&two_sites, std::slice::from_ref(&rule), 2, 100);

    assert_eq!(
        evolution.node_count(),
        5,
        "root, two step-1 sites, one step-2 node from each"
    );
    assert_eq!(evolution.nodes_at_step(1), vec![1, 2]);
    assert_eq!(evolution.nodes_at_step(2), vec![3, 4]);
    assert_eq!(evolution.max_step(), 2);

    let branching = HypergraphEvolution::run_multiway(
        &Hypergraph::from_edges(vec![vec![0, 1, 2]]),
        &[rule, RewriteRule::edge_split()],
        3,
        50,
    );
    assert!(
        branching.node_count() >= 3,
        "two rules branch the tree, got {} nodes",
        branching.node_count()
    );

    let parents: HashSet<usize> = (0..branching.node_count())
        .filter_map(|id| {
            branching
                .get_node(id)
                .expect("invariant: id < node_count")
                .parent
        })
        .collect();
    let leaves = branching.leaves();
    for &leaf in &leaves {
        assert!(
            !parents.contains(&leaf),
            "node {leaf} has a child but leaves() returned it; leaves {leaves:?}"
        );
    }
    for id in 0..branching.node_count() {
        assert!(
            parents.contains(&id) || leaves.contains(&id),
            "node {id} has no child but leaves() omitted it; leaves {leaves:?}"
        );
    }

    let stats = branching.statistics();
    assert_eq!(stats.total_nodes, branching.node_count());
    assert_eq!(stats.max_step, branching.max_step());
    assert_eq!(stats.branch_count, leaves.len());
    assert_eq!(
        stats.rule_applications.len(),
        2,
        "one count per supplied rule"
    );
    assert_eq!(
        stats.rule_applications.iter().sum::<usize>(),
        branching.node_count() - 1,
        "every non-root node is one rule application; counts {:?}",
        stats.rule_applications
    );
}

// ===========================================================================
// Arm A — causal invariance
// ===========================================================================

/// Every isomorphic-state branch pair of the `A→BB` fixture has isomorphic
/// causal graphs, and every Wilson loop closes at holonomy `1.0`.
#[test]
fn confluent_fixture_branch_pairs_are_causally_isomorphic() {
    let evolution = confluent_fixture();

    let expected_node_1 = CausalGraph::from_events(vec![CausalEvent {
        rule_index: 0,
        consumed: vec![EdgeId(0)],
        produced: vec![EdgeId(2), EdgeId(3)],
    }]);
    assert_eq!(
        evolution.causal_graph(1),
        Some(expected_node_1.clone()),
        "node 1: expected {:?}, got {:?}",
        Some(&expected_node_1),
        evolution.causal_graph(1)
    );

    let pairs = isomorphic_state_pairs(&evolution);
    assert_eq!(
        pairs.len(),
        1,
        "expected 1 isomorphic-state pair, got {}: {pairs:?}",
        pairs.len()
    );
    for &(left, right) in &pairs {
        let a = evolution
            .causal_graph(left)
            .expect("invariant: left is an evolution node");
        let b = evolution
            .causal_graph(right)
            .expect("invariant: right is an evolution node");
        assert_eq!(
            a.compare(&b),
            CausalComparison::Isomorphic,
            "branch pair ({left}, {right}): expected Isomorphic, got {:?}; \
             edges {:?} against {:?}",
            a.compare(&b),
            a.causal_edges().collect::<Vec<_>>(),
            b.causal_edges().collect::<Vec<_>>()
        );
    }

    let loops = evolution.find_wilson_loops();
    assert_eq!(
        loops.len(),
        1,
        "expected one Wilson loop, got {}",
        loops.len()
    );
    assert_eq!(
        loops[0].path,
        vec![0, 1, 3, 4, 2, 0],
        "root down one branch, across the tips, back up the other"
    );
    assert_eq!((loops[0].base, loops[0].length), (0, 6));
    for (index, wilson) in loops.iter().enumerate() {
        assert!(
            (wilson.holonomy - 1.0).abs() < 1e-12,
            "loop {index}: expected holonomy 1.0, got {}",
            wilson.holonomy
        );
    }

    let result = evolution.analyze_causal_invariance();
    assert!(result.is_invariant, "every loop closes");
    assert_eq!(result.loops_analyzed, 1);
    assert!(result.non_trivial_loops.is_empty());
    assert!(
        result.average_deviation.abs() < 1e-12,
        "expected average deviation 0.0, got {}",
        result.average_deviation
    );
    assert!(
        result.max_deviation.abs() < 1e-12,
        "expected max deviation 0.0, got {}",
        result.max_deviation
    );
    assert!(evolution.is_causally_invariant());
}

/// The `collapse` fixture has a branch pair whose causal graphs separate, and
/// 6 of its 18 Wilson loops open.
#[test]
fn collapse_fixture_separates_a_branch_pair() {
    let evolution = non_confluent_fixture();

    let pairs = isomorphic_state_pairs(&evolution);
    assert!(
        !pairs.is_empty(),
        "the fixture must reach isomorphic states on distinct branches"
    );
    let separated = pairs
        .iter()
        .filter(|&&(left, right)| {
            let a = evolution
                .causal_graph(left)
                .expect("invariant: left is an evolution node");
            let b = evolution
                .causal_graph(right)
                .expect("invariant: right is an evolution node");
            a.compare(&b) == CausalComparison::NotIsomorphic
        })
        .count();
    assert!(
        separated > 0,
        "expected at least one NotIsomorphic branch pair, got 0 of {}",
        pairs.len()
    );

    let loops = evolution.find_wilson_loops();
    assert_eq!(
        loops.len(),
        18,
        "expected 18 Wilson loops, got {}",
        loops.len()
    );
    let closed = loops
        .iter()
        .filter(|wilson| (wilson.holonomy - 1.0).abs() < 1e-12)
        .count();
    let open = loops
        .iter()
        .filter(|wilson| wilson.holonomy.abs() < 1e-12)
        .count();
    assert_eq!(
        (closed, open),
        (12, 6),
        "expected 12 closing and 6 separating; holonomies {:?}",
        loops
            .iter()
            .map(|wilson| wilson.holonomy)
            .collect::<Vec<_>>()
    );
    for wilson in &loops {
        assert_eq!(
            wilson.path.first(),
            Some(&wilson.base),
            "loop opens at its base"
        );
        assert_eq!(
            wilson.path.last(),
            Some(&wilson.base),
            "loop closes at its base"
        );
        assert_eq!(wilson.length, wilson.path.len());
    }

    let result = evolution.analyze_causal_invariance();
    assert!(!result.is_invariant, "six loops open");
    assert_eq!(result.loops_analyzed, 18);
    assert_eq!(result.non_trivial_loops.len(), 6);
    assert!(
        (result.max_deviation - 1.0).abs() < 1e-12,
        "expected max deviation 1.0, got {}",
        result.max_deviation
    );
    assert!(
        (result.average_deviation - 6.0 / 18.0).abs() < 1e-12,
        "expected average deviation 1/3, got {}",
        result.average_deviation
    );
    assert!(!evolution.is_causally_invariant());
}

/// `compare` returns `Isomorphic` exactly when a permutation of the events
/// carries every causal edge to a causal edge.
#[test]
fn compare_agrees_with_brute_force_permutation_isomorphism() {
    let perms: Vec<Vec<Vec<usize>>> = (0..=8).map(all_perm_indices).collect();

    // Events 0–3 produce, events 4–7 consume; the underlying undirected shape
    // is one 8-cycle.
    let c8 = vec![
        event(0, &[], &[100, 101]),
        event(0, &[], &[111, 112]),
        event(0, &[], &[122, 123]),
        event(0, &[], &[133, 130]),
        event(0, &[100, 130], &[]),
        event(0, &[101, 111], &[]),
        event(0, &[112, 122], &[]),
        event(0, &[123, 133], &[]),
    ];
    // Same counts and same in/out degree sequences, two 4-cycles.
    let two_c4 = vec![
        event(0, &[], &[200, 201]),
        event(0, &[], &[210, 211]),
        event(0, &[], &[222, 223]),
        event(0, &[], &[232, 233]),
        event(0, &[200, 210], &[]),
        event(0, &[201, 211], &[]),
        event(0, &[222, 232], &[]),
        event(0, &[223, 233], &[]),
    ];
    let mirrored_c8 = vec![
        event(0, &[], &[300, 303]),
        event(0, &[], &[311, 310]),
        event(0, &[], &[322, 321]),
        event(0, &[], &[333, 332]),
        event(0, &[300, 310], &[]),
        event(0, &[311, 321], &[]),
        event(0, &[322, 332], &[]),
        event(0, &[333, 303], &[]),
    ];
    // Two orientations of the same undirected path on four events: a directed
    // chain, and a chain whose middle two edges leave one event.
    let directed_chain = vec![
        event(0, &[], &[400]),
        event(0, &[400], &[401]),
        event(0, &[401], &[402]),
        event(0, &[402], &[]),
    ];
    let shared_source = vec![
        event(0, &[], &[410]),
        event(0, &[], &[411, 412]),
        event(0, &[410, 411], &[]),
        event(0, &[412], &[]),
    ];

    let mut corpus: Vec<CausalGraph> = vec![
        CausalGraph::from_events(vec![]),
        CausalGraph::from_events(vec![event(0, &[0], &[10]), event(0, &[10], &[11])]),
        CausalGraph::from_events(vec![event(1, &[5], &[20]), event(1, &[20], &[21])]),
        CausalGraph::from_events(vec![event(0, &[0], &[10]), event(0, &[1], &[11])]),
        CausalGraph::from_events(c8),
        CausalGraph::from_events(two_c4),
        CausalGraph::from_events(mirrored_c8),
        CausalGraph::from_events(directed_chain),
        CausalGraph::from_events(shared_source),
    ];
    corpus.extend(distinct_causal_shapes(&confluent_fixture()));
    corpus.extend(distinct_causal_shapes(&non_confluent_fixture()));

    for graph in &corpus {
        assert!(
            graph.event_count() <= 8,
            "the permutation search runs at n <= 8, got {} events",
            graph.event_count()
        );
    }

    let mut isomorphic = 0usize;
    let mut not_isomorphic = 0usize;
    for (i, left) in corpus.iter().enumerate() {
        for (j, right) in corpus.iter().enumerate() {
            let verdict = left.compare(right);
            assert_ne!(
                verdict,
                CausalComparison::Undecided,
                "pair ({i}, {j}): the step budget was reached at {} events",
                left.event_count()
            );
            let expected = brute_force_isomorphic(left, right, &perms[left.event_count()]);
            assert_eq!(
                verdict == CausalComparison::Isomorphic,
                expected,
                "pair ({i}, {j}): compare said {verdict:?}, the permutation search said \
                 {expected}; edges {:?} against {:?}",
                left.causal_edges().collect::<Vec<_>>(),
                right.causal_edges().collect::<Vec<_>>()
            );
            if expected {
                isomorphic += 1;
            } else {
                not_isomorphic += 1;
            }
        }
    }
    assert!(
        isomorphic > 0 && not_isomorphic > 0,
        "the corpus must separate both ways, got {isomorphic} isomorphic and \
         {not_isomorphic} not"
    );
}

/// Hand-built `from_events` pairs pin `Isomorphic` and `NotIsomorphic`
/// directly, and pin which instances contribute a causal edge.
#[test]
fn hand_built_causal_graph_pairs() {
    // Event 0 produces 10 and 11; event 1 consumes 10 (dependent) and the
    // pre-existing instance 1 (no producer, so no edge).
    let chain =
        CausalGraph::from_events(vec![event(0, &[0], &[10, 11]), event(0, &[10, 1], &[12])]);
    assert_eq!(chain.event_count(), 2);
    assert_eq!(
        chain.causal_edges().collect::<Vec<_>>(),
        vec![(EventId(0), EventId(1))],
        "only instance 10 links a producer to a consumer"
    );
    assert_eq!(chain.causal_edge_count(), 1);
    assert!(!chain.is_empty());

    let antichain =
        CausalGraph::from_events(vec![event(0, &[0], &[10, 11]), event(0, &[1], &[12])]);
    assert_eq!(
        antichain.causal_edge_count(),
        0,
        "neither event consumes an instance the other produced"
    );

    // Isomorphic under a non-identity relabelling: the same two-event chain
    // over disjoint instance identities and different rule indices.
    let relabelled =
        CausalGraph::from_events(vec![event(7, &[5], &[20, 21]), event(9, &[20, 6], &[22])]);
    assert_eq!(
        chain.compare(&relabelled),
        CausalComparison::Isomorphic,
        "rule_index and instance identity are not part of the relation"
    );
    assert!(chain.is_isomorphic_to(&relabelled));

    assert_eq!(
        chain.compare(&antichain),
        CausalComparison::NotIsomorphic,
        "a 2-event chain has 1 causal edge, a 2-event antichain has 0"
    );

    // Same event count and same causal-edge count, different shape.
    let path = CausalGraph::from_events(vec![
        event(0, &[0], &[10]),
        event(0, &[10], &[11]),
        event(0, &[11], &[12]),
    ]);
    let fork = CausalGraph::from_events(vec![
        event(0, &[0], &[10, 11]),
        event(0, &[10], &[12]),
        event(0, &[11], &[13]),
    ]);
    assert_eq!(
        (path.event_count(), path.causal_edge_count()),
        (fork.event_count(), fork.causal_edge_count()),
        "3 events and 2 causal edges each"
    );
    assert_eq!(
        path.compare(&fork),
        CausalComparison::NotIsomorphic,
        "a path and a fork on the same counts are not isomorphic; edges {:?} against {:?}",
        path.causal_edges().collect::<Vec<_>>(),
        fork.causal_edges().collect::<Vec<_>>()
    );
    assert_eq!(path.compare(&path.clone()), CausalComparison::Isomorphic);

    let empty = CausalGraph::from_events(vec![]);
    assert!(empty.is_empty());
    assert_eq!(empty.event_count(), 0);
    assert_eq!(
        empty.compare(&CausalGraph::default()),
        CausalComparison::Isomorphic
    );

    // The events themselves are readable back in application order.
    assert_eq!(
        chain.events()[1].consumed,
        vec![EdgeId(10), EdgeId(1)],
        "the second event consumed instances 10 and 1"
    );
    assert_eq!(chain.events()[0].produced, vec![EdgeId(10), EdgeId(11)]);
    assert_eq!(chain.events()[0].rule_index, 0);
}

// ===========================================================================
// Arm A — gauge reading
// ===========================================================================

/// The rewrite gauge group's structure constants, the plaquette and total
/// action, and lattice construction.
#[test]
fn gauge_group_action_and_lattice() {
    let group = HypergraphRewriteGroup::new(3);
    assert_eq!(group.num_rules(), 3);
    assert!(
        group.structure_constant_for(0, 0, 0).abs() < 1e-10,
        "f^aab vanishes, got {}",
        group.structure_constant_for(0, 0, 0)
    );
    assert!(
        (group.structure_constant_for(0, 1, 2) - 1.0).abs() < 1e-10,
        "distinct indices mix, got {}",
        group.structure_constant_for(0, 1, 2)
    );
    assert_eq!(HypergraphRewriteGroup::new(4).representation_dim(), 16);

    assert_eq!(HypergraphRewriteGroup::LIE_ALGEBRA_DIM, 3);
    let is_abelian = HypergraphRewriteGroup::IS_ABELIAN;
    assert!(!is_abelian, "rule application order matters");
    assert_eq!(HypergraphRewriteGroup::SPACETIME_DIM, 1);
    assert_eq!(HypergraphRewriteGroup::name(), "HypergraphRewrite");
    assert!(
        (HypergraphRewriteGroup::structure_constant(0, 1, 2) - 1.0).abs() < 1e-10,
        "the trait constant routes to the 3-rule group, got {}",
        HypergraphRewriteGroup::structure_constant(0, 1, 2)
    );

    assert!(
        plaquette_action(1.0).abs() < 1e-10,
        "a flat plaquette has zero action, got {}",
        plaquette_action(1.0)
    );
    assert!(
        (plaquette_action(0.5) - 0.5_f64.ln().abs()).abs() < 1e-10,
        "S = -ln(h), got {}",
        plaquette_action(0.5)
    );
    assert!(plaquette_action(0.0).is_infinite());
    assert!(
        total_action(&[1.0, 1.0, 1.0]).abs() < 1e-10,
        "three flat plaquettes, got {}",
        total_action(&[1.0, 1.0, 1.0])
    );
    assert!(
        (total_action(&[1.0, 0.5, 1.0]) - plaquette_action(0.5)).abs() < 1e-10,
        "total action sums the plaquettes, got {}",
        total_action(&[1.0, 0.5, 1.0])
    );

    let mut line: HypergraphLattice<1> =
        HypergraphLattice::new([5], HypergraphRewriteGroup::new(2), vec![]);
    assert_eq!(line.rules().len(), 0);
    assert_eq!(line.step_count(), 0);
    assert!(
        line.set_state(&[2], Hypergraph::from_edges(vec![vec![0, 1, 2]])),
        "site 2 is inside a length-5 line"
    );
    assert!(
        !line.set_state(&[5], Hypergraph::new()),
        "site 5 is outside a length-5 line"
    );
    assert_eq!(
        line.get_state(&[2]).map(Hypergraph::edge_count),
        Some(1),
        "the stored state is the one that was set"
    );

    let plane: HypergraphLattice<2> = HypergraphLattice::new(
        [4, 4],
        HypergraphRewriteGroup::new(3),
        vec![RewriteRule::wolfram_a_to_bb()],
    );
    assert_eq!(plane.rules().len(), 1);
    assert_eq!(plane.get_state(&[0, 0]).map(Hypergraph::edge_count), None);
}

// ===========================================================================
// Arm B — wasserstein_1 against exhaustive optima
// ===========================================================================

/// Rows of the rational-margin family.
const ROWS: usize = 3;
/// Columns of the rational-margin family.
const COLS: usize = 4;
/// Margin denominator: masses are integer multiples of 1/12.
const DENOM: u32 = 12;

/// A disagreeing rational-margin case: trial index, row margins, column
/// margins, cost matrix, solver value, exact value.
type RationalCase = (usize, [u32; ROWS], [u32; COLS], Vec<Vec<f64>>, f64, f64);

/// Minimum cost over every non-negative integer table with row margins `r`
/// and column margins `c`, in units of 1/`DENOM`.
fn exact_by_contingency_tables(r: &[u32; ROWS], c: &[u32; COLS], cost: &[Vec<f64>]) -> f64 {
    fn walk(
        row: usize,
        col: usize,
        table: &mut [[u32; COLS]; ROWS],
        r: &[u32; ROWS],
        c: &[u32; COLS],
        cost: &[Vec<f64>],
        best: &mut f64,
    ) {
        if row == ROWS {
            for (j, &want) in c.iter().enumerate() {
                let got: u32 = (0..ROWS).map(|i| table[i][j]).sum();
                if got != want {
                    return;
                }
            }
            let total: f64 = (0..ROWS)
                .flat_map(|i| (0..COLS).map(move |j| (i, j)))
                .map(|(i, j)| f64::from(table[i][j]) * cost[i][j])
                .sum();
            *best = best.min(total);
            return;
        }
        let placed: u32 = (0..col).map(|j| table[row][j]).sum();
        let remaining = r[row] - placed;
        if col == COLS - 1 {
            table[row][col] = remaining;
            walk(row + 1, 0, table, r, c, cost, best);
            table[row][col] = 0;
            return;
        }
        for amount in 0..=remaining {
            table[row][col] = amount;
            walk(row, col + 1, table, r, c, cost, best);
        }
        table[row][col] = 0;
    }

    let mut best = f64::INFINITY;
    let mut table = [[0_u32; COLS]; ROWS];
    walk(0, 0, &mut table, r, c, cost, &mut best);
    best / f64::from(DENOM)
}

/// Minimum cost over the k! permutation couplings of uniform 1/k margins.
#[allow(clippy::cast_precision_loss)]
fn exact_by_permutations(cost: &[Vec<f64>]) -> f64 {
    fn walk(depth: usize, perm: &mut Vec<usize>, cost: &[Vec<f64>], best: &mut f64) {
        if depth == 1 {
            let total: f64 = perm.iter().enumerate().map(|(i, &p)| cost[i][p]).sum();
            *best = best.min(total);
            return;
        }
        walk(depth - 1, perm, cost, best);
        for i in 0..depth - 1 {
            if depth.is_multiple_of(2) {
                perm.swap(i, depth - 1);
            } else {
                perm.swap(0, depth - 1);
            }
            walk(depth - 1, perm, cost, best);
        }
    }

    let k = cost.len();
    let mut perm: Vec<usize> = (0..k).collect();
    let mut best = f64::INFINITY;
    walk(k, &mut perm, cost, &mut best);
    best / k as f64
}

/// Integer cost matrix with entries in `0..=9`.
fn random_costs(rng: &mut Lcg, rows: usize, cols: usize) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| {
                    let digit = u32::try_from(rng.next_usize(0, 9))
                        .expect("invariant: next_usize(0, 9) returns at most 9");
                    f64::from(digit)
                })
                .collect()
        })
        .collect()
}

/// 300 seeded 3x4 instances with margins in twelfths — at least one instance
/// carrying a zero-mass support point in a marginal — agree with the
/// contingency-table optimum to 1e-9.
#[test]
fn wasserstein_matches_contingency_table_optimum_on_rational_margins() {
    let mut rng = Lcg::new(0x0387_0001);
    let trials = 300;
    let mut zero_margin_cases = 0;
    let mut worst = 0.0_f64;
    let mut first_bad: Option<RationalCase> = None;

    for trial in 0..trials {
        let mut r = [0_u32; ROWS];
        let mut c = [0_u32; COLS];
        for _ in 0..DENOM {
            r[rng.next_usize(0, ROWS - 1)] += 1;
            c[rng.next_usize(0, COLS - 1)] += 1;
        }
        if r.contains(&0) || c.contains(&0) {
            zero_margin_cases += 1;
        }
        let cost = random_costs(&mut rng, ROWS, COLS);
        let mu: Vec<f64> = r.iter().map(|&x| f64::from(x) / f64::from(DENOM)).collect();
        let nu: Vec<f64> = c.iter().map(|&x| f64::from(x) / f64::from(DENOM)).collect();

        let got = wasserstein_1(&mu, &nu, &cost);
        let want = exact_by_contingency_tables(&r, &c, &cost);
        let delta = (got - want).abs();
        if delta > worst {
            worst = delta;
        }
        if delta > 1e-9 && first_bad.is_none() {
            first_bad = Some((trial, r, c, cost, got, want));
        }
    }

    assert!(
        first_bad.is_none(),
        "solver disagrees with the contingency-table optimum on {trials} 3x4 instances \
         (worst |delta| {worst}); first: {first_bad:?}"
    );
    assert!(
        zero_margin_cases > 0,
        "the family must reach zero-mass margins, got {zero_margin_cases} of {trials}"
    );
}

/// 400 seeded uniform k x k instances for each k in 2..=6 agree with the
/// permutation minimum to 1e-9.
#[test]
#[allow(clippy::cast_precision_loss)]
fn wasserstein_matches_permutation_minimum_on_uniform_margins() {
    let mut rng = Lcg::new(0x0387_0002);
    for k in 2..=6_usize {
        let trials = 400;
        let mut worst = 0.0_f64;
        let mut first_bad: Option<(usize, Vec<Vec<f64>>, f64, f64)> = None;

        for trial in 0..trials {
            let cost = random_costs(&mut rng, k, k);
            let mu = vec![1.0 / k as f64; k];
            let got = wasserstein_1(&mu, &mu, &cost);
            let want = exact_by_permutations(&cost);
            let delta = (got - want).abs();
            if delta > worst {
                worst = delta;
            }
            if delta > 1e-9 && first_bad.is_none() {
                first_bad = Some((trial, cost, got, want));
            }
        }

        assert!(
            first_bad.is_none(),
            "k={k}: solver disagrees with the permutation minimum on {trials} instances \
             (worst |delta| {worst}); first: {first_bad:?}"
        );
    }
}

/// 300 seeded 3x4 rational-margin instances embedded in a 6x8 support with
/// zero-mass rows and columns interleaved — the shape `edge_ollivier_ricci`
/// builds from a union of two neighbourhoods — still agree with the
/// contingency-table optimum of the embedded instance to 1e-9.
#[test]
fn wasserstein_zero_mass_padding_leaves_the_optimum() {
    let mut rng = Lcg::new(0x0387_0003);
    let trials = 300;
    let mut worst = 0.0_f64;
    let mut first_bad: Option<RationalCase> = None;

    for trial in 0..trials {
        let mut r = [0_u32; ROWS];
        let mut c = [0_u32; COLS];
        for _ in 0..DENOM {
            r[rng.next_usize(0, ROWS - 1)] += 1;
            c[rng.next_usize(0, COLS - 1)] += 1;
        }
        let core = random_costs(&mut rng, ROWS, COLS);

        // Even support positions carry the embedded instance; odd positions
        // carry no mass on either marginal.
        let mut cost = random_costs(&mut rng, 2 * ROWS, 2 * COLS);
        for (i, row) in core.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                cost[2 * i][2 * j] = value;
            }
        }
        let mut mu = vec![0.0; 2 * ROWS];
        let mut nu = vec![0.0; 2 * COLS];
        for (i, &mass) in r.iter().enumerate() {
            mu[2 * i] = f64::from(mass) / f64::from(DENOM);
        }
        for (j, &mass) in c.iter().enumerate() {
            nu[2 * j] = f64::from(mass) / f64::from(DENOM);
        }

        let got = wasserstein_1(&mu, &nu, &cost);
        let want = exact_by_contingency_tables(&r, &c, &core);
        let delta = (got - want).abs();
        if delta > worst {
            worst = delta;
        }
        if delta > 1e-9 && first_bad.is_none() {
            first_bad = Some((trial, r, c, cost, got, want));
        }
    }

    assert!(
        first_bad.is_none(),
        "zero-padded solver disagrees with the contingency-table optimum on {trials} \
         instances (worst |delta| {worst}); first: {first_bad:?}"
    );
}

// ===========================================================================
// Arm B — #331 identity and symmetry
// ===========================================================================

/// A distribution of total mass 1 over `n` support points.
fn distribution_strategy(n: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(0.0_f64..=1.0, n).prop_map(|raw| {
        let total: f64 = raw.iter().sum();
        if total <= 0.0 {
            let uniform = 1.0 / raw.len() as f64;
            vec![uniform; raw.len()]
        } else {
            raw.iter().map(|x| x / total).collect()
        }
    })
}

/// A symmetric non-negative ground matrix with a zero diagonal.
fn ground_strategy(n: usize) -> impl Strategy<Value = Vec<Vec<f64>>> {
    prop::collection::vec(0.0_f64..=8.0, n * n).prop_map(move |flat| {
        let mut ground = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let value = flat[i * n + j];
                ground[i][j] = value;
                ground[j][i] = value;
            }
        }
        ground
    })
}

/// A `(mu, nu, ground)` instance at a support size drawn from `2..=6`.
fn instance_strategy() -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<Vec<f64>>)> {
    (2_usize..=6).prop_flat_map(|n| {
        (
            distribution_strategy(n),
            distribution_strategy(n),
            ground_strategy(n),
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// `W₁(mu, mu) = 0` over random distributions of equal mass and random
    /// symmetric non-negative ground matrices with a zero diagonal.
    #[test]
    fn wasserstein_is_zero_on_a_repeated_marginal((mu, _nu, ground) in instance_strategy()) {
        let self_distance = wasserstein_1(&mu, &mu, &ground);
        prop_assert!(
            self_distance.abs() < 1e-9,
            "expected 0.0, got {self_distance} on mu {mu:?} and ground {ground:?}"
        );
    }

    /// `W₁(mu, nu) = W₁(nu, mu)` over the same family.
    #[test]
    fn wasserstein_is_symmetric_in_its_marginals((mu, nu, ground) in instance_strategy()) {
        let forward = wasserstein_1(&mu, &nu, &ground);
        let backward = wasserstein_1(&nu, &mu, &ground);
        prop_assert!(
            (forward - backward).abs() < 1e-9,
            "expected equal, got {forward} against {backward} on mu {mu:?}, nu {nu:?}, \
             ground {ground:?}"
        );
    }
}

// ===========================================================================
// Arm B — Ollivier–Ricci over the exact transport solver
// ===========================================================================

/// Reinterpret a generated undirected graph as a branchial graph at `step`.
fn branchial_from_ungraph(step: usize, g: &UnGraph<(), ()>) -> BranchialGraph {
    let nodes: Vec<MultiwayNodeId> = (0..g.node_count())
        .map(|i| MultiwayNodeId::new(BranchId(i), step))
        .collect();
    let edges = g
        .edge_indices()
        .filter_map(|e| g.edge_endpoints(e))
        .map(|(a, b)| (nodes[a.index()], nodes[b.index()]))
        .collect();
    BranchialGraph { step, nodes, edges }
}

/// The `i`-th pinned topology, as a branchial graph at step 1. Indices match
/// the `#163` set: `0` sparse Erdős–Rényi, `3` 3-regular, `4` path,
/// `5` Petersen, `6` 4x6 grid.
fn topology_fixture(i: usize) -> BranchialGraph {
    let g: UnGraph<(), ()> = match i {
        0 => gnp_random_graph(24, 0.06, Some(0x00C0_FFEE), || (), || ())
            .expect("gnp_random_graph(24, 0.06) has valid arguments"),
        3 => random_regular_graph(20, 3, Some(0x0000_D1CE), || (), || ())
            .expect("random_regular_graph(20, 3) has an even node*degree product"),
        4 => path_graph(Some(25), None, || (), || (), false)
            .expect("path_graph(25) has valid arguments"),
        5 => {
            petersen_graph(5, 2, || (), || ()).expect("petersen_graph(5, 2) is the Petersen graph")
        }
        _ => grid_graph(Some(4), Some(6), None, || (), || (), false)
            .expect("grid_graph(4, 6) has valid arguments"),
    };
    branchial_from_ungraph(1, &g)
}

/// Undirected adjacency lists over `graph.nodes` positions.
fn adjacency(graph: &BranchialGraph) -> Vec<Vec<usize>> {
    let idx_of: HashMap<MultiwayNodeId, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
    for &(a, b) in &graph.edges {
        if let (Some(&ia), Some(&ib)) = (idx_of.get(&a), idx_of.get(&b)) {
            adj[ia].push(ib);
            adj[ib].push(ia);
        }
    }
    adj
}

/// All-pairs unweighted hop distances, `f64::INFINITY` when unreachable.
fn all_pairs_bfs(adj: &[Vec<usize>], n: usize) -> Vec<Vec<f64>> {
    let mut dist = vec![vec![f64::INFINITY; n]; n];
    for (source, row) in dist.iter_mut().enumerate() {
        row[source] = 0.0;
        let mut queue = VecDeque::new();
        queue.push_back(source);
        while let Some(u) = queue.pop_front() {
            let d = row[u] + 1.0;
            for &v in &adj[u] {
                if d < row[v] {
                    row[v] = d;
                    queue.push_back(v);
                }
            }
        }
    }
    dist
}

/// The distinct unordered non-loop endpoint pairs of `graph`, as node indices.
fn edge_pairs(graph: &BranchialGraph) -> Vec<(usize, usize)> {
    let idx_of: HashMap<MultiwayNodeId, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();
    let mut pairs: Vec<(usize, usize)> = graph
        .edges
        .iter()
        .filter_map(|&(a, b)| {
            let ia = *idx_of.get(&a)?;
            let ib = *idx_of.get(&b)?;
            let (u, v) = if ia < ib { (ia, ib) } else { (ib, ia) };
            (u != v).then_some((u, v))
        })
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// W₁ between the uniform neighbour measures of `u` and `v` over the union of
/// the two neighbourhoods — the marginals `edge_ollivier_ricci` builds, with
/// their structural zeros.
#[allow(clippy::cast_precision_loss)]
fn w1_union_support(adj: &[Vec<usize>], dist: &[Vec<f64>], u: usize, v: usize) -> f64 {
    let (nu_list, nv_list) = (&adj[u], &adj[v]);
    let mut support: Vec<usize> = nu_list.clone();
    for &w in nv_list {
        if !support.contains(&w) {
            support.push(w);
        }
    }
    support.sort_unstable();
    let idx: HashMap<usize, usize> = support.iter().enumerate().map(|(i, &w)| (w, i)).collect();

    let mut mu = vec![0.0; support.len()];
    let mut nu = vec![0.0; support.len()];
    for &w in nu_list {
        mu[idx[&w]] = 1.0 / nu_list.len() as f64;
    }
    for &w in nv_list {
        nu[idx[&w]] = 1.0 / nv_list.len() as f64;
    }
    let ground: Vec<Vec<f64>> = support
        .iter()
        .map(|&i| support.iter().map(|&j| dist[i][j]).collect())
        .collect();
    wasserstein_1(&mu, &nu, &ground)
}

/// The same measures with every zero-mass row and column dropped: the
/// neighbourhoods indexed directly, no structural zeros.
#[allow(clippy::cast_precision_loss)]
fn w1_restricted_support(adj: &[Vec<usize>], dist: &[Vec<f64>], u: usize, v: usize) -> f64 {
    let (nu_list, nv_list) = (&adj[u], &adj[v]);
    let mu = vec![1.0 / nu_list.len() as f64; nu_list.len()];
    let nu = vec![1.0 / nv_list.len() as f64; nv_list.len()];
    let ground: Vec<Vec<f64>> = nu_list
        .iter()
        .map(|&i| nv_list.iter().map(|&j| dist[i][j]).collect())
        .collect();
    wasserstein_1(&mu, &nu, &ground)
}

/// Greatest common divisor of two positive integers.
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Minimum of `Σ T_ij · cost_ij` over non-negative integer tables `T` with row
/// margins `r` and column margins `c`, in exact integer arithmetic.
fn exact_integer_transport(r: &[u32], c: &[u32], cost: &[Vec<u32>]) -> u64 {
    /// Row margins and cost matrix, fixed across the walk.
    struct Table<'a> {
        r: &'a [u32],
        cost: &'a [Vec<u32>],
    }

    fn walk(
        t: &Table,
        row: usize,
        col: usize,
        left: u32,
        rem: &mut [u32],
        acc: u64,
        best: &mut u64,
    ) {
        if acc >= *best {
            return;
        }
        if left == 0 {
            if row + 1 == t.r.len() {
                *best = acc;
                return;
            }
            walk(t, row + 1, 0, t.r[row + 1], rem, acc, best);
            return;
        }
        if col == rem.len() {
            return;
        }
        let hi = left.min(rem[col]);
        for take in (0..=hi).rev() {
            rem[col] -= take;
            walk(
                t,
                row,
                col + 1,
                left - take,
                rem,
                acc + u64::from(take) * u64::from(t.cost[row][col]),
                best,
            );
            rem[col] += take;
        }
    }

    let mut rem = c.to_vec();
    let mut best = u64::MAX;
    walk(&Table { r, cost }, 0, 0, r[0], &mut rem, 0, &mut best);
    best
}

/// W₁ between the uniform neighbour measures of `u` and `v`, as the exact
/// minimum over integer transport tables in units of `1/lcm(d_u, d_v)`.
/// `None` when any ground cost between the two neighbourhoods is not a finite
/// whole number.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn w1_exact_integer(adj: &[Vec<usize>], dist: &[Vec<f64>], u: usize, v: usize) -> Option<f64> {
    let (nu_list, nv_list) = (&adj[u], &adj[v]);
    let du = u32::try_from(nu_list.len()).ok()?;
    let dv = u32::try_from(nv_list.len()).ok()?;
    let lcm = du / gcd(du, dv) * dv;

    let mut cost = Vec::with_capacity(nu_list.len());
    for &i in nu_list {
        let mut row = Vec::with_capacity(nv_list.len());
        for &j in nv_list {
            let d = dist[i][j];
            if !d.is_finite() || d.fract() != 0.0 || d < 0.0 {
                return None;
            }
            row.push(d as u32);
        }
        cost.push(row);
    }

    let r = vec![lcm / du; nu_list.len()];
    let c = vec![lcm / dv; nv_list.len()];
    Some(exact_integer_transport(&r, &c, &cost) as f64 / f64::from(lcm))
}

/// Every Petersen edge has κ = -1/3 through `from_branchial`.
///
/// Petersen is cubic with girth 5, so for an edge (u, v) the neighbourhoods
/// are `N(u) = {v, a, b}` and `N(v) = {u, c, d}` with all six labels distinct
/// and `d(a, c) = d(a, d) = d(b, c) = d(b, d) = 2`, `d(v, c) = d(v, d) =
/// d(a, u) = d(b, u) = 1`. The cheapest coupling routes v to c or d and one of
/// a, b to u, giving W₁ = (1 + 1 + 2)/3 = 4/3 and κ = 1 - 4/3.
#[test]
fn petersen_edges_have_curvature_minus_one_third() {
    let bg = topology_fixture(5);
    let curv = OllivierRicciCurvature::from_branchial(&bg);
    let pairs = edge_pairs(&bg);

    assert_eq!(
        pairs.len(),
        15,
        "Petersen has 15 edges, got {}",
        pairs.len()
    );
    let want = -1.0 / 3.0;
    for &(u, v) in &pairs {
        let kappa = curv.sectional_curvature(u, v);
        assert!(
            (kappa - want).abs() < 1e-9,
            "Petersen edge ({u},{v}): kappa {kappa}, want {want}"
        );
    }
    assert_eq!(curv.dimension(), 10, "Petersen has 10 vertices");
    assert_eq!(curv.step(), 1, "the fixture sits at step 1");
    assert!(!curv.is_flat(), "every edge carries kappa {want}");
    for vertex in 0..curv.dimension() {
        let ricci = curv.ricci_curvature(vertex);
        assert!(
            (ricci - want).abs() < 1e-9,
            "vertex {vertex}: Ricci {ricci}, want {want} (every incident edge is {want})"
        );
    }
    assert!(
        (curv.scalar_curvature() - want).abs() < 1e-9,
        "scalar {}, want {want}",
        curv.scalar_curvature()
    );
}

/// On fixtures 0, 3, 4, 5 and 6, the union-support call agrees to 1e-9 with
/// the exact integer transport minimum on every scored edge whose
/// neighbourhood ground costs are finite whole numbers.
#[test]
fn union_support_matches_exact_integer_transport_on_topology_fixtures() {
    for i in [0, 3, 4, 5, 6] {
        let bg = topology_fixture(i);
        let adj = adjacency(&bg);
        let dist = all_pairs_bfs(&adj, bg.nodes.len());

        let mut checked = 0;
        let mut skipped = 0;
        let mut worst = 0.0_f64;
        let mut first_bad: Option<(usize, usize, f64, f64)> = None;

        for (u, v) in edge_pairs(&bg) {
            if !dist[u][v].is_finite() || dist[u][v] == 0.0 {
                continue;
            }
            if adj[u].is_empty() || adj[v].is_empty() {
                continue;
            }
            let Some(exact) = w1_exact_integer(&adj, &dist, u, v) else {
                skipped += 1;
                continue;
            };
            checked += 1;
            let solver = w1_union_support(&adj, &dist, u, v);
            let delta = (solver - exact).abs();
            if delta > worst {
                worst = delta;
            }
            if delta > 1e-9 && first_bad.is_none() {
                first_bad = Some((u, v, solver, exact));
            }
        }

        assert!(
            checked > 0,
            "fixture {i}: no edges checked ({skipped} skipped)"
        );
        assert!(
            first_bad.is_none(),
            "fixture {i}: solver disagrees with the exact integer transport minimum on \
             {checked} checked edges, {skipped} skipped (worst |delta| {worst}); \
             first (u, v, solver, exact): {first_bad:?}"
        );
    }
}

/// On fixtures 0, 3, 4, 5 and 6, the union-support call and the
/// zero-mass-dropped call agree on every scored edge.
#[test]
fn union_support_agrees_with_restricted_support_on_topology_fixtures() {
    for i in [0, 3, 4, 5, 6] {
        let bg = topology_fixture(i);
        let adj = adjacency(&bg);
        let dist = all_pairs_bfs(&adj, bg.nodes.len());

        let mut scored = 0;
        let mut worst = 0.0_f64;
        let mut first_bad: Option<(usize, usize, f64, f64)> = None;

        for (u, v) in edge_pairs(&bg) {
            if !dist[u][v].is_finite() || dist[u][v] == 0.0 {
                continue;
            }
            if adj[u].is_empty() || adj[v].is_empty() {
                continue;
            }
            scored += 1;
            let union = w1_union_support(&adj, &dist, u, v);
            let restricted = w1_restricted_support(&adj, &dist, u, v);
            let delta = (union - restricted).abs();
            if delta > worst {
                worst = delta;
            }
            if delta > 1e-9 && first_bad.is_none() {
                first_bad = Some((u, v, union, restricted));
            }
        }

        assert!(scored > 0, "fixture {i}: no scored edges");
        assert!(
            first_bad.is_none(),
            "fixture {i}: union support disagrees with restricted support on \
             {scored} scored edges (worst |delta| {worst}); first (u, v, union, restricted): \
             {first_bad:?}"
        );
    }
}

// ===========================================================================
// Arm B — the multiway → branchial → curvature pipeline
// ===========================================================================

/// The binary-fork evolution `n ↦ [2n, 2n+1]` to depth 2, whose step-2
/// branchial slice is K₄.
fn binary_fork_evolution() -> MultiwayEvolutionGraph<u32, usize> {
    run_multiway_bfs(1_u32, |&n| vec![(2 * n, 0, 0), (2 * n + 1, 1, 1)], 2, 8)
}

/// `run_multiway_bfs` → `BranchialGraph` → `OllivierRicciCurvature` on a slice
/// whose curvature is computed by hand.
///
/// The step-2 slice is K₄: for an edge `(u, v)` the neighbourhoods are
/// `N(u) = {v, a, b}` and `N(v) = {u, a, b}` at ground distance 1 everywhere
/// off the diagonal, so the cheapest coupling keeps `a` and `b` in place and
/// moves 1/3 from `v` to `u`, giving `W₁ = 1/3` and `κ = 1 − 1/3 = 2/3`. The
/// step-1 slice is K₂: `μ_u = δ_v`, `μ_v = δ_u`, `W₁ = d(u, v) = 1`, `κ = 0`.
#[test]
fn branchial_pipeline_reproduces_hand_computed_curvature() {
    let graph = binary_fork_evolution();

    let root = graph.roots()[0];
    assert_eq!(root.step, 0);
    assert_eq!(root.branch_id, BranchId(0));
    assert_eq!(
        graph
            .get_node(&root)
            .expect("invariant: the root is registered")
            .state,
        1,
        "the root carries the seed state"
    );
    let forward = graph
        .get_forward_edges(&root)
        .expect("invariant: the root forks");
    assert_eq!(forward.len(), 2, "the root forks in two");
    let kinds: Vec<MultiwayEdgeKind> = forward.iter().map(|edge| edge.kind.clone()).collect();
    assert_eq!(
        kinds,
        vec![
            MultiwayEdgeKind::Fork { rule_index: 0 },
            MultiwayEdgeKind::Fork { rule_index: 1 }
        ],
        "both root edges are forks, carrying the step function's rule indices"
    );
    assert_eq!(
        forward
            .iter()
            .map(|edge| edge.to.step)
            .collect::<Vec<usize>>(),
        vec![1, 1],
        "both fork targets sit at step 1"
    );

    let foliation = extract_branchial_foliation(&graph);
    assert_eq!(
        foliation
            .iter()
            .map(|slice| (slice.step, slice.nodes.len(), slice.edges.len()))
            .collect::<Vec<_>>(),
        vec![(0, 1, 0), (1, 2, 1), (2, 4, 6)],
        "the fork doubles the slice each step and every pair shares the root"
    );

    let slice = BranchialGraph::from_evolution_at_step(&graph, 2);
    let curv = OllivierRicciCurvature::from_branchial(&slice);
    assert_eq!(curv.dimension(), 4);
    assert_eq!(curv.step(), 2);
    let want = 2.0 / 3.0;
    for u in 0..4 {
        for v in (u + 1)..4 {
            let kappa = curv.sectional_curvature(u, v);
            assert!(
                (kappa - want).abs() < 1e-9,
                "K4 edge ({u},{v}): kappa {kappa}, want {want}"
            );
        }
        let ricci = curv.ricci_curvature(u);
        assert!(
            (ricci - want).abs() < 1e-9,
            "K4 vertex {u}: Ricci {ricci}, want {want}"
        );
    }
    assert!(
        (curv.scalar_curvature() - want).abs() < 1e-9,
        "K4 scalar {}, want {want}",
        curv.scalar_curvature()
    );
    assert!(!curv.is_flat(), "K4 carries kappa {want}");

    let via_evolution = OllivierRicciCurvature::from_evolution_at_step(&graph, 2);
    assert!(
        (via_evolution.scalar_curvature() - curv.scalar_curvature()).abs() < 1e-12,
        "from_evolution_at_step and from_branchial disagree: {} against {}",
        via_evolution.scalar_curvature(),
        curv.scalar_curvature()
    );

    let full = OllivierFoliation::from_evolution(&graph);
    assert_eq!(full.curvatures.len(), 3, "steps 0, 1 and 2");
    let profile = full.irreducibility_profile();
    assert_eq!(profile.len(), 3);
    assert!(
        profile[0].abs() < 1e-12 && profile[1].abs() < 1e-12,
        "the 1-node and K2 slices are flat, got {profile:?}"
    );
    assert!(
        (profile[2] - want).abs() < 1e-9,
        "the K4 slice has |scalar| {want} and zero edge variance, got {}",
        profile[2]
    );
    assert!(
        !full.is_globally_flat(),
        "the K4 slice is not flat, so the foliation is not globally flat"
    );
    assert!(
        (full.average_irreducibility() - want / 3.0).abs() < 1e-9,
        "expected {} , got {}",
        want / 3.0,
        full.average_irreducibility()
    );

    let k2 =
        OllivierRicciCurvature::from_branchial(&BranchialGraph::from_evolution_at_step(&graph, 1));
    assert!(
        k2.sectional_curvature(0, 1).abs() < 1e-9,
        "K2 edge: kappa {}, want 0.0",
        k2.sectional_curvature(0, 1)
    );
    assert!(k2.is_flat(), "K2 is flat");
}

// ===========================================================================
// Arm B — #332 to_petgraph contracts
// ===========================================================================

/// `BranchialGraph::to_petgraph` keeps a duplicated pair as two edges and
/// drops an edge whose endpoint is not in `nodes`.
#[cfg(feature = "rustworkx")]
#[test]
fn branchial_to_petgraph_keeps_parallel_edges_and_drops_dangling() {
    let a = MultiwayNodeId::new(BranchId(0), 3);
    let b = MultiwayNodeId::new(BranchId(1), 3);
    let c = MultiwayNodeId::new(BranchId(2), 3);
    let unregistered = MultiwayNodeId::new(BranchId(9), 3);

    let graph = BranchialGraph {
        step: 3,
        nodes: vec![a, b, c],
        edges: vec![(a, b), (a, b), (a, unregistered)],
    };
    let (pg, order) = graph.to_petgraph();

    assert_eq!(order.len(), 3, "one index per registered node");
    assert_eq!(pg.node_count(), 3, "three registered nodes");
    assert_eq!(
        pg.edge_count(),
        2,
        "both (a, b) entries survive and (a, unregistered) is dropped"
    );
}

/// `MultiwayEvolutionGraph::to_petgraph` keeps parallel parent → child edges
/// and drops an edge whose target is not a registered node.
#[cfg(feature = "rustworkx")]
#[test]
fn multiway_to_petgraph_keeps_parallel_edges_and_drops_dangling() {
    let mut graph: MultiwayEvolutionGraph<u32, ()> = MultiwayEvolutionGraph::new();
    let root = graph.add_root(0);
    let child = graph.add_sequential_step(root, 1, ());

    // Two merge edges over the same ordered pair, and one to a node that was
    // never registered.
    graph.add_merge_edge(child, root, ());
    graph.add_merge_edge(child, root, ());
    graph.add_merge_edge(child, MultiwayNodeId::new(BranchId(9), 7), ());

    assert_eq!(graph.node_count(), 2, "only the root and its child exist");
    assert_eq!(
        graph.edge_count(),
        4,
        "one sequential edge and three merge edges are recorded"
    );

    let (pg, order) = graph.to_petgraph();
    assert_eq!(order, vec![root, child], "ascending step order");
    assert_eq!(pg.node_count(), 2);
    assert_eq!(
        pg.edge_count(),
        3,
        "the sequential edge and both parallel merge edges survive; the edge to the \
         unregistered node is dropped"
    );
}
