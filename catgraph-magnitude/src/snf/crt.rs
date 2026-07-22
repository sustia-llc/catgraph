//! CRT primitives for the multi-prime integer SNF lift.
//!
//! Prime selection ([`select_primes_for_bound`]) and Chinese-Remainder
//! reconstruction ([`crt_reconstruct_signed`]) plus the `i128`-safe modular
//! arithmetic helpers they rely on. The integer-SNF composition that consumes
//! these primitives lives in [`crate::snf::integer`]; the `#35` clarity split
//! preserves the original `snf::crt_lift::*` paths via a re-export shim.

use catgraph::errors::CatgraphError;

/// Select primes `p_1 < p_2 < ...` for CRT reconstruction.
///
/// All primes are in the range `(2^30, 2^31)` (large enough to fit in `i64`
/// for per-prime `smith_normal_form` arithmetic; small enough that the
/// product up to `k_max = 16` stays bounded). Returns the shortest such
/// sequence — i.e., the minimum number of primes whose product exceeds
/// `2 · bound`.
///
/// The walk runs descending from `2^31` so the largest primes are picked
/// first, minimising the number of primes needed in total. The Mersenne
/// `2^31 − 1 = 2 147 483 647` is the first hit.
///
/// # Errors
///
/// - More than `k_max` primes would be required.
/// - The target `2 · bound` overflows `u128`.
/// - The `(2^30, 2^31)` range is exhausted before the target is reached
///   (vanishingly unlikely for the shipped fixture sizes — there are
///   ~50 million such primes).
///
/// # Panics
///
/// Will not panic on any valid input. The internal `.expect` calls on
/// `i64::try_from(p)` and `u128::try_from(p)` are unreachable by
/// construction: `primal::Sieve::primes_from(1 << 30)` yields `p: usize`
/// values, and the `p >= (1 << 31)` break clause guarantees every collected
/// prime fits in both `i64` and `u128` (whose ranges include all values
/// up to `2^31`).
pub fn select_primes_for_bound(bound: u128, k_max: usize) -> Result<Vec<i64>, CatgraphError> {
    use std::collections::VecDeque;

    use primal::Sieve;

    // Sieve all primes up to 2^31 once (cached for the duration of the call).
    let sieve = Sieve::new(1 << 31);
    let target = bound
        .checked_mul(2)
        .ok_or_else(|| CatgraphError::Composition {
            message: "select_primes_for_bound: 2·bound overflows u128".to_string(),
        })?;

    // `primal::Sieve::primes_from` is forward-only (`SievePrimes` is not
    // `DoubleEndedIterator`). To pick primes descending from `2^31 − 1` —
    // which minimises the count needed to exceed `2 · bound` — collect a
    // bounded sliding window of size `k_max` during the forward walk; at
    // end it holds the `k_max` largest primes in `(2^30, 2^31)` in
    // ascending order. Reversing yields descending. `VecDeque` keeps the
    // pop-front + push-back at O(1) so the overall walk is O(N) in the
    // ~50 million primes of the range. Memory is O(k_max), independent of
    // the range size.
    let mut window: VecDeque<i64> = VecDeque::with_capacity(k_max);
    for p in sieve.primes_from(1 << 30) {
        if p >= (1 << 31) {
            break;
        }
        let p_i64 = i64::try_from(p).expect("prime < 2^31 fits in i64");
        if window.len() == k_max {
            window.pop_front();
        }
        window.push_back(p_i64);
    }

    // Walk the window descending: largest primes first.
    let mut chosen = Vec::new();
    let mut product: u128 = 1;
    for p in window.iter().rev() {
        chosen.push(*p);
        product = product.saturating_mul(u128::try_from(*p).expect("prime fits in u128"));
        if product > target {
            return Ok(chosen);
        }
    }
    // Exhausted the window (i.e. used all k_max primes) without exceeding
    // target.
    Err(CatgraphError::Composition {
        message: format!(
            "select_primes_for_bound: k_max={k_max} primes insufficient for \
             bound={bound} (product={product} < target={target}); \
             escalate k_max or use a sparser fixture"
        ),
    })
}

