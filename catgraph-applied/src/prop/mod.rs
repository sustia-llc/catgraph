//! Props and the free prop on a signature.
//!
//! F&S *Seven Sketches in Compositionality* §5.2:
//! - **Def 5.2.** A *prop* is a symmetric strict monoidal category with
//!   `Ob = ℕ` and tensor = addition on objects. Morphisms `m → n` are the
//!   "`m`-ary-in, `n`-ary-out" building blocks of a compositional theory.
//! - **Def 5.25.** The *free prop* `Free(G)` on a signature `(G, s, t)` — a
//!   set of generators `G` with declared source/target arities `s, t: G → ℕ`
//!   — is the prop whose morphisms are all well-formed expressions built
//!   from `G` under composition (`;`), tensor (`⊗`), identities, and
//!   symmetric braiding, modulo the SMC axioms.
//!
//! # This implementation
//!
//! Morphisms of `Free(G)` are arity-tracked expression trees ([`PropExpr`]).
//! Smart constructors on [`Free`] enforce arity at construction time:
//! composition requires matching interface; tensor concatenates.
//!
//! ## Equality
//!
//! Equality on [`PropExpr`] is **structural** — two expressions are equal
//! iff their trees match. Equivalence modulo the SMC axioms (interchange,
//! unitors, braiding naturality) is handled by the presentation / equations
//! type (`Def 5.33`); see the [`presentation`] module. `Free(G)` gives
//! a faithful pre-quotient representation: every morphism of the free prop
//! has a `PropExpr` witness, but distinct witnesses may represent the same
//! morphism.
//!
//! ## Relationship to `catgraph` core
//!
//! `PropExpr<G>` implements the standard catgraph trait hierarchy:
//! [`Composable<Vec<()>>`], [`Monoidal`], [`HasIdentity<Vec<()>>`], and
//! [`SymmetricMonoidalMorphism<()>`]. Objects are represented as
//! `Vec<()>` of length `n` (standing for the prop object `n ∈ ℕ`).

use std::borrow::Cow;
use std::marker::PhantomData;

use catgraph::category::{Composable, HasIdentity};
use catgraph::errors::CatgraphError;
use catgraph::monoidal::{Monoidal, SymmetricMonoidalMorphism};
use permutations::Permutation;

/// The monochromatic word `•ⁿ` over the one-element palette `Λ = {•}`, which
/// this crate spells `()`.
///
/// Single-sorted [`PropSignature`] impls build their interface words with it:
/// `fn source_word(&self) -> Cow<'_, [()]> { mono_word(self.source()) }`.
/// `Vec<()>` is ZST-backed, so the `Cow::Owned` never allocates.
#[must_use]
pub fn mono_word(n: usize) -> Cow<'static, [()]> {
    Cow::Owned(vec![(); n])
}

