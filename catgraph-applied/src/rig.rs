//! Rigs (a.k.a. semirings) — F&S Seven Sketches Def 5.36.
//!
//! A **rig** is a tuple `(R, 0, 1, +, *)` where:
//! - `(R, +, 0)` is a commutative monoid,
//! - `(R, *, 1)` is a monoid,
//! - `*` distributes over `+` from both sides,
//! - `0` is absorbing: `a * 0 = 0 = 0 * a`.
//!
//! This is a ring without negatives. The [`Rig`] trait packages
//! `deep_causality_num::{Zero, One}` + `Add` + `Mul` with a marker. A blanket
//! impl lifts any concrete type satisfying those bounds. `Zero`/`One` are
//! sourced from `deep_causality_num` (Phase 2 re-substrate); `Rig` itself stays
//! a catgraph-native semiring — it is **never** a `deep_causality_num::Ring`,
//! whose lowest ring requires `Sub` (which `BoolRig`/`Tropical` lack).
//!
//! ## Concrete instances
//!
//! - [`BoolRig`] — `(∨, ∧)` Boolean rig. Recovers `Rel<Λ>`-style semantics
//!   when used as a hom-annotation in `WeightedCospan` (Phase 6).
//! - [`UnitInterval`] — `[0,1]` Viterbi semiring `(max, ·)`. Per BTV 2021
//!   the monoidal structure `(·, 1)` is the primary enrichment base for
//!   language categories; magnitude computations (BV 2025) use the
//!   embedding into ℝ rather than the Rig axioms directly.
//! - [`Tropical`] — min-plus semiring `([0,∞], min, +, +∞, 0)`. Used for
//!   magnitude homology and as the Lawvere-metric enrichment base
//!   (v0.5.1); `d = -ln π` converts `UnitInterval → Tropical`.
//! - [`F64Rig`] — plain real rig for `Mat(R)` and `SFG_R` demos.
//!
//! Primitive integer and float types are rigs via the blanket impl, since
//! `deep_causality_num` provides their `Zero`/`One`. Note that
//! `rust_decimal::Decimal` (used by [`petri_net`](crate::petri_net) for token
//! multiplicities) is **not** automatically a rig under the DC re-substrate —
//! it does not implement `deep_causality_num::{Zero, One}`; a downstream impl
//! would be required to use it as a `Rig` coefficient.
//!
//! # `Eq + Hash` on `f64`-wrapping rigs
//!
//! [`UnitInterval`], [`Tropical`], and [`F64Rig`] manually implement `Eq` and
//! `Hash` via bit-exact `f64::to_bits()`. This is required by the v0.5.1
//! [`PropSignature`](crate::prop::PropSignature) supertrait widening:
//! [`SfgGenerator<R>`](crate::sfg::SfgGenerator) now requires `R: Eq + Hash`,
//! and the [`prop::presentation::kb::CongruenceClosure`](crate::prop::presentation::kb::CongruenceClosure)
//! term graph uses `SfgGenerator<R>` as a `HashMap` key. NaN caveats inherit
//! from `PartialEq`: a NaN payload would be non-reflexive; callers should not
//! construct NaN values in these newtypes (the [`UnitInterval::new`] validator
//! already rejects them).

use deep_causality_num::{One, Zero};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A rig (semiring). Blanket-impl'd for any `T: Clone + PartialEq + Zero + One + Add + Mul`.
///
/// Runtime axiom verification is available via [`verify_rig_axioms`] — see that
/// function's docstring for the 8 invariants it checks.
pub trait Rig: Clone + PartialEq + Zero + One + Add<Output = Self> + Mul<Output = Self> {}

impl<T> Rig for T where T: Clone + PartialEq + Zero + One + Add<Output = T> + Mul<Output = T> {}

/// Boolean rig `({false, true}, false, true, ∨, ∧)`.
///
/// The simplest non-trivial rig. Recovers `Rel<Λ>`-style edge semantics
/// when used as a hom-annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoolRig(pub bool);

impl Zero for BoolRig {
    fn zero() -> Self {
        BoolRig(false)
    }
    fn is_zero(&self) -> bool {
        !self.0
    }
}

impl One for BoolRig {
    fn one() -> Self {
        BoolRig(true)
    }
    fn is_one(&self) -> bool {
        self.0
    }
}

impl Add for BoolRig {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        BoolRig(self.0 || other.0)
    }
}

impl Mul for BoolRig {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        BoolRig(self.0 && other.0)
    }
}

