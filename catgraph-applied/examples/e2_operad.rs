//! E2 operad (little disks) API demonstration.
//!
//! Shows constructing E2 configurations of disks in the unit disk,
//! identity, operadic substitution, coalescence, min_closeness,
//! embedding from E1, name changes, and extracting sub-circles.

use catgraph::category::HasIdentity;
use catgraph::operadic::Operadic;
use catgraph_applied::e1_operad::E1;
use catgraph_applied::e2_operad::E2;

// ============================================================================
// Constructing E2 configurations
// ============================================================================

fn constructing() {
    println!("=== Constructing E2 Configurations ===\n");

    // A valid 2-ary configuration: two non-overlapping disks inside the unit disk
    // Each disk is (name, (center_x, center_y), radius)
    let disks = vec![("left", (-0.5, 0.0), 0.3), ("right", (0.5, 0.0), 0.3)];
    let config = E2::new(disks, true);
    println!("2 disjoint disks         = {:?}", config.is_ok());

    // Invalid: overlapping disks (with overlap check)
    let overlapping = vec![(0, (0.0, 0.0), 0.5), (1, (0.3, 0.0), 0.5)];
    let bad = E2::new(overlapping, true);
    println!(
        "overlapping (checked)    = {:?}",
        bad.err().map(|e| e.to_string())
    );

    // Invalid: disk extends outside the unit disk
    let outside = vec![(0, (0.8, 0.0), 0.5)];
    let bad2 = E2::new(outside, true);
    println!(
        "outside unit disk        = {:?}",
        bad2.err().map(|e| e.to_string())
    );

    // Duplicate names are rejected
    let dup_names = vec![(0, (-0.3, 0.0), 0.2), (0, (0.3, 0.0), 0.2)];
    let bad3 = E2::new(dup_names, true);
    println!(
        "duplicate names          = {:?}",
        bad3.err().map(|e| e.to_string())
    );

    // Empty (nullary) configuration
    let nullary: E2<i32> = E2::new(vec![], true).unwrap();
    println!("nullary (0 disks)        = {:?}", nullary.min_closeness());

    println!();
}

// ============================================================================
// Identity
// ============================================================================

fn identity() {
    println!("=== Identity ===\n");

    // The identity is a single disk covering the entire unit disk
    let id: E2<&str> = E2::identity(&"center");
    let circles = id.extract_sub_circles();
    println!("identity sub_circles = {:?}", circles);
    println!("  (one disk at origin with radius 1.0, named \"center\")");

    println!();
}

// ============================================================================
// Operadic substitution
// ============================================================================

fn operadic_substitution() {
    println!("=== Operadic Substitution ===\n");

    // Outer: two disks
    let mut outer = E2::new(vec![("a", (-0.5, 0.0), 0.3), ("b", (0.5, 0.0), 0.3)], true).unwrap();

    // Inner: two smaller disks
    let inner = E2::new(vec![("c", (-0.3, 0.0), 0.4), ("d", (0.3, 0.0), 0.4)], false).unwrap();

    // Substitute inner into disk "a" of outer.
    // Inner's disks get rescaled to fit inside outer's disk "a".
    let result = outer.operadic_substitution("a", inner);
    println!("substitution into 'a'    = {:?}", result.is_ok());
    let circles = outer.extract_sub_circles();
    println!("after substitution       = {:?}", circles);
    println!("  (disk 'a' replaced by rescaled 'c' and 'd', plus original 'b')");

    // Identity substitution: replace a disk with the identity -> no change
    let mut config = E2::new(vec![("x", (0.0, 0.0), 0.5)], true).unwrap();
    let id = E2::identity(&"x");
    let result = config.operadic_substitution("x", id);
    println!("\nsubstitute identity      = {:?}", result.is_ok());
    println!(
        "after identity sub       = {:?}",
        config.extract_sub_circles()
    );

    // Substituting a nullary removes the disk
    let mut config = E2::new(vec![("p", (-0.4, 0.0), 0.2), ("q", (0.4, 0.0), 0.2)], true).unwrap();
    let nullary: E2<&str> = E2::new(vec![], true).unwrap();
    let result = config.operadic_substitution("p", nullary);
    println!("\nsubstitute nullary       = {:?}", result.is_ok());
    println!(
        "after removing 'p'       = {:?}",
        config.extract_sub_circles()
    );

    // Non-existent input name fails
    let mut config = E2::new(vec![("only", (0.0, 0.0), 0.5)], true).unwrap();
    let extra = E2::identity(&"z");
    let result = config.operadic_substitution("missing", extra);
    println!(
        "\nnon-existent input       = {:?}",
        result.err().map(|e| e.to_string())
    );

    println!();
}

// ============================================================================
// Coalescence
// ============================================================================

