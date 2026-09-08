//! `CoalitionEvaluator` parity with fresh evaluation over a generated coalition
//! family, covering the borders the `SCHUR_SLOW_FALLBACK_TOL` guard diverts.
//!
//! Lineage: generalizes the `coalition_eval` unit pins
//! `value_with_report_error_parity` (error parity on one 3-agent fixture) and
//! `value_delta_matches_two_fresh_calls` (delta base on one 4-agent fixture)
//! from fixed fixtures to a generated instance family (#225).
//!
//! An instance is `3..=6` agents, a member set `S` with both `S` and its
//! complement non-empty, and a coupling per ordered pair `(i, j)`, `i ≠ j`,
//! present with probability one half. A weight comes from two arms: a
//! `wide_range_f64` magnitude clamped into `[0, 1]` and snapped to `0.0` below
//! [`WEIGHT_FLOOR`], and the moderate `0.0..=1.0` band. A third arm replaces
//! one candidate's couplings by `(member, candidate, 1 − δ)` and
//! `(candidate, member, 1.0)`, with `δ` the relative separation of a
//! `near_cancellation_pair` draw.
//!
//! Every evaluation is at `t = 1`, the scale `coalition_value` and
//! `coalition_value_delta` pin.

use catgraph_magnitude::{
    CoalitionEvaluator, EvalPath, EvalScratch, INCREMENTAL_REL_TOL, coalition_value,
    coalition_value_delta,
};
use catgraph_testutil::approx_rel;
use catgraph_testutil::strategy::{near_cancellation_pair, wide_range_f64};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

/// Instances drawn by [`value_with_parity_and_path_census`].
const DRAWS: usize = 1024;

/// Smallest non-zero coupling the `wide_range_f64` arm emits; smaller
/// magnitudes are snapped to `0.0`.
///
/// A closure entry is a product of at most six drawn couplings, so every
/// non-zero entry of a generated closure stays normal.
const WEIGHT_FLOOR: f64 = 1e-12;

/// Ceiling on `|s|` for a candidate whose incremental and fresh values differ
/// by more than [`INCREMENTAL_REL_TOL`], over a [`DRAWS`]-draw run.
const DISAGREEMENT_S_CEILING: f64 = 1e-8;

/// Ceiling on the number of compared candidates whose incremental and fresh
/// values differ by more than [`INCREMENTAL_REL_TOL`], over a [`DRAWS`]-draw
/// run.
const DISAGREEMENT_BOUND: usize = 160;

/// Ceiling on the largest incremental-vs-fresh relative gap over a
/// [`DRAWS`]-draw run.
const GAP_BOUND: f64 = 1e-4;

/// One generated coalition problem: the agent domain, the coupling table, and
/// the base coalition `S`.
#[derive(Clone, Debug)]
struct Instance {
    agents: Vec<usize>,
    couplings: Vec<(usize, usize, f64)>,
    members: Vec<usize>,
}

impl Instance {
    /// Agent indices outside `S`, ascending — the valid candidates.
    fn candidates(&self) -> Vec<usize> {
        (0..self.agents.len())
            .filter(|i| !self.members.contains(i))
            .collect()
    }

    /// `S ∪ {candidate}`, with the candidate last (magnitude is order-invariant).
    fn joined(&self, candidate: usize) -> Vec<usize> {
        let mut joined = self.members.clone();
        joined.push(candidate);
        joined
    }
}

/// A coupling probability in `[0, 1]`: a `wide_range_f64` magnitude clamped
/// into the domain [`CoalitionEvaluator::new`] accepts and floored at
/// [`WEIGHT_FLOOR`], or the moderate band.
fn coupling_weight() -> impl Strategy<Value = f64> {
    prop_oneof![
        1 => wide_range_f64().prop_map(|w| {
            let clamped = w.abs().clamp(0.0, 1.0);
            if clamped < WEIGHT_FLOOR { 0.0 } else { clamped }
        }),
        3 => 0.0f64..=1.0,
    ]
}

/// Assemble an [`Instance`] from the drawn parts.
///
/// `member_bits` selects `S`, corrected to keep `S` and its complement
/// non-empty. `weights` and `present` are indexed by ordered pair `(i, j)`,
/// `i ≠ j`, in row-major order. `perturb`, when present, clears the first
/// candidate's couplings and replaces them with the clone-arm pair against the
/// first member.
fn assemble(
    n: usize,
    member_bits: &[bool],
    weights: &[f64],
    present: &[bool],
    perturb: Option<(f64, f64)>,
) -> Instance {
    let agents: Vec<usize> = (0..n).collect();

    let mut members: Vec<usize> = (0..n).filter(|&i| member_bits[i]).collect();
    if members.is_empty() {
        members.push(0);
    }
    if members.len() == n {
        members.pop();
    }

    let mut couplings = Vec::new();
    let mut slot = 0usize;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if present[slot] {
                couplings.push((i, j, weights[slot]));
            }
            slot += 1;
        }
    }

    if let Some((a, b)) = perturb {
        let member = members[0];
        let candidate = (0..n)
            .find(|c| !members.contains(c))
            .expect("invariant: the correction above leaves a non-member");
        let delta = ((a - b) / a).abs();
        couplings.retain(|&(i, j, _)| i != candidate && j != candidate);
        couplings.push((member, candidate, 1.0 - delta));
        couplings.push((candidate, member, 1.0));
    }

    Instance {
        agents,
        couplings,
        members,
    }
}

