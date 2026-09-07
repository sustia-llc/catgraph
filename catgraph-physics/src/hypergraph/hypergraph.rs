//! Hypergraph data structure for Wolfram Physics model.
//!
//! A [`Hypergraph`] stores a set of vertices (identified by `usize`) and a
//! collection of [`Hyperedge`]s. Vertices are tracked in a `BTreeSet` and
//! auto-registered when hyperedges are added. Supports pattern matching
//! via [`Hypergraph::find_matches`] for DPO rewrite rule application,
//! isomorphism comparison, and compaction (vertex ID renumbering).

use super::causal_graph::CausalComparison;
use super::hyperedge::Hyperedge;
use super::isomorphism::{Digraph, compare_digraphs, refine_digraph};
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

/// Hypergraph: `usize` vertices, ordered hyperedges; self-loops and parallel
/// hyperedges allowed.
///
/// # Example
///
/// ```rust
/// use catgraph_physics::hypergraph::Hypergraph;
///
/// let mut graph = Hypergraph::new();
///
/// // Add vertices (auto-created when adding hyperedges)
/// graph.add_hyperedge(vec![0, 1, 2]);
/// graph.add_hyperedge(vec![2, 3]);
///
/// assert_eq!(graph.vertex_count(), 4);
/// assert_eq!(graph.edge_count(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct Hypergraph {
    /// Set of vertices (tracks which vertex IDs are in use).
    vertices: BTreeSet<usize>,

    /// Collection of hyperedges.
    edges: Vec<Hyperedge>,

    /// Next available vertex ID for auto-generation.
    next_vertex_id: usize,
}

