//! Arrow substrate — catgraph-syntax's own value-level Arrow algebra.
//!
//! A strong category in the sense of John Hughes' Arrow interface (*Generalising
//! Monads to Arrows*, Science of Computer Programming 37(1–3), 2000): [`Id`] and
//! [`Compose`] give the category, [`First`] / [`Second`] / [`Split`] the monoidal
//! strength, [`Fanout`] the Cartesian diagonal, and [`Lift`] embeds a plain
//! function. Every combinator returns a *new concrete type*, so composition is
//! total and monomorphized — no `dyn`, no boxing — and [`arrow`] /
//! [`ArrowBuilder`] hide those types behind a fluent chain. This is a **lineage**
//! citation, not a theorem anchor: the crate's anchors are F&S 2018 / 2019, and
//! nothing here claims an Arrow-law completeness result.
//!
//! The algebra is catgraph's own as of
//! [#222](https://github.com/sustia-llc/catgraph/issues/222). This module was
//! previously a re-export seam over an external algebra crate's Arrow module and
//! is now the definition site, leaving catgraph-syntax with a
//! `catgraph` + `catgraph-applied` + `thiserror` dependency set (plus optional
//! `serde`). The module *name* is kept so the public path
//! `catgraph_syntax::arrow_seam` and the ten names it exports are unchanged.
//!
//! # What the typed builder consumes
//!
//! The Arrow algebra is the **execution target** for the typed builder
//! ([`crate::traced`], Phase S5): a [`Traced<A, G>`](crate::traced::Traced) pairs
//! an executable [`Arrow`] with the
//! [`PropExpr`](catgraph_applied::prop::PropExpr) term it denotes, so a morphism
//! can be both *run* and *reasoned about* from one value. These are the names it
//! builds on:
//!
//! - [`Arrow`] — the trait (`run` + the combinator methods).
//! - [`Compose`] — sequential composition `f >>> g` (the term-level `;`), behind
//!   [`Traced::then`](crate::traced::Traced::then).
//! - [`Split`] — the true tensor `(A, C) → (B, D)` (the term-level `⊗`), behind
//!   [`Traced::par`](crate::traced::Traced::par).
//! - [`Id`] — the identity arrow, behind [`traced_id`](crate::traced::traced_id).
//! - [`Lift`] — the pure-function lift, behind
//!   [`traced_braid_1_1`](crate::traced::traced_braid_1_1) and the caller's
//!   generator arrows.
//!
//! # The rest of the surface
//!
//! These round the algebra out for downstream users; the builder does not need
//! them:
//!
//! - [`arrow`] / [`ArrowBuilder`] — the fluent lift/construction path; the
//!   ergonomic way for a downstream crate to build arrows to feed
//!   [`traced_generator`](crate::traced::traced_generator).
//! - [`First`] / [`Second`] — tensor with an identity on one side; achievable
//!   through the builder anyway (`par` with a
//!   [`traced_id`](crate::traced::traced_id)), so not exposed as a dedicated
//!   `Traced` combinator.
//! - [`Fanout`] — the Cartesian diagonal `A → (A, A)`; **rejected** by the
//!   builder, because pairing it with a term would let the arrow duplicate a wire
//!   no term generator copied (`Fanout` ≠ Frobenius `δ`) — [`crate::traced`]'s
//!   *Deliberate omissions* is the canonical statement; the type stays public so
//!   that distinction can be *named*.
//!
//! # Deliberate minimality
//!
//! The `⊕` half of Hughes' interface — `left` / `right` / `choice` / `fanin` and
//! the `Either`-valued combinator structs behind them — is **not** provided.
//! Nothing above this module routes over a coproduct: the prop / Frobenius
//! surface is monoidal over `⊗`, and [`crate::traced`] pairs each combinator with
//! a [`PropExpr`](catgraph_applied::prop::PropExpr) term former, of which there is
//! none for a sum. An owned algebra carries what it uses, so those four are
//! absent rather than unused — a breaking change for any downstream caller that
//! reached them as provided methods through this module's earlier re-export.
//!
//! # Historical note
//!
//! While this module re-exported an external Arrow implementation it also
//! recorded what it did *not* re-export: that crate's free-monad carrier
//! (`Free` / `FreeWitness`), its `IoAction` effect family, and its `EndoArrow`
//! iteration arrow. Two of those statements outlive the re-export, because they
//! are design positions rather than artifacts of a dependency:
//!
//! - **[`PropExpr<G>`](catgraph_applied::prop::PropExpr) is the term type.**
//!   Applied's congruence-closure engine requires `Eq + Hash` on terms (see
//!   [`PropSignature`](catgraph_applied::prop::PropSignature)), which `PropExpr`
//!   derives; a free-monad carrier whose `Eq` is capability-gated and which ships
//!   no `Hash` cannot back it. That verdict was reached in catgraph
//!   [#93](https://github.com/sustia-llc/catgraph/issues/93) (with
//!   [#76](https://github.com/sustia-llc/catgraph/issues/76)) and is independent
//!   of who owns the Arrow algebra.
//! - **No iteration and no effect executor.** The S5
//!   [`Traced`](crate::traced::Traced) builder wants no fixed-point / loop
//!   combinator, and an effect executor whose `run` consumes `self` cannot back
//!   the term-plus-arrow pairing at all. So this algebra defines neither.

