//! Temporal cospan chain bridge.
//!
//! Maps interval sequences to composable cospan chains in the discrete-time
//! category. Builds a 1D simplicial complex from contiguous interval sequences
//! and provides conservation verification (contiguity + monotonicity), 1-form
//! integration, and the bridge into [`catgraph::cospan::Cospan`] composition.
//!
//! For 2D discrete exterior calculus on multiway confluence diamonds, see the
//! `dec` feature in `irreducible::multiway_stokes` (downstream consumer).

use crate::interval::DiscreteInterval;
use catgraph::cospan::Cospan;
use catgraph::errors::CatgraphError;

/// A simplicial complex representing temporal structure of computation.
///
/// The complex has dimension 1:
/// - 0-skeleton: time step vertices
/// - 1-skeleton: interval edges connecting consecutive times
#[derive(Debug, Clone)]
pub struct TemporalComplex {
    /// Time points (vertices of the 0-skeleton).
    time_points: Vec<usize>,
    /// Step counts for each interval (edge weights).
    step_counts: Vec<usize>,
}

impl TemporalComplex {
    /// Creates a temporal complex from a sequence of intervals.
    ///
    /// # Errors
    ///
    /// Returns [`TemporalComplexError::EmptyIntervals`] if the interval slice is
    /// empty, or [`TemporalComplexError::InsufficientPoints`] if fewer than two
    /// distinct time points are present after deduplication.
    pub fn from_intervals(intervals: &[DiscreteInterval]) -> Result<Self, TemporalComplexError> {
        if intervals.is_empty() {
            return Err(TemporalComplexError::EmptyIntervals);
        }

        let mut time_points: Vec<usize> = Vec::new();
        for interval in intervals {
            if time_points.is_empty() || time_points.last() != Some(&interval.start) {
                time_points.push(interval.start);
            }
            time_points.push(interval.end);
        }

        time_points.sort_unstable();
        time_points.dedup();

        if time_points.len() < 2 {
            return Err(TemporalComplexError::InsufficientPoints(time_points.len()));
        }

        let step_counts: Vec<usize> = time_points
            .windows(2)
            .map(|w| w[1].saturating_sub(w[0]))
            .collect();

        Ok(Self {
            time_points,
            step_counts,
        })
    }

    /// Returns the number of time steps (vertices).
    #[inline]
    #[must_use]
    pub fn num_time_steps(&self) -> usize {
        self.time_points.len()
    }

    /// Returns the number of intervals (edges).
    #[inline]
    #[must_use]
    pub fn num_intervals(&self) -> usize {
        self.step_counts.len()
    }

    /// Returns the time points.
    #[inline]
    #[must_use]
    pub fn time_points(&self) -> &[usize] {
        &self.time_points
    }

    /// Returns the step counts for each interval.
    #[inline]
    #[must_use]
    pub fn step_counts(&self) -> &[usize] {
        &self.step_counts
    }

    /// Converts the interval sequence to a 1-form (coefficient vector).
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn intervals_to_form(&self) -> Vec<f64> {
        self.step_counts.iter().map(|&s| s as f64).collect()
    }

    /// Integrates a 1-form over the full chain.
    #[must_use]
    pub fn integrate(&self, form: &[f64]) -> f64 {
        form.iter().sum()
    }

    /// Verifies if the interval sequence satisfies conservation.
    ///
    /// Conservation means:
    /// 1. **Contiguity** -- all step counts are positive (no zero-length gaps)
    /// 2. **Monotonicity** -- time flows forward
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn verify_conservation(&self) -> ConservationResult {
        let is_contiguous = self.step_counts.iter().all(|&s| s > 0);
        let total_complexity: f64 = self.step_counts.iter().map(|&s| s as f64).sum();
        let is_monotonic = self.time_points.windows(2).all(|w| w[0] < w[1]);

        ConservationResult {
            is_conserved: is_contiguous && is_monotonic,
            is_contiguous,
            is_monotonic,
            total_complexity,
            num_intervals: self.num_intervals(),
            num_time_steps: self.num_time_steps(),
        }
    }

    /// Converts the temporal complex into a chain of composable cospans.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn to_cospan_chain(&self) -> Vec<Cospan<u32>> {
        let mut cospans = Vec::new();

        for i in 0..self.num_intervals() {
            let t_start = self.time_points[i] as u32;
            let t_end = self.time_points[i + 1] as u32;
            let left = vec![0];
            let right = vec![1];
            let middle = vec![t_start, t_end];

            cospans.push(Cospan::new(left, right, middle));
        }

        cospans
    }

    /// Composes the full cospan chain into a single composite cospan.
    ///
    /// # Errors
    ///
    /// Returns `CatgraphError::Composition` if the cospan chain is empty
    /// or if adjacent cospans are not composable.
    pub fn compose_cospan_chain(&self) -> Result<Cospan<u32>, CatgraphError> {
        catgraph::cospan::compose_chain(self.to_cospan_chain())
    }
}