impl Hypergraph {
    /// Creates a new empty hypergraph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vertices: BTreeSet::new(),
            edges: Vec::new(),
            next_vertex_id: 0,
        }
    }

    /// Creates a hypergraph with the given initial capacity for edges.
    #[must_use]
    pub fn with_capacity(edge_capacity: usize) -> Self {
        Self {
            vertices: BTreeSet::new(),
            edges: Vec::with_capacity(edge_capacity),
            next_vertex_id: 0,
        }
    }

    /// Creates a hypergraph from a list of hyperedges.
    ///
    /// # Example
    ///
    /// ```rust
    /// use catgraph_physics::hypergraph::Hypergraph;
    ///
    /// let graph = Hypergraph::from_edges(vec![
    ///     vec![0, 1, 2],
    ///     vec![2, 3, 4],
    /// ]);
    /// assert_eq!(graph.vertex_count(), 5);
    /// assert_eq!(graph.edge_count(), 2);
    /// ```
    pub fn from_edges<I, E>(edges: I) -> Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Hyperedge>,
    {
        let mut graph = Self::new();
        for edge in edges {
            graph.add_hyperedge_obj(edge.into());
        }
        graph
    }

    // ========================================================================
    // Vertex Operations
    // ========================================================================

    /// Returns the number of vertices.
    #[inline]
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns true if the hypergraph contains the given vertex.
    #[inline]
    #[must_use]
    pub fn contains_vertex(&self, v: usize) -> bool {
        self.vertices.contains(&v)
    }

    /// Returns an iterator over all vertices.
    pub fn vertices(&self) -> impl Iterator<Item = usize> + '_ {
        self.vertices.iter().copied()
    }

    /// Adds a new vertex and returns its ID.
    ///
    /// If `id` is provided, uses that ID (and updates `next_vertex_id` if needed).
    /// Otherwise, generates a new ID.
    pub fn add_vertex(&mut self, id: Option<usize>) -> usize {
        let v = id.unwrap_or_else(|| {
            let v = self.next_vertex_id;
            self.next_vertex_id += 1;
            v
        });

        if v >= self.next_vertex_id {
            self.next_vertex_id = v + 1;
        }

        self.vertices.insert(v);
        v
    }

    /// Removes a vertex and all hyperedges containing it.
    ///
    /// Returns true if the vertex existed.
    pub fn remove_vertex(&mut self, v: usize) -> bool {
        if !self.vertices.remove(&v) {
            return false;
        }

        // Remove all edges containing this vertex
        self.edges.retain(|e| !e.contains(&v));
        true
    }

    // ========================================================================
    // Edge Operations
    // ========================================================================

    /// Returns the number of hyperedges.
    #[inline]
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns true if the hypergraph has no edges.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Returns an iterator over all hyperedges.
    pub fn edges(&self) -> impl Iterator<Item = &Hyperedge> {
        self.edges.iter()
    }

    /// Returns a reference to the edge at the given index.
    #[must_use]
    pub fn get_edge(&self, index: usize) -> Option<&Hyperedge> {
        self.edges.get(index)
    }

    /// Adds a hyperedge from a vector of vertices.
    ///
    /// Automatically registers any new vertices.
    ///
    /// # Returns
    ///
    /// The index of the added edge.
    pub fn add_hyperedge(&mut self, vertices: Vec<usize>) -> usize {
        self.add_hyperedge_obj(Hyperedge::new(vertices))
    }

    /// Adds a hyperedge object.
    ///
    /// # Returns
    ///
    /// The index of the added edge.
    pub fn add_hyperedge_obj(&mut self, edge: Hyperedge) -> usize {
        // Register all vertices
        for &v in edge.vertices() {
            if v >= self.next_vertex_id {
                self.next_vertex_id = v + 1;
            }
            self.vertices.insert(v);
        }

        let index = self.edges.len();
        self.edges.push(edge);
        index
    }

    /// Removes the hyperedge at the given index.
    ///
    /// # Returns
    ///
    /// The removed edge, or None if index was out of bounds.
    pub fn remove_edge(&mut self, index: usize) -> Option<Hyperedge> {
        if index < self.edges.len() {
            Some(self.edges.remove(index))
        } else {
            None
        }
    }

    /// Removes all hyperedges matching the given predicate.
    ///
    /// # Returns
    ///
    /// The number of edges removed.
    pub fn remove_edges_where<F>(&mut self, predicate: F) -> usize
    where
        F: Fn(&Hyperedge) -> bool,
    {
        let initial_count = self.edges.len();
        self.edges.retain(|e| !predicate(e));
        initial_count - self.edges.len()
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Returns all hyperedges containing the given vertex.
    #[must_use]
    pub fn edges_containing(&self, v: usize) -> Vec<usize> {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.contains(&v))
            .map(|(i, _)| i)
            .collect()
    }

    /// Returns the degree of a vertex (number of hyperedges containing it).
    #[must_use]
    pub fn degree(&self, v: usize) -> usize {
        self.edges.iter().filter(|e| e.contains(&v)).count()
    }

    /// Returns the neighbors of a vertex (vertices sharing a hyperedge).
    #[must_use]
    pub fn neighbors(&self, v: usize) -> BTreeSet<usize> {
        let mut neighbors = BTreeSet::new();
        for edge in &self.edges {
            if edge.contains(&v) {
                for &u in edge.vertices() {
                    if u != v {
                        neighbors.insert(u);
                    }
                }
            }
        }
        neighbors
    }

    /// Finds all hyperedges matching the given pattern.
    ///
    /// The pattern is matched structurally (same arity, vertex mapping exists).
    ///
    /// # Returns
    ///
    /// A vector of (`edge_index`, `vertex_mapping`) pairs.
    #[must_use]
    pub fn find_matches(&self, pattern: &Hyperedge) -> Vec<(usize, HashMap<usize, usize>)> {
        let mut matches = Vec::new();

        for (i, edge) in self.edges.iter().enumerate() {
            if edge.arity() == pattern.arity() {
                // Check if there's a valid mapping from pattern vertices to edge vertices
                let pattern_verts: Vec<_> = pattern.vertices().to_vec();
                let edge_verts: Vec<_> = edge.vertices().to_vec();

                let mut mapping = HashMap::new();
                let mut valid = true;

                for (pv, ev) in pattern_verts.iter().zip(edge_verts.iter()) {
                    if let Some(&existing) = mapping.get(pv) {
                        if existing != *ev {
                            valid = false;
                            break;
                        }
                    } else {
                        mapping.insert(*pv, *ev);
                    }
                }

                if valid {
                    matches.push((i, mapping));
                }
            }
        }

        matches
    }

    // ========================================================================
    // Utility Operations
    // ========================================================================

    /// Clears all edges but keeps vertices.
    pub fn clear_edges(&mut self) {
        self.edges.clear();
    }

    /// Clears everything (vertices and edges).
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.edges.clear();
        self.next_vertex_id = 0;
    }

    /// Creates a deep clone with remapped vertex IDs starting from 0.
    ///
    /// # Panics
    ///
    /// Panics if a hyperedge references a vertex not present in the graph.
    #[must_use]
    pub fn compact(&self) -> (Self, HashMap<usize, usize>) {
        let mut old_to_new = HashMap::new();

        for (new_id, &v) in self.vertices.iter().enumerate() {
            old_to_new.insert(v, new_id);
        }

        let mut compact = Hypergraph::with_capacity(self.edges.len());
        for edge in &self.edges {
            let new_edge = edge.rename_vertices(|v| *old_to_new.get(&v).unwrap());
            compact.add_hyperedge_obj(new_edge);
        }

        (compact, old_to_new)
    }

    /// Computes a fingerprint invariant under vertex relabelling and edge
    /// reordering.
    ///
    /// Hashes the vertex count, the edge count, the sorted arity multiset and
    /// the sorted multiset of the stable colour-refinement colours of the
    /// incidence digraph, so a relabelled or edge-reordered copy carries the
    /// same fingerprint. Equal fingerprints do not imply isomorphism.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();

        self.vertices.len().hash(&mut hasher);
        self.edges.len().hash(&mut hasher);

        let mut arities: Vec<_> = self.edges.iter().map(Hyperedge::arity).collect();
        arities.sort_unstable();
        arities.hash(&mut hasher);

        let mut colours = refine_digraph(&self.incidence_digraph());
        colours.sort_unstable();
        colours.hash(&mut hasher);

        hasher.finish()
    }

    /// Compares this hypergraph with `other` up to isomorphism.
    ///
    /// An isomorphism is a bijection of the vertices carrying this
    /// hypergraph's hyperedge multiset onto `other`'s with positions
    /// preserved. Vertex count, edge count, the sorted degree sequence and the
    /// sorted arity multiset screen out some negatives; a colour refinement
    /// and backtracking search over the incidence digraphs settles the rest,
    /// and reports [`CausalComparison::Undecided`] after
    /// [`CausalGraph::MAX_SEARCH_STEPS`](super::causal_graph::CausalGraph::MAX_SEARCH_STEPS)
    /// candidate assignments.
    #[must_use]
    pub fn compare(&self, other: &Hypergraph) -> CausalComparison {
        if self.vertex_count() != other.vertex_count() || self.edge_count() != other.edge_count() {
            return CausalComparison::NotIsomorphic;
        }

        let mut self_degrees: Vec<_> = self.vertices.iter().map(|&v| self.degree(v)).collect();
        let mut other_degrees: Vec<_> = other.vertices.iter().map(|&v| other.degree(v)).collect();
        self_degrees.sort_unstable();
        other_degrees.sort_unstable();
        if self_degrees != other_degrees {
            return CausalComparison::NotIsomorphic;
        }

        let mut self_arities: Vec<_> = self.edges.iter().map(Hyperedge::arity).collect();
        let mut other_arities: Vec<_> = other.edges.iter().map(Hyperedge::arity).collect();
        self_arities.sort_unstable();
        other_arities.sort_unstable();
        if self_arities != other_arities {
            return CausalComparison::NotIsomorphic;
        }

        compare_digraphs(&self.incidence_digraph(), &other.incidence_digraph())
    }

    /// Returns true when [`Self::compare`] reports
    /// [`CausalComparison::Isomorphic`].
    ///
    /// [`CausalComparison::Undecided`] returns false.
    #[must_use]
    pub fn is_isomorphic_to(&self, other: &Hypergraph) -> bool {
        matches!(self.compare(other), CausalComparison::Isomorphic)
    }

    /// The typed incidence digraph: one node per vertex, per hyperedge and per
    /// hyperedge slot, coloured `0`, `1` and `2`.
    ///
    /// A hyperedge node of arity at least one points at its first slot, slot
    /// `i` at slot `i + 1` up to the last, and slot `i` at the vertex node the
    /// hyperedge holds at position `i`.
    fn incidence_digraph(&self) -> Digraph {
        let vertex_index: HashMap<usize, usize> = self
            .vertices
            .iter()
            .enumerate()
            .map(|(index, &v)| (v, index))
            .collect();

        let mut colours = vec![0u64; self.vertices.len()];
        let mut edges = Vec::new();
        let mut next = self.vertices.len();

        for edge in &self.edges {
            let edge_node = next;
            next += 1;
            colours.push(1);

            let mut previous = edge_node;
            for &v in edge.vertices() {
                let slot = next;
                next += 1;
                colours.push(2);

                edges.push((previous, slot));
                edges.push((
                    slot,
                    *vertex_index
                        .get(&v)
                        .expect("invariant: every hyperedge vertex is a registered vertex"),
                ));
                previous = slot;
            }
        }

        Digraph { colours, edges }
    }
}

