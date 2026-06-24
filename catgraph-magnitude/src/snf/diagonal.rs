//! Diagonal → Smith Normal Form via Storjohann §7.7 D&C merge,
//! and bi-diagonal → Smith Normal Form via Storjohann §7.12's fused
//! 9-step pipeline (crate-internal `bidiagonal_to_smith`).
//!
//! Reference: Storjohann 2000, *Algorithms for Matrix Canonical Forms*,
//! Proposition 7.7 (diagonal-to-Smith), Theorem 7.11 (recursive merge),
//! Lemma 7.10 (2×2 base case), Proposition 7.12 (super-diagonal sweep).
//!
//! License notice: this file is a port of Apache-2.0-licensed code from
//! `events555/modularsnf` at SHA `d62535e`
//! (`crates/modularsnf/src/diagonal.rs` and `crates/modularsnf/src/snf.rs`).
//!
//! Storage is `Vec<Vec<i64>>` (workspace stays ndarray-free per design doc §2.4).
//!
//! # Algorithm overview (diagonal → Smith)
//!
//! A diagonal matrix `diag(s_0, …, s_{m−1})` over `Z/N` is in Smith Normal
//! Form iff `gcd(s_i, N) | gcd(s_{i+1}, N)` for every `i`. Storjohann §7.7
//! enforces this divisibility chain via bottom-up divide-and-conquer:
//!
//! 1. **Pad** to the next power of two with zeros (chain-trivially absorbed
//!    on crop-back).
//! 2. **Bottom-up merge**: start from `n` 1×1 SNF blocks; at each level
//!    pairwise-merge them via `merge_raw` (Theorem 7.11) until a single
//!    block remains.
//! 3. **Crop** to the original `n × n` shape.
//!
//! `merge_raw` recurses into `merge_scalars` (Lemma 7.10) at the leaves;
//! both produce unimodular `U`, `V` such that `U · diag(a, b) · V ≡ S (mod n)`
//! with `S` in Smith Normal Form.
//!
//! # Algorithm overview (bi-diagonal → Smith, Storjohann §7.12)
//!
//! `bidiagonal_to_smith` (crate-internal) takes an upper-bi-diagonal
//! (bandwidth-≤-2) matrix and returns its Smith Normal Form via a fused
//! 9-step pipeline that combines §7.12 super-diagonal sweep with §7.7 /
//! §7.10 / §7.11 SNF and a final §7.3 index-reduction pass:
//!
//! 1. **Step 1 (split with spike)** — sweep super-diagonal column up + down
//!    using `gcdex` 2×2 row ops, producing a "spike" column at row `n1`.
//! 2. **Step 2 (recursive blocks)** — Smith-form principal sub-blocks
//!    `B1` (top-left `n1×n1`) and `B2` (bottom-right `(n−n1−1)×(n−n1−1)`)
//!    by recursing into `bidiagonal_to_smith` itself.
//! 3. **Step 3 (permute)** — permute the spike to last row/column.
//! 4. **Step 4 (Smith on `n−1 × n−1`)** — call [`diagonal_to_smith`] on the
//!    leading principal block (Storjohann §7.7).
//! 5. **Steps 5–8 (gcd chain)** — locate the first zero diagonal entry
//!    `idx_k`, swap columns `idx_k ↔ last_col`, and run
//!    ripple-up + ripple-down loops to enforce the divisibility chain.
//! 6. **Step 9 (index reduction)** — apply a reversal permutation followed
//!    by [`index1_reduce_on_columns`] (Storjohann §7.3) to canonicalize the
//!    final form.
//!
//! [`index1_reduce_on_columns`]: crate::snf::echelon::index1_reduce_on_columns
//!
//! # Helper-organisation note
//!
//! Local helpers (`subblock`, `assign_block`, `matmul_mod_add`,
//! `left_apply_block_pair`, `right_apply_block_pair`, `is_snf_block_zero`, `get_rank`)
//! mirror the same names in `snf::band`. They are deliberately re-ported
//! locally rather than de-duplicated across modules: [`snf::band`]'s helpers
//! are kept private per the Task 14 code-quality review (encapsulation
//! finding), and cross-module helper sharing at this layer would couple two
//! paper-faithful ports that may diverge in future maintenance.
//!
//! [`snf::band`]: crate::snf::band

#![allow(
    clippy::many_single_char_names,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::type_complexity,
    reason = "Storjohann textbook conventions: a, b, n, g, s, t, u, v, q, r, i, j throughout §7.7 + §7.10 + §7.11; the 2-block transform helpers (left_apply_block_pair / right_apply_block_pair) intentionally take the four (u00, u01, u10, u11) sub-blocks atomically to mirror the paper's block-matrix algebra; paper-faithful names like a1/a2/b1/b2/u_loc/v_loc trip similar_names without aiding readability; the (U, V, S) triple of Vec<Vec<i64>> mirrors snf::band / snf::echelon return shapes"
)]

use crate::snf::band::matmul_mod;
use crate::snf::echelon::{apply_row_2x2_pair, identity, index1_reduce_on_columns};
use crate::snf::zmod::{div, gcd_three, gcdex, mul_mod, posmod, posmod_i128};

