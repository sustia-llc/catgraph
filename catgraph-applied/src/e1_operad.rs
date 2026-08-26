//! E1 (little intervals) operad: configurations of disjoint subintervals of \[0, 1\].
//!
//! Supports operadic substitution, coalescence, monoid homomorphism, and
//! minimum closeness between adjacent intervals.

use std::ops::MulAssign;

use itertools::Itertools;

use crate::F32_EPSILON;
use crate::rig::One;
use catgraph::{category::HasIdentity, errors::CatgraphError, operadic::Operadic};

use rand_core::Rng;

type IntervalCoord = f32;

/// One uniform sample from the `f32` grid of \[0, 1): the top 24 bits of a
/// single `u32` word, scaled by 2⁻²⁴.
///
/// In-tree because the published dependency is `rand_core` alone (#239), which
/// supplies raw words and no float distributions. Every value `k · 2⁻²⁴`
/// (`k < 2²⁴`) is exactly representable in [`IntervalCoord`], the maximum is
/// `1 − 2⁻²⁴`, and the range is genuinely half-open — the strict upper bound
/// is this function's postcondition, not a consumer tolerance. Finite by
/// construction.
#[inline]
#[allow(clippy::cast_precision_loss)] // k < 2^24 is exactly representable in f32
fn uniform_unit(rng: &mut impl Rng) -> IntervalCoord {
    (rng.next_u32() >> 8) as IntervalCoord * (1.0 / (1u32 << 24) as IntervalCoord)
}

/// An n-ary operation in the E1 operad: a configuration of `n` disjoint subintervals of \[0, 1\].
#[derive(Clone, Debug)]
pub struct E1 {
    arity: usize,
    sub_intervals: Vec<(IntervalCoord, IntervalCoord)>,
}

impl E1 {
    /// Arity of this configuration (number of sub-intervals).
    #[must_use]
    pub const fn arity(&self) -> usize {
        self.arity
    }

    /// Immutable view of the sub-intervals.
    #[must_use]
    pub fn sub_intervals(&self) -> &[(IntervalCoord, IntervalCoord)] {
        &self.sub_intervals
    }

    /// Create an n-ary E1 configuration from subintervals of \[0, 1\].
    ///
    /// When `overlap_check` is true, validates disjointness and sorts by left endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Operadic`] if any interval extends outside \[0, 1\] or overlaps
    /// when `overlap_check` is true.
    ///
    /// # Panics
    ///
    /// Panics if `partial_cmp` returns `None` for `IntervalCoord` — should not occur with finite floats.
    pub fn new(
        sub_intervals: Vec<(IntervalCoord, IntervalCoord)>,
        overlap_check: bool,
    ) -> Result<Self, CatgraphError> {
        for (a, b) in &sub_intervals {
            if *a >= *b - F32_EPSILON {
                return Err(CatgraphError::Operadic {
                    message: format!("Subinterval ({a}, {b}) has non-positive width"),
                });
            }
            if *a < -F32_EPSILON {
                return Err(CatgraphError::Operadic {
                    message: format!("Subinterval ({a}, {b}) starts below 0"),
                });
            }
            if *b > 1.0 + F32_EPSILON {
                return Err(CatgraphError::Operadic {
                    message: format!("Subinterval ({a}, {b}) ends above 1"),
                });
            }
        }
        if overlap_check {
            let mut new_sub_intervals = sub_intervals.clone();
            new_sub_intervals.sort_by(|i1, i2| i1.0.partial_cmp(&i2.0).unwrap());
            for ((_, b), (c, _)) in new_sub_intervals.iter().tuple_windows() {
                if *b >= *c + F32_EPSILON {
                    return Err(CatgraphError::Operadic {
                        message: "The subintervals cannot overlap".to_string(),
                    });
                }
            }
            Ok(Self {
                arity: sub_intervals.len(),
                sub_intervals: new_sub_intervals,
            })
        } else {
            Ok(Self {
                arity: sub_intervals.len(),
                sub_intervals,
            })
        }
    }