/// The instance family the three properties range over.
fn instance() -> impl Strategy<Value = Instance> {
    (3usize..=6)
        .prop_flat_map(|n| {
            let pairs = n * (n - 1);
            (
                Just(n),
                prop::collection::vec(any::<bool>(), n),
                prop::collection::vec(coupling_weight(), pairs),
                prop::collection::vec(any::<bool>(), pairs),
                prop::option::of(near_cancellation_pair()),
            )
        })
        .prop_map(|(n, member_bits, weights, present, perturb)| {
            assemble(n, &member_bits, &weights, &present, perturb)
        })
}

/// `|a − b|` scaled by `max(|a|, |b|, 1)` — at most `tol` exactly when
/// `approx_rel(a, b, tol, tol)` holds.
fn relative_gap(a: f64, b: f64) -> f64 {
    if a == b {
        return 0.0;
    }
    (a - b).abs() / a.abs().max(b.abs()).max(1.0)
}

/// Bump `census`, indexed `[Fast, Slow, SlowNearSingular, MergeOnly, other]`.
///
/// [`EvalPath`] is `#[non_exhaustive]`, so a variant added upstream lands in
/// the trailing slot rather than failing to compile here.
fn tally(path: EvalPath, census: &mut [usize; 5]) {
    match path {
        EvalPath::Fast => census[0] += 1,
        EvalPath::Slow => census[1] += 1,
        EvalPath::SlowNearSingular => census[2] += 1,
        EvalPath::MergeOnly => census[3] += 1,
        _ => census[4] += 1,
    }
}

/// Property A — `value_with` against a fresh `coalition_value` on `S ∪ {x}`,
/// for every candidate of [`DRAWS`] instances drawn at proptest's fixed
/// deterministic seed.
///
/// Per candidate: the two routes are `Ok` together and `Err` together, and
/// where both are `Ok` both values are finite.
///
/// Over the run: [`CoalitionEvaluator::new`] accepts every drawn instance;
/// `Fast`, `Slow` and `SlowNearSingular` are each taken at least once and no
/// path outside the four named variants is taken; every candidate whose two
/// values differ by more than [`INCREMENTAL_REL_TOL`] took the `Fast` path
/// with `|s|` at most [`DISAGREEMENT_S_CEILING`]; there are at most
/// [`DISAGREEMENT_BOUND`] such candidates; and the largest relative gap is at
/// most [`GAP_BOUND`].
///
/// The run-level figures are accumulated across cases, which is why the sample
/// is driven directly rather than through `proptest!`.
#[test]
fn value_with_parity_and_path_census() {
    let strategy = instance();
    let mut runner = TestRunner::deterministic();

    let mut census = [0usize; 5];
    let mut rejected = 0usize;
    let mut candidates = 0usize;
    let mut compared = 0usize;
    let mut worst_gap = 0.0f64;
    let mut disagreements = 0usize;
    let mut worst_s = 0.0f64;

    for _ in 0..DRAWS {
        let instance = strategy
            .new_tree(&mut runner)
            .expect("invariant: no branch of `instance` fails generation")
            .current();

        let Ok(evaluator) = CoalitionEvaluator::new(
            &instance.agents,
            &instance.couplings,
            &instance.members,
            1.0,
        ) else {
            rejected += 1;
            continue;
        };

        for x in instance.candidates() {
            candidates += 1;
            let incremental = evaluator.value_with(x);
            let fresh = coalition_value(&instance.agents, &instance.couplings, &instance.joined(x));
            assert_eq!(
                incremental.is_err(),
                fresh.is_err(),
                "candidate {x}: value_with is_err = {}, fresh coalition_value is_err = {} \
                 (members {:?}, couplings {:?})",
                incremental.is_err(),
                fresh.is_err(),
                instance.members,
                instance.couplings
            );

            let (Ok(incremental), Ok(fresh)) = (incremental, fresh) else {
                continue;
            };
            let report = evaluator
                .value_with_report(x)
                .expect("invariant: value_with returned Ok on this candidate");
            tally(report.path(), &mut census);
            compared += 1;

            assert!(
                incremental.is_finite() && fresh.is_finite(),
                "candidate {x} on {:?}: incremental {incremental}, fresh {fresh} \
                 (members {:?}, couplings {:?})",
                report.path(),
                instance.members,
                instance.couplings
            );

            let gap = relative_gap(incremental, fresh);
            worst_gap = worst_gap.max(gap);
            if !approx_rel(incremental, fresh, INCREMENTAL_REL_TOL, INCREMENTAL_REL_TOL) {
                disagreements += 1;
                let s = report.schur_complement().map_or(f64::INFINITY, f64::abs);
                worst_s = worst_s.max(s);
                assert!(
                    report.path() == EvalPath::Fast && s <= DISAGREEMENT_S_CEILING,
                    "candidate {x} on {:?}: incremental {incremental} vs fresh {fresh}, \
                     gap {gap:e} > INCREMENTAL_REL_TOL {INCREMENTAL_REL_TOL:e} at |s| = {s:e}, \
                     expected the Fast path with |s| <= {DISAGREEMENT_S_CEILING:e} \
                     (members {:?}, couplings {:?})",
                    report.path(),
                    instance.members,
                    instance.couplings
                );
            }
        }
    }

    assert_eq!(
        rejected, 0,
        "CoalitionEvaluator::new rejected {rejected} of {DRAWS} drawn instances"
    );
    assert!(
        census[0] > 0 && census[1] > 0 && census[2] > 0 && census[4] == 0,
        "EvalPath census [Fast, Slow, SlowNearSingular, MergeOnly, other] = {census:?} over \
         {compared} compared of {candidates} candidates from {DRAWS} draws; expected each of \
         the first three positive and the last zero"
    );
    assert!(
        disagreements <= DISAGREEMENT_BOUND,
        "{disagreements} of {compared} compared candidates fell outside INCREMENTAL_REL_TOL \
         {INCREMENTAL_REL_TOL:e} (largest |s| among them {worst_s:e}), bound \
         {DISAGREEMENT_BOUND}"
    );
    assert!(
        worst_gap <= GAP_BOUND,
        "largest incremental-vs-fresh relative gap {worst_gap:e} over {compared} compared \
         candidates exceeds {GAP_BOUND:e}"
    );
}

