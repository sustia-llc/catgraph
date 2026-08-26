//! Decorated cospans: cospans of finite sets equipped with extra structure on
//! the apex, following Fong & Spivak's *Seven Sketches in Compositionality*
//! (arXiv:1803.05316v3), Definition 6.75.
//!
//! A decorated cospan is a pair `(c, d)` where `c` is a cospan of finite sets
//! `X → N ← Y` and `d ∈ F(N)` is a decoration on the apex. The decoration
//! function is modelled by a lax symmetric monoidal functor
//! `F : (FinSet, +) → (Set, ×)`, i.e. a [`Decoration`] implementation.
//!
//! Under this framework, composition of decorated cospans uses the pushout of
//! the underlying cospans together with `F`'s pushforward on the coequalizer
//! quotient, and monoidal product uses the laxator `φ_{a,b}` to combine
//! decorations across disjoint apices. This is the bridge between the strict
//! cospan machinery in [`catgraph::cospan`] and domain-specific decorated
//! structures (open Petri nets, open graphs, open dynamical systems, …).
//!
//! This module defines the [`Decoration`] trait (`F` on objects + laxator +
//! pushforward) and the generic [`DecoratedCospan`] struct, with pushout-based
//! [`Composable`] composition (invoking `D::pushforward` on the coequalizer
//! quotient, per F&S Def 6.75), [`Monoidal`] parallel product via the laxator,
//! and a [`HypergraphCategory`] instance supplying the Frobenius generators
//! inherited from `Cospan`, realizing F&S **Theorem 6.77**.
//!
//! # Examples
//!
//! A flat `u32`-valued tally decoration composed in parallel via the
//! monoidal product. The laxator `combine` adds the two tallies.
//!
//! ```
//! use catgraph::cospan::Cospan;
//! use catgraph::monoidal::Monoidal;
//! use catgraph_applied::decorated_cospan::{DecoratedCospan, Decoration};
//!
//! struct Tally;
//! impl Decoration for Tally {
//!     type Apex = u32;
//!     fn empty(_: usize) -> u32 { 0 }
//!     fn combine(a: u32, b: u32) -> u32 { a + b }
//!     fn pushforward(d: u32, _: &[usize]) -> u32 { d }
//! }
//!
//! let c1 = Cospan::<char>::new(vec![0], vec![0], vec!['a']).unwrap();
//! let d1 = DecoratedCospan::<char, Tally>::new(c1, 3);
//! let c2 = Cospan::<char>::new(vec![0], vec![0], vec!['b']).unwrap();
//! let d2 = DecoratedCospan::<char, Tally>::new(c2, 5);
//! let mut prod = d1;
//! prod.monoidal(d2);
//! assert_eq!(prod.decoration, 8);
//! ```
//!
//! See `examples/decorated_cospan_circuit.rs` for an `EdgeSet`-valued
//! decoration modelling the textbook `Circ` example (§6.4 Ex 6.79–6.86).

use std::fmt::Debug;

use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;
use catgraph::errors::CatgraphError;
use catgraph::hypergraph_category::HypergraphCategory;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use permutations::Permutation;

/// A lax symmetric monoidal functor `F : (FinSet, +) → (Set, ×)` supplying
/// decorations on cospan apices.
///
/// Implementers specify what extra structure lives on top of the apex of a
/// cospan (graph edges, Petri net transitions, dynamical system laws, …) and
/// how that structure transforms under
///
/// 1. the empty apex (`F` on the initial object `0 ∈ FinSet`),
/// 2. disjoint union of apices (`F`'s laxator `φ_{a,b} : F(a) × F(b) → F(a+b)`),
///    and
/// 3. pushout quotients of the apex (`F` applied to the coequalizer map
///    produced during cospan composition).
///
/// Together these are exactly the data required to turn a span of cospans
/// into a decorated-cospan category (Fong–Spivak Def 6.75, Thm 6.77).
pub trait Decoration: Sized {
    /// The set `F(N)` of decorations on an apex of size `n`.
    type Apex: Clone + Debug + PartialEq;

    /// `F` on objects: the canonical "empty" decoration for an apex of size
    /// `n`. In most concrete instances this is a zero element, an empty edge
    /// set, or the unique element of a singleton. The parameter `n` is the
    /// apex cardinality and is retained because some decorations (e.g.
    /// vector-valued markings) depend on it even in the empty case.
    fn empty(n: usize) -> Self::Apex;