/// Unit interval `[0, 1] ⊂ ℝ` as a rig under `(max, ·)`, the Viterbi semiring.
///
/// `(max, 0)` is the additive commutative monoid;
/// `(·, 1)` is the multiplicative monoid.
/// Distributivity holds because `max(a, b) · c = max(a · c, b · c)` on `[0, 1]`.
///
/// # Relationship to BTV 2021 language enrichment
///
/// BTV 2021 enriches the language category over the **monoidal** structure
/// `([0,1], ≤, ·, 1)`, not the rig axioms. The additive `max` structure is
/// only used when Unit Interval is treated as an idempotent rig (e.g. for
/// matrix representations via `Mat(UnitInterval)`). Magnitude computations
/// in BV 2025 operate via the embedding `UnitInterval → ℝ` via `-ln`, not
/// via rig arithmetic directly.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UnitInterval(f64);

impl UnitInterval {
    /// Construct, validating `value ∈ [0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns [`catgraph::errors::CatgraphError::RigAxiomViolation`] if the
    /// value is outside `[0, 1]` or is NaN.
    pub fn new(value: f64) -> Result<Self, catgraph::errors::CatgraphError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(catgraph::errors::CatgraphError::RigAxiomViolation {
                axiom: "UnitInterval range [0, 1]",
                witness: format!("value = {value}"),
            });
        }
        Ok(UnitInterval(value))
    }

    #[must_use]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Zero for UnitInterval {
    fn zero() -> Self {
        UnitInterval(0.0)
    }
    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
}

impl One for UnitInterval {
    fn one() -> Self {
        UnitInterval(1.0)
    }
    fn is_one(&self) -> bool {
        self.0 == 1.0
    }
}

impl Add for UnitInterval {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        UnitInterval(self.0.max(other.0))
    }
}

impl Mul for UnitInterval {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        UnitInterval(self.0 * other.0)
    }
}

// Bit-exact `Eq + Hash` for use as a `HashMap` key in the congruence-closure
// term graph (v0.5.1 `PropSignature: Eq + Hash` widening). NaN caveats
// inherit from `PartialEq`; [`UnitInterval::new`] rejects NaN on construction.
impl Eq for UnitInterval {}
impl std::hash::Hash for UnitInterval {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Tropical (min-plus) semiring over `[0, ∞]`, with `+∞` as the additive
/// zero and `0` as the multiplicative unit. Represents Lawvere metric-space
/// distances directly; use as the enrichment base for
/// `LawvereMetricSpace<T>` (v0.5.1).
///
/// Axioms: `(min, +∞)` is commutative monoid; `(+, 0)` is commutative monoid;
/// `+` distributes over `min`; `+∞ + x = +∞` (absorbing).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Tropical(pub f64);

impl Zero for Tropical {
    fn zero() -> Self {
        Tropical(f64::INFINITY)
    }
    fn is_zero(&self) -> bool {
        self.0.is_infinite() && self.0 > 0.0
    }
}

impl One for Tropical {
    fn one() -> Self {
        Tropical(0.0)
    }
    // Tropical's multiplicative unit is `Tropical(0.0)` (real 0 under +).
    fn is_one(&self) -> bool {
        self.0 == 0.0
    }
}

impl Add for Tropical {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Tropical(if self.0 < other.0 { self.0 } else { other.0 })
    }
}

impl Mul for Tropical {
    type Output = Self;
    // Tropical multiplication *is* real addition — this is the defining
    // property of the (min, +) semiring, not a misuse of the operator.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn mul(self, other: Self) -> Self {
        Tropical(self.0 + other.0)
    }
}

// Bit-exact `Eq + Hash` for use as a `HashMap` key in the congruence-closure
// term graph (v0.5.1 `PropSignature: Eq + Hash` widening). NaN caveats
// inherit from `PartialEq`.
impl Eq for Tropical {}
impl std::hash::Hash for Tropical {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Plain real rig `(ℝ, 0, 1, +, ·)`.
///
/// Included primarily for `Mat(R)` and `SFG_R` demonstration purposes. Note
/// that `F64Rig` is actually a **ring** (has negatives); we use the rig
/// layer because the Thm 5.60 presentation and Mat(R) theory only require
/// rig axioms.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct F64Rig(pub f64);

impl Zero for F64Rig {
    fn zero() -> Self {
        F64Rig(0.0)
    }
    fn is_zero(&self) -> bool {
        self.0 == 0.0
    }
}

impl One for F64Rig {
    fn one() -> Self {
        F64Rig(1.0)
    }
    fn is_one(&self) -> bool {
        self.0 == 1.0
    }
}

impl Add for F64Rig {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        F64Rig(self.0 + other.0)
    }
}

impl Mul for F64Rig {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        F64Rig(self.0 * other.0)
    }
}

