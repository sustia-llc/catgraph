//! Ollivier-Ricci curvature over the exact transport solver (issue #387).
//!
//! `edge_ollivier_ricci` scores an edge on the *union* of the two endpoint
//! neighbourhoods, so both marginals it hands `wasserstein_1` carry zero-mass
//! support points. These tests pin the Petersen graph's edge curvature,
//! agreement between the union-support call and the exact minimum over integer
//! transport tables, and agreement between the union-support call and the same
//! instance with its zero-mass rows and columns dropped.
//!
//! Fixtures are the seeded `#163` topologies; the helpers mirror
//! `tests/branchial_analysis.rs`.

use std::collections::{HashMap, VecDeque};

use catgraph_physics::multiway::{
    BranchId, BranchialGraph, DiscreteCurvature, MultiwayNodeId, OllivierRicciCurvature,
    wasserstein_1,
};
use rustworkx_core::generators::{
    gnp_random_graph, grid_graph, path_graph, petersen_graph, random_regular_graph,
};
use rustworkx_core::petgraph::graph::UnGraph;

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
