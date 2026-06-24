//! Generic trace analysis for step-by-step computational evolution.
//!
//! The [`StepTrace`] trait provides a common interface for any system that
//! evolves in discrete steps (cellular automata, multiway evolution, Petri net
//! firing sequences, agent coalitions). [`analyze_trace`] performs contiguity
//! checking, repeat detection, and complexity ratio computation over any
//! implementor.
//!
//! [`StepTrace`] captures the *structural* concern (fingerprints, intervals,
//! halt status). The interpretive judgment "is this computation
//! Wolfram-irreducible?" lives in the free function [`is_irreducible`], which
//! ties the term to Gorard 2023 in its rustdoc; downstream coalition consumers
//! that don't need the Wolfram framing can use the trait + `analyze_trace`
//! without it.

use crate::interval::DiscreteInterval;
use std::collections::HashMap;
use std::fmt;

/// Common interface for execution histories that evolve in discrete steps.
///
/// Implementors expose state fingerprints, discrete intervals, and step
/// metadata. [`analyze_trace`] then performs the shared contiguity check,
/// repeat detection, and complexity ratio computation.
pub trait StepTrace {
    /// Return the fingerprint of the initial state followed by the fingerprint
    /// after each transition, in order. The first element is the initial state
    /// (step 0); subsequent elements are states after steps 1, 2, ...
    fn state_fingerprints(&self) -> Vec<u64>;

    /// Map each transition to a discrete interval of the discrete-time category.
    fn to_intervals(&self) -> Vec<DiscreteInterval>;

    /// The number of transitions (steps) executed.
    fn step_count(&self) -> usize;

    /// Whether the computation halted naturally (vs. hitting a step limit).
    fn halted(&self) -> bool;
}

/// A repeated state detected during an execution trace.
///
/// When the same fingerprint appears at two different steps, the computation
/// could theoretically "jump" from the first occurrence to the second,
/// indicating potential reducibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatDetection {
    /// Step number of the first occurrence.
    pub start_step: usize,
    /// Step number of the repeated occurrence.
    pub end_step: usize,
    /// Number of steps in the cycle (`end_step - start_step`).
    pub cycle_length: usize,
}

impl fmt::Display for RepeatDetection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repeat: step {} → step {} (length {})",
            self.start_step, self.end_step, self.cycle_length
        )
    }
}

/// Detect repeated states from an iterator of `(step, fingerprint)` pairs.
///
/// The iterator should yield `(0, initial_fp)` followed by `(1, fp_after_step_1)`, etc.
/// Returns all instances where a fingerprint was seen at an earlier step.
pub fn detect_repeats(fingerprints: impl Iterator<Item = (usize, u64)>) -> Vec<RepeatDetection> {
    let mut seen: HashMap<u64, usize> = HashMap::new();
    let mut repeats = Vec::new();

    for (step, fp) in fingerprints {
        if let Some(&prev_step) = seen.get(&fp) {
            repeats.push(RepeatDetection {
                start_step: prev_step,
                end_step: step,
                cycle_length: step - prev_step,
            });
        }
        seen.insert(fp, step);
    }

    repeats
}

/// Result of generic trace analysis.
#[derive(Clone, Debug)]
pub struct TraceAnalysis {
    /// Whether the interval sequence is contiguous *and* no repeated states
    /// were detected — the structural prerequisite of Wolfram-irreducibility.
    /// See [`is_irreducible`] for the interpretive wrapper.
    pub is_irreducible: bool,
    /// Whether the interval sequence is contiguous (composable end-to-end).
    pub is_sequence_contiguous: bool,
    /// The total composed interval `[0, n]`, or `None` if not composable.
    pub total_interval: Option<DiscreteInterval>,
    /// All detected repeated states.
    pub repeats: Vec<RepeatDetection>,
    /// Ratio of actual steps to the minimum required (steps before first repeat).
    pub complexity_ratio: f64,
    /// Total number of steps executed.
    pub step_count: usize,
}

