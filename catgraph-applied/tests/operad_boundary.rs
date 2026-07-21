use catgraph::errors::CatgraphError;
use catgraph::operadic::Operadic;
use catgraph_applied::e1_operad::E1;
use catgraph_applied::e2_operad::E2;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn e1_exact_unit_interval() {
    let e1 = E1::new(vec![(0.0, 1.0)], true);
    assert!(e1.is_ok());
    assert_eq!(e1.unwrap().extract_sub_intervals(), vec![(0.0, 1.0)]);
}

#[test]
fn e1_barely_invalid() {
    // -0.01 is well beyond epsilon tolerance
    let result = E1::new(vec![(-0.01, 0.5)], true);
    assert!(result.is_err());
    match result.unwrap_err() {
        CatgraphError::Operadic { message: msg } => assert!(msg.contains("below 0"), "got: {msg}"),
        other => panic!("Expected Operadic error, got: {other:?}"),
    }
}

#[test]
fn e1_touching_intervals() {
    // Two intervals that touch at 0.5 — within epsilon tolerance
    let result = E1::new(vec![(0.0, 0.5), (0.5, 1.0)], true);
    assert!(
        result.is_ok(),
        "Touching intervals should be accepted: {result:?}"
    );
}

#[test]
fn e1_overlapping_intervals() {
    // Intervals [0.0, 0.6) and [0.5, 1.0) genuinely overlap by 0.1
    let result = E1::new(vec![(0.0, 0.6), (0.5, 1.0)], true);
    assert!(result.is_err());
    match result.unwrap_err() {
        CatgraphError::Operadic { message: msg } => assert!(msg.contains("overlap"), "got: {msg}"),
        other => panic!("Expected Operadic error, got: {other:?}"),
    }
}

#[test]
fn e2_disk_at_boundary() {
    // Disk at (0.5, 0.0) with radius 0.5 — edge touches unit disk boundary
    // dist(origin, center) + radius = 0.5 + 0.5 = 1.0
    let result = E2::new(vec![(0, (0.5, 0.0), 0.5)], true);
    assert!(
        result.is_ok(),
        "Disk touching boundary should be accepted: {result:?}"
    );
}

#[test]
fn e2_disk_outside() {
    // Disk at (0.8, 0.0) with radius 0.5 — extends past unit disk
    // dist(origin, center) + radius = 0.8 + 0.5 = 1.3 > 1.0
    let result = E2::new(vec![(0, (0.8, 0.0), 0.5)], true);
    assert!(result.is_err());
    match result.unwrap_err() {
        CatgraphError::Operadic { message: msg } => {
            assert!(msg.contains("not contained"), "got: {msg}");
        }
        other => panic!("Expected Operadic error, got: {other:?}"),
    }
}

#[test]
fn e2_touching_disks() {
    // Two disks whose edges touch (closeness ~0) — should NOT be detected as overlap
    // Disk 1: center (-0.4, 0.0), radius 0.2
    // Disk 2: center (0.4, 0.0), radius 0.2
    // Distance between centers = 0.8, sum of radii = 0.4, closeness = 0.4
    // Actually let's use disks that truly touch:
    // Disk 1: center (-0.25, 0.0), radius 0.25
    // Disk 2: center (0.25, 0.0), radius 0.25
    // Distance between centers = 0.5, sum of radii = 0.5, closeness = 0.0
    let result = E2::new(vec![(0, (-0.25, 0.0), 0.25), (1, (0.25, 0.0), 0.25)], true);
    assert!(
        result.is_ok(),
        "Touching disks (closeness ~0) should not be overlap: {result:?}"
    );
}

#[test]
fn e1_to_e2_embedding() {
    // Create E1 with 3 non-overlapping intervals
    let e1 = E1::new(vec![(0.0, 0.2), (0.3, 0.5), (0.7, 0.9)], true).unwrap();
    let e2: E2<usize> = E2::from_e1_config(e1, |idx| idx);

    // Verify all disks fit in the unit disk by re-constructing with overlap check
    let circles = e2.extract_sub_circles();
    assert_eq!(circles.len(), 3);
    let result = E2::new(circles, true);
    assert!(
        result.is_ok(),
        "E1-to-E2 embedding should produce valid E2: {result:?}"
    );
}