/// A signature `(G, s, t)` for a free prop: every generator declares a source
/// word [`PropSignature::source_word`] and a target word
/// [`PropSignature::target_word`] over a color alphabet `Λ`
/// ([`PropSignature::Color`]); the arities [`PropSignature::source`] /
/// [`PropSignature::target`] are their lengths.
///
/// # Λ-colored words ([#79](https://github.com/sustia-llc/catgraph/issues/79))
///
/// F&S 2019 Def 3.9 takes the objects of a Λ-colored prop to be the free monoid
/// `List(Λ)` — objectwise-free over the palette — and Thm 3.14 builds the free
/// hypergraph category `Cospan_Λ` over it. A generator's interface is therefore
/// a *word* over `Λ`, not merely a natural number. Choosing `Color = ()`
/// collapses `List(Λ)` back to `ℕ` and recovers the single-sorted prop of F&S
/// 2018 Def 5.25; [`mono_word`] is the helper for that case.
///
/// # Invariant: overridden arities must equal the word lengths
///
/// `source` / `target` are **provided** as `source_word().len()` /
/// `target_word().len()`. An impl may override them — every single-sorted impl
/// does, and then derives its words from them — but an override must stay equal
/// to the corresponding word length. The diagram layer
/// ([`presentation::smc_nf`]) is positional and reads arities; the colored
/// well-formedness pass reads words; a divergence would make the two disagree
/// about the same generator.
///
/// # Supertrait bounds
///
/// `PropSignature` requires `Eq + Hash + Ord` in addition to
/// `Clone + PartialEq + Debug`.
///
/// `Eq + Hash` are needed by the [`presentation::kb::CongruenceClosure`]
/// decision procedure, which uses `G` as a `HashMap` key in its term graph.
/// `Ord` supplies the *content-derived tie-break* the normal-form engine needs
/// where two candidate orderings are otherwise indistinguishable — closed↔closed
/// block order and the scalar tie
/// ([#174](https://github.com/sustia-llc/catgraph/issues/174) residual (b)).
/// It is `Ord` rather than a separate key type precisely for the `Ord` law that
/// equal keys imply equal generators. Migration:
///
/// - Derived-`PartialEq` types: add `Eq, Hash, PartialOrd, Ord` to the
///   `#[derive(...)]`.
/// - Types containing `f64`: provide manual `Eq` + `Hash` impls via
///   `to_bits()` (bit-exact except `-0.0` normalizes to `0.0` to satisfy the
///   `Eq`/`Hash` contract) and a manual `Ord` via `f64::total_cmp` on the same
///   `-0.0`-normalized value; see [`crate::rig::UnitInterval`] /
///   [`crate::rig::Tropical`] / [`crate::rig::F64Rig`].
pub trait PropSignature: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Debug + Ord {
    /// The color alphabet `Λ`. `()` recovers the single-sorted prop.
    type Color: Clone + Eq + std::hash::Hash + std::fmt::Debug;

    /// Source word `s(g) ∈ List(Λ)` — the colors of the input ports, in order.
    fn source_word(&self) -> Cow<'_, [Self::Color]>;
    /// Target word `t(g) ∈ List(Λ)` — the colors of the output ports, in order.
    fn target_word(&self) -> Cow<'_, [Self::Color]>;

    /// Source arity `s(g) ∈ ℕ`. Provided as the length of [`source_word`];
    /// an override must agree with it.
    ///
    /// [`source_word`]: PropSignature::source_word
    fn source(&self) -> usize {
        self.source_word().len()
    }
    /// Target arity `t(g) ∈ ℕ`. Provided as the length of [`target_word`];
    /// an override must agree with it.
    ///
    /// [`target_word`]: PropSignature::target_word
    fn target(&self) -> usize {
        self.target_word().len()
    }
}

/// Arity-tracked free-prop expression tree over a signature `G`.
///
/// Every node carries enough information to recover the arity of the
/// subterm rooted at it via [`PropExpr::source`] and [`PropExpr::target`]
/// — `Compose` chains resolve in O(height), while `Tensor` visits both
/// halves, so the worst case is proportional to the subterm's size. Smart
/// constructors on [`Free`] produce only well-formed expressions; raw
/// variant construction is available but callers must uphold the
/// composition-arity invariant themselves. In this fold and the colored
/// `check`/`infer` interpreters, arity sums that would overflow `usize`
/// saturate to `usize::MAX` — a sentinel no real wire bundle can have, so
/// length checks report [`CatgraphError::CompositionSizeMismatch`] instead
/// of wrapping (reject-don't-wrap, #180).
///
/// Saturation is the right answer only where the sum is *compared* against a
/// real length. The deeper passes size collections from it instead, so they
/// screen it out rather than saturate (#196): [`PropExpr::arities_fit`] tests
/// exactly the overflowing-sum class — an infeasibly huge arity written
/// *literally* (`Identity(usize::MAX)`) involves no sum and passes it (#197) —
/// `content_of` and `nf` reject outside it in both build profiles,
/// and the equality APIs above them ([`Presentation::eq_mod`], [`ColoredExpr::eq_colored`])
/// stay total by gating on it.
///
/// [`Presentation::eq_mod`]: crate::prop::presentation::Presentation::eq_mod
/// [`ColoredExpr::eq_colored`]: crate::prop::colored::ColoredExpr::eq_colored
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PropExpr<G: PropSignature> {
    /// `id_n : n → n`.
    Identity(usize),
    /// Symmetric braiding `σ_{m,n} : m+n → m+n` that swaps the two blocks.
    Braid(usize, usize),
    /// A generator `g ∈ G`.
    Generator(G),
    /// Sequential composition `f ; g` (requires `f.target() == g.source()`).
    Compose(Box<PropExpr<G>>, Box<PropExpr<G>>),
    /// Parallel tensor `f ⊗ g`.
    Tensor(Box<PropExpr<G>>, Box<PropExpr<G>>),
}

