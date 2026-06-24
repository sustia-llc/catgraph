//! Shared SNF correctness helpers for integration tests.
//!
//! `verify_snf_invariants` checks the four Wikipedia PID-SNF properties
//! adapted to `Z/nZ` (Storjohann 2000): `U·A·V ≡ S`, `S` diagonal, divisibility
//! chain on `gcd(s_i, n)`, and unimodularity `gcd(det U, n) = 1` +
//! `gcd(det V, n) = 1`.
//!
//! Determinant computed via recursive minor expansion; capped at `dim ≤ 8`.
//! For larger fixtures (e.g. Phase E acceptance), invariant checking is
//! delegated to the consumer-level acceptance gate (Task 23 numerical vs
//! structural agreement on 5 fixtures).

#![allow(
    dead_code,
    reason = "Helper module shared across integration test targets; cargo test reports per-target dead-code warnings when only some targets call a given helper."
)]
#![allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    reason = "Storjohann §3 textbook conventions: u/v/s for the SNF triple, a for input, n for modulus, m for transform argument; (i, j) index pair simultaneously needed for cell-wise assertions on U·A·V ≡ S."
)]

use catgraph_magnitude::snf::band::matmul_mod;
use catgraph_magnitude::snf::zmod::{gcd_raw, posmod, posmod_i128};

const MAX_DET_DIM: usize = 8;

/// Recursive determinant mod `n` via cofactor expansion along row 0. Capped
/// at `MAX_DET_DIM`; panics for larger inputs (use Gaussian-elimination-based
/// determinant if v0.4.0 raises the cap).
///
/// Internal to integration-test invariant helpers; not a stable API surface.
/// `pub` only to satisfy cargo's integration-test reachability for
/// `verify_snf_invariants` callers across multiple test targets.
pub fn det_mod(m: &[Vec<i64>], n: i64) -> i64 {
    let dim = m.len();
    assert!(
        dim <= MAX_DET_DIM,
        "det_mod: dim {dim} exceeds MAX_DET_DIM = {MAX_DET_DIM}; use Gaussian determinant for larger transforms"
    );
    if dim == 0 {
        return 1;
    }
    if dim == 1 {
        return posmod(m[0][0], n);
    }
    if dim == 2 {
        let val =
            i128::from(m[0][0]) * i128::from(m[1][1]) - i128::from(m[0][1]) * i128::from(m[1][0]);
        return posmod_i128(val, n);
    }
    let mut acc: i128 = 0;
    for j in 0..dim {
        // Build (dim-1) × (dim-1) minor: drop row 0, drop col j.
        let minor: Vec<Vec<i64>> = m[1..]
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter_map(|(c, &v)| if c == j { None } else { Some(v) })
                    .collect()
            })
            .collect();
        let sub = i128::from(det_mod(&minor, n));
        let term = i128::from(m[0][j]) * sub;
        if j % 2 == 0 {
            acc += term;
        } else {
            acc -= term;
        }
    }
    posmod_i128(acc, n)
}

/// Verify the four Wikipedia / Storjohann SNF correctness properties for a
/// returned `(U, V, S)` triple over `Z/nZ`:
///
/// 1. `U · A · V ≡ S (mod n)`.
/// 2. `S` is diagonal (off-diagonal entries are 0 mod n).
/// 3. Divisibility chain: `gcd(s_i, n) | gcd(s_{i+1}, n)` for nonzero leading
///    entries; trailing zeros at the bottom of the chain.
/// 4. **Unimodularity:** `gcd(det U, n) = 1` and `gcd(det V, n) = 1` —
///    equivalent to "U and V are invertible over Z/nZ" (Storjohann 2000).
///
/// Panics on any invariant violation with a descriptive message.
pub fn verify_snf_invariants(
    u: &[Vec<i64>],
    v: &[Vec<i64>],
    s: &[Vec<i64>],
    a: &[Vec<i64>],
    n: i64,
) {
    // (1) Transform identity.
    let ua = matmul_mod(u, a, n);
    let uav = matmul_mod(&ua, v, n);
    for (i, urow) in uav.iter().enumerate() {
        for (j, &val) in urow.iter().enumerate() {
            assert_eq!(val, s[i][j], "U·A·V ≢ S at ({i}, {j})");
        }
    }

    // (2) S diagonal.
    for (i, row) in s.iter().enumerate() {
        for (j, &val) in row.iter().enumerate() {
            if i != j {
                assert_eq!(val % n, 0, "S[{i}][{j}] = {val} ≠ 0 (mod {n})");
            }
        }
    }

    // (3) Divisibility chain on gcd(s_i, n) with trailing zeros. Defense-in-
    // depth: assert all rows of S have equal length so the diagonal-length
    // derivation is meaningful (well-formed S from `smith_normal_form` always
    // satisfies this; a malformed caller-passed S would otherwise silently
    // skip chain checks).
    if let Some(first_row) = s.first() {
        for (i, row) in s.iter().enumerate().skip(1) {
            assert_eq!(
                row.len(),
                first_row.len(),
                "S row {i} length mismatch: expected {}, got {}",
                first_row.len(),
                row.len()
            );
        }
    }
    let r = s.len().min(s.first().map_or(0, Vec::len));
    let mut prev_gcd: Option<i64> = None;
    let mut seen_zero = false;
    for i in 0..r {
        let g = gcd_raw(posmod(s[i][i], n), n);
        // posmod normalizes to [0, n), and gcd_raw(0, n) = n; both forms
        // indicate an S entry that is 0 mod n. The g == 0 arm is dead-but-
        // defensive — kept for symmetry with future variants of `posmod`.
        if g == 0 || g == n {
            seen_zero = true;
            continue;
        }
        assert!(
            !seen_zero,
            "S[{i}][{i}] gcd-with-n = {g} appears after a zero entry — chain violation"
        );
        if let Some(prev) = prev_gcd {
            assert_eq!(
                g % prev,
                0,
                "chain violation at i={i}: gcd={g}, prev={prev}"
            );
        }
        prev_gcd = Some(g);
    }

    // (4) Unimodularity.
    let du = det_mod(u, n);
    assert_eq!(
        gcd_raw(du, n),
        1,
        "U not unimodular mod {n}: gcd(det U = {du}, {n}) ≠ 1"
    );
    let dv = det_mod(v, n);
    assert_eq!(
        gcd_raw(dv, n),
        1,
        "V not unimodular mod {n}: gcd(det V = {dv}, {n}) ≠ 1"
    );
}
