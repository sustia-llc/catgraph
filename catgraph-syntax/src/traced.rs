//! Typed builder over the Arrow seam (Phase S5) — the executable/denotational
//! pairing that completes the [#5](https://github.com/sustia-llc/catgraph/issues/5)
//! milestone surface.
//!
//! A [`Traced<A, G>`] pairs a runnable haft [`Arrow`]
//! with the [`PropExpr<G>`](catgraph_applied::prop::PropExpr) *term* it denotes,
//! so a single value can be both **run** (via the arrow) and **reasoned about**
//! (via the term — printed, parsed, evaluated under any
//! [`ArrowModel`](crate::eval::ArrowModel), normalized, or fed to the presentation
//! engine). This is the typed track of the design's *Arrow bridge*: the S3
//! [interpreter](crate::eval) works over `Vec<V>` wire bundles sized by `usize`
//! arities, while haft arrows live in a world of nested pairs; [`Wires`] is the
//! lawful, arity-preserving bridge between the two.
//!
//! # Intellectual lineage
//!
//! The combinator vocabulary ([`then`](Traced::then) = `>>>`, [`par`](Traced::par)
//! = `***`, the identity/generator builders) is John Hughes' Arrow interface
//! (*Generalising Monads to Arrows*, Science of Computer Programming 37, 2000),
//! reached here through haft's value-level Arrow algebra. This is a **lineage**
//! citation, not a theorem anchor: the milestone law below (S5's coherence
//! contract) is what is proven, not an Arrow-law completeness result.
//!
//! # The coherence contract (the S5 milestone law)
//!
//! For every [`Traced<A, G>`] built through the combinators in this module, and
//! every model `M: ArrowModel<G, Value = V>`:
//!
//! ```text
//! eval(t.term(), &model, input.flatten()) == Ok(t.run(input).flatten())
//! ```
//!
//! Running the arrow on a typed [`Wires`] bundle and flattening the result equals
//! evaluating the paired term on the flattened input. Because wire shapes are
//! *type-level* (a `Traced`'s interface is fixed by `A::In`/`A::Out`), this cannot
//! be proptested over random shapes; the test suite instead exercises every
//! combinator with a family of hand-built pipelines over
//! [`SfgGenerator<i64>`](catgraph_applied::sfg::SfgGenerator) /
//! [`SfgModel`](crate::eval::SfgModel), each checked over proptest-random input
//! *values*. The structural check in [`traced_generator`] pins **arities**; the
//! value-level agreement between an arrow and the model the term is evaluated
//! under is the caller's contract (the coherence tests demonstrate it for the
//! shipped SFG examples).
//!
//! # Deliberate omissions
//!
//! Three combinators are intentionally **not** offered — each would either need
//! machinery out of this phase's scope or would let the arrow and the term denote
//! *different* morphisms, breaking the coherence law:
//!
//! - **General `braid(m, n)`.** Only [`traced_braid_1_1`] ships. A general braid
//!   would have to rebracket arbitrary nested pair types at the type level (turn
//!   `((A, B), C)` into a permuted nesting), which the [`Wires`] encoding does not
//!   express — flatten canonicalizes *values*, not *types*. Out of scope.
//! - **`fanout` (`&&&`) — rejected.** haft's [`Fanout`](crate::arrow_seam::Fanout)
//!   is the Cartesian diagonal `A → (A, A)` (copy is free in `Set`). Pairing it
//!   with a term would let the arrow *duplicate* a wire while no term generator
//!   did — the arrow and the term would denote different morphisms. Copying is a
//!   *model* concern (a Frobenius comultiplication `δ` the model must supply, e.g.
//!   [`SfgGenerator::Copy`](catgraph_applied::sfg::SfgGenerator::Copy) whose
//!   `Clone` lives in [`SfgModel`](crate::eval::SfgModel)), never a free structure
//!   map. This is the interpreter's *no-`Clone`-in-`eval`* discipline (see
//!   [`crate::eval`]'s "No `Clone` on the wire values") and the seam's
//!   `Fanout` note ([`crate::arrow_seam`]) made **type-level**: the diagonal is
//!   simply unreachable through this builder.
//! - **Spider arrows.** haft's [`Arrow`] has no
//!   Frobenius structure, so μ/η/δ/ε have no arrow realization here; spiders are
//!   interpreter / matrix territory ([`crate::eval`], [`crate::frobenius`]).
//!
//! [`EndoArrow`](https://docs.rs/deep_causality_haft) (haft's iteration arrow)
//! stays excluded as well — it is not re-exported by [`crate::arrow_seam`], and a
//! loop / fixed-point combinator is not wanted by this design.

