//! Relative-plus-absolute float comparison.

/// Returns `true` when `a` and `b` agree to within a relative tolerance `rel`
/// **or** an absolute floor `abs`:
///
/// ```text
/// |a − b| <= max(abs, rel · max(|a|, |b|))
/// ```
///
/// `abs = 0.0` gives a pure relative test; `rel = 0.0` a pure absolute one.
/// `NaN` on either side compares as not close, itself included; matching
/// infinities are close, opposite ones are not, and an infinity is not close to
/// a finite value.
///
/// Both sides of the comparison are computed in `f64` and so saturate: with
/// `rel ∈ [1, 2)` and `a`, `b` near `±f64::MAX` of opposite sign both sides are
/// infinite and the `<=` holds, so `approx_rel(f64::MAX, -f64::MAX, 1.5, 0.0)`
/// is `true`; a `NaN` `rel` or `abs` is swallowed by `f64::max` and passes.
///
/// # Examples
///
/// ```
/// use catgraph_testutil::approx_rel;
///
/// assert!(approx_rel(1.0, 1.0 + 1e-12, 1e-9, 0.0));
/// assert!(!approx_rel(1e6, 1e6 + 1.0, 1e-9, 0.0));
///
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
        // Unequal with one side infinite: the difference below is inf or NaN.
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
        assert!(!approx_rel(0.0, 1e-300, 1e-9, 0.0));
        assert!(approx_rel(0.0, 1e-300, 1e-9, 1e-200));
        assert!(approx_rel(0.0, 0.0, 0.0, 0.0));
        assert!(approx_rel(0.0, -0.0, 0.0, 0.0));
    }

    #[test]
    fn pure_absolute_mode_matches_the_old_convention() {
        assert!(approx_rel(1.0, 1.0 + 5e-10, 0.0, 1e-9));
        assert!(!approx_rel(1.0, 1.0 + 5e-9, 0.0, 1e-9));
    }

    #[test]
    fn nan_is_never_close() {
        assert!(!approx_rel(f64::NAN, f64::NAN, 1e-9, 1e-9));
        assert!(!approx_rel(f64::NAN, 1.0, 1e-9, 1e-9));
        assert!(!approx_rel(1.0, f64::NAN, 1e-9, 1e-9));
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
