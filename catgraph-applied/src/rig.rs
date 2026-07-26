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
//!   magnitude homology and as the Lawvere-metric enrichment base;
//!   `d = -ln π` converts `UnitInterval → Tropical`.
//! - [`F64Rig`] — plain real rig for `Mat(R)` and `SFG_R` demos.
//! - [`Checked<T>`] — a poison-on-overflow wrapper over a primitive integer
//!   rig; see the overflow policy below.
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
//! `Hash` via `f64::to_bits()`. This is required by the
//! [`PropSignature`](crate::prop::PropSignature) `Eq + Hash` supertrait bounds:
//! [`SfgGenerator<R>`](crate::sfg::SfgGenerator) now requires `R: Eq + Hash`,
//! and the [`prop::presentation::kb::CongruenceClosure`](crate::prop::presentation::kb::CongruenceClosure)
//! term graph uses `SfgGenerator<R>` as a `HashMap` key. Hashing is bit-exact
//! EXCEPT `-0.0` normalizes to `0.0` so that hashing agrees with the derived
//! IEEE `PartialEq` (under which `-0.0 == 0.0`) — required by the `Eq`/`Hash`
//! contract, and the fix for #58 (a `-0.0` and `0.0` splitting a congruence
//! class). NaN caveats inherit from `PartialEq`: a NaN payload would be
//! non-reflexive; callers should not construct NaN values in these newtypes
//! (the [`UnitInterval::new`] validator already rejects them).
//!
//! # Overflow policy (workspace policy of record, #88)
//!
//! Rig arithmetic anywhere in the workspace — [`MatR::matmul`](crate::mat::MatR),
//! [`sfg_to_mat`](crate::sfg_to_mat::sfg_to_mat),
//! [`MatrixNFFunctor`](crate::prop::presentation::functorial::MatrixNFFunctor),
//! and catgraph-syntax's `eval` / `SfgModel` — is *exactly* `R`'s own
//! [`Add`] and [`Mul`]. No layer above `R` inserts a check, so the overflow
//! behaviour of a computation is decided entirely by the rig you instantiate.
//! This section is the single place that behaviour is specified; downstream
//! docs cite it rather than restating it.
//!
//! | Rig family | Overflow behaviour |
//! | --- | --- |
//! | [`BoolRig`], [`UnitInterval`] | No integer overflow is possible — `∨`/`∧` and `max`/`·` on `[0,1]` are closed. |
//! | [`Tropical`], [`F64Rig`] | Float families; no integer overflow. They keep IEEE-754 `inf`/NaN semantics, documented honestly rather than papered over (the #58 posture). |
//! | [`Z`](crate::z::Z) / other BigInt-valued rigs | **Exact.** Arbitrary-precision integers cannot overflow; this is the zero-caveat answer and the recommended coefficient ring whenever counts are unbounded. |
//! | Primitive integers (`i64`, `u32`, …) via the blanket impl | **Inherited from `R`**, i.e. Rust's profile-dependent behaviour: a debug build panics on overflow, a release build wraps silently. |
//! | [`Checked<T>`] over a primitive integer | **Opt-in detection.** Overflow produces the absorbing sentinel [`Checked::Poison`], which propagates to the result where a caller can see it. |
//!
//! **Saturating arithmetic is rejected**, and is not offered even as an opt-in.
//! Clamping at `T::MAX` yields a silently *wrong* value that is
//! indistinguishable from a legitimate one, and it breaks distributivity
//! (`a·(b+c) ≠ a·b + a·c` once any operand saturates) — so a saturating rig
//! would not be a rig at all, while also destroying the evidence that anything
//! went wrong. [`Checked<T>`] loses strictly less (one axiom, visibly).

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

// `Eq + Hash` for use as a `HashMap` key in the congruence-closure term graph
// (`PropSignature: Eq + Hash` bound). Hashing is bit-exact via `to_bits()`
// EXCEPT `-0.0` normalizes to `0.0`, so hashing agrees with the derived IEEE
// `PartialEq` (under which `-0.0 == 0.0`) — required by the `Eq`/`Hash`
// contract. NaN caveats inherit from `PartialEq`; [`UnitInterval::new`] rejects
// NaN on construction.
impl Eq for UnitInterval {}
impl std::hash::Hash for UnitInterval {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // -0.0 == 0.0 under the derived PartialEq, so they must hash alike
        // (Eq/Hash contract). All other values keep bit-exact hashing.
        let bits = if self.0 == 0.0 {
            0.0f64.to_bits()
        } else {
            self.0.to_bits()
        };
        bits.hash(state);
    }
}

