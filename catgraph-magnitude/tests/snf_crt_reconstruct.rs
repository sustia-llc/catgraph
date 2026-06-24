//! §1.10 CRT reconstruction + sign-symmetric lift.
//! Recovers an integer from residues `r_i = x mod p_i` and lifts to
//! `[-⌊∏p/2⌋, ⌊∏p/2⌋]`.

use catgraph_magnitude::snf::crt_lift::crt_reconstruct_signed;

#[test]
fn crt_simple_two_prime() {
    // x ≡ 2 mod 5, x ≡ 3 mod 7 → x = 17 (sign-symmetric: x ∈ [-17, 17]; pick 17).
    let primes = vec![5, 7];
    let residues = vec![2, 3];
    let result = crt_reconstruct_signed(&primes, &residues).unwrap();
    assert_eq!(result, 17, "CRT(2 mod 5, 3 mod 7) = 17");
}

#[test]
fn crt_negative_via_sign_symmetric() {
    // x ≡ 3 mod 5, x ≡ 4 mod 7 → x = 18; but with sign-symmetric (∏p=35; 35/2=17),
    // x > 17 ⇒ lift to x - 35 = -17.
    let primes = vec![5, 7];
    let residues = vec![3, 4];
    let result = crt_reconstruct_signed(&primes, &residues).unwrap();
    assert_eq!(result, -17, "sign-symmetric CRT(3 mod 5, 4 mod 7) = -17");
}

#[test]
fn crt_zero() {
    let primes = vec![5, 7];
    let residues = vec![0, 0];
    assert_eq!(crt_reconstruct_signed(&primes, &residues).unwrap(), 0);
}

#[test]
fn crt_mismatched_lengths_errors() {
    let primes = vec![5, 7];
    let residues = vec![2, 3, 4];
    assert!(crt_reconstruct_signed(&primes, &residues).is_err());
}

#[test]
fn crt_four_large_primes_near_2_pow_31() {
    // T13 C-1 regression: with k=4 primes near 2^31, the naive triple-product
    // `r · m · m_inv` overflows i128 (silent wrap in release; panic in debug).
    // The mul_mod_i128 helper reduces intermediate products mod P.
    // This is the production-shape fixture the T14 `smith_normal_form_integer`
    // consumer will exercise via `select_primes_for_bound` (k_max=16).
    let primes = vec![
        2_147_483_647_i64,
        2_147_483_629,
        2_147_483_587,
        2_147_483_579,
    ];
    let x_true: i64 = 12_345_678;
    let residues: Vec<i64> = primes.iter().map(|&p| x_true % p).collect();
    assert_eq!(
        crt_reconstruct_signed(&primes, &residues).unwrap(),
        x_true,
        "k=4 large-prime CRT must round-trip; regression for T13 C-1 i128 overflow"
    );
}
