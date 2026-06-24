//! Storjohann Phase 1 — band reduction (Lemmas 7.3, 7.4).
//!
//! Rust port of `modularsnf/src/band.rs` (Apache-2.0 from `events555/modularsnf`
//! SHA `d62535e`).
//!
//! Storage is `Vec<Vec<i64>>` (workspace stays ndarray-free per design doc §2.4).
//!
//! Reduces an upper b-banded matrix to bandwidth ⌊b/2⌋ + 1, iteratively
//! reaching bi-diagonal (b = 2). Two subroutines:
//! - `triang` (Lemma 7.3): clears upper triangle of a sub-block via column ops.
//! - `shift` (Lemma 7.4): chases fill-in down the diagonal.
//!
//! Also exports `matmul_mod` as `pub`; consumed by `snf::diagonal` (Task 16) and
//! integration tests (e.g. `tests/snf_band.rs` unimodular-invariant assertions).

#![allow(
    clippy::many_single_char_names,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::type_complexity,
    reason = "Storjohann textbook conventions: a, b, c, m, n, s, t, u, v, w, k, i, j, l throughout §7.3 + §7.4; the block-application APIs intentionally take rectangular Vec<Vec<i64>> inputs by index where iter()/zip() would obscure the row-column geometry; paper-faithful names like c_mat/c_prime/c1/c2/c2_prime trip similar_names without aiding readability; the multi-Vec<Vec<i64>> return types of shift and band_reduction are paper-driven (B', U_block, V_block / A_reduced, U_band, V_band, b_new) and a type alias would obscure rather than clarify"
)]

use crate::snf::echelon::{identity, lemma_3_1};
use crate::snf::zmod::{posmod, posmod_i128};

/// Modular matrix multiplication: `(a · b) mod n`.
///
/// `a` is `m × k`, `b` is `k × p`, output is `m × p`. All entries are reduced
/// to canonical `[0, n)` form via `posmod_i128`.
///
/// `pub` for reuse by `snf::diagonal` (Task 16) and integration tests; ported
/// here as a ride-along since `band` is the first consumer.
#[must_use]
pub fn matmul_mod(a: &[Vec<i64>], b: &[Vec<i64>], n: i64) -> Vec<Vec<i64>> {
    let m = a.len();
    let k = a.first().map_or(0, Vec::len);
    let p = b.first().map_or(0, Vec::len);
    debug_assert!(b.len() == k, "matmul_mod: inner dimensions must match");
    debug_assert!(
        a.iter().all(|row| row.len() == k),
        "matmul_mod: a not rectangular"
    );
    debug_assert!(
        b.iter().all(|row| row.len() == p),
        "matmul_mod: b not rectangular"
    );
    let mut out = vec![vec![0i64; p]; m];
    for i in 0..m {
        for j in 0..p {
            let mut acc: i128 = 0;
            for l in 0..k {
                acc += i128::from(a[i][l]) * i128::from(b[l][j]);
            }
            out[i][j] = posmod_i128(acc, n);
        }
    }
    out
}

/// Extract sub-block `m[r0..r1][c0..c1]` as a fresh `Vec<Vec<i64>>`.
fn sub_block(m: &[Vec<i64>], r0: usize, r1: usize, c0: usize, c1: usize) -> Vec<Vec<i64>> {
    m[r0..r1].iter().map(|row| row[c0..c1].to_vec()).collect()
}

/// Assign sub-block `block` into `m[r0..][c0..]` in place.
///
/// `block.len()` rows are written to `m[r0..r0 + block.len()]`; each row's
/// `block[i].len()` cols are written to `m[r0 + i][c0..c0 + block[i].len()]`.
fn assign_block(m: &mut [Vec<i64>], block: &[Vec<i64>], r0: usize, c0: usize) {
    for (i, brow) in block.iter().enumerate() {
        for (j, &val) in brow.iter().enumerate() {
            m[r0 + i][c0 + j] = val;
        }
    }
}

