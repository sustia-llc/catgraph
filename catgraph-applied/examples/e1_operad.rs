//! E1 operad (little intervals) API demonstration.
//!
//! Shows constructing E1 configurations of intervals in [0,1], identity,
//! operadic substitution, coalescence, min_closeness, and the monoid
//! homomorphism via go_to_monoid.

use catgraph::category::HasIdentity;
use catgraph::operadic::Operadic;
use catgraph_applied::e1_operad::E1;

// ============================================================================
// Constructing E1 configurations
// ============================================================================

fn constructing() {
    println!("=== Constructing E1 Configurations ===\n");

    // A valid 3-ary configuration: three disjoint intervals in [0,1]
    let intervals = vec![(0.0, 0.2), (0.3, 0.5), (0.7, 0.9)];
    let config = E1::new(intervals, true);
    println!("3 disjoint intervals     = {:?}", config.is_ok());

    // Invalid: overlapping intervals (with overlap check enabled)
    let overlapping = vec![(0.0, 0.5), (0.4, 0.8)];
    let bad = E1::new(overlapping, true);
    println!(
        "overlapping (checked)    = {:?}",
        bad.err().map(|e| e.to_string())
    );

    // Invalid: interval extends beyond [0,1]
    let out_of_range = vec![(0.5, 1.5)];
    let bad2 = E1::new(out_of_range, true);
    println!(
        "out of range             = {:?}",
        bad2.err().map(|e| e.to_string())
    );

    // Invalid: empty interval (start >= end)
    let degenerate = vec![(0.5, 0.5)];
    let bad3 = E1::new(degenerate, true);
    println!(
        "degenerate (zero width)  = {:?}",
        bad3.err().map(|e| e.to_string())
    );

    // Empty (nullary) configuration is valid
    let nullary = E1::new(vec![], true);
    println!("nullary (0 intervals)    = {:?}", nullary.is_ok());

    println!();
}

// ============================================================================
// Identity
// ============================================================================

fn identity() {
    println!("=== Identity ===\n");

    // The identity is the single interval [0,1] — a 1-ary operation
    let id = E1::identity(&());
    let intervals = id.extract_sub_intervals();
    println!("identity intervals = {:?}", intervals);
    println!("  (the entire [0,1] interval, arity 1)");

    println!();
}

// ============================================================================
// Operadic substitution
// ============================================================================

fn operadic_substitution() {
    println!("=== Operadic Substitution ===\n");

    // Outer: two intervals [0.1, 0.4] and [0.6, 0.9]
    let outer_display = E1::new(vec![(0.1, 0.4), (0.6, 0.9)], true).unwrap();
    println!(
        "outer intervals = {:?}",
        outer_display.extract_sub_intervals()
    );

    // Rebuild outer for mutation (extract_sub_intervals consumes)
    let mut outer = E1::new(vec![(0.1, 0.4), (0.6, 0.9)], true).unwrap();

    // Inner: three small intervals
    let inner = E1::new(vec![(0.0, 0.3), (0.4, 0.6), (0.7, 1.0)], true).unwrap();

    // Substitute inner into the first slot (index 0) of outer.
    // The inner's intervals get rescaled to fit inside outer's [0.1, 0.4].
    let result = outer.operadic_substitution(0, inner);
    println!("substitution result      = {:?}", result.is_ok());
    println!(
        "after substitution       = {:?}",
        outer.extract_sub_intervals()
    );
    println!("  (inner's 3 intervals replace outer's 1st, arity now 4)");

    // Substituting identity leaves the configuration unchanged
    let mut config = E1::new(vec![(0.2, 0.5), (0.7, 0.9)], true).unwrap();
    let id = E1::identity(&());
    let result = config.operadic_substitution(1, id);
    println!("\nsubstitute identity      = {:?}", result.is_ok());
    println!(
        "unchanged config         = {:?}",
        config.extract_sub_intervals()
    );

    // Substituting a nullary operation removes that slot
    let mut config = E1::new(vec![(0.1, 0.3), (0.5, 0.8)], true).unwrap();
    let nullary = E1::new(vec![], true).unwrap();
    let result = config.operadic_substitution(0, nullary);
    println!("\nsubstitute nullary       = {:?}", result.is_ok());
    println!(
        "after removing slot 0    = {:?}",
        config.extract_sub_intervals()
    );

    // Out-of-bounds substitution fails
    let mut config = E1::new(vec![(0.1, 0.3)], true).unwrap();
    let extra = E1::identity(&());
    let result = config.operadic_substitution(5, extra);
    println!(
        "\nout-of-bounds sub        = {:?}",
        result.err().map(|e| e.to_string())
    );

    println!();
}

