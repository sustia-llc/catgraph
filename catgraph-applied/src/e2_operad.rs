//! E2 (little disks) operad: configurations of named disjoint disks inside the unit disk.
//!
//! Supports operadic substitution, coalescence, minimum closeness, embedding from E1,
//! and name management for disk labels.

use itertools::Itertools;
use num::pow;
use std::collections::HashSet;

use crate::{F32_EPSILON, e1_operad::E1};
use catgraph::{category::HasIdentity, errors::CatgraphError, operadic::Operadic};

type PointCenter = (f32, f32);
type Radius = f32;

fn disk_contains(
    c: PointCenter,
    r: Radius,
    query_center: PointCenter,
    query_radius: Option<Radius>,
) -> bool {
    let displace: PointCenter = (c.0 - query_center.0, c.1 - query_center.1);
    let r_eps = r + F32_EPSILON;
    let center_contained = displace.0 * displace.0 + displace.1 * displace.1 <= r_eps * r_eps;
    if center_contained {
        if let Some(real_rad) = query_radius {
            let dist_c_qc_squared = pow(c.0 - query_center.0, 2) + pow(c.1 - query_center.1, 2);
            let dist_c_qc = dist_c_qc_squared.sqrt();
            dist_c_qc + real_rad <= r + F32_EPSILON
        } else {
            // D(c,r) contains the point query_center
            true
        }
    } else {
        false
    }
}

fn disk_closeness(a: PointCenter, b: Radius, c: PointCenter, d: Radius) -> Radius {
    let dist_a_c_squared = pow(c.0 - a.0, 2) + pow(c.1 - a.1, 2);
    let dist_a_c = dist_a_c_squared.sqrt();
    dist_a_c - (b + d)
}

fn disk_overlaps(a: PointCenter, b: Radius, c: PointCenter, d: Radius) -> bool {
    disk_closeness(a, b, c, d) < -F32_EPSILON
}

/// An n-ary operation in the E2 operad: a configuration of `n` named disjoint disks in the unit disk.
#[derive(Clone, Debug)]
pub struct E2<Name> {
    arity: usize,
    sub_circles: Vec<(Name, PointCenter, Radius)>,
}

impl<Name> E2<Name> {
    /// Arity of this configuration (number of sub-disks).
    #[must_use]
    pub const fn arity_of(&self) -> usize {
        self.arity
    }

    /// Immutable view of the sub-disks.
    #[must_use]
    pub fn sub_circles(&self) -> &[(Name, PointCenter, Radius)] {
        &self.sub_circles
    }
}

