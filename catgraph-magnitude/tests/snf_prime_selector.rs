//! Multi-prime selector for CRT reconstruction.
//! Selects primes from `(2^30, 2^31)` whose product exceeds `2 · bound`.
//!
//! Runtime: ~9s per test in debug-mode (256MB `Sieve::new(1 << 31)`
//! allocation dominates); ~0.8s in release. Run with `--release` if test
//! runtime becomes a CI bottleneck (the Sieve walk itself is O(N) over
//! ~50M primes; the allocation is the constant-factor cost).

use catgraph_magnitude::snf::crt_lift::select_primes_for_bound;

#[test]
fn select_primes_one_prime_sufficient() {
    let primes = select_primes_for_bound(100, 16).unwrap();
    assert_eq!(primes.len(), 1, "100 fits in a single Mersenne prime");
    // The descending walk deterministically picks
    // the largest prime < 2^31 first, which is Mersenne 2^31 − 1 = 2_147_483_647.
    // Catches a future "ascending" refactor that would silently still pass `> 200`.
    assert_eq!(
        primes[0], 2_147_483_647,
        "first prime is Mersenne 2^31 − 1 (largest prime < 2^31)"
    );
}

#[test]
fn select_primes_multi_required_for_large_bound() {
    // 2^60 product requires ~2 30-bit primes.
    let primes = select_primes_for_bound(1_u128 << 60, 16).unwrap();
    assert!(primes.len() >= 2, "2^60 bound needs >= 2 primes");
    let product: u128 = primes.iter().map(|&p| u128::try_from(p).unwrap()).product();
    assert!(
        product > 2 * (1_u128 << 60),
        "prime product exceeds 2·bound"
    );
}

#[test]
fn select_primes_exceeds_k_max() {
    let result = select_primes_for_bound(u128::MAX / 2, 2);
    assert!(result.is_err(), "k_max=2 insufficient for u128::MAX/2");
}
