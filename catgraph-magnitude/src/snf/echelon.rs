//! Storjohann Lemma 3.1 row-echelon form over Z/N — Rust port of
//! `modularsnf/src/echelon.rs`.
//!
//! Reference: Storjohann 2000, *Algorithms for Matrix Canonical Forms*, §3.
//!
//! License notice: this file is a port of Apache-2.0-licensed code from
//! `events555/modularsnf` at SHA `d62535e`.
//!
//! Storage is `Vec<Vec<i64>>` (workspace stays ndarray-free per design doc §2.4);
//! `MatR<Q>` interop happens at the SNF public boundary in Task 17.

#![allow(
    clippy::many_single_char_names,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    reason = "Storjohann textbook conventions: a, b, n, g, s, t, u, v, k, r, i, j throughout §3 + §7; the 2×2 row-transform APIs intentionally take 8–9 atomic args (rows + s/t/u/v Bezout coeffs + modulus); the index loops walk paired m1/m2/u/t indices, not a single iter()"
)]

use crate::snf::zmod::{div, gcd_three, posmod, posmod_i128};

/// Construct the `n × n` identity matrix as `Vec<Vec<i64>>`.
///
/// Reused by `snf::band` (Task 14) for padded-matrix initialization in
/// `band_reduction`; promoted to `pub(crate)` over a separate `matrix_ops`
/// module since the surface area does not yet justify one.
#[inline]
pub(crate) fn identity(n: usize) -> Vec<Vec<i64>> {
    (0..n)
        .map(|i| (0..n).map(|j| i64::from(i == j)).collect())
        .collect()
}

/// Apply a 2×2 row transform to rows `r0`, `r1` of matrix `m` (in place).
///
/// ```text
/// [s t] [row_r0]   [new_r0]
/// [u v] [row_r1] = [new_r1]
/// ```
///
/// Mirrors `modularsnf::echelon::apply_row_2x2` (Storjohann §3, atomic row reduction).
#[inline]
pub(crate) fn apply_row_2x2(
    m: &mut [Vec<i64>],
    r0: usize,
    r1: usize,
    s: i64,
    t: i64,
    u: i64,
    v: i64,
    n: i64,
) {
    let cols = m[r0].len();
    for j in 0..cols {
        let a = m[r0][j];
        let b = m[r1][j];
        m[r0][j] = posmod_i128(
            i128::from(s) * i128::from(a) + i128::from(t) * i128::from(b),
            n,
        );
        m[r1][j] = posmod_i128(
            i128::from(u) * i128::from(a) + i128::from(v) * i128::from(b),
            n,
        );
    }
}

/// Apply a 2×2 row transform to two matrices `m1`, `m2` simultaneously.
///
/// Mirrors `modularsnf::echelon::apply_row_2x2_pair`. Used by `lemma_3_1` to
/// keep the transform `U` synchronized with the working matrix `T`.
#[inline]
pub(crate) fn apply_row_2x2_pair(
    m1: &mut [Vec<i64>],
    m2: &mut [Vec<i64>],
    r0: usize,
    r1: usize,
    s: i64,
    t: i64,
    u: i64,
    v: i64,
    n: i64,
) {
    apply_row_2x2(m1, r0, r1, s, t, u, v, n);
    apply_row_2x2(m2, r0, r1, s, t, u, v, n);
}

/// Lemma 3.1: row-echelon form via extended-GCD elimination over Z/N.
///
/// Returns `(U, T, rank)` where `U · A ≡ T (mod n)` and `T` is in row-echelon
/// form. `U` is unimodular over `Z/nZ`.
///
/// Mirrors `modularsnf::echelon::lemma_3_1` (Storjohann §3 Lemma 3.1).
#[must_use]
pub fn lemma_3_1(a: &[Vec<i64>], n: i64) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, usize) {
    let n_rows = a.len();
    let n_cols = a.first().map_or(0, Vec::len);

    debug_assert!(
        a.iter().all(|row| row.len() == n_cols),
        "lemma_3_1: input matrix must be rectangular"
    );

    let mut u = identity(n_rows);
    let mut t: Vec<Vec<i64>> = a.to_vec();

    let mut r = 0usize;

    for k in 0..n_cols {
        if r >= n_rows {
            break;
        }

        for i in (r + 1)..n_rows {
            let a_val = t[r][k];
            let b_val = t[i][k];

            if posmod(b_val, n) == 0 {
                continue;
            }

            // qualified call avoids shadowing local `v` below
            let (_, gex) = crate::snf::zmod::gcdex(a_val, b_val, n);
            let s = gex[0][0];
            let tv = gex[0][1];
            let uv = gex[1][0];
            let v = gex[1][1];
            apply_row_2x2_pair(&mut t, &mut u, r, i, s, tv, uv, v, n);
        }

        if posmod(t[r][k], n) != 0 {
            r += 1;
        }
    }

    (u, t, r)
}

/// Index-1 reduction on the first `k` columns (Storjohann §7.3 step 9).
///
/// Returns `(U, T)` where `U · A ≡ T (mod n)` and the strictly-upper part of
/// the leading `k × k` block has been reduced modulo the diagonal entries.
///
/// Used by Task 14 / `snf::band` for Storjohann Lemma 7.3 band reduction.
///
/// Mirrors `modularsnf::echelon::index1_reduce_on_columns`.
#[must_use]
pub fn index1_reduce_on_columns(
    a: &[Vec<i64>],
    k: usize,
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let n_rows = a.len();
    let n_cols = a.first().map_or(0, Vec::len);

    debug_assert!(
        a.iter().all(|row| row.len() == n_cols),
        "index1_reduce_on_columns: input matrix must be rectangular"
    );

    let mut u = identity(n_rows);
    let mut t: Vec<Vec<i64>> = a.to_vec();

    for j in 1..k {
        let sj = t[j][j];
        for i in 0..j {
            let x = t[i][j];
            if posmod(x, n) == 0 {
                continue;
            }
            // Reduce x to a canonical representative modulo the diagonal annihilator:
            // `b_ass = gcd(sj, n)` is the associate of `sj` in Z/N (Storjohann §1.1's
            // `ass()` function), and `x mod b_ass` gives the unique coset rep of x
            // modulo Z/(sj). When sj is a unit (b_ass = 0 in our convention), no
            // reduction is needed.
            // ring.gcd(sj, 0) = gcd_raw(posmod(sj, n), n) = gcd_three(sj, 0, n).
            let rem = {
                let b_ass = gcd_three(sj, 0, n);
                let x_val = posmod(x, n);
                if b_ass == 0 { x_val } else { x_val % b_ass }
            };
            let diff = posmod_i128(i128::from(x) - i128::from(rem), n);
            // Fallback: when div returns Err, diff is structurally zero in the
            // annihilator quotient — quo = 0 is paper-correct (Storjohann §7.3).
            let quo = div(diff, sj, n).unwrap_or(0);
            let phi = posmod(-quo, n);

            let cols = t[i].len();
            for c in 0..cols {
                t[i][c] = posmod_i128(
                    i128::from(t[i][c]) + i128::from(phi) * i128::from(t[j][c]),
                    n,
                );
                u[i][c] = posmod_i128(
                    i128::from(u[i][c]) + i128::from(phi) * i128::from(u[j][c]),
                    n,
                );
            }
        }
    }

    (u, t)
}