/// Result of conservation verification for a temporal complex.
#[derive(Debug, Clone, PartialEq)]
pub struct ConservationResult {
    /// Whether the computation is conserved (contiguous and monotonic).
    pub is_conserved: bool,
    /// Whether intervals are contiguous (no gaps).
    pub is_contiguous: bool,
    /// Whether time is monotonically increasing.
    pub is_monotonic: bool,
    /// Total complexity (sum of all step counts).
    pub total_complexity: f64,
    /// Number of intervals in the trajectory.
    pub num_intervals: usize,
    /// Number of time steps (vertices).
    pub num_time_steps: usize,
}

impl ConservationResult {
    /// Returns the average complexity per interval.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[inline]
    #[must_use]
    pub fn average_complexity(&self) -> f64 {
        if self.num_intervals == 0 {
            0.0
        } else {
            self.total_complexity / self.num_intervals as f64
        }
    }

    /// Checks if the trajectory is well-formed (contiguous and monotonic).
    #[inline]
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.is_contiguous && self.is_monotonic
    }
}

/// Errors that can occur during temporal complex construction.
#[derive(Debug, Clone, PartialEq)]
pub enum TemporalComplexError {
    /// No intervals provided.
    EmptyIntervals,
    /// Insufficient time points to form a complex.
    InsufficientPoints(usize),
}

impl std::fmt::Display for TemporalComplexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIntervals => {
                write!(f, "Cannot create temporal complex from empty intervals")
            }
            Self::InsufficientPoints(n) => {
                write!(f, "Need at least 2 time points, got {n}")
            }
        }
    }
}

impl std::error::Error for TemporalComplexError {}

#[cfg(test)]
mod tests {
    use super::*;
    use catgraph::category::Composable;

    #[test]
    fn test_temporal_complex_creation() {
        let intervals = vec![
            DiscreteInterval::new(0, 2),
            DiscreteInterval::new(2, 5),
            DiscreteInterval::new(5, 7),
        ];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        assert_eq!(complex.num_time_steps(), 4);
        assert_eq!(complex.num_intervals(), 3);
        assert_eq!(complex.step_counts(), &[2, 3, 2]);
    }

    #[test]
    fn test_intervals_to_form() {
        let intervals = vec![DiscreteInterval::new(0, 2), DiscreteInterval::new(2, 5)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let form = complex.intervals_to_form();
        assert_eq!(form, vec![2.0, 3.0]);
    }

    #[test]
    fn test_conservation_contiguous() {
        let intervals = vec![
            DiscreteInterval::new(0, 1),
            DiscreteInterval::new(1, 2),
            DiscreteInterval::new(2, 3),
        ];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let result = complex.verify_conservation();
        assert!(result.is_contiguous);
        assert!(result.is_monotonic);
        assert!((result.total_complexity - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_empty_intervals_error() {
        let result = TemporalComplex::from_intervals(&[]);
        assert!(matches!(result, Err(TemporalComplexError::EmptyIntervals)));
    }

    #[test]
    fn test_integration_over_chain() {
        let intervals = vec![DiscreteInterval::new(0, 3), DiscreteInterval::new(3, 7)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let form = complex.intervals_to_form();
        let integrated = complex.integrate(&form);
        assert!((integrated - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_cospan_chain_composable_via_catgraph() {
        let intervals = vec![
            DiscreteInterval::new(0, 1),
            DiscreteInterval::new(1, 2),
            DiscreteInterval::new(2, 3),
        ];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let cospans = complex.to_cospan_chain();
        for i in 0..cospans.len() - 1 {
            assert!(cospans[i].composable(&cospans[i + 1]).is_ok());
        }
    }

    #[test]
    fn test_compose_cospan_chain() {
        let intervals = vec![
            DiscreteInterval::new(0, 3),
            DiscreteInterval::new(3, 7),
            DiscreteInterval::new(7, 10),
        ];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let composite = complex.compose_cospan_chain().unwrap();
        assert_eq!(composite.domain(), vec![0u32]);
        assert_eq!(composite.codomain(), vec![10u32]);
    }

    #[test]
    fn test_cospan_labels_are_time_points() {
        let intervals = vec![
            DiscreteInterval::new(0, 2),
            DiscreteInterval::new(2, 5),
            DiscreteInterval::new(5, 7),
        ];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let cospans = complex.to_cospan_chain();
        assert_eq!(cospans[0].middle(), &[0u32, 2]);
        assert_eq!(cospans[1].middle(), &[2u32, 5]);
        assert_eq!(cospans[2].middle(), &[5u32, 7]);
    }

    #[test]
    fn test_average_complexity() {
        let intervals = vec![
            DiscreteInterval::new(0, 2),
            DiscreteInterval::new(2, 6),
            DiscreteInterval::new(6, 8),
        ];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let result = complex.verify_conservation();
        assert!((result.average_complexity() - 8.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_single_interval_cospan() {
        let intervals = vec![DiscreteInterval::new(5, 12)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let cospans = complex.to_cospan_chain();
        assert_eq!(cospans.len(), 1);
        assert_eq!(cospans[0].middle(), &[5u32, 12]);
    }
}