    /// `F` on `+`: combine decorations on disjoint apices into a decoration
    /// on their sum. Corresponds to the functor's laxator
    /// `φ_{a,b} : F(a) × F(b) → F(a + b)`.
    fn combine(a: Self::Apex, b: Self::Apex) -> Self::Apex;

    /// `F` on pushout quotients: given a decoration on the pre-pushout apex
    /// and the quotient map `q : {0, …, n-1} → {0, …, m-1}` (as a slice
    /// whose `i`th entry is the image of the `i`th pre-pushout element),
    /// produce the decoration on the pushed-out apex.
    ///
    /// This is the image under `F` of the coequalizer arrow that appears in
    /// cospan composition.
    fn pushforward(d: Self::Apex, quotient: &[usize]) -> Self::Apex;
}

/// A cospan of finite sets together with a decoration on its apex.
///
/// The `Lambda` parameter is the middle-vertex label type of the underlying
/// [`Cospan`]; the `D` parameter is a [`Decoration`] functor whose associated
/// apex type determines the shape of the decoration.
///
/// `PartialEq` compares both fields — the underlying cospan and the decoration
/// — and is hand-written so that it asks only for `D: Decoration`, not the
/// `D: PartialEq` a derive would demand of the marker type. There is no `Eq`:
/// [`Decoration::Apex`] is bounded by `PartialEq` only, so nothing here can
/// promise reflexivity. Comparing the `cospan` field alone still works through
/// [`Cospan::structurally_equal`] (equivalently `==`) or the public leg/middle
/// accessors.
#[derive(Clone, Debug)]
pub struct DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    /// The underlying (undecorated) cospan.
    pub cospan: Cospan<Lambda>,
    /// The decoration on the cospan's apex, valued in `F(|middle|)`.
    pub decoration: D::Apex,
}

impl<Lambda, D> PartialEq for DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    /// The cospan's `(left, right, middle)` triple, then the decoration.
    fn eq(&self, other: &Self) -> bool {
        self.cospan == other.cospan && self.decoration == other.decoration
    }
}

impl<Lambda, D> DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    /// Construct a decorated cospan from an underlying cospan and a decoration.
    ///
    /// No consistency check is performed between `cospan.middle().len()` and
    /// the shape of `decoration` — that invariant is the responsibility of
    /// the specific [`Decoration`] implementation.
    #[must_use]
    pub fn new(cospan: Cospan<Lambda>, decoration: D::Apex) -> Self {
        Self { cospan, decoration }
    }
}

/// Sequential composition of decorated cospans (Fong–Spivak Def 6.75,
/// Thm 6.77).
///
/// Delegates the underlying cospan composition to
/// [`Cospan::compose_with_quotient`] (which performs the pushout on the
/// shared interface and returns the coequalizer quotient
/// `q : N_1 + N_2 -> N`), combines the two decorations via
/// [`Decoration::combine`], and pushes the combined decoration forward
/// through `q` via [`Decoration::pushforward`].
///
/// Concretely:
///
/// ```text
///     (c1 ; c2).decoration = F(q)(combine(d1, d2))
/// ```
///
/// Correct for all `Decoration` impls including those whose apex data
/// references apex indices (e.g. edge-set decorations for circuits).
impl<Lambda, D> Composable<Vec<Lambda>> for DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        let (cospan, quotient) = self.cospan.compose_with_quotient(&other.cospan)?;
        let combined = D::combine(self.decoration.clone(), other.decoration.clone());
        let decoration = D::pushforward(combined, &quotient);
        Ok(Self { cospan, decoration })
    }

    fn domain(&self) -> Vec<Lambda> {
        self.cospan.domain()
    }

    fn codomain(&self) -> Vec<Lambda> {
        self.cospan.codomain()
    }
}

/// Monoidal (parallel) product of decorated cospans.
///
/// Delegates the underlying cospan tensor to [`Cospan::monoidal`] (disjoint
/// union of apices with shifted indices) and combines the two decorations
/// via [`Decoration::combine`], which models the lax monoidal functor's
/// laxator `φ_{a,b} : F(a) × F(b) → F(a + b)`.
///
/// Unlike composition, the monoidal product does *not* quotient the apex,
/// so `pushforward` is not needed here — `combine` alone is the full
/// action of `F` on the `+` operation.
impl<Lambda, D> Monoidal for DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    fn monoidal(&mut self, other: Self) {
        self.cospan.monoidal(other.cospan);
        // Swap in a placeholder decoration so we can own the current one
        // and feed it into `D::combine` by value. The placeholder value is
        // immediately overwritten before this method returns.
        let mine = std::mem::replace(&mut self.decoration, D::empty(0));
        self.decoration = D::combine(mine, other.decoration);
    }
}