/// Compute the Smith Normal Form of a diagonal matrix over `Z/nZ`.
///
/// Returns `(U, V, S)` where `U`, `V` are unimodular over `Z/nZ` and
/// `U · D · V ≡ S (mod n)` with `S` diagonal and satisfying the
/// divisibility chain `gcd(s_i, n) | gcd(s_{i+1}, n)`.
///
/// Reference: Storjohann 2000 §7.7 (Proposition 7.7).
///
/// # Algorithm
///
/// 1. **Edge case**: if `D` is `0×0` or `1×1`, return `(I, I, D)` unchanged.
/// 2. **Pad**: extend `D` with zero rows/columns to reach the next power of
///    two `size = next_power_of_two(n)`.
/// 3. **D&C merge**: bottom-up pairwise merge via `smith_from_diagonal_raw`
///    until a single `size × size` SNF block emerges.
/// 4. **Crop**: extract the leading `n × n` block of `(U, V, S)`.
///
/// # Inputs
///
/// - `d`: square diagonal matrix over `Z/nZ` stored as `Vec<Vec<i64>>`. The
///   off-diagonal entries are read but expected to be zero; non-diagonal
///   inputs are not rejected (matches upstream semantics) but the
///   divisibility-chain output is only meaningful when `D` is diagonal.
/// - `n`: modulus.
///
/// # Returns
///
/// `(U, V, S)` all `dim × dim` over `Z/nZ`, with `S` in Smith Normal Form.
///
/// # Panics
///
/// None in normal use. Internal `unwrap_or(0)` swallows the `Err` arm of
/// [`div`] per Storjohann §7.10 (the quotient is structurally zero in the
/// annihilator quotient when `div` would error).
///
/// # Example
///
/// ```
/// use catgraph_magnitude::snf::diagonal::diagonal_to_smith;
/// use catgraph_magnitude::snf::zmod::gcd_raw;
///
/// // diag(6, 4) over Z/12 — gcd(6,12) = 6, gcd(4,12) = 4; SNF chain s_0 | s_1.
/// let n = 12;
/// let d = vec![vec![6, 0], vec![0, 4]];
/// let (_u, _v, s) = diagonal_to_smith(&d, n);
/// // The two diagonals satisfy the SNF divisibility chain modulo n.
/// let g0 = gcd_raw(s[0][0], n);
/// let g1 = gcd_raw(s[1][1], n);
/// assert_eq!(g1 % g0, 0);
/// ```
#[must_use]
pub fn diagonal_to_smith(d: &[Vec<i64>], n: i64) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let dim = d.len();
    if dim <= 1 {
        return (identity(dim), identity(dim), d.to_vec());
    }

    // Pad to the next power of two, run bottom-up D&C merge, crop back.
    // `usize::next_power_of_two` is exact on `usize`; no cast required.
    let size = dim.next_power_of_two();
    let mut pad = vec![vec![0i64; size]; size];
    for i in 0..dim {
        for j in 0..dim {
            pad[i][j] = d[i][j];
        }
    }

    let (u, v, s) = smith_from_diagonal_raw(&pad, n);

    (
        subblock(&u, 0, dim, 0, dim),
        subblock(&v, 0, dim, 0, dim),
        subblock(&s, 0, dim, 0, dim),
    )
}

// ---------------------------------------------------------------------------
// Bottom-up D&C driver (Storjohann §7.7).
// ---------------------------------------------------------------------------

/// Bottom-up iterative diagonal SNF for a power-of-two-sized matrix.
///
/// Mirrors `modularsnf::diagonal::_smith_from_diagonal_raw`. Builds `n` 1×1
/// SNF blocks and pairwise-merges them via [`merge_raw`] until a single
/// `size × size` block remains, doubling the block size at each level.
fn smith_from_diagonal_raw(
    diag: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let size = diag.len();
    if size <= 1 {
        return (identity(size), identity(size), diag.to_vec());
    }

    // Initialize: `size` 1×1 SNF blocks (U = V = I_1, S = [d_ii]).
    let mut blocks: Vec<(Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>)> = (0..size)
        .map(|i| (identity(1), identity(1), vec![vec![diag[i][i]]]))
        .collect();

    let mut bsz = 1usize;
    while bsz < size {
        let mut new_blocks: Vec<(Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>)> =
            Vec::with_capacity(blocks.len() / 2);

        for pair in blocks.chunks_exact(2) {
            let (u1, v1, a) = &pair[0];
            let (u2, v2, b) = &pair[1];

            let (u_merge, v_merge, s_merge) = merge_raw(a, b, n);

            // u_block = block_diag(u1, u2)  →  U_total = u_merge · u_block.
            let mut u_block = vec![vec![0i64; 2 * bsz]; 2 * bsz];
            assign_block(&mut u_block, u1, 0, 0);
            assign_block(&mut u_block, u2, bsz, bsz);
            let u_total = matmul_mod(&u_merge, &u_block, n);

            // v_block = block_diag(v1, v2)  →  V_total = v_block · v_merge.
            let mut v_block = vec![vec![0i64; 2 * bsz]; 2 * bsz];
            assign_block(&mut v_block, v1, 0, 0);
            assign_block(&mut v_block, v2, bsz, bsz);
            let v_total = matmul_mod(&v_block, &v_merge, n);

            new_blocks.push((u_total, v_total, s_merge));
        }

        blocks = new_blocks;
        bsz *= 2;
    }

    blocks.into_iter().next().expect(
        "smith_from_diagonal_raw: chunks_exact(2) over a power-of-two-sized blocks Vec \
         terminates with exactly one element",
    )
}

// ---------------------------------------------------------------------------
// Recursive merge (Storjohann Theorem 7.11).
// ---------------------------------------------------------------------------

