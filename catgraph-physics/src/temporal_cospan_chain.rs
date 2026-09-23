//! Temporal cospan chain bridge.
//!
//! Maps interval sequences to composable cospan chains in the discrete-time
//! category. Builds a 1D simplicial complex from interval sequences
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
/// - 1-skeleton: one edge per input interval, in input order
#[derive(Debug, Clone)]
pub struct TemporalComplex {
    /// Input intervals as `(start, end)`, in input order.
    intervals: Vec<(usize, usize)>,
    /// Interval endpoints in input order, consecutive equal points merged.
    time_points: Vec<usize>,
    /// `end - start` (saturating) of each input interval, in input order.
    step_counts: Vec<usize>,
}

impl TemporalComplex {
    /// Creates a temporal complex from a sequence of intervals, kept in input
    /// order.
    ///
    /// # Errors
    ///
    /// Returns [`TemporalComplexError::EmptyIntervals`] if the interval slice is
    /// empty, or [`TemporalComplexError::InsufficientPoints`] if the endpoint
    /// sequence, with consecutive equal points merged, has fewer than two
    /// points.
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

        time_points.dedup();

        if time_points.len() < 2 {
            return Err(TemporalComplexError::InsufficientPoints(time_points.len()));
        }

        let intervals: Vec<(usize, usize)> = intervals.iter().map(|i| (i.start, i.end)).collect();
        let step_counts: Vec<usize> = intervals
            .iter()
            .map(|&(start, end)| end.saturating_sub(start))
            .collect();

