//! Multi-prime CRT integer SNF lift.
//!
//! Algorithm overview:
//! 1. Compute Hadamard bound `H(A) = ∏_i ||a_i||_2`.
//! 2. Select primes `p_1 < p_2 < ...` such that `∏ p_i > 2 H(A)`.
//! 3. For each prime: [`smith_normal_form`](super::smith_normal_form)`(A, p_i)`.
//! 4. Good-prime filter; bail if all primes are bad.
//! 5. CRT-reconstruct each invariant factor; sign-symmetric lift to ℤ.
//!
//! Per Storjohann (2000) §7. `modularsnf` provides dev-only cross-validation
//! under the `modularsnf-oracle` feature flag. Prime selection + CRT
//! reconstruction live in [`crate::snf::crt`]; this module composes them.

use catgraph::errors::CatgraphError;

use crate::snf::crt::{crt_reconstruct_signed, select_primes_for_bound};

/// Compute Hadamard bound `H(A) = ∏_i √(Σ_j a_ij²)`.
///
/// Returns ⌈H(A)⌉ as `u128` (Hadamard bounds can exceed `i64` for moderate
/// matrix sizes).
///
/// # Errors
///
/// - Any intermediate sum-of-squares overflows `i128` (entry value squared, or row sum).
/// - The product of row-norms exceeds `f64` range (matrix too large or too dense).
/// - The product exceeds `u128` range after `f64` accumulation.
pub fn hadamard_bound(a: &[Vec<i64>]) -> Result<u128, CatgraphError> {
    let mut bound: f64 = 1.0;
    for row in a {
        let sum_sq: i128 = row.iter().try_fold(0_i128, |acc, &x| {
            // `checked_mul` is defensive: for `x: i64`, `x²` is bounded by
            // `(2^63 − 1)² ≈ 2^126 < i128::MAX = 2^127 − 1`, so the multiplication
            // cannot overflow on the current i64 input type. Preserved for
            // forward-compatibility with future widening of entry types
            // (e.g. an `IntegerLikeRig::to_i128` path); the unreachable error
            // branch's allocation cost is one branch in a non-hot path.
            let x_sq = i128::from(x).checked_mul(i128::from(x)).ok_or_else(|| {
                CatgraphError::Composition {
                    message: format!("Hadamard bound: i128 overflow on x^2 where x={x}"),
                }
            })?;
            acc.checked_add(x_sq)
                .ok_or_else(|| CatgraphError::Composition {
                    message: format!(
                        "Hadamard bound: i128 overflow accumulating row-sum (acc={acc:e}, x={x})"
                    ),
                })
        })?;
        #[allow(
            clippy::cast_precision_loss,
            reason = "Hadamard bound is a rough sizing estimate; f64 precision suffices for prime-product sizing"
        )]
        let row_norm = (sum_sq as f64).sqrt();
        bound *= row_norm;
        if !bound.is_finite() {
            return Err(CatgraphError::Composition {
                message: "Hadamard bound exceeds f64 range; matrix too large or too dense"
                    .to_string(),
            });
        }
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "u128::MAX = 2^128 − 1 rounds UP to 2^128 in f64 (the next representable mantissa); \
                  the `bound > u128_max_f64` comparison is therefore conservative — any bound that \
                  passes is strictly less than 2^128 in f64-rounded form, so the subsequent \
                  `bound.ceil() as u128` saturating-cast is safe."
    )]
    let u128_max_f64 = u128::MAX as f64;
    if bound > u128_max_f64 {
        return Err(CatgraphError::Composition {
            message: format!("Hadamard bound {bound:e} exceeds u128 range"),
        });
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bound is positive and finite, verified above"
    )]
    Ok(bound.ceil() as u128)
}

