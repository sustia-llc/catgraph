//! Hyperedge type for hypergraph structures.
//!
//! A [`Hyperedge`] is an ordered sequence of vertex IDs, generalizing a graph
//! edge to connect any number of vertices. Ordered semantics match Wolfram
//! Physics conventions: `{0, 1, 2} != {2, 1, 0}`. Provides constructors for
//! common arities (unary, binary, ternary) and set-theoretic operations
//! (overlap, intersection, subset).

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

/// A hyperedge connecting multiple vertices.
///
/// Unlike regular graph edges that connect exactly two vertices, a hyperedge
/// can connect any number of vertices. This is the fundamental building block
/// of hypergraphs used in the Wolfram Physics model.
///
/// # Ordering
///
/// Hyperedges can be either ordered (sequence of vertices) or unordered (set of vertices).
/// By default, we use ordered hyperedges as they're more common in Wolfram Physics rules.
///
/// # Example
///
/// ```rust
/// use catgraph_physics::hypergraph::Hyperedge;
///
/// // Create an ordered hyperedge {0, 1, 2}
/// let edge = Hyperedge::new(vec![0, 1, 2]);
/// assert_eq!(edge.arity(), 3);
/// assert!(edge.contains(&1));
///
/// // Ordered hyperedges: {0, 1, 2} != {2, 1, 0}
/// let edge2 = Hyperedge::new(vec![2, 1, 0]);
/// assert_ne!(edge, edge2);
/// ```
#[derive(Debug, Clone, Eq)]
pub struct Hyperedge {
    /// Vertices connected by this hyperedge (ordered).
    vertices: Vec<usize>,
}

impl Hyperedge {
    /// Creates a new hyperedge connecting the given vertices.
    ///
    /// # Arguments
    ///
    /// * `vertices` - The vertices connected by this hyperedge (order matters).
    ///
    /// # Example
    ///
    /// ```rust
    /// use catgraph_physics::hypergraph::Hyperedge;
    ///
    /// let edge = Hyperedge::new(vec![0, 1, 2]);
    /// assert_eq!(edge.vertices(), &[0, 1, 2]);
    /// ```
    #[must_use]
    pub fn new(vertices: Vec<usize>) -> Self {
        Self { vertices }
    }

    /// Creates an empty hyperedge (no vertices).
    #[must_use]
    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    /// Creates a unary hyperedge (single vertex).
    #[must_use]
    pub fn unary(v: usize) -> Self {
        Self { vertices: vec![v] }
    }

    /// Creates a binary hyperedge (two vertices, like a regular edge).
    #[must_use]
    pub fn binary(v1: usize, v2: usize) -> Self {
        Self {
            vertices: vec![v1, v2],
        }
    }

    /// Creates a ternary hyperedge (three vertices).
    #[must_use]
    pub fn ternary(v1: usize, v2: usize, v3: usize) -> Self {
        Self {
            vertices: vec![v1, v2, v3],
        }
    }

    /// Returns the vertices of this hyperedge.
    #[inline]
    #[must_use]
    pub fn vertices(&self) -> &[usize] {
        &self.vertices
    }

    /// Returns a mutable reference to the vertices.
    #[inline]
    pub fn vertices_mut(&mut self) -> &mut Vec<usize> {
        &mut self.vertices
    }

    /// Returns the arity (number of vertices) of this hyperedge.
    #[inline]
    #[must_use]
    pub fn arity(&self) -> usize {
        self.vertices.len()
    }

    /// Returns true if this hyperedge is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Returns true if this hyperedge contains the given vertex.
    #[inline]
    #[must_use]
    pub fn contains(&self, vertex: &usize) -> bool {
        self.vertices.contains(vertex)
    }

    /// Returns the set of unique vertices (ignoring order and duplicates).
    #[must_use]
    pub fn vertex_set(&self) -> BTreeSet<usize> {
        self.vertices.iter().copied().collect()
    }

    /// Returns true if this hyperedge is a subset of another (ignoring order).
    #[must_use]
    pub fn is_subset_of(&self, other: &Hyperedge) -> bool {
        let self_set = self.vertex_set();
        let other_set = other.vertex_set();
        self_set.is_subset(&other_set)
    }

    /// Returns true if this hyperedge overlaps with another (shares at least one vertex).
    #[must_use]
    pub fn overlaps(&self, other: &Hyperedge) -> bool {
        self.vertices.iter().any(|v| other.contains(v))
    }

    /// Returns the intersection of vertices with another hyperedge.
    #[must_use]
    pub fn intersection(&self, other: &Hyperedge) -> BTreeSet<usize> {
        let self_set = self.vertex_set();
        let other_set = other.vertex_set();
        self_set.intersection(&other_set).copied().collect()
    }

    /// Applies a vertex renaming map to this hyperedge.
    ///
    /// # Arguments
    ///
    /// * `map` - A function that maps old vertex IDs to new vertex IDs.
    ///
    /// # Returns
    ///
    /// A new hyperedge with renamed vertices.
    #[must_use]
    pub fn rename_vertices<F>(&self, map: F) -> Self
    where
        F: Fn(usize) -> usize,
    {
        Self {
            vertices: self.vertices.iter().map(|&v| map(v)).collect(),
        }
    }

