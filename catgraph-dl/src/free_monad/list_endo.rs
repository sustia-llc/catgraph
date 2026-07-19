//! The list endofunctor `ListEndo<A> : X ↦ 1 + A × X`.
//!
//! CDL Example B.19. The free monad on this endofunctor is, up to
//! isomorphism, the list type with an arbitrary terminator:
//!
//! ```text
//! FreeMnd(1 + A × −)(Z) ≅ List_{Z+1}(A)
//! ```
//!
//! In Rust we encode `1 + A × X` as `Option<(A, X)>`: `None` is the unit
//! summand `1` (the cons-list `Nil`), `Some((a, x))` is the product
//! summand `A × X` (the cons-cell `Cons(a, x)`). With `Z = ()` the encoding
//! collapses to `Free<ListEndo<A>, ()> ≅ Vec<A>`; with general `Z` the
//! bijection is to `(Vec<A>, Z)` — the list is built up from `A`s until a
//! terminator `Z` is reached.
//!
//! ## Bijection
//!
//! The two helpers below witness the iso:
//!
//! - [`free_mnd_to_vec`] — destruct: walk down the cons-cells, collecting
//!   the `A` values; when the terminator `Pure(z)` is reached, return
//!   `(items, z)`.
//! - [`vec_to_free_mnd`] — construct: build the cons-cell tower right-to-
//!   left, with the supplied `terminator` at the deepest `Pure`.
//!
//! Properties (verified in `tests/free_monad_bijections.rs`):
//!
//! - `free_mnd_to_vec(vec_to_free_mnd(items, z)) = (items, z)`.
//! - `vec_to_free_mnd(free_mnd_to_vec(t)) = t` (where `t :
//!   Free<ListEndo<A>, Z>`).
//!
//! ## Iteration discipline
//!
//! `free_mnd_to_vec` walks the input loop-style with an explicit
//! `current` rebound on each iteration. Recursive walks of `Free` would
//! consume O(n) stack per layer; tests probe up to ~100-element vectors
//! and would blow the default test stack on a recursive walk.

use core::marker::PhantomData;

use crate::container::Container;
use crate::endofunctor::{DebugFunctor, EqFunctor, Free, Functor, HKT, NoConstraint, Satisfies};

/// The endofunctor `1 + A × −` for a fixed alphabet `A`.
///
/// The `Type<X>` projection is `Option<(A, X)>` — `None` for the unit
/// summand, `Some((a, x))` for the product summand.
///
/// CDL Example B.19. The free monad on this endofunctor is `List_{Z+1}(A)`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ListEndo<A>(PhantomData<A>);

impl<A> ListEndo<A> {
    /// Construct a fresh `ListEndo<A>` type witness. Zero-sized.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<A> HKT for ListEndo<A> {
    type Constraint = NoConstraint;
    type Type<X> = Option<(A, X)>;
}

impl<A> Functor<Self> for ListEndo<A> {
    fn fmap<X, Y, Func>(fx: Option<(A, X)>, mut f: Func) -> Option<(A, Y)>
    where
        X: Satisfies<NoConstraint>,
        Y: Satisfies<NoConstraint>,
        Func: FnMut(X) -> Y,
    {
        // Identity law: `fmap(None, _) = None`, `fmap(Some((a, x)), id) =
        // Some((a, x))`. Composition law: `fmap(fmap(fx, f), g) = fmap(fx,
        // |x| g(f(x)))` — the `Option::map` + tuple-second-map composition
        // discharges both directly.
        fx.map(|(a, x)| (a, f(x)))
    }
}

// Opt-in structural equality for `Free<ListEndo<A>, Z>` (and `Cofree`): route
// the comparison of the functor hole `Option<(A, T)>` through `Option`/tuple's
// own `==`. Bounded `A: PartialEq` so the label participates; `T: PartialEq`
// comes from the trait method. Mirrors haft's `OptionWitness: EqFunctor`.
impl<A: PartialEq> EqFunctor for ListEndo<A> {
    fn eq_type<T: PartialEq>(a: &Option<(A, T)>, b: &Option<(A, T)>) -> bool {
        a == b
    }
}

// Opt-in `Debug` for `Free<ListEndo<A>, Z>`: delegate the functor hole to
// `Option<(A, T)>`'s own `Debug`. Bounded `A: Debug`.
impl<A: core::fmt::Debug> DebugFunctor for ListEndo<A> {
    fn fmt_type<T: core::fmt::Debug>(
        fa: &Option<(A, T)>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        core::fmt::Debug::fmt(fa, f)
    }
}