/// Tropical (min-plus) semiring over `[0, ∞]`, with `+∞` as the additive
/// zero and `0` as the multiplicative unit. Represents Lawvere metric-space
/// distances directly; use as the enrichment base for
/// `LawvereMetricSpace<T>`.
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

// `Eq + Hash` for use as a `HashMap` key in the congruence-closure term graph
// (`PropSignature: Eq + Hash` bound). Hashing is bit-exact via `to_bits()`
// EXCEPT `-0.0` normalizes to `0.0`, so hashing agrees with the derived IEEE
// `PartialEq` (under which `-0.0 == 0.0`) — required by the `Eq`/`Hash`
// contract. NaN caveats inherit from `PartialEq`.
impl Eq for Tropical {}
impl std::hash::Hash for Tropical {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // -0.0 == 0.0 under the derived PartialEq, so they must hash alike
        // (Eq/Hash contract). All other values keep bit-exact hashing.
        let bits = if self.0 == 0.0 {
            0.0f64.to_bits()
        } else {
            self.0.to_bits()
        };
        bits.hash(state);
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

// Ring + field operations on `F64Rig`, a prerequisite for the
// catgraph-magnitude Möbius path. `F64Rig` is the only `Ring + Div`-bounded
// rig in the workspace; `mobius_function::<F64Rig>` needs all
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
    /// the rig.
    ///
    /// Note: `i64` values exceeding 2^53 lose precision in `f64`; the SNF
    /// call sites use only `±1`, so this is not a concern in practice.
    #[allow(clippy::cast_precision_loss)]
    fn from(n: i64) -> Self {
        F64Rig(n as f64)
    }
}

// `Eq + Hash` for use as a `HashMap` key in the congruence-closure term graph
// (`PropSignature: Eq + Hash` bound). Hashing is bit-exact via `to_bits()`
// EXCEPT `-0.0` normalizes to `0.0`, so hashing agrees with the derived IEEE
// `PartialEq` (under which `-0.0 == 0.0`) — required by the `Eq`/`Hash`
// contract. NaN caveats inherit from `PartialEq`.
impl Eq for F64Rig {}
impl std::hash::Hash for F64Rig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // -0.0 == 0.0 under the derived PartialEq, so they must hash alike
        // (Eq/Hash contract). All other values keep bit-exact hashing.
        let bits = if self.0 == 0.0 {
            0.0f64.to_bits()
        } else {
            self.0.to_bits()
        };
        bits.hash(state);
    }
}

/// Checked integer arithmetic, the substrate [`Checked<T>`] detects overflow
/// with.
///
/// Implemented for every primitive integer type by delegating to the std
/// inherent `checked_add` / `checked_mul`; a `None` return **is** the overflow
/// signal. The trait is catgraph-local rather than a `num-traits` import
/// because the workspace sources only `Zero` / `One` from an external numeric
/// crate (`deep_causality_num`) — see the crate-root substrate rules.
///
/// Downstream types may implement it to make themselves wrappable in
/// [`Checked`]; the contract is that `None` means "the exact result is not
/// representable in `Self`", never "the operation is undefined for other
/// reasons".
pub trait CheckedOps: Sized {
    /// Add, returning `None` when the exact sum is not representable.
    fn checked_add(&self, rhs: &Self) -> Option<Self>;
    /// Multiply, returning `None` when the exact product is not representable.
    fn checked_mul(&self, rhs: &Self) -> Option<Self>;
}

macro_rules! impl_checked_ops_for_primitive_int {
    ($($t:ty),+ $(,)?) => {
        $(
            impl CheckedOps for $t {
                fn checked_add(&self, rhs: &Self) -> Option<Self> {
                    <$t>::checked_add(*self, *rhs)
                }
                fn checked_mul(&self, rhs: &Self) -> Option<Self> {
                    <$t>::checked_mul(*self, *rhs)
                }
            }
        )+
    };
}

impl_checked_ops_for_primitive_int!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

