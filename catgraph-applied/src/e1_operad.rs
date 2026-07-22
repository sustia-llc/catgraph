//! E1 (little intervals) operad: configurations of disjoint subintervals of \[0, 1\].
//!
//! Supports operadic substitution, coalescence, monoid homomorphism, and
//! minimum closeness between adjacent intervals.

use std::ops::MulAssign;

use deep_causality_num::One;
use itertools::Itertools;
use rand::RngExt;

use crate::F32_EPSILON;
use catgraph::{category::HasIdentity, errors::CatgraphError, operadic::Operadic};

type IntervalCoord = f32;

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
    /// The sort's `partial_cmp` expect panics if any sample is not finite, which
    /// cannot occur for `random_range(0.0..1.0)`. The terminal `expect` documents
    /// the resampling invariant and likewise cannot fire.
    pub fn random(cur_arity: usize, rng: &mut impl RngExt) -> Self {
        // Strictly above the `E1::new` width threshold (`F32_EPSILON`), with slack,
        // so every accepted interval has positive width and neighbouring intervals
        // are strictly disjoint. The guarantee comes entirely from this loop:
        // `random` constructs with `overlap_check = false`, so `E1::new` only
        // re-checks widths and bounds, not disjointness.
        const MIN_SEPARATION: f32 = 2.0 * F32_EPSILON;

        let sub_ints = loop {
            let mut sub_ints: Vec<IntervalCoord> = (0..2 * cur_arity)
                .map(|_| rng.random_range(0.0..1.0))
                .collect();
            sub_ints.sort_unstable_by(|a, b| {
                a.partial_cmp(b)
                    .expect("invariant: samples from random_range(0.0..1.0) are finite")
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
            .chunks_exact(2)
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
            self.sub_intervals.extend(new_subs);
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
}
