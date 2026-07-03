//! Finite skeletal category with identity-only endomorphisms — input type
//! for Leinster 2008 Cor 1.5 integer-exact Möbius inversion.
//!
//! Per Leinster 2008 *The Euler characteristic of a category*
//! (arXiv:0610260v1) Cor 1.5 (page 6), the Möbius function `μ: Ob 𝔸 × Ob 𝔸 → ℤ` takes **integer**
//! values when 𝔸 is a finite skeletal category whose only endomorphisms
//! are identities (equivalently: 𝔸 is **circuit-free** in the non-identity
//! arrow graph, with trivial automorphism group at every object).
//!
//! [`PosetCategory<NodeId>`] carves exactly this case: it stores an
//! arrow-count matrix `ζ ∈ ℕ^{n×n}` (the *zeta function* of 𝔸) with
//! diagonal `ζ[i][i] ∈ {0, 1}` (identity arrow or none) and validates the
//! circuit-free invariant at construction. Posets are the canonical
//! example (`ζ ∈ {0, 1}`), but the type also admits the wider
//! identity-endomorphism-only case where `ζ[i][j]` may exceed 1 off the
//! diagonal.
//!
//! This module is the **input** type for `mobius_function_via_chains_exact`.
//! It does not perform the Möbius inversion itself.

use std::collections::HashMap;
use std::hash::Hash;

use catgraph::errors::CatgraphError;

/// Finite skeletal category with identity-only endomorphisms.
///
/// Stores the zeta function `ζ: Ob 𝔸 × Ob 𝔸 → ℕ` as a dense matrix where
/// `ζ[i][j]` counts the arrows from `objects[i]` to `objects[j]`. For
/// **posets** the matrix is `{0, 1}`-valued; for more general
/// identity-endomorphism-only categories `ζ[i][j]` may exceed `1` off the
/// diagonal. The diagonal `ζ[i][i] ∈ {0, 1}` records whether the identity
/// arrow is present.
///
/// **Invariant (enforced):** the non-identity arrow graph is circuit-free.
/// Construction via [`PosetCategory::from_arrow_counts`] verifies this via a
/// depth-first cycle check; the [`PosetCategory::from_partial_order`]
/// constructor builds a `{0, 1}`-valued matrix that is trivially
/// circuit-free under the antisymmetry of `≤`, so no validation is needed
/// on that path.
///
/// The type is `Clone + Debug + PartialEq + Eq` (the latter pair derived
/// when `NodeId: Eq + Hash`, which the internal `HashMap` index already
/// requires for any operation that actually constructs the type). `Hash`
/// is NOT derived: `HashMap` does not implement `Hash`. The
/// `PartialEq + Eq` pair is sufficient for `assert_eq!(cat1, cat2)` in
/// downstream tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosetCategory<NodeId: Eq + Hash> {
    objects: Vec<NodeId>,
    /// `zeta[i][j]` = number of arrows from `objects[i]` to `objects[j]`.
    /// Diagonal `zeta[i][i] ∈ {0, 1}` (the identity arrow or none).
    zeta: Vec<Vec<u64>>,
    /// Index lookup for O(1) `zeta(a, b)` queries.
    index: HashMap<NodeId, usize>,
}

