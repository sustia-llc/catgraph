//! Colour refinement and backtracking isomorphism search over coloured
//! directed graphs.
//!
//! [`CausalGraph::compare`](super::causal_graph::CausalGraph::compare) and
//! [`Hypergraph::compare`](super::hypergraph::Hypergraph::compare) both encode
//! their question as a [`Digraph`] and settle it here.

use super::causal_graph::{CausalComparison, CausalGraph};
use std::collections::HashSet;

/// A directed graph over `0..colours.len()`, each node carrying an initial
/// colour.
pub(super) struct Digraph {
    /// Initial colour of each node, indexed by node.
    pub colours: Vec<u64>,

    /// Arcs as `(source, target)` node pairs.
    pub edges: Vec<(usize, usize)>,
}

impl Digraph {
    /// The predecessor and successor lists of each node.
    fn adjacency(&self) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
        let n = self.colours.len();
        let mut predecessors = vec![Vec::new(); n];
        let mut successors = vec![Vec::new(); n];
        for &(source, target) in &self.edges {
            successors[source].push(target);
            predecessors[target].push(source);
        }
        (predecessors, successors)
    }
}

/// Colour refinement seeded from `initial`: each round replaces a node's colour
/// by its old colour together with the sorted colour multisets of its
/// predecessors and successors, stopping when the partition stops getting
/// finer.
pub(super) fn refine(
    initial: &[u64],
    predecessors: &[Vec<usize>],
    successors: &[Vec<usize>],
) -> Vec<u64> {
    let n = initial.len();
    let mut colours = initial.to_vec();

    let mut distinct_initial = colours.clone();
    distinct_initial.sort_unstable();
    distinct_initial.dedup();
    let mut classes = distinct_initial.len();

    for _ in 0..n {
        let mut signatures: Vec<(u64, Vec<u64>, Vec<u64>)> = Vec::with_capacity(n);
        for v in 0..n {
            let mut before: Vec<u64> = predecessors[v].iter().map(|&u| colours[u]).collect();
            let mut after: Vec<u64> = successors[v].iter().map(|&u| colours[u]).collect();
            before.sort_unstable();
            after.sort_unstable();
            signatures.push((colours[v], before, after));
        }

        let mut distinct = signatures.clone();
        distinct.sort();
        distinct.dedup();
        if distinct.len() == classes {
            break;
        }

        classes = distinct.len();
        colours = signatures
            .iter()
            .map(|signature| {
                let index = distinct
                    .binary_search(signature)
                    .expect("invariant: every signature is in the deduplicated signature list");
                index as u64
            })
            .collect();
    }

    colours
}

/// The stable refinement colours of `graph` read on its own.
pub(super) fn refine_digraph(graph: &Digraph) -> Vec<u64> {
    let (predecessors, successors) = graph.adjacency();
    refine(&graph.colours, &predecessors, &successors)
}