fn coalescence() {
    println!("=== Coalescence ===\n");

    // Two small disks; coalesce them into a single larger disk
    let mut config = E2::new(vec![(0, (0.0, 0.2), 0.15), (1, (0.0, -0.2), 0.15)], true).unwrap();

    // Check if the coalescing disk can contain both sub-disks
    let can = config.can_coalesce_boxes(((0.0, 0.0), 0.5));
    println!("can coalesce at (0,0) r=0.5 = {:?}", can.is_ok());

    // Perform coalescence
    let result = config.coalesce_boxes((99, (0.0, 0.0), 0.5));
    println!("coalescence result          = {:?}", result.is_ok());
    println!(
        "after coalescence           = {:?}",
        config.extract_sub_circles()
    );

    // Invalid: coalescing disk outside the unit disk
    let config2: E2<i32> = E2::new(vec![], true).unwrap();
    let can2 = config2.can_coalesce_boxes(((2.0, 0.0), 0.5));
    println!("\ncoalesce outside unit disk   = {:?}", can2.err());

    println!();
}

// ============================================================================
// Min closeness
// ============================================================================

fn min_closeness() {
    println!("=== Min Closeness ===\n");

    let config = E2::new(vec![(0, (0.0, 0.0), 0.2), (1, (0.6, 0.0), 0.2)], true).unwrap();
    println!("disks: (0,0) r=0.2 and (0.6,0) r=0.2");
    println!("min_closeness = {:?}", config.min_closeness());
    println!("  (center distance 0.6, radii sum 0.4, gap = 0.2)");

    // Three disks: the closest pair determines min_closeness
    let config3 = E2::new(
        vec![
            (0, (-0.5, 0.0), 0.15),
            (1, (0.0, 0.0), 0.15),
            (2, (0.5, 0.0), 0.15),
        ],
        true,
    )
    .unwrap();
    println!("\n3 disks along x-axis, r=0.15, spaced 0.5 apart");
    println!("min_closeness = {:?}", config3.min_closeness());
    println!("  (gap = 0.5 - 0.3 = 0.2 between adjacent pairs)");

    // Single disk: no closeness defined
    let single = E2::new(vec![(0, (0.0, 0.0), 0.5)], true).unwrap();
    println!("\nsingle disk");
    println!("min_closeness = {:?}", single.min_closeness());

    println!();
}

// ============================================================================
// Embedding from E1
// ============================================================================

fn from_e1() {
    println!("=== Embedding from E1 ===\n");

    // Create an E1 configuration
    let e1 = E1::new(vec![(0.1, 0.3), (0.6, 0.8)], true).unwrap();
    println!("E1 intervals: [0.1,0.3], [0.6,0.8]");

    // Embed into E2: intervals map to disks along the x-axis
    let e2: E2<usize> = E2::from_e1_config(e1, |idx| idx);
    let circles = e2.extract_sub_circles();
    println!("embedded as E2 disks:");
    for (name, center, radius) in &circles {
        println!(
            "  name={name}, center=({:.2},{:.2}), radius={radius:.2}",
            center.0, center.1
        );
    }
    println!("  (centers derived from interval midpoints mapped to [-1,1])");

    // Identity in E1 maps to the full unit disk in E2
    let e1_id = E1::identity(&());
    let e2_from_id: E2<usize> = E2::from_e1_config(e1_id, |idx| idx);
    let id_circles = e2_from_id.extract_sub_circles();
    println!("\nE1 identity -> E2:");
    for (name, center, radius) in &id_circles {
        println!(
            "  name={name}, center=({:.2},{:.2}), radius={radius:.2}",
            center.0, center.1
        );
    }

    println!();
}

// ============================================================================
// Name operations
// ============================================================================

fn name_operations() {
    println!("=== Name Operations ===\n");

    // change_names: transform all disk names with a function
    let config = E2::new(vec![(0, (-0.3, 0.0), 0.2), (1, (0.3, 0.0), 0.2)], true).unwrap();
    let renamed = config.change_names(|n| format!("disk_{n}"));
    let circles = renamed.extract_sub_circles();
    println!("change_names (int -> string):");
    for (name, center, radius) in &circles {
        println!(
            "  name={name}, center=({:.2},{:.2}), radius={radius:.2}",
            center.0, center.1
        );
    }

    // change_name: rename a single disk
    let mut config2 = E2::new(
        vec![("alpha", (-0.3, 0.0), 0.2), ("beta", (0.3, 0.0), 0.2)],
        true,
    )
    .unwrap();
    config2.change_name(("alpha", "gamma"));
    let circles2 = config2.extract_sub_circles();
    println!("\nchange_name 'alpha' -> 'gamma':");
    for (name, center, radius) in &circles2 {
        println!(
            "  name={name}, center=({:.2},{:.2}), radius={radius:.2}",
            center.0, center.1
        );
    }

    println!();
}

fn main() {
    constructing();
    identity();
    operadic_substitution();
    coalescence();
    min_closeness();
    from_e1();
    name_operations();
}