#[test]
fn e1_substitution_validity() {
    // Compose two valid E1s via operadic_substitution, verify result is valid
    let mut outer = E1::new(vec![(0.1, 0.4), (0.6, 0.9)], true).unwrap();
    let inner = E1::new(vec![(0.1, 0.4), (0.6, 0.9)], true).unwrap();

    let result = outer.operadic_substitution(0, inner);
    assert!(result.is_ok(), "Substitution should succeed: {result:?}");

    // The resulting sub-intervals should still be valid within [0,1]
    let intervals = outer.extract_sub_intervals();
    for (a, b) in &intervals {
        assert!(*a >= 0.0, "interval start {a} should be >= 0");
        assert!(*b <= 1.0, "interval end {b} should be <= 1");
        assert!(*a < *b, "interval ({a}, {b}) should have positive width");
    }

    // Verify we can construct a new E1 from these intervals
    let reconstructed = E1::new(intervals, true);
    assert!(
        reconstructed.is_ok(),
        "Substitution result should be reconstructible: {reconstructed:?}"
    );
}

#[test]
fn e2_substitution_validity() {
    // Compose two valid E2s via operadic_substitution, verify result is valid
    let mut outer: E2<i32> =
        E2::new(vec![(0, (-0.5, 0.0), 0.3), (1, (0.5, 0.0), 0.3)], true).unwrap();

    let inner: E2<i32> = E2::new(vec![(2, (0.0, 0.0), 0.5)], false).unwrap();

    let result = outer.operadic_substitution(0, inner);
    assert!(result.is_ok(), "E2 substitution should succeed: {result:?}");

    // Verify the result has 2 circles (removed 0, added 2, kept 1)
    let circles = outer.extract_sub_circles();
    assert_eq!(circles.len(), 2);

    // Verify we can construct a new E2 from these circles (geometry valid)
    let reconstructed = E2::new(circles, false);
    assert!(
        reconstructed.is_ok(),
        "E2 substitution result should be reconstructible: {reconstructed:?}"
    );
}

// --- E2 advanced method integration tests ---

#[test]
fn e2_coalesce_absorbs_subcircles() {
    // Three small disks near the origin; coalesce two of them into a larger disk.
    let mut e2: E2<&str> = E2::new(
        vec![
            ("a", (0.0, 0.0), 0.1),
            ("b", (0.2, 0.0), 0.1),
            ("c", (-0.5, 0.0), 0.1),
        ],
        true,
    )
    .unwrap();

    // Coalescing disk centered at (0.1, 0.0) r=0.25 contains "a" and "b" centers
    // but is disjoint from "c" (center -0.5 is far away).
    let result = e2.coalesce_boxes(("ab", (0.1, 0.0), 0.25));
    assert!(result.is_ok(), "Coalesce should succeed: {result:?}");

    let circles = e2.extract_sub_circles();
    // "a" and "b" absorbed, replaced by "ab"; "c" remains
    assert_eq!(circles.len(), 2);
    let names: Vec<&str> = circles.iter().map(|(n, _, _)| *n).collect();
    assert!(names.contains(&"ab"), "coalesced disk should be present");
    assert!(names.contains(&"c"), "disjoint disk should remain");
    assert!(!names.contains(&"a"), "'a' should have been absorbed");
    assert!(!names.contains(&"b"), "'b' should have been absorbed");
}

#[test]
fn e2_coalesce_then_substitute() {
    // Create E2 with 3 disks, coalesce 2, then substitute into the coalesced circle.
    // Disks 1 and 2 are non-overlapping: centers 0.3 apart, radii 0.1 each, closeness = 0.1.
    let mut e2: E2<i32> = E2::new(
        vec![
            (1, (0.0, 0.0), 0.1),
            (2, (0.3, 0.0), 0.1),
            (3, (-0.5, 0.0), 0.1),
        ],
        true,
    )
    .unwrap();

    // Coalesce disks 1 and 2 into disk 10: centered at (0.15, 0.0), r=0.5
    // contains disk 1: dist(0.15, (0,0)) + 0.1 = 0.15 + 0.1 = 0.25 <= 0.5 ✓
    // contains disk 2: dist(0.15, (0.3,0)) + 0.1 = 0.15 + 0.1 = 0.25 <= 0.5 ✓
    // disjoint from disk 3: closeness = dist(0.15, (-0.5,0)) - (0.5 + 0.1) = 0.65 - 0.6 = 0.05 > 0 ✓
    // fits in unit disk: dist(origin, (0.15,0)) + 0.5 = 0.15 + 0.5 = 0.65 <= 1.0 ✓
    let result = e2.coalesce_boxes((10, (0.15, 0.0), 0.5));
    assert!(result.is_ok(), "Coalesce should succeed: {result:?}");

    // Now substitute an inner E2 into the coalesced circle (name 10)
    let inner: E2<i32> =
        E2::new(vec![(20, (0.0, 0.0), 0.4), (21, (0.0, 0.5), 0.3)], false).unwrap();

    let sub_result = e2.operadic_substitution(10, inner);
    assert!(
        sub_result.is_ok(),
        "Substitution into coalesced circle should succeed: {sub_result:?}"
    );

    let circles = e2.extract_sub_circles();
    let names: Vec<i32> = circles.iter().map(|(n, _, _)| *n).collect();
    // Circle 10 replaced by 20 and 21; circle 3 remains
    assert!(names.contains(&20));
    assert!(names.contains(&21));
    assert!(names.contains(&3));
    assert!(!names.contains(&10));
    assert_eq!(circles.len(), 3);
}