/// Integer Smith Normal Form via multi-prime CRT reconstruction.
///
/// Returns `(U, V, S)` over ℤ such that `U · A · V = S` with `S` diagonal
/// and `s_0 | s_1 | ... | s_r` over ℤ (the classical integer SNF).
///
/// Composes the three substrate primitives ([`hadamard_bound`],
/// [`select_primes_for_bound`], [`crt_reconstruct_signed`]) with the
/// modular [`smith_normal_form`](super::smith_normal_form), plus a final
/// **integer chain rebalance** step that normalises the CRT-lifted diagonal
/// into canonical Smith form via the elementary-divisor / determinantal-divisor
/// identities.
///
/// # Algorithm
///
/// 1. **Hadamard bound** — `H(A) = ∏_i ||a_i||_2` (upper-bounds `|det(A)|`).
/// 2. **Select primes** — pick primes from `(2^30, 2^31)` whose product
///    exceeds `2 · H(A)`, with `k_max = 16`.
/// 3. **Per-prime SNF** — call [`smith_normal_form`](super::smith_normal_form)
///    on `A` for each selected prime; record per-prime `(U, V, S)`.
/// 4. **Good-prime filter** — drop primes whose modular SNF disagrees with
///    the consensus on the count of unit-coprime diagonal entries (a
///    rank-mod-`p` proxy). The first prime sets the canonical rank; any
///    subsequent prime with a different rank is "bad" and skipped.
/// 5. **CRT-reconstruct diagonal** — for each diagonal index `j`,
///    reconstruct the per-prime diagonal product `∏ s_i (mod p)` into an
///    integer in `[−⌊P/2⌋, ⌊P/2⌋]` via [`crt_reconstruct_signed`]. This
///    yields `d_j ∈ ℤ` such that `∏_j d_j = det(A)` (up to sign), but
///    `(d_0, …, d_{r−1})` is generally **not** in Smith form: the modular
///    SNF normalises divisibility differently from the integer SNF, and
///    for `p` coprime to the invariant factors the diagonal can be any
///    factorisation of `det(A)` permitted by `Z/pZ` units.
/// 6. **Chain rebalance** — apply the integer SNF of a diagonal integer
///    matrix via the elementary-divisor identity `s_k = D_k / D_{k−1}`
///    where `D_k = gcd of all k×k principal minors of diag(|d_0|, …)`.
///    For a diagonal matrix this reduces to `D_k = gcd of all k-subset
///    products of {|d_0|, …, |d_{r−1}|}`. The recurrence
///    `s_k = gcd(d_0, …, d_{r−1}) / (s_0 · … · s_{k−1})` is a fold
///    over per-step GCDs; implementation detail in `integer_chain_rebalance`
///    (private fn).
/// 7. **Return U + V from the first good prime** — a simplification.
///    Full per-entry CRT for U + V is deferred (#35; consumers requesting
///    integer-exact U + V will surface the need).
///
/// # Why the chain rebalance is needed
///
/// The modular SNF over `Z/pZ` is **not canonical when `p` is coprime to
/// the invariant factors**: it places any units-of-`Z/p` factorisation of
/// `det(A)` on the diagonal, not specifically the integer invariant factors.
/// For the Wikipedia 3×3 with integer SNF `diag(2, 2, 156)`, the modular
/// SNF over `p = 2^31 − 1` returns `diag(2, 6, −52)` (whose product is
/// `−det(A) = −624`); each entry is correct mod `p` but the chain
/// `2 | 6 | −52` is not the integer chain `2 | 2 | 156`. Rebalancing
/// via GCD-of-subset-products produces the canonical integer chain
/// (Newman 1972 §1.4 Theorem II.9; cross-ref Smith 1861).
///
/// # Inputs
///
/// - `a`: rectangular matrix over ℤ stored as `Vec<Vec<i64>>`. Shape is
///   `rows × cols` with `rows = a.len()` and `cols = a[0].len()`.
///
/// # Returns
///
/// `Ok((U, V, S))` with shapes `(rows × rows, cols × cols, rows × cols)`.
/// `S` is in integer Smith Normal Form with non-negative invariant factors
/// on the principal diagonal and zeros elsewhere. `U` + `V` are the
/// modular transforms from the first good prime (a simplification;
/// full per-entry CRT for `U` + `V` is deferred, #35).
///
/// # Edge cases
///
/// - Empty input (`rows == 0 || cols == 0`): returns `Ok((vec![], vec![],
///   vec![]))` — the empty SNF is trivially valid.
/// - Zero matrix: returns `S = 0` (the chain rebalance preserves zeros).
/// - Identity matrix: returns `S = I_{min(rows,cols)}` padded with zeros.
///
/// # Errors
///
/// - [`hadamard_bound`] error (matrix too large or too dense for `f64` /
///   `u128` accumulation).
/// - [`select_primes_for_bound`] error (more than `k_max = 16` primes
///   needed for the lift; vanishingly unlikely for the shipped fixtures, but
///   surfaced rather than silently truncated).
/// - All selected primes are "bad" (rank inconsistent across all 16
///   primes). Defensively unreachable: it would require every prime in
///   `(2^30, 2^31)` to divide an invariant factor of `A`, which has
///   measure zero for any fixed matrix.
/// - [`crt_reconstruct_signed`] error (e.g. final value exceeds `i64`
///   range — defensive; the magnitude-homology fixtures stay well
///   under `i64::MAX`).
/// - [`smith_normal_form`](super::smith_normal_form) error propagated
///   from the per-prime call (non-rectangular input, non-positive modulus
///   — neither fires here since the modulus is `> 2^30`).
///
/// # Example
///
/// ```
/// use catgraph_magnitude::snf::smith_normal_form_integer;
///
/// // Wikipedia integer SNF over ℤ: diag(2, 2, 156).
/// // <https://en.wikipedia.org/wiki/Smith_normal_form#Example>
/// let a = vec![vec![2, 4, 4], vec![-6, 6, 12], vec![10, 4, 16]];
/// let (_u, _v, s) = smith_normal_form_integer(&a).unwrap();
/// assert_eq!(s[0][0], 2);
/// assert_eq!(s[1][1], 2);
/// assert_eq!(s[2][2], 156);
/// ```
///
/// # References
///
/// Storjohann (2000) §7 + Bradley-Vigneaux 2025 (algorithm sketch)
/// augmented with Newman (1972) §1.4 Thm II.9 integer
/// chain rebalance via determinantal divisors. Cross-validated dev-only
/// against `events555/modularsnf` at SHA `d62535e` under the
/// `modularsnf-oracle` feature flag.
pub fn smith_normal_form_integer(
    a: &[Vec<i64>],
) -> Result<(Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>), CatgraphError> {
    let rows = a.len();
    let cols = if rows == 0 { 0 } else { a[0].len() };
    if rows == 0 || cols == 0 {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }
    let dim_min = rows.min(cols);

    // 1. Hadamard bound.
    let bound = hadamard_bound(a)?;

    // 2. Select primes.
    let primes = select_primes_for_bound(bound, 16)?;

    // 3-4. SNF mod each prime + good-prime filter (by consistent rank).
    //
    // Rank proxy: count diagonal entries `s[i][i]` that are nonzero and
    // coprime to `p` (i.e. `gcd(s[i][i], p) != p`). For prime `p` this is
    // equivalent to `s[i][i] mod p != 0`. "Good" primes agree on this
    // count; outliers are dropped — they correspond to primes that happen
    // to divide some invariant factor and so produce a `0` on the diagonal
    // where ℤ would have a unit.
    let mut per_prime: Vec<(i64, Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>)> =
        Vec::with_capacity(primes.len());
    let mut canonical_rank: Option<usize> = None;
    for &p in &primes {
        let (u, v, s) = super::smith_normal_form(a, p)?;
        let rank: usize = (0..dim_min)
            .filter(|&i| s[i][i] != 0 && super::zmod::gcd_raw(s[i][i], p) != p)
            .count();
        match canonical_rank {
            None => canonical_rank = Some(rank),
            Some(r) if r != rank => continue, // bad prime; skip
            _ => {}
        }
        per_prime.push((p, u, v, s));
    }
    if per_prime.is_empty() {
        return Err(CatgraphError::Composition {
            message: format!(
                "smith_normal_form_integer: all {} selected primes were 'bad' \
                 (rank mismatch); escalate k_max=16 (defensively unreachable \
                 in practice)",
                primes.len()
            ),
        });
    }

    // 5. CRT-reconstruct each diagonal entry to an integer in
    //    `[−⌊P/2⌋, ⌊P/2⌋]`. The result is the modular SNF's diagonal
    //    interpreted over ℤ, NOT necessarily in canonical Smith form
    //    — see step 6.
    let good_primes: Vec<i64> = per_prime.iter().map(|(p, _, _, _)| *p).collect();
    let mut diag_lifted: Vec<i64> = Vec::with_capacity(dim_min);
    for j in 0..dim_min {
        let residues: Vec<i64> = per_prime.iter().map(|(_, _, _, s)| s[j][j]).collect();
        diag_lifted.push(crt_reconstruct_signed(&good_primes, &residues)?);
    }

    // 6. Chain rebalance: the modular SNF's diagonal is correct mod
    //    `∏ good_primes` but generally not in integer Smith form. Apply
    //    the integer SNF of a diagonal matrix via the determinantal-divisor
    //    identity `s_k = D_k / D_{k-1}` where `D_k = gcd of all k-subset
    //    products of |diag_lifted|`.
    let s_diag = integer_chain_rebalance(&diag_lifted)?;

    // 7. Place the rebalanced chain on the (rows × cols) diagonal.
    let mut s_lifted: Vec<Vec<i64>> = vec![vec![0; cols]; rows];
    for (j, &val) in s_diag.iter().enumerate() {
        s_lifted[j][j] = val;
    }

    // 8. Return U + V from the first good prime (cheaper than per-entry
    //    CRT; per-entry CRT for U + V is deferred).
    let (_, u_ref, v_ref, _) = &per_prime[0];
    Ok((u_ref.clone(), v_ref.clone(), s_lifted))
}