impl Default for Hypergraph {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Hypergraph {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices && self.edges == other.edges
    }
}

impl Eq for Hypergraph {}

impl std::fmt::Display for Hypergraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Hypergraph({} vertices, {} edges)",
            self.vertex_count(),
            self.edge_count()
        )?;
        for (i, edge) in self.edges.iter().enumerate() {
            writeln!(f, "  [{i}]: {edge}")?;
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use catgraph_testutil::Lcg;

    #[test]
    fn test_hypergraph_new() {
        let graph = Hypergraph::new();
        assert_eq!(graph.vertex_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.is_empty());
    }

    #[test]
    fn test_hypergraph_add_edge() {
        let mut graph = Hypergraph::new();
        graph.add_hyperedge(vec![0, 1, 2]);

        assert_eq!(graph.vertex_count(), 3);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.contains_vertex(0));
        assert!(graph.contains_vertex(1));
        assert!(graph.contains_vertex(2));
    }

    #[test]
    fn test_hypergraph_from_edges() {
        let graph = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2], vec![2, 0]]);

        assert_eq!(graph.vertex_count(), 3);
        assert_eq!(graph.edge_count(), 3);
    }

    #[test]
    fn test_hypergraph_remove_edge() {
        let mut graph = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2]]);

        let removed = graph.remove_edge(0);
        assert!(removed.is_some());
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_hypergraph_neighbors() {
        let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![2, 3, 4]]);

        let neighbors_0 = graph.neighbors(0);
        assert!(neighbors_0.contains(&1));
        assert!(neighbors_0.contains(&2));
        assert!(!neighbors_0.contains(&3));

        let neighbors_2 = graph.neighbors(2);
        assert!(neighbors_2.contains(&0));
        assert!(neighbors_2.contains(&1));
        assert!(neighbors_2.contains(&3));
        assert!(neighbors_2.contains(&4));
    }

    #[test]
    fn test_hypergraph_degree() {
        let graph = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![2, 3], vec![2, 4]]);

        assert_eq!(graph.degree(0), 1);
        assert_eq!(graph.degree(2), 3); // In all three edges
        assert_eq!(graph.degree(3), 1);
    }

    #[test]
    fn test_hypergraph_compact() {
        let mut graph = Hypergraph::new();
        graph.add_hyperedge(vec![5, 10, 15]);
        graph.add_hyperedge(vec![10, 20]);

        let (compact, mapping) = graph.compact();

        assert_eq!(compact.vertex_count(), 4);
        assert!(compact.vertices().all(|v| v < 4));

        // Check mapping
        assert_eq!(mapping.len(), 4);
    }

    #[test]
    fn test_hypergraph_fingerprint() {
        let g1 = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2]]);
        let g2 = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2]]);
        let g3 = Hypergraph::from_edges(vec![vec![0, 1], vec![2, 3]]);

        assert_eq!(g1.fingerprint(), g2.fingerprint());
        assert_ne!(g1.fingerprint(), g3.fingerprint());
    }

    #[test]
    fn test_hypergraph_edges_containing() {
        let graph = Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2], vec![0, 2]]);

        let edges_with_1 = graph.edges_containing(1);
        assert_eq!(edges_with_1.len(), 2);
        assert!(edges_with_1.contains(&0));
        assert!(edges_with_1.contains(&1));
    }

    #[test]
    fn test_hypergraph_isomorphic() {
        let g1 = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]);
        let g2 = Hypergraph::from_edges(vec![vec![10, 11, 12], vec![11, 12, 13]]);

        assert!(g1.is_isomorphic_to(&g2));

        let g3 = Hypergraph::from_edges(vec![vec![0, 1, 2], vec![3, 4, 5]]); // Different structure
        assert!(!g1.is_isomorphic_to(&g3));
    }

    /// The 6-cycle against two triangles.
    fn six_cycle_and_two_triangles() -> (Hypergraph, Hypergraph) {
        (
            Hypergraph::from_edges(vec![
                vec![0, 1],
                vec![1, 2],
                vec![2, 3],
                vec![3, 4],
                vec![4, 5],
                vec![5, 0],
            ]),
            Hypergraph::from_edges(vec![
                vec![0, 1],
                vec![1, 2],
                vec![2, 0],
                vec![3, 4],
                vec![4, 5],
                vec![5, 3],
            ]),
        )
    }

    /// `compare` on pairs the count / degree / arity prefilter leaves open:
    /// three non-isomorphic pairs, four isomorphic ones, and two the prefilter
    /// itself rejects.
    #[test]
    fn compare_settles_the_pairs_the_prefilter_leaves_open() {
        let (six_cycle, two_triangles) = six_cycle_and_two_triangles();
        let separated: Vec<(&str, Hypergraph, Hypergraph)> = vec![
            ("6-cycle against two triangles", six_cycle, two_triangles),
            (
                "{{0,1},{1,2}} against {{0,1},{2,1}}",
                Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2]]),
                Hypergraph::from_edges(vec![vec![0, 1], vec![2, 1]]),
            ),
            (
                "{{0,0,1}} against {{0,1,1}}",
                Hypergraph::from_edges(vec![vec![0, 0, 1]]),
                Hypergraph::from_edges(vec![vec![0, 1, 1]]),
            ),
        ];
        for (name, left, right) in &separated {
            assert_eq!(
                left.compare(right),
                CausalComparison::NotIsomorphic,
                "{name}: expected NotIsomorphic, got {:?}",
                left.compare(right)
            );
            assert!(
                !left.is_isomorphic_to(right),
                "{name}: expected is_isomorphic_to false, got true"
            );
        }

        let mut isolated = Hypergraph::from_edges(vec![vec![0]]);
        isolated.add_vertex(Some(7));
        let matched: Vec<(&str, Hypergraph, Hypergraph)> = vec![
            (
                "{{0,1,2},{1,2,3}} against its +10 relabelling",
                Hypergraph::from_edges(vec![vec![0, 1, 2], vec![1, 2, 3]]),
                Hypergraph::from_edges(vec![vec![10, 11, 12], vec![11, 12, 13]]),
            ),
            (
                "{{0,1,2}} against {{2,1,0}}",
                Hypergraph::from_edges(vec![vec![0, 1, 2]]),
                Hypergraph::from_edges(vec![vec![2, 1, 0]]),
            ),
            (
                "{{0,1},{1,2}} against its edge-order swap",
                Hypergraph::from_edges(vec![vec![0, 1], vec![1, 2]]),
                Hypergraph::from_edges(vec![vec![1, 2], vec![0, 1]]),
            ),
            (
                "two empty hypergraphs",
                Hypergraph::new(),
                Hypergraph::new(),
            ),
        ];
        for (name, left, right) in &matched {
            assert_eq!(
                left.compare(right),
                CausalComparison::Isomorphic,
                "{name}: expected Isomorphic, got {:?}",
                left.compare(right)
            );
            assert!(
                left.is_isomorphic_to(right),
                "{name}: expected is_isomorphic_to true, got false"
            );
        }

        let one_edge = Hypergraph::from_edges(vec![vec![0]]);
        assert_eq!(
            isolated.compare(&one_edge),
            CausalComparison::NotIsomorphic,
            "an extra isolated vertex separates: vertex counts {} against {}",
            isolated.vertex_count(),
            one_edge.vertex_count()
        );
        let with_empty_edge = Hypergraph::from_edges(vec![vec![0], Vec::new()]);
        let doubled = Hypergraph::from_edges(vec![vec![0], vec![0]]);
        assert_eq!(
            with_empty_edge.compare(&doubled),
            CausalComparison::NotIsomorphic,
            "an empty hyperedge separates: arities {:?} against {:?}",
            with_empty_edge
                .edges()
                .map(Hyperedge::arity)
                .collect::<Vec<_>>(),
            doubled.edges().map(Hyperedge::arity).collect::<Vec<_>>()
        );
    }

    /// The directed `n`-cycle `base → base+1 → … → base+n-1 → base`.
    fn directed_cycle(n: usize, base: usize) -> Vec<Vec<usize>> {
        (0..n).map(|i| vec![base + i, base + (i + 1) % n]).collect()
    }

    /// The directed `n`-cycle on `0..n` with every edge reversed.
    fn reversed_cycle(n: usize) -> Vec<Vec<usize>> {
        (0..n).map(|i| vec![(i + 1) % n, i]).collect()
    }

    /// `compare` on vertex-transitive inputs, where the refinement leaves every
    /// vertex in one colour class: the directed `n`-cycle against its reversal,
    /// the `2n`-cycle against two `n`-cycles, and three 3-cycles against an
    /// interleaved relabelling of them. Each pair's fingerprints are equal.
    #[test]
    fn compare_decides_vertex_transitive_inputs() {
        for n in [6usize, 7, 8, 9, 10, 12] {
            let forward = Hypergraph::from_edges(directed_cycle(n, 0));
            let backward = Hypergraph::from_edges(reversed_cycle(n));
            assert_eq!(
                forward.compare(&backward),
                CausalComparison::Isomorphic,
                "the {n}-cycle against its reversal: expected Isomorphic, got {:?}",
                forward.compare(&backward)
            );
            assert_eq!(
                forward.fingerprint(),
                backward.fingerprint(),
                "the {n}-cycle against its reversal: expected equal fingerprints, got {} \
                 and {}",
                forward.fingerprint(),
                backward.fingerprint()
            );
        }

        for n in [4usize, 5, 6] {
            let big = Hypergraph::from_edges(directed_cycle(2 * n, 0));
            let mut split = directed_cycle(n, 0);
            split.extend(directed_cycle(n, n));
            let split = Hypergraph::from_edges(split);
            assert_eq!(
                big.compare(&split),
                CausalComparison::NotIsomorphic,
                "the {}-cycle against two {n}-cycles: expected NotIsomorphic, got {:?}",
                2 * n,
                big.compare(&split)
            );
            assert_eq!(
                big.fingerprint(),
                split.fingerprint(),
                "the {}-cycle against two {n}-cycles: expected equal fingerprints, got {} \
                 and {}",
                2 * n,
                big.fingerprint(),
                split.fingerprint()
            );
        }

        let mut disjoint = directed_cycle(3, 0);
        disjoint.extend(directed_cycle(3, 3));
        disjoint.extend(directed_cycle(3, 6));
        let disjoint = Hypergraph::from_edges(disjoint);
        let interleaved = Hypergraph::from_edges(vec![
            vec![0, 3],
            vec![3, 6],
            vec![6, 0],
            vec![1, 4],
            vec![4, 7],
            vec![7, 1],
            vec![2, 5],
            vec![5, 8],
            vec![8, 2],
        ]);
        assert_eq!(
            disjoint.compare(&interleaved),
            CausalComparison::Isomorphic,
            "three 3-cycles against their interleaved relabelling: expected Isomorphic, \
             got {:?}",
            disjoint.compare(&interleaved)
        );
        assert_eq!(
            disjoint.fingerprint(),
            interleaved.fingerprint(),
            "three 3-cycles against their interleaved relabelling: expected equal \
             fingerprints, got {} and {}",
            disjoint.fingerprint(),
            interleaved.fingerprint()
        );
    }

    /// The 6-cycle and two triangles share a fingerprint and compare
    /// `NotIsomorphic`.
    #[test]
    fn equal_fingerprints_do_not_imply_isomorphism() {
        let (six_cycle, two_triangles) = six_cycle_and_two_triangles();
        assert_eq!(
            six_cycle.fingerprint(),
            two_triangles.fingerprint(),
            "expected equal fingerprints, got {} and {}",
            six_cycle.fingerprint(),
            two_triangles.fingerprint()
        );
        assert_eq!(
            six_cycle.compare(&two_triangles),
            CausalComparison::NotIsomorphic,
            "expected NotIsomorphic, got {:?}",
            six_cycle.compare(&two_triangles)
        );
    }

    /// A hypergraph over `vertices` and `edges`.
    fn build(vertices: &[usize], edges: &[Vec<usize>]) -> Hypergraph {
        let mut graph = Hypergraph::new();
        for &v in vertices {
            graph.add_vertex(Some(v));
        }
        for edge in edges {
            graph.add_hyperedge(edge.clone());
        }
        graph
    }

    /// A seeded corpus of small ordered hypergraphs, each with the vertex list
    /// and the edge list that built it.
    fn seeded_corpus(count: usize) -> Vec<(Vec<usize>, Vec<Vec<usize>>)> {
        let mut rng = Lcg::new(0x164_0000_0000_0001);
        let mut corpus = Vec::with_capacity(count);
        for index in 0..count {
            let vertex_count = rng.next_usize(1, 6);
            let mut vertices: Vec<usize> = (0..vertex_count).collect();
            let edge_count = rng.next_usize(0, 5);
            let mut edges = Vec::with_capacity(edge_count);
            for _ in 0..edge_count {
                let arity = rng.next_usize(0, 3);
                edges.push(
                    (0..arity)
                        .map(|_| rng.next_usize(0, vertex_count - 1))
                        .collect::<Vec<usize>>(),
                );
            }
            if index % 3 == 0 {
                vertices.push(vertex_count);
            }
            corpus.push((vertices, edges));
        }
        corpus
    }

    /// A uniform permutation of `0..n`, drawn from `rng`.
    fn permutation(rng: &mut Lcg, n: usize) -> Vec<usize> {
        let mut p: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            p.swap(i, rng.next_usize(0, i));
        }
        p
    }

    /// `fingerprint` is constant on 200 seeded `v ↦ 10·π(v) + 3` relabellings
    /// and edge reorderings, and separates a measured floor of the corpus's
    /// non-isomorphic pairs.
    #[test]
    fn fingerprint_is_invariant_and_separating() {
        let corpus = seeded_corpus(200);
        let mut rng = Lcg::new(0x164_0000_0000_0002);

        let mut graphs = Vec::with_capacity(corpus.len());
        for (index, (vertices, edges)) in corpus.iter().enumerate() {
            let graph = build(vertices, edges);

            let width = vertices.iter().copied().max().unwrap_or(0) + 1;
            let pi = permutation(&mut rng, width);
            let relabel = |v: usize| 10 * pi[v] + 3;
            let mut shuffled: Vec<Vec<usize>> = edges
                .iter()
                .map(|edge| edge.iter().map(|&v| relabel(v)).collect())
                .collect();
            for i in (1..shuffled.len()).rev() {
                shuffled.swap(i, rng.next_usize(0, i));
            }
            let relabelled_vertices: Vec<usize> = vertices.iter().map(|&v| relabel(v)).collect();
            let relabelled = build(&relabelled_vertices, &shuffled);

            assert_eq!(
                graph.fingerprint(),
                relabelled.fingerprint(),
                "corpus {index}: expected equal fingerprints, got {} and {}; edges {edges:?} \
                 against {shuffled:?}",
                graph.fingerprint(),
                relabelled.fingerprint()
            );
            assert_eq!(
                graph.compare(&relabelled),
                CausalComparison::Isomorphic,
                "corpus {index}: expected Isomorphic, got {:?}; edges {edges:?} against \
                 {shuffled:?}",
                graph.compare(&relabelled)
            );

            graphs.push(graph);
        }

        let mut not_isomorphic = 0usize;
        let mut distinguished = 0usize;
        for left in 0..graphs.len() {
            for right in (left + 1)..graphs.len() {
                match graphs[left].compare(&graphs[right]) {
                    CausalComparison::NotIsomorphic => {
                        not_isomorphic += 1;
                        if graphs[left].fingerprint() != graphs[right].fingerprint() {
                            distinguished += 1;
                        }
                    }
                    CausalComparison::Isomorphic => {}
                    CausalComparison::Undecided => panic!(
                        "pair ({left}, {right}): the step budget was reached at {} vertices",
                        graphs[left].vertex_count()
                    ),
                }
            }
        }
        assert_eq!(
            not_isomorphic,
            19718,
            "expected 19718 non-isomorphic pairs over the corpus, got {not_isomorphic} of \
             {} unordered pairs",
            graphs.len() * (graphs.len() - 1) / 2
        );
        assert!(
            distinguished >= 19712,
            "expected the fingerprint to separate at least 19712 of the {not_isomorphic} \
             non-isomorphic pairs, got {distinguished}"
        );
    }
}