/// Identity morphism on a tensor word `obj`.
///
/// Delegates to [`Cospan::identity`] (a cospan with `|obj|` apex nodes, each
/// connected identically to one domain and one codomain slot) and attaches
/// the empty decoration for that apex size.
impl<Lambda, D> HasIdentity<Vec<Lambda>> for DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    fn identity(obj: &Vec<Lambda>) -> Self {
        Self {
            cospan: Cospan::identity(obj),
            decoration: D::empty(obj.len()),
        }
    }
}

/// Symmetric monoidal structure (braiding / permutation of tensor factors).
///
/// [`SymmetricMonoidalMorphism`] exposes three methods: [`permute_side`]
/// mutates the morphism by pre/post-composing with a permutation of one leg,
/// while [`from_permutation_on_domain`] and [`from_permutation_on_codomain`]
/// construct a pure-braiding morphism from a permutation on a typed tensor
/// word. In every case the apex cardinality is unchanged — permutations
/// re-label leg targets, not apex nodes — so the decoration is carried through
/// unmodified (`permute_side`) or initialised to the empty decoration on an
/// apex of size `types.len()` (the two constructors).
///
/// [`permute_side`]: SymmetricMonoidalMorphism::permute_side
/// [`from_permutation_on_domain`]: SymmetricMonoidalMorphism::from_permutation_on_domain
/// [`from_permutation_on_codomain`]: SymmetricMonoidalMorphism::from_permutation_on_codomain
impl<Lambda, D> SymmetricMonoidalMorphism<Lambda> for DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        self.cospan.permute_side(p, of_codomain);
    }

    fn from_permutation_on_domain(p: Permutation, types: &[Lambda]) -> Result<Self, CatgraphError> {
        Ok(Self {
            decoration: D::empty(types.len()),
            cospan: Cospan::from_permutation_on_domain(p, types)?,
        })
    }

    fn from_permutation_on_codomain(
        p: Permutation,
        types: &[Lambda],
    ) -> Result<Self, CatgraphError> {
        Ok(Self {
            decoration: D::empty(types.len()),
            cospan: Cospan::from_permutation_on_codomain(p, types)?,
        })
    }
}

/// Hypergraph-category structure on decorated cospans (Fong–Spivak Thm 6.77).
///
/// Each Frobenius generator is obtained by wrapping the corresponding
/// [`Cospan`] generator together with the empty decoration on the generator's
/// apex. The apex size is `1` for unit/counit/multiplication/comultiplication
/// and `2` for the derived cup/cap, matching the middle set size of the
/// underlying [`Cospan`] generator.
impl<Lambda, D> HypergraphCategory<Lambda> for DecoratedCospan<Lambda, D>
where
    Lambda: Eq + Copy + Debug,
    D: Decoration,
{
    fn unit(z: Lambda) -> Self {
        Self {
            cospan: Cospan::unit(z),
            decoration: D::empty(1),
        }
    }

    fn counit(z: Lambda) -> Self {
        Self {
            cospan: Cospan::counit(z),
            decoration: D::empty(1),
        }
    }

    fn multiplication(z: Lambda) -> Self {
        Self {
            cospan: Cospan::multiplication(z),
            decoration: D::empty(1),
        }
    }

    fn comultiplication(z: Lambda) -> Self {
        Self {
            cospan: Cospan::comultiplication(z),
            decoration: D::empty(1),
        }
    }

    fn cup(z: Lambda) -> Result<Self, CatgraphError> {
        Ok(Self {
            cospan: Cospan::cup(z)?,
            decoration: D::empty(2),
        })
    }

    fn cap(z: Lambda) -> Result<Self, CatgraphError> {
        Ok(Self {
            cospan: Cospan::cap(z)?,
            decoration: D::empty(2),
        })
    }
}

#[cfg(test)]
mod tests {
    // The trivial decoration's `Apex` is `()`, so these two lints fire on
    // every call to `Trivial::{empty, combine, pushforward}`.
    #![allow(clippy::let_unit_value, clippy::unit_arg)]