#[test]
fn e2_cannot_coalesce_partial_overlap() {
    // A subcircle that partially overlaps the coalescing disk (neither fully
    // contained nor fully disjoint) should produce an error.
    let e2: E2<i32> = E2::new(vec![(1, (0.0, 0.0), 0.3)], true).unwrap();

    // Coalescing disk at (0.25, 0.0) r=0.15:
    //   disk_contains check: dist(0.25, (0,0)) = 0.25, 0.25 + 0.3 = 0.55 > 0.15 → NOT contained
    //   disk_overlaps check: closeness = 0.25 - (0.15 + 0.3) = -0.2 < -eps → overlaps → NOT disjoint
    // So it's a bad config (partial overlap).
    let result = e2.can_coalesce_boxes(((0.25, 0.0), 0.15));
    assert!(result.is_err(), "Partial overlap should fail");
    match result.unwrap_err() {
        CatgraphError::Operadic { message: msg } => {
            assert!(
                msg.contains("contained within or disjoint"),
                "Error should mention containment/disjoint requirement, got: {msg}"
            );
        }
        e => panic!("expected Operadic error, got: {e:?}"),
    }
}

#[test]
fn e2_min_closeness_basic() {
    // Three disks with known pairwise distances.
    // Disk A at (-0.4, 0.0) r=0.1, Disk B at (0.0, 0.0) r=0.1, Disk C at (0.4, 0.0) r=0.1
    // Closeness A-B: dist=0.4, radii_sum=0.2, closeness=0.2
    // Closeness B-C: dist=0.4, radii_sum=0.2, closeness=0.2
    // Closeness A-C: dist=0.8, radii_sum=0.2, closeness=0.6
    // min_closeness = 0.2
    let e2: E2<i32> = E2::new(
        vec![
            (0, (-0.4, 0.0), 0.1),
            (1, (0.0, 0.0), 0.1),
            (2, (0.4, 0.0), 0.1),
        ],
        true,
    )
    .unwrap();

    let mc = e2.min_closeness();
    assert!(mc.is_some());
    assert!(
        (mc.unwrap() - 0.2).abs() < 0.001,
        "Expected min_closeness ~0.2, got {}",
        mc.unwrap()
    );
}

#[test]
fn e2_min_closeness_single_disk() {
    // Arity 1 → min_closeness returns None (no pairs to compare)
    let e2: E2<i32> = E2::new(vec![(0, (0.0, 0.0), 0.5)], true).unwrap();
    assert_eq!(e2.min_closeness(), None);

    // Arity 0 → also None
    let e2_empty: E2<i32> = E2::new(vec![], true).unwrap();
    assert_eq!(e2_empty.min_closeness(), None);
}

#[test]
fn e2_change_names_preserves_geometry() {
    let circles = vec![
        (1, (-0.3, 0.2), 0.15),
        (2, (0.4, -0.1), 0.2),
        (3, (0.0, 0.0), 0.1),
    ];
    let e2: E2<i32> = E2::new(circles.clone(), true).unwrap();

    // Rename: n -> n * 100
    let renamed = e2.change_names(|n| n * 100);
    let new_circles = renamed.extract_sub_circles();

    assert_eq!(new_circles.len(), circles.len());
    for (orig, renamed) in circles.iter().zip(new_circles.iter()) {
        // Name changed
        assert_eq!(renamed.0, orig.0 * 100);
        // Center preserved
        assert!((renamed.1.0 - orig.1.0).abs() < f32::EPSILON);
        assert!((renamed.1.1 - orig.1.1).abs() < f32::EPSILON);
        // Radius preserved
        assert!((renamed.2 - orig.2).abs() < f32::EPSILON);
    }
}