/// Recursive merge of two SNF blocks. Returns `(U, V, S)` as `2k × 2k`
/// arrays from `k × k` inputs, with `S` in Smith Normal Form.
///
/// Mirrors `modularsnf::diagonal::_merge_raw` (Storjohann Theorem 7.11).
///
/// The merge proceeds in four pairwise scalar/sub-block steps over the
/// quadrant index map `[A1, A2, B1, B2]`, followed by a rank-permutation
/// step when `B2` retains nonzero rank after merging.
fn merge_raw(
    a_arr: &[Vec<i64>],
    b_arr: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let k = a_arr.len();
    if k == 0 {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    if k == 1 {
        return merge_scalars(a_arr[0][0], b_arr[0][0], n);
    }

    let t = k / 2;
    let nn = 2 * k;

    let mut a1 = subblock(a_arr, 0, t, 0, t);
    let mut a2 = subblock(a_arr, t, k, t, k);
    let mut b1 = subblock(b_arr, 0, t, 0, t);
    let mut b2 = subblock(b_arr, t, k, t, k);

    let mut u_total = identity(nn);
    let mut v_total = identity(nn);

    // Quadrant index map: starting row/col of each of A1, A2, B1, B2 in the
    // 2k × 2k assembled block-diagonal layout.
    let index_map = [0usize, t, k, k + t];

    // Step 1: merge A1 ↔ B1.
    apply_step(
        &mut a1,
        &mut b1,
        0,
        2,
        &mut u_total,
        &mut v_total,
        &index_map,
        t,
        n,
    );
    // Step 2: merge A2 ↔ B2.
    apply_step(
        &mut a2,
        &mut b2,
        1,
        3,
        &mut u_total,
        &mut v_total,
        &index_map,
        t,
        n,
    );
    // Step 3: merge A2 ↔ B1.
    apply_step(
        &mut a2,
        &mut b1,
        1,
        2,
        &mut u_total,
        &mut v_total,
        &index_map,
        t,
        n,
    );
    // Step 4: merge B1 ↔ B2.
    apply_step(
        &mut b1,
        &mut b2,
        2,
        3,
        &mut u_total,
        &mut v_total,
        &index_map,
        t,
        n,
    );

    // Rank-permutation step: when B2 is nonzero and rank(B1) < t, permute
    // the (B1, B2) blocks so the final assembly maintains the Smith
    // divisibility chain.
    //
    // invariant: `is_snf_block_zero` checks the SNF-block leading diagonal (`arr[0][0]`),
    // which is sufficient because subsequent diagonal entries of an SNF block
    // are forced to zero by the divisibility chain. See `is_snf_block_zero` rustdoc.
    if !is_snf_block_zero(&b2, n) {
        let r_b1 = get_rank(&b1, n);
        if r_b1 < t {
            let r_b2 = get_rank(&b2, n);

            // Build the 2t × 2t permutation `p_arr` placing the nonzero
            // ranks of B1 and B2 in the leading positions, then padding
            // with the trailing positions in order.
            let mut p_arr = vec![vec![0i64; 2 * t]; 2 * t];
            for i in 0..r_b1 {
                p_arr[i][i] = 1;
            }
            for i in 0..r_b2 {
                p_arr[r_b1 + i][t + i] = 1;
            }
            let mut cr = r_b1 + r_b2;
            for i in 0..(t - r_b1) {
                p_arr[cr][r_b1 + i] = 1;
                cr += 1;
            }
            for i in 0..(t - r_b2) {
                p_arr[cr][t + r_b2 + i] = 1;
                cr += 1;
            }

            // Lift p_arr to the full 2k × 2k space at the (k, k) corner.
            let mut p_glob = identity(nn);
            assign_block(&mut p_glob, &p_arr, k, k);
            let p_glob_t = transpose(&p_glob);

            u_total = matmul_mod(&p_glob, &u_total, n);
            v_total = matmul_mod(&v_total, &p_glob_t, n);

            // Permute the (B1, B2) sub-blocks to match.
            let mut b_comb = vec![vec![0i64; 2 * t]; 2 * t];
            assign_block(&mut b_comb, &b1, 0, 0);
            assign_block(&mut b_comb, &b2, t, t);
            let p_arr_t = transpose(&p_arr);
            let s_target = matmul_mod(&matmul_mod(&p_arr, &b_comb, n), &p_arr_t, n);
            b1 = subblock(&s_target, 0, t, 0, t);
            b2 = subblock(&s_target, t, 2 * t, t, 2 * t);
        }
    }

    // Assemble S_final = block_diag(A1, A2, B1, B2).
    let mut s_final = vec![vec![0i64; nn]; nn];
    assign_block(&mut s_final, &a1, 0, 0);
    assign_block(&mut s_final, &a2, t, t);
    assign_block(&mut s_final, &b1, k, k);
    assign_block(&mut s_final, &b2, k + t, k + t);

    (u_total, v_total, s_final)
}

/// One step of the four-way merge in [`merge_raw`].
///
/// If both `block1` and `block2` are zero mod `n`, the step is skipped.
/// Otherwise: merge them via [`merge_raw`] recursively, write the merged
/// `S` back into `block1` / `block2`, and accumulate the merge transforms
/// into `u_total` / `v_total` at quadrant indices `idx1` / `idx2`.
fn apply_step(
    block1: &mut Vec<Vec<i64>>,
    block2: &mut Vec<Vec<i64>>,
    idx1: usize,
    idx2: usize,
    u_total: &mut [Vec<i64>],
    v_total: &mut [Vec<i64>],
    index_map: &[usize; 4],
    t: usize,
    n: i64,
) {
    // invariant: `is_snf_block_zero` checks the SNF-block leading diagonal — see its
    // rustdoc. Both blocks zero ⇒ merging is a no-op (identity transforms).
    if is_snf_block_zero(block1, n) && is_snf_block_zero(block2, n) {
        return;
    }

    let (u_loc, v_loc, s_loc) = merge_raw(block1, block2, n);

    *block1 = subblock(&s_loc, 0, t, 0, t);
    *block2 = subblock(&s_loc, t, 2 * t, t, 2 * t);

    let s1 = index_map[idx1];
    let s2 = index_map[idx2];

    let u00 = subblock(&u_loc, 0, t, 0, t);
    let u01 = subblock(&u_loc, 0, t, t, 2 * t);
    let u10 = subblock(&u_loc, t, 2 * t, 0, t);
    let u11 = subblock(&u_loc, t, 2 * t, t, 2 * t);
    left_apply_block_pair(u_total, &u00, &u01, &u10, &u11, s1, s2, t, n);

    let v00 = subblock(&v_loc, 0, t, 0, t);
    let v01 = subblock(&v_loc, 0, t, t, 2 * t);
    let v10 = subblock(&v_loc, t, 2 * t, 0, t);
    let v11 = subblock(&v_loc, t, 2 * t, t, 2 * t);
    right_apply_block_pair(v_total, &v00, &v01, &v10, &v11, s1, s2, t, n);
}

// ---------------------------------------------------------------------------
// Scalar base case (Storjohann Lemma 7.10).
// ---------------------------------------------------------------------------

/// Lemma 7.10 base case: merge two scalar SNF entries `(a)`, `(b)` over
/// `Z/nZ` via gcdex/div. Returns `(U, V, S)` as 2×2 arrays with
/// `U · diag(a, b) · V ≡ S (mod n)`.
///
/// Mirrors `modularsnf::diagonal::_merge_scalars`.
fn merge_scalars(a: i64, b: i64, n: i64) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    // gcdex returns (g, [[s, t], [u_var, v_var]]); use suffixed names to avoid
    // shadowing the conventional `u_var` / `v_var` matrix variable names below.
    let (g, mat) = gcdex(a, b, n);
    let (s, t, u_var, v_var) = (mat[0][0], mat[0][1], mat[1][0], mat[1][1]);

    if g % n == 0 {
        return (identity(2), identity(2), vec![vec![0, 0], vec![0, 0]]);
    }

    let tb = mul_mod(t, b, n);
    // SAFETY (Storjohann §7.10): `div(tb, g, n)` may return `Err` only when
    // the quotient is structurally zero in the annihilator quotient (i.e.
    // `g | tb` is false in `Z/nZ`). In that case the algebraic value is `0`,
    // so `unwrap_or(0)` is paper-faithful, not a silent fault. Mirrors
    // upstream `modularsnf::diagonal::_merge_scalars`.
    let q_raw = div(tb, g, n).unwrap_or(0);
    let q = posmod(-q_raw, n);

    let u_arr = vec![
        vec![posmod(s, n), posmod(t, n)],
        vec![posmod(u_var, n), posmod(v_var, n)],
    ];

    // V is unimodular: det(V) = 1·(1 + q) − q·1 = 1 (mod n). Mirrors the
    // Storjohann §7.10 column-operation form used by upstream `events555/modularsnf`.
    // Confirmed by `verify_snf_invariants` in the test suite (Phase G code-quality
    // reviewer M-1).
    let v_arr = vec![vec![1i64, q], vec![1i64, posmod(1 + q, n)]];

    // S = U · diag(a, b) · V.
    let ab = vec![vec![a, 0i64], vec![0i64, b]];
    let s_arr = matmul_mod(&matmul_mod(&u_arr, &ab, n), &v_arr, n);

    (u_arr, v_arr, s_arr)
}