impl fmt::Display for TraceAnalysis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Trace Analysis:")?;
        writeln!(f, "  Steps: {}", self.step_count)?;
        writeln!(f, "  Is irreducible: {}", self.is_irreducible)?;
        writeln!(f, "  Sequence contiguous: {}", self.is_sequence_contiguous)?;
        if let Some(ref interval) = self.total_interval {
            writeln!(f, "  Total interval: {interval}")?;
        }
        writeln!(f, "  Repeats found: {}", self.repeats.len())?;
        for repeat in &self.repeats {
            writeln!(f, "    - {repeat}")?;
        }
        writeln!(f, "  Complexity ratio: {:.3}", self.complexity_ratio)?;
        Ok(())
    }
}

/// Perform shared structural analysis on any [`StepTrace`] implementor.
///
/// 1. Checks interval contiguity (each interval's end == next interval's start)
/// 2. Detects repeated states via [`detect_repeats`]
/// 3. Computes the complexity ratio (actual steps / steps before first repeat)
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn analyze_trace(trace: &impl StepTrace) -> TraceAnalysis {
    let intervals = trace.to_intervals();

    let is_sequence_contiguous = intervals.windows(2).all(|w| w[0].is_composable_with(&w[1]));

    let total_interval = if intervals.is_empty() {
        None
    } else {
        let mut result = intervals[0];
        let mut ok = true;
        for interval in &intervals[1..] {
            if let Some(composed) = result.then(*interval) {
                result = composed;
            } else {
                ok = false;
                break;
            }
        }
        if ok { Some(result) } else { None }
    };

    let fingerprints = trace.state_fingerprints();
    let repeats = detect_repeats(fingerprints.iter().copied().enumerate());

    let actual_steps = trace.step_count();
    let min_steps = if repeats.is_empty() {
        actual_steps
    } else {
        repeats
            .iter()
            .map(|r| r.start_step)
            .min()
            .unwrap_or(actual_steps)
    };
    let complexity_ratio = if min_steps == 0 {
        1.0
    } else {
        actual_steps as f64 / min_steps as f64
    };

    TraceAnalysis {
        is_irreducible: is_sequence_contiguous && repeats.is_empty(),
        is_sequence_contiguous,
        total_interval,
        repeats,
        complexity_ratio,
        step_count: actual_steps,
    }
}