/// Transpose a rectangular matrix.
fn transpose(m: &[Vec<i64>]) -> Vec<Vec<i64>> {
    let rows = m.len();
    let cols = m.first().map_or(0, Vec::len);
    let mut t = vec![vec![0i64; rows]; cols];
    for (i, row) in m.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            t[j][i] = val;
        }
    }
    t
}

/// Right-apply a `k × k` `block` to columns `[start, start + k)` of `m`:
/// `m[:, start..start+k] <- (m[:, start..start+k] · block) mod n`.
///
/// `pub(crate)` since `band_reduction` and `shift` both need it. Mirrors
/// `modularsnf::band::right_apply_block`.
pub(crate) fn right_apply_block(m: &mut [Vec<i64>], block: &[Vec<i64>], start: usize, n: i64) {
    let k = block.len();
    // Guard: lemma_3_1 may yield a 0×0 transform on degenerate sub-blocks.
    if k == 0 {
        return;
    }
    let rows = m.len();
    let cols_block = sub_block(m, 0, rows, start, start + k);
    let new_cols = matmul_mod(&cols_block, block, n);
    assign_block(m, &new_cols, 0, start);
}

/// Left-apply a `k × k` `block` to rows `[start, start + k)` of `m`:
/// `m[start..start+k, :] <- (block · m[start..start+k, :]) mod n`.
///
/// `pub(crate)` since `band_reduction` and `shift` both need it. Mirrors
/// `modularsnf::band::left_apply_block`.
pub(crate) fn left_apply_block(m: &mut [Vec<i64>], block: &[Vec<i64>], start: usize, n: i64) {
    let k = block.len();
    // Guard: lemma_3_1 may yield a 0×0 transform on degenerate sub-blocks.
    if k == 0 {
        return;
    }
    let rows_block = sub_block(m, start, start + k, 0, m[0].len());
    let new_rows = matmul_mod(block, &rows_block, n);
    assign_block(m, &new_rows, start, 0);
}

/// Triang step (Storjohann §7.3 Lemma 7.3): triangulate top-right block of an
/// upper-b-banded matrix. Returns `(B', W)` where `W` is the `s2 × s2` right
/// transform that clears the lower-triangular part of the transposed `s1 × s2`
/// block.
///
/// `b_mat` is the `(s1 + s2) × (s1 + s2)` working block; only columns
/// `[s1, s1 + s2)` are touched in `B'`.
fn triang(b_mat: &[Vec<i64>], b: usize, n: i64) -> (Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let s1 = b / 2;
    let s2 = b - 1;
    let n1 = s1 + s2;

    // Extract top-right s1 × s2 block, transpose to s2 × s1.
    let b2 = sub_block(b_mat, 0, s1, s1, s1 + s2);
    let c = transpose(&b2);

    // lemma_3_1 on the transposed block.
    let (u_left, _, _) = lemma_3_1(&c, n);
    let w = transpose(&u_left);

    // B' = B · block_diag(I_s1, W). Only columns s1..n1 are affected.
    let mut b_prime = b_mat.to_vec();
    let cols = sub_block(&b_prime, 0, b_prime.len(), s1, n1);
    let new_cols = matmul_mod(&cols, &w, n);
    assign_block(&mut b_prime, &new_cols, 0, s1);

    (b_prime, w)
}

/// Shift step (Storjohann §7.4 Lemma 7.4): chase fill-in down the diagonal.
/// Returns `(C', U_block, V_block)`.
///
/// `c_mat` is the `2 s2 × 2 s2` working block. `U_block` is the `s2 × s2`
/// left transform produced by `lemma_3_1` on the top-left block; `V_block`
/// is the `s2 × s2` right transform produced by `lemma_3_1` on the
/// transposed `(U_block · top-right)` product.
fn shift(c_mat: &[Vec<i64>], b: usize, n: i64) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let s2 = b - 1;

    // Block partition: C1 = top-left s2 × s2; C2 = top-right s2 × s2.
    let c1 = sub_block(c_mat, 0, s2, 0, s2);
    let c2 = sub_block(c_mat, 0, s2, s2, 2 * s2);

    let (u1, _, _) = lemma_3_1(&c1, n);

    // C2' = U1 · C2.
    let c2_prime = matmul_mod(&u1, &c2, n);

    // Triangulate (C2')^T.
    let c2_prime_t = transpose(&c2_prime);
    let (u2, _, _) = lemma_3_1(&c2_prime_t, n);
    let v_block = transpose(&u2);

    // C' = block_diag(U1, I) · C · block_diag(I, V_block):
    //   - left-apply U1 to top s2 rows;
    //   - right-apply V_block to right s2 columns.
    let mut c_prime = c_mat.to_vec();
    left_apply_block(&mut c_prime, &u1, 0, n);
    right_apply_block(&mut c_prime, &v_block, s2, n);

    (c_prime, u1, v_block)
}