        Ok(Self {
            intervals,
            time_points,
            step_counts,
        })
    }

    /// Returns the number of time points (vertices).
    #[inline]
    #[must_use]
    pub fn num_time_steps(&self) -> usize {
        self.time_points.len()
    }

    /// Returns the number of input intervals (edges).
    #[inline]
    #[must_use]
    pub fn num_intervals(&self) -> usize {
        self.intervals.len()
    }

    /// Returns the interval endpoints in input order, consecutive equal points
    /// merged.
    #[inline]
    #[must_use]
    pub fn time_points(&self) -> &[usize] {
        &self.time_points
    }

    /// Returns `end - start` (saturating) of each input interval, in input
    /// order.
    #[inline]
    #[must_use]
    pub fn step_counts(&self) -> &[usize] {
        &self.step_counts
    }

    /// Converts the interval sequence to a 1-form: the step counts as `f64`, in
    /// input order.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn intervals_to_form(&self) -> Vec<f64> {
        self.step_counts.iter().map(|&s| s as f64).collect()
    }

    /// Integrates a 1-form over the full chain: the sum of its coefficients.
    #[must_use]
    pub fn integrate(&self, form: &[f64]) -> f64 {
        form.iter().sum()
    }

    /// Checks the input interval sequence for contiguity and monotonicity and
    /// reports the span `last.end - first.start` of the chain.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn verify_conservation(&self) -> ConservationResult {
        let is_contiguous = self.intervals.windows(2).all(|w| w[0].1 == w[1].0);
        let is_monotonic = self.intervals.iter().all(|&(start, end)| start <= end)
            && self.intervals.windows(2).all(|w| w[0].0 <= w[1].0);
        let &(first_start, _) = self
            .intervals
            .first()
            .expect("invariant: from_intervals rejects an empty interval slice");
        let &(_, last_end) = self
            .intervals
            .last()
            .expect("invariant: from_intervals rejects an empty interval slice");
        let total_complexity = last_end as f64 - first_start as f64;

        ConservationResult {
            is_conserved: is_contiguous && is_monotonic,
            is_contiguous,
            is_monotonic,
            total_complexity,
            num_intervals: self.num_intervals(),
            num_time_steps: self.num_time_steps(),
        }
    }

    /// Converts the temporal complex into one cospan per input interval, in
    /// input order, each with middle `[start, end]`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    #[must_use]
    pub fn to_cospan_chain(&self) -> Vec<Cospan<u32>> {
        let mut cospans = Vec::new();

        for &(start, end) in &self.intervals {
            let t_start = start as u32;
            let t_end = end as u32;
            let left = vec![0];
            let right = vec![1];
            let middle = vec![t_start, t_end];

            // Correct by construction: literal legs `[0]` / `[1]` into a literal
            // two-vertex apex.
            cospans.push(Cospan::new_unchecked(left, right, middle));
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
    /// `is_contiguous && is_monotonic`.
    pub is_conserved: bool,
    /// Each interval ends where the next starts.
    pub is_contiguous: bool,
    /// Every interval has `start <= end` and each interval starts no earlier
    /// than the one before it.
    pub is_monotonic: bool,
    /// `last.end - first.start` of the input sequence, as `f64`; negative when
    /// the last interval ends before the first starts.
    pub total_complexity: f64,
    /// Number of input intervals.
    pub num_intervals: usize,
    /// Number of time points (vertices).
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

    #[test]
    fn test_conservation_gap_detected() {
        let intervals = vec![DiscreteInterval::new(0, 2), DiscreteInterval::new(5, 7)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let r = complex.verify_conservation();
        let integral = complex.integrate(&complex.intervals_to_form());
        assert!(
            !r.is_conserved,
            "gap [0,2],[5,7]: is_conserved = {} (expected false)",
            r.is_conserved
        );
        assert!(
            !r.is_contiguous,
            "gap [0,2],[5,7]: is_contiguous = {} (expected false)",
            r.is_contiguous
        );
        assert!(
            r.is_monotonic,
            "gap [0,2],[5,7]: is_monotonic = {} (expected true)",
            r.is_monotonic
        );
        assert!(
            (integral - 4.0).abs() < 1e-10 && (r.total_complexity - 7.0).abs() < 1e-10,
            "gap [0,2],[5,7]: integral = {integral}, total_complexity = {} (expected 4 vs 7)",
            r.total_complexity
        );
    }

    #[test]
    fn test_conservation_overlap_detected() {
        let intervals = vec![DiscreteInterval::new(0, 3), DiscreteInterval::new(2, 5)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let r = complex.verify_conservation();
        let integral = complex.integrate(&complex.intervals_to_form());
        assert!(
            !r.is_conserved,
            "overlap [0,3],[2,5]: is_conserved = {} (expected false)",
            r.is_conserved
        );
        assert!(
            !r.is_contiguous,
            "overlap [0,3],[2,5]: is_contiguous = {} (expected false)",
            r.is_contiguous
        );
        assert!(
            r.is_monotonic,
            "overlap [0,3],[2,5]: is_monotonic = {} (expected true)",
            r.is_monotonic
        );
        assert!(
            (integral - 6.0).abs() < 1e-10 && (r.total_complexity - 5.0).abs() < 1e-10,
            "overlap [0,3],[2,5]: integral = {integral}, total_complexity = {} (expected 6 vs 5)",
            r.total_complexity
        );
    }

    #[test]
    fn test_conservation_out_of_order_detected() {
        let intervals = vec![DiscreteInterval::new(2, 4), DiscreteInterval::new(0, 2)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let r = complex.verify_conservation();
        let integral = complex.integrate(&complex.intervals_to_form());
        assert!(
            !r.is_conserved,
            "out of order [2,4],[0,2]: is_conserved = {} (expected false)",
            r.is_conserved
        );
        assert!(
            !r.is_monotonic,
            "out of order [2,4],[0,2]: is_monotonic = {} (expected false)",
            r.is_monotonic
        );
        assert!(
            (integral - 4.0).abs() < 1e-10 && r.total_complexity.abs() < 1e-10,
            "out of order [2,4],[0,2]: integral = {integral}, total_complexity = {} (expected 4 vs 0)",
            r.total_complexity
        );
    }

    #[test]
    fn test_conservation_contiguous_telescopes() {
        let intervals = vec![DiscreteInterval::new(0, 2), DiscreteInterval::new(2, 4)];
        let complex = TemporalComplex::from_intervals(&intervals).unwrap();
        let r = complex.verify_conservation();
        let integral = complex.integrate(&complex.intervals_to_form());
        assert!(
            r.is_conserved,
            "[0,2],[2,4]: is_conserved = {} (expected true)",
            r.is_conserved
        );
        assert!(
            r.is_contiguous,
            "[0,2],[2,4]: is_contiguous = {} (expected true)",
            r.is_contiguous
        );
        assert!(
            r.is_monotonic,
            "[0,2],[2,4]: is_monotonic = {} (expected true)",
            r.is_monotonic
        );
        assert!(
            (integral - 4.0).abs() < 1e-10 && (r.total_complexity - 4.0).abs() < 1e-10,
            "[0,2],[2,4]: integral = {integral}, total_complexity = {} (expected 4 == 4)",
            r.total_complexity
        );
    }
}
