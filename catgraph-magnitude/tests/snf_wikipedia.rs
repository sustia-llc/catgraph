//! Wikipedia worked-example cross-validation for `smith_normal_form`.
//!
//! From <https://en.wikipedia.org/wiki/Smith_normal_form#Example>: the 3×3
//! integer matrix
//!
//! ```text
//!   A = [[ 2,  4,  4],
//!        [-6,  6, 12],
//!        [10,  4, 16]]
//! ```
//!
//! has integer SNF `diag(2, 2, 156)` over `Z` (chain: 2 | 2 | 156).
//!
//! Our `smith_normal_form(A, n)` returns the modular SNF over `Z/nZ`. For
//! `n` divisible by `lcm(2, 156) = 156`, the modular invariants must match
//! the integer ones: `gcd(s_0, n) = 2`, `gcd(s_1, n) = 2`, `gcd(s_2, n) = 156`.
//! For `n` coprime to 156, the modular SNF will instead place all non-trivial
//! divisibility into the trailing entry — the Z-vs-Z/N distinction made
//! explicit in `smith_normal_form`'s rustdoc Interpretation section.

#![allow(
    clippy::many_single_char_names,
    reason = "Storjohann §3 textbook conventions: u/v/s for the SNF triple, a for input, n for modulus, g for gcd — match the public API and paper notation"
)]

mod common;

use catgraph_magnitude::snf::smith_normal_form;
use catgraph_magnitude::snf::zmod::gcd_raw;

const WIKIPEDIA_3X3: [[i64; 3]; 3] = [[2, 4, 4], [-6, 6, 12], [10, 4, 16]];
const WIKIPEDIA_CHAIN_N: i64 = 1_872; // 12 · lcm(2, 2, 156). Any n divisible by 156 recovers the integer chain.
const WIKIPEDIA_CHAIN_GCDS: [i64; 3] = [2, 2, 156];

#[test]
fn snf_wikipedia_3x3_matches_textbook_invariants_mod_n_multiple() {
    let a: Vec<Vec<i64>> = WIKIPEDIA_3X3.iter().map(|r| r.to_vec()).collect();
    let n = WIKIPEDIA_CHAIN_N;

    let (u, v, s) = smith_normal_form(&a, n).unwrap();
    common::snf_invariants::verify_snf_invariants(&u, &v, &s, &a, n);

    // Wikipedia's integer SNF is diag(2, 2, 156); over Z/n with n divisible
    // by 156, gcd(s_i, n) must equal the textbook integer invariant.
    for i in 0..3 {
        let g = gcd_raw(s[i][i], n);
        assert_eq!(
            g, WIKIPEDIA_CHAIN_GCDS[i],
            "Wikipedia invariant mismatch at i={i}: got gcd = {g}, expected {}",
            WIKIPEDIA_CHAIN_GCDS[i]
        );
    }
}