/// Rebalance a CRT-lifted diagonal `(d_0, …, d_{r−1})` into the canonical
/// integer Smith chain `(s_0, …, s_{r−1})` with `s_0 | s_1 | … | s_{r−1}`.
///
/// Implements Newman (1972) §1.4 Thm II.9: the integer Smith normal form
/// of a diagonal integer matrix `diag(d_0, …, d_{r−1})` is `diag(s_0, …,
/// s_{r−1})` where `s_k = D_k / D_{k−1}`, with `D_k = gcd of all k×k
/// principal minors`. For a diagonal matrix the `k×k` principal minors are
/// products of `k`-subsets of `{d_0, …, d_{r−1}}`, so:
///
/// ```text
/// D_0 = 1
/// D_1 = gcd(d_0, d_1, …, d_{r−1})
/// D_2 = gcd(d_i · d_j : i < j)
/// …
/// D_r = |∏ d_i|
/// ```
///
/// We compute `D_k` incrementally as `gcd` over `k`-subset products. For
/// the small `r ≤ ~20` cases in the magnitude-homology fixtures, the
/// `2^r` subset enumeration is acceptable; a polynomial dynamic-programming
/// pass would hoist this if needed.
///
/// Returns absolute values throughout (canonical Smith form has
/// non-negative invariant factors).
///
/// # Errors
///
/// - Subset product overflows `i128` during `D_k` accumulation. For the
///   magnitude-homology fixtures (`r ≤ 20`, CRT-lifted entries
///   bounded by `|det(A)|`) this is unreachable, but the explicit error
///   prevents a silent wrap from corrupting an invariant factor.
/// - An invariant factor `s_k` exceeds `i64` range. Defensive: for the
///   magnitude-homology consumer, individual factors are bounded
///   by `|det(A)|` which fits comfortably in i64.
///
/// Worked example (Wikipedia 3×3): pre-rebalance diag `(2, 6, 52)` (taking
/// absolute values of CRT-lifted `(2, 6, −52)`). `D_0 = 1`,
/// `D_1 = gcd(2, 6, 52) = 2`, `D_2 = gcd(2·6, 2·52, 6·52) = gcd(12, 104,
/// 312) = 4`, `D_3 = 2·6·52 = 624`. Chain: `s_0 = 2/1 = 2`, `s_1 = 4/2 =
/// 2`, `s_2 = 624/4 = 156`. Output: `(2, 2, 156)`.
fn integer_chain_rebalance(diag: &[i64]) -> Result<Vec<i64>, CatgraphError> {
    let r = diag.len();
    if r == 0 {
        return Ok(Vec::new());
    }
    // Take absolute values up front; the integer SNF is sign-invariant on
    // the diagonal (units `±1` are absorbed into U / V).
    let abs: Vec<i128> = diag.iter().map(|&d| i128::from(d).abs()).collect();

    // Zeros sink to the trailing positions. If any d_j == 0, the
    // determinantal divisors D_k for k > nonzero-count are 0, so the
    // tail of the chain is all zeros. Partition into nonzero leading
    // segment + zero trailing tail; chain-rebalance the nonzero part.
    let nonzero: Vec<i128> = abs.iter().copied().filter(|&x| x != 0).collect();
    let zero_count = r - nonzero.len();

    // Compute D_k for k = 0..=nz_len via subset-product GCDs.
    let nz_len = nonzero.len();
    let mut det_divisors: Vec<i128> = vec![0; nz_len + 1];
    det_divisors[0] = 1;
    #[allow(
        clippy::needless_range_loop,
        reason = "k is the cardinality of the subset enumerated at each step, not an iterator over det_divisors; expressing this as enumerate() obscures the D_k = gcd of all k×k principal minors paper-mapping (Newman 1972 §1.4 Thm II.9)"
    )]
    for k in 1..=nz_len {
        let mut g: i128 = 0;
        let mut overflow: Option<(usize, usize)> = None;
        // Enumerate all k-subsets of {0, …, nz_len-1} via combinations.
        // For nz_len ≤ ~20 (the shipped fixture size) this is ≤ ~1M iterations.
        // Use `checked_mul` so a subset-product overflow
        // surfaces as an explicit error rather than `saturating_mul`'s
        // silent `i128::MAX` (which would corrupt the gcd downstream).
        enumerate_subsets(nz_len, k, &mut |subset| {
            if overflow.is_some() {
                return;
            }
            let mut prod: i128 = 1;
            for &i in subset {
                if let Some(p) = prod.checked_mul(nonzero[i]) {
                    prod = p;
                } else {
                    overflow = Some((k, i));
                    return;
                }
            }
            g = gcd_i128(g, prod);
        });
        if let Some((k_bad, i_bad)) = overflow {
            return Err(CatgraphError::Composition {
                message: format!(
                    "integer_chain_rebalance: i128 overflow during D_{k_bad} subset-product \
                     accumulation at index {i_bad} (nonzero[{i_bad}]={}); \
                     escalate to BigInt-native rebalance",
                    nonzero[i_bad]
                ),
            });
        }
        det_divisors[k] = g;
    }

    // s_k = D_k / D_{k-1}.
    let mut chain: Vec<i64> = Vec::with_capacity(r);
    for window in det_divisors.windows(2) {
        let s_k = window[1] / window[0];
        // Surface s_k > i64::MAX as an explicit error rather
        // than the previous `unwrap_or(i64::MAX)` silent saturation. For
        // the magnitude-homology fixtures s_k divides |det(A)| which is
        // i64-bounded; the explicit error defends against future regressions.
        let s_k_i64 = i64::try_from(s_k).map_err(|_| CatgraphError::Composition {
            message: format!(
                "integer_chain_rebalance: invariant factor {s_k} exceeds i64 range; \
                 escalate to BigInt-native rebalance"
            ),
        })?;
        chain.push(s_k_i64);
    }
    // Pad with zeros for the trailing rank-deficient slots.
    chain.extend(std::iter::repeat_n(0, zero_count));
    Ok(chain)
}