use core::marker::PhantomData;

/// A value-level arrow `In → Out`: a runnable, composable transformation.
///
/// An implementor supplies only [`In`](Arrow::In), [`Out`](Arrow::Out) and
/// [`run`](Arrow::run); the combinators are provided. `run` takes `&self`, so an
/// arrow is reusable.
///
/// # Category theory
///
/// The trait is a **strong category**: [`Id`] / [`Compose`] give the category,
/// [`first`](Arrow::first) / [`second`](Arrow::second) / [`split`](Arrow::split)
/// give the monoidal strength, and [`fanout`](Arrow::fanout) the Cartesian
/// diagonal. Every combinator returns a new concrete type, so composition is
/// **total** and composites compose further. The `⊕` combinators are deliberately
/// absent — see the module docs.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Arrow`",
    note = "lift a function with `Lift::new(f)` (or the `arrow(f)` builder), or implement `Arrow` for your operator type"
)]
pub trait Arrow {
    /// The input object the arrow consumes.
    type In;
    /// The output object the arrow produces.
    type Out;

    /// Apply the arrow to an input.
    fn run(&self, input: Self::In) -> Self::Out;

    /// Sequential composition `f >>> g`: run `self`, then `g` on its output.
    #[inline]
    #[must_use]
    fn compose<G>(self, g: G) -> Compose<Self, G>
    where
        Self: Sized,
        G: Arrow<In = Self::Out>,
    {
        Compose::new(self, g)
    }

    /// `first`: lift `A → B` to `(A, C) → (B, C)`, passing the second component
    /// through.
    #[inline]
    #[must_use]
    fn first<C>(self) -> First<Self, C>
    where
        Self: Sized,
    {
        First::new(self)
    }

    /// `second`: lift `A → B` to `(C, A) → (C, B)`, passing the first component
    /// through.
    #[inline]
    #[must_use]
    fn second<C>(self) -> Second<Self, C>
    where
        Self: Sized,
    {
        Second::new(self)
    }

    /// The monoidal product `***`: run `self` and `g` in parallel on a pair —
    /// `(A, C) → (B, D)` from `self: A → B` and `g: C → D`.
    #[inline]
    #[must_use]
    fn split<G>(self, g: G) -> Split<Self, G>
    where
        Self: Sized,
        G: Arrow,
    {
        Split::new(self, g)
    }

    /// Fanout `&&&`: feed one input to two arrows — `A → (B, C)` from
    /// `self: A → B` and `g: A → C`. Requires `In: Clone` (the input is
    /// duplicated), which is why the typed builder rejects it — see the module
    /// docs.
    #[inline]
    #[must_use]
    fn fanout<G>(self, g: G) -> Fanout<Self, G>
    where
        Self: Sized,
        G: Arrow<In = Self::In>,
        Self::In: Clone,
    {
        Fanout::new(self, g)
    }
}

/// The identity arrow `A → A` — the unit of composition.
pub struct Id<A>(PhantomData<A>);

impl<A> Id<A> {
    /// Constructs the identity arrow.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Id(PhantomData)
    }
}

impl<A> Default for Id<A> {
    #[inline]
    fn default() -> Self {
        Id::new()
    }
}

impl<A> Arrow for Id<A> {
    type In = A;
    type Out = A;

    #[inline]
    fn run(&self, input: A) -> A {
        input
    }
}