impl<G: PropSignature> PropExpr<G> {
    /// Source arity of this morphism.
    ///
    /// A `Braid` or `Tensor` whose halves sum past `usize::MAX` saturates
    /// there rather than overflowing; `usize::MAX` matches no real wire
    /// bundle, so the composition checks report
    /// [`CatgraphError::CompositionSizeMismatch`] instead of wrapping.
    /// Saturation is not injective: two independently overflowing arities
    /// both read `usize::MAX` and compare equal (e.g. in `compose`'s arity
    /// check) — the composition-arity invariant on [`PropExpr`] covers this.
    #[must_use]
    pub fn source(&self) -> usize {
        match self {
            PropExpr::Identity(n) => *n,
            PropExpr::Braid(m, n) => m.saturating_add(*n),
            PropExpr::Generator(g) => g.source(),
            PropExpr::Compose(f, _) => f.source(),
            PropExpr::Tensor(f, g) => f.source().saturating_add(g.source()),
        }
    }

    /// Target arity of this morphism.
    ///
    /// Saturates at `usize::MAX` on overflow, exactly as [`PropExpr::source`].
    #[must_use]
    pub fn target(&self) -> usize {
        match self {
            PropExpr::Identity(n) => *n,
            PropExpr::Braid(m, n) => m.saturating_add(*n),
            PropExpr::Generator(g) => g.target(),
            PropExpr::Compose(_, g) => g.target(),
            PropExpr::Tensor(f, g) => f.target().saturating_add(g.target()),
        }
    }

    /// The exact `(source, target)` arity pair of this subterm, or `None` if
    /// **any** `Braid` or `Tensor` width anywhere in it sums past `usize::MAX`.
    ///
    /// The checked companion to [`source`](PropExpr::source) /
    /// [`target`](PropExpr::target), which saturate and so cannot tell an
    /// overflowed width from a genuine `usize::MAX` one. Where the arity is
    /// compared against a real slice length that distinction does not matter —
    /// `usize::MAX` matches nothing either way. Where it is used as a
    /// **magnitude** — sizing a `Vec`, a range bound, a matrix dimension — it
    /// does, and the consumer screens with this instead (#196).
    ///
    /// Unlike `source`/`target`, this inspects the whole subterm on every
    /// variant, including the half of a `Compose` that does not contribute to
    /// the requested boundary: an overflow buried there is still an overflow.
    #[must_use]
    pub fn checked_arities(&self) -> Option<(usize, usize)> {
        match self {
            PropExpr::Identity(n) => Some((*n, *n)),
            PropExpr::Braid(m, n) => {
                let width = m.checked_add(*n)?;
                Some((width, width))
            }
            PropExpr::Generator(g) => Some((g.source(), g.target())),
            PropExpr::Compose(f, g) => {
                let (f_source, _) = f.checked_arities()?;
                let (_, g_target) = g.checked_arities()?;
                Some((f_source, g_target))
            }
            PropExpr::Tensor(f, g) => {
                let (f_source, f_target) = f.checked_arities()?;
                let (g_source, g_target) = g.checked_arities()?;
                Some((
                    f_source.checked_add(g_source)?,
                    f_target.checked_add(g_target)?,
                ))
            }
        }
    }

    /// Whether every arity sum in this subterm fits in `usize` — i.e. whether
    /// [`source`](PropExpr::source) and [`target`](PropExpr::target) report
    /// exact values rather than the saturated sentinel.
    ///
    /// This is the domain of the passes that consume an arity as a magnitude:
    /// [`content_of`] and [`nf`] reject outside it, and the equality APIs above
    /// them gate on it to stay total (#196).
    ///
    /// It screens overflow, not magnitude: a huge arity written *literally*
    /// (e.g. `Identity(usize::MAX)`) involves no sum and passes, yet is just as
    /// infeasible for those consumers — bounding it stays the caller's
    /// obligation (#197).
    ///
    /// [`content_of`]: crate::prop::presentation::content::content_of
    /// [`nf`]: crate::prop::presentation::smc_nf::nf
    #[must_use]
    pub fn arities_fit(&self) -> bool {
        self.checked_arities().is_some()
    }