impl<Name> E2<Name>
where
    Name: Eq + std::hash::Hash + Clone + std::fmt::Debug,
{
    /// Create an n-ary E2 configuration from named disks inside the unit disk.
    ///
    /// When `overlap_check` is true, validates pairwise disjointness. Names must be unique.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Operadic`] if any sub-circle extends outside the unit disk,
    /// names are not unique, or circles overlap when `overlap_check` is true.
    pub fn new(
        sub_circles: Vec<(Name, PointCenter, Radius)>,
        overlap_check: bool,
    ) -> Result<Self, CatgraphError> {
        for (_a, b, c) in &sub_circles {
            if !disk_contains((0.0, 0.0), 1.0, *b, Some(*c)) {
                return Err(CatgraphError::Operadic {
                    message: format!(
                        "Subcircle at ({}, {}), r={} is not contained in the unit disk",
                        b.0, b.1, c
                    ),
                });
            }
        }
        if !sub_circles.iter().map(|(a, _, _)| a).all_unique() {
            return Err(CatgraphError::Operadic {
                message: "each subcircle must have a unique name".to_string(),
            });
        }
        if overlap_check {
            for d_pair in sub_circles.iter().combinations(2) {
                let d1 = d_pair[0];
                let d2 = d_pair[1];
                if disk_overlaps(d1.1, d1.2, d2.1, d2.2) {
                    return Err(CatgraphError::Operadic {
                        message: "The input circles cannot overlap".to_string(),
                    });
                }
            }
        }
        Ok(Self {
            arity: sub_circles.len(),
            sub_circles,
        })
    }

    /// Merge all subdisks contained within the given disk into a single named disk.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Operadic`] if the circle doesn't contain all sub-circles.
    pub fn coalesce_boxes(
        &mut self,
        all_in_this_circle: (Name, PointCenter, Radius),
    ) -> Result<(), CatgraphError> {
        self.can_coalesce_boxes((all_in_this_circle.1, all_in_this_circle.2))?;
        let (a, b, c) = all_in_this_circle;
        self.sub_circles
            .retain(|(_, d, _)| !disk_contains(b, c, *d, None));
        self.sub_circles.push((a, b, c));
        self.arity = self.sub_circles.len();
        Ok(())
    }

    /// Check whether coalescence is valid: each subdisk must be fully contained or disjoint.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Operadic`] if coalescence is invalid.
    pub fn can_coalesce_boxes(
        &self,
        all_in_this_disk: (PointCenter, Radius),
    ) -> Result<(), CatgraphError> {
        let (a, b) = all_in_this_disk;
        if !disk_contains((0.0, 0.0), 1.0, a, Some(b)) {
            return Err(CatgraphError::Operadic {
                message: "The coalescing disk must be contained in the unit disk".to_string(),
            });
        }
        for cur_pair in &self.sub_circles {
            let (_, c, d) = cur_pair;
            let contained_within = disk_contains(a, b, *c, Some(*d));
            let disjoint_from = !disk_overlaps(a, b, *c, *d);
            let bad_config = !(contained_within || disjoint_from);
            if bad_config {
                return Err(CatgraphError::Operadic {
                    message: "All subcircles must be either contained within or disjoint from the coalescing disk"
                        .to_string(),
                });
            }
        }
        Ok(())
    }

    /// Minimum gap between any pair of subdisks. Returns `None` for arity < 2.
    #[must_use]
    pub fn min_closeness(&self) -> Option<Radius> {
        if self.arity < 2 {
            return None;
        }
        let mut min_seen = 2.0;
        for circle_pair in self.sub_circles.iter().combinations(2) {
            let circ_0 = circle_pair[0];
            let circ_1 = circle_pair[1];
            let cur_dist = disk_closeness(circ_0.1, circ_0.2, circ_1.1, circ_1.2);
            if cur_dist < min_seen {
                min_seen = cur_dist;
            }
        }
        Some(min_seen)
    }

    /// Embed an E1 configuration into E2 by mapping \[0, 1\] intervals to disks along the x-axis.
    pub fn from_e1_config(e1_config: E1, disk_namer: impl Fn(usize) -> Name) -> Self {
        let sub_intervals = e1_config.extract_sub_intervals();
        // Map E1 interval [a,b] ⊂ [0,1] to E2 disk in unit disk:
        // Center: midpoint (a+b)/2 mapped from [0,1] to [-1,1] → ((a+b)-1, 0)
        // Radius: full interval width (not half) because center coord is doubled
        let sub_circles = sub_intervals.iter().enumerate().map(|(idx, interval)| {
            let new_center = ((interval.1 + interval.0) - 1.0, 0.0);
            let new_radius = interval.1 - interval.0;
            (disk_namer(idx), new_center, new_radius)
        });
        Self {
            arity: sub_circles.len(),
            sub_circles: sub_circles.collect_vec(),
        }
    }

    /// Transform all disk names via `name_changer`, producing an `E2<Name2>`.
    pub fn change_names<Name2: Eq + std::hash::Hash + Clone + std::fmt::Debug>(
        self,
        name_changer: impl Fn(Name) -> Name2,
    ) -> E2<Name2> {
        let new_sub_circles = self
            .sub_circles
            .into_iter()
            .map(|old_sub| (name_changer(old_sub.0), old_sub.1, old_sub.2))
            .collect_vec();
        E2 {
            arity: new_sub_circles.len(),
            sub_circles: new_sub_circles,
        }
    }

    /// Consume self and return the named subdisks.
    #[must_use]
    pub fn extract_sub_circles(self) -> Vec<(Name, PointCenter, Radius)> {
        self.sub_circles
    }

    /// Rename a single disk. No-op if the old name is not found. Panics if the new name collides.
    ///
    /// # Panics
    ///
    /// Panics if the new name already exists among sub-circles.
    pub fn change_name(&mut self, name_change: (Name, Name)) {
        let idx_change = self.sub_circles.iter().position(|p| p.0 == name_change.0);
        if let Some(real_idx_change) = idx_change {
            assert!(
                self.sub_circles
                    .iter()
                    .all(|(a, _, _)| *a == name_change.0 || *a != name_change.1),
                "each subcircle must have a unique name"
            );
            self.sub_circles[real_idx_change].0 = name_change.1;
        }
    }
}

impl<Name> HasIdentity<Name> for E2<Name>
where
    Name: Eq + std::hash::Hash + Clone + std::fmt::Debug,
{
    fn identity(to_name: &Name) -> Self {
        Self {
            arity: 1,
            sub_circles: vec![(to_name.clone(), (0.0, 0.0), 1.0)],
        }
    }
}

