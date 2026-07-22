//! Dev-only test/bench utilities shared across the catgraph workspace.
//!
//! This crate is **unpublished** (`publish = false`) and dev-only: it must only
//! ever appear under a member's `[dev-dependencies]`, never in a published
//! crate's `[dependencies]`. It exists to dedup helpers that had drifted into
//! seven near-identical inline copies across tests, benches, and examples
//! (issue #33).
//!
//! The only helper today is [`Lcg`], a minimal deterministic linear
//! congruential generator used to build seeded fixtures without pulling `rand`
//! into the dev-dependency graph.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The LCG multiplier — the 64-bit constant from the PCG family / Knuth MMIX
/// lineage (also used by `pcg64`). Shared by every stream this type produces.
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// The standard additive increment (Knuth MMIX). Used by [`Lcg::new`]; a stream
/// with a different increment is available via [`Lcg::with_increment`].
const STANDARD_INCREMENT: u64 = 1_442_695_040_888_963_407;

/// A tiny deterministic linear congruential generator yielding `f64` in
/// `[0.0, 1.0)`.
///
/// # Byte-identity contract
///
/// The output stream is a **fixed contract**: workspace tests pin numeric
/// outcomes and benches document wall-times against these fixtures, so the
/// stream must never change bit-for-bit. Each step is
///
/// ```text
/// state = state.wrapping_mul(MULTIPLIER).wrapping_add(increment);
/// out   = ((state >> 33) as f64) / ((1u64 << 31) as f64);
/// ```
///
/// with `MULTIPLIER = 6_364_136_223_846_793_005`. [`Lcg::new`] uses the standard
/// increment `1_442_695_040_888_963_407`; [`Lcg::with_increment`] lets a caller
/// pin a different one (the `catgraph-physics` wasserstein bench historically
/// used `1`, kept for stream compatibility). The `state >> 33` extraction and
/// the `/ 2^31` divisor are load-bearing — do not "clean them up".
///
/// Seed preparation (e.g. `seed | 1` to avoid a zero-state fixed point) is the
/// **caller's** responsibility and stays at the call site: `new` and
/// `with_increment` take the seed as the literal initial state.
///
/// This lineage traces to issue #33 (dedup of seven drifted inline copies).
///
/// # Examples
///
/// ```
/// use catgraph_testutil::Lcg;
///
/// let mut rng = Lcg::new(42);
/// let x = rng.next_f64();
/// assert!((0.0..1.0).contains(&x));
/// ```
pub struct Lcg {
    state: u64,
    increment: u64,
}

impl Lcg {
    /// Creates an LCG seeded at `seed` using the standard MMIX increment.
    ///
    /// The seed is used as the literal initial state; any `seed | 1`-style
    /// preparation is the caller's responsibility (see the type-level
    /// byte-identity contract).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            increment: STANDARD_INCREMENT,
        }
    }

    /// Creates an LCG seeded at `seed` with a caller-chosen `increment`.
    ///
    /// Exists to preserve the historically divergent stream used by the
    /// `catgraph-physics` wasserstein bench (`increment = 1`); see the
    /// byte-identity contract on [`Lcg`] (#33).
    #[must_use]
    pub fn with_increment(seed: u64, increment: u64) -> Self {
        Self {
            state: seed,
            increment,
        }
    }

    /// Advances the generator and returns the next value in `[0.0, 1.0)`.
    #[allow(clippy::cast_precision_loss)]
    pub fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(MULTIPLIER)
            .wrapping_add(self.increment);
        ((self.state >> 33) as f64) / ((1u64 << 31) as f64)
    }

    /// Uniform integer in `[lo, hi]` (inclusive).
    ///
    /// The `#[allow]` guards cover the intentional casts: `range` is small in
    /// practice (a handful of values), well within `f64` precision; the result
    /// is bounded by `range` so truncation cannot exceed `hi`.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn next_usize(&mut self, lo: usize, hi: usize) -> usize {
        let range = (hi - lo + 1) as f64;
        lo + (self.next_f64() * range) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden values pinning the stream contract forever. Computed from the
    /// pre-refactor inline formula (issue #33 scratch extraction); stored as raw
    /// bit patterns so the comparison is exact and unambiguous.
    #[test]
    fn golden_streams() {
        // (a) new(42): state = 42, standard increment.
        let mut a = Lcg::new(42);
        let a_expected = [
            0x3fe2_2ef1_5d80_0000_u64,
            0x3fcc_dbfc_5200_0000,
            0x3fda_6bf1_6900_0000,
            0x3fe4_2c38_87c0_0000,
        ];
        for &bits in &a_expected {
            assert_eq!(a.next_f64().to_bits(), bits);
        }

        // (b) with_increment(42, 1): the wasserstein-bench stream.
        let mut b = Lcg::with_increment(42, 1);
        let b_expected = [
            0x3fdf_5c83_db80_0000_u64,
            0x3fde_003f_b000_0000,
            0x3fe1_ed5b_4a40_0000,
            0x3f99_867e_7000_0000,
        ];
        for &bits in &b_expected {
            assert_eq!(b.next_f64().to_bits(), bits);
        }

        // (c) new(0xA11CE | 1): the coalition-fixture stream.
        let mut c = Lcg::new(0xA11CE | 1);
        let c_expected = [
            0x3fde_517b_0300_0000_u64,
            0x3fe9_d985_5900_0000,
            0x3fed_a5c3_5b00_0000,
            0x3f9f_d638_8000_0000,
        ];
        for &bits in &c_expected {
            assert_eq!(c.next_f64().to_bits(), bits);
        }
    }

    #[test]
    fn range_is_unit_interval() {
        let mut rng = Lcg::new(1);
        for _ in 0..1000 {
            let x = rng.next_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn next_usize_is_bounded() {
        let mut rng = Lcg::new(7);
        for _ in 0..1000 {
            let k = rng.next_usize(2, 5);
            assert!((2..=5).contains(&k));
        }
    }
}
