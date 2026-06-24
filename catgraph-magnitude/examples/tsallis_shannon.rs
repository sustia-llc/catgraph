//! Shannon recovery demo: `lim_{t→1} H_t(p) = −Σ pᵢ ln pᵢ`.
//!
//! Evaluates `tsallis_entropy(p, t)` at a dense mesh of `t` values
//! approaching 1 from above over 50 seeded random probability distributions
//! of sizes 2–5. Prints the worst-case `|H_t(p) − H_1(p)|` at each `δt`
//! and asserts paper-anchored numerical bounds.
//!
//! ## Paper anchor
//!
//! BV 2025 **Rem 3.11** (p.18): for any probability mass function `p : S → [0,1]`,
//!
//! ```text
//! lim_{t→1} H_t(p) = lim_{t→1} (1 − Σ p(s)^t)/(t − 1) = −Σ p(s) ln p(s) =: H(p).
//! ```
//!
//! The limit arises by L'Hôpital's rule (the `0/0` form at `t = 1`). This
//! example verifies that the `catgraph_magnitude::magnitude::tsallis_entropy`
//! implementation tracks this limit:
//!
//! - For `|t − 1| < TSALLIS_SHANNON_EPS = 1e-6`: the function returns
//!   `−Σ pᵢ ln pᵢ` directly (special-case branch), so the error is **exactly
//!   zero** regardless of the distribution.
//! - For `|t − 1| ≥ TSALLIS_SHANNON_EPS`: the function evaluates the Tsallis
//!   branch, and the Taylor remainder `O(δt · Σ pᵢ ln²(pᵢ))` bounds the error.
//!   With `δt = 1e-3` and distributions of size ≤ 5 the bound is `< 5e-3`.
//!
//! ## Random generation
//!
//! A minimal PCG-32-style `u64` LCG (same as in `tests/lm_category.rs`) seeds
//! from a fixed constant for full reproducibility, without a `rand` dev-dep.

use catgraph_magnitude::TSALLIS_SHANNON_EPS;
use catgraph_magnitude::magnitude::tsallis_entropy;

// ---------------------------------------------------------------------------
// Minimal deterministic pseudo-random generator
// ---------------------------------------------------------------------------

/// A `u64` LCG used throughout the catgraph workspace for seeded sampling.
/// Parameters from Knuth MMIX.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// Next value in `[0.0, 1.0)`.
    #[allow(clippy::cast_precision_loss)]
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
    }

    /// Uniform integer in `[lo, hi]` (inclusive).
    ///
    /// The two `#[allow]` guards cover the intentional casts: `range` is at most
    /// 4 here (distributions are size 2–5), well within `f64` precision; the
    /// result is bounded by `range` so truncation cannot exceed `hi`.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn next_usize(&mut self, lo: usize, hi: usize) -> usize {
        let range = (hi - lo + 1) as f64;
        lo + (self.next_f64() * range) as usize
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Shannon entropy `H(p) = −Σ pᵢ ln pᵢ` with `0 · ln 0 = 0`.
fn shannon(p: &[f64]) -> f64 {
    p.iter()
        .filter(|&&pi| pi > 0.0)
        .map(|&pi| -pi * pi.ln())
        .sum()
}