    /// The braiding network shared by both #258 constructors.
    ///
    /// Split out of the trait impl rather than duplicated, so that
    /// `from_permutation_on_domain` and `from_permutation_on_codomain` cannot
    /// drift apart on a carrier where they are required to coincide. See
    /// [`SymmetricMonoidalMorphism::from_permutation_on_domain`] for the
    /// convention and the S-functor square.
    fn braiding_expr(p: &Permutation, arity: usize) -> Result<Self, CatgraphError> {
        let n = arity;
        if p.len() != n {
            return Err(CatgraphError::Composition {
                message: format!(
                    "PropExpr::from_permutation: permutation has len {} but {n} types provided",
                    p.len(),
                ),
            });
        }
        // Input-indexed: the wire at position i is routed to position perm[i].
        let perm: Vec<usize> = (0..n).map(|i| p.apply(i)).collect();
        let mut expr = PropExpr::Identity(n);
        for t in adjacent_swaps(&perm) {
            // A swap at t exchanges positions t and t+1 of an n-element array,
            // so t + 1 < n and the right-hand identity has width n - t - 2 ≥ 0.
            // Both subtractions are `checked_` so that a future change to
            // `adjacent_swaps` can only surface as a loud panic on a stated
            // invariant, never as a wrapped (release) or differently-panicking
            // (debug) width — the arithmetic itself cannot be slipped past.
            let right = n
                .checked_sub(t)
                .and_then(|rest| rest.checked_sub(2))
                .expect("invariant: adjacent_swaps only emits t with t + 1 < n, so t + 2 <= n");
            let layer = Free::<G>::tensor(
                Free::<G>::tensor(Free::identity(t), Free::braid(1, 1)),
                Free::identity(right),
            );
            expr = Free::<G>::compose(expr, layer)?;
        }
        Ok(expr)
    }
}

/// Marker type for the *prop itself* (the category). Values of `Prop<G>` are
/// [`PropExpr<G>`]. See module docs for the equality caveat.
pub struct Prop<G: PropSignature>(PhantomData<G>);

/// Smart-constructor namespace producing well-formed [`PropExpr<G>`] values
/// — morphisms of the free prop on signature `G`.
pub struct Free<G: PropSignature>(PhantomData<G>);

impl<G: PropSignature> Free<G> {
    /// `id_n : n → n`.
    #[must_use]
    pub fn identity(n: usize) -> PropExpr<G> {
        PropExpr::Identity(n)
    }

    /// Symmetric braiding `σ_{m,n} : m+n → m+n`.
    #[must_use]
    pub fn braid(m: usize, n: usize) -> PropExpr<G> {
        PropExpr::Braid(m, n)
    }

    /// Generator inclusion `g ∈ G ↪ Free(G)`.
    #[must_use]
    pub fn generator(g: G) -> PropExpr<G> {
        PropExpr::Generator(g)
    }

    /// Sequential composition `f ; g` with arity check.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::CompositionSizeMismatch`] if
    /// `f.target() != g.source()`.
    pub fn compose(f: PropExpr<G>, g: PropExpr<G>) -> Result<PropExpr<G>, CatgraphError> {
        if f.target() != g.source() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: f.target(),
                actual: g.source(),
            });
        }
        Ok(PropExpr::Compose(Box::new(f), Box::new(g)))
    }

    /// Parallel tensor `f ⊗ g`. Arity sums trivially; no failure case.
    #[must_use]
    pub fn tensor(f: PropExpr<G>, g: PropExpr<G>) -> PropExpr<G> {
        PropExpr::Tensor(Box::new(f), Box::new(g))
    }
}