// Ring + field operations on `F64Rig` (added 2026-04-25 as a prerequisite
// for catgraph-magnitude v0.1.0). `F64Rig` is the only `Ring + Div`-bounded
// rig in the workspace; `mobius_function::<F64Rig>` (Phase 6A.2) needs all
// four of `Neg`, `Sub`, `Div`, and `From<f64>`. The ring/field bound stays
// off `Rig` itself — these impls live only on `F64Rig` (and any future
// real-valued rig that elects to adopt them).
impl Neg for F64Rig {
    type Output = Self;
    fn neg(self) -> Self {
        F64Rig(-self.0)
    }
}

impl Sub for F64Rig {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        F64Rig(self.0 - other.0)
    }
}

impl Div for F64Rig {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        F64Rig(self.0 / other.0)
    }
}

impl From<f64> for F64Rig {
    fn from(x: f64) -> Self {
        F64Rig(x)
    }
}

impl From<i64> for F64Rig {
    /// Convert a signed integer to `F64Rig`. Used by
    /// `catgraph-magnitude`'s `boundary_matrix` to lift `(-1)^i` signs into
    /// the rig. **v0.5.5 addition.**
    ///
    /// Note: `i64` values exceeding 2^53 lose precision in `f64`; the SNF
    /// call sites use only `±1`, so this is not a concern in practice.
    #[allow(clippy::cast_precision_loss)]
    fn from(n: i64) -> Self {
        F64Rig(n as f64)
    }
}

// Bit-exact `Eq + Hash` for use as a `HashMap` key in the congruence-closure
// term graph (v0.5.1 `PropSignature: Eq + Hash` widening). NaN caveats
// inherit from `PartialEq`.
impl Eq for F64Rig {}
impl std::hash::Hash for F64Rig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Base-change between rigs: a `From → To` conversion functor.
///
/// Implementations should document whether the conversion preserves rig
/// structure (homomorphism) or merely embeds the set. The shipped impl
/// `UnitInterval → Tropical` via `d = -ln π` is a semiring *anti*-homomorphism:
/// it reverses monoid orientations (`[0,1]` with `·` becomes `[0,∞]` with `+`).
pub trait BaseChange<From: Rig>: Rig {
    fn base_change(from: From) -> Self;
}

impl BaseChange<UnitInterval> for Tropical {
    /// Lawvere metric embedding: `d(p) = -ln p` with `d(0) = +∞`.
    ///
    /// This is the standard BTV 2021 recipe for embedding probability
    /// `[0,1]` into the distance space `[0,∞]`.
    fn base_change(p: UnitInterval) -> Self {
        if p.0 == 0.0 {
            Tropical(f64::INFINITY)
        } else {
            Tropical(-p.0.ln())
        }
    }
}

