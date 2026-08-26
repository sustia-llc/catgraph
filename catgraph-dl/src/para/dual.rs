//! The forward-mode dual number `a + b·ε`, `ε² = 0` (feature `ad`):
//! `Add`/`Sub`/`Mul`/`Div`/`Neg`/`AddAssign`/`MulAssign`, scalar `Mul<T>`,
//! [`Zero`], [`One`]; derives `Copy`, `Clone`, `PartialEq`, `PartialOrd`,
//! `Debug`; bounds per impl.

use core::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub};

use catgraph_applied::rig::{One, Zero};

/// Dual number `a + b·ε`, `ε² = 0`: evaluating at
/// [`Dual::variable(x₀)`](Dual::variable) `= x₀ + 1·ε` gives `f(x₀)` in
/// [`value`](Dual::value) and `f'(x₀)` in [`derivative`](Dual::derivative).
///
/// # Examples
///
/// ```
/// use catgraph_dl::para::ad::Dual;
///
/// // f(x) = x³ + 2x, evaluated with its derivative at x = 3.
/// let x = Dual::variable(3.0_f64);
/// let y = x * x * x + x + x;
/// assert_eq!(y.value(), 33.0); // 3³ + 2·3
/// assert_eq!(y.derivative(), 29.0); // 3·3² + 2
/// ```
///
/// # Serde (features `serde` and `ad`)
///
/// Round-trips as its two public components; a loaded `du` is whatever the
/// document says, not a seeded [`variable`](Dual::variable). The wire shape
/// carries no type tag and `serde_json` writes non-finite components as
/// `null`, which does not read back.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dual<T> {
    /// The real part `a` — the function value.
    pub re: T,
    /// The infinitesimal coefficient `b` — the derivative in the `ε` channel.
    pub du: T,
}

impl<T> Dual<T> {
    /// Construct `re + du·ε` from both components.
    #[inline]
    pub const fn new(re: T, du: T) -> Self {
        Self { re, du }
    }
}

impl<T: Copy> Dual<T> {
    /// The real part `a` — the function value `f(x₀)`.
    #[inline]
    pub const fn value(&self) -> T {
        self.re
    }

    /// The infinitesimal coefficient `b` — the derivative `f'(x₀)`.
    #[inline]
    pub const fn derivative(&self) -> T {
        self.du
    }
}

impl<T: Zero> Dual<T> {
    /// Construct the constant `re + 0·ε` — a value with zero derivative.
    #[inline]
    pub fn constant(re: T) -> Self {
        Self {
            re,
            du: <T as Zero>::zero(),
        }
    }
}

impl<T: Zero + One> Dual<T> {
    /// Construct the differentiation seed `re + 1·ε` — the independent variable.
    #[inline]
    pub fn variable(re: T) -> Self {
        Self {
            re,
            du: <T as One>::one(),
        }
    }
}

/// `(a + bε) + (c + dε) = (a + c) + (b + d)ε`.
impl<T: Add<Output = T>> Add for Dual<T> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.re + rhs.re, self.du + rhs.du)
    }
}

impl<T: AddAssign> AddAssign for Dual<T> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.du += rhs.du;
    }
}

/// `(a + bε) − (c + dε) = (a − c) + (b − d)ε`.
impl<T: Sub<Output = T>> Sub for Dual<T> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.re - rhs.re, self.du - rhs.du)
    }
}

/// **The chain rule.** `(a + bε)(c + dε) = ac + (ad + bc)ε`, the `ε²` term
/// vanishing by `ε² = 0`.
impl<T: Copy + Add<Output = T> + Mul<Output = T>> Mul for Dual<T> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self::new(self.re * rhs.re, self.re * rhs.du + self.du * rhs.re)
    }
}

impl<T: Copy + Add<Output = T> + Mul<Output = T>> MulAssign for Dual<T> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

/// Scalar action `(a + bε)·s = (a·s) + (b·s)ε` — the `Module<T>` action, which
/// scales both channels because `s` carries no derivative of its own.
impl<T: Copy + Mul<Output = T>> Mul<T> for Dual<T> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: T) -> Self {
        Self::new(self.re * rhs, self.du * rhs)
    }
}

/// `−(a + bε) = (−a) + (−b)ε`.
impl<T: Neg<Output = T>> Neg for Dual<T> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.re, -self.du)
    }
}

/// **The quotient rule**, `(a + bε)/(c + dε) = a/c + ((bc − ad)/c²)ε`, defined
/// only where the divisor's real part `c` is invertible.
///
/// There is deliberately no `DivAssign` twin: `ε` is a zero divisor (`ε·ε = 0`),
/// so a dual has no total multiplicative inverse and must not be mistaken for a
/// field element. Dividing by a dual whose real part is zero is the caller's
/// error, and produces whatever `T`'s own division produces (for `f64`, an
/// infinity or NaN) — this type adds no guard of its own.
impl<T: Copy + Sub<Output = T> + Mul<Output = T> + Div<Output = T>> Div for Dual<T> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        Self::new(
            self.re / rhs.re,
            (self.du * rhs.re - self.re * rhs.du) / (rhs.re * rhs.re),
        )
    }
}

/// The additive identity `0 + 0·ε`.
impl<T: Copy + Zero> Zero for Dual<T> {
    #[inline]
    fn zero() -> Self {
        Self::new(<T as Zero>::zero(), <T as Zero>::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.re.is_zero() && self.du.is_zero()
    }
}

/// The multiplicative identity `1 + 0·ε` — a *constant* one, carrying no
/// derivative, which is what makes `1 · v = v` hold in the `ε` channel too.
impl<T: Copy + Zero + One + Add<Output = T> + Mul<Output = T>> One for Dual<T> {
    #[inline]
    fn one() -> Self {
        Self::new(<T as One>::one(), <T as Zero>::zero())
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.re.is_one() && self.du.is_zero()
    }
}
