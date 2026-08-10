// Portions derived from deep_causality_haft 0.4.2 (the crate this substrate
// replaced at #222), used under the MIT license:
// SPDX-License-Identifier: MIT
// Copyright (c) 2023 - 2026. The DeepCausality Authors.
// Copyright (c) 2026 sustia-llc.

//! The two-variant sum [`Either<L, R>`].
//!
//! This module is private; the type is re-exported through
//! [`crate::endofunctor`].

/// A value of one of two types, `Left(L)` or `Right(R)`.
///
/// Deliberately distinct from `Result<L, R>`, which already means
/// success-or-error: a categorical coproduct is a branch, not a failure. The
/// crate's own use is
/// [`TreeEndo<A>`](crate::free_monad::tree_endo::TreeEndo)'s object map
/// `Type<X> = Either<A, (X, X)>` — leaf label on the left, the two subtree slots
/// on the right.
///
/// Also deliberately distinct from the `either` crate's `Either` that core and
/// applied use in their cospan APIs: this sum is part of the owned endofunctor
/// substrate ([#222](https://github.com/sustia-llc/catgraph/issues/222)
/// decision D-2) — it appears inside GAT projections and `depth`'s pattern
/// matches, and pinning the substrate's shape to a foreign crate's type is
/// what #222 removed. No API takes both; a consumer bridging them matches
/// explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Either<L, R> {
    /// The left variant.
    Left(L),
    /// The right variant.
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Returns `true` if this is a `Left` value.
    #[inline]
    #[must_use]
    pub const fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }

    /// Returns `true` if this is a `Right` value.
    #[inline]
    #[must_use]
    pub const fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    /// Returns the left value if present, consuming `self`.
    #[inline]
    #[must_use]
    pub fn left(self) -> Option<L> {
        match self {
            Either::Left(l) => Some(l),
            Either::Right(_) => None,
        }
    }

    /// Returns the right value if present, consuming `self`.
    #[inline]
    #[must_use]
    pub fn right(self) -> Option<R> {
        match self {
            Either::Left(_) => None,
            Either::Right(r) => Some(r),
        }
    }

    /// Borrows the inner value as `Either<&L, &R>`. The non-consuming
    /// counterpart to [`left`](Self::left) / [`right`](Self::right), for a
    /// caller holding `&Either<L, R>` over a non-`Copy` payload.
    #[inline]
    #[must_use]
    pub const fn as_ref(&self) -> Either<&L, &R> {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(r),
        }
    }

    /// Mutably borrows the inner value as `Either<&mut L, &mut R>`, for
    /// in-place edits through a `&mut Either<L, R>`.
    #[inline]
    #[must_use]
    pub fn as_mut(&mut self) -> Either<&mut L, &mut R> {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(r),
        }
    }
}
