//! Integer-exact ring `Z(BigInt)` — the substrate for Leinster 2008 Cor 1.5
//! integer-exact Möbius inversion in catgraph-magnitude §1.17.
//!
//! `Z` wraps [`num::BigInt`] for unbounded integer arithmetic. It picks up the
//! [`crate::rig::Rig`] blanket impl via `Clone + PartialEq + Zero + One + Add +
//! Mul`, and [`crate::integer::ZAlgebra`] via `Neg + Sub + from_i64` plus the
//! crate-private `Sealed` seal in `crate::integer::private`.

use crate::integer::{ZAlgebra, private::Sealed};
// catgraph's `Zero`/`One` for `Z` itself; `num`'s, aliased, drive the inner BigInt.
use crate::rig::{One, Zero};
use catgraph::CanonicalEncode;
use num::bigint::Sign;
use num::{BigInt, One as NumOne, Zero as NumZero};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Mul, Neg, Sub};

/// Integer-exact ring over [`num::BigInt`], satisfying the [`crate::rig::Rig`]
/// blanket impl and the sealed [`ZAlgebra`] trait.
///
/// # Examples
///
/// ```
/// use catgraph_applied::ZAlgebra;
/// use catgraph_applied::z::Z;
///
/// let a = Z::from_i64(3);
/// let b = Z::from_i64(5);
/// assert_eq!(a.clone() + b.clone(), Z::from_i64(8));
/// assert_eq!(a * b, Z::from_i64(15));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Z(pub BigInt);

impl Z {
    /// Construct from an existing [`BigInt`].
    #[must_use]
    pub fn new(value: BigInt) -> Self {
        Z(value)
    }
}

impl Zero for Z {
    fn zero() -> Self {
        Z(<BigInt as NumZero>::zero())
    }
    fn is_zero(&self) -> bool {
        NumZero::is_zero(&self.0)
    }
}

impl One for Z {
    fn one() -> Self {
        Z(<BigInt as NumOne>::one())
    }
    fn is_one(&self) -> bool {
        NumOne::is_one(&self.0)
    }
}

impl Add for Z {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Z(self.0 + other.0)
    }
}

impl Mul for Z {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Z(self.0 * other.0)
    }
}

impl Neg for Z {
    type Output = Self;
    fn neg(self) -> Self {
        Z(-self.0)
    }
}

impl Sub for Z {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Z(self.0 - other.0)
    }
}

impl From<i64> for Z {
    fn from(n: i64) -> Self {
        Z(BigInt::from(n))
    }
}

// Equivalent to `#[derive(Hash)]`; written out to keep the `Eq`/`Hash` contract
// visible at this site.
impl Hash for Z {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// One sign byte (`0` negative, `1` zero, `2` positive), then the magnitude's
/// little-endian bytes from `BigInt::to_bytes_le` as a length-prefixed
/// `Vec<u8>`.
impl CanonicalEncode for Z {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        let (sign, magnitude) = self.0.to_bytes_le();
        out.push(match sign {
            Sign::Minus => 0,
            Sign::NoSign => 1,
            Sign::Plus => 2,
        });
        magnitude.encode_canonical(out);
    }
}

impl ZAlgebra for Z {
    fn from_i64(n: i64) -> Self {
        Z::from(n)
    }
}

// The gated `Sealed` impl that licenses `Z`'s `ZAlgebra` impl above.
impl Sealed for Z {}