// ---------------------------------------------------------------------------
// Local helpers (mirrors of snf::band's private helpers; deliberately not
// shared — see module-level rustdoc).
// ---------------------------------------------------------------------------

/// Extract sub-block `m[r0..r1][c0..c1]` as a fresh `Vec<Vec<i64>>`.
fn subblock(m: &[Vec<i64>], r0: usize, r1: usize, c0: usize, c1: usize) -> Vec<Vec<i64>> {
    m[r0..r1].iter().map(|row| row[c0..c1].to_vec()).collect()
}

/// Assign sub-block `block` into `m[r0..][c0..]` in place.
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

/// Element-wise `(a + b) mod n`.
///
/// Mirrors upstream `_matmul_mod_add` (named for parity with the modularsnf
/// helper despite not performing matrix multiplication — see `merge_raw`'s
/// 2-block transform construction `u00 · r1 + u01 · r2`). Called four times
/// per `apply_step` per recursion level; `#[inline]` lets the optimiser fuse
/// the elementwise add into the surrounding `matmul_mod` consumers.
#[inline]
fn matmul_mod_add(a: &[Vec<i64>], b: &[Vec<i64>], n: i64) -> Vec<Vec<i64>> {
    debug_assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b.iter())
        .map(|(arow, brow)| {
            debug_assert_eq!(arow.len(), brow.len());
            arow.iter()
                .zip(brow.iter())
                .map(|(&x, &y)| posmod_i128(i128::from(x) + i128::from(y), n))
                .collect()
        })
        .collect()
}

/// Left-apply a 2-block transform: rows `[s1, s1 + t)` and `[s2, s2 + t)` of `m`.
///
/// `[new_r1; new_r2] = [[u00, u01]; [u10, u11]] · [r1; r2]` (mod n).
fn left_apply_block_pair(
    m: &mut [Vec<i64>],
    u00: &[Vec<i64>],
    u01: &[Vec<i64>],
    u10: &[Vec<i64>],
    u11: &[Vec<i64>],
    s1: usize,
    s2: usize,
    t: usize,
    n: i64,
) {
    let ncols = m[0].len();
    let r1 = subblock(m, s1, s1 + t, 0, ncols);
    let r2 = subblock(m, s2, s2 + t, 0, ncols);
    let new_r1 = matmul_mod_add(&matmul_mod(u00, &r1, n), &matmul_mod(u01, &r2, n), n);
    let new_r2 = matmul_mod_add(&matmul_mod(u10, &r1, n), &matmul_mod(u11, &r2, n), n);
    assign_block(m, &new_r1, s1, 0);
    assign_block(m, &new_r2, s2, 0);
}

/// Right-apply a 2-block transform: cols `[s1, s1 + t)` and `[s2, s2 + t)` of `m`.
///
/// `[new_c1, new_c2] = [c1, c2] · [[v00, v01]; [v10, v11]]` (mod n).
fn right_apply_block_pair(
    m: &mut [Vec<i64>],
    v00: &[Vec<i64>],
    v01: &[Vec<i64>],
    v10: &[Vec<i64>],
    v11: &[Vec<i64>],
    s1: usize,
    s2: usize,
    t: usize,
    n: i64,
) {
    let nrows = m.len();
    let c1 = subblock(m, 0, nrows, s1, s1 + t);
    let c2 = subblock(m, 0, nrows, s2, s2 + t);
    let new_c1 = matmul_mod_add(&matmul_mod(&c1, v00, n), &matmul_mod(&c2, v10, n), n);
    let new_c2 = matmul_mod_add(&matmul_mod(&c1, v01, n), &matmul_mod(&c2, v11, n), n);
    assign_block(m, &new_c1, 0, s1);
    assign_block(m, &new_c2, 0, s2);
}