use std::vec;

use catgraph_applied::prop::{Free, PropExpr, PropSignature};

use crate::arrow_seam::{Arrow, Compose, Id, Lift, Split};
use crate::errors::SyntaxError;

/// A single typed wire carrying one value of type `V`.
///
/// The atom of the [`Wires`] encoding: it has [`COUNT`](Wires::COUNT) `1` and
/// flattens to a one-element bundle. The newtype (rather than a bare `V`) is what
/// keeps the [`Wires`] impls coherent — a blanket `impl<V> Wires<V> for V` would
/// overlap the pair impl.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Wire<V>(pub V);

/// A typed wire bundle: the bridge between a haft arrow's nested-pair interface
/// and the [interpreter](crate::eval)'s flat `Vec<V>` bundles.
///
/// An implementor is a *tree of pairs* over [`Wire<V>`] leaves and the empty
/// bundle `()`; [`flatten`](Wires::flatten) collapses any such tree to the
/// canonical left-to-right `Vec<V>`, and [`COUNT`](Wires::COUNT) is that vector's
/// length known at compile time. **Any pair-tree shape is a valid bundle** —
/// `(L, R)`, `((A, B), C)`, and `(A, (B, C))` with the same leaves all flatten to
/// the same vector. That is exactly why
/// [`Split`]'s input `(In1, In2)` is automatically a
/// `Wires` bundle: [`par`](Traced::par) tensors two sub-bundles by *pairing* them,
/// with no rebracketing, and flatten canonicalizes the nesting away.
///
/// # The encoding (why these three impls, no blanket)
///
/// - `()` — zero wires ([`COUNT`](Wires::COUNT) `0`).
/// - [`Wire<V>`] — one wire ([`COUNT`](Wires::COUNT) `1`).
/// - `(L, R)` — [`COUNT`](Wires::COUNT) = `L::COUNT + R::COUNT`, flatten = `L`
///   then `R`.
///
/// A blanket `impl<V> Wires<V> for V` would overlap the pair impl (a pair is
/// itself a `V`), and a unit-terminated cons-list encoding (`(Wire<V>, ())`) is
/// not [`Split`]-compatible — `Split`'s `Out` is a bare
/// `(B, D)` pair. Leaf-plus-pair is the shape that matches haft's arrows.
pub trait Wires<V>: Sized {
    /// The number of wires in this bundle — the length of
    /// [`flatten`](Wires::flatten)'s output, known at compile time.
    const COUNT: usize;

    /// Collapse the pair-tree to its canonical left-to-right value vector.
    fn flatten(self) -> Vec<V>;

    /// Rebuild a bundle of this shape from `values`, checking the length.
    ///
    /// # Errors
    ///
    /// Returns [`SyntaxError::WireCount`] if `values.len() != Self::COUNT`.
    fn unflatten(values: Vec<V>) -> Result<Self, SyntaxError> {
        let actual = values.len();
        if actual != Self::COUNT {
            return Err(SyntaxError::WireCount {
                expected: Self::COUNT,
                actual,
                context: "Wires::unflatten bundle length",
            });
        }
        let mut iter = values.into_iter();
        Self::unflatten_from(&mut iter)
    }

    /// Streaming core of [`unflatten`](Wires::unflatten): draw exactly
    /// [`COUNT`](Wires::COUNT) values off `iter`, leaving the rest.
    ///
    /// This mirrors the S3 interpreter's `take_exact` cursor pattern so nested
    /// bundles consume cleanly: a `(L, R)` draws `L`'s wires then `R`'s from the
    /// same iterator. Prefer [`unflatten`](Wires::unflatten), which length-checks
    /// up front; this method assumes the caller guarantees enough values.
    ///
    /// # Errors
    ///
    /// Returns [`SyntaxError::WireCount`] if `iter` runs dry before this bundle's
    /// wires are filled (unreachable via [`unflatten`](Wires::unflatten), whose
    /// length check precedes the draw).
    fn unflatten_from(iter: &mut vec::IntoIter<V>) -> Result<Self, SyntaxError>;
}

