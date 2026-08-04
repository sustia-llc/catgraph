//! Relative-plus-absolute float comparison (#169).
//!
//! Workspace tests compare floats against scattered absolute epsilons —
//! `1e-12` / `1e-10` / `1e-9` / `5e-2`, at 80-odd sites in the integration
//! tests alone, before counting `#[cfg(test)]` modules and examples. Only a
//! couple derive their bound from an error analysis
//! (`catgraph-magnitude/tests/euler_char_identity.rs` and the `L·D·Lᵀ = ζ`
//! backward-error guard in `tests/magnitude_f64.rs`). An absolute epsilon is
//! only meaningful at one magnitude: `1e-9` is a strict test on values near
//! `1.0` and a vacuous one on values near `1e6`. [`approx_rel`] makes the
//! rel/abs split explicit so a reader can see which regime an assertion means
//! to police.
//!
//! This mirrors a convention the production code already uses —
//! `catgraph_magnitude::coalition_eval::INCREMENTAL_REL_TOL` is a **relative**
//! tolerance guarding two arithmetic routes to the same real number.

/// Returns `true` when `a` and `b` agree to within a relative tolerance `rel`
/// **or** an absolute floor `abs`.
///
/// The test is
///
/// ```text
/// |a − b| <= max(abs, rel · max(|a|, |b|))
/// ```
///
/// # Choosing `rel` and `abs`
///
/// - `rel` polices *significant digits*: `1e-9` means "agree to ~9 digits",
///   and stays meaningful whether the values are `1e-30` or `1e30`.
/// - `abs` is the floor that keeps the test from becoming impossible near
///   zero, where a relative bound demands exact equality. Set it to the noise
///   level the computation can actually reach — for a Gaussian elimination at
///   small `n`, a few orders above `f64::EPSILON`.
///
/// Passing `abs = 0.0` gives a pure relative test; passing `rel = 0.0` gives
/// the old absolute-epsilon behavior, which is occasionally what you want (a
/// quantity that is *defined* to live near a fixed scale).
///
/// # Non-finite values
///
/// - `NaN` on either side is never close to anything, including itself.
/// - Two identical infinities are close; opposite infinities are not, and an
///   infinity is never close to a finite value.
///
/// # Overflow caveat
///
/// Both `|a − b|` and `rel · max(|a|, |b|)` are computed in `f64`, so operands
/// at the top of the range can overflow the comparison. With `rel ∈ [1, 2)` and
/// `a`, `b` near `±f64::MAX` of opposite sign, both sides evaluate to infinity
/// and the `<=` holds, returning `true` where the exact answer is `false`
/// (e.g. `approx_rel(f64::MAX, -f64::MAX, 1.5, 0.0)`). This does not affect any
/// realistic test tolerance: for `rel < 1` the overflowed `|a − b|` correctly
/// exceeds a finite bound. A `NaN` passed as `rel` or `abs` is likewise
/// swallowed by `f64::max` and makes the comparison pass — pass real tolerances.
///
/// # Examples
///
/// ```
/// use catgraph_testutil::approx_rel;
///
/// // Relative agreement survives a change of scale that an absolute
/// // epsilon of 1e-9 would wave through.
/// assert!(approx_rel(1.0, 1.0 + 1e-12, 1e-9, 0.0));
/// assert!(!approx_rel(1e6, 1e6 + 1.0, 1e-9, 0.0));
///
/// // The absolute floor rescues comparisons against zero.
/// assert!(!approx_rel(0.0, 1e-30, 1e-9, 0.0));
/// assert!(approx_rel(0.0, 1e-30, 1e-9, 1e-20));
/// ```
#[must_use]
pub fn approx_rel(a: f64, b: f64, rel: f64, abs: f64) -> bool {
    if a.is_nan() || b.is_nan() {
        return false;
    }
    if a == b {
        // Covers exact equality including matching infinities and ±0.0.
        return true;
    }
    if a.is_infinite() || b.is_infinite() {
        // Not equal (handled above) and at least one is infinite: the
        // difference below would be inf or NaN, neither of which is a
        // meaningful closeness verdict.
        return false;
    }
    let scale = a.abs().max(b.abs());
    (a - b).abs() <= abs.max(rel * scale)
}

