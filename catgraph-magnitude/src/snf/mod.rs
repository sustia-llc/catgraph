//! Smith Normal Form over Z/N (Storjohann 2000) — Rust port over `MatR<Q>`.
//!
//! Algorithmic reference: `events555/modularsnf` at SHA `d62535e`. Algorithm
//! doc at upstream `docs/algorithm.md`. License Apache-2.0.
//!
//! Public surface:
//! - [`phase_1_to_bidiagonal`] — compose echelon + iterated band reduction
//!   into a single bi-diagonalisation pass.

#![allow(
    clippy::type_complexity,
    clippy::many_single_char_names,
    reason = "the (U, T, V) triple of Vec<Vec<i64>> mirrors the Storjohann §7 paper-faithful return shape used throughout snf::band and snf::echelon; a type alias would hide the algebraic geometry rather than clarify it. Single-char names (a, n, t, u, v, s) match Storjohann §1.1 + §3 textbook conventions and the (U, V, S) tuple identity in the public surface — module-level allow mirrors snf::band, snf::diagonal, snf::echelon, snf::zmod"
)]

pub mod band;
pub mod crt;
pub mod crt_lift;
pub mod diagonal;
pub mod echelon;
pub mod integer;
pub mod zmod;

pub use integer::smith_normal_form_integer;

use catgraph::errors::CatgraphError;
use catgraph_applied::mat::MatR;

use crate::chain_complex::IntegerLikeRig;
use crate::snf::band::{band_reduction, compute_upper_bandwidth, matmul_mod};
use crate::snf::diagonal::bidiagonal_to_smith;
use crate::snf::echelon::{identity, lemma_3_1};
use crate::snf::zmod::posmod;

/// SNF Phase 1: reduce `A` over Z/N to upper bi-diagonal form. Returns
/// `(U, T, V)` with `U @ A @ V ≡ T (mod n)` and `T` upper bi-diagonal
/// (bandwidth ≤ 2: only `T[i][i]` and `T[i][i+1]` may be nonzero).
///
/// Composes Lemma 3.1 row echelon ([`echelon::lemma_3_1`]) with iterated
/// Lemma 7.3 / 7.4 band reduction ([`band::band_reduction`]) per
/// `modularsnf::snf::smith_square` Phase 1 (lines 28-46 of the upstream
/// reference at SHA `d62535e`).
///
/// # Inputs
///
/// - `a`: square matrix over Z/N stored as `Vec<Vec<i64>>` (size
///   `n_rows × n_rows`; the row-count `n_rows` is named separately to
///   disambiguate from the modulus parameter `n`).
/// - `n`: modulus.
///
/// # Returns
///
/// `(U, T, V)` all `n_rows × n_rows` over Z/N, with `T` upper bi-diagonal.
///
/// # Algorithm
///
/// 1. Row echelon: `(U_ech, T_0, _) = lemma_3_1(A, n)`.
/// 2. Iterate band reduction while `b > 2`:
///    `(T_{k+1}, U_step, V_step, b_{k+1}) = band_reduction(T_k, b_k, 0, n)`.
///    Accumulate `U_band ← U_step · U_band` (left-multiply) and
///    `V_band ← V_band · V_step` (right-multiply).
/// 3. Final: `U = U_band · U_ech`, `T = T_final`, `V = V_band`.
///
/// # Edge case
///
/// If `n_rows == 0`, returns `(empty, empty, empty)` (mirrors upstream).
///
/// # Panics
///
/// None in practice. The inner [`band_reduction`] preconditions
/// (`b >= 1`, `t_param <= 2 * b`) are statically satisfied by this
/// call shape: the `while b > 2` loop guard ensures `b >= 1` at every
/// `band_reduction` call site, and `t_param = 0` is hard-coded.
///
/// [`echelon::lemma_3_1`]: crate::snf::echelon::lemma_3_1
/// [`band::band_reduction`]: crate::snf::band::band_reduction
/// [`band_reduction`]: crate::snf::band::band_reduction
#[must_use]
pub fn phase_1_to_bidiagonal(
    a: &[Vec<i64>],
    n: i64,
) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let n_rows = a.len();

    if n_rows == 0 {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    // Step 1: row echelon (Lemma 3.1). Discard the rank — Phase 1 only
    // needs U_ech and T_0.
    let (u_ech, t0, _rank) = lemma_3_1(a, n);

    // Step 2: iterated band reduction (Lemmas 7.3 / 7.4) until b ≤ 2.
    // b_mat starts as T_0 (echelon output); is mutated in-place by the loop;
    // emerges as the bi-diagonal T returned in Step 3.
    let mut b_mat = t0;
    let mut b = compute_upper_bandwidth(&b_mat, n);

    let mut u_band_total = identity(n_rows);
    let mut v_band_total = identity(n_rows);

    while b > 2 {
        let (b_new_mat, u_step, v_step, b_new) = band_reduction(&b_mat, b, 0, n);
        b_mat = b_new_mat;
        // Left-multiply U: U_band_total ← U_step · U_band_total.
        u_band_total = matmul_mod(&u_step, &u_band_total, n);
        // Right-multiply V: V_band_total ← V_band_total · V_step.
        v_band_total = matmul_mod(&v_band_total, &v_step, n);
        b = b_new;
    }

    // Step 3: combine echelon + band transforms into the Phase 1 outputs.
    let u_total = matmul_mod(&u_band_total, &u_ech, n);
    (u_total, b_mat, v_band_total)
}

