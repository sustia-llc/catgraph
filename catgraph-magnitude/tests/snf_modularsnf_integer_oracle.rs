//! Dev-only cross-validation of cg-magnitude's mod-p SNF backend against
//! `events555/modularsnf` (Rust + ndarray reference implementation,
//! Apache-2.0, SHA `d62535e`). Gated by the `modularsnf-oracle` feature.
//!
//! # Cross-validation strategy: rank-mod-p, NOT entry-by-entry
//!
//! `modularsnf` exposes `smith_normal_form(a: &Array2<i64>, modulus: i64)`
//! returning `(U, V, S)` over `Z/N`. cg-magnitude exposes a structurally
//! identical `snf::smith_normal_form(a: &[Vec<i64>], n: i64)` over `Z/n`.
//! Both crates compute SNF modulo the same prime, but each impl picks its
//! own units-of-`Z/p` factorisation — so the diagonal entries `s[i][i]`
//! will not match entry-by-entry between the two backends even on the
//! same matrix. What they MUST agree on is the **rank**, i.e. the count
//! of diagonal entries coprime to `p` (equivalently, nonzero mod `p`).
//!
//! Rank is the canonical Z/p invariant: it's the dimension of the image,
//! which is impl-independent. This is the cross-validation surface that
//! makes mathematical sense for cg-magnitude's `smith_normal_form_integer`
//! pipeline — the per-prime SNF rank is what feeds into the
//! 2-prime cross-check used to filter "good primes" in
//! `smith_normal_form_integer` (see `crt_lift.rs` step 4).
//!
//! Cross-validating the full integer SNF (`smith_normal_form_integer`)
//! against modularsnf would require either (a) re-running it with
//! modularsnf as the per-prime SNF backend and comparing to cg-magnitude's
//! own backend or (b) a third-party integer-SNF reference. Both are
//! deferred as out-of-scope here.
//!
//! To run:
//!
//! ```text
//! cargo test -p catgraph-magnitude --features modularsnf-oracle
//! ```

#![cfg(feature = "modularsnf-oracle")]

use catgraph_magnitude::snf::smith_normal_form as cg_snf_mod;
use ndarray::Array2;
use proptest::prelude::*;

/// Mersenne prime `2^31 − 1`. Same primary used by
/// `crt_lift::select_primes_for_bound` and `magnitude_homology_rank`'s
/// 2-prime cross-check, so cross-validation at this `p` exercises the
/// canonical hot path.
const ORACLE_PRIME: i64 = 2_147_483_647;

fn vec_to_array2(a: &[Vec<i64>]) -> Array2<i64> {
    let rows = a.len();
    let cols = if rows == 0 { 0 } else { a[0].len() };
    Array2::from_shape_fn((rows, cols), |(i, j)| a[i][j])
}

fn rank_mod_p_vec(s: &[Vec<i64>], p: i64, dim_min: usize) -> usize {
    (0..dim_min)
        .filter(|&i| s[i][i] != 0 && s[i][i].rem_euclid(p) != 0)
        .count()
}

fn rank_mod_p_array(s: &Array2<i64>, p: i64, dim_min: usize) -> usize {
    (0..dim_min)
        .filter(|&i| s[[i, i]] != 0 && s[[i, i]].rem_euclid(p) != 0)
        .count()
}

proptest! {
    #[test]
    fn snf_mod_p_rank_agrees_with_modularsnf_2x2(
        a in proptest::collection::vec(
            proptest::collection::vec(-10_i64..=10, 2..=2),
            2..=2,
        ),
    ) {
        // cg-magnitude path: vec-of-vec, in-place modular SNF.
        let (_u_us, _v_us, s_us) = cg_snf_mod(&a, ORACLE_PRIME).unwrap();

        // modularsnf path: ndarray Array2, padded-to-square modular SNF.
        let array_a = vec_to_array2(&a);
        let (_u_mod, _v_mod, s_mod) =
            modularsnf::smith_normal_form(&array_a, ORACLE_PRIME).unwrap();

        let dim_min = a.len().min(a[0].len());
        let rank_us = rank_mod_p_vec(&s_us, ORACLE_PRIME, dim_min);
        let rank_mod = rank_mod_p_array(&s_mod, ORACLE_PRIME, dim_min);

        prop_assert_eq!(
            rank_us, rank_mod,
            "cg-magnitude and modularsnf must agree on rank mod p={} \
             on input {:?}: cg-magnitude rank = {}, modularsnf rank = {}",
            ORACLE_PRIME, a, rank_us, rank_mod
        );
    }

    // Follow-up: extend the proptest grid from `n = 2` to `n ∈ {2, 3, 4}`.
    // n=4 is the smallest fixture where the chain-rebalance interactions at
    // 4×4 scale (and non-trivial rank-recovery beyond rank ∈ {0, 1, 2}) get
    // exercised.

    #[test]
    fn snf_mod_p_rank_agrees_with_modularsnf_3x3(
        a in proptest::collection::vec(
            proptest::collection::vec(-10_i64..=10, 3..=3),
            3..=3,
        ),
    ) {
        let (_u_us, _v_us, s_us) = cg_snf_mod(&a, ORACLE_PRIME).unwrap();
        let array_a = vec_to_array2(&a);
        let (_u_mod, _v_mod, s_mod) =
            modularsnf::smith_normal_form(&array_a, ORACLE_PRIME).unwrap();

        let dim_min = a.len().min(a[0].len());
        let rank_us = rank_mod_p_vec(&s_us, ORACLE_PRIME, dim_min);
        let rank_mod = rank_mod_p_array(&s_mod, ORACLE_PRIME, dim_min);

        prop_assert_eq!(
            rank_us, rank_mod,
            "cg-magnitude and modularsnf must agree on rank mod p={} \
             on input {:?}: cg-magnitude rank = {}, modularsnf rank = {}",
            ORACLE_PRIME, a, rank_us, rank_mod
        );
    }

    #[test]
    fn snf_mod_p_rank_agrees_with_modularsnf_4x4(
        a in proptest::collection::vec(
            proptest::collection::vec(-10_i64..=10, 4..=4),
            4..=4,
        ),
    ) {
        let (_u_us, _v_us, s_us) = cg_snf_mod(&a, ORACLE_PRIME).unwrap();
        let array_a = vec_to_array2(&a);
        let (_u_mod, _v_mod, s_mod) =
            modularsnf::smith_normal_form(&array_a, ORACLE_PRIME).unwrap();

        let dim_min = a.len().min(a[0].len());
        let rank_us = rank_mod_p_vec(&s_us, ORACLE_PRIME, dim_min);
        let rank_mod = rank_mod_p_array(&s_mod, ORACLE_PRIME, dim_min);

        prop_assert_eq!(
            rank_us, rank_mod,
            "cg-magnitude and modularsnf must agree on rank mod p={} \
             on input {:?}: cg-magnitude rank = {}, modularsnf rank = {}",
            ORACLE_PRIME, a, rank_us, rank_mod
        );
    }
}