impl<V> Wires<V> for () {
    const COUNT: usize = 0;

    fn flatten(self) -> Vec<V> {
        Vec::new()
    }

    fn unflatten_from(_iter: &mut vec::IntoIter<V>) -> Result<Self, SyntaxError> {
        Ok(())
    }
}

impl<V> Wires<V> for Wire<V> {
    const COUNT: usize = 1;

    fn flatten(self) -> Vec<V> {
        vec![self.0]
    }

    fn unflatten_from(iter: &mut vec::IntoIter<V>) -> Result<Self, SyntaxError> {
        match iter.next() {
            Some(v) => Ok(Wire(v)),
            None => Err(SyntaxError::WireCount {
                expected: 1,
                actual: 0,
                context: "Wires::unflatten wire",
            }),
        }
    }
}

impl<V, L: Wires<V>, R: Wires<V>> Wires<V> for (L, R) {
    const COUNT: usize = L::COUNT + R::COUNT;

    fn flatten(self) -> Vec<V> {
        let mut out = self.0.flatten();
        out.extend(self.1.flatten());
        out
    }

    fn unflatten_from(iter: &mut vec::IntoIter<V>) -> Result<Self, SyntaxError> {
        let left = L::unflatten_from(iter)?;
        let right = R::unflatten_from(iter)?;
        Ok((left, right))
    }
}

/// A morphism carried as both an executable arrow and the term it denotes.
///
/// `Traced<A, G>` bundles a haft [`Arrow`] `A` with the
/// [`PropExpr<G>`](catgraph_applied::prop::PropExpr) it stands for. The fields are
/// **private on purpose**: the two halves are kept in sync — the term's source
/// arity equals `A::In`'s [`Wires::COUNT`], its target equals `A::Out`'s — *only*
/// because the sole way to obtain a `Traced` is through the paired constructors in
/// this module ([`traced_generator`], [`traced_id`], [`traced_braid_1_1`],
/// [`then`](Traced::then), [`par`](Traced::par)), each of which advances arrow and
/// term together. There is no constructor from parts, so this invariant cannot be
/// violated from outside the module.
pub struct Traced<A, G: PropSignature> {
    arrow: A,
    term: PropExpr<G>,
}

impl<A, G: PropSignature> Traced<A, G> {
    /// The [`PropExpr<G>`](catgraph_applied::prop::PropExpr) this morphism denotes
    /// — print it, parse over it, evaluate it under any
    /// [`ArrowModel`](crate::eval::ArrowModel), or feed it to the presentation
    /// engine.
    #[must_use]
    pub fn term(&self) -> &PropExpr<G> {
        &self.term
    }

    /// Split into the raw arrow and term.
    ///
    /// This is a one-way door: there is **no** constructor that reassembles a
    /// `Traced` from an `(A, PropExpr<G>)` pair, precisely because an arbitrary
    /// pair need not satisfy the arrow/term sync invariant (see the type docs).
    /// Use this only to hand the two halves to code that consumes them
    /// independently.
    #[must_use]
    pub fn into_parts(self) -> (A, PropExpr<G>) {
        (self.arrow, self.term)
    }
}

impl<A: Arrow, G: PropSignature> Traced<A, G> {
    /// Run the executable arrow on a typed input bundle, delegating to
    /// [`Arrow::run`]. The paired term is
    /// untouched; [`term`](Traced::term) still denotes what this computes.
    pub fn run(&self, input: A::In) -> A::Out {
        self.arrow.run(input)
    }

    /// Sequential composition `self >>> other` — run `self`, then `other` on its
    /// output. **Infallible.**
    ///
    /// The bound `B: Arrow<In = A::Out>` makes the interface types *equal*, so
    /// `A::Out` and `B::In` have the same [`Wires::COUNT`]; combined with the
    /// `Traced` sync invariant (each term's arities track its arrow's interface
    /// types), `self.term.target() == other.term.source()`, so
    /// [`Free::compose`](catgraph_applied::prop::Free::compose) cannot fail. This
    /// is the payoff of the typed track: an arity error that the untyped
    /// interpreter would surface at runtime is here ruled out at compile time.
    #[must_use]
    pub fn then<B>(self, other: Traced<B, G>) -> Traced<Compose<A, B>, G>
    where
        B: Arrow<In = A::Out>,
    {
        Traced {
            arrow: self.arrow.compose(other.arrow),
            term: Free::compose(self.term, other.term).expect(
                "invariant: type-level interface agreement makes the term composition arity-safe",
            ),
        }
    }