/// Top-level Smith Normal Form over `Z/nZ`.
///
/// Returns `(U, V, S)` such that `U · A · V ≡ S (mod n)`, with `S` diagonal in
/// Smith Normal Form: the divisibility chain
/// `gcd(s_0, n) | gcd(s_1, n) | … | gcd(s_{k-1}, n)` holds on the principal
/// diagonal entries.
///
/// # Interpretation
///
/// **This is the modular SNF over `Z/nZ`, not the classical integer SNF over
/// `Z`.** The two coincide iff the modulus `n` exceeds a Hadamard bound on
/// `A` (so the integer invariant factors lift faithfully out of `Z/nZ`); for
/// smaller `n` the modular result is a paper-faithful Storjohann 2000 SNF
/// over the quotient ring `Z/nZ`, which is the correct object for:
///
/// - rank computation modulo `n` (caller picks `n` prime → `rank_p`),
/// - magnitude-homology rank recovery (`magnitude_homology_rank`
///   uses single-prime SNF + majority vote across primes),
/// - any consumer that operates internally over `Z/nZ`.
///
/// Implications:
///
/// - **`U`, `V` are unimodular over `Z/nZ`**, meaning `gcd(det U, n) = 1` and
///   `gcd(det V, n) = 1`. Over `Z` they may have determinant outside `±1`.
/// - **The chain is `gcd(s_i, n) | gcd(s_{i+1}, n)`**, not the integer chain
///   `s_i | s_{i+1}`. The two agree under the lift condition above.
/// - **Diagonal entries are canonicalised into `[0, n)`** via `posmod`; over
///   `Z` the standard normalisation is the non-negative associate.
///
/// Consumers wanting the integer SNF (full invariant-factor structure, not
/// just rank) need either a Hadamard-bound modulus or multi-prime CRT
/// reconstruction; both are deferred (#35). For integer rank recovery, the
/// `chain_complex::magnitude_homology_rank` consumer uses single-prime SNF
/// over a Mersenne prime + majority vote across primes.
///
/// # Algorithm
///
/// Composes the full Storjohann 2000 pipeline:
///
/// 1. **Phase 1** ([`phase_1_to_bidiagonal`]) — Lemma 3.1 row echelon
///    composed with iterated Lemma 7.3 / 7.4 band reduction down to upper
///    bi-diagonal form (bandwidth ≤ 2).
/// 2. **Phase 2** ([`crate::snf::diagonal::bidiagonal_to_smith`]) — fused
///    9-step pipeline: Storjohann §7.12 split-with-spike, recursive block
///    diagonalisation, permutation, §7.7 / Thm 7.11 / Lemma 7.10
///    diagonal-to-Smith on the leading `(n-1)×(n-1)` block, gcd-chain
///    enforcement, and §3 index-1 reduction.
/// 3. **Rectangular handling** — non-square inputs are zero-padded to a
///    `max(rows, cols)`-square at entry and the resulting `(U_pad, V_pad,
///    S_pad)` cropped to `(rows×rows, cols×cols, rows×cols)` on exit
///    (mirrors `modularsnf::smith_normal_form` lib.rs lines 22-39 at SHA
///    `d62535e`).
///
/// # Inputs
///
/// - `a`: rectangular matrix over `Z/nZ` stored as `Vec<Vec<i64>>`. Shape
///   is `rows × cols` with `rows = a.len()` and `cols = a[0].len()` (when
///   non-empty); all rows must have equal length (validated).
/// - `n`: modulus, strictly positive.
///
/// # Returns
///
/// `Ok((U, V, S))` with shapes `(rows × rows, cols × cols, rows × cols)`
/// over `Z/nZ` and `S` in Smith Normal Form.
///
/// # Edge case
///
/// If `rows == 0` or `cols == 0`, returns `Ok((I_rows, I_cols, A.clone()))`
/// — the empty SNF is trivially valid (mirrors upstream).
///
/// # Errors
///
/// Returns `Err(CatgraphError::Composition { message })` when:
///
/// - `n <= 0` — modulus must be strictly positive.
/// - `a` has rows of differing lengths — input must be rectangular.
///
/// # Examples
///
/// ```
/// use catgraph_magnitude::snf::band::matmul_mod;
/// use catgraph_magnitude::snf::smith_normal_form;
///
/// // Storjohann §3 example from the modularsnf README Quick Start.
/// let a = vec![vec![2, 4, 0], vec![6, 8, 3], vec![0, 3, 9]];
/// let n = 36;
/// let (u, v, s) = smith_normal_form(&a, n).unwrap();
///
/// // U · A · V ≡ S (mod n).
/// let ua = matmul_mod(&u, &a, n);
/// let uav = matmul_mod(&ua, &v, n);
/// assert_eq!(uav, s);
/// ```
///
/// [`phase_1_to_bidiagonal`]: crate::snf::phase_1_to_bidiagonal
/// [`crate::snf::diagonal::bidiagonal_to_smith`]: crate::snf::diagonal
pub fn smith_normal_form(
    a: &[Vec<i64>],
    n: i64,
) -> Result<(Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>), CatgraphError> {
    // Step 1: validate modulus.
    if n <= 0 {
        return Err(CatgraphError::Composition {
            message: format!("smith_normal_form requires n > 0; got n = {n}"),
        });
    }

    let rows = a.len();
    let cols = if rows == 0 { 0 } else { a[0].len() };

    // Step 2: validate rectangular shape.
    for (i, row) in a.iter().enumerate() {
        if row.len() != cols {
            return Err(CatgraphError::Composition {
                message: format!(
                    "smith_normal_form requires rectangular input; row {i} has \
                     length {} but row 0 has length {cols}",
                    row.len()
                ),
            });
        }
    }

    // Step 3: empty edge case — empty SNF is trivially valid.
    if rows == 0 || cols == 0 {
        return Ok((identity(rows), identity(cols), a.to_vec()));
    }

    // Step 4: pad to square s_dim = max(rows, cols), normalising entries
    // through posmod to canonicalise representatives.
    let s_dim = rows.max(cols);
    let mut a_pad = vec![vec![0i64; s_dim]; s_dim];
    for i in 0..rows {
        for j in 0..cols {
            a_pad[i][j] = posmod(a[i][j], n);
        }
    }

    // Step 5: Phase 1 — bi-diagonalise A_pad.
    let (u_p1, t, v_p1) = phase_1_to_bidiagonal(&a_pad, n);

    // Step 6: Phase 2 — bi-diagonal → Smith.
    let (u_p2, v_p2, s_pad) = bidiagonal_to_smith(&t, n);

    // Step 7: compose transforms.
    //   U_combined = U_p2 · U_p1   (left transforms applied left-to-right
    //                                in execution order).
    //   V_combined = V_p1 · V_p2   (right transforms applied right-to-left
    //                                in execution order).
    let u_combined = matmul_mod(&u_p2, &u_p1, n);
    let v_combined = matmul_mod(&v_p1, &v_p2, n);

    // Step 8: crop back to (rows×rows, cols×cols, rows×cols).
    let u: Vec<Vec<i64>> = u_combined
        .iter()
        .take(rows)
        .map(|row| row.iter().take(rows).copied().collect())
        .collect();
    let v: Vec<Vec<i64>> = v_combined
        .iter()
        .take(cols)
        .map(|row| row.iter().take(cols).copied().collect())
        .collect();
    let s: Vec<Vec<i64>> = s_pad
        .iter()
        .take(rows)
        .map(|row| row.iter().take(cols).copied().collect())
        .collect();

    Ok((u, v, s))
}