proptest! {
    /// Property B — the four `value_with*` entry points agree on `is_err()` for
    /// every agent index and for two out-of-range indices, and the indices that
    /// name a member or lie out of range all error.
    #[test]
    fn error_parity_over_every_candidate(instance in instance()) {
        let n = instance.agents.len();
        if let Ok(evaluator) =
            CoalitionEvaluator::new(&instance.agents, &instance.couplings, &instance.members, 1.0)
        {
            let mut scratch = EvalScratch::new();
            for x in (0..n).chain([n, n + 7]) {
                let plain = evaluator.value_with(x).is_err();
                prop_assert_eq!(
                    plain,
                    evaluator.value_with_report(x).is_err(),
                    "candidate {}: value_with is_err = {}, value_with_report disagrees",
                    x,
                    plain
                );
                prop_assert_eq!(
                    plain,
                    evaluator.value_with_scratch(x, &mut scratch).is_err(),
                    "candidate {}: value_with is_err = {}, value_with_scratch disagrees",
                    x,
                    plain
                );
                prop_assert_eq!(
                    plain,
                    evaluator.value_with_report_scratch(x, &mut scratch).is_err(),
                    "candidate {}: value_with is_err = {}, value_with_report_scratch disagrees",
                    x,
                    plain
                );
                if x >= n || instance.members.contains(&x) {
                    prop_assert!(
                        plain,
                        "candidate {} is out of range or already a member, so it must error",
                        x
                    );
                }
            }
        }
    }

    /// Property C — `coalition_value_delta`'s base component is bitwise
    /// `coalition_value(S)` whenever the pair evaluates, and the pair never
    /// evaluates where `coalition_value(S)` errors.
    #[test]
    fn delta_base_is_bitwise_coalition_value(instance in instance()) {
        let base = coalition_value(&instance.agents, &instance.couplings, &instance.members);
        for x in instance.candidates() {
            let delta =
                coalition_value_delta(&instance.agents, &instance.couplings, &instance.members, x);
            match (&base, &delta) {
                (Ok(fresh), Ok((paired, _))) => prop_assert_eq!(
                    paired.to_bits(),
                    fresh.to_bits(),
                    "candidate {}: delta base {} vs coalition_value(S) {}",
                    x,
                    paired,
                    fresh
                ),
                (Err(fresh), Ok((paired, _))) => prop_assert!(
                    false,
                    "candidate {}: delta returned base {} where coalition_value(S) errored: {}",
                    x,
                    paired,
                    fresh
                ),
                _ => {}
            }
        }
    }
}
