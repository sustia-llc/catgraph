//! Hypergraph data structure for Wolfram Physics model.
//!
//! A [`Hypergraph`] stores a set of vertices (identified by `usize`) and a
//! collection of [`Hyperedge`]s. Vertices are tracked in a `BTreeSet` and
//! auto-registered when hyperedges are added. Supports pattern matching
//! via [`Hypergraph::find_matches`] for DPO rewrite rule application,
//! isomorphism heuristics, and compaction (vertex ID renumbering).

use super::hyperedge::Hyperedge;
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

/// A hypergraph consisting of vertices and hyperedges.
///
/// A hypergraph is a generalization of a graph where edges (hyperedges) can
/// connect any number of vertices. This is the fundamental structure in the
/// Wolfram Physics model.
///
/// # Structure
///
/// - Vertices are identified by `usize` IDs
/// - Hyperedges are ordered sequences of vertex IDs
/// - Multiple hyperedges can share vertices
/// - Self-loops and parallel hyperedges are allowed
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

    /// Computes a fingerprint for fast comparison.
    ///
    /// Two hypergraphs with the same fingerprint are likely equal
    /// (but not guaranteed due to hash collisions).
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();

        // Hash vertex count and edge count
        self.vertices.len().hash(&mut hasher);
        self.edges.len().hash(&mut hasher);

        // Hash canonical form of edges (sorted)
        let mut edge_fingerprints: Vec<_> = self.edges.iter().map(Hyperedge::fingerprint).collect();
        edge_fingerprints.sort_unstable();
        edge_fingerprints.hash(&mut hasher);

        hasher.finish()
    }

    /// Checks structural equality (ignoring vertex IDs).
    ///
    /// Two hypergraphs are structurally equal if there exists a bijection
    /// between their vertices that preserves hyperedge structure.
    #[must_use]
    pub fn is_isomorphic_to(&self, other: &Hypergraph) -> bool {
        // Quick checks
        if self.vertex_count() != other.vertex_count() {
            return false;
        }
        if self.edge_count() != other.edge_count() {
            return false;
        }

        // Check degree sequences
        let mut self_degrees: Vec<_> = self.vertices.iter().map(|&v| self.degree(v)).collect();
        let mut other_degrees: Vec<_> = other.vertices.iter().map(|&v| other.degree(v)).collect();
        self_degrees.sort_unstable();
        other_degrees.sort_unstable();

        if self_degrees != other_degrees {
            return false;
        }

        // Check edge arity multiset
        let mut self_arities: Vec<_> = self.edges.iter().map(Hyperedge::arity).collect();
        let mut other_arities: Vec<_> = other.edges.iter().map(Hyperedge::arity).collect();
        self_arities.sort_unstable();
        other_arities.sort_unstable();

        self_arities == other_arities

        // Note: Full isomorphism checking is NP-complete for general hypergraphs.
        // This is a heuristic check that catches most non-isomorphic cases.
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
        // Different structures should (usually) have different fingerprints
        // Note: This could occasionally fail due to hash collisions
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
}