/// Round-trip wrapper around [`smith_normal_form`] for `MatR<R>` inputs.
///
/// `MatR<R>` → `Vec<Vec<i64>>` (via [`IntegerLikeRig::to_i64`]) →
/// [`smith_normal_form`] → `MatR<R>` (via `R::from(i64)`).
///
/// Useful when a downstream consumer holds matrices in `MatR<R>` form
/// (e.g. `catgraph-magnitude` boundary matrices over `F64Rig` or
/// [`Z`](catgraph_applied::z::Z)) and wants SNF without dropping to the
/// raw `Vec<Vec<i64>>` backend.
///
/// Returns `(U, V, S)` matching the [`smith_normal_form`] convention:
/// `U · A · V ≡ S (mod n)` with `S` in Smith Normal Form over `Z/nZ`.
///
/// # Caveats
///
/// Same modular-vs-integer caveats as [`smith_normal_form`] apply: this
/// is the modular SNF over `Z/nZ`, not the classical integer SNF over
/// `Z`. The two coincide iff `n` exceeds a Hadamard bound on the input;
/// see [`smith_normal_form`] docs for the full discussion.
///
/// # Errors
///
/// - Any entry of `m` exceeds `i64` range (propagated from
///   [`IntegerLikeRig::to_i64`]).
/// - Modulus `n` exceeds `i64` range (same).
/// - Any [`smith_normal_form`] error (non-positive modulus, etc.).
/// - Internal [`MatR::new`] shape mismatch (should not occur in practice;
///   the shapes are propagated from [`smith_normal_form`]'s contract).
///
/// # References
///
/// Substrate for the multi-prime CRT integer SNF lift.
pub fn smith_normal_form_matr<R>(
    m: &MatR<R>,
    n: &R,
) -> Result<(MatR<R>, MatR<R>, MatR<R>), CatgraphError>
where
    R: IntegerLikeRig,
{
    let rows = m.rows();
    let cols = m.cols();
    let mut entries_i64: Vec<Vec<i64>> = Vec::with_capacity(rows);
    for row in m.entries() {
        let mut row_i64 = Vec::with_capacity(cols);
        for x in row {
            row_i64.push(x.to_i64()?);
        }
        entries_i64.push(row_i64);
    }
    let n_i64 = n.to_i64()?;
    let (u, v, s) = smith_normal_form(&entries_i64, n_i64)?;

    let lift = |flat: Vec<Vec<i64>>, r: usize, c: usize| -> Result<MatR<R>, CatgraphError> {
        let mut lifted: Vec<Vec<R>> = Vec::with_capacity(r);
        for row in flat {
            let mut lifted_row = Vec::with_capacity(c);
            lifted_row.extend(row.into_iter().map(R::from));
            lifted.push(lifted_row);
        }
        MatR::<R>::new(r, c, lifted)
    };
    Ok((
        lift(u, rows, rows)?,
        lift(v, cols, cols)?,
        lift(s, rows, cols)?,
    ))
}