#[test]
fn e2_change_names_then_substitute() {
    // Build an E2 with integer names, rename to strings, then substitute using new names.
    let e2: E2<i32> = E2::new(vec![(1, (-0.4, 0.0), 0.3), (2, (0.4, 0.0), 0.3)], true).unwrap();

    let renamed: E2<String> = e2.change_names(|n| format!("disk_{n}"));
    let circles = renamed.extract_sub_circles();
    // Verify names are "disk_1" and "disk_2"
    let names: Vec<&str> = circles.iter().map(|(n, _, _)| n.as_str()).collect();
    assert!(names.contains(&"disk_1"));
    assert!(names.contains(&"disk_2"));

    // Reconstruct (extract_sub_circles consumes), so rebuild
    let mut renamed: E2<String> = E2::new(
        vec![
            ("disk_1".to_string(), (-0.4, 0.0), 0.3),
            ("disk_2".to_string(), (0.4, 0.0), 0.3),
        ],
        true,
    )
    .unwrap();

    // Substitute into "disk_1" with an inner E2
    let inner: E2<String> = E2::new(vec![("inner_a".to_string(), (0.0, 0.0), 0.5)], false).unwrap();

    let result = renamed.operadic_substitution("disk_1".to_string(), inner);
    assert!(
        result.is_ok(),
        "Substitution with renamed labels should succeed: {result:?}"
    );

    let final_circles = renamed.extract_sub_circles();
    let final_names: Vec<&str> = final_circles.iter().map(|(n, _, _)| n.as_str()).collect();
    assert!(
        final_names.contains(&"inner_a"),
        "inner disk should be present"
    );
    assert!(
        final_names.contains(&"disk_2"),
        "untouched disk should remain"
    );
    assert!(
        !final_names.contains(&"disk_1"),
        "substituted disk should be gone"
    );
    assert_eq!(final_circles.len(), 2);
}

// --- E1 secondary method integration tests ---

#[test]
fn e1_random_produces_valid_config() {
    let mut rng = StdRng::seed_from_u64(141);

    for arity in 1..=10 {
        // Many trials per arity so the seeded run exercises many draws, including
        // batches that must be resampled to clear the minimum-separation guarantee.
        for _ in 0..20 {
            let e1 = E1::random(arity, &mut rng);
            let intervals = e1.extract_sub_intervals();

            assert_eq!(
                intervals.len(),
                arity,
                "arity {arity}: interval count should match"
            );

            for (i, &(a, b)) in intervals.iter().enumerate() {
                assert!(
                    a >= 0.0,
                    "arity {arity}, interval {i}: start {a} should be >= 0"
                );
                assert!(
                    b <= 1.0,
                    "arity {arity}, interval {i}: end {b} should be <= 1"
                );
                assert!(
                    a < b,
                    "arity {arity}, interval {i}: ({a}, {b}) should have positive width"
                );
            }

            // Intervals should be sorted (canonicalized by extract_sub_intervals)
            for i in 1..intervals.len() {
                assert!(
                    intervals[i].0 >= intervals[i - 1].1,
                    "arity {arity}: intervals should be non-overlapping and sorted, \
                     but interval {} ends at {} while interval {} starts at {}",
                    i - 1,
                    intervals[i - 1].1,
                    i,
                    intervals[i].0,
                );
            }

            // The result should be reconstructible with overlap checking enabled
            let reconstructed = E1::new(intervals, true);
            assert!(
                reconstructed.is_ok(),
                "arity {arity}: random config should pass overlap check: {reconstructed:?}"
            );
        }
    }
}

#[test]
fn e1_random_zero_arity() {
    let mut rng = StdRng::seed_from_u64(1410);
    let e1 = E1::random(0, &mut rng);
    let intervals = e1.extract_sub_intervals();
    assert!(intervals.is_empty(), "arity 0 should produce no intervals");
}

#[test]
fn e1_go_to_monoid_reduces_to_single() {
    // A 3-interval config mapped through a monoid (f64 multiplication).
    // Each interval (a, b) maps to its width (b - a), then we multiply them all.
    let mut e1 = E1::new(vec![(0.0, 0.2), (0.3, 0.6), (0.8, 1.0)], true).unwrap();

    let product: f64 = e1.go_to_monoid(|(a, b)| f64::from(b - a));

    let expected = 0.2 * 0.3 * 0.2;
    assert!(
        (product - expected).abs() < 1e-6,
        "Expected product ~{expected}, got {product}"
    );
}