    /// Generate a random valid E1 configuration with the given arity.
    ///
    /// Draws `2 * cur_arity` samples uniformly from \[0, 1), sorts them, and pairs
    /// consecutive values into intervals. A raw draw can place adjacent sorted
    /// samples arbitrarily close, yielding a zero-width or sub-epsilon interval that
    /// [`E1::new`] rejects; to guarantee a valid result the whole batch is resampled
    /// until every adjacent pair of sorted coordinates is separated by more than
    /// `MIN_SEPARATION`.
    ///
    /// # Postconditions
    ///
    /// - The returned configuration has exactly `cur_arity` intervals (empty when
    ///   `cur_arity == 0`).
    /// - Every interval has width greater than `MIN_SEPARATION` (`2·F32_EPSILON`,
    ///   i.e. `2e-6`).
    /// - Intervals are pairwise strictly disjoint with gaps greater than
    ///   `MIN_SEPARATION`, formed by pairing sorted coordinates
    ///   `(s0, s1), (s2, s3), …`.
    ///
    /// # Panics
    ///
    /// - If `2 * cur_arity` (the draw count) overflows `usize`.
    /// - The sort's `partial_cmp` expect and the terminal `expect` document
    ///   invariants — samples are finite by construction (see `uniform_unit`),
    ///   resampled coordinates are strictly separated — and cannot fire.
    ///
    /// # Termination
    ///
    /// The separation resampling makes termination probabilistic, and it
    /// requires the generator to yield uniformly distributed words. The accept
    /// probability for `m = 2 * cur_arity` sorted uniform draws is roughly
    /// `(1 − m · MIN_SEPARATION)^m` — a negligible retry rate at small
    /// arities (≈ 1e-3 at arity 10), shrinking steeply once arities reach the
    /// low thousands, and effectively non-terminating well before the
    /// pigeonhole bound `m · MIN_SEPARATION > 1` (arity ≈ 250 000). A
    /// degenerate generator (constant or low-entropy words) collapses the
    /// draws and never separates. There is deliberately no iteration cap:
    /// supply a real RNG and practical arities.
    ///
    /// # RNG supply
    ///
    /// The `rng` is any `rand_core 0.10` [`Rng`] implementor — an engine from
    /// the *caller's* own RNG crate (`StdRng`, `rand_chacha 0.10`'s
    /// `ChaCha20Rng`, …). The engine must sit on the rand_core **0.10** line:
    /// one built against rand_core 0.9 does not satisfy the bound, and the
    /// compiler reports the mismatch as two same-named `Rng` traits. The
    /// bound and its base trait are re-exported at the crate root
    /// ([`crate::Rng`], [`crate::TryRng`]) so callers can *name* them — a
    /// generic wrapper, or a custom engine via `TryRng<Error = Infallible>`
    /// (`Rng` itself is blanket-implemented and cannot be implemented
    /// directly) — without a direct `rand_core` dependency; engines
    /// themselves still come from the caller's chosen crate. The uniform
    /// \[0, 1) sampler is in-tree, so this crate's published edge is
    /// `rand_core` alone — no distributions, no engines, and no OS entropy
    /// anywhere in `src` (#239, #232).
    ///
    /// ```
    /// use catgraph_applied::Rng; // the re-exported supply contract
    /// use catgraph_applied::e1_operad::E1;
    /// use rand::{SeedableRng, rngs::StdRng};
    ///
    /// // The bound is nameable through this crate alone.
    /// fn sample(rng: &mut impl Rng) -> E1 {
    ///     E1::random(3, rng)
    /// }
    /// assert_eq!(sample(&mut StdRng::seed_from_u64(7)).arity(), 3);
    /// ```
    pub fn random(cur_arity: usize, rng: &mut impl Rng) -> Self {
        // Strictly above the `E1::new` width threshold (`F32_EPSILON`), with slack,
        // so every accepted interval has positive width and neighbouring intervals
        // are strictly disjoint. The guarantee comes entirely from this loop:
        // `random` constructs with `overlap_check = false`, so `E1::new` only
        // re-checks widths and bounds, not disjointness.
        const MIN_SEPARATION: f32 = 2.0 * F32_EPSILON;

        let draw_count = cur_arity
            .checked_mul(2)
            .expect("E1::random draw count 2 * cur_arity overflows usize");
        let sub_ints = loop {
            let mut sub_ints: Vec<IntervalCoord> =
                (0..draw_count).map(|_| uniform_unit(rng)).collect();
            sub_ints.sort_unstable_by(|a, b| {
                a.partial_cmp(b)
                    .expect("invariant: uniform_unit samples are finite by construction")
            });
            // An empty sample vec (cur_arity == 0) has no adjacent pairs, so `all`
            // is vacuously true and the loop exits on the first iteration.
            let well_separated = sub_ints
                .iter()
                .tuple_windows()
                .all(|(a, b): (&IntervalCoord, &IntervalCoord)| *b - *a > MIN_SEPARATION);
            if well_separated {
                break sub_ints;
            }
        };
        let sub_intervals: Vec<(IntervalCoord, IntervalCoord)> = sub_ints
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| (chunk[0], chunk[1]))
            .collect();
        Self::new(sub_intervals, false).expect(
            "invariant: resampled coordinates are strictly separated within [0,1], \
             so every interval has positive width",
        )
    }

    fn canonicalize(&mut self) {
        self.sub_intervals
            .sort_by(|i1, i2| i1.0.partial_cmp(&i2.0).unwrap());
    }

    /// Apply a monoid homomorphism: map each interval through `interval_fn` and multiply in order.
    pub fn go_to_monoid<M: One + MulAssign>(
        &mut self,
        interval_fn: impl Fn((IntervalCoord, IntervalCoord)) -> M,
    ) -> M {
        self.canonicalize();
        let mut acc = M::one();
        self.sub_intervals.iter().for_each(|x| {
            acc *= interval_fn(*x);
        });
        acc
    }

    /// Merge all subintervals contained within `all_in_this_interval` into a single interval.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Operadic`] if the interval doesn't contain all sub-intervals.
    pub fn coalesce_boxes(
        &mut self,
        all_in_this_interval: (IntervalCoord, IntervalCoord),
    ) -> Result<(), CatgraphError> {
        self.can_coalesce_boxes(all_in_this_interval)?;
        let (a, b) = all_in_this_interval;
        self.sub_intervals.retain(|(c, d)| *d <= a || *c >= b);
        self.sub_intervals.push((a, b));
        self.arity = self.sub_intervals.len();
        Ok(())
    }

    /// Check whether coalescence is valid: each subinterval must be fully contained or disjoint.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Operadic`] if coalescence is invalid.
    pub fn can_coalesce_boxes(
        &self,
        all_in_this_interval: (IntervalCoord, IntervalCoord),
    ) -> Result<(), CatgraphError> {
        let (a, b) = all_in_this_interval;
        if a >= b - F32_EPSILON || a < -F32_EPSILON || b > 1.0 + F32_EPSILON {
            return Err(CatgraphError::Operadic {
                message: "The coalescing interval must be an interval contained in (0,1)"
                    .to_string(),
            });
        }
        for cur_pair in &self.sub_intervals {
            let (c, d) = cur_pair;
            let contained_within = *c >= a - F32_EPSILON && *d <= b + F32_EPSILON;
            let disjoint_from = *d <= a + F32_EPSILON || *c >= b - F32_EPSILON;
            let bad_config = !(contained_within || disjoint_from);
            if bad_config {
                return Err(CatgraphError::Operadic {
                    message: "All subintervals must be either contained within or disjoint from the coalescing interval"
                        .to_string(),
                });
            }
        }
        Ok(())
    }

    /// Minimum gap between consecutive intervals. Returns `None` for arity < 2.
    ///
    /// # Panics
    ///
    /// Panics if sub-intervals are not in canonical sorted order.
    #[must_use]
    pub fn min_closeness(&self) -> Option<IntervalCoord> {
        if self.arity < 2 {
            return None;
        }
        assert!(
            self.sub_intervals.iter().is_sorted_by(|i1, i2| i1
                .0
                .partial_cmp(&i2.0)
                .expect("No incomparable IntervalCoord issues with NaN etc")
                .is_le()),
            "Should be in canonical form already"
        );
        let mut min_closeness = 1.0;
        for (i1, i2) in self.sub_intervals.iter().tuple_windows() {
            let cur_closeness = i2.0 - i1.1;
            if cur_closeness < min_closeness {
                min_closeness = cur_closeness;
            }
        }
        Some(min_closeness)
    }

    /// Consume self and return the subintervals in canonical (sorted) order.
    #[must_use]
    pub fn extract_sub_intervals(mut self) -> Vec<(IntervalCoord, IntervalCoord)> {
        self.canonicalize();
        self.sub_intervals
    }
}

