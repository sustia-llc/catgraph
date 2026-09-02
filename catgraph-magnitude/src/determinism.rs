//! Deterministic-transition rank of an LM-enriched category.
//!
//! For an (acyclic) language model the magnitude-homology rank `MH_1` at grade
//! `ℓ = 0` has a clean closed form: it counts the **covering relations of the
//! distance-0 (probability-1) sub-order** — the "atomic" deterministic
//! transitions `π(y | x) = 1`.
//!
//! In the Leinster–Shulman magnitude-homology complex the boundary omits only
//! interior vertices of a chain, so a 1-chain `(x₀, x₁)` has none and
//! `∂_1 = 0`, giving `MH_1(ℓ) = C_1(ℓ) / im ∂_2`. At `ℓ = 0`, `C_1(0)` is free
//! abelian on the distance-0 edges `(x, y)`, `x ≠ y`, where
//! `d(x, y) = −ln π(y | x) = 0 ⟺ π(y | x) = 1`, and `im ∂_2` is spanned by the
//! distance-0 edges factoring through an intermediate. The quotient is free
//! abelian on the covering relations of the distance-0 order, so
//! `rank MH_1(0)` counts covering deterministic transitions and is `> 0` iff
//! the LM has at least one deterministic transition.
//!
//! The quantity is a structural invariant (BV 2025 / Leinster–Shulman 2017): a
//! deterministic transition is a forced continuation, so it measures how much
//! of the LM is deterministic.

use crate::chain_complex::{ChainIndex, magnitude_homology_rank};
use crate::lm_category::LmCategory;
use crate::{CatgraphError, F64Rig};

impl LmCategory {
    /// The number of atomic deterministic (`π = 1`) transitions: `rank MH_1(ℓ=0)`,
    /// equivalently the covering relations of the distance-0 sub-order.
    ///
    /// `> 0` iff the LM contains at least one deterministic transition. See the
    /// module documentation for the derivation.
    ///
    /// # Errors
    ///
    /// Propagates [`enriched_space`](Self::enriched_space) (BFS-cap) and
    /// [`magnitude_homology_rank`] (SNF cross-check) failures.
    pub fn deterministic_transition_rank(&self) -> Result<usize, CatgraphError> {
        let space = self.enriched_space()?;
        // MH_1 needs chains up to degree 2 (the boundary ∂_2 enters rank H_1).
        let idx = ChainIndex::new(&space, 2);
        magnitude_homology_rank::<F64Rig>(&idx, &space, 1, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lm(objects: &[&str], edges: &[(&str, &str, f64)], term: &[&str]) -> LmCategory {
        let mut m = LmCategory::new(objects.iter().map(|s| (*s).to_owned()).collect());
        for (a, b, p) in edges {
            m.add_transition(a, b, *p).unwrap();
        }
        for t in term {
            m.mark_terminating(t);
        }
        m
    }

    #[test]
    fn counts_covering_deterministic_transitions() {
        // Probabilistic only → 0 covering deterministic edges.
        let star = lm(
            &["⊥", "a", "b"],
            &[("⊥", "a", 0.6), ("⊥", "b", 0.4)],
            &["a", "b"],
        );
        assert_eq!(star.deterministic_transition_rank().unwrap(), 0);

        // Single π=1 edge → 1.
        let one = lm(&["⊥", "a"], &[("⊥", "a", 1.0)], &["a"]);
        assert_eq!(one.deterministic_transition_rank().unwrap(), 1);

        // Two-step deterministic chain ⊥→a→c → 2 covering edges
        // (the composite ⊥→c is non-covering, excluded).
        let chain = lm(
            &["⊥", "a", "c"],
            &[("⊥", "a", 1.0), ("a", "c", 1.0)],
            &["c"],
        );
        assert_eq!(chain.deterministic_transition_rank().unwrap(), 2);

        // Diamond rejoin a→d, b→d (both π=1; ⊥→{a,b}=0.5 probabilistic) → 2.
        let dia = lm(
            &["⊥", "a", "b", "d"],
            &[
                ("⊥", "a", 0.5),
                ("⊥", "b", 0.5),
                ("a", "d", 1.0),
                ("b", "d", 1.0),
            ],
            &["d"],
        );
        assert_eq!(dia.deterministic_transition_rank().unwrap(), 2);

        // Length-3 deterministic chain a→b→c→d → 3.
        let long = lm(
            &["a", "b", "c", "d"],
            &[("a", "b", 1.0), ("b", "c", 1.0), ("c", "d", 1.0)],
            &["d"],
        );
        assert_eq!(long.deterministic_transition_rank().unwrap(), 3);
    }

    #[test]
    fn positive_iff_has_deterministic_transition() {
        let det = lm(&["⊥", "a", "b"], &[("⊥", "a", 1.0)], &["a", "b"]);
        assert!(det.deterministic_transition_rank().unwrap() > 0);

        let prob = lm(
            &["⊥", "a", "b"],
            &[("⊥", "a", 0.7), ("⊥", "b", 0.3)],
            &["a", "b"],
        );
        assert_eq!(prob.deterministic_transition_rank().unwrap(), 0);
    }
}
