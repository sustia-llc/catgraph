//! Seeded branchial topology fixtures shared by the `multiway` unit tests.
//!
//! The generator calls and seeds match the `#163` fixture set in
//! `tests/branchial_analysis.rs`, so both sets index the same seven graphs.

use std::collections::HashMap;

use rustworkx_core::generators::{
    barabasi_albert_graph, gnp_random_graph, grid_graph, path_graph, petersen_graph,
    random_regular_graph,
};
use rustworkx_core::petgraph::graph::UnGraph;

use super::branchial::BranchialGraph;
use super::evolution_graph::{BranchId, MultiwayNodeId};

/// Number of indices [`topology_fixture`] distinguishes.
pub(crate) const TOPOLOGY_FIXTURE_COUNT: usize = 7;

/// Reinterprets an undirected graph as a branchial graph at `step`.
///
/// Generated node `i` becomes `MultiwayNodeId::new(BranchId(i), step)`;
/// generated edges become common-ancestor edges verbatim.
pub(crate) fn branchial_from_ungraph(step: usize, g: &UnGraph<(), ()>) -> BranchialGraph {
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

/// The `i`-th pinned topology, as a branchial graph at step 1.
///
/// Indices are stable: `0` sparse Erdős–Rényi (usually disconnected),
/// `1` dense Erdős–Rényi, `2` Barabási–Albert (scale-free hubs),
/// `3` 3-regular, `4` path, `5` Petersen, `6` 4×6 grid.
pub(crate) fn topology_fixture(i: usize) -> BranchialGraph {
    let g: UnGraph<(), ()> = match i {
        0 => gnp_random_graph(24, 0.06, Some(0x00C0_FFEE), || (), || ())
            .expect("gnp_random_graph(24, 0.06) has valid arguments"),
        1 => gnp_random_graph(30, 0.5, Some(0x0BAD_C0DE), || (), || ())
            .expect("gnp_random_graph(30, 0.5) has valid arguments"),
        2 => barabasi_albert_graph(40, 3, Some(0x0000_5EED), None, || (), || ())
            .expect("barabasi_albert_graph(40, 3) has valid arguments"),
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

/// All-pairs distance summary of [`topology_fixture`] `i` at index `i`: `(sum
/// of every finite entry, count of infinite entries, largest finite entry)`.
pub(crate) const DISTANCE_SUMMARY: [(f64, usize, f64); TOPOLOGY_FIXTURE_COUNT] = [
    (1112.0, 246, 8.0),
    (1322.0, 0, 2.0),
    (3400.0, 0, 4.0),
    (1114.0, 0, 6.0),
    (5200.0, 0, 24.0),
    (150.0, 0, 2.0),
    (1840.0, 0, 8.0),
];

/// Asserts two distance matrices are equal, reporting the first differing
/// entry as `(row, column)` with both values, or the two shapes.
#[cfg_attr(not(feature = "rustworkx"), allow(dead_code))]
pub(crate) fn assert_same_matrix(got: &[Vec<f64>], want: &[Vec<f64>], label: &str) {
    assert_eq!(
        got.len(),
        want.len(),
        "{label}: row count {} vs {}",
        got.len(),
        want.len()
    );
    for (i, (got_row, want_row)) in got.iter().zip(want).enumerate() {
        assert_eq!(
            got_row.len(),
            want_row.len(),
            "{label}: row {i} length {} vs {}",
            got_row.len(),
            want_row.len()
        );
        for (j, (&g, &w)) in got_row.iter().zip(want_row).enumerate() {
            assert!(g == w, "{label}: first difference at ({i},{j}): {g} vs {w}");
        }
    }
}

/// Undirected adjacency lists over `graph.nodes` positions: every edge
/// contributes both directions, and an edge with an endpoint outside
/// `graph.nodes` contributes nothing.
pub(crate) fn adjacency(graph: &BranchialGraph) -> Vec<Vec<usize>> {
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