/// `true` iff the (0, 0) entry of `arr` is `0 (mod n)`. Mirrors upstream
/// `_is_zero`: **an SNF block is zero iff its leading diagonal entry is zero**
/// (subsequent diagonals are forced zero by the divisibility chain).
///
/// **Caller contract.** Only sound on inputs that are SNF blocks (or on
/// rectangular sub-blocks where the SNF divisibility chain has been
/// established). Renamed v0.3.1 from `is_zero` per Phase G rust-dev-v2 M-4 —
/// the prior name was a generic "is the block zero?" check that would
/// silently produce wrong answers if reused on non-SNF blocks.
fn is_snf_block_zero(arr: &[Vec<i64>], n: i64) -> bool {
    if arr.is_empty() || arr[0].is_empty() {
        return true;
    }
    arr[0][0] % n == 0
}

/// Count nonzero diagonal entries `(mod n)` of `arr`. Mirrors upstream
/// `_get_rank`.
fn get_rank(arr: &[Vec<i64>], n: i64) -> usize {
    let dim = arr.len().min(arr.first().map_or(0, Vec::len));
    let mut rank = 0;
    for i in 0..dim {
        if arr[i][i] % n != 0 {
            rank += 1;
        }
    }
    rank
}

// ===========================================================================
// bidiagonal_to_smith (Storjohann §7.12 fused 9-step pipeline).
// ===========================================================================

/// Compute the Smith Normal Form of an upper-bi-diagonal matrix over `Z/nZ`.
///
/// Returns `(U, V, S)` where `U`, `V` are unimodular over `Z/nZ` and
/// `U · T · V ≡ S (mod n)` with `S` in Smith Normal Form.
///
/// Reference: Storjohann 2000 §7.12 (Proposition 7.12) fused with §7.7 +
/// §7.10 + §7.11 (via [`diagonal_to_smith`] in step 4) and §7.3 (via
/// [`index1_reduce_on_columns`] in step 9).
///
/// # Algorithm
///
/// Mirrors `modularsnf::snf::smith_from_upper_2_banded`. Sequentially runs
/// nine sub-steps; see the module-level rustdoc for the high-level walk
/// and the per-`bidiag_step…` rustdocs for paper-line references.
///
/// # Inputs
///
/// - `t`: square upper-bi-diagonal matrix over `Z/nZ` stored as
///   `Vec<Vec<i64>>`. The bandwidth precondition (only `T[i][i]` and
///   `T[i][i+1]` nonzero) is not statically enforced; non-bi-diagonal
///   inputs may give nonsense Smith forms.
/// - `n`: modulus.
///
/// # Returns
///
/// `(U, V, S)` all `dim × dim` over `Z/nZ`, with `S` in Smith Normal Form.
///
/// # Edge case
///
/// If `n_rows <= 1`, returns `(I, I, T)` unchanged.
///
/// [`index1_reduce_on_columns`]: crate::snf::echelon::index1_reduce_on_columns
#[must_use]
pub(crate) fn bidiagonal_to_smith(
    t: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let n_rows = t.len();
    if n_rows <= 1 {
        return (identity(n_rows), identity(n_rows), t.to_vec());
    }

    let (u1, v1, t1, n1) = bidiag_step1_split_with_spike(t, n);
    let (u2, v2, t2) = bidiag_step2_recursive_blocks(&t1, n1, n);
    let (u3, v3, t3) = bidiag_step3_permute(&t2, n1, n);
    let (u4, v4, t4) = bidiag_step4_smith_on_n_minus_1(&t3, n);
    let (u5, v5, t5, k) = bidiag_step5_to_8_gcd_chain(&t4, n);
    let (u6, v6, t6) = bidiag_step9_index_reduction(&t5, k, n);

    // Chain: U_total = U6 · U5 · U4 · U3 · U2 · U1
    let u_total = chain_matmul_left(&[&u6, &u5, &u4, &u3, &u2, &u1], n);
    // Chain: V_total = V1 · V2 · V3 · V4 · V5 · V6
    let v_total = chain_matmul_left(&[&v1, &v2, &v3, &v4, &v5, &v6], n);
    (u_total, v_total, t6)
}

/// Test-only re-export for integration testing of the `pub(crate)` SNF interior.
/// Hidden from public docs; not part of the v0.3.0 stable API.
#[doc(hidden)]
#[must_use]
pub fn bidiagonal_to_smith_for_testing(
    t: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    bidiagonal_to_smith(t, n)
}

/// Left-fold [`matmul_mod`] over `factors`: returns `factors[0] · factors[1] · … · factors[N−1]`.
///
/// Used by [`bidiagonal_to_smith`] to chain the six per-step `U` and `V`
/// transforms. Mirrors the explicit nested `matmul_mod` calls at upstream
/// `snf.rs:85-110`.
#[must_use]
fn chain_matmul_left(factors: &[&[Vec<i64>]], n: i64) -> Vec<Vec<i64>> {
    let (first, rest) = factors
        .split_first()
        .expect("chain_matmul_left: factors must be non-empty");
    rest.iter()
        .fold(first.to_vec(), |acc, f| matmul_mod(&acc, f, n))
}

// ---------------------------------------------------------------------------
// Step 1: split with spike (Storjohann §7.12).
// ---------------------------------------------------------------------------