    /// Parallel product `self *** other` — the monoidal tensor `(A₁, C) → (B₁, D)`
    /// pairing two independent morphisms. **Infallible** (tensor sums arities;
    /// [`Free::tensor`](catgraph_applied::prop::Free::tensor) has no failure case).
    #[must_use]
    pub fn par<B>(self, other: Traced<B, G>) -> Traced<Split<A, B>, G>
    where
        B: Arrow,
    {
        Traced {
            arrow: self.arrow.split(other.arrow),
            term: Free::tensor(self.term, other.term),
        }
    }
}

/// Pair a signature generator with an executable arrow, checking their arities
/// agree.
///
/// This is the **only** fallible constructor: it verifies that the arrow's typed
/// interface matches the generator's declared arity —
/// `<A::In as Wires<V>>::COUNT == generator.source()` and
/// `<A::Out as Wires<V>>::COUNT == generator.target()`. The check is
/// **structural (arity) only**; whether the arrow's *values* agree with the
/// [`ArrowModel`](crate::eval::ArrowModel) the term is later evaluated under is
/// the caller's contract (the coherence tests demonstrate it for the shipped SFG
/// examples). The resulting term is the generator leaf
/// [`Free::generator(generator)`](catgraph_applied::prop::Free::generator).
///
/// # Errors
///
/// Returns [`SyntaxError::WireCount`] if the input-bundle wire count differs from
/// `generator.source()`, or the output-bundle wire count from
/// `generator.target()`.
pub fn traced_generator<V, A, G>(generator: G, arrow: A) -> Result<Traced<A, G>, SyntaxError>
where
    A: Arrow,
    A::In: Wires<V>,
    A::Out: Wires<V>,
    G: PropSignature,
{
    let source = generator.source();
    let target = generator.target();
    let in_count = <A::In as Wires<V>>::COUNT;
    let out_count = <A::Out as Wires<V>>::COUNT;
    if in_count != source {
        return Err(SyntaxError::WireCount {
            expected: source,
            actual: in_count,
            context: "traced_generator input bundle vs generator source arity",
        });
    }
    if out_count != target {
        return Err(SyntaxError::WireCount {
            expected: target,
            actual: out_count,
            context: "traced_generator output bundle vs generator target arity",
        });
    }
    Ok(Traced {
        arrow,
        term: Free::generator(generator),
    })
}

/// The identity morphism on a `W`-shaped bundle — the haft [`Id`] arrow paired
/// with [`Free::identity(W::COUNT)`](catgraph_applied::prop::Free::identity).
/// Infallible.
#[must_use]
pub fn traced_id<V, W, G>() -> Traced<Id<W>, G>
where
    W: Wires<V>,
    G: PropSignature,
{
    Traced {
        arrow: Id::new(),
        term: Free::identity(<W as Wires<V>>::COUNT),
    }
}

/// The single-wire swap `(Wire<V>, Wire<V>) → (Wire<V>, Wire<V>)` paired with
/// [`Free::braid(1, 1)`](catgraph_applied::prop::Free::braid) — the one braid this
/// builder ships (see the module docs on why general `braid(m, n)` is omitted).
/// Infallible.
#[must_use]
pub fn traced_braid_1_1<V, G>() -> Traced<PairSwap<V>, G>
where
    G: PropSignature,
{
    Traced {
        arrow: Lift::new(swap_pair::<V> as fn((Wire<V>, Wire<V>)) -> (Wire<V>, Wire<V>)),
        term: Free::braid(1, 1),
    }
}

/// The concrete arrow type behind [`traced_braid_1_1`]: a [`Lift`] of the
/// private pair-swap function pointer. Named (rather than an unnameable closure)
/// so `traced_braid_1_1`'s return type is fully concrete.
pub type PairSwap<V> =
    Lift<(Wire<V>, Wire<V>), (Wire<V>, Wire<V>), fn((Wire<V>, Wire<V>)) -> (Wire<V>, Wire<V>)>;

/// Swap the two wires of a pair — the pure function [`traced_braid_1_1`] lifts.
fn swap_pair<V>((a, b): (Wire<V>, Wire<V>)) -> (Wire<V>, Wire<V>) {
    (b, a)
}