/// Bubble-sort `perm` ascending; return the positions of the adjacent
/// transpositions in the order performed. `adjacent_swaps(p)[i] = t` means the
/// `i`-th swap exchanged positions `t` and `t + 1`.
///
/// This is the shared decomposition core behind all four call sites, which
/// split by how their perm is indexed:
///
/// - **Input-indexed, consumed forward** — [`crate::mat_to_sfg`]'s
///   `permutation_sfg` and `PropExpr`'s
///   [`SymmetricMonoidalMorphism::from_permutation`], which map each `t` to a
///   braid layer `id_t ⊗ σ ⊗ id_{k-t-2}` in swap order (the first over
///   [`SignalFlowGraph`](crate::sfg::SignalFlowGraph), the second over
///   [`PropExpr`]; same network, same convention).
/// - **Output-indexed, consumed REVERSED** — [`presentation::smc_nf`]'s
///   `decompose_braid` + `canonicalize_run`, which build `Layer<G>` values from
///   the reversed sequence: their perms say which *input* each output position
///   holds, so the sort word undoes the braid and its reversal rebuilds it.
///
/// `O(k²)` swaps for `k = perm.len()`; the empty vector is returned when `perm`
/// is already sorted (including `k ≤ 1`). Applying the returned swaps to `perm`
/// left-to-right yields the ascending sort.
pub(crate) fn adjacent_swaps(perm: &[usize]) -> Vec<usize> {
    let k = perm.len();
    let mut arr = perm.to_vec();
    let mut swaps = Vec::new();
    for pass in 0..k {
        for t in 0..k.saturating_sub(pass + 1) {
            if arr[t] > arr[t + 1] {
                arr.swap(t, t + 1);
                swaps.push(t);
            }
        }
    }
    swaps
}

// ---- Integration with catgraph trait hierarchy -------------------------------

/// Objects of a prop are natural numbers, encoded as `Vec<()>` of the
/// corresponding length so that `PropExpr<G>` can implement
/// `Composable<Vec<()>>` uniformly with the rest of the workspace.
fn as_object(n: usize) -> Vec<()> {
    vec![(); n]
}

impl<G: PropSignature> HasIdentity<Vec<()>> for PropExpr<G> {
    fn identity(on_this: &Vec<()>) -> Self {
        PropExpr::Identity(on_this.len())
    }
}

impl<G: PropSignature> Composable<Vec<()>> for PropExpr<G> {
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        if self.target() != other.source() {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: self.target(),
                actual: other.source(),
            });
        }
        Ok(PropExpr::Compose(
            Box::new(self.clone()),
            Box::new(other.clone()),
        ))
    }

    fn domain(&self) -> Vec<()> {
        as_object(self.source())
    }

    fn codomain(&self) -> Vec<()> {
        as_object(self.target())
    }
}

impl<G: PropSignature> Monoidal for PropExpr<G> {
    fn monoidal(&mut self, other: Self) {
        let lhs = std::mem::replace(self, PropExpr::Identity(0));
        *self = PropExpr::Tensor(Box::new(lhs), Box::new(other));
    }
}