/// Mirrors `snf.rs::step1_split_with_spike` (lines 116-149).
///
/// Sweeps the super-diagonal column at index `n1 = (n - 1) / 2` upward
/// from `n - 1` to `n1 + 1` and downward from `n1 + 1` to `n - 2`,
/// producing a spike at row `n1`. Only `U` is non-trivial; `V` is identity.
///
/// Returns `(U, V, T_new, n1)`.
#[must_use]
fn bidiag_step1_split_with_spike(
    a: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>, usize) {
    let dim = a.len();
    let n1 = (dim - 1) / 2;

    let mut u = identity(dim);
    let mut t: Vec<Vec<i64>> = a.to_vec();

    // Sweep upward: rows pair (k - 1, k) for k = n - 1 .. n1 + 1.
    //
    // Note on the gcdex 2-tuple unpack used throughout `bidiag_step1_*` +
    // `bidiag_step5_*` + `bidiag_step8_*`: `gcdex(a, b, n) -> (g, [[s, t],
    // [u, v]])` where `s·a + t·b = g` (extended-Bezout) and `u·a + v·b = 0`
    // (orthogonal pair, completing the 2×2 to a unimodular matrix over Z/N).
    // The (s, t, u, v) names are the conventional Storjohann 2000 §7 row-op
    // names; we suffix `tv` and `uv` only on rebinding from the matrix to
    // dodge the local `t`/`u` matrix names in scope.
    for k in ((n1 + 1)..dim).rev() {
        let r0 = k - 1;
        let r1 = k;
        let a_val = t[r0][k];
        let b_val = t[r1][k];
        let (_g, gex) = gcdex(a_val, b_val, n);
        let s = gex[0][0];
        let tv = gex[0][1];
        let uv = gex[1][0];
        let v = gex[1][1];
        // Upstream comment: swapped (uv, v, s, tv) order — eliminates the lower
        // entry in column k by writing the new pivot into row r0 = k - 1.
        apply_row_2x2_pair(&mut t, &mut u, r0, r1, uv, v, s, tv, n);
    }

    // Sweep downward: rows pair (k, k + 1) for k = n1 + 1 .. n - 2.
    for k in (n1 + 1)..(dim - 1) {
        let r0 = k;
        let r1 = k + 1;
        let a_val = t[r0][k];
        let b_val = t[r1][k];
        let (_g, gex) = gcdex(a_val, b_val, n);
        let s = gex[0][0];
        let tv = gex[0][1];
        let uv = gex[1][0];
        let v = gex[1][1];
        apply_row_2x2_pair(&mut t, &mut u, r0, r1, s, tv, uv, v, n);
    }

    let v = identity(dim);
    (u, v, t, n1)
}

// ---------------------------------------------------------------------------
// Step 2: recursive blocks (Storjohann §7.12).
// ---------------------------------------------------------------------------

/// Mirrors `snf.rs::step2_recursive_blocks` (lines 152-178).
///
/// Recursively Smith-forms the principal sub-blocks
/// `B1 = T[..n1, ..n1]` and `B2 = T[n1+1.., n1+1..]` via
/// [`bidiagonal_to_smith`], embeds the local `(U, V)` transforms into the
/// full `dim × dim` block-diagonal layout (rows/cols `n1` left as the
/// spike), and applies them.
#[must_use]
fn bidiag_step2_recursive_blocks(
    a: &[Vec<i64>],
    n1: usize,
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let dim = a.len();

    let b1 = subblock(a, 0, n1, 0, n1);
    let b2 = subblock(a, n1 + 1, dim, n1 + 1, dim);

    let (u1_loc, v1_loc, _s1) = bidiagonal_to_smith(&b1, n);
    let (u2_loc, v2_loc, _s2) = bidiagonal_to_smith(&b2, n);

    let mut u = identity(dim);
    let mut v = identity(dim);

    assign_block(&mut u, &u1_loc, 0, 0);
    assign_block(&mut u, &u2_loc, n1 + 1, n1 + 1);
    assign_block(&mut v, &v1_loc, 0, 0);
    assign_block(&mut v, &v2_loc, n1 + 1, n1 + 1);

    let a_new = matmul_mod(&matmul_mod(&u, a, n), &v, n);
    (u, v, a_new)
}

// ---------------------------------------------------------------------------
// Step 3: permute spike to last position (Storjohann §7.12).
// ---------------------------------------------------------------------------

/// Mirrors `snf.rs::step3_permute` (lines 181-221).
///
/// Builds a permutation `P` mapping `old_i → new_i` where:
/// - `old_i < n1`: identity (`new_i = old_i`).
/// - `old_i == n1`: spike row → last row (`new_i = dim - 1`).
/// - `old_i > n1`: shift down by 1 (`new_i = old_i - 1`).
///
/// Returns `(P, P^T, P · A · P^T)`.
///
/// `P` and `P^T` are permutation matrices (0/1 entries, exactly one 1 per
/// row and per column). `P · A · P^T` is computed by direct rearrangement
/// (faster + exact, sidestepping the modulus): the entry `(i, j)` of the
/// product equals `A[σ⁻¹(i), σ⁻¹(j)]` where `σ` is the permutation encoded
/// by `P`. We then `posmod` the result to canonicalize entries into `[0, n)`.
#[must_use]
fn bidiag_step3_permute(
    a: &[Vec<i64>],
    n1: usize,
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let dim = a.len();

    let mut perm = vec![vec![0i64; dim]; dim];
    // Build P[new_i][old_i] = 1.
    for old_i in 0..dim {
        let new_i = match old_i.cmp(&n1) {
            std::cmp::Ordering::Less => old_i,
            std::cmp::Ordering::Equal => dim - 1,
            std::cmp::Ordering::Greater => old_i - 1,
        };
        perm[new_i][old_i] = 1;
    }
    let perm_t = transpose(&perm);

    // Direct rearrangement of `P · A · P^T`: per-row scan to find the unique
    // `1` in each row of `P` defines the row + column rearrangement. We avoid
    // a modulus-dependent matmul here since permutation matrices have
    // 0/1 entries and the modular reduction would be a no-op except for the
    // wrap on `A`'s entries below.
    let mut a_final = vec![vec![0i64; dim]; dim];
    // For each row `i`, locate `row_src` — the unique `old_i` with
    // P[i][old_i] = 1. Equivalently `i = new_i(row_src)`.
    let mut row_map = vec![0usize; dim];
    for i in 0..dim {
        for k in 0..dim {
            if perm[i][k] != 0 {
                row_map[i] = k;
                break;
            }
        }
    }
    // Permute rows + columns by `row_map`. Step 2's matmul_mod outputs are
    // already canonical in `[0, n)` by construction, so this is a pure
    // rearrangement — no `posmod` required on the hot path. We `debug_assert`
    // canonicalization in dev builds to catch any prior-step regression that
    // would otherwise propagate into step 4 silently.
    for i in 0..dim {
        for j in 0..dim {
            let val = a[row_map[i]][row_map[j]];
            debug_assert!(
                val == posmod(val, n),
                "bidiag_step3_permute: input entry T[{}][{}] = {val} not canonical in [0, n) \
                 for n = {n}; step 2 invariant violated",
                row_map[i],
                row_map[j],
            );
            a_final[i][j] = val;
        }
    }

    (perm, perm_t, a_final)
}