// ============================================================================
// Coalescence
// ============================================================================

fn coalescence() {
    println!("=== Coalescence ===\n");

    // Three intervals; coalesce the first two into one larger interval
    let config_display = E1::new(vec![(0.1, 0.2), (0.3, 0.4), (0.7, 0.9)], true).unwrap();
    println!(
        "before coalescence       = {:?}",
        config_display.extract_sub_intervals()
    );

    let mut config = E1::new(vec![(0.1, 0.2), (0.3, 0.4), (0.7, 0.9)], true).unwrap();

    // Check if coalescence is valid (all intervals must be contained or disjoint)
    let can = config.can_coalesce_boxes((0.05, 0.45));
    println!("can coalesce [0.05,0.45] = {:?}", can.is_ok());

    // Perform the coalescence
    let result = config.coalesce_boxes((0.05, 0.45));
    println!("coalesce result          = {:?}", result.is_ok());
    println!(
        "after coalescence        = {:?}",
        config.extract_sub_intervals()
    );
    println!("  (two intervals merged into one)");

    // Invalid coalescence: interval partially overlaps
    let config2 = E1::new(vec![(0.1, 0.3), (0.4, 0.7)], true).unwrap();
    let can2 = config2.can_coalesce_boxes((0.2, 0.5));
    println!("\npartial overlap check    = {:?}", can2.is_err());

    println!();
}

// ============================================================================
// Min closeness
// ============================================================================

fn min_closeness() {
    println!("=== Min Closeness ===\n");

    let config = E1::new(vec![(0.0, 0.2), (0.5, 0.7), (0.8, 1.0)], true).unwrap();
    println!("intervals: [0,0.2], [0.5,0.7], [0.8,1.0]");
    println!("min_closeness = {:?}", config.min_closeness());
    println!("  (gap between [0.5,0.7] and [0.8,1.0] = 0.1)");

    let wide_gaps = E1::new(vec![(0.0, 0.1), (0.5, 0.6)], true).unwrap();
    println!("\nintervals: [0,0.1], [0.5,0.6]");
    println!("min_closeness = {:?}", wide_gaps.min_closeness());

    // Single interval: no closeness defined
    let single = E1::new(vec![(0.2, 0.8)], true).unwrap();
    println!("\nsingle interval");
    println!("min_closeness = {:?}", single.min_closeness());

    println!();
}

// ============================================================================
// Monoid homomorphism (go_to_monoid)
// ============================================================================

fn monoid_homomorphism() {
    println!("=== Monoid Homomorphism ===\n");

    // go_to_monoid maps intervals to a monoid, multiplying the results together.
    // Example: compute the product of interval widths.
    let mut config = E1::new(vec![(0.0, 0.3), (0.5, 0.8)], false).unwrap();
    let product: f64 = config.go_to_monoid(|(a, b)| f64::from(b - a));
    println!("intervals: [0,0.3], [0.5,0.8]");
    println!("product of widths = {product:.6}");
    println!("  (0.3 * 0.3 = 0.09)");

    // With identity (single interval [0,1]): width = 1.0
    let mut id = E1::identity(&());
    let id_product: f64 = id.go_to_monoid(|(a, b)| f64::from(b - a));
    println!("\nidentity width product = {id_product:.6}");

    println!();
}

fn main() {
    constructing();
    identity();
    operadic_substitution();
    coalescence();
    min_closeness();
    monoid_homomorphism();
}