/// Container presentation of `1 + A × −` (Abbott–Altenkirch–Ghani 2003, via
/// CDL). Shapes are `Option<A>`: the unit summand `None` (a `Nil` cell, arity
/// 0) and the product summand `Some(a)` (a `Cons` cell carrying its label `a`,
/// arity 1 — the single recursive slot). `A: PartialEq + Debug` so the shape
/// carries into the machine-checked container laws.
impl<A: PartialEq + core::fmt::Debug> Container for ListEndo<A> {
    type Shape = Option<A>;

    fn arity(shape: &Self::Shape) -> usize {
        match shape {
            None => 0,
            Some(_) => 1,
        }
    }

    fn decompose<X>(fx: Option<(A, X)>) -> (Self::Shape, Vec<X>) {
        match fx {
            None => (None, Vec::new()),
            Some((a, x)) => (Some(a), vec![x]),
        }
    }

    fn recompose<X>(shape: Self::Shape, contents: Vec<X>) -> Option<Option<(A, X)>> {
        match shape {
            // `None` shape (arity 0): reconstruct iff no contents were supplied.
            None => contents.is_empty().then_some(None),
            // `Some(a)` shape (arity 1): `TryFrom<Vec<X>> for [X; 1]` rejects
            // any other length.
            Some(a) => {
                let [x] = <[X; 1]>::try_from(contents).ok()?;
                Some(Some((a, x)))
            }
        }
    }
}

/// Destruct a `Free<ListEndo<A>, Z>` into its `Vec<A>` payload and
/// terminator.
///
/// CDL Example B.19. Walks the cons-cell tower iteratively (loop, not
/// recursion) so it is safe on long lists.
///
/// # Returns
///
/// `(items, terminator)` such that `items` are the `A` values encountered
/// in cons-order and `terminator` is the `Z` payload of the deepest
/// `Pure` leaf.
///
/// # Panics
///
/// Panics if a `Suspend(None)` cell is encountered with no `Pure(z)`
/// terminator above it. Such a value is non-canonical — the encoding
/// produced by [`vec_to_free_mnd`] always terminates with
/// `Free::Pure(z)`. A user constructing `Free::Suspend(None)` directly
/// has produced a value with no `Z` payload to return; we surface that as
/// a panic with a diagnostic rather than fabricate a value.
#[must_use]
pub fn free_mnd_to_vec<A, Z>(input: Free<ListEndo<A>, Z>) -> (Vec<A>, Z) {
    let mut items: Vec<A> = Vec::new();
    let mut current = input;
    loop {
        match current {
            Free::Pure(z) => return (items, z),
            // haft boxes the recursion *inside* the functor hole, so the node is
            // `Option<(A, Box<Free<…>>)>` — no outer `Box` to deref.
            Free::Suspend(node) => match node {
                None => {
                    // `1` summand: a `Nil`-shaped cell with no terminator
                    // payload. By construction the `Free` type forces a
                    // `Z`-bearing `Pure` somewhere; the canonical
                    // encoding produced by `vec_to_free_mnd` never emits
                    // a bare `Nil`-`Suspend` — it terminates with `Pure(z)`.
                    // A user-constructed value reaching this arm has no
                    // sensible `Z` to return; we panic with a diagnostic.
                    panic!(
                        "free_mnd_to_vec: encountered bare ListEndo `None` Suspend without \
                         a terminator; non-canonical Free value. The canonical encoding \
                         (produced by vec_to_free_mnd) terminates with Free::Pure(z), \
                         not Free::Suspend(None)."
                    );
                }
                Some((a, rest)) => {
                    items.push(a);
                    current = *rest;
                }
            },
        }
    }
}

/// Construct a `Free<ListEndo<A>, Z>` from a vector of items and a
/// terminator.
///
/// CDL Example B.19. Builds the cons-cell tower right-to-left so that the
/// resulting structure walks left-to-right when destructed.
///
/// # Examples
///
/// ```ignore
/// // Empty list with `()` terminator → `Pure(())`.
/// let empty = vec_to_free_mnd::<u32, ()>(Vec::new(), ());
/// assert!(matches!(empty, Free::Pure(())));
/// ```
#[must_use]
pub fn vec_to_free_mnd<A, Z>(items: Vec<A>, terminator: Z) -> Free<ListEndo<A>, Z> {
    let mut acc: Free<ListEndo<A>, Z> = Free::Pure(terminator);
    for a in items.into_iter().rev() {
        // Box the recursive slot *inside* the `Option` hole (haft's shape).
        acc = Free::Suspend(Some((a, Box::new(acc))));
    }
    acc
}