impl Operadic<usize> for E1 {
    fn operadic_substitution(
        &mut self,
        which_input: usize,
        other_obj: Self,
    ) -> Result<(), CatgraphError> {
        if which_input >= self.arity {
            return Err(CatgraphError::Operadic {
                message: format!(
                    "There aren't enough inputs to graft onto the {}'th one",
                    which_input + 1
                ),
            });
        }
        self.canonicalize();
        let (a, b) = self.sub_intervals[which_input];
        let length_subbed = b - a;
        let mut new_subs = other_obj
            .sub_intervals
            .into_iter()
            .map(|(c, d)| (c * length_subbed + a, d * length_subbed + a));
        let first_new_subs = new_subs.next();
        if let Some(actual_first) = first_new_subs {
            self.sub_intervals[which_input] = actual_first;
            for (offset, image) in new_subs.enumerate() {
                self.sub_intervals.insert(which_input + 1 + offset, image);
            }
            self.arity += other_obj.arity - 1;
        } else {
            _ = self.sub_intervals.swap_remove(which_input);
            self.arity -= 1;
        }
        Ok(())
    }
}

impl HasIdentity<()> for E1 {
    fn identity((): &()) -> Self {
        Self {
            arity: 1,
            sub_intervals: vec![(0.0, 1.0)],
        }
    }
}