/// A rig element that remembers whether an overflow has happened: either a
/// [`Value`](Checked::Value) or the absorbing poison sentinel
/// [`Poison`](Checked::Poison), written `⊥`.
///
/// `Checked<T>` is the workspace's **opt-in overflow-detection** rig (#88; the
/// policy of record is in this module's docs). Wrapping a primitive integer
/// rig in it makes overflow *visible in the result* instead of profile-
/// dependent: `Checked::new(i64::MAX) + Checked::new(1)` is `⊥`, in both debug
/// and release builds, and that `⊥` survives every subsequent operation all
/// the way out to the caller, who can test it with [`is_poisoned`].
///
/// It satisfies the [`Rig`] blanket impl (`Clone + PartialEq + Zero + One +
/// Add + Mul`), so `Checked<i64>` drops into
/// [`MatR`](crate::mat::MatR), [`sfg_to_mat`](crate::sfg_to_mat::sfg_to_mat),
/// [`MatrixNFFunctor`](crate::prop::presentation::functorial::MatrixNFFunctor),
/// and catgraph-syntax's `eval` / `SfgModel` with no change to those call
/// sites.
///
/// # Semantics
///
/// `⊥` is **fully absorbing**: if either operand is `⊥` the result is `⊥`, and
/// `(Value a) ⊕ (Value b)` is `⊥` exactly when the checked primitive operation
/// reports that the exact result is not representable.
///
/// Full absorption deliberately includes **`⊥ × 0 = ⊥`**. Special-casing zero
/// to give `⊥ × 0 = 0` would erase a detected overflow — a caller could
/// multiply a poisoned intermediate by zero and get a clean-looking answer
/// back — and it would break distributivity in any ring extension, where
/// `⊥ × (1 + (−1))` must agree with `⊥ × 1 + ⊥ × (−1)`.
///
/// # Laws
///
/// Full absorption is what keeps the algebra well behaved: associativity,
/// commutativity, and both distributive laws hold on all of `Checked<T>`,
/// because every equation with a `⊥` anywhere in it evaluates to `⊥` on both
/// sides.
///
/// **Exactly one axiom is lost: the absorbing zero.**
/// [`verify_rig_axioms`]'s axiom 8 requires `a * 0 == 0`, and that fails for
/// `a = ⊥` — the one poisoned cone of the value space. Everywhere else
/// (`Value(_)` samples only) all eight axioms hold, and `verify_rig_axioms`
/// passes.
///
/// The honest reading, mirroring [`F64Rig`]'s NaN posture (#58), is: **a rig on
/// the unpoisoned subset, with `⊥` adjoined as an absorbing sentinel.** That is
/// strictly less loss than saturating arithmetic, which gives wrong values
/// *and* breaks distributivity (see the module-level overflow policy).
///
/// # Text form
///
/// [`Display`](std::fmt::Display) renders `Poison` as the single character `⊥`
/// and [`FromStr`](std::str::FromStr) reads it back, so a `Checked<i64>` can be
/// a `SfgGenerator::Scalar` payload in catgraph-syntax's `scalar:<r>` token
/// (which must stay one whitespace- and paren-free lexical atom).
///
/// [`is_poisoned`]: Checked::is_poisoned
///
/// # Examples
///
/// ```
/// use catgraph_applied::rig::Checked;
/// use deep_causality_num::Zero;
///
/// let big = Checked::new(4_000_000_000_i64);
/// let overflowed = big * big;
/// assert!(overflowed.is_poisoned());
///
/// // ⊥ is fully absorbing — multiplying by zero does NOT clear it.
/// assert!((overflowed * Checked::zero()).is_poisoned());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Checked<T> {
    /// A representable value — no overflow has occurred on any path into it.
    Value(T),
    /// `⊥`: an overflow was detected somewhere upstream.
    Poison,
}

impl<T> Checked<T> {
    /// Wrap a value. Never poisoned.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Checked::Value(value)
    }

    /// `true` if this is `⊥`, i.e. an overflow was detected upstream.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        matches!(self, Checked::Poison)
    }

    /// Borrow the wrapped value, or `None` if poisoned.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Checked::Value(v) => Some(v),
            Checked::Poison => None,
        }
    }

    /// Unwrap to the value, or `None` if poisoned.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        match self {
            Checked::Value(v) => Some(v),
            Checked::Poison => None,
        }
    }
}

impl<T> From<T> for Checked<T> {
    fn from(value: T) -> Self {
        Checked::Value(value)
    }
}

impl<T: CheckedOps> Add for Checked<T> {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Checked::Value(a), Checked::Value(b)) => {
                a.checked_add(&b).map_or(Checked::Poison, Checked::Value)
            }
            // Full absorption: a detected overflow is never recoverable.
            _ => Checked::Poison,
        }
    }
}