    /// Returns a canonical form of this hyperedge (sorted vertices).
    ///
    /// Useful for comparing hyperedges ignoring vertex order.
    #[must_use]
    pub fn canonical(&self) -> Self {
        let mut vertices = self.vertices.clone();
        vertices.sort_unstable();
        Self { vertices }
    }

    /// Computes a fingerprint for fast comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl PartialEq for Hyperedge {
    fn eq(&self, other: &Self) -> bool {
        // Ordered comparison
        self.vertices == other.vertices
    }
}

impl Hash for Hyperedge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.vertices.hash(state);
    }
}

impl std::fmt::Display for Hyperedge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, v) in self.vertices.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
        }
        write!(f, "}}")
    }
}

impl From<Vec<usize>> for Hyperedge {
    fn from(vertices: Vec<usize>) -> Self {
        Self::new(vertices)
    }
}

impl<const N: usize> From<[usize; N]> for Hyperedge {
    fn from(vertices: [usize; N]) -> Self {
        Self::new(vertices.to_vec())
    }
}

impl Hyperedge {
    /// Returns an iterator over the vertices in this hyperedge.
    pub fn iter(&self) -> std::slice::Iter<'_, usize> {
        self.vertices.iter()
    }
}

impl IntoIterator for Hyperedge {
    type Item = usize;
    type IntoIter = std::vec::IntoIter<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.vertices.into_iter()
    }
}

impl<'a> IntoIterator for &'a Hyperedge {
    type Item = &'a usize;
    type IntoIter = std::slice::Iter<'a, usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.vertices.iter()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperedge_new() {
        let edge = Hyperedge::new(vec![0, 1, 2]);
        assert_eq!(edge.arity(), 3);
        assert_eq!(edge.vertices(), &[0, 1, 2]);
    }

    #[test]
    fn test_hyperedge_constructors() {
        assert!(Hyperedge::empty().is_empty());
        assert_eq!(Hyperedge::unary(5).vertices(), &[5]);
        assert_eq!(Hyperedge::binary(1, 2).vertices(), &[1, 2]);
        assert_eq!(Hyperedge::ternary(1, 2, 3).vertices(), &[1, 2, 3]);
    }

    #[test]
    fn test_hyperedge_contains() {
        let edge = Hyperedge::new(vec![0, 1, 2]);
        assert!(edge.contains(&0));
        assert!(edge.contains(&1));
        assert!(edge.contains(&2));
        assert!(!edge.contains(&3));
    }

    #[test]
    fn test_hyperedge_equality_ordered() {
        let e1 = Hyperedge::new(vec![0, 1, 2]);
        let e2 = Hyperedge::new(vec![0, 1, 2]);
        let e3 = Hyperedge::new(vec![2, 1, 0]);

        assert_eq!(e1, e2);
        assert_ne!(e1, e3); // Order matters
    }

    #[test]
    fn test_hyperedge_vertex_set() {
        let edge = Hyperedge::new(vec![2, 0, 1, 2]); // Duplicate 2
        let set = edge.vertex_set();
        assert_eq!(set.len(), 3);
        assert!(set.contains(&0));
        assert!(set.contains(&1));
        assert!(set.contains(&2));
    }

    #[test]
    fn test_hyperedge_overlaps() {
        let e1 = Hyperedge::new(vec![0, 1, 2]);
        let e2 = Hyperedge::new(vec![2, 3, 4]);
        let e3 = Hyperedge::new(vec![5, 6, 7]);

        assert!(e1.overlaps(&e2)); // Share vertex 2
        assert!(!e1.overlaps(&e3)); // No overlap
    }

    #[test]
    fn test_hyperedge_intersection() {
        let e1 = Hyperedge::new(vec![0, 1, 2, 3]);
        let e2 = Hyperedge::new(vec![2, 3, 4, 5]);
        let intersection = e1.intersection(&e2);

        assert_eq!(intersection.len(), 2);
        assert!(intersection.contains(&2));
        assert!(intersection.contains(&3));
    }

    #[test]
    fn test_hyperedge_rename() {
        let edge = Hyperedge::new(vec![0, 1, 2]);
        let renamed = edge.rename_vertices(|v| v + 10);
        assert_eq!(renamed.vertices(), &[10, 11, 12]);
    }

    #[test]
    fn test_hyperedge_canonical() {
        let edge = Hyperedge::new(vec![3, 1, 2]);
        let canonical = edge.canonical();
        assert_eq!(canonical.vertices(), &[1, 2, 3]);
    }

    #[test]
    fn test_hyperedge_display() {
        let edge = Hyperedge::new(vec![0, 1, 2]);
        assert_eq!(format!("{edge}"), "{0, 1, 2}");
    }

    #[test]
    fn test_hyperedge_from() {
        let e1: Hyperedge = vec![1, 2, 3].into();
        let e2: Hyperedge = [4, 5, 6].into();

        assert_eq!(e1.vertices(), &[1, 2, 3]);
        assert_eq!(e2.vertices(), &[4, 5, 6]);
    }
}
