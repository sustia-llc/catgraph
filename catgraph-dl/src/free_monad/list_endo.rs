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
//! collapses to `FreeMnd<ListEndo<A>, ()> ≅ Vec<A>`; with general `Z` the
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
//!   FreeMnd<ListEndo<A>, Z>`).
//!
//! ## Iteration discipline
//!
//! `free_mnd_to_vec` walks the input loop-style with an explicit
//! `current` rebound on each iteration. Recursive walks of `FreeMnd` would
//! consume O(n) stack per layer; tests probe up to ~100-element vectors
//! and would blow the default test stack on a recursive walk.

use core::marker::PhantomData;

use super::free_mnd::{EndoFunctor, FreeMnd};

/// The endofunctor `1 + A × −` for a fixed alphabet `A`.
///
/// The `Apply<X>` projection is `Option<(A, X)>` — `None` for the unit
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

impl<A> EndoFunctor for ListEndo<A> {
    type Apply<X> = Option<(A, X)>;

    fn fmap<X, Y, G>(fx: Self::Apply<X>, f: G) -> Self::Apply<Y>
    where
        G: Fn(X) -> Y,
    {
        // Identity law: `fmap(None, _) = None`, `fmap(Some((a, x)), id) =
        // Some((a, x))`. Composition law: `fmap(fmap(fx, f), g) = fmap(fx,
        // |x| g(f(x)))` — the `Option::map` + tuple-second-map composition
        // discharges both directly.
        fx.map(|(a, x)| (a, f(x)))
    }
}

/// Destruct a `FreeMnd<ListEndo<A>, Z>` into its `Vec<A>` payload and
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
/// Panics if a `Roll(None)` cell is encountered with no `Pure(z)`
/// terminator above it. Such a value is non-canonical — the encoding
/// produced by [`vec_to_free_mnd`] always terminates with
/// `FreeMnd::Pure(z)`. A user constructing `FreeMnd::Roll(None)` directly
/// has produced a value with no `Z` payload to return; we surface that as
/// a panic with a diagnostic rather than fabricate a value.
#[must_use]
pub fn free_mnd_to_vec<A, Z>(input: FreeMnd<ListEndo<A>, Z>) -> (Vec<A>, Z) {
    let mut items: Vec<A> = Vec::new();
    let mut current = input;
    loop {
        match current {
            FreeMnd::Pure(z) => return (items, z),
            FreeMnd::Roll(boxed) => match *boxed {
                None => {
                    // `1` summand: a `Nil`-shaped cell with no terminator
                    // payload. By construction the FreeMnd type forces a
                    // `Z`-bearing `Pure` somewhere; the canonical
                    // encoding produced by `vec_to_free_mnd` never emits
                    // a bare `Nil`-`Roll` — it terminates with `Pure(z)`.
                    // A user-constructed value reaching this arm has no
                    // sensible `Z` to return; we panic with a diagnostic.
                    panic!(
                        "free_mnd_to_vec: encountered bare ListEndo `None` Roll without \
                         a terminator; non-canonical FreeMnd value. The canonical encoding \
                         (produced by vec_to_free_mnd) terminates with FreeMnd::Pure(z), \
                         not FreeMnd::Roll(None)."
                    );
                }
                Some((a, rest)) => {
                    items.push(a);
                    current = rest;
                }
            },
        }
    }
}

/// Construct a `FreeMnd<ListEndo<A>, Z>` from a vector of items and a
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
/// assert!(matches!(empty, FreeMnd::Pure(())));
/// ```
#[must_use]
pub fn vec_to_free_mnd<A, Z>(items: Vec<A>, terminator: Z) -> FreeMnd<ListEndo<A>, Z> {
    let mut acc: FreeMnd<ListEndo<A>, Z> = FreeMnd::Pure(terminator);
    for a in items.into_iter().rev() {
        acc = FreeMnd::roll(Some((a, acc)));
    }
    acc
}
