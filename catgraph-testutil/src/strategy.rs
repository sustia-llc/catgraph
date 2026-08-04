//! Shared `proptest` float strategies (#169).
//!
//! Before this module the workspace had exactly one *named* float strategy —
//! `finite_f64() = -1e6f64..1e6f64` in `catgraph-dl/tests/common/mod.rs` — plus
//! roughly a dozen inline `lo..hi` ranges and `select` lists of the same shape,
//! spread across `catgraph-magnitude`, `catgraph-applied`, and `catgraph-dl`
//! (e.g. `-10.0f64..10.0`, `0.5f64..3.0`, `0.0_f64..=1.0`, `-1e-7_f64..1e-7`,
//! `-100.0f64..100.0`). Every one is **uniform-magnitude**: they never produce
//! subnormals, values near [`f64::MAX`], or operands spanning many orders of
//! magnitude, so the numerically-delicate code paths were only ever exercised
//! on benign inputs.
//!
//! [`wide_range_f64`] fixes the coverage gap; [`near_cancellation_pair`]
//! targets the specific regime the workspace's guards are written against —
//! subtracting two nearly equal numbers, where the leading digits cancel and
//! the result is dominated by rounding noise.
//!
//! Both strategies yield **finite** values only. NaN and infinity are excluded
//! deliberately: the consumers pair these with
//! [`approx_rel`](crate::approx_rel), whose contract makes NaN unequal to
//! everything, so admitting them would test the tolerance helper rather than
//! the code under test.
//!
//! # Coverage is measured, not assumed
//!
//! Each claim below about what these strategies reach is backed by a unit test
//! that samples and counts, and fails if the claim stops holding. That is
//! deliberate: the first draft of this module asserted coverage it did not have
//! (the subnormal range was represented by two literal edge values, and
//! `near_cancellation_pair` produced `a == b` for ~5% of samples).

use proptest::prelude::*;

/// Mantissa mask — clearing the exponent and sign bits of an `f64` leaves a
/// value in the subnormal range (or zero).
const MANTISSA_MASK: u64 = 0x000F_FFFF_FFFF_FFFF;

/// Representative edge-case magnitudes, sampled explicitly because a random
/// strategy essentially never hits them.
const EDGE_CASES: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    f64::EPSILON,
    -f64::EPSILON,
    f64::MIN_POSITIVE,        // smallest normal, ~2.2e-308
    -f64::MIN_POSITIVE,       //
    5e-324,                   // smallest positive subnormal
    -5e-324,                  //
    f64::MAX,                 //
    f64::MIN,                 // = -f64::MAX
    1.0 - f64::EPSILON / 2.0, // largest f64 below 1.0
    1.0 + f64::EPSILON,       // smallest f64 above 1.0
];

/// A finite `f64` spanning the **full** exponent range, including subnormals
/// and magnitudes near [`f64::MAX`].
///
/// Four sources are mixed so the strategy is neither all-extremes nor
/// all-benign:
///
/// 1. **Bit-pattern uniform** (weight 3) — reinterprets a random `u64` as an
///    `f64` and rejects the non-finite patterns. This is what reaches the huge
///    exponents. The reason a `lo..hi` range cannot is distributional, not
///    mechanical: a value-uniform range spreads its mass evenly over *values*,
///    so all but a vanishing fraction of draws land in the top decade of the
///    range, while sampling bit patterns spreads mass evenly over *exponents*.
/// 2. **Subnormal** (weight 1) — a random mantissa with the exponent zeroed.
///    This branch, **not** the bit-pattern one, is what actually supplies
///    subnormal coverage: only about `2⁻¹¹ ≈ 0.05%` of finite bit patterns are
///    subnormal, so branch 1 alone yields a handful per 20k draws, and those
///    would be swamped by the two literal subnormals in branch 4. Measured by
///    `wide_range_covers_each_regime`.
/// 3. **Moderate range** (weight 2) — the `-1e6..1e6` band the existing
///    workspace strategies use, kept so ordinary arithmetic still dominates
///    enough of the sample to catch ordinary bugs.
/// 4. **Edge cases** (weight 1) — `±0.0`, `±1.0`, `±f64::EPSILON`, the smallest
///    normal and smallest subnormal of each sign, `f64::MAX` / `f64::MIN`, and
///    the two floats adjacent to `1.0`, sampled explicitly because a random
///    strategy essentially never hits them.
///
/// Shrinking works through whichever branch produced the value; the bit-pattern
/// branch shrinks the underlying `u64`, which is not magnitude-monotonic, so a
/// failure from that branch may shrink to a differently-scaled counterexample
/// than you expect. That is a fair trade for reaching the regime at all.
pub fn wide_range_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => any::<u64>()
            .prop_map(f64::from_bits)
            .prop_filter("finite", |x: &f64| x.is_finite()),
        1 => (any::<u64>(), any::<bool>())
            .prop_map(|(bits, negative)| {
                let magnitude = f64::from_bits(bits & MANTISSA_MASK);
                if negative { -magnitude } else { magnitude }
            }),
        2 => -1e6f64..1e6f64,
        1 => proptest::sample::select(EDGE_CASES),
    ]
}