/// `i128` GCD via Euclid's algorithm, treating `gcd(0, x) = |x|` and
/// `gcd(0, 0) = 0`. Used by [`integer_chain_rebalance`] for the
/// determinantal-divisor accumulation.
fn gcd_i128(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Iterate every `k`-subset of `{0, …, n−1}` in lexicographic order,
/// invoking `f` on each subset as a borrowed `&[usize]`. Used by
/// [`integer_chain_rebalance`] for determinantal-divisor enumeration.
///
/// For `n ≤ ~20` (the magnitude-homology fixture size), `C(n, k) ≤ ~1M`
/// is acceptable. A polynomial DP alternative would apply if larger
/// fixtures land.
fn enumerate_subsets<F>(n: usize, k: usize, f: &mut F)
where
    F: FnMut(&[usize]),
{
    if k == 0 {
        f(&[]);
        return;
    }
    if k > n {
        return;
    }
    let mut idx: Vec<usize> = (0..k).collect();
    loop {
        f(&idx);
        // Advance to next k-combination in lex order.
        let mut i = k;
        while i > 0 {
            i -= 1;
            if idx[i] < n - k + i {
                idx[i] += 1;
                for j in i + 1..k {
                    idx[j] = idx[j - 1] + 1;
                }
                break;
            }
            if i == 0 {
                return;
            }
        }
    }
}
