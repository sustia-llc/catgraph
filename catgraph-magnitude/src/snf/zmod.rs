//! Z/N ring helpers — Rust port of `modularsnf/src/ring.rs`.
//!
//! Reference: Storjohann 2000, *Algorithms for Matrix Canonical Forms*, §1.1.
//!
//! License notice: this file is a port of Apache-2.0-licensed code from
//! `events555/modularsnf` at SHA `d62535e`.
//!
//! All helpers are `pub` for use by the `snf::echelon` and `snf::band`
//! modules (Phase C, Tasks 13–15) and by the integration test suite under
//! `tests/snf_*.rs`. `cargo test --lib snf::zmod` exercises them through
//! the in-module test suite below.

#![allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::must_use_candidate,
    reason = "posmod_i128 truncates a result already reduced mod n, so |result| < n <= i64::MAX; single-char names (a, b, n, g, s, t, u, v, q) match Storjohann §1.1 + textbook Bezout conventions; the helpers are pure mod-N arithmetic primitives whose return values are always consumed by the SNF interior — per-fn #[must_use] would be redundant noise louder than the bug it'd catch"
)]

/// Extended GCD: returns `(g, x, y)` such that `a*x + b*y = g`.
#[inline]
pub fn egcd(a: i64, b: i64) -> (i64, i64, i64) {
    let (mut r0, mut r1) = (a, b);
    let (mut s0, mut s1) = (1i64, 0i64);
    let (mut t0, mut t1) = (0i64, 1i64);
    while r1 != 0 {
        let q = r0 / r1;
        let tmp = r1;
        r1 = r0 - q * r1;
        r0 = tmp;
        let tmp = s1;
        s1 = s0 - q * s1;
        s0 = tmp;
        let tmp = t1;
        t1 = t0 - q * t1;
        t0 = tmp;
    }
    (r0, s0, t0)
}