/// Lifts a plain function `F: Fn(A) -> B` into an [`Arrow`] `A → B`.
///
/// The input/output types `A` / `B` are carried in the type (via [`PhantomData`])
/// rather than left to the `Fn` bound alone: `Fn`'s argument type is not treated
/// as uniquely determined by `F`, so a `Lift<F>` would be rejected (`E0207`).
/// Use [`Lift::new`] (or the [`arrow`] builder) so callers never write the
/// [`PhantomData`].
pub struct Lift<A, B, F>(F, PhantomData<fn(A) -> B>);

impl<A, B, F> Lift<A, B, F>
where
    F: Fn(A) -> B,
{
    /// Lifts `f` into an arrow.
    #[inline]
    #[must_use]
    pub const fn new(f: F) -> Self {
        Lift(f, PhantomData)
    }
}

impl<A, B, F> Arrow for Lift<A, B, F>
where
    F: Fn(A) -> B,
{
    type In = A;
    type Out = B;

    #[inline]
    fn run(&self, input: A) -> B {
        (self.0)(input)
    }
}

/// Sequential composition `f >>> g`: the arrow that runs `f`, then `g` on its
/// output.
///
/// Composition is **total** — `Compose<F, G>` type-checks whenever `G::In = F::Out`
/// — and `Compose` is itself an [`Arrow`], so composites compose further.
pub struct Compose<F, G>(F, G);

impl<F, G> Compose<F, G> {
    /// Builds `f >>> g`. Prefer [`Arrow::compose`].
    #[inline]
    #[must_use]
    pub const fn new(f: F, g: G) -> Self {
        Compose(f, g)
    }
}

impl<F, G> Arrow for Compose<F, G>
where
    F: Arrow,
    G: Arrow<In = F::Out>,
{
    type In = F::In;
    type Out = G::Out;

    #[inline]
    fn run(&self, input: F::In) -> G::Out {
        self.1.run(self.0.run(input))
    }
}

/// The monoidal product `***`: runs two arrows in parallel on a pair —
/// `(A, C) → (B, D)` from `f: A → B` and `g: C → D`.
///
/// This is the tensor the typed builder pairs with
/// [`Free::tensor`](catgraph_applied::prop::Free::tensor): two independent
/// morphisms side by side, arities adding, nothing shared between them.
pub struct Split<F, G>(F, G);

impl<F, G> Split<F, G> {
    /// Builds `f *** g`. Prefer [`Arrow::split`].
    #[inline]
    #[must_use]
    pub const fn new(f: F, g: G) -> Self {
        Split(f, g)
    }
}

impl<F, G> Arrow for Split<F, G>
where
    F: Arrow,
    G: Arrow,
{
    type In = (F::In, G::In);
    type Out = (F::Out, G::Out);

    #[inline]
    fn run(&self, (a, c): (F::In, G::In)) -> (F::Out, G::Out) {
        (self.0.run(a), self.1.run(c))
    }
}

/// Strength on the first component: lifts `F: A → B` to `(A, C) → (B, C)`,
/// passing the second component through unchanged.
pub struct First<F, C>(F, PhantomData<C>);

impl<F, C> First<F, C> {
    /// Builds the `first` arrow. Prefer [`Arrow::first`].
    #[inline]
    #[must_use]
    pub const fn new(f: F) -> Self {
        First(f, PhantomData)
    }
}

impl<F, C> Arrow for First<F, C>
where
    F: Arrow,
{
    type In = (F::In, C);
    type Out = (F::Out, C);

    #[inline]
    fn run(&self, (a, c): (F::In, C)) -> (F::Out, C) {
        (self.0.run(a), c)
    }
}

/// Strength on the second component: lifts `F: A → B` to `(C, A) → (C, B)`,
/// passing the first component through unchanged.
pub struct Second<F, C>(F, PhantomData<C>);

impl<F, C> Second<F, C> {
    /// Builds the `second` arrow. Prefer [`Arrow::second`].
    #[inline]
    #[must_use]
    pub const fn new(f: F) -> Self {
        Second(f, PhantomData)
    }
}

impl<F, C> Arrow for Second<F, C>
where
    F: Arrow,
{
    type In = (C, F::In);
    type Out = (C, F::Out);

    #[inline]
    fn run(&self, (c, a): (C, F::In)) -> (C, F::Out) {
        (c, self.0.run(a))
    }
}

/// Fanout `&&&`: feeds one input to two arrows — `A → (B, C)` from `f: A → B` and
/// `g: A → C`. Requires the input to be `Clone` (it is duplicated).
///
/// The copy is a *Cartesian* diagonal, not a Frobenius `δ`; [`crate::traced`]'s
/// *Deliberate omissions* is the canonical statement of why the typed builder
/// refuses to pair it with a term.
pub struct Fanout<F, G>(F, G);

