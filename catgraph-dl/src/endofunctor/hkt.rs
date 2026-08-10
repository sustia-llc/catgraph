//! The arity-1 witness tower: [`HKT`] (object map), [`Functor`] (morphism map),
//! [`Pure`] (the unit `η : Id ⇒ F`), and [`Monad`] (`bind`, with `μ = join`
//! derived from it).
//!
//! This module is private; the tower is re-exported through
//! [`crate::endofunctor`], whose module docs carry the witness-first design
//! rationale.

/// The object map of an arity-1 type constructor — the "hole" is the GAT
/// parameter `T`.
///
/// Rust has no higher-kinded types: a generic parameter must be a type, not a
/// type constructor, so `trait Functor<F<A>>` is unwritable. The workaround is
/// the **GAT witness pattern** — a zero-sized `W` stands in for the type
/// constructor and this Generic Associated Type projects it back to the concrete
/// type. `ListEndo<A>` projects to `Option<(A, X)>`, `OptionWitness` to
/// `Option<T>`, and so on. The projection carries no inner-type bound: CDL's
/// ambient category is `Set`, so every `T` is admissible by construction.
pub trait HKT {
    /// The type constructor this witness stands for, with `T` in the hole.
    type Type<T>;
}

/// The morphism map of an endofunctor: lift `f : A → B` to
/// `F(f) : F(A) → F(B)`.
///
/// `fmap` is a **static** method on the witness — call `W::fmap(x, f)`, never
/// `x.fmap(f)`; the witness is a type-level token and is never instantiated.
///
/// # Laws
///
/// ```text
/// fmap(fx, |x| x) == fx                             (identity)
/// fmap(fmap(fx, f), g) == fmap(fx, |x| g(f(x)))     (composition)
/// ```
///
/// These are documented obligations, not machine-checked at compile time; the
/// shipped witnesses discharge them in `tests/functor_laws.rs`. The laws are
/// stated for **pure** morphisms — see the [module docs](crate::endofunctor)
/// for why a stateful `FnMut` can make the composition law appear to fail
/// without the witness being non-functorial.
pub trait Functor<F: HKT> {
    /// Apply `f` inside the container, preserving its structure.
    fn fmap<A, B, Func>(m_a: F::Type<A>, f: Func) -> F::Type<B>
    where
        Func: FnMut(A) -> B;
}

/// The unit `η : Id ⇒ F` — lift a bare value into the functor.
///
/// Split out from [`Monad`] so a witness can be *pointed* (CDL Def B.3, the
/// [`Pointed`](crate::natural::Pointed) marker) without being a monad.
///
/// # Law
///
/// **Naturality**: `fmap(pure(a), f) == pure(f(a))` — the square
/// `F(f) ∘ η_A = η_B ∘ f` commutes for every pure `f`, which is what makes
/// `pure` a natural transformation rather than an arbitrary family of
/// injections (Mac Lane, *CWM* §I.4).
pub trait Pure<F: HKT> {
    /// Lift `value` into the minimal `F`-context.
    fn pure<T>(value: T) -> F::Type<T>;
}

/// A monad on `Set`: [`Functor`] + [`Pure`] + `bind`, with the multiplication
/// `μ` supplied as the derived [`join`](Monad::join).
///
/// The hierarchy is `Functor + Pure` rather than the Haskell
/// `Monad: Applicative`; this crate ships no `Applicative`, and the
/// monad-algebra verifiers in [`crate::algebra`] need only `η = pure` and
/// `μ = join`.
///
/// # Laws
///
/// The Kleisli-triple laws (Moggi, *Notions of Computation and Monads*, 1991):
///
/// ```text
/// bind(pure(a), f) == f(a)                                  (left identity)
/// bind(m, pure) == m                                        (right identity)
/// bind(bind(m, f), g) == bind(m, |x| bind(f(x), g))         (associativity)
/// ```
///
/// Stated for pure functions, as with the [`Functor`] laws.
pub trait Monad<F: HKT>: Functor<F> + Pure<F> {
    /// Kleisli composition: run `f` on the contained value and flatten.
    fn bind<A, B, Func>(m_a: F::Type<A>, f: Func) -> F::Type<B>
    where
        Func: FnMut(A) -> F::Type<B>;

    /// The monad multiplication `μ : F ∘ F ⇒ F`, as `bind` with the identity.
    fn join<A>(m_m_a: F::Type<F::Type<A>>) -> F::Type<A> {
        Self::bind(m_m_a, |x| x)
    }
}