impl<T: CheckedOps> Mul for Checked<T> {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        match (self, other) {
            (Checked::Value(a), Checked::Value(b)) => {
                a.checked_mul(&b).map_or(Checked::Poison, Checked::Value)
            }
            // Full absorption — including `⊥ * 0 = ⊥`. See the type docs: the
            // zero special-case would erase a detected overflow.
            _ => Checked::Poison,
        }
    }
}

impl<T: CheckedOps + Zero> Zero for Checked<T> {
    fn zero() -> Self {
        Checked::Value(T::zero())
    }
    /// `false` for `⊥` — a poisoned value is not the additive identity.
    fn is_zero(&self) -> bool {
        matches!(self, Checked::Value(v) if v.is_zero())
    }
}

impl<T: CheckedOps + One> One for Checked<T> {
    fn one() -> Self {
        Checked::Value(T::one())
    }
    /// `false` for `⊥` — a poisoned value is not the multiplicative identity.
    fn is_one(&self) -> bool {
        matches!(self, Checked::Value(v) if v.is_one())
    }
}

/// The single-character lexeme for [`Checked::Poison`]. Kept paren-, space-
/// and metacharacter-free so `scalar:⊥` stays one lexical atom under
/// catgraph-syntax's `GeneratorSyntax` token contract.
const POISON_TOKEN: &str = "⊥";

impl<T: std::fmt::Display> std::fmt::Display for Checked<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Checked::Value(v) => write!(f, "{v}"),
            Checked::Poison => f.write_str(POISON_TOKEN),
        }
    }
}

impl<T: std::str::FromStr> std::str::FromStr for Checked<T> {
    /// `T`'s own parse error, unchanged. No dedicated error type is needed:
    /// the `⊥` case is a *success*, not a failure, so nothing but `T`'s
    /// failures can be reported. Poison is strictly the overflow sentinel and
    /// is never used as a dumping ground for parse errors.
    type Err = T::Err;