/// Assert [`approx_rel`], reporting both tolerances and the residual on failure.
///
/// Takes `(a, b, rel, abs)` plus an optional trailing `format!` message.
///
/// # Examples
///
/// ```
/// use catgraph_testutil::assert_approx_rel;
///
/// let computed = 1.0 + 1e-12;
/// assert_approx_rel!(computed, 1.0, 1e-9, 0.0);
/// assert_approx_rel!(computed, 1.0, 1e-9, 0.0, "round trip at n = {}", 4);
/// ```
#[macro_export]
macro_rules! assert_approx_rel {
    ($a:expr, $b:expr, $rel:expr, $abs:expr $(,)?) => {
        $crate::assert_approx_rel!($a, $b, $rel, $abs, "")
    };
    ($a:expr, $b:expr, $rel:expr, $abs:expr, $($msg:tt)+) => {{
        let a: f64 = $a;
        let b: f64 = $b;
        let rel: f64 = $rel;
        let abs: f64 = $abs;
        assert!(
            $crate::approx_rel(a, b, rel, abs),
            "approx_rel failed: {} vs {} \
             (residual = {:.3e}, allowed = {:.3e}, rel = {:.3e}, abs = {:.3e}) {}",
            a,
            b,
            (a - b).abs(),
            abs.max(rel * a.abs().max(b.abs())),
            rel,
            abs,
            format_args!($($msg)+),
        );
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_tolerance_tracks_scale() {
        // The same 9-digit relative agreement, three scales apart.
        assert!(approx_rel(1.0, 1.0 + 1e-12, 1e-9, 0.0));
        assert!(approx_rel(1e30, 1e30 * (1.0 + 1e-12), 1e-9, 0.0));
        assert!(approx_rel(1e-30, 1e-30 * (1.0 + 1e-12), 1e-9, 0.0));

        // ...and the same 9-digit disagreement, likewise.
        assert!(!approx_rel(1.0, 1.0 + 1e-6, 1e-9, 0.0));
        assert!(!approx_rel(1e30, 1e30 * (1.0 + 1e-6), 1e-9, 0.0));
    }

    #[test]
    fn absolute_epsilon_loses_meaning_at_scale_but_the_floor_is_explicit() {
        // This is the failure mode the helper exists to make visible: an
        // absolute 1e-9 is vacuous at 1e6, where consecutive f64 values are
        // already ~1e-10 apart, so a full unit of error slips through.
        let a = 1e6;
        let b = 1e6 + 1.0;
        assert!(
            approx_rel(a, b, 0.0, 1e9),
            "absolute-only, huge floor: passes"
        );
        assert!(!approx_rel(a, b, 1e-9, 0.0), "relative: correctly rejects");
    }

    #[test]
    fn zero_needs_the_absolute_floor() {
        // A relative bound against zero demands exact equality, because the
        // scale it multiplies is itself zero.
        assert!(!approx_rel(0.0, 1e-300, 1e-9, 0.0));
        assert!(approx_rel(0.0, 1e-300, 1e-9, 1e-200));
        assert!(approx_rel(0.0, 0.0, 0.0, 0.0));
        assert!(approx_rel(0.0, -0.0, 0.0, 0.0));
    }

    #[test]
    fn pure_absolute_mode_matches_the_old_convention() {
        // rel = 0.0 reproduces `(a - b).abs() < eps` (up to <= vs <).
        assert!(approx_rel(1.0, 1.0 + 5e-10, 0.0, 1e-9));
        assert!(!approx_rel(1.0, 1.0 + 5e-9, 0.0, 1e-9));
    }

    #[test]
    fn nan_is_never_close() {
        assert!(!approx_rel(f64::NAN, f64::NAN, 1e-9, 1e-9));
        assert!(!approx_rel(f64::NAN, 1.0, 1e-9, 1e-9));
        assert!(!approx_rel(1.0, f64::NAN, 1e-9, 1e-9));
        // Even an infinite tolerance must not rescue NaN.
        assert!(!approx_rel(f64::NAN, 1.0, f64::INFINITY, f64::INFINITY));
    }

    #[test]
    fn infinities_compare_by_identity() {
        assert!(approx_rel(f64::INFINITY, f64::INFINITY, 0.0, 0.0));
        assert!(approx_rel(f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, 0.0));
        assert!(!approx_rel(f64::INFINITY, f64::NEG_INFINITY, 1e-9, 1e-9));
        assert!(!approx_rel(f64::INFINITY, 1e300, 1e-9, 1e-9));
    }

    #[test]
    fn is_symmetric() {
        // The scale is max(|a|, |b|), so the verdict cannot depend on argument
        // order — a property the naive `|a-b| <= rel*|a|` form does not have.
        let samples = [0.0, -0.0, 1.0, -1.0, 1e-300, 1e300, 3.7, -2.5e8];
        for &a in &samples {
            for &b in &samples {
                assert_eq!(
                    approx_rel(a, b, 1e-9, 1e-12),
                    approx_rel(b, a, 1e-9, 1e-12),
                    "asymmetric verdict for ({a}, {b})"
                );
            }
        }
    }

    #[test]
    fn macro_accepts_both_arities() {
        assert_approx_rel!(1.0, 1.0 + 1e-12, 1e-9, 0.0);
        assert_approx_rel!(1.0, 1.0 + 1e-12, 1e-9, 0.0, "with context {}", 42);
    }
}