impl<G: PropSignature> SymmetricMonoidalMorphism<()> for PropExpr<G> {
    /// The pure-braiding morphism `n → n` realizing the permutation `p`, as a
    /// network of adjacent transpositions.
    ///
    /// **Convention:** a wire entering at position `i` exits at position
    /// `perm[i] = p.apply(i)` — the perm is *input-indexed*. Under the functor
    /// `S` of F&S Thm 5.53 ([`crate::sfg_to_mat`]) the result is therefore the
    /// permutation matrix with `entries[i][p.apply(i)] = 1`, i.e. exactly
    /// [`MatR::permutation_matrix`](crate::mat::MatR::permutation_matrix); the
    /// two are checked against each other over all permutations of `n = 3` and
    /// `n = 4` in `tests/prop.rs` ([#252]).
    ///
    /// **Construction:** `perm` is bubble-sorted into ascending order by the
    /// shared `adjacent_swaps` core, and each adjacent swap at position `t`
    /// becomes one braid layer `Identity(t) ⊗ Braid(1, 1) ⊗ Identity(n-t-2)`,
    /// composed in the swap order (input side first). `O(n²)` layers for `n`
    /// wires; the identity permutation (including `n ≤ 1`) sorts with no swaps
    /// and yields `Identity(n)`.
    ///
    /// Those layers are `O(n²)` *deep*, not merely `O(n²)` many: the result is a
    /// left-nested `Compose` spine, and every consumer walks it recursively
    /// (`sfg_to_mat`, `content_of`'s builder, and the derived `Drop`/`PartialEq`
    /// /`Hash`). A few hundred wires is enough to overflow a default stack. This
    /// is the same shape `permutation_sfg` already produces, and bounding arity
    /// magnitude remains the caller's obligation
    /// ([#197](https://github.com/sustia-llc/catgraph/issues/197)) — noted here
    /// because a layer *count* reads as a cost note and this one is also a depth.
    ///
    /// The sibling construction is `permutation_sfg` in [`crate::mat_to_sfg`],
    /// which builds the same network over [`SignalFlowGraph`] from the same
    /// core and the same input-indexed convention. [`presentation::smc_nf`]'s
    /// `decompose_braid` consumes the core REVERSED instead, because its perms
    /// are output-indexed — see the `adjacent_swaps` doc comment.
    ///
    /// `types` contributes only its length: objects of a prop are natural
    /// numbers (encoded `Vec<()>`), so there is no color to place on one side
    /// or the other. The two #258 constructors —
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// and
    /// [`from_permutation_on_codomain`](SymmetricMonoidalMorphism::from_permutation_on_codomain)
    /// — therefore return the same morphism, and both route through this
    /// helper. Same story on every single-sorted carrier in this crate
    /// ([`MatR`](crate::mat::MatR), [`MatKron`](crate::mat_kron::MatKron)).
    ///
    /// **This is the whole workspace's convention.** The wiring built here —
    /// domain wire `i` routed to codomain wire `p.apply(i)` — is the one
    /// `Cospan`, `Span`, `Corel` and `FrobeniusMorphism` build too.
    ///
    /// ⚠ The pre-#258 version of this comment said the opposite: that
    /// `M::from_permutation(p, types, true)` gave "`p` on `PropExpr`/`MatR` and
    /// `p⁻¹` on `Cospan`/`Corel`". That was **wrong**, and worth recording
    /// because it is an easy mistake to re-make. It read the `p.inv()` in
    /// `Cospan`'s builder as the realized permutation, but that `p.inv()` is
    /// the cospan's *right leg index vector*: connectivity in a cospan runs
    /// through the apex, so domain `i` meets codomain `k` when
    /// `left[i] == right[k]`, i.e. when `k == p.apply(i)`. The inversion in the
    /// leg is what keeps the wiring un-inverted. Independent check on
    /// `p = rotation_left(3, 1)` over `['A','B','C']`: `Cospan`'s codomain is
    /// `['C','A','B']`, so the label at domain position 0 (`'A'`) sits at
    /// codomain position 1 — and `p.apply(0) == 1`.
    ///
    /// So the `S`-functor square
    /// `sfg_to_mat(from_permutation_*(p)) == MatR::from_permutation_*(p)`
    /// (F&S Thm 5.53) holds on **both** named methods, and it holds because
    /// both carriers agree with the cospan family rather than by deviating
    /// together. The exhaustive `n = 3` / `n = 4` oracle in `tests/prop.rs`
    /// checks it against a literal hand-written matrix on the `MatR` side, so a
    /// symmetric drift cannot pass it.
    ///
    /// **Construction cost.** Those layers are `O(n²)` *deep*, not merely
    /// `O(n²)` many: the result is a left-nested `Compose` spine, and every
    /// consumer walks it recursively (`sfg_to_mat`, `content_of`'s builder, and
    /// the derived `Drop`/`PartialEq`/`Hash`). A few hundred wires is enough to
    /// overflow a default stack. This is the same shape `permutation_sfg`
    /// already produces, and bounding arity magnitude remains the caller's
    /// obligation ([#197](https://github.com/sustia-llc/catgraph/issues/197)).
    ///
    /// [`SignalFlowGraph`]: crate::sfg::SignalFlowGraph
    /// [#252]: https://github.com/sustia-llc/catgraph/issues/252
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `p.len() != types.len()`.
    ///
    /// The layer composition can in principle also return
    /// [`CatgraphError::CompositionSizeMismatch`], but every layer is built at
    /// width `n` against a running term of width `n`, so it cannot occur; the
    /// `?` mirrors `permutation_sfg`.
    fn from_permutation_on_domain(p: Permutation, types: &[()]) -> Result<Self, CatgraphError> {
        Self::braiding_expr(&p, types.len())
    }