/// Compares two coloured digraphs up to a colour-preserving, arc-preserving
/// bijection.
///
/// Node count, arc count, and a colour refinement on the disjoint union screen
/// out some negatives; a backtracking search over the refined colour classes
/// settles every case they leave, and reports [`CausalComparison::Undecided`]
/// after [`CausalGraph::MAX_SEARCH_STEPS`] candidate assignments.
pub(super) fn compare_digraphs(left: &Digraph, right: &Digraph) -> CausalComparison {
    let n = left.colours.len();
    if n != right.colours.len() || left.edges.len() != right.edges.len() {
        return CausalComparison::NotIsomorphic;
    }
    if n == 0 {
        return CausalComparison::Isomorphic;
    }

    // Refine on the disjoint union so the two colourings are comparable.
    let mut predecessors = vec![Vec::new(); 2 * n];
    let mut successors = vec![Vec::new(); 2 * n];
    for &(source, target) in &left.edges {
        successors[source].push(target);
        predecessors[target].push(source);
    }
    for &(source, target) in &right.edges {
        successors[n + source].push(n + target);
        predecessors[n + target].push(n + source);
    }
    let mut initial = Vec::with_capacity(2 * n);
    initial.extend_from_slice(&left.colours);
    initial.extend_from_slice(&right.colours);
    let colours = refine(&initial, &predecessors, &successors);

    let mut left_colours: Vec<u64> = colours[..n].to_vec();
    let mut right_colours: Vec<u64> = colours[n..].to_vec();
    left_colours.sort_unstable();
    right_colours.sort_unstable();
    if left_colours != right_colours {
        return CausalComparison::NotIsomorphic;
    }

    let candidates: Vec<Vec<usize>> = (0..n)
        .map(|v| (0..n).filter(|&w| colours[v] == colours[n + w]).collect())
        .collect();
    let order = search_order(n, &left.edges, &candidates);

    let left_edges: HashSet<(usize, usize)> = left.edges.iter().copied().collect();
    let right_edges: HashSet<(usize, usize)> = right.edges.iter().copied().collect();

    let mut mapping = vec![usize::MAX; n];
    let mut used = vec![false; n];
    let mut steps = 0usize;

    match backtrack(
        0,
        &order,
        &candidates,
        &left_edges,
        &right_edges,
        &mut mapping,
        &mut used,
        &mut steps,
    ) {
        Some(true) => CausalComparison::Isomorphic,
        Some(false) => CausalComparison::NotIsomorphic,
        None => CausalComparison::Undecided,
    }
}

/// The order [`backtrack`] places the `n` left nodes in: fewest candidates
/// first, and after the first node the fewest-candidate node adjacent to one
/// already placed, falling back to the fewest-candidate unplaced node when no
/// unplaced node is adjacent to the placed set.
fn search_order(n: usize, edges: &[(usize, usize)], candidates: &[Vec<usize>]) -> Vec<usize> {
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(source, target) in edges {
        neighbours[source].push(target);
        neighbours[target].push(source);
    }

    let mut by_candidates: Vec<usize> = (0..n).collect();
    by_candidates.sort_by_key(|&v| candidates[v].len());

    let mut placed = vec![false; n];
    let mut adjacent = vec![false; n];
    let mut order = Vec::with_capacity(n);

    for _ in 0..n {
        let next = by_candidates
            .iter()
            .copied()
            .find(|&v| !placed[v] && adjacent[v])
            .or_else(|| by_candidates.iter().copied().find(|&v| !placed[v]))
            .expect("invariant: fewer than n nodes are placed on each of the n rounds");

        placed[next] = true;
        order.push(next);
        for &u in &neighbours[next] {
            adjacent[u] = true;
        }
    }

    order
}

/// Extends a partial node bijection.
///
/// `Some(true)` = a full bijection was found, `Some(false)` = the candidate
/// space was exhausted, `None` = the step budget was reached.
#[allow(clippy::too_many_arguments)]
fn backtrack(
    depth: usize,
    order: &[usize],
    candidates: &[Vec<usize>],
    left_edges: &HashSet<(usize, usize)>,
    right_edges: &HashSet<(usize, usize)>,
    mapping: &mut [usize],
    used: &mut [bool],
    steps: &mut usize,
) -> Option<bool> {
    if depth == order.len() {
        return Some(true);
    }

    let v = order[depth];
    for &w in &candidates[v] {
        *steps += 1;
        if *steps > CausalGraph::MAX_SEARCH_STEPS {
            return None;
        }
        if used[w] {
            continue;
        }

        let consistent = order[..depth].iter().all(|&u| {
            let image = mapping[u];
            left_edges.contains(&(u, v)) == right_edges.contains(&(image, w))
                && left_edges.contains(&(v, u)) == right_edges.contains(&(w, image))
        });
        if !consistent {
            continue;
        }

        mapping[v] = w;
        used[w] = true;
        let deeper = backtrack(
            depth + 1,
            order,
            candidates,
            left_edges,
            right_edges,
            mapping,
            used,
            steps,
        );
        mapping[v] = usize::MAX;
        used[w] = false;

        match deeper {
            Some(true) => return Some(true),
            None => return None,
            Some(false) => {}
        }
    }

    Some(false)
}