impl<NodeId: Clone + Eq + Hash> PosetCategory<NodeId> {
    /// Construct from a partial-order predicate `le(&a, &b) == a ≤ b`.
    ///
    /// Auto-sets `ζ[i][j] = 1` whenever `le(objects[i], objects[j])`; else
    /// `0`. The diagonal is always `1` (identity arrows). This path
    /// guarantees `{0, 1}`-valued `ζ` and a circuit-free non-identity
    /// arrow graph by the antisymmetry of `≤`, so no validation is run.
    ///
    /// In debug builds a `debug_assert!` checks that `le` is reflexive at
    /// every object (`le(x, x) == true`); the diagonal override `ζ[i][i] = 1`
    /// runs unconditionally in release.
    ///
    /// # Examples
    ///
    /// ```
    /// use catgraph_magnitude::PosetCategory;
    ///
    /// // 3-chain 0 ≤ 1 ≤ 2 from the standard u32 order.
    /// let cat = PosetCategory::<u32>::from_partial_order(vec![0, 1, 2], |a, b| a <= b);
    /// assert_eq!(cat.size(), 3);
    /// assert_eq!(cat.zeta(&0, &2), 1); // 0 ≤ 2
    /// assert_eq!(cat.zeta(&2, &0), 0); // 2 not ≤ 0
    /// assert_eq!(cat.zeta(&1, &1), 1); // identity
    /// ```
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `le` is not reflexive on the supplied
    /// `objects`. Release builds silently override the diagonal to `1`.
    #[must_use]
    pub fn from_partial_order<F>(objects: Vec<NodeId>, le: F) -> Self
    where
        F: Fn(&NodeId, &NodeId) -> bool,
    {
        let n = objects.len();
        let mut zeta = vec![vec![0_u64; n]; n];
        for i in 0..n {
            for j in 0..n {
                if le(&objects[i], &objects[j]) {
                    zeta[i][j] = 1;
                }
            }
            debug_assert!(
                le(&objects[i], &objects[i]),
                "PosetCategory::from_partial_order: predicate violates reflexivity at index {i}"
            );
            zeta[i][i] = 1; // identity always present (asserted reflexive in debug)
        }
        let index: HashMap<NodeId, usize> = objects
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, o)| (o, i))
            .collect();
        PosetCategory {
            objects,
            zeta,
            index,
        }
    }

    /// Construct from a raw arrow-count matrix `ζ ∈ ℕ^{n×n}`.
    ///
    /// Validates the structural invariants for Cor 1.5 applicability:
    /// - `ζ` is `n × n` where `n = objects.len()`.
    /// - Diagonal entries `ζ[i][i] ∈ {0, 1}` (identity arrows only).
    /// - The non-identity arrow graph (edges `i → j` for `i ≠ j` with
    ///   `ζ[i][j] > 0`) is **circuit-free**.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] when any of the three
    /// invariants is violated. The error message names the violated
    /// invariant.
    ///
    /// # Examples
    ///
    /// Skeletal upper-triangular category with identities is accepted:
    ///
    /// ```
    /// use catgraph_magnitude::PosetCategory;
    ///
    /// let cat = PosetCategory::<u32>::from_arrow_counts(
    ///     vec![0, 1, 2],
    ///     vec![vec![1, 1, 0], vec![0, 1, 1], vec![0, 0, 1]],
    /// )
    /// .expect("upper-triangular ζ with unit diagonal is circuit-free");
    /// assert_eq!(cat.size(), 3);
    /// ```
    ///
    /// A 2-object cycle (no identities) is rejected:
    ///
    /// ```
    /// use catgraph_magnitude::PosetCategory;
    ///
    /// let result = PosetCategory::<u32>::from_arrow_counts(
    ///     vec![0, 1],
    ///     vec![vec![0, 1], vec![1, 0]],
    /// );
    /// assert!(result.is_err(), "2-cycle violates circuit-free invariant");
    /// ```
    // No `#[must_use]`: return type is `Result<Self, _>`, which is itself
    // `#[must_use]` (avoids clippy::double_must_use).
    pub fn from_arrow_counts(
        objects: Vec<NodeId>,
        zeta: Vec<Vec<u64>>,
    ) -> Result<Self, CatgraphError> {
        let n = objects.len();
        if zeta.len() != n || zeta.iter().any(|r| r.len() != n) {
            return Err(CatgraphError::Composition {
                message: format!("PosetCategory: zeta must be {n}x{n}"),
            });
        }
        for (i, row) in zeta.iter().enumerate() {
            if row[i] > 1 {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "PosetCategory: ζ[{i}][{i}] = {} must be 0 or 1 (identity only)",
                        row[i]
                    ),
                });
            }
        }
        if has_cycle(&zeta) {
            return Err(CatgraphError::Composition {
                message:
                    "PosetCategory: non-identity arrow graph contains a cycle (not circuit-free)"
                        .to_string(),
            });
        }
        let index: HashMap<NodeId, usize> = objects
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, o)| (o, i))
            .collect();
        Ok(PosetCategory {
            objects,
            zeta,
            index,
        })
    }

    /// Number of objects in the category.
    #[must_use]
    pub fn size(&self) -> usize {
        self.objects.len()
    }

    /// Slice view of the object list (caller-supplied order).
    #[must_use]
    pub fn objects(&self) -> &[NodeId] {
        &self.objects
    }

    /// Arrow count `ζ(a, b) ∈ ℕ`. Returns `0` if either object is not in
    /// the category.
    #[must_use]
    pub fn zeta(&self, a: &NodeId, b: &NodeId) -> u64 {
        match (self.index.get(a), self.index.get(b)) {
            (Some(&i), Some(&j)) => self.zeta[i][j],
            _ => 0,
        }
    }

    /// Raw zeta matrix (objects-indexed) as a row slice. Row `i`,
    /// column `j` is the arrow count from `objects[i]` to `objects[j]`.
    ///
    /// Returns `&[Vec<u64>]` rather than `&Vec<Vec<u64>>` so callers don't
    /// take a dependency on the inner storage being a `Vec`; indexing,
    /// `.len()`, and `.iter()` are all available unchanged.
    #[must_use]
    pub fn zeta_matrix(&self) -> &[Vec<u64>] {
        &self.zeta
    }
}

/// Depth-first cycle detection on the non-identity arrow graph of `zeta`.
///
/// Uses the standard three-colour DFS: `0 = unvisited`, `1 = in-progress`
/// (on the current DFS stack), `2 = done`. Encountering an in-progress
/// vertex signals a back-edge → cycle.
fn has_cycle(zeta: &[Vec<u64>]) -> bool {
    let n = zeta.len();
    let mut visited = vec![0_u8; n];
    for start in 0..n {
        if visited[start] == 0 && dfs_cycle_helper(zeta, start, &mut visited) {
            return true;
        }
    }
    false
}

/// Recursive DFS helper for [`has_cycle`]. Returns `true` once a cycle is
/// discovered; the caller short-circuits.
fn dfs_cycle_helper(zeta: &[Vec<u64>], v: usize, visited: &mut [u8]) -> bool {
    visited[v] = 1;
    for u in 0..zeta.len() {
        if u != v && zeta[v][u] > 0 {
            if visited[u] == 1 {
                return true;
            }
            if visited[u] == 0 && dfs_cycle_helper(zeta, u, visited) {
                return true;
            }
        }
    }
    visited[v] = 2;
    false
}