    /// The same morphism as
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// — see there for why the two coincide on a single-sorted carrier, and for
    /// the S-functor square.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if `p.len() != types.len()`.
    fn from_permutation_on_codomain(p: Permutation, types: &[()]) -> Result<Self, CatgraphError> {
        Self::braiding_expr(&p, types.len())
    }

    /// Permute the wires on one side of `self`, in place: postcompose a
    /// braiding onto the codomain (`of_codomain = true`) or precompose one onto
    /// the domain (`of_codomain = false`).
    ///
    /// Source and target *counts* are invariant either way — the spliced
    /// braiding is an endomorphism of the permuted side's arity.
    ///
    /// # Which side gets `p` and which gets `p.inv()`
    ///
    /// The semantics of record are [`MatR`](crate::mat::MatR)'s `permute_side`:
    /// *right-multiply by `P` to permute columns (codomain side); left-multiply
    /// by `Pᵀ` to permute rows (domain side)*. `MatR`'s composition **is**
    /// `matmul` and [`sfg_to_mat`](crate::sfg_to_mat) sends `Compose(f, g)` to
    /// `S(f).matmul(S(g))`, so `Compose(_, b)` is right-multiplication by `b`
    /// and `Compose(b, _)` is left-multiplication by it. With
    /// `P = MatR::permutation_matrix(p)` — exactly what
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// realizes under `S` — that reads:
    ///
    /// - **codomain:** `self ; from_permutation_on_domain(p)` (`self · P`)
    /// - **domain:** `from_permutation_on_domain(p.inv()) ; self` (`Pᵀ · self`)
    ///
    /// The domain side takes `p.inv()` because `Pᵀ = P⁻¹` for a permutation
    /// matrix, and `permutation_matrix` is the *only* thing this impl can
    /// splice: `MatR`'s `Pᵀ` has `entries[p.apply(i)][i] = 1`, while
    /// `permutation_matrix(p.inv())` has `entries[i][p.inv().apply(i)] = 1` —
    /// substituting `i = p.apply(j)` turns the second into the first, so they
    /// are the same matrix. **The two sides are therefore not symmetric**, and
    /// passing `p` on both would silently transpose one of them.
    ///
    /// The inversion is applied to the *permutation*, and since #258 that is
    /// the only place it could go: the direction flag is gone, and the two
    /// named constructors coincide on this single-sorted carrier, so neither
    /// of them can produce `p⁻¹` from `p`.
    ///
    /// # Cost
    ///
    /// One
    /// [`from_permutation_on_domain`](SymmetricMonoidalMorphism::from_permutation_on_domain)
    /// network per call: `O(n²)` braid layers for `n` wires on the permuted
    /// side. Those layers are `O(n²)` **deep**, not merely `O(n²)` many — the
    /// same spine hazard that constructor documents, since the layers form a
    /// left-nested `Compose` spine that every consumer walks recursively
    /// (`sfg_to_mat`, `content_of`'s builder, and the derived `Drop` /
    /// `PartialEq` / `Hash`). A few hundred wires is enough to overflow a
    /// default stack; bounding arity magnitude remains the caller's obligation
    /// ([#197](https://github.com/sustia-llc/catgraph/issues/197)).
    ///
    /// # Length mismatch
    ///
    /// The trait signature is non-fallible, so a `p` whose length does not
    /// match the permuted side's arity — a caller bug — leaves `self`
    /// **unchanged** rather than panicking, matching `MatR`'s defensive no-op.
    ///
    /// # Oracle
    ///
    /// The `S`-functor square
    /// `sfg_to_mat(f.permute_side(p, side)) == sfg_to_mat(f).permute_side(p, side)`
    /// is asserted for both values of `side` over every permutation of `n = 3`
    /// and `n = 4` in `tests/prop.rs`
    /// ([#252](https://github.com/sustia-llc/catgraph/issues/252)). Before that
    /// fix this method spliced `PropExpr::Braid(0, n)` — which is `Identity(n)`,
    /// as `presentation::smc_nf`'s Step 0 rewrite states outright — so every
    /// call was a silent no-op.
    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        // Precompose (domain side) or postcompose (codomain side) with the
        // braiding network of the appropriate arity. Source/target counts
        // remain invariant because braids are endomorphisms.
        let n = if of_codomain {
            self.target()
        } else {
            self.source()
        };
        if p.len() != n {
            // Invariant: callers should only pass permutations that match
            // the side being permuted. A length mismatch is a caller bug,
            // so we leave `self` unchanged (defensive) rather than panic.
            return;
        }
        // Codomain side postcomposes `P`; domain side precomposes `Pᵀ = P⁻¹`,
        // which is `from_permutation_on_domain(p.inv())` — see the doc comment.
        // The inversion has to be spelled out here: the two named constructors
        // coincide on this carrier, so neither of them supplies it.
        let perm = if of_codomain { p.clone() } else { p.inv() };
        let types = as_object(n);
        let braid: PropExpr<G> =
            <Self as SymmetricMonoidalMorphism<()>>::from_permutation_on_domain(perm, &types)
                .expect(
                    "invariant: from_permutation_on_domain's only error is a \
                     p.len() != types.len() mismatch, and p.len() == n == types.len() was just \
                     checked (p.inv() preserves length)",
                );
        let old = std::mem::replace(self, PropExpr::Identity(0));
        *self = if of_codomain {
            PropExpr::Compose(Box::new(old), Box::new(braid))
        } else {
            PropExpr::Compose(Box::new(braid), Box::new(old))
        };
    }
}