    /// Exactly `"⊥"` parses to [`Checked::Poison`]; every other input is
    /// delegated to `T::from_str` and wrapped in [`Checked::Value`].
    ///
    /// The `⊥` arm is checked *first*, so a `T` whose own `FromStr` happens to
    /// accept `"⊥"` is shadowed for that one input. No primitive integer does.
    ///
    /// Round-trip law: `s.parse::<Checked<T>>()` recovers `x` from
    /// `x.to_string()` for both variants whenever `T` itself round-trips.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == POISON_TOKEN {
            return Ok(Checked::Poison);
        }
        s.parse::<T>().map(Checked::Value)
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

    // #58: `-0.0` and `0.0` are `==` under the derived IEEE `PartialEq`, so the
    // `Eq`/`Hash` contract requires them to hash alike. Before the fix they
    // hashed differently (`(-0.0).to_bits() != (0.0).to_bits()`), splitting
    // congruence classes keyed on these rigs. Each rig asserts both halves:
    // the `==` (documents the contract) and the equal hashes (the fix).
    // Both values are hashed with the SAME `BuildHasher` — a per-call
    // `RandomState` reseeds and would make even identical values disagree.
    fn hashes_agree<T: std::hash::Hash>(a: &T, b: &T) -> bool {
        use std::hash::{BuildHasher, RandomState};
        let state = RandomState::new();
        state.hash_one(a) == state.hash_one(b)
    }

    #[test]
    fn f64rig_signed_zero_eq_and_hash_agree() {
        assert_eq!(F64Rig(0.0), F64Rig(-0.0));
        assert!(hashes_agree(&F64Rig(0.0), &F64Rig(-0.0)));
    }

    #[test]
    fn tropical_signed_zero_eq_and_hash_agree() {
        assert_eq!(Tropical(0.0), Tropical(-0.0));
        assert!(hashes_agree(&Tropical(0.0), &Tropical(-0.0)));
    }

    #[test]
    fn unit_interval_signed_zero_eq_and_hash_agree() {
        // `-0.0 >= 0.0`, so `UnitInterval::new(-0.0)` passes the [0, 1] check.
        let neg_zero = UnitInterval::new(-0.0).unwrap();
        let pos_zero = UnitInterval::new(0.0).unwrap();
        assert_eq!(pos_zero, neg_zero);
        assert!(hashes_agree(&pos_zero, &neg_zero));
    }

    // ---- `Checked<T>` (#88) --------------------------------------------------

    /// `4e9 * 4e9 = 1.6e19` exceeds `i64::MAX ≈ 9.22e18` — the issue's own
    /// overflow witness, which a release build would silently wrap.
    const BIG: i64 = 4_000_000_000;

    #[test]
    fn checked_add_overflow_poisons() {
        let sum = Checked::new(i64::MAX) + Checked::new(1_i64);
        assert!(sum.is_poisoned());
        // The non-overflowing neighbour stays a value.
        assert_eq!(
            Checked::new(i64::MAX - 1) + Checked::new(1_i64),
            Checked::new(i64::MAX)
        );
    }

    #[test]
    fn checked_mul_overflow_poisons() {
        let product = Checked::new(BIG) * Checked::new(BIG);
        assert!(product.is_poisoned());
        assert_eq!(product.value(), None);
        assert_eq!(product.into_value(), None);
    }

    #[test]
    fn poison_is_fully_absorbing() {
        let poison = Checked::<i64>::Poison;
        let x = Checked::new(7_i64);

        assert!((poison + x).is_poisoned());
        assert!((x + poison).is_poisoned());
        assert!((poison * x).is_poisoned());
        assert!((x * poison).is_poisoned());

        // The pinned law caveat: `⊥ × 0 = ⊥`, NOT `0`. A zero special-case
        // would erase the detected overflow.
        assert!((poison * Checked::<i64>::zero()).is_poisoned());
        assert!((Checked::<i64>::zero() * poison).is_poisoned());
    }

    #[test]
    fn checked_zero_and_one_are_unpoisoned() {
        assert_eq!(Checked::<i64>::zero(), Checked::new(0_i64));
        assert_eq!(Checked::<i64>::one(), Checked::new(1_i64));
        assert!(!Checked::<i64>::zero().is_poisoned());
        assert!(!Checked::<i64>::one().is_poisoned());

        assert!(Checked::<i64>::zero().is_zero());
        assert!(Checked::<i64>::one().is_one());
        // Poison is neither identity.
        assert!(!Checked::<i64>::Poison.is_zero());
        assert!(!Checked::<i64>::Poison.is_one());
    }

    #[test]
    fn verify_axioms_checked_i64_unpoisoned_sample() {
        // All eight axioms hold on the unpoisoned subset. Values are small
        // enough that no intermediate product overflows into `⊥`.
        let samples = [
            Checked::new(0_i64),
            Checked::new(1_i64),
            Checked::new(-3_i64),
            Checked::new(7_i64),
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
    fn verify_axioms_checked_i64_poisoned_loses_exactly_absorbing_zero() {
        // The documented single lost axiom: `⊥ * 0 == 0` is false. Every
        // earlier axiom passes (full absorption makes both sides `⊥`), so the
        // first — and only — violation `verify_rig_axioms` reports is #8.
        let err = verify_rig_axioms(
            &Checked::<i64>::Poison,
            &Checked::new(2_i64),
            &Checked::new(3_i64),
        )
        .expect_err("a poisoned sample must violate the absorbing-zero axiom");
        match err {
            catgraph::errors::CatgraphError::RigAxiomViolation { axiom, .. } => {
                assert_eq!(axiom, "absorbing zero");
            }
            other => panic!("expected RigAxiomViolation, got {other:?}"),
        }
    }

    #[test]
    fn checked_display_from_str_round_trip() {
        let value = Checked::new(-42_i64);
        assert_eq!(value.to_string(), "-42");
        assert_eq!("-42".parse::<Checked<i64>>().unwrap(), value);

        let poison = Checked::<i64>::Poison;
        assert_eq!(poison.to_string(), "⊥");
        assert_eq!("⊥".parse::<Checked<i64>>().unwrap(), poison);

        // A parse failure stays an error — poison is the overflow sentinel
        // only, never a dumping ground for unparseable input.
        assert!("not-a-number".parse::<Checked<i64>>().is_err());
    }

    #[test]
    fn checked_poison_propagates_through_matmul() {
        use crate::mat::MatR;

        // Only the (0, 0) dot product overflows: BIG * BIG.
        let big = Checked::new(BIG);
        let zero = Checked::<i64>::zero();
        let one = Checked::<i64>::one();
        let m = MatR::new(2, 2, vec![vec![big, zero], vec![zero, one]]).unwrap();

        let product = m.matmul(&m).unwrap();
        let entries = product.entries();
        assert!(entries[0][0].is_poisoned());
        // Untouched entries are unaffected — poison is per-value, not per-matrix.
        assert_eq!(entries[0][1], zero);
        assert_eq!(entries[1][0], zero);
        assert_eq!(entries[1][1], one);
    }
}