/// Plain GCD over `i64`, taking absolute values up front.
///
/// # Caveats
///
/// `i64::MIN` is unsupported as an input — `.abs()` overflows. SNF callers
/// always normalize through `posmod` first, so this cannot fire in practice;
/// kept undocumented in modularsnf for parity but surfaced here for clarity.
#[inline]
pub fn gcd_raw(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Positive modulo over `i64` — always returns a value in `[0, n)`.
#[inline]
pub fn posmod(a: i64, n: i64) -> i64 {
    ((a % n) + n) % n
}

/// Positive modulo over a widened `i128` intermediate, returning a value in `[0, n)`.
#[inline]
pub fn posmod_i128(a: i128, n: i64) -> i64 {
    let n128 = i128::from(n);
    (((a % n128) + n128) % n128) as i64
}

/// Modular addition `(a + b) mod n` with `i128` intermediate.
#[inline]
pub fn add_mod(a: i64, b: i64, n: i64) -> i64 {
    posmod_i128(i128::from(a) + i128::from(b), n)
}

/// Modular subtraction `(a − b) mod n` with `i128` intermediate.
#[inline]
pub fn sub_mod(a: i64, b: i64, n: i64) -> i64 {
    posmod_i128(i128::from(a) - i128::from(b), n)
}

/// Modular multiplication `(a · b) mod n` with `i128` intermediate.
#[inline]
pub fn mul_mod(a: i64, b: i64, n: i64) -> i64 {
    posmod_i128(i128::from(a) * i128::from(b), n)
}

/// Three-way GCD: `gcd(a, b, n)`.
// Mirrors RingZModN::gcd; differs from plan stub by normalizing a, b through posmod first.
#[inline]
pub fn gcd_three(a: i64, b: i64, n: i64) -> i64 {
    gcd_raw(posmod(a, n), gcd_raw(posmod(b, n), n))
}

/// Extended GCD over `Z/nZ` returning a unimodular 2×2 row-reduction matrix.
///
/// Returns `(g, [[s, t], [u, v]])` where:
///
/// - `m[0][0] = s`, `m[0][1] = t` — Bezout coefficients: `s·a + t·b ≡ g (mod n)`.
/// - `m[1][0] = u`, `m[1][1] = v` — eliminating row: `u·a + v·b ≡ 0 (mod n)`.
/// - `det(M) = s·v − t·u` is a unit mod `n` (the matrix is unimodular over `Z/nZ`).
///
/// Equivalently, with `g = gcd(a, b, n)`:
///
/// ```text
/// [[s, t], [u, v]] · [a, b]^T ≡ [g, 0]^T  (mod n).
/// ```
///
/// Mirrors the modularsnf `RingZModN::gcdex` algorithm (Storjohann §1.1
/// "Atomic Reduction").
pub fn gcdex(a: i64, b: i64, n: i64) -> (i64, [[i64; 2]; 2]) {
    let a_val = posmod(a, n);
    let b_val = posmod(b, n);

    // Fast path: b is a multiple of a in Z/N. The single guard
    // `gcd_raw(a_val, n) != 0 && b_val % gcd_raw(a_val, n) == 0` is sufficient
    // (egcd(a_val, n).0 == gcd_raw(a_val, n) by definition); modularsnf's
    // RingZModN::gcdex collapses both checks into the same predicate.
    if a_val != 0 {
        let g = gcd_raw(a_val, n);
        if g != 0 && b_val % g == 0 {
            // Inline RingZModN::div(b_val, a_val): the (g, x, _) here equals
            // (gcd_raw(a_val, n), egcd(a_val, n).1, _).
            let (_, x, _) = egcd(a_val, n);
            let q = posmod_i128(i128::from(x) * i128::from(b_val / g), n / g);
            return (a_val, [[1, 0], [posmod(-q, n), 1]]);
        }
    }

    // Standard extended Euclidean.
    let (mut r0, mut r1) = (a_val, b_val);
    let (mut s0, mut s1) = (1i64, 0i64);
    let (mut t0, mut t1) = (0i64, 1i64);

    while r1 != 0 {
        let q = r0 / r1;
        let tmp = r1;
        r1 = r0 - q * r1;
        r0 = tmp;
        let tmp = s1;
        s1 = s0 - q * s1;
        s0 = tmp;
        let tmp = t1;
        t1 = t0 - q * t1;
        t0 = tmp;
    }

    if r0 == 0 {
        return (0, [[1, 0], [0, 1]]);
    }

    let u = -(b_val / r0);
    let v = a_val / r0;

    (
        posmod(r0, n),
        [[posmod(s0, n), posmod(t0, n)], [posmod(u, n), posmod(v, n)]],
    )
}

/// Stabilizer per Storjohann Lemma 1.1: returns `c` such that
/// `gcd(a + c·b, n) = gcd(a, b, n)`.
///
/// # Panics
///
/// - **Debug-only**: panics if `n < 2`. The stabilizer is undefined for the
///   trivial moduli `n ∈ {0, 1}` (and `n < 0` violates the `Z/nZ` precondition);
///   the search loop `0..n` is empty in those cases.
/// - **Always**: panics with an invariant-violation message if the search
///   exhausts `0..n` without finding a stabilizer. By Storjohann 2000 Lemma 1.1
///   ("Atomic Reduction"), a stabilizer always exists for `n >= 2`, so reaching
///   this branch indicates a caller-side invariant violation (e.g., a value
///   outside `Z/nZ` slipped through a normalization step).
pub fn stab(a: i64, b: i64, n: i64) -> i64 {
    debug_assert!(n >= 2, "stab: requires n >= 2 (modulus precondition)");
    let a = posmod(a, n);
    let b = posmod(b, n);
    let target = gcd_three(a, b, n);
    for x in 0..n {
        let candidate = posmod_i128(i128::from(a) + i128::from(x) * i128::from(b), n);
        let current = gcd_raw(candidate, n);
        if current == target {
            return x;
        }
    }
    panic!("stab: invariant violation — no stabilizer found for a={a}, b={b}, n={n}")
}

/// Z/N division: returns the quotient `a/b` if `b` divides `a` mod n.
///
/// Mirrors `modularsnf::ring::RingZModN::div`. Returns `Err` when the
/// quotient does not exist in `Z/nZ` (i.e., `gcd(b, n) ∤ a`).
///
/// # Errors
///
/// Returns an error string when the modular quotient does not exist
/// (i.e., `gcd(b, n)` does not divide `a` in `Z/nZ`).
pub fn div(a: i64, b: i64, n: i64) -> Result<i64, String> {
    let a_val = posmod(a, n);
    let b_val = posmod(b, n);
    let (g, x, _) = egcd(b_val, n);
    if g == 0 || a_val % g != 0 {
        return Err(format!("{a} not divisible by {b} in Z/{n}"));
    }
    Ok(posmod_i128(i128::from(x) * i128::from(a_val / g), n / g))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn egcd_recovers_bezout_identity() {
        let (g, s, t) = egcd(12, 18);
        assert_eq!(g, 6);
        assert_eq!(12 * s + 18 * t, g);
    }

    #[test]
    #[allow(
        clippy::erasing_op,
        reason = "preserve the `0·s` term to make the Bezout identity 0·s + 5·t = g textually obvious"
    )]
    fn egcd_handles_zero() {
        let (g, s, t) = egcd(0, 5);
        assert_eq!(g, 5);
        assert_eq!(0 * s + 5 * t, g);
    }

    #[test]
    fn gcdex_unimodular_matrix_eliminates_b() {
        // For (a, b) = (12, 18) mod 36:
        // [[s, t], [u, v]] * [a, b]^T = [g, 0]^T with g = gcd(a, b, N) = 6.
        let n = 36;
        let (g, m) = gcdex(12, 18, n);
        assert_eq!(g, 6);
        // Verify: m[0][0]*12 + m[0][1]*18 ≡ 6 (mod 36)
        let lhs0 = posmod_i128(i128::from(m[0][0]) * 12 + i128::from(m[0][1]) * 18, n);
        assert_eq!(lhs0, 6);
        // Verify: m[1][0]*12 + m[1][1]*18 ≡ 0 (mod 36)
        let lhs1 = posmod_i128(i128::from(m[1][0]) * 12 + i128::from(m[1][1]) * 18, n);
        assert_eq!(lhs1, 0);
        // Verify det(m) is a unit mod 36 (gcd(det, 36) = 1).
        let det = posmod_i128(
            i128::from(m[0][0]) * i128::from(m[1][1]) - i128::from(m[0][1]) * i128::from(m[1][0]),
            n,
        );
        assert_eq!(gcd_raw(det, n), 1);
    }

    #[test]
    fn stab_lemma_1_1_property() {
        // ∃ c such that gcd(a + cb, N) = gcd(a, b, N).
        let n = 36;
        let a = 4;
        let b = 6;
        let c = stab(a, b, n);
        let target = gcd_three(a, b, n);
        assert_eq!(gcd_raw(posmod_i128(i128::from(a + c * b), n), n), target);
    }

    #[test]
    fn div_recovers_quotient_when_divisible() {
        // 6 / 2 = 3 in Z/7 (2*3 = 6 mod 7)
        assert_eq!(div(6, 2, 7).unwrap(), 3);
    }

    #[test]
    fn div_errors_when_not_divisible() {
        // 1 / 2 not divisible in Z/4 (gcd(2,4) = 2 doesn't divide 1)
        assert!(div(1, 2, 4).is_err());
    }
}