/// CRT-reconstruct a value `x` from residues `r_i = x mod p_i`, then apply
/// sign-symmetric lift: if `x > ⌊∏p_i / 2⌋`, return `x − ∏p_i`. Returns the
/// integer in `[−⌊∏p_i / 2⌋, ⌊∏p_i / 2⌋]`.
///
/// Standard CRT via Bezout: `x = Σ r_i · (P/p_i) · ((P/p_i)⁻¹ mod p_i) mod P`
/// where `P = ∏ p_i`.
///
/// # Errors
///
/// - `primes.len() != residues.len()`.
/// - Any prime ≤ 0.
/// - Product `∏ p_i` overflows `i128`.
/// - Modular inverse `(P/p_i)⁻¹ mod p_i` is undefined (e.g. non-coprime
///   primes — would be a programmer error since the caller selects
///   distinct primes from `(2^30, 2^31)`).
/// - Final reconstructed value exceeds `i64` range (similarly defensive;
///   consumers operate on bounded magnitude-homology fixtures).
pub fn crt_reconstruct_signed(primes: &[i64], residues: &[i64]) -> Result<i64, CatgraphError> {
    if primes.len() != residues.len() {
        return Err(CatgraphError::Composition {
            message: format!(
                "crt_reconstruct_signed: primes.len()={} != residues.len()={}",
                primes.len(),
                residues.len(),
            ),
        });
    }
    if let Some((idx, &p)) = primes.iter().enumerate().find(|&(_, &p)| p <= 0) {
        return Err(CatgraphError::Composition {
            message: format!("crt_reconstruct_signed: prime {p} at index {idx} is non-positive"),
        });
    }

    let product: i128 = primes.iter().try_fold(1_i128, |acc, &p| {
        acc.checked_mul(i128::from(p))
            .ok_or_else(|| CatgraphError::Composition {
                message: "crt_reconstruct_signed: prime product overflows i128".to_string(),
            })
    })?;

    let mut x: i128 = 0;
    for (&p, &r) in primes.iter().zip(residues.iter()) {
        let p_i128 = i128::from(p);
        let m = product / p_i128;
        let m_inv = mod_inverse_i128(m, p_i128).ok_or_else(|| CatgraphError::Composition {
            message: format!("crt_reconstruct_signed: m_inv mod p={p} undefined"),
        })?;
        let r_i128 = i128::from(r);
        // Compute `(r · m · m_inv) mod product` via i128-safe `mul_mod_i128`.
        // The naive `r * m * m_inv` triple-multiplication overflows i128 for
        // k ≥ 4 primes near 2^31 (silent wrap in release-mode); the helper
        // routes through `num::BigInt` to avoid the overflow path.
        let r_m = mul_mod_i128(r_i128, m, product);
        let r_m_minv = mul_mod_i128(r_m, m_inv, product);
        x = (x + r_m_minv).rem_euclid(product);
    }

    // Sign-symmetric lift. The caller picks primes > 2, so `product`
    // is odd and `half = (product − 1) / 2` (integer division); the strict
    // `x > half` is canonical (no off-by-one on `x == half`).
    let half = product / 2;
    if x > half {
        x -= product;
    }

    // Constrain to i64.
    if x > i128::from(i64::MAX) || x < i128::from(i64::MIN) {
        return Err(CatgraphError::Composition {
            message: format!("crt_reconstruct_signed: result {x} exceeds i64 range"),
        });
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "x verified to fit in i64 range above"
    )]
    Ok(x as i64)
}

/// Extended GCD-based modular inverse on `i128`. Returns `Some(x)` such that
/// `a · x ≡ 1 (mod m)`, or `None` if `gcd(a, m) ≠ 1` or `m == 0`.
///
/// The `m == 0` early return guards against an `a.rem_euclid(m)` panic when
/// a future caller (outside the current `crt_reconstruct_signed` path) calls
/// this directly with a zero modulus. Within `crt_reconstruct_signed` the
/// modulus `m = P / p_i` is always ≥ 1 because the positivity check on
/// primes guarantees `p_i ≥ 1` and `P ≥ p_i`.
fn mod_inverse_i128(a: i128, m: i128) -> Option<i128> {
    if m == 0 {
        return None;
    }
    let (g, x, _) = extended_gcd_i128(a.rem_euclid(m), m);
    if g == 1 { Some(x.rem_euclid(m)) } else { None }
}

/// Extended GCD: returns `(g, x, y)` such that `a · x + b · y = g = gcd(a, b)`.
///
/// Recursive form; recursion depth is Fibonacci-bounded at `O(log min(a, b))`
/// — ≤ ~90 levels for arbitrary `i128` inputs and ≤ ~31 levels for the
/// `crt_reconstruct_signed` caller (primes < 2^31). Well within Rust's
/// default 8 MB stack.
fn extended_gcd_i128(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 {
        (a, 1, 0)
    } else {
        let (g, x1, y1) = extended_gcd_i128(b, a % b);
        (g, y1, x1 - (a / b) * y1)
    }
}

/// Computes `(a · b) mod m` without `i128` overflow. Used by
/// [`crt_reconstruct_signed`] to combine residue × coefficient × inverse
/// triples when individual terms (`m ≈ P/p`, `m_inv < p`, `r < p`) multiply
/// to values exceeding `i128::MAX` (silent wrap in release-mode without
/// this helper; triggers for k ≥ 4 primes from `(2^30, 2^31)`).
///
/// Routes through `num::BigInt` for the multiplication step, then reduces
/// mod `m` before lowering back to `i128`. The cast is safe because
/// `result < m ≤ i128::MAX` by construction.
fn mul_mod_i128(a: i128, b: i128, m: i128) -> i128 {
    use num::BigInt;
    use num::ToPrimitive;
    let result = (BigInt::from(a) * BigInt::from(b)) % BigInt::from(m);
    result
        .to_i128()
        .expect("(a · b) mod m fits in i128 since result is reduced mod m ≤ i128::MAX")
}