    use super::{DecoratedCospan, Decoration};
    use catgraph::category::Composable;
    use catgraph::cospan::Cospan;

    /// The trivial decoration functor `F(n) = {*}`. Every apex carries the
    /// unique unit decoration; laxator and pushforward are forced.
    #[derive(Debug)]
    struct Trivial;

    impl Decoration for Trivial {
        type Apex = ();

        fn empty(_n: usize) -> Self::Apex {}

        fn combine(_a: Self::Apex, _b: Self::Apex) -> Self::Apex {}

        fn pushforward(_d: Self::Apex, _quotient: &[usize]) -> Self::Apex {}
    }

    #[test]
    fn trivial_decoration_sanity() {
        // Build a small char-labelled cospan: left=[0], right=[1], middle=['a','b'].
        let cospan = Cospan::<char>::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        // `Trivial::empty(2)` returns `()`; bind explicitly so clippy's
        // `unit_arg` lint sees an intentional unit decoration rather than a
        // function call whose only return is `()`.
        let decoration: <Trivial as Decoration>::Apex = Trivial::empty(2);
        let decorated: DecoratedCospan<char, Trivial> = DecoratedCospan::new(cospan, decoration);

        assert_eq!(decorated.decoration, ());
        assert_eq!(decorated.cospan.middle(), &['a', 'b']);
        assert_eq!(decorated.cospan.left_to_middle(), &[0]);
        assert_eq!(decorated.cospan.right_to_middle(), &[1]);

        // Exercise the remaining `Decoration` methods.
        let combined: <Trivial as Decoration>::Apex =
            Trivial::combine(Trivial::empty(1), Trivial::empty(1));
        assert_eq!(combined, ());
        let pushed: <Trivial as Decoration>::Apex =
            Trivial::pushforward(Trivial::empty(2), &[0, 0]);
        assert_eq!(pushed, ());
    }

    /// `==` compares both fields, and is available for a `Decoration` marker
    /// that is not itself `PartialEq`.
    ///
    /// **What this ranges over.** One `Decoration` (`usize`-valued), one apex
    /// size, and the three cases that separate the two conjuncts of `eq`:
    /// both fields equal, decoration differing alone, cospan differing alone.
    /// It does not sweep `Lambda` types or decoration types, and it says
    /// nothing about `Eq` — there is deliberately no `Eq` impl, since
    /// `Decoration::Apex` is bounded by `PartialEq` only.
    ///
    /// Falsification: dropping the `decoration` conjunct from `eq` reddens the
    /// second assertion, dropping the `cospan` conjunct reddens the third, and
    /// replacing the impl with a derive fails to compile on `Counter:
    /// PartialEq`.
    #[test]
    fn decorated_cospan_equality_compares_both_fields() {
        let cospan = || Cospan::<char>::new(vec![0], vec![1], vec!['a', 'b']).unwrap();

        let a: DecoratedCospan<char, Counter> = DecoratedCospan::new(cospan(), 3);
        let same: DecoratedCospan<char, Counter> = DecoratedCospan::new(cospan(), 3);
        assert!(a == same, "same triple, same decoration");

        let other_decoration: DecoratedCospan<char, Counter> = DecoratedCospan::new(cospan(), 4);
        assert!(
            a != other_decoration,
            "the decoration is part of the value: 3 vs 4"
        );

        let other_cospan: DecoratedCospan<char, Counter> = DecoratedCospan::new(
            Cospan::<char>::new(vec![1], vec![1], vec!['a', 'b']).unwrap(),
            3,
        );
        assert!(
            a != other_cospan,
            "the cospan is part of the value: left [0] vs [1]"
        );
    }

    /// A flat `usize`-valued decoration: empty is `0`, combine is `+`,
    /// and pushforward is the identity (counters are apex-invariant).
    #[derive(Debug)]
    struct Counter;

    impl Decoration for Counter {
        type Apex = usize;

        fn empty(_n: usize) -> Self::Apex {
            0
        }

        fn combine(a: Self::Apex, b: Self::Apex) -> Self::Apex {
            a + b
        }

        fn pushforward(d: Self::Apex, _quotient: &[usize]) -> Self::Apex {
            d
        }
    }