impl<F, G> Fanout<F, G> {
    /// Builds `f &&& g`. Prefer [`Arrow::fanout`].
    #[inline]
    #[must_use]
    pub const fn new(f: F, g: G) -> Self {
        Fanout(f, g)
    }
}

impl<F, G> Arrow for Fanout<F, G>
where
    F: Arrow,
    G: Arrow<In = F::In>,
    F::In: Clone,
{
    type In = F::In;
    type Out = (F::Out, G::Out);

    #[inline]
    fn run(&self, input: F::In) -> (F::Out, G::Out) {
        (self.0.run(input.clone()), self.1.run(input))
    }
}

/// Return type of [`ArrowBuilder::then_fn`], factored out to satisfy
/// `clippy::type_complexity`.
type ThenFn<S, C, G> = ArrowBuilder<Compose<S, Lift<<S as Arrow>::Out, C, G>>>;

/// A fluent builder over the [`Arrow`] algebra that hides the combinator types.
///
/// The builder threads the growing arrow type through `Self`, so a caller writes a
/// left-to-right chain and never names [`Compose`] / [`Split`] / [`Lift`] — the
/// textual form of a wiring diagram, and the same encoding
/// `std::iter::Iterator`'s adapters use.
///
/// Start a chain with [`arrow`] (lifting a function) or [`ArrowBuilder::new`]
/// (wrapping an existing arrow); end it with [`build`](ArrowBuilder::build) (yield
/// the composed arrow, e.g. to feed
/// [`traced_generator`](crate::traced::traced_generator)) or
/// [`run`](ArrowBuilder::run) (apply it).
///
/// ```
/// use catgraph_syntax::arrow_seam::{Arrow, arrow};
///
/// let pipeline = arrow(|x: i32| x + 1).then_fn(|x| x * 2).build();
/// assert_eq!(pipeline.run(3), 8);
/// ```
pub struct ArrowBuilder<S>(S);

/// Starts an arrow chain by lifting a function `F: Fn(A) -> B` into a builder.
#[inline]
#[must_use]
pub fn arrow<A, B, F>(f: F) -> ArrowBuilder<Lift<A, B, F>>
where
    F: Fn(A) -> B,
{
    ArrowBuilder(Lift::new(f))
}

impl<S> ArrowBuilder<S>
where
    S: Arrow,
{
    /// Wraps an existing arrow in a builder.
    #[inline]
    #[must_use]
    pub const fn new(arrow: S) -> Self {
        ArrowBuilder(arrow)
    }

    /// Sequential step: compose with another arrow (`then` is an alias of
    /// [`Arrow::compose`]).
    #[inline]
    #[must_use]
    pub fn then<G>(self, g: G) -> ArrowBuilder<Compose<S, G>>
    where
        G: Arrow<In = S::Out>,
    {
        ArrowBuilder(self.0.compose(g))
    }

    /// Sequential step lifting a raw closure, so the caller need not write
    /// [`Lift::new`].
    #[inline]
    #[must_use]
    pub fn then_fn<C, G>(self, g: G) -> ThenFn<S, C, G>
    where
        G: Fn(S::Out) -> C,
    {
        ArrowBuilder(self.0.compose(Lift::new(g)))
    }

    /// Parallel-product step (`par` is an alias of [`Arrow::split`] / `***`).
    #[inline]
    #[must_use]
    pub fn par<G>(self, g: G) -> ArrowBuilder<Split<S, G>>
    where
        G: Arrow,
    {
        ArrowBuilder(self.0.split(g))
    }

    /// Fanout step (`&&&`): feed the same input to a second arrow.
    #[inline]
    #[must_use]
    pub fn fanout<G>(self, g: G) -> ArrowBuilder<Fanout<S, G>>
    where
        G: Arrow<In = S::In>,
        S::In: Clone,
    {
        ArrowBuilder(self.0.fanout(g))
    }

    /// Terminal: yield the composed [`Arrow`] value (reusable, further
    /// composable).
    #[inline]
    #[must_use]
    pub fn build(self) -> S {
        self.0
    }

    /// Terminal: apply the composed arrow to an input.
    #[inline]
    pub fn run(&self, input: S::In) -> S::Out {
        self.0.run(input)
    }
}
