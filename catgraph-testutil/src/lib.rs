//! Dev-only test/bench helpers shared across the catgraph workspace.
//!
//! **Unpublished** (`publish = false`): it appears under a member's
//! `[dev-dependencies]`, never in a published crate's `[dependencies]`.
//!
//! - [`Lcg`] — deterministic linear congruential generator over `f64` in
//!   `[0.0, 1.0)`.
//! - [`approx_rel`] and [`assert_approx_rel!`] — relative-plus-absolute float
//!   comparison.
//! - [`strategy`] — `proptest` float strategies over the full `f64` exponent
//!   range and over near-cancelling pairs.
//! - [`all_perms`] and [`all_perm_indices`] — exhaustive `Sₙ` enumeration.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod approx;
mod perm;
pub mod strategy;

pub use approx::approx_rel;
pub use perm::{all_perm_indices, all_perms};

/// The LCG multiplier (Knuth MMIX).
const MULTIPLIER: u64 = 6_364_136_223_846_793_005;

/// The additive increment (Knuth MMIX) used by [`Lcg::new`].
const STANDARD_INCREMENT: u64 = 1_442_695_040_888_963_407;

/// A deterministic linear congruential generator yielding `f64` in
/// `[0.0, 1.0)`.
///
/// # Stream contract
///
/// The output stream is fixed bit-for-bit. Each step is
///
/// ```text
/// state = state.wrapping_mul(MULTIPLIER).wrapping_add(increment);
/// out   = ((state >> 33) as f64) / ((1u64 << 31) as f64);
/// ```
///
/// with `MULTIPLIER = 6_364_136_223_846_793_005`. [`Lcg::new`] uses the
/// increment `1_442_695_040_888_963_407`; [`Lcg::with_increment`] takes any
/// increment. Both take the seed as the literal initial state, so seed
/// preparation such as `seed | 1` stays at the call site.
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
    /// Creates an LCG whose initial state is `seed`, with the MMIX increment.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed,
            increment: STANDARD_INCREMENT,
        }
    }

    /// Creates an LCG whose initial state is `seed`, with `increment`.
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

    /// Uniform integer in the inclusive range `[lo, hi]`, for `lo <= hi` and
    /// `hi - lo + 1` exactly representable in `f64`.
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

    /// Three streams — `new(42)`, `with_increment(42, 1)`, `new(0xA11CE | 1)` —
    /// against their first four values as raw bit patterns.
    #[test]
    fn golden_streams() {
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