/// Wolfram-irreducibility judgment on a structural trace.
///
/// A trace is *Wolfram-irreducible* (per Gorard 2023, *A Functorial Perspective
/// on (Multi)computational Irreducibility*) when its evolution cannot be
/// shortcut to an equivalent shorter computation. The structural prerequisite
/// is contiguous interval composition and no repeated states; this function
/// returns the same boolean as [`TraceAnalysis::is_irreducible`].
///
/// Coalition / replay consumers that want the structural fact without buying
/// into the Wolfram framing should use [`analyze_trace`] directly and read
/// `analysis.is_sequence_contiguous && analysis.repeats.is_empty()`.
pub fn is_irreducible(trace: &impl StepTrace) -> bool {
    analyze_trace(trace).is_irreducible
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubTrace {
        fingerprints: Vec<u64>,
        intervals: Vec<DiscreteInterval>,
        halted: bool,
    }

    impl StepTrace for StubTrace {
        fn state_fingerprints(&self) -> Vec<u64> {
            self.fingerprints.clone()
        }
        fn to_intervals(&self) -> Vec<DiscreteInterval> {
            self.intervals.clone()
        }
        fn step_count(&self) -> usize {
            self.intervals.len()
        }
        fn halted(&self) -> bool {
            self.halted
        }
    }

    #[test]
    fn detect_repeats_no_repeats() {
        let fps = vec![(0, 100u64), (1, 200), (2, 300)];
        let repeats = detect_repeats(fps.into_iter());
        assert!(repeats.is_empty());
    }

    #[test]
    fn detect_repeats_single_cycle() {
        let fps = vec![(0, 100u64), (1, 200), (2, 100)];
        let repeats = detect_repeats(fps.into_iter());
        assert_eq!(repeats.len(), 1);
        assert_eq!(repeats[0].start_step, 0);
        assert_eq!(repeats[0].end_step, 2);
        assert_eq!(repeats[0].cycle_length, 2);
    }

    #[test]
    fn detect_repeats_multiple_cycles() {
        let fps = vec![(0, 10u64), (1, 20), (2, 10), (3, 20)];
        let repeats = detect_repeats(fps.into_iter());
        assert_eq!(repeats.len(), 2);
        assert_eq!(
            repeats[0],
            RepeatDetection {
                start_step: 0,
                end_step: 2,
                cycle_length: 2
            }
        );
        assert_eq!(
            repeats[1],
            RepeatDetection {
                start_step: 1,
                end_step: 3,
                cycle_length: 2
            }
        );
    }

    #[test]
    fn detect_repeats_empty() {
        let repeats = detect_repeats(std::iter::empty());
        assert!(repeats.is_empty());
    }

    #[test]
    fn analyze_trace_irreducible() {
        let trace = StubTrace {
            fingerprints: vec![1, 2, 3, 4],
            intervals: vec![
                DiscreteInterval::new(0, 1),
                DiscreteInterval::new(1, 2),
                DiscreteInterval::new(2, 3),
            ],
            halted: true,
        };
        let analysis = analyze_trace(&trace);
        assert!(analysis.is_irreducible);
        assert!(analysis.is_sequence_contiguous);
        assert!(analysis.repeats.is_empty());
        assert_eq!(analysis.step_count, 3);
        assert!((analysis.complexity_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn analyze_trace_with_cycle() {
        let trace = StubTrace {
            fingerprints: vec![1, 2, 1],
            intervals: vec![DiscreteInterval::new(0, 1), DiscreteInterval::new(1, 2)],
            halted: false,
        };
        let analysis = analyze_trace(&trace);
        assert!(!analysis.is_irreducible);
        assert!(analysis.is_sequence_contiguous);
        assert_eq!(analysis.repeats.len(), 1);
    }

    #[test]
    fn analyze_trace_non_contiguous() {
        let trace = StubTrace {
            fingerprints: vec![1, 2, 3],
            intervals: vec![DiscreteInterval::new(0, 1), DiscreteInterval::new(3, 4)],
            halted: true,
        };
        let analysis = analyze_trace(&trace);
        assert!(!analysis.is_irreducible);
        assert!(!analysis.is_sequence_contiguous);
    }

    #[test]
    fn analyze_trace_empty() {
        let trace = StubTrace {
            fingerprints: vec![42],
            intervals: vec![],
            halted: true,
        };
        let analysis = analyze_trace(&trace);
        assert!(analysis.is_irreducible);
        assert!(analysis.repeats.is_empty());
        assert_eq!(analysis.step_count, 0);
        assert!(analysis.total_interval.is_none());
    }

    #[test]
    fn repeat_detection_display() {
        let r = RepeatDetection {
            start_step: 3,
            end_step: 7,
            cycle_length: 4,
        };
        let s = format!("{r}");
        assert!(s.contains("step 3"));
        assert!(s.contains("step 7"));
        assert!(s.contains("length 4"));
    }

    #[test]
    fn trace_analysis_display() {
        let trace = StubTrace {
            fingerprints: vec![1, 2, 3],
            intervals: vec![DiscreteInterval::new(0, 1), DiscreteInterval::new(1, 2)],
            halted: true,
        };
        let analysis = analyze_trace(&trace);
        let s = format!("{analysis}");
        assert!(s.contains("Steps: 2"));
        assert!(s.contains("Is irreducible: true"));
    }

    #[test]
    fn is_irreducible_free_fn_matches_analysis_field() {
        let trace = StubTrace {
            fingerprints: vec![1, 2, 3, 4],
            intervals: vec![
                DiscreteInterval::new(0, 1),
                DiscreteInterval::new(1, 2),
                DiscreteInterval::new(2, 3),
            ],
            halted: true,
        };
        assert!(is_irreducible(&trace));
        assert_eq!(is_irreducible(&trace), analyze_trace(&trace).is_irreducible);
    }

    #[test]
    fn is_irreducible_false_on_cycle() {
        let trace = StubTrace {
            fingerprints: vec![1, 2, 1],
            intervals: vec![DiscreteInterval::new(0, 1), DiscreteInterval::new(1, 2)],
            halted: false,
        };
        assert!(!is_irreducible(&trace));
    }
}