/// Compute the upper bandwidth of `m` modulo `n`: the smallest `w >= 0`
/// such that `m[i][j] ≡ 0 (mod n)` for all `j > i + w − 1`.
///
/// A diagonal-only matrix has bandwidth 1; a full upper-triangular `n × n`
/// matrix has bandwidth `n`; a zero matrix has bandwidth 0.
///
/// Mirrors `modularsnf::band::compute_upper_bandwidth`.
#[must_use]
pub fn compute_upper_bandwidth(m: &[Vec<i64>], n: i64) -> usize {
    let n_rows = m.len();
    let n_cols = m.first().map_or(0, Vec::len);
    let mut max_offset: Option<usize> = None;
    for i in 0..n_rows {
        for j in i..n_cols {
            if posmod(m[i][j], n) != 0 {
                let offset = j - i;
                match max_offset {
                    None => max_offset = Some(offset),
                    Some(cur) if offset > cur => max_offset = Some(offset),
                    _ => {}
                }
            }
        }
    }
    match max_offset {
        None => 0,
        Some(o) => o + 1,
    }
}

/// Full band reduction: reduce upper bandwidth from `b` to ⌊b/2⌋ + 1 in one
/// pass. Returns `(A_reduced, U_band, V_band, b_new)` where
/// `U_band · A · V_band ≡ A_reduced (mod n)` and `b_new = b/2 + 1`.
///
/// Iteration over `band_reduction` reaches bi-diagonal (`b = 2`). When `b ≤ 2`,
/// returns the input unchanged with identity transforms.
///
/// `t_param` is the Storjohann anchor row used to bound the row-stride loop:
/// the effective working width is `n − t_param`, and the padded matrix has
/// size `n + 2b − t_param`.
///
/// Mirrors `modularsnf::band::band_reduction` (Storjohann §7 Phase 1 driver).
///
/// # Preconditions
///
/// - `b >= 1` — bandwidth must be positive (`s2 = b - 1` would underflow `usize`).
/// - `t_param <= 2 * b` — the padding `pad = 2 * b - t_param` must not underflow
///   `usize`. The typical caller passes `t_param = 0` (full-matrix anchor) or a
///   small offset row; both are well within this bound.
/// - `m` is square (`m.len() == m[0].len()`).
///
/// Violations are caught by `debug_assert!` in debug builds; in release builds
/// they manifest as `usize` underflow / arithmetic panics.
///
/// # Algorithm
///
/// Implements Storjohann 2000 §7.3 + Lemma 7.4. The driver alternates two block
/// transforms over a padded `n_big × n_big` working matrix where
/// `n_big = n + 2b − t_param`:
///
/// **Derived quantities.** With input bandwidth `b`:
/// - `s1 = b / 2` — outer stride (row block size for triang).
/// - `s2 = b - 1` — inner stride (block size for shift).
/// - `n1 = s1 + s2` — triang sub-block dimension.
/// - `n2 = 2 * s2` — shift sub-block dimension.
/// - `pad = 2 * b - t_param` — padding rows/cols added to the right + below
///   the input so triang/shift sub-blocks never overflow the matrix bounds.
///
/// **Outer loop (`num_i = ⌈(n − t_param) / s1⌉` iterations).** Iteration `i`
/// triangulates the `n1 × n1` sub-block at row/col `top = i * s1` via Lemma
/// 7.3 (`triang`), producing an `s2 × s2` right transform `W` that clears the
/// lower-triangular part of the transposed top-right block. `W` is right-applied
/// to columns `[top + s1, top + n1)` of the working matrix and accumulated into
/// `v_big`.
///
/// **Inner loop (`num_j` iterations).** For each `i`, `num_j` shift steps
/// chase the resulting fill-in down the diagonal. Iteration `j` operates on
/// the `n2 × n2` sub-block at offset `(i + 1) * s1 + j * s2` via Lemma 7.4
/// (`shift`), producing an `s2 × s2` left transform `U_block` and right
/// transform `V_block`. Both are applied to the working matrix and accumulated
/// into `u_big` / `v_big` respectively.
///
/// **Result.** The top-left `n × n` slice of the padded `n_big × n_big`
/// working matrix is the band-reduced output `A_reduced`; analogous slices of
/// `u_big` / `v_big` form `U_band` / `V_band`. The new bandwidth `b_new =
/// b / 2 + 1` is achieved by construction (one pass at most halves the
/// bandwidth).
#[must_use]
pub fn band_reduction(
    m: &[Vec<i64>],
    b: usize,
    t_param: usize,
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>, usize) {
    debug_assert!(b >= 1, "band_reduction: bandwidth must be >= 1");
    let n_rows = m.len();
    debug_assert!(
        m.iter().all(|row| row.len() == n_rows),
        "band_reduction: input matrix must be square (n × n)"
    );

    if b <= 2 {
        return (m.to_vec(), identity(n_rows), identity(n_rows), b);
    }

    let s1 = b / 2;
    let s2 = b - 1;
    let n1 = s1 + s2;
    let n2 = 2 * s2;

    debug_assert!(
        t_param <= 2 * b,
        "band_reduction: t_param must be <= 2*b for pad calculation"
    );
    let pad = 2 * b - t_param;
    let n_big = n_rows + pad;

    // Padded matrix: top-left n × n holds the input.
    let mut b_mat = vec![vec![0i64; n_big]; n_big];
    assign_block(&mut b_mat, m, 0, 0);

    let mut u_big = identity(n_big);
    let mut v_big = identity(n_big);

    let num_i = if n_rows > t_param {
        (n_rows - t_param).div_ceil(s1)
    } else {
        0
    };

    for i in 0..num_i {
        let top = i * s1;
        if top + n1 > n_big {
            break;
        }

        // Triang step on the (s1 + s2) × (s1 + s2) leading sub-block.
        let b_block = sub_block(&b_mat, top, top + n1, top, top + n1);
        let (_, w) = triang(&b_block, b, n);

        let w_start = top + s1;
        right_apply_block(&mut b_mat, &w, w_start, n);
        right_apply_block(&mut v_big, &w, w_start, n);

        // Shift steps along the diagonal.
        let numer = if n_rows > t_param + (i + 1) * s1 {
            n_rows - t_param - (i + 1) * s1
        } else {
            continue;
        };
        let num_j = numer.div_ceil(s2);

        for j in 0..num_j {
            let offset = (i + 1) * s1 + j * s2;
            if offset + n2 > n_big {
                break;
            }

            let c_block = sub_block(&b_mat, offset, offset + n2, offset, offset + n2);
            let (_, u_block, v_block) = shift(&c_block, b, n);

            left_apply_block(&mut b_mat, &u_block, offset, n);
            right_apply_block(&mut b_mat, &v_block, offset + s2, n);
            left_apply_block(&mut u_big, &u_block, offset, n);
            right_apply_block(&mut v_big, &v_block, offset + s2, n);
        }
    }

    // Extract the n × n top-left block from the padded results.
    let a_reduced = sub_block(&b_mat, 0, n_rows, 0, n_rows);
    let u_band = sub_block(&u_big, 0, n_rows, 0, n_rows);
    let v_band = sub_block(&v_big, 0, n_rows, 0, n_rows);

    // b / 2 + 1 == ⌊b/2⌋ + 1 (Rust usize division is floor for non-negative).
    let b_new = b / 2 + 1;
    (a_reduced, u_band, v_band, b_new)
}
