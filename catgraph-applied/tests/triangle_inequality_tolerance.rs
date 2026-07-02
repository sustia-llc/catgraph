//! Issue #29 regression: `LawvereMetricSpace::triangle_inequality_holds_within`.
//!
//! Distances derived from `−ln` of max-product closures satisfy the triangle
//! inequality mathematically, but `−ln(a·b)` and `(−ln a)+(−ln b)` differ by a
//! few ULPs, so the exact (zero-tolerance) check spuriously fails on realistic
//! non-dyadic couplings. `triangle_inequality_holds_within(tol)` absorbs that
//! ULP noise; `triangle_inequality_holds()` (== `_within(0.0)`) stays exact.

#![allow(clippy::float_cmp)]

use catgraph_applied::lawvere_metric::LawvereMetricSpace;

/// Build a 3-object space over `0..3` from a directed distance table
/// (`table[from][to]`, `+∞` for "unreachable").
fn space_from_table(table: [[f64; 3]; 3]) -> LawvereMetricSpace<usize> {
    LawvereMetricSpace::<usize>::from_distance_fn(3, |a, b| table[a][b])
}

/// A genuine (macroscopic) violation is caught at *any* sane tolerance, and the
/// exact check equals `_within(0.0)`.
#[test]
fn genuine_violation_is_caught() {
    // d(0,2) = 5 but d(0,1) + d(1,2) = 2 — a real triangle-inequality breach.
    let inf = f64::INFINITY;
    let table = [[0.0, 1.0, 5.0], [inf, 0.0, 1.0], [inf, inf, 0.0]];
    let space = space_from_table(table);
    assert!(
        !space.triangle_inequality_holds(),
        "5 > 1 + 1 is a real violation"
    );
    assert!(
        !space.triangle_inequality_holds_within(0.0),
        "_within(0.0) must equal the exact check"
    );
    assert!(
        !space.triangle_inequality_holds_within(1e-9),
        "a macroscopic 5 vs 2 breach survives a 1e-9 tolerance"
    );
}

/// A ULP-scale "violation" produced by the `−ln`-of-product vs sum-of-`−ln`
/// rewrite fails the exact check but passes at `tol = 1e-9`.
#[test]
fn ulp_scale_violation_passes_within_tolerance() {
    // p = 1/3, q = 1/6 (non-dyadic). The closed shortest path 0→1→2 has
    // probability p·q, so d(0,2) = −ln(p·q) while d(0,1)+d(1,2) = −ln p − ln q.
    // These differ by ~4.44e-16 with d(0,2) strictly larger — a spurious breach.
    let p = 1.0_f64 / 3.0;
    let q = 1.0_f64 / 6.0;
    let d01 = -p.ln();
    let d12 = -q.ln();
    let d02 = -(p * q).ln();
    // Sanity: the ULP breach is real and strictly positive in this build.
    assert!(
        d02 > d01 + d12,
        "expected a ULP-scale strict breach on 1/3, 1/6"
    );

    let inf = f64::INFINITY;
    let table = [[0.0, d01, d02], [inf, 0.0, d12], [inf, inf, 0.0]];
    let space = space_from_table(table);

    // Exact check (and its alias) catch the ULP noise as a "violation" ...
    assert!(
        !space.triangle_inequality_holds(),
        "exact check trips on ULP noise"
    );
    assert!(
        !space.triangle_inequality_holds_within(0.0),
        "_within(0.0) ≡ exact"
    );
    // ... but a 1e-9 tolerance (orders above ULP noise) accepts it.
    assert!(
        space.triangle_inequality_holds_within(1e-9),
        "1e-9 tolerance must absorb the ~4e-16 ULP breach"
    );
}

/// Infinity semantics are preserved: `sum = +∞ ⇒` never a violation;
/// `d(x, z) = +∞` with a finite sum ⇒ a violation (tolerance cannot rescue it).
#[test]
fn infinity_semantics_preserved() {
    let inf = f64::INFINITY;

    // Case A — sum = +∞ ⇒ ok. Only finite forward distance is d(0,2) = 1; the
    // path 0→1→2 is unreachable (both legs +∞), so its sum is +∞ ≥ 1.
    let sum_inf = [[0.0, inf, 1.0], [inf, 0.0, inf], [inf, inf, 0.0]];
    let space_a = space_from_table(sum_inf);
    assert!(
        space_a.triangle_inequality_holds_within(1e-9),
        "sum = +∞ is never a violation (+∞ + tol = +∞)"
    );

    // Case B — d(0,2) = +∞ while d(0,1)+d(1,2) = 2 (finite) ⇒ a violation.
    let dxz_inf = [[0.0, 1.0, inf], [inf, 0.0, 1.0], [inf, inf, 0.0]];
    let space_b = space_from_table(dxz_inf);
    assert!(
        !space_b.triangle_inequality_holds_within(1e-9),
        "d(x,z) = +∞ over a finite sum is a violation the tolerance cannot absorb"
    );
    assert!(
        !space_b.triangle_inequality_holds(),
        "exact check agrees on the infinite-dxz violation"
    );
}

/// A clean, well-separated metric passes both the exact and tolerant checks —
/// tolerance does not mask a valid space.
#[test]
fn clean_metric_passes_exact_and_tolerant() {
    let table = [[0.0, 1.0, 2.0], [1.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
    let space = space_from_table(table);
    assert!(space.triangle_inequality_holds());
    assert!(space.triangle_inequality_holds_within(0.0));
    assert!(space.triangle_inequality_holds_within(1e-9));
}