/// Verify all 8 semiring axioms hold for three sample values `a, b, c: R`.
///
/// # The 8 axioms
///
/// 1. Additive commutativity: `a + b == b + a`
/// 2. Additive associativity: `(a + b) + c == a + (b + c)`
/// 3. Additive identity: `a + zero == a`
/// 4. Multiplicative associativity: `(a * b) * c == a * (b * c)`
/// 5. Multiplicative identity: `a * one == a == one * a`
/// 6. Left distributivity: `a * (b + c) == (a * b) + (a * c)`
/// 7. Right distributivity: `(a + b) * c == (a * c) + (b * c)`
/// 8. Absorbing zero: `a * zero == zero == zero * a`
///
/// # Errors
///
/// Returns [`catgraph::errors::CatgraphError::RigAxiomViolation`] on the
/// first violation, with a human-readable witness.
///
/// # Floating-point rigs
///
/// For rigs backed by `f64` (`UnitInterval`, `Tropical`, `F64Rig`), callers
/// should pre-filter NaN samples and may tolerate an `eps` of `1e-9` by
/// pre-normalizing values — this function uses `PartialEq` exactly.
pub fn verify_rig_axioms<R>(a: &R, b: &R, c: &R) -> Result<(), catgraph::errors::CatgraphError>
where
    R: Rig + std::fmt::Debug,
{
    let zero = R::zero();
    let one = R::one();

    let check = |cond: bool,
                 axiom: &'static str,
                 witness: String|
     -> Result<(), catgraph::errors::CatgraphError> {
        if cond {
            Ok(())
        } else {
            Err(catgraph::errors::CatgraphError::RigAxiomViolation { axiom, witness })
        }
    };

    check(
        a.clone() + b.clone() == b.clone() + a.clone(),
        "additive commutativity",
        format!("a={a:?}, b={b:?}"),
    )?;
    check(
        (a.clone() + b.clone()) + c.clone() == a.clone() + (b.clone() + c.clone()),
        "additive associativity",
        format!("a={a:?}, b={b:?}, c={c:?}"),
    )?;
    check(
        a.clone() + zero.clone() == a.clone(),
        "additive identity",
        format!("a={a:?}"),
    )?;
    check(
        (a.clone() * b.clone()) * c.clone() == a.clone() * (b.clone() * c.clone()),
        "multiplicative associativity",
        format!("a={a:?}, b={b:?}, c={c:?}"),
    )?;
    check(
        a.clone() * one.clone() == a.clone() && one.clone() * a.clone() == a.clone(),
        "multiplicative identity",
        format!("a={a:?}"),
    )?;
    check(
        a.clone() * (b.clone() + c.clone()) == (a.clone() * b.clone()) + (a.clone() * c.clone()),
        "left distributivity",
        format!("a={a:?}, b={b:?}, c={c:?}"),
    )?;
    check(
        (a.clone() + b.clone()) * c.clone() == (a.clone() * c.clone()) + (b.clone() * c.clone()),
        "right distributivity",
        format!("a={a:?}, b={b:?}, c={c:?}"),
    )?;
    check(
        a.clone() * zero.clone() == zero.clone() && zero.clone() * a.clone() == zero.clone(),
        "absorbing zero",
        format!("a={a:?}"),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_rig_zero_one_distinct() {
        assert_ne!(BoolRig::zero(), BoolRig::one());
    }

    #[test]
    fn unit_interval_rejects_out_of_range() {
        assert!(UnitInterval::new(1.5).is_err());
        assert!(UnitInterval::new(-0.1).is_err());
        assert!(UnitInterval::new(f64::NAN).is_err());
        assert!(UnitInterval::new(0.0).is_ok());
        assert!(UnitInterval::new(1.0).is_ok());
    }

    #[test]
    fn tropical_zero_is_infinity() {
        assert!(Tropical::zero().0.is_infinite());
        // Tropical::one() is defined as Tropical(0.0); exact equality is
        // intentional — we are testing the definition, not a computation.
        assert!(Tropical::one().0.abs() < f64::EPSILON);
    }

    #[test]
    fn base_change_unit_interval_to_tropical() {
        // p = 1.0 → -ln(1.0) = 0.0
        let p = UnitInterval::new(1.0).unwrap();
        let t = Tropical::base_change(p);
        assert!((t.0 - 0.0).abs() < 1e-9);

        // p = 0.0 → +∞ (special case)
        let p = UnitInterval::new(0.0).unwrap();
        let t = Tropical::base_change(p);
        assert!(t.0.is_infinite() && t.0 > 0.0);
    }

    #[test]
    fn verify_axioms_bool_rig() {
        for a in [BoolRig(false), BoolRig(true)] {
            for b in [BoolRig(false), BoolRig(true)] {
                for c in [BoolRig(false), BoolRig(true)] {
                    verify_rig_axioms(&a, &b, &c).unwrap_or_else(|e| {
                        panic!("BoolRig axiom failed at ({a:?}, {b:?}, {c:?}): {e}")
                    });
                }
            }
        }
    }

    #[test]
    fn verify_axioms_f64_rig_sample() {
        let samples = [F64Rig(0.0), F64Rig(1.0), F64Rig(2.5), F64Rig(-1.0)];
        for a in &samples {
            for b in &samples {
                for c in &samples {
                    verify_rig_axioms(a, b, c).unwrap();
                }
            }
        }
    }

    #[test]
    fn f64rig_from_i64_lifts_sign() {
        let neg_one: F64Rig = F64Rig::from(-1_i64);
        assert!((neg_one.0 - (-1.0)).abs() < f64::EPSILON);
        let pos_one: F64Rig = F64Rig::from(1_i64);
        assert!((pos_one.0 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn verify_axioms_unit_interval_sample() {
        // Use dyadic fractions (exactly representable in f64) to avoid IEEE-754
        // rounding drift tripping `PartialEq`-based axiom checks; see function
        // docstring "Floating-point rigs" note.
        let samples = [
            UnitInterval::new(0.0).unwrap(),
            UnitInterval::new(0.25).unwrap(),
            UnitInterval::new(0.5).unwrap(),
            UnitInterval::new(1.0).unwrap(),
        ];
        for a in &samples {
            for b in &samples {
                for c in &samples {
                    verify_rig_axioms(a, b, c).unwrap();
                }
            }
        }
    }

    #[test]
    fn verify_axioms_tropical_sample() {
        let samples = [
            Tropical(f64::INFINITY),
            Tropical(0.0),
            Tropical(1.5),
            Tropical(5.0),
        ];
        for a in &samples {
            for b in &samples {
                for c in &samples {
                    verify_rig_axioms(a, b, c).unwrap();
                }
            }
        }
    }
}