pub mod colored;
pub mod presentation;

#[cfg(test)]
mod tests {
    use super::adjacent_swaps;

    /// Apply a swap sequence to `perm` left-to-right and return the result.
    fn apply_swaps(perm: &[usize], swaps: &[usize]) -> Vec<usize> {
        let mut arr = perm.to_vec();
        for &t in swaps {
            arr.swap(t, t + 1);
        }
        arr
    }

    #[test]
    fn empty_perm_has_no_swaps() {
        assert_eq!(adjacent_swaps(&[]), Vec::<usize>::new());
    }

    #[test]
    fn single_element_has_no_swaps() {
        assert_eq!(adjacent_swaps(&[0]), Vec::<usize>::new());
    }

    #[test]
    fn already_sorted_has_no_swaps() {
        assert_eq!(adjacent_swaps(&[0, 1, 2, 3]), Vec::<usize>::new());
    }

    #[test]
    fn full_reversal() {
        // [2,1,0] bubble-sorts via swaps at positions 0, 1, 0.
        assert_eq!(adjacent_swaps(&[2, 1, 0]), vec![0, 1, 0]);
    }

    #[test]
    fn transpose_riffle_perm() {
        // mat_to_sfg's L3 regrouping for a 2×3 matrix: input-major i*cols+j
        // routed to output-major j*rows+i (rows=2, cols=3).
        let mut perm = vec![0usize; 6];
        for i in 0..2 {
            for j in 0..3 {
                perm[i * 3 + j] = j * 2 + i;
            }
        }
        assert_eq!(perm, vec![0, 2, 4, 1, 3, 5]);
        let swaps = adjacent_swaps(&perm);
        assert_eq!(apply_swaps(&perm, &swaps), vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn swaps_sort_the_permutation() {
        // Property: applying the returned swaps to `perm` yields the sorted array.
        let cases: &[&[usize]] = &[
            &[],
            &[0],
            &[1, 0],
            &[2, 0, 1],
            &[3, 2, 1, 0],
            &[0, 2, 4, 1, 3, 5],
            &[4, 3, 2, 1, 0],
        ];
        for &perm in cases {
            let mut sorted = perm.to_vec();
            sorted.sort_unstable();
            let swaps = adjacent_swaps(perm);
            assert_eq!(apply_swaps(perm, &swaps), sorted, "perm = {perm:?}");
        }
    }
}