#[test]
fn e1_go_to_monoid_single_interval() {
    // Single interval: monoid result should equal the single mapped value.
    let mut e1 = E1::new(vec![(0.1, 0.9)], true).unwrap();
    let result: f64 = e1.go_to_monoid(|(a, b)| f64::from(b - a));

    let expected = 0.8;
    assert!(
        (result - expected).abs() < 1e-6,
        "Single interval monoid should be ~{expected}, got {result}"
    );
}

#[test]
fn e1_go_to_monoid_identity_element() {
    // Empty config (arity 0): go_to_monoid should return M::one().
    let mut e1 = E1::new(vec![], true).unwrap();
    let result: f64 = e1.go_to_monoid(|(_a, _b)| 42.0);
    assert!(
        (result - 1.0).abs() < 1e-12,
        "Empty config should return monoid identity (1.0), got {result}"
    );
}

#[test]
fn e1_coalesce_boxes_merges_adjacent() {
    // Three intervals; coalesce the first two into one covering interval.
    let mut e1 = E1::new(vec![(0.0, 0.2), (0.3, 0.5), (0.7, 0.9)], true).unwrap();

    // Coalescing interval (0.0, 0.55) contains the first two intervals
    // and is disjoint from (0.7, 0.9).
    let result = e1.coalesce_boxes((0.0, 0.55));
    assert!(result.is_ok(), "Coalesce should succeed: {result:?}");

    let intervals = e1.extract_sub_intervals();
    assert_eq!(intervals.len(), 2, "Should have 2 intervals after coalesce");
    assert!(
        intervals.contains(&(0.0, 0.55)),
        "Coalesced interval should be present"
    );
    assert!(
        intervals.contains(&(0.7, 0.9)),
        "Disjoint interval should remain"
    );
}

#[test]
fn e1_coalesce_boxes_error_partial_overlap() {
    // Coalescing interval partially overlaps an existing sub-interval.
    let e1 = E1::new(vec![(0.0, 0.5), (0.6, 0.9)], true).unwrap();

    // (0.3, 0.7) partially overlaps both intervals — neither fully contains
    // nor is disjoint from either one.
    let result = e1.can_coalesce_boxes((0.3, 0.7));
    assert!(result.is_err(), "Partial overlap should fail");
    match result.unwrap_err() {
        CatgraphError::Operadic { message: msg } => {
            assert!(
                msg.contains("contained within or disjoint"),
                "Error should mention containment/disjoint requirement, got: {msg}"
            );
        }
        e => panic!("expected Operadic error, got: {e:?}"),
    }
}

#[test]
fn e1_coalesce_boxes_error_invalid_interval() {
    let e1 = E1::new(vec![(0.1, 0.5)], true).unwrap();

    // Zero-width interval
    let result = e1.can_coalesce_boxes((0.3, 0.3));
    assert!(
        result.is_err(),
        "Zero-width coalescing interval should fail"
    );

    // Inverted interval
    let result = e1.can_coalesce_boxes((0.5, 0.2));
    assert!(result.is_err(), "Inverted coalescing interval should fail");

    // Interval extending past 1.0
    let result = e1.can_coalesce_boxes((0.8, 1.5));
    assert!(result.is_err(), "Coalescing interval past 1.0 should fail");
}

#[test]
fn e1_min_closeness_measures_gap() {
    // Three intervals with known gaps: gap between 1st-2nd = 0.1, gap between 2nd-3rd = 0.2
    let e1 = E1::new(vec![(0.0, 0.2), (0.3, 0.5), (0.7, 0.9)], true).unwrap();
    let mc = e1.min_closeness();
    assert!(mc.is_some());
    assert!(
        (mc.unwrap() - 0.1).abs() < 0.001,
        "Expected min_closeness ~0.1, got {}",
        mc.unwrap()
    );
}

#[test]
fn e1_min_closeness_touching_intervals() {
    // Two intervals that touch: gap = 0.0
    let e1 = E1::new(vec![(0.0, 0.5), (0.5, 1.0)], true).unwrap();
    let mc = e1.min_closeness();
    assert!(mc.is_some());
    assert!(
        mc.unwrap().abs() < 0.001,
        "Touching intervals should have min_closeness ~0.0, got {}",
        mc.unwrap()
    );
}

#[test]
fn e1_min_closeness_single_and_empty() {
    // Arity 1: no pairs to compare
    let e1 = E1::new(vec![(0.0, 1.0)], true).unwrap();
    assert_eq!(
        e1.min_closeness(),
        None,
        "Single interval should return None"
    );

    // Arity 0: also no pairs
    let e1_empty = E1::new(vec![], true).unwrap();
    assert_eq!(
        e1_empty.min_closeness(),
        None,
        "Empty config should return None"
    );
}