#[cfg(test)]
mod test {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Single module-level seed (workspace bench convention — see
    /// `benches/mat_ops_bench.rs`). Tests needing an independent stream
    /// thread a small offset off this constant.
    const SEED: u64 = 1001;

    /// A fixed-word engine over `rand_core::TryRng` — pins the sampler's
    /// endpoints without any rand-family engine.
    struct FixedWord(u32);

    impl rand_core::TryRng for FixedWord {
        type Error = core::convert::Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(self.0)
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok((u64::from(self.0) << 32) | u64::from(self.0))
        }
        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
            for chunk in dst.chunks_mut(4) {
                let word = self.0.to_le_bytes();
                chunk.copy_from_slice(&word[..chunk.len()]);
            }
            Ok(())
        }
    }

    /// A minimal xorshift32 engine over `rand_core::TryRng` — the documented
    /// custom-engine route, exercised end to end with no rand-family crate.
    struct XorShift32(u32);

    impl XorShift32 {
        fn step(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            x
        }
    }

    impl rand_core::TryRng for XorShift32 {
        type Error = core::convert::Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(self.step())
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok((u64::from(self.step()) << 32) | u64::from(self.step()))
        }
        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
            for chunk in dst.chunks_mut(4) {
                let word = self.step().to_le_bytes();
                chunk.copy_from_slice(&word[..chunk.len()]);
            }
            Ok(())
        }
    }

    #[test]
    fn uniform_unit_endpoints_are_exact_and_half_open() {
        use super::uniform_unit;

        // Zero word -> exactly 0.0.
        assert_eq!(uniform_unit(&mut FixedWord(0)), 0.0);
        // Max word -> the top of the ladder, one f32 grid step below 1.
        let max = uniform_unit(&mut FixedWord(u32::MAX));
        assert_eq!(max, 1.0 - f32::EPSILON / 2.0);
        assert!(max < 1.0, "the range must stay strictly half-open");
    }

    #[test]
    fn e1_random_accepts_a_custom_tryrng_engine() {
        use super::E1;

        let mut rng = XorShift32(0x2A2A_2A2A);
        let e1 = E1::random(4, &mut rng);
        assert_eq!(e1.arity(), 4);
        for (a, b) in e1.sub_intervals() {
            assert!(*a >= 0.0, "left endpoint below 0: {a}");
            assert!(*b < 1.0, "right endpoint reaches 1.0: {b}");
            assert!(a < b, "non-positive width: ({a}, {b})");
        }
    }

    #[test]
    fn identity_e1_nullary() {
        use super::E1;
        use catgraph::category::HasIdentity;
        use catgraph::errors::CatgraphError;
        use catgraph::operadic::Operadic;
        use catgraph::{assert_err, assert_ok};

        let mut x = E1::identity(&());
        let zero_ary = E1::new(vec![], true).unwrap();
        let composed = x.operadic_substitution(0, zero_ary);
        assert_ok!(composed);
        assert_eq!(x.arity, 0);
        assert_eq!(x.sub_intervals, vec![]);

        let mut x = E1::identity(&());
        let zero_ary = E1::new(vec![], true).unwrap();
        let composed = x.operadic_substitution(1, zero_ary);
        assert_err!(composed);

        let id = E1::identity(&());
        let mut x = E1::new(vec![], true).unwrap();
        let composed = x.operadic_substitution(0, id);
        assert_eq!(
            composed,
            Err(CatgraphError::Operadic {
                message: "There aren't enough inputs to graft onto the 1'th one".to_string()
            })
        );
        let id = E1::identity(&());
        let composed = x.operadic_substitution(5, id);
        assert_eq!(
            composed,
            Err(CatgraphError::Operadic {
                message: "There aren't enough inputs to graft onto the 6'th one".to_string()
            })
        );
    }

    #[test]
    fn identity_e1_random() {
        use super::E1;
        use catgraph::assert_ok;
        use catgraph::category::HasIdentity;
        use catgraph::operadic::Operadic;
        use rand::RngExt;

        let arity_max: u8 = 20;
        let mut rng = StdRng::seed_from_u64(SEED);
        let trial_num = 10;

        for _ in 0..trial_num {
            let used_arity: u8 = rng.random_range(1..arity_max);
            // Route through `E1::random` (not an inline draw-sort-pair copy of its
            // old body) so the fixture inherits its minimum-separation guarantee.
            let mut as_e1_v1 = E1::random(used_arity as usize, &mut rng);
            let as_e1_v2 = as_e1_v1.clone();
            let sub_intervals = as_e1_v1.sub_intervals().to_vec();
            let which_to_replace = rng.random_range(0..used_arity);
            let id = E1::identity(&());
            let composed = as_e1_v1.operadic_substitution(which_to_replace as usize, id);
            assert_ok!(composed);
            assert_eq!(as_e1_v1.arity, used_arity as usize);
            assert_eq!(as_e1_v1.sub_intervals, sub_intervals);
            let mut id = E1::identity(&());
            let composed = id.operadic_substitution(0, as_e1_v2);
            assert_ok!(composed);
            assert_eq!(id.arity, used_arity as usize);
            assert_eq!(id.sub_intervals, sub_intervals);
        }
    }

    #[test]
    fn two_random_nontrivials() {
        use super::E1;
        use catgraph::assert_ok;
        use catgraph::operadic::Operadic;
        use rand::RngExt;

        let arity_max: u8 = 20;
        let mut rng = StdRng::seed_from_u64(SEED + 1);
        let trial_num = 10;

        for _ in 0..trial_num {
            let used_arity_1: u8 = rng.random_range(1..arity_max);
            // Route through `E1::random` (not an inline draw-sort-pair copy of its
            // old body) so both fixtures inherit its minimum-separation guarantee.
            let as_e1_v1 = E1::random(used_arity_1 as usize, &mut rng);

            let used_arity_2: u8 = rng.random_range(1..arity_max);
            let mut as_e1_v2 = E1::random(used_arity_2 as usize, &mut rng);
            let sub_intervals = as_e1_v2.sub_intervals().to_vec();

            let which_to_replace = rng.random_range(0..used_arity_2);

            let split_box = as_e1_v2.sub_intervals[which_to_replace as usize];

            let composed = as_e1_v2.operadic_substitution(which_to_replace as usize, as_e1_v1);
            assert_ok!(composed);
            assert_eq!(as_e1_v2.arity, (used_arity_1 + used_arity_2 - 1) as usize);
            for (which, interval) in sub_intervals.iter().enumerate() {
                if which == (which_to_replace as usize) {
                    assert!(!as_e1_v2.sub_intervals.contains(interval));
                } else {
                    assert!(as_e1_v2.sub_intervals.contains(interval));
                }
            }
            let res = as_e1_v2.coalesce_boxes(split_box);
            assert_ok!(res);
            assert_eq!(as_e1_v2.arity, used_arity_2 as usize);
            for interval in &sub_intervals {
                assert!(as_e1_v2.sub_intervals.contains(interval));
            }
        }
    }

    /// Numeric oracle for [`E1::operadic_substitution`] at one point of its
    /// input space: outer `[(1/8, 1/4), (1/2, 3/4)]`, slot `0`, inner
    /// `[(1/4, 1/2), (3/4, 7/8)]`.
    ///
    /// Asserts the resulting arity and every output interval as a literal, in
    /// order.
    ///
    /// Every coordinate here, and every product and sum reaching it, is a
    /// dyadic rational with denominator at most 64, hence exact in `f32`: the
    /// assertions are exact equalities.
    ///
    /// One `(outer, slot, inner)` triple is one point of that space. No other
    /// slot, arity, or configuration is covered here.
    #[test]
    fn e1_substitution_numeric_oracle() {
        use super::E1;
        use catgraph::assert_ok;
        use catgraph::operadic::Operadic;

        let mut outer = E1::new(vec![(0.125, 0.25), (0.5, 0.75)], true).unwrap();
        let inner = E1::new(vec![(0.25, 0.5), (0.75, 0.875)], true).unwrap();

        let composed = outer.operadic_substitution(0, inner);
        assert_ok!(composed);

        let expected: Vec<(f32, f32)> = vec![
            (0.15625, 0.1875),   // image of inner (1/4, 1/2), at the substituted slot
            (0.21875, 0.234375), // image of inner (3/4, 7/8), directly after it
            (0.5, 0.75),         // slot 1, geometry unchanged
        ];
        assert_eq!(outer.arity(), 3);
        assert_eq!(outer.sub_intervals(), expected.as_slice());
    }

    /// Outer `[(1/8, 1/4), (1/2, 3/4)]`, inner `[(1/4, 1/2), (3/4, 7/8)]` into
    /// slot 0 (`x ↦ x/8 + 1/8`): result `[(5/32, 3/16), (7/32, 15/64), (1/2, 3/4)]`,
    /// gaps `1/32`, `17/64`, `min_closeness = 1/32`. All dyadic, exact in `f32`.
    #[test]
    fn e1_substitution_into_non_final_slot_keeps_canonical_order() {
        use super::E1;
        use catgraph::assert_ok;
        use catgraph::operadic::Operadic;

        let mut outer = E1::new(vec![(0.125, 0.25), (0.5, 0.75)], true).unwrap();
        let inner = E1::new(vec![(0.25, 0.5), (0.75, 0.875)], true).unwrap();

        let composed = outer.operadic_substitution(0, inner);
        assert_ok!(composed);

        assert_eq!(outer.min_closeness(), Some(0.03125));

        let expected: Vec<(f32, f32)> = vec![
            (0.15625, 0.1875),   // 5/32, 3/16
            (0.21875, 0.234375), // 7/32, 15/64
            (0.5, 0.75),
        ];
        assert_eq!(outer.arity(), 3);
        assert_eq!(
            outer.sub_intervals(),
            expected.as_slice(),
            "images must occupy the substituted slot's position"
        );
    }
}