// ---------------------------------------------------------------------------
// Step 4: Smith on (n - 1) × (n - 1) principal block (Storjohann §7.7).
// ---------------------------------------------------------------------------

/// Mirrors `snf.rs::step4_smith_on_n_minus_1` (lines 224-244).
///
/// Diagonalizes the leading `(dim − 1) × (dim − 1)` principal block via
/// [`diagonal_to_smith`] and embeds the resulting `(U, V)` into the
/// full `dim × dim` layout.
#[must_use]
fn bidiag_step4_smith_on_n_minus_1(
    a: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let dim = a.len();

    let b = subblock(a, 0, dim - 1, 0, dim - 1);
    let (u_loc, v_loc, _s_loc) = diagonal_to_smith(&b, n);

    let mut u = identity(dim);
    let mut v = identity(dim);
    assign_block(&mut u, &u_loc, 0, 0);
    assign_block(&mut v, &v_loc, 0, 0);

    let a_new = matmul_mod(&matmul_mod(&u, a, n), &v, n);
    (u, v, a_new)
}

// ---------------------------------------------------------------------------
// Steps 5–8: gcd chain (Storjohann §7.12 + §7.10 mixed).
// ---------------------------------------------------------------------------

/// Mirrors `snf.rs::step5_to_8_gcd_chain` (lines 247-385).
///
/// 1. **Step 5** — locate `idx_k` = first zero diagonal entry; eliminate
///    nonzero entries below `idx_k` in the last column via `gcdex` row ops.
/// 2. **Step 6** — swap columns `idx_k ↔ last_col` in `T` and `V`.
/// 3. **Step 7 (ripple-up)** — for `i = idx_k − 1 … 0`: compute the inline
///    `stab` coefficient `c`, the quotient `q`, then row-add `c·row[i+1]`
///    to `row[i]` and column-add `q·col[i]` to `col[i+1]`.
/// 4. **Step 8 (ripple-down)** — for `i = 0 … idx_k − 1`: if `T[i][col_target]`
///    is nonzero, run a `gcdex` 2×2 column transform on columns `i`, `col_target`.
///
/// Returns `(U, V, T_new, k)` where `k = idx_k + 1` is the rank threshold
/// passed to step 9.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Storjohann §7.12 fuses steps 5-8 deliberately — they share idx_k + t + u + v mutable state through a single loop body; splitting forces tuple-passing of four mut refs plus additional Vec allocations on each step boundary. v0.4.0 forward-look §1.7 documents the deliberate non-split."
)]
fn bidiag_step5_to_8_gcd_chain(
    a: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>, usize) {
    let dim = a.len();

    let mut u = identity(dim);
    let mut v = identity(dim);
    let mut t: Vec<Vec<i64>> = a.to_vec();

    // Find idx_k: first zero diagonal entry; falls back to `dim - 1` if all
    // entries on `[0, dim - 1)` are nonzero.
    let mut idx_k = dim - 1;
    for i in 0..(dim - 1) {
        if posmod(t[i][i], n) == 0 {
            idx_k = i;
            break;
        }
    }

    let last_col = dim - 1;

    // Step 5: eliminate entries below idx_k in the last column.
    for row in (idx_k + 1)..dim {
        let target_val = t[row][last_col];
        if posmod(target_val, n) == 0 {
            continue;
        }
        let pivot_val = t[idx_k][last_col];
        let (_g, gex) = gcdex(pivot_val, target_val, n);
        let s = gex[0][0];
        let tv = gex[0][1];
        let uv = gex[1][0];
        let vv = gex[1][1];
        apply_row_2x2_pair(&mut t, &mut u, idx_k, row, s, tv, uv, vv, n);
    }

    // Step 6: swap columns idx_k and last_col in T and V.
    if idx_k != last_col {
        for row in 0..dim {
            t[row].swap(idx_k, last_col);
            v[row].swap(idx_k, last_col);
        }
    }

    let col_target = idx_k;

    // Step 7: ripple-up loop. Empty if idx_k == 0.
    if idx_k > 0 {
        for i in (0..idx_k).rev() {
            let a_ik = t[i][col_target];
            let a_i1k = t[i + 1][col_target];
            let a_ii = t[i][i];

            // Inline stab(a_ik, a_i1k, a_ii) — mirrors upstream snf.rs:301-313.
            // Find c ∈ [0, n) with gcd(a_ik + c·a_i1k, a_ii) = gcd(a_ik, a_i1k, a_ii).
            // Distinct from `crate::snf::zmod::stab(a, b, n)` (the standard ring
            // stabilizer searches for c with gcd(a + cb, n) = gcd(a, b, n)).
            // Storjohann §7.10 guarantees the loop finds a witness; the `c = 0`
            // initialization is a mathematical no-op fallback (row[i] += 0·row[i+1]).
            //
            // **Performance note (Phase G code-quality M-2):** the search is
            // O(n) worst-case over the rank-recovery prime modulus (up to
            // `2^31 − 1`). In the rank-recovery-prime regime invoked by
            // `magnitude_homology_rank`, the witness is typically found at
            // very small `c` (often `c = 0` when `target_gcd == gcd(a_ik, a_ii)`,
            // a frequent case for sparse `±1`/`0` boundary matrices). Worst-case
            // O(n) is acceptable for v0.3.0 fixture sizes (n ≤ 5,
            // boundary-matrix dimensions ≤ 60); a release-build bounded search
            // is candidate for v0.4.0 forward-look §1.18 if larger fixtures
            // surface. The `debug_assert` below validates the precondition in
            // dev builds.
            let target_gcd = gcd_three(a_ik, gcd_three(a_i1k, a_ii, n), n);
            let mut c: i64 = 0;
            let mut found = false;
            for x in 0..n {
                let candidate =
                    posmod_i128(i128::from(a_ik) + i128::from(x) * i128::from(a_i1k), n);
                let current = gcd_three(candidate, a_ii, n);
                if current == target_gcd {
                    c = x;
                    found = true;
                    break;
                }
            }
            debug_assert!(
                found || target_gcd == gcd_three(a_ik, a_ii, n),
                "bidiag_step5_to_8: stab fallback c = 0 used at i = {i}, but \
                 target_gcd = {target_gcd} ≠ gcd(a_ik, a_ii, n) = {} — \
                 Storjohann §7.10 existence precondition violated",
                gcd_three(a_ik, a_ii, n),
            );

            let s_next = t[i + 1][i + 1];
            let numerator = posmod_i128(i128::from(c) * i128::from(s_next), n);

            // quo: numerator / a_ii in Z/N, modulo the annihilator structure.
            // Mirrors upstream's RingZModN::quo expansion (snf.rs:319-329).
            let a_ii_ass = gcd_three(a_ii, 0, n);
            let num_mod = posmod(numerator, n);
            let rem = if a_ii_ass == 0 {
                num_mod
            } else {
                num_mod % a_ii_ass
            };
            let diff = posmod_i128(i128::from(num_mod) - i128::from(rem), n);
            // SAFETY (Storjohann §7.10): when `div(diff, a_ii, n)` would error,
            // the algebraic value of the quotient is structurally zero in the
            // annihilator quotient. `unwrap_or(0)` is paper-faithful, mirroring
            // upstream `snf.rs:328`.
            let q_raw = div(diff, a_ii, n).unwrap_or(0);
            let q = posmod(-q_raw, n);

            // Row op: row[i] += c · row[i + 1]. Encoded as 2×2 (s, t, u, v) = (1, c, 0, 1).
            apply_row_2x2_pair(&mut t, &mut u, i, i + 1, 1, c, 0, 1, n);

            // Column op: col[i + 1] += q · col[i]. Apply to T and V.
            for row in 0..dim {
                t[row][i + 1] = posmod_i128(
                    i128::from(t[row][i + 1]) + i128::from(q) * i128::from(t[row][i]),
                    n,
                );
                v[row][i + 1] = posmod_i128(
                    i128::from(v[row][i + 1]) + i128::from(q) * i128::from(v[row][i]),
                    n,
                );
            }
        }
    }

    // Step 8: ripple-down loop. Empty if idx_k == 0.
    for i in 0..idx_k {
        let pivot = t[i][i];
        let target = t[i][col_target];

        if posmod(target, n) == 0 {
            continue;
        }

        let (_g, gex) = gcdex(pivot, target, n);
        let s = gex[0][0];
        let tv = gex[0][1];
        let uv = gex[1][0];
        let vv = gex[1][1];

        // Column 2×2 transform on columns i, col_target — apply to T and V.
        for row in 0..dim {
            let ci = t[row][i];
            let ck = t[row][col_target];
            t[row][i] = posmod_i128(
                i128::from(s) * i128::from(ci) + i128::from(tv) * i128::from(ck),
                n,
            );
            t[row][col_target] = posmod_i128(
                i128::from(uv) * i128::from(ci) + i128::from(vv) * i128::from(ck),
                n,
            );

            let vi = v[row][i];
            let vk = v[row][col_target];
            v[row][i] = posmod_i128(
                i128::from(s) * i128::from(vi) + i128::from(tv) * i128::from(vk),
                n,
            );
            v[row][col_target] = posmod_i128(
                i128::from(uv) * i128::from(vi) + i128::from(vv) * i128::from(vk),
                n,
            );
        }
    }

    (u, v, t, idx_k + 1)
}

