//! Canonical byte encoding and the fingerprint computed from it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// A value's canonical byte encoding, appended to `out`.
///
/// Byte layout of the implementations in this module:
///
/// - `u8 u16 u32 u64 u128 i8 i16 i32 i64 i128`: fixed-width little-endian of
///   their own width.
/// - `usize` as `u64` little-endian, `isize` as `i64` little-endian.
/// - `bool`: one byte, `0` or `1`. `char`: its scalar value as `u32`
///   little-endian.
/// - `str`, `String`: the byte length as `u64` little-endian, then the UTF-8
///   bytes.
/// - `[T]`, `Vec<T>`, `[T; N]`, `VecDeque<T>`: the element count as `u64`
///   little-endian, then each element's encoding in order.
/// - `Option<T>`: the byte `0` for `None`, or the byte `1` then the value.
/// - `()` and tuples of arity 1 to 6: the concatenation of the fields'
///   encodings, in field order (`()` encodes to no bytes).
/// - `&T`, `Box<T>`: the encoding of the `T`.
/// - `BTreeSet<T>`, `BTreeMap<K, V>`: the entry count as `u64` little-endian,
///   then each entry in iteration order (a map entry is its key then its
///   value).
///
/// `HashMap` and `HashSet` have no implementation; a type holding one encodes
/// its entries in a sorted order in its own implementation.
pub trait CanonicalEncode {
    /// Append this value's canonical encoding to `out`.
    fn encode_canonical(&self, out: &mut Vec<u8>);
}

/// The first 8 bytes of the BLAKE3 hash of `value`'s canonical encoding, read
/// as a little-endian `u64`.
#[must_use]
pub fn canonical_fingerprint<T: CanonicalEncode + ?Sized>(value: &T) -> u64 {
    let mut bytes = Vec::new();
    value.encode_canonical(&mut bytes);
    let hash = blake3::hash(&bytes);
    let mut head = [0u8; 8];
    head.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(head)
}

/// Append `len` as a `u64` little-endian length prefix.
fn encode_len(len: usize, out: &mut Vec<u8>) {
    (len as u64).encode_canonical(out);
}

macro_rules! impl_fixed_width {
    ($($t:ty),+ $(,)?) => {
        $(
            impl CanonicalEncode for $t {
                fn encode_canonical(&self, out: &mut Vec<u8>) {
                    out.extend_from_slice(&self.to_le_bytes());
                }
            }
        )+
    };
}

impl_fixed_width!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl CanonicalEncode for usize {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        (*self as u64).encode_canonical(out);
    }
}

impl CanonicalEncode for isize {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        (*self as i64).encode_canonical(out);
    }
}

impl CanonicalEncode for bool {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
}

impl CanonicalEncode for char {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        u32::from(*self).encode_canonical(out);
    }
}

impl CanonicalEncode for str {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_len(self.len(), out);
        out.extend_from_slice(self.as_bytes());
    }
}

impl CanonicalEncode for String {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.as_str().encode_canonical(out);
    }
}

impl<T: CanonicalEncode> CanonicalEncode for [T] {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_len(self.len(), out);
        for item in self {
            item.encode_canonical(out);
        }
    }
}

impl<T: CanonicalEncode> CanonicalEncode for Vec<T> {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.as_slice().encode_canonical(out);
    }
}

impl<T: CanonicalEncode, const N: usize> CanonicalEncode for [T; N] {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        self.as_slice().encode_canonical(out);
    }
}

impl<T: CanonicalEncode> CanonicalEncode for VecDeque<T> {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_len(self.len(), out);
        for item in self {
            item.encode_canonical(out);
        }
    }
}

impl<T: CanonicalEncode> CanonicalEncode for Option<T> {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        match self {
            None => out.push(0),
            Some(value) => {
                out.push(1);
                value.encode_canonical(out);
            }
        }
    }
}

impl CanonicalEncode for () {
    fn encode_canonical(&self, _out: &mut Vec<u8>) {}
}

macro_rules! impl_tuple {
    ($($name:ident),+) => {
        impl<$($name: CanonicalEncode),+> CanonicalEncode for ($($name,)+) {
            #[allow(non_snake_case)]
            fn encode_canonical(&self, out: &mut Vec<u8>) {
                let ($($name,)+) = self;
                $($name.encode_canonical(out);)+
            }
        }
    };
}

impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);

impl<T: CanonicalEncode + ?Sized> CanonicalEncode for &T {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        (**self).encode_canonical(out);
    }
}

impl<T: CanonicalEncode + ?Sized> CanonicalEncode for Box<T> {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        (**self).encode_canonical(out);
    }
}

impl<T: CanonicalEncode> CanonicalEncode for BTreeSet<T> {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_len(self.len(), out);
        for item in self {
            item.encode_canonical(out);
        }
    }
}

impl<K: CanonicalEncode, V: CanonicalEncode> CanonicalEncode for BTreeMap<K, V> {
    fn encode_canonical(&self, out: &mut Vec<u8>) {
        encode_len(self.len(), out);
        for (key, value) in self {
            key.encode_canonical(out);
            value.encode_canonical(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes<T: CanonicalEncode + ?Sized>(value: &T) -> Vec<u8> {
        let mut out = Vec::new();
        value.encode_canonical(&mut out);
        out
    }

    #[test]
    fn layout_bytes() {
        let cases: Vec<(&str, Vec<u8>, Vec<u8>)> = vec![
            ("0x0102u16", bytes(&0x0102u16), vec![0x02, 0x01]),
            ("-1i8", bytes(&-1i8), vec![0xff]),
            ("1usize", bytes(&1usize), vec![1, 0, 0, 0, 0, 0, 0, 0]),
            ("-1isize", bytes(&-1isize), vec![0xff; 8]),
            ("true", bytes(&true), vec![1]),
            ("'a'", bytes(&'a'), vec![0x61, 0, 0, 0]),
            (
                "\"ab\"",
                bytes("ab"),
                vec![2, 0, 0, 0, 0, 0, 0, 0, b'a', b'b'],
            ),
            (
                "[7u8; 2]",
                bytes(&[7u8; 2]),
                vec![2, 0, 0, 0, 0, 0, 0, 0, 7, 7],
            ),
            ("None::<u8>", bytes(&None::<u8>), vec![0]),
            ("Some(5u8)", bytes(&Some(5u8)), vec![1, 5]),
            ("()", bytes(&()), vec![]),
            ("(1u8, 2u8)", bytes(&(1u8, 2u8)), vec![1, 2]),
            (
                "BTreeMap {1u8: 2u8}",
                bytes(&BTreeMap::from([(1u8, 2u8)])),
                vec![1, 0, 0, 0, 0, 0, 0, 0, 1, 2],
            ),
        ];
        for (name, observed, expected) in cases {
            assert_eq!(
                observed, expected,
                "{name}: observed {observed:?}, expected {expected:?}"
            );
        }
    }
}