    #[test]
    fn counter_compose_adds_decorations() {
        use catgraph::category::Composable;

        // c1: domain = ['a'], codomain = ['b']. Middle has two elements.
        let c1 = Cospan::<char>::new(vec![0], vec![1], vec!['a', 'b']).unwrap();
        let d1 = DecoratedCospan::<char, Counter>::new(c1, 3);

        // c2: domain = ['b'], codomain = ['b']. Must share the 'b'
        // interface with c1.codomain() for pushout composition to succeed.
        let c2 = Cospan::<char>::new(vec![0], vec![0], vec!['b']).unwrap();
        let d2 = DecoratedCospan::<char, Counter>::new(c2, 5);

        let composed = d1
            .compose(&d2)
            .expect("decorated cospan composition should succeed");

        // F(+) laxator applied via compose: counters add.
        assert_eq!(composed.decoration, 8);
    }

    #[test]
    fn counter_hypergraph_identity() {
        use catgraph::category::HasIdentity;

        let id = DecoratedCospan::<char, Counter>::identity(&vec!['a', 'b']);
        assert_eq!(id.cospan.domain(), vec!['a', 'b']);
        assert_eq!(id.cospan.codomain(), vec!['a', 'b']);
        // Empty decoration for an apex of size 2 on `Counter` is `0`.
        assert_eq!(id.decoration, 0);
    }

    #[test]
    fn counter_hypergraph_category_generators() {
        use catgraph::hypergraph_category::HypergraphCategory;

        let eta = DecoratedCospan::<char, Counter>::unit('a');
        assert!(eta.cospan.domain().is_empty());
        assert_eq!(eta.cospan.codomain(), vec!['a']);
        assert_eq!(eta.decoration, 0);

        let eps = DecoratedCospan::<char, Counter>::counit('a');
        assert_eq!(eps.cospan.domain(), vec!['a']);
        assert!(eps.cospan.codomain().is_empty());
        assert_eq!(eps.decoration, 0);
    }

    #[test]
    fn counter_hypergraph_mu_delta() {
        use catgraph::hypergraph_category::HypergraphCategory;

        let mu = DecoratedCospan::<char, Counter>::multiplication('a');
        assert_eq!(mu.cospan.domain(), vec!['a', 'a']);
        assert_eq!(mu.cospan.codomain(), vec!['a']);
        assert_eq!(mu.decoration, 0);

        let delta = DecoratedCospan::<char, Counter>::comultiplication('a');
        assert_eq!(delta.cospan.domain(), vec!['a']);
        assert_eq!(delta.cospan.codomain(), vec!['a', 'a']);
        assert_eq!(delta.decoration, 0);
    }

    #[test]
    fn counter_hypergraph_cup_cap() {
        use catgraph::hypergraph_category::HypergraphCategory;

        let cup = DecoratedCospan::<char, Counter>::cup('a').unwrap();
        assert!(cup.cospan.domain().is_empty());
        assert_eq!(cup.cospan.codomain(), vec!['a', 'a']);

        let cap = DecoratedCospan::<char, Counter>::cap('a').unwrap();
        assert_eq!(cap.cospan.domain(), vec!['a', 'a']);
        assert!(cap.cospan.codomain().is_empty());
    }

    #[test]
    fn counter_from_permutation_shape() {
        use catgraph::monoidal::SymmetricMonoidalMorphism;
        use permutations::Permutation;

        // Swap of two wires: permutation (0 1).
        let swap = Permutation::transposition(2, 0, 1);
        let braid = DecoratedCospan::<char, Counter>::from_permutation_on_domain(swap, &['a', 'b'])
            .unwrap();
        // domain/codomain labels are carried through — for a swap on ['a','b']
        // the codomain label sequence is the permuted one.
        assert_eq!(braid.cospan.domain(), vec!['a', 'b']);
        assert_eq!(braid.decoration, 0);
    }

    #[test]
    fn counter_monoidal_combines_decorations() {
        use catgraph::monoidal::Monoidal;

        let c1 = Cospan::<char>::new(vec![0], vec![0], vec!['a']).unwrap();
        let d1 = DecoratedCospan::<char, Counter>::new(c1, 2);

        let c2 = Cospan::<char>::new(vec![0], vec![0], vec!['b']).unwrap();
        let d2 = DecoratedCospan::<char, Counter>::new(c2, 7);

        let mut prod = d1;
        prod.monoidal(d2);

        // F(+) laxator applied via monoidal: counters add.
        assert_eq!(prod.decoration, 9);
    }
}
