//! CRT primitives for the multi-prime integer SNF lift.
//!
//! Prime selection ([`select_primes_for_bound`]) and Chinese-Remainder
//! reconstruction ([`crt_reconstruct_signed`]) plus the `i128`-safe modular
//! arithmetic helpers they rely on. The integer-SNF composition that consumes
//! these primitives lives in [`crate::snf::integer`]; the `#35` clarity split
//! preserves the original `snf::crt_lift::*` paths via a re-export shim.

use catgraph::errors::CatgraphError;

/// The 16 largest primes below `2^31`, in descending order.
///
/// Every entry lies in `(2^30, 2^31)`: large enough that each fits in `i64`
/// for per-prime `smith_normal_form` arithmetic, small enough that products
/// up to `k_max = 16` stay bounded. The product of all 16 is ≈ `2^496`, so
/// this table always suffices for the magnitude-homology fixture regime.
///
/// The list is **self-verifying**: the unit test
/// `largest_primes_table_is_correct` re-derives it by scanning odd numbers
/// downward from `2^31 − 1` with trial division and asserts an exact match,
/// so any typo (a composite, a gap, a mis-ordering) fails the build.
const LARGEST_PRIMES_BELOW_2_POW_31: [i64; 16] = [
    2_147_483_647,
    2_147_483_629,
    2_147_483_587,
    2_147_483_579,
    2_147_483_563,
    2_147_483_549,
    2_147_483_543,
    2_147_483_497,
    2_147_483_489,
    2_147_483_477,
    2_147_483_423,
    2_147_483_399,
    2_147_483_353,
    2_147_483_323,
    2_147_483_269,
    2_147_483_249,
];

/// Select primes `p_1 > p_2 > ...` for CRT reconstruction.
///
/// Draws from `LARGEST_PRIMES_BELOW_2_POW_31`, the 16 largest primes in
/// `(2^30, 2^31)`, in descending order. Accumulates their product until it
/// exceeds `2 · bound` and returns the shortest such prefix — i.e., the
/// minimum number of (largest-available) primes whose product exceeds
/// `2 · bound`. Picking the largest primes first minimises the count needed.
///
/// `k_max` caps the number of primes requested; it is additionally capped by
/// the table length (16), so `k_max > 16` is silently clamped to 16.
///
/// # Errors
///
/// - The available primes (`min(k_max, 16)` of them) are insufficient — their
///   product does not exceed `2 · bound`. The full table multiplies to ≈
///   `2^496`, so this only fires for bounds outside the magnitude-homology
///   fixture regime.
/// - The target `2 · bound` overflows `u128`.
pub fn select_primes_for_bound(bound: u128, k_max: usize) -> Result<Vec<i64>, CatgraphError> {
    let target = bound
        .checked_mul(2)
        .ok_or_else(|| CatgraphError::Composition {
            message: "select_primes_for_bound: 2·bound overflows u128".to_string(),
        })?;

    let available = k_max.min(LARGEST_PRIMES_BELOW_2_POW_31.len());
    let mut chosen = Vec::new();
    let mut product: u128 = 1;
    for &p in LARGEST_PRIMES_BELOW_2_POW_31.iter().take(available) {
        chosen.push(p);
        // Each `p < 2^31` fits in u128; `saturating_mul` guards the tail —
        // saturation at `u128::MAX` only occurs once the true product already
        // exceeds any `target ≤ u128::MAX`, so the early return stays correct.
        product = product.saturating_mul(u128::try_from(p).expect("prime fits in u128"));
        if product > target {
            return Ok(chosen);
        }
    }
    Err(CatgraphError::Composition {
        message: format!(
            "select_primes_for_bound: {available} primes (k_max={k_max}, capped at \
             {} available) insufficient for bound={bound} (product={product} < \
             target={target}); the 16 primes below 2^31 multiply to ≈2^496 — a \
             bound exceeding that is outside the magnitude-homology fixture regime",
            LARGEST_PRIMES_BELOW_2_POW_31.len()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Trial-division primality on `i64` (odd `n > 2`). Cheap for the ≤ 16
    /// entries checked here: loops to `⌊√n⌋` only.
    fn is_prime_trial(n: i64) -> bool {
        if n < 2 {
            return false;
        }
        if n % 2 == 0 {
            return n == 2;
        }
        let mut d = 3_i64;
        while d.saturating_mul(d) <= n {
            if n % d == 0 {
                return false;
            }
            d += 2;
        }
        true
    }

    /// Self-verifying: re-derive the 16 largest primes below `2^31` by
    /// scanning odd numbers downward from `2^31 − 1` and comparing to the
    /// baked-in [`LARGEST_PRIMES_BELOW_2_POW_31`]. Any typo — a composite, a
    /// skipped prime, or a mis-ordering — fails this test, keeping the const
    /// correct forever.
    #[test]
    fn largest_primes_table_is_correct() {
        // Structural invariants.
        assert_eq!(LARGEST_PRIMES_BELOW_2_POW_31[0], (1_i64 << 31) - 1);
        for w in LARGEST_PRIMES_BELOW_2_POW_31.windows(2) {
            assert!(w[0] > w[1], "table must be strictly descending: {w:?}");
        }
        for &p in &LARGEST_PRIMES_BELOW_2_POW_31 {
            assert!(
                (1_i64 << 30) < p && p < (1_i64 << 31),
                "prime {p} must lie in (2^30, 2^31)"
            );
            assert!(is_prime_trial(p), "table entry {p} must be prime");
        }

        // Independent re-derivation: the 16 largest primes below 2^31,
        // scanning odd candidates downward.
        let mut derived: Vec<i64> = Vec::with_capacity(16);
        let mut n = (1_i64 << 31) - 1; // 2^31 - 1 is odd
        while derived.len() < 16 {
            if is_prime_trial(n) {
                derived.push(n);
            }
            n -= 2;
        }
        assert_eq!(
            derived,
            LARGEST_PRIMES_BELOW_2_POW_31.to_vec(),
            "re-derived largest-16 primes disagree with the baked-in table"
        );
    }

    #[test]
    fn select_primes_returns_shortest_prefix() {
        // Tiny bound → one prime suffices (2^31−1 > 2·10).
        let one = select_primes_for_bound(10, 16).unwrap();
        assert_eq!(one, vec![LARGEST_PRIMES_BELOW_2_POW_31[0]]);

        // A bound just above p0/2 forces a second prime.
        let p0 = u128::try_from(LARGEST_PRIMES_BELOW_2_POW_31[0]).unwrap();
        let two = select_primes_for_bound(p0, 16).unwrap();
        assert_eq!(
            two,
            vec![
                LARGEST_PRIMES_BELOW_2_POW_31[0],
                LARGEST_PRIMES_BELOW_2_POW_31[1]
            ]
        );
    }

    #[test]
    fn select_primes_k_max_clamped_to_table_len() {
        // k_max above the table length is clamped, not an error, for a bound
        // one prime already covers.
        let chosen = select_primes_for_bound(10, 999).unwrap();
        assert_eq!(chosen.len(), 1);
    }

    #[test]
    fn select_primes_errors_when_insufficient() {
        // k_max = 1 but bound needs more than a single ~2^31 prime.
        let p0 = u128::try_from(LARGEST_PRIMES_BELOW_2_POW_31[0]).unwrap();
        let err = select_primes_for_bound(p0, 1).unwrap_err();
        let CatgraphError::Composition { message } = err else {
            panic!("expected Composition error");
        };
        assert!(message.contains("insufficient"), "got: {message}");
    }
}