impl<Name> Operadic<Name> for E2<Name>
where
    Name: Eq + std::hash::Hash + Clone + std::fmt::Debug,
{
    fn operadic_substitution(
        &mut self,
        which_input: Name,
        other_obj: Self,
    ) -> Result<(), CatgraphError> {
        let idx_of_input = self
            .sub_circles
            .iter()
            .position(|item| item.0 == which_input);
        if let Some(real_idx) = idx_of_input {
            let (_, inserted_center, inserted_radius) = self.sub_circles.swap_remove(real_idx);
            let selfnames: HashSet<Name> = self
                .sub_circles
                .iter()
                .map(|(selfname, _, _)| selfname.clone())
                .collect();
            let not_still_unique = other_obj
                .sub_circles
                .iter()
                .any(|cur| selfnames.contains(&cur.0));
            if not_still_unique {
                return Err(CatgraphError::Operadic {
                    message: "each subcircle must have a unique name".to_string(),
                });
            }
            let new_circles = other_obj.sub_circles.into_iter().map(|cur| {
                let new_center = (
                    cur.1.0 * inserted_radius + inserted_center.0,
                    cur.1.1 * inserted_radius + inserted_center.1,
                );
                (cur.0, new_center, cur.2 * inserted_radius)
            });
            self.sub_circles.extend(new_circles);
            self.arity = self.sub_circles.len();
            Ok(())
        } else {
            Err(CatgraphError::Operadic {
                message: format!("No such input {which_input:?} found"),
            })
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // identity constructor produces exact 0.0/1.0 by construction
mod test {
    use super::*;
    use catgraph::category::HasIdentity;
    use catgraph::operadic::Operadic;
    use catgraph::{assert_err, assert_ok};

    #[test]
    fn identity_e2_nullary() {
        let mut x = E2::identity(&0);
        let zero_ary = E2::new(vec![], true).unwrap();
        let composed = x.operadic_substitution(0, zero_ary);
        assert_ok!(composed);
        assert_eq!(x.arity, 0);
        assert_eq!(x.sub_circles, vec![]);

        let mut x = E2::identity(&0);
        let zero_ary = E2::new(vec![], true).unwrap();
        let composed = x.operadic_substitution(1, zero_ary);
        assert_err!(composed);

        let id = E2::identity(&0);
        let mut x = E2::new(vec![], true).unwrap();
        let composed = x.operadic_substitution(0, id);
        assert_eq!(
            composed,
            Err(CatgraphError::Operadic {
                message: "No such input 0 found".to_string()
            })
        );
        let id = E2::identity(&0);
        let composed = x.operadic_substitution(5, id);
        assert_eq!(
            composed,
            Err(CatgraphError::Operadic {
                message: "No such input 5 found".to_string()
            })
        );
    }

    #[test]
    fn disk_contains_point() {
        // Point at origin in unit disk
        assert!(disk_contains((0.0, 0.0), 1.0, (0.0, 0.0), None));
        // Point at edge
        assert!(disk_contains((0.0, 0.0), 1.0, (1.0, 0.0), None));
        // Point outside
        assert!(!disk_contains((0.0, 0.0), 1.0, (2.0, 0.0), None));
    }

    #[test]
    fn disk_contains_disk() {
        // Small disk at center inside unit disk
        assert!(disk_contains((0.0, 0.0), 1.0, (0.0, 0.0), Some(0.5)));
        // Disk that exceeds boundary
        assert!(!disk_contains((0.0, 0.0), 1.0, (0.5, 0.0), Some(0.6)));
    }

    #[test]
    fn disk_overlaps_test() {
        // Overlapping disks
        assert!(disk_overlaps((0.0, 0.0), 0.5, (0.5, 0.0), 0.5));
        // Non-overlapping disks
        assert!(!disk_overlaps((0.0, 0.0), 0.2, (1.0, 0.0), 0.2));
    }

    #[test]
    fn disk_closeness_test() {
        // Adjacent disks (touching)
        let close = disk_closeness((0.0, 0.0), 0.5, (1.0, 0.0), 0.5);
        assert!(close.abs() < 0.001);

        // Separated disks
        let far = disk_closeness((0.0, 0.0), 0.2, (2.0, 0.0), 0.2);
        assert!(far > 0.0);
    }

    #[test]
    fn e2_new_basic() {
        let circles = vec![(0, (0.0, 0.0), 0.3), (1, (0.5, 0.0), 0.2)];
        let e2 = E2::new(circles, true).unwrap();
        assert_eq!(e2.arity, 2);
    }

    #[test]
    fn e2_new_empty() {
        let e2: E2<i32> = E2::new(vec![], true).unwrap();
        assert_eq!(e2.arity, 0);
    }

    #[test]
    fn e2_identity() {
        let id = E2::identity(&"center");
        assert_eq!(id.arity, 1);
        assert_eq!(id.sub_circles.len(), 1);
        assert_eq!(id.sub_circles[0].0, "center");
        assert_eq!(id.sub_circles[0].1, (0.0, 0.0));
        assert_eq!(id.sub_circles[0].2, 1.0);
    }

    #[test]
    fn e2_min_closeness() {
        let circles = vec![(0, (0.0, 0.0), 0.2), (1, (0.6, 0.0), 0.2)];
        let e2 = E2::new(circles, true).unwrap();
        let close = e2.min_closeness();
        assert!(close.is_some());
        // Distance between centers is 0.6, radii sum is 0.4, so closeness is 0.2
        assert!((close.unwrap() - 0.2).abs() < 0.001);
    }

    #[test]
    fn e2_min_closeness_single() {
        let id = E2::identity(&0);
        assert!(id.min_closeness().is_none());
    }

    #[test]
    fn e2_from_e1_config() {
        use crate::e1_operad::E1;
        let e1 = E1::identity(&());
        let e2 = E2::from_e1_config(e1, |idx| idx);
        assert_eq!(e2.arity, 1);
    }

    #[test]
    fn e2_change_names() {
        let circles = vec![(0, (0.0, 0.0), 0.5)];
        let e2 = E2::new(circles, false).unwrap();
        let renamed = e2.change_names(|n| n + 10);
        assert_eq!(renamed.sub_circles[0].0, 10);
    }

    #[test]
    fn e2_change_name() {
        let circles = vec![(0, (0.0, 0.0), 0.5), (1, (0.5, 0.0), 0.2)];
        let mut e2 = E2::new(circles, false).unwrap();
        e2.change_name((0, 10));
        assert!(e2.sub_circles.iter().find(|c| c.0 == 10).is_some());
    }

    #[test]
    fn e2_operadic_substitution() {
        let mut outer = E2::identity(&0);
        let inner = E2::new(vec![(1, (0.0, 0.0), 0.5), (2, (0.0, 0.5), 0.3)], false).unwrap();

        let result = outer.operadic_substitution(0, inner);
        assert!(result.is_ok());
        assert_eq!(outer.arity, 2);
    }

    #[test]
    fn e2_operadic_substitution_nested() {
        // Create outer with two circles
        let mut outer = E2::new(vec![(0, (-0.5, 0.0), 0.3), (1, (0.5, 0.0), 0.3)], true).unwrap();
        // Substitute into first circle
        let inner = E2::new(vec![(2, (0.0, 0.0), 0.5)], false).unwrap();
        let result = outer.operadic_substitution(0, inner);
        assert!(result.is_ok());
        // Now we have circles 1 and 2
        assert_eq!(outer.arity, 2);
    }

    #[test]
    fn e2_can_coalesce_boxes() {
        let circles = vec![(0, (0.0, 0.0), 0.2), (1, (0.0, 0.5), 0.2)];
        let e2 = E2::new(circles, true).unwrap();

        // A disk at (0.0, 0.25) r=0.5 is inside unit disk and contains both subcircles
        let result = e2.can_coalesce_boxes(((0.0, 0.25), 0.5));
        assert!(result.is_ok());
    }

    #[test]
    fn e2_can_coalesce_small_disk_inside_unit() {
        // Small disk well inside the unit disk with no subcircles to conflict
        let e2: E2<i32> = E2::new(vec![], true).unwrap();
        let result = e2.can_coalesce_boxes(((0.2, 0.3), 0.1));
        assert!(result.is_ok());
    }

    #[test]
    fn e2_cannot_coalesce_disk_outside_unit() {
        // Disk centered at (2.0, 0.0) with radius 0.5 is outside the unit disk
        let e2: E2<i32> = E2::new(vec![], true).unwrap();
        let result = e2.can_coalesce_boxes(((2.0, 0.0), 0.5));
        let err = result.unwrap_err();
        assert!(
            matches!(err, CatgraphError::Operadic { ref message } if message.contains("unit disk")),
            "expected Operadic error about unit disk, got: {err:?}"
        );
    }
}