/// A pair `(a, b)` of **distinct** finite floats that are nearly equal, so that
/// `a − b` suffers catastrophic cancellation.
///
/// `b = a · (1 + δ)` with `|δ|` drawn **log-uniformly** from `[1e-17, 1e-8)`.
/// The log-uniform draw is load-bearing: a linear range over those nine decades
/// puts ~99.99% of its mass in the top decade `[1e-9, 1e-8)`, so the small-`δ`
/// end — where the subtraction destroys nearly the whole significand — is
/// effectively unreachable. Sampling the exponent instead gives each decade
/// equal weight.
///
/// `a` comes from [`wide_range_f64`], so the pair is near-cancelling at *any*
/// scale rather than only near `1.0`.
///
/// # Guarantees
///
/// - Both components are finite. Pairs where `a · (1 + δ)` leaves the finite
///   range are rejected, so `a` near [`f64::MAX`] does not silently yield an
///   infinite `b`.
/// - `a != b`. Without this filter the strategy degenerates on two families:
///   `a = ±0.0` (the product is again zero) and subnormal or tiny `a`, where
///   `a · δ` falls below half an ULP and the product rounds straight back to
///   `a`. Together those were ~5% of samples before the filter, and the pair is
///   not a cancellation at all when it happens.
///
/// The workspace guard this models directly is `coalition_eval`'s Schur
/// complement `s = 1 − vᵀμu` — a genuine two-operand cancellation residue,
/// carrying a 15-line rustdoc essay on exactly this failure mode.
///
/// The other two numerically-delicate guards are **related but differently
/// shaped**, and this strategy does not model their inputs: `magnitude`'s
/// Tsallis switch tests proximity of the *parameter* `t` to 1, and
/// `wasserstein`'s degenerate-basis repair compares flows against zero. Both
/// want their own generators (tracked in the follow-up issue split from #169).
pub fn near_cancellation_pair() -> impl Strategy<Value = (f64, f64)> {
    let delta =
        (1.0f64..10.0, 9i32..=17, any::<bool>()).prop_map(|(mantissa, exponent, negative)| {
            let magnitude = mantissa * 10f64.powi(-exponent);
            if negative { -magnitude } else { magnitude }
        });

    (wide_range_f64(), delta)
        .prop_map(|(a, d)| (a, a * (1.0 + d)))
        .prop_filter("finite and distinct", |(a, b)| {
            a.is_finite() && b.is_finite() && a != b
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    use std::collections::BTreeSet;

    /// Draw `count` values from `strategy` with a fixed seed.
    fn sample<T>(strategy: &impl Strategy<Value = T>, count: usize) -> Vec<T> {
        let mut runner = TestRunner::deterministic();
        (0..count)
            .map(|_| {
                strategy
                    .new_tree(&mut runner)
                    .expect("invariant: these strategies have no failing generation path")
                    .current()
            })
            .collect()
    }

    proptest! {
        #[test]
        fn wide_range_is_always_finite(x in wide_range_f64()) {
            prop_assert!(x.is_finite(), "strategy yielded {x}");
        }

        #[test]
        fn near_cancellation_pairs_are_finite_and_close(
            (a, b) in near_cancellation_pair()
        ) {
            prop_assert!(a.is_finite() && b.is_finite());
            // `a != b` is the property that makes the pair a *cancellation*
            // rather than a no-op. Without it the strategy silently degenerates:
            // `a = 0.0` gives `b = 0.0`, and for a subnormal `a` the product
            // `a·(1 + δ)` rounds straight back to `a`.
            prop_assert!(a != b, "pair is degenerate: a == b == {a:e}");

            let relative = ((a - b) / a).abs();
            prop_assert!(
                relative <= 1e-7,
                "pair is not near-cancelling: a = {a}, b = {b}, rel = {relative:e}"
            );
        }
    }

    /// The coverage claims in [`wide_range_f64`]'s docs, checked rather than
    /// asserted — including the claim that the *subnormal branch*, not the
    /// bit-pattern branch or the two literal edge values, is what carries
    /// subnormal coverage.
    #[test]
    fn wide_range_covers_each_regime() {
        let values = sample(&wide_range_f64(), 20_000);

        let mut subnormals = BTreeSet::new();
        let mut huge = 0_usize;
        let mut ordinary = 0_usize;

        for &value in &values {
            if value != 0.0 && value.abs() < f64::MIN_POSITIVE {
                subnormals.insert(value.to_bits());
            } else if value.abs() > 1e100 {
                huge += 1;
            } else if value.abs() < 1e6 {
                ordinary += 1;
            }
        }

        // Distinct subnormals, not a count: the two literal `±5e-324` edge
        // values would otherwise satisfy a bare count on their own, and the
        // first draft of this module passed exactly that way.
        assert!(
            subnormals.len() > 100,
            "wide_range_f64 produced only {} distinct subnormal(s) — the \
             subnormal branch is not doing its job",
            subnormals.len()
        );
        assert!(huge > 0, "no magnitude above 1e100");
        assert!(ordinary > 0, "the moderate band is not being sampled");
    }

    /// The log-uniform claim in [`near_cancellation_pair`]'s docs: every decade
    /// of the `δ` band must be reachable, not just the top one.
    #[test]
    fn near_cancellation_reaches_every_decade_of_the_delta_band() {
        let pairs = sample(&near_cancellation_pair(), 20_000);

        let mut tightest = f64::INFINITY;
        let mut below_1e12 = 0_usize;
        let mut below_1e15 = 0_usize;

        for &(a, b) in &pairs {
            assert!(a != b, "degenerate pair survived the filter: {a:e}");
            let relative = ((a - b) / a).abs();
            tightest = tightest.min(relative);
            if relative < 1e-12 {
                below_1e12 += 1;
            }
            if relative < 1e-15 {
                below_1e15 += 1;
            }
        }

        // A linear range over [1e-17, 1e-8) puts ~99.99% of its mass in the top
        // decade; the measured counts here were 7 and 0 per 50k before the
        // switch to a log-uniform draw.
        assert!(
            below_1e12 > 200,
            "only {below_1e12} pairs separated by less than 1e-12 — the delta \
             band is not being sampled log-uniformly"
        );
        assert!(
            below_1e15 > 0,
            "no pair reached the sub-1e-15 regime (tightest = {tightest:e}), \
             where the subtraction destroys essentially the whole significand"
        );
    }
}