// ---------------------------------------------------------------------------
// Step 9: index reduction (Storjohann §7.3).
// ---------------------------------------------------------------------------

/// Mirrors `snf.rs::step9_index_reduction` (lines 388-419).
///
/// Builds the reversal permutation `P` on rows `0..=idx_k` (with
/// `idx_k = k − 1`), conjugates `A` by it, runs
/// [`index1_reduce_on_columns`] over the first `k` columns, and reverses
/// back. Only `U` is non-trivial; `V` is identity.
///
/// [`index1_reduce_on_columns`]: crate::snf::echelon::index1_reduce_on_columns
#[must_use]
fn bidiag_step9_index_reduction(
    a: &[Vec<i64>],
    k: usize,
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let dim = a.len();
    let idx_k = k - 1;

    // Build reversal permutation P:
    // - rows 0..=idx_k: reversed (P[i][idx_k - i] = 1).
    // - rows idx_k+1..dim: identity (P[i][i] = 1).
    let mut p = vec![vec![0i64; dim]; dim];
    for i in 0..dim {
        if i <= idx_k {
            p[i][idx_k - i] = 1;
        } else {
            p[i][i] = 1;
        }
    }

    // A_perm = P · A · P (P is its own transpose for this involution).
    let a_perm = matmul_mod(&matmul_mod(&p, a, n), &p, n);

    // Index-1 reduction on the first k columns.
    let (u_red, _a_red) = index1_reduce_on_columns(&a_perm, k, n);

    // U_final = P · U_red · P, V_final = I.
    let u_final = matmul_mod(&matmul_mod(&p, &u_red, n), &p, n);
    let v_final = identity(dim);

    let a_final = matmul_mod(&u_final, a, n);

    (u_final, v_final, a_final)
}
