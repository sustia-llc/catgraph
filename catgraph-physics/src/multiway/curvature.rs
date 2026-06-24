//! Discrete curvature trait and foliation for multiway systems.
//!
//! Provides a trait-based architecture for computing curvature on branchial
//! graphs, with pluggable backends (Ollivier-Ricci, manifold embedding).

use std::fmt::{self, Debug, Display};

/// Discrete curvature on a branchial graph.
///
/// Two backends: Ollivier-Ricci (default) and Riemannian manifold embedding
/// (feature = "manifold-curvature").
pub trait DiscreteCurvature: Clone + Debug {
    /// Scalar curvature R (trace of Ricci). 0 = flat, >0 = sphere-like, <0 = saddle-like.
    #[must_use]
    fn scalar_curvature(&self) -> f64;

    /// Whether the branchial space is flat (within tolerance).
    #[must_use]
    fn is_flat(&self) -> bool;

    /// Ricci curvature at vertex i.
    #[must_use]
    fn ricci_curvature(&self, vertex: usize) -> f64;

    /// Sectional curvature for the 2-plane spanned by vertices i, j.
    #[must_use]
    fn sectional_curvature(&self, i: usize, j: usize) -> f64;

    /// Irreducibility indicator. Higher = more irreducible.
    #[must_use]
    fn irreducibility_indicator(&self) -> f64;

    /// Dimension (number of branches).
    #[must_use]
    fn dimension(&self) -> usize;

    /// Time step this curvature was computed for.
    #[must_use]
    fn step(&self) -> usize;
}

/// Curvature analysis across all time steps of a multiway evolution.
///
/// Generic over any [`DiscreteCurvature`] backend (Ollivier-Ricci, manifold
/// embedding, etc.). Stores one curvature value per branchial graph in the
/// foliation and provides aggregate metrics: global flatness, irreducibility
/// profile, and average irreducibility indicator.
#[derive(Debug, Clone)]
pub struct CurvatureFoliation<C: DiscreteCurvature> {
    /// Curvature at each time step.
    pub curvatures: Vec<C>,
}

impl<C: DiscreteCurvature> CurvatureFoliation<C> {
    /// Construct a foliation from a sequence of curvature values.
    #[must_use]
    pub fn from_curvatures(curvatures: Vec<C>) -> Self {
        Self { curvatures }
    }

    /// Check if the entire evolution has flat branchial geometry.
    #[must_use]
    pub fn is_globally_flat(&self) -> bool {
        self.curvatures.iter().all(DiscreteCurvature::is_flat)
    }

    /// Get the irreducibility profile over time.
    ///
    /// Returns a vector of irreducibility indicators, one per step.
    #[must_use]
    pub fn irreducibility_profile(&self) -> Vec<f64> {
        self.curvatures
            .iter()
            .map(DiscreteCurvature::irreducibility_indicator)
            .collect()
    }

    /// Compute average irreducibility indicator.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn average_irreducibility(&self) -> f64 {
        if self.curvatures.is_empty() {
            return 0.0;
        }

        let total: f64 = self
            .curvatures
            .iter()
            .map(DiscreteCurvature::irreducibility_indicator)
            .sum();
        total / self.curvatures.len() as f64
    }
}

impl<C: DiscreteCurvature> Display for CurvatureFoliation<C> {
    #[allow(clippy::cast_precision_loss)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_scalar: f64 = self
            .curvatures
            .iter()
            .map(DiscreteCurvature::scalar_curvature)
            .sum();
        let average_scalar = if self.curvatures.is_empty() {
            0.0
        } else {
            total_scalar / self.curvatures.len() as f64
        };

        writeln!(f, "Curvature Foliation:")?;
        writeln!(f, "  Steps analyzed: {}", self.curvatures.len())?;
        writeln!(f, "  Total scalar curvature: {total_scalar:.6}")?;
        writeln!(f, "  Average scalar curvature: {average_scalar:.6}")?;
        writeln!(f, "  Globally flat: {}", self.is_globally_flat())?;
        write!(
            f,
            "  Average irreducibility: {:.6}",
            self.average_irreducibility()
        )
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::DiscreteCurvature;

    /// Verify trait conformance for any `DiscreteCurvature` implementation.
    pub fn assert_trait_conformance<C: DiscreteCurvature>(
        curv: &C,
        expected_dim: usize,
        expected_step: usize,
    ) {
        assert_eq!(curv.dimension(), expected_dim, "dimension mismatch");
        assert_eq!(curv.step(), expected_step, "step mismatch");
        assert!(
            curv.irreducibility_indicator() >= 0.0,
            "indicator must be non-negative"
        );

        // If flat, scalar curvature should be near zero
        if curv.is_flat() {
            assert!(
                curv.scalar_curvature().abs() < 1e-6,
                "flat curvature should have scalar ≈ 0, got {}",
                curv.scalar_curvature()
            );
        }
    }
}
