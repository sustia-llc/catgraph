//! Shared `proptest` float strategies.
//!
//! [`wide_range_f64`] draws over the full `f64` exponent range;
//! [`near_cancellation_pair`] draws pairs whose subtraction cancels the leading
//! digits. Both yield **finite** values only: NaN and infinity are excluded.

use proptest::prelude::*;

/// Mantissa mask: clearing the exponent and sign bits of an `f64` leaves a
/// value in the subnormal range, or zero.
const MANTISSA_MASK: u64 = 0x000F_FFFF_FFFF_FFFF;

/// Edge-case magnitudes drawn from explicitly.
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
/// Four sources, mixed at these weights:
///
/// 1. **Bit-pattern uniform** (weight 3) — a random `u64` reinterpreted as an
///    `f64`, non-finite patterns rejected.
/// 2. **Subnormal** (weight 1) — a random mantissa with the exponent zeroed.
/// 3. **Moderate range** (weight 2) — the `-1e6..1e6` band.
/// 4. **Edge cases** (weight 1) — `±0.0`, `±1.0`, `±f64::EPSILON`, the smallest
///    normal and smallest subnormal of each sign, `f64::MAX` / `f64::MIN`, and
///    the two floats adjacent to `1.0`.
///
/// Shrinking runs through whichever branch produced the value; the bit-pattern
/// branch shrinks the underlying `u64`, which is not magnitude-monotonic, so a
/// failure from that branch can shrink to a differently-scaled counterexample.
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
/// `b = a · (1 + δ)`, with `a` from [`wide_range_f64`] and `|δ|` drawn
/// **log-uniformly** from `[1e-17, 1e-8)` — the exponent is sampled, so each
/// decade of the band carries equal weight.
///
/// # Guarantees
///
/// - Both components are finite: pairs where `a · (1 + δ)` leaves the finite
///   range are rejected.
/// - `a != b`: pairs where the product rounds back to `a` — `a = ±0.0`, and
///   subnormal or tiny `a`, where `a · δ` falls below half an ULP — are
///   rejected.
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
            prop_assert!(a != b, "pair is degenerate: a == b == {a:e}");

            let relative = ((a - b) / a).abs();
            prop_assert!(
                relative <= 1e-7,
                "pair is not near-cancelling: a = {a}, b = {b}, rel = {relative:e}"
            );
        }
    }

    /// 20 000 draws from [`wide_range_f64`]: over 100 distinct subnormals, at
    /// least one magnitude above `1e100`, and at least one below `1e6`.
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
        // values satisfy a bare count on their own.
        assert!(
            subnormals.len() > 100,
            "wide_range_f64 produced only {} distinct subnormal(s) — the \
             subnormal branch is not doing its job",
            subnormals.len()
        );
        assert!(huge > 0, "no magnitude above 1e100");
        assert!(ordinary > 0, "the moderate band is not being sampled");
    }

    /// 20 000 draws from [`near_cancellation_pair`]: over 200 pairs with
    /// relative separation below `1e-12`, and at least one below `1e-15`.
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