/// Generate `n_dists` random normalized distributions of sizes in `[min_k, max_k]`.
fn random_distributions(
    rng: &mut Lcg,
    n_dists: usize,
    min_k: usize,
    max_k: usize,
) -> Vec<Vec<f64>> {
    (0..n_dists)
        .map(|_| {
            let k = rng.next_usize(min_k, max_k);
            // Sample k values in (0, 1] — lower-bound 0.01 prevents near-zero
            // denominators. Same strategy as the workspace's existing proptests.
            let raw: Vec<f64> = (0..k).map(|_| 0.01 + 0.99 * rng.next_f64()).collect();
            let total: f64 = raw.iter().sum();
            raw.into_iter().map(|x| x / total).collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    // Seed chosen to reproduce across runs; same LCG as tests/lm_category.rs.
    let mut rng = Lcg::new(42);
    let dists = random_distributions(&mut rng, 50, 2, 5);

    // t mesh: six values approaching 1 from above.
    // The first four are in the Tsallis branch; the last two are within
    // TSALLIS_SHANNON_EPS = 1e-6 and hit the Shannon special-case branch.
    let delta_ts: &[f64] = &[1e-2, 1e-3, 1e-4, 1e-5, 1e-6, 1e-7];

    println!("Tsallis-Shannon recovery  (BV 2025 Rem 3.11)");
    println!("Worst-case |H_t(p) − H_1(p)| over 50 random distributions of size 2–5");
    println!();
    println!(
        "  {:<10}  {:<10}  {:<22}  branch",
        "t", "|t − 1|", "worst |H_t − H_1|"
    );
    println!("  {}", "-".repeat(64));

    let mut worst_tsallis_at_1e3: f64 = 0.0;
    let mut worst_tsallis_at_1e6: f64 = 0.0;
    let mut worst_tsallis_at_1e7: f64 = 0.0;

    for &dt in delta_ts {
        let t = 1.0 + dt;
        let branch = if (t - 1.0).abs() < TSALLIS_SHANNON_EPS {
            "Shannon (exact)"
        } else {
            "Tsallis"
        };

        let worst: f64 = dists
            .iter()
            .map(|p| (tsallis_entropy(p, t) - shannon(p)).abs())
            .fold(0.0_f64, f64::max);

        println!("  {t:<10.8}  {dt:<10.1e}  {worst:<22.6e}  {branch}");

        // Stash specific values for assertions below.
        #[allow(clippy::float_cmp)]
        if dt == 1e-3 {
            worst_tsallis_at_1e3 = worst;
        }
        #[allow(clippy::float_cmp)]
        if dt == 1e-6 {
            worst_tsallis_at_1e6 = worst;
        }
        #[allow(clippy::float_cmp)]
        if dt == 1e-7 {
            worst_tsallis_at_1e7 = worst;
        }
    }
    println!();

    // -----------------------------------------------------------------------
    // Assertions
    // -----------------------------------------------------------------------

    // (1) Within the special-case threshold the error is exactly zero.
    //     |t − 1| = 1e-6 and 1e-7 both satisfy < TSALLIS_SHANNON_EPS = 1e-6.
    assert!(
        worst_tsallis_at_1e6 == 0.0,
        "Shannon special case (δt=1e-6) should be exact, got worst error {worst_tsallis_at_1e6:.3e}"
    );
    assert!(
        worst_tsallis_at_1e7 == 0.0,
        "Shannon special case (δt=1e-7) should be exact, got worst error {worst_tsallis_at_1e7:.3e}"
    );

    // (2) For δt = 1e-3 (Tsallis branch), worst error < 5e-3.
    //     Taylor bound: residual ~ δt · max_p Σ pᵢ ln²(pᵢ).
    //     With δt = 1e-3 and distributions of size ≤ 5 the bound is comfortably
    //     below 5e-3 (experimentally ~5e-4 for these 50 distributions).
    assert!(
        worst_tsallis_at_1e3 < 5e-3,
        "Tsallis-Shannon worst error at δt=1e-3 = {worst_tsallis_at_1e3:.3e} ≥ 5e-3"
    );

    println!("Assertions passed:");
    println!(
        "  Shannon special case at δt=1e-6 and δt=1e-7: error = 0 (exact, returns −Σ p ln p directly)"
    );
    println!("  Tsallis branch at δt=1e-3: worst |H_t − H_1| = {worst_tsallis_at_1e3:.3e} < 5e-3");
    println!();
    println!(
        "Headline: worst-case Shannon recovery error < {:.0e} for δt < TSALLIS_SHANNON_EPS = {TSALLIS_SHANNON_EPS:.0e}.",
        1e-12_f64
    );
}
