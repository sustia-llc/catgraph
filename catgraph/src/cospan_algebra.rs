//! Cospan-algebras: lax symmetric monoidal functors from cospans to sets
//! (Fong-Spivak §2.1, Definition 2.2).
//!
//! A cospan-algebra `(Λ, a)` consists of a label set `Λ` and a lax symmetric
//! monoidal functor `a: Cospan_Λ → C` for some target category `C`.
//!
//! The [`CospanAlgebra`] trait captures this functoriality element-wise:
//! - [`map_cospan`](CospanAlgebra::map_cospan) transforms a single element under a cospan
//! - [`lax_monoidal`](CospanAlgebra::lax_monoidal) combines elements from `a(x)` and `a(y)` into `a(x ⊕ y)`
//! - [`unit`](CospanAlgebra::unit) provides the element of `a(I)`
//!
//! ## Implementations
//!
//! - [`PartitionAlgebra`]: the initial cospan-algebra where `a(x) = Cospan(0, x)` (Example 2.3)
//! - [`NameAlgebra`]: `a(x) = H(I, P(x))` — named morphisms via the compact closed structure (Prop 3.2)

use std::fmt::Debug;

use crate::{
    category::{Composable, HasIdentity},
    cospan::Cospan,
    errors::CatgraphError,
    monoidal::Monoidal,
};

/// A lax symmetric monoidal functor `a: Cospan_Λ → C`, operating element-wise.
///
/// `Elem` is the type of elements in the target sets `a(x)`.
/// The functor maps each cospan `c: m → p ← n` to a function `a(c): a(m) → a(n)`,
/// realized by [`map_cospan`](Self::map_cospan) applied to individual elements.
pub trait CospanAlgebra<Lambda: Eq + Copy + Debug> {
    /// Element type in the target category.
    type Elem;

    /// Apply the functorial action of a cospan to an element.
    ///
    /// Given `c: m → p ← n` and `e ∈ a(dom(c))`, produces `a(c)(e) ∈ a(cod(c))`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the element is incompatible with the cospan's domain.
    fn map_cospan(
        &self,
        cospan: &Cospan<Lambda>,
        element: &Self::Elem,
    ) -> Result<Self::Elem, CatgraphError>;

    /// Lax monoidal coherence map: `a(x) × a(y) → a(x ⊕ y)`.
    ///
    /// Combines an element from `a(x)` and an element from `a(y)` into
    /// an element of `a(x ⊕ y)`.
    fn lax_monoidal(&self, a: &Self::Elem, b: &Self::Elem) -> Self::Elem;

    /// Unit coherence: the distinguished element of `a(I) = a([])`.
    fn unit(&self) -> Self::Elem;
}

// ---------------------------------------------------------------------------
// PartitionAlgebra (Example 2.3)
// ---------------------------------------------------------------------------

/// The partition cospan-algebra: `Part_Λ(x) = Cospan_Λ(0, x)`.
///
/// An element of `Part(x)` is a cospan from `[]` to `x` — it describes a way
/// to partition `x` into labeled groups. This is the initial cospan-algebra
/// (every cospan-algebra receives a unique map from `Part`).
///
/// - `map_cospan`: pushout composition `e ; c` where `e: [] → m` and `c: m → p ← n`.
/// - `lax_monoidal`: monoidal product of cospans.
/// - `unit`: the empty cospan `[] → [] ← []`.
#[derive(Default)]
pub struct PartitionAlgebra;

impl<Lambda> CospanAlgebra<Lambda> for PartitionAlgebra
where
    Lambda: Eq + Copy + Debug,
{
    type Elem = Cospan<Lambda>;

    fn map_cospan(
        &self,
        cospan: &Cospan<Lambda>,
        element: &Self::Elem,
    ) -> Result<Self::Elem, CatgraphError> {
        // element: [] → m (a partition of the domain)
        // cospan: m → p ← n
        // result: element ; cospan = [] → p ← n ... but we want the
        // induced element in a(cod(c)), which is a cospan [] → n.
        //
        // Composing element ([] → m) with cospan (m → n via pushout)
        // gives a cospan [] → n, which is an element of Part(n).
        element.compose(cospan)
    }

    fn lax_monoidal(&self, a: &Self::Elem, b: &Self::Elem) -> Self::Elem {
        let mut result = a.clone();
        result.monoidal(b.clone());
        result
    }

    fn unit(&self) -> Self::Elem {
        Cospan::empty()
    }
}

// ---------------------------------------------------------------------------
// NameAlgebra (§4.1: A_H(x) = H(I, P(x)))
// ---------------------------------------------------------------------------

use crate::frobenius::{
    FrobeniusMorphism, FrobeniusOperation, from_decomposition, special_frobenius_morphism,
};

/// The name cospan-algebra: `A_H(x) = H(I, P(x))` — named morphisms.
///
/// An element of `A_H(x)` is a `FrobeniusMorphism` with domain `[]` and codomain `x`
/// (a "name" in the sense of Fong-Spivak Prop 3.2).
///
/// - `map_cospan`: interprets the cospan as a Frobenius morphism via
///   [`from_decomposition`], then composes with the named element.
/// - `lax_monoidal`: monoidal product of morphisms.
/// - `unit`: identity on `[]`.
///
/// The `BlackBoxLabel` type parameter is carried for compatibility with the
/// Frobenius morphism infrastructure.
pub struct NameAlgebra<BlackBoxLabel: Eq + Clone + Send + Sync> {
    _phantom: std::marker::PhantomData<BlackBoxLabel>,
}

impl<BlackBoxLabel: Eq + Clone + Send + Sync> NameAlgebra<BlackBoxLabel> {
    /// Create a new `NameAlgebra` instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<BlackBoxLabel: Eq + Clone + Send + Sync> Default for NameAlgebra<BlackBoxLabel> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Lambda, BlackBoxLabel> CospanAlgebra<Lambda> for NameAlgebra<BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    type Elem = FrobeniusMorphism<Lambda, BlackBoxLabel>;

    fn map_cospan(
        &self,
        cospan: &Cospan<Lambda>,
        element: &Self::Elem,
    ) -> Result<Self::Elem, CatgraphError> {
        // element: [] → domain(cospan) as a FrobeniusMorphism
        // cospan: domain → codomain
        // We interpret the cospan as a FrobeniusMorphism and compose.
        let cospan_morph: FrobeniusMorphism<Lambda, BlackBoxLabel> = cospan_to_frobenius(cospan)?;
        let mut result = element.clone();
        crate::category::ComposableMutating::compose(&mut result, cospan_morph)?;
        Ok(result)
    }

    fn lax_monoidal(&self, a: &Self::Elem, b: &Self::Elem) -> Self::Elem {
        let mut result = a.clone();
        result.monoidal(b.clone());
        result
    }

    fn unit(&self) -> Self::Elem {
        FrobeniusMorphism::identity(&vec![])
    }
}

// ---------------------------------------------------------------------------
// Lemma 4.3: A_F natural transformation from a hypergraph functor F: H → H'
// ---------------------------------------------------------------------------

/// Lemma 4.3 — induced cospan-algebra morphism `α: A_H → A_H'` from a hypergraph
/// functor `F: H → H'`.
///
/// The element-wise formula is `α_x(e) := F(e)` — a simple pointwise application
/// of the functor to each element of `A_H(x) = H(I, P(x))`. The content of
/// Lemma 4.3 is that this pointwise map is automatically a monoidal natural
/// transformation between the two name-algebras whenever `F` preserves
/// composition, tensor product, and Frobenius structure.
///
/// The paper states Lemma 4.3 for an **io** (identity-on-objects) hypergraph
/// functor over a fixed `Λ`; this function additionally accepts cross-label
/// functors (`L1 ≠ L2`, e.g. `RelabelingFunctor`) — a beyond-paper
/// generalization in the direction of the paper's Eq (29) naturality square
/// (the cross-Λ `Cospan_f` machinery itself is not implemented; see
/// `docs/FS19-AUDIT.md` and #109).
///
/// This free function is the direct embodiment of the paper's construction:
/// it is a thin wrapper around `F.map_mor(e)` whose purpose is to make the
/// Lemma 4.3 correspondence explicit at the type level. Callers plug in any
/// [`HypergraphFunctor`](crate::hypergraph_functor::HypergraphFunctor) impl and
/// obtain the induced algebra map for free.
///
/// # Type parameters
///
/// - `L1`: source label set
/// - `L2`: target label set
/// - `Src`: source category morphism type (must impl
///   [`HypergraphCategory<L1>`](crate::hypergraph_category::HypergraphCategory))
/// - `Tgt`: target category morphism type (must impl
///   [`HypergraphCategory<L2>`](crate::hypergraph_category::HypergraphCategory))
/// - `F`: the hypergraph functor
///
/// # Errors
///
/// Propagates any [`CatgraphError`] returned by `functor.map_mor`.
///
/// # Verification
///
/// Naturality, monoidality, and unit preservation of the induced morphism are
/// verified as proptests in `tests/cospan_algebra.rs` for two concrete cases:
///
/// - `F = RelabelingFunctor` on `Cospan<L1> → Cospan<L2>` with two
///   [`PartitionAlgebra`] instances
/// - `F = CospanToFrobeniusFunctor` on `Cospan<L> → FrobeniusMorphism<L, BL>`
///   relating [`PartitionAlgebra`] to [`NameAlgebra`]
pub fn functor_induced_algebra_map<L1, L2, Src, Tgt, F>(
    functor: &F,
    element: &Src,
) -> Result<Tgt, CatgraphError>
where
    L1: Eq + Copy + Debug,
    L2: Eq + Copy + Debug,
    Src: crate::hypergraph_category::HypergraphCategory<L1>,
    Tgt: crate::hypergraph_category::HypergraphCategory<L2>,
    F: crate::hypergraph_functor::HypergraphFunctor<L1, L2, Src, Tgt>,
{
    functor.map_mor(element)
}

/// Convert a `Cospan<Lambda>` into a `FrobeniusMorphism` by decomposing
/// each leg through epi-mono factorization (Fong-Spivak Lemma 3.6).
///
/// This is the morphism-mapping component of the hypergraph functor
/// `Cospan_Λ → FrobeniusMorphism_Λ` (Prop 3.8).
///
/// # Errors
///
/// Returns [`CatgraphError`] if the epi-mono decomposition fails for either leg.
pub fn cospan_to_frobenius<Lambda, BlackBoxLabel>(
    cospan: &Cospan<Lambda>,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    use crate::category::ComposableMutating;
    use crate::finset::Decomposition;

    let domain = cospan.domain();
    let codomain = cospan.codomain();
    let middle = cospan.middle();
    let middle_len = middle.len();

    // Identity fast path.
    //
    // `left == right` on its own is *not* enough: the all-merged cospan
    // `[a,a] → {•} ← [a,a]` also satisfies it (both legs are `[0, 0]`), yet it
    // is the 2→2 spider, not the identity. The cospan is the identity (up to
    // apex isomorphism) exactly when the common leg is a *bijection* onto the
    // apex (#285). `[a] → {•,•} ← [a]` with both legs `[0]` also fails the new
    // guard — its apex carries a node no leg hits — and takes the general
    // route below, which emits `η;ε` for the spare node. Since #350 that
    // scalar **survives**: the answer is a depth-2 term carrying the bubble,
    // not `identity(['a'])`, so for this shape #285 changed the output as well
    // as the code path (pinned by `cospan_to_frobenius_unhit_apex_node_keeps_the_bubble`).
    //
    // `domain == codomain` is implied by `legs_agree` (both boundaries are
    // `middle[leg[i]]`), so it is not a separate conjunct. `domain()` above has
    // already indexed `middle` by every leg entry, so an out-of-range leg
    // (reachable only through `Cospan::new_unchecked`) panics there, before
    // this guard; the `get_mut` form below is merely total, not a defense.
    let leg = cospan.left_to_middle();
    let legs_agree = leg == cospan.right_to_middle();
    // A leg of exactly `middle_len` entries hitting each apex node at most once
    // hits every one of them exactly once.
    let leg_is_bijection = leg.len() == middle_len && {
        let mut hit = vec![false; middle_len];
        leg.iter().all(|&i| match hit.get_mut(i) {
            Some(slot) => !std::mem::replace(slot, true),
            None => false,
        })
    };
    if legs_agree && leg_is_bijection {
        return Ok(FrobeniusMorphism::identity(&domain));
    }

    // Compute leftover for a leg: codomain elements beyond max image index.
    let leftover = |map: &[usize]| -> usize {
        if map.is_empty() {
            middle_len
        } else {
            let max_idx = map.iter().copied().max().unwrap_or(0);
            middle_len.saturating_sub(max_idx + 1)
        }
    };

    // Build the left leg: domain → middle
    let left_map = cospan.left_to_middle().to_vec();
    let left_leftover = leftover(&left_map);
    let left_decomp = Decomposition::try_from((left_map, left_leftover)).map_err(|e| {
        CatgraphError::Composition {
            message: format!("left leg decomposition failed: {e}"),
        }
    })?;
    let left_morph: FrobeniusMorphism<Lambda, BlackBoxLabel> =
        from_decomposition(left_decomp, &domain, middle)?;

    // Build the right leg: codomain → middle, then flip to get middle → codomain
    let right_map = cospan.right_to_middle().to_vec();
    let right_leftover = leftover(&right_map);
    let right_decomp = Decomposition::try_from((right_map, right_leftover)).map_err(|e| {
        CatgraphError::Composition {
            message: format!("right leg decomposition failed: {e}"),
        }
    })?;
    let mut right_morph: FrobeniusMorphism<Lambda, BlackBoxLabel> =
        from_decomposition(right_decomp, &codomain, middle)?;
    // hflip reverses the morphism (dagger in the Frobenius sense).
    right_morph.hflip(&std::convert::identity);

    // Compose: domain → middle → codomain
    let mut result = left_morph;
    ComposableMutating::compose(&mut result, right_morph)?;
    Ok(result)
}

/// Interpret a single Frobenius generator as a `Cospan<Lambda>`.
///
/// The four Frobenius generators and the identity come straight from
/// [`HypergraphCategory`](crate::hypergraph_category::HypergraphCategory); the
/// braiding `σ: [z, w] → [w, z]` is the two-vertex apex whose right leg is the
/// transposition. `Spider(z, d1, d2)` is interpreted by recursing on the
/// generator decomposition [`special_frobenius_morphism`] gives it, so the
/// spider's semantics here are the crate's own rather than a second guess at
/// them.
///
/// The `(0, 0)` arm builds the bubble `η;ε` directly instead of recursing.
/// That carve-out was **load-bearing** until #350: `special_frobenius_morphism`
/// returns the *simplified* term, and the simplifier's rule 3 cancelled `η;ε`
/// under the extra-special axiom, so recursing returned the empty cospan for a
/// non-identity scalar. Rule 3 is gone and the carve-out is now **measured
/// redundant** — disabling this arm and letting `(0, 0)` recurse leaves
/// `cargo test -p catgraph` green. It is kept so this function's bubble does
/// not depend on what the layer simplifier does to a two-layer term; expect
/// cargo-mutants to score its deletion MISSED, and read that as accurate.
fn generator_to_cospan<Lambda, BlackBoxLabel>(
    op: &FrobeniusOperation<Lambda, BlackBoxLabel>,
) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    use crate::category::Composable;
    use crate::hypergraph_category::HypergraphCategory;

    Ok(match op {
        FrobeniusOperation::Unit(z) => Cospan::unit(*z),
        FrobeniusOperation::Counit(z) => Cospan::counit(*z),
        FrobeniusOperation::Multiplication(z) => Cospan::multiplication(*z),
        FrobeniusOperation::Comultiplication(z) => Cospan::comultiplication(*z),
        FrobeniusOperation::Identity(z) => Cospan::identity(&vec![*z]),
        // σ: [z, w] → [w, z]. Correct by construction: both legs are
        // permutations of `0..2` and the apex has exactly two vertices.
        FrobeniusOperation::SymmetricBraiding(z, w) => {
            Cospan::new_unchecked(vec![0, 1], vec![1, 0], vec![*z, *w])
        }
        // The bubble, built directly rather than through the simplifier's
        // output. Redundant since #350 (measured — see this function's docs),
        // kept so the `(0, 0)` scalar does not depend on the layer simplifier.
        // Pinned by `tests::scfm_equal_scalars_have_equal_images`.
        FrobeniusOperation::Spider(z, 0, 0) => Cospan::unit(*z).compose(&Cospan::counit(*z))?,
        FrobeniusOperation::Spider(z, d1, d2) => frobenius_to_cospan(
            &special_frobenius_morphism::<Lambda, BlackBoxLabel>(*d1, *d2, *z),
        )?,
        FrobeniusOperation::UnSpecifiedBox(_, srcs, tgts) => {
            return Err(CatgraphError::Interpret {
                context: format!(
                    "frobenius_to_cospan has no interpretation for an \
                     UnSpecifiedBox ({} in, {} out): Cospan is the free \
                     hypergraph category on the generators alone; resolve it \
                     through a MorphismSystem first",
                    srcs.len(),
                    tgts.len()
                ),
            });
        }
    })
}

/// Interpret a `FrobeniusMorphism<Lambda, _>` as a `Cospan<Lambda>` — inverse
/// in spirit to [`cospan_to_frobenius`].
///
/// Each layer is the monoidal product of its blocks' generator cospans, in block
/// order, and the morphism is the pushout composite of those layers. The
/// `Frobenius`-valued twin of this function is
/// [`Frobenius::interpret_frob`](crate::frobenius::Frobenius::interpret_frob),
/// which has the same shape; `Cospan` is [`Composable`], not
/// [`ComposableMutating`](crate::category::ComposableMutating), so it cannot
/// implement `Frobenius` and reuse that body.
///
/// It is also re-exported as
/// [`frobenius::frobenius_to_cospan`](crate::frobenius::frobenius_to_cospan).
/// That path briefly named a *second* implementation of the same map, added
/// under #283 on a branch parallel to the one adding this one; #336 unified
/// them after measuring that they agree over 363 terms while both bodies were
/// live (383 now, in `frobenius::to_cospan_pin`, where the retired algorithm
/// lives on as the oracle). ⚠ Two things changed for callers of the
/// `frobenius::` path: the
/// bounds now require `Send + Sync`, and an `UnSpecifiedBox` is rejected with
/// [`CatgraphError::Interpret`] rather than `CatgraphError::Composition`.
///
/// # Deciding Def 2.5 equality
///
/// Because [`Cospan`] carries no presentation — only the apex quotient — this is
/// the decision procedure for Def 2.5 equality of two *parallel*
/// `FrobeniusMorphism`s: compare `frobenius_to_cospan(f)?.canonical_form()` with
/// `g`'s. `FrobeniusMorphism`'s own `PartialEq` cannot serve there. It compares
/// **presentations**, and it separates both sides of every one of the eleven
/// Def 2.5 equations — `tests/frobenius_axioms.rs` measures that, and runs the
/// battery through this function for the `FrobeniusMorphism` carrier.
///
/// # Scalars, and composition
///
/// The interpretation is of the diagram **as presented**. `FrobeniusMorphism`'s
/// own [`compose`](crate::category::ComposableMutating::compose) runs
/// `two_layer_simplify`, which until #350 carried a unit/counit rule deleting
/// an `η` feeding directly into an `ε` — the *extra*-special move. `Cospan` is
/// the theory of **special** commutative Frobenius monoids and keeps that
/// bubble as a genuine scalar, so a term carrying that pattern reached this
/// function having already lost a bubble the semantics kept:
/// `interp(f # g) != interp(f) # interp(g)`. Deleting the rule closed that gap,
/// and the same test now measures the two routes **agreeing** on the witness
/// that used to separate them
/// (`tests/frobenius_axioms.rs::frobenius_scalar_loop_survives_to_interpretation`).
///
/// Agreement on one witness is not commutation in general, and nothing here
/// proves the normalizer sound: the claim that the
/// remaining rules are sound has been wrong before. `Spider(z, m, 0)` followed by
/// `Spider(z, 0, k)` used to fuse into `Spider(z, m, k)`, wiring two disconnected
/// components together — a *connectivity* change, strictly worse than a dropped
/// scalar. Rule 4 now requires `n >= 1` (with a redundant zero-output filter on
/// the placement lookup), and `spider_fusion_needs_a_wire_between_the_two_spiders`
/// in the same test file holds the *behaviour* there — the conjunction of the two
/// defenses, not either alone. Treat divergences as things to be measured one at
/// a time.
///
/// # What Prop 3.8 does and does not license here
///
/// F&S 2019 Prop 3.8 is a one-to-one correspondence between special commutative
/// Frobenius monoids (SCFMs) in a symmetric monoidal category `C` and strict
/// symmetric monoidal functors `(Cospan, ⊕) → (C, ⊗)`. **Both** of its
/// directions concern functors *out of* `Cospan`, so neither of them is this
/// map. What it licenses is this: the black-box-free part of
/// `FrobeniusMorphism_Λ` is the free SCFM prop on `Λ`, `Cospan_Λ` carries an
/// SCFM structure on each object `[l]` (Example 2.8), and Prop 3.8 turns that
/// structure into the interpreting functor this function computes.
///
/// **Both sides are now the *special* theory (#350).** Until then they were
/// not: `Cospan` keeps the bubble `η;ε` as a genuine `0 → 0` non-identity,
/// while `FrobeniusMorphism`'s layer simplifier carried rule 3 and quotiented
/// by the *extra-special* axiom `ε ∘ η = id_I` on top of SCFM. That axiom is
/// not among the nine equations of Def 2.5 (F&S 2019 §2), and adding it is a
/// **quotient**, not an extension — Baez–Erbele's description of the free
/// symmetric monoidal category on a commutative *extra-special* Frobenius
/// monoid (equivalence relations on `X ⊔ Y`) matches what this crate calls
/// [`Corel`](crate::corel::Corel), a carrier kept apart from `Cospan`
/// everywhere else. Nothing in-tree proves that identification; it is a match
/// of descriptions, not a theorem. So the two ends of this map named two
/// different theories, and the measured unsoundness and incompleteness on
/// scalars were that mismatch showing through. Deleting rule 3 removes it, at
/// the cost named in the CHANGELOG: `η;ε` no longer collapses.
///
/// Both refuting witnesses are gone, measured — they are now pinned the other
/// way up in this module's `tests::scalar_bubbles_survive_in_both_directions`:
///
/// - The **soundness** witness. A spelled `η;ε` and `Spider(z, 0, 0)` are the
///   same scalar; they used to interpret to `apex 0` and `apex 1` respectively,
///   and now both give the bubble (`apex 1`, one scalar class). Also pinned in
///   `tests::scfm_equal_scalars_have_equal_images`.
/// - The **completeness** witness. A spelled `η;ε` and
///   `FrobeniusMorphism::identity(&vec![])` used to share the empty cospan
///   though they are not SCFM-equal; they now have different images
///   (`apex 1` against `apex 0`).
///
/// ⚠ Two refuted counterexamples are not a proof: nothing here establishes
/// that this map is sound or complete for SCFM-equality in general, and the
/// normalizer's remaining rules have been wrong before (see the spider-fusion
/// paragraph above). What the measurements support is the narrower statement
/// that the *known* scalar divergences are closed. See also
/// [`cospan_canon`](crate::cospan_canon)'s module docs. Read
/// [`Cospan::canonical_form`](crate::cospan_canon::CospanCanon) on these images
/// as a *semantic* equality — far better than `==` on `FM`, whose derived `Eq`
/// compares layer vectors and so separates diagrams that are equal on any
/// reading.
///
/// # Examples
///
/// Two syntactically different terms, equal under the Frobenius axioms:
///
/// ```
/// use catgraph::category::ComposableMutating;
/// use catgraph::cospan_algebra::frobenius_to_cospan;
/// use catgraph::frobenius::{FrobeniusMorphism, FrobeniusOperation};
///
/// type FM = FrobeniusMorphism<char, String>;
///
/// // μ ; δ, spelled out …
/// let mut spelled: FM = FrobeniusOperation::Multiplication('a').into();
/// spelled.compose(FrobeniusOperation::Comultiplication('a').into()).unwrap();
/// // … and the same map as a single 2-to-2 spider.
/// let spider: FM = FrobeniusOperation::Spider('a', 2, 2).into();
///
/// assert_eq!(
///     frobenius_to_cospan(&spelled).unwrap().canonical_form(),
///     frobenius_to_cospan(&spider).unwrap().canonical_form(),
/// );
/// ```
///
/// # Errors
///
/// - [`CatgraphError::Interpret`] if the morphism contains an
///   [`UnSpecifiedBox`](FrobeniusOperation::UnSpecifiedBox): a black box denotes
///   nothing in the free hypergraph category. Pass it through
///   [`MorphismSystem`](crate::frobenius::MorphismSystem) first if it needs one.
///   The message names the generator and its arities; both are pinned in
///   `frobenius::to_cospan_pin::black_boxes_are_rejected_by_both`.
/// - [`CatgraphError::Interpret`] if a layer has no blocks but a non-empty
///   interface (a malformed morphism). A block-free layer whose interface is
///   empty is `id_I` and is interpreted, not rejected — that is how
///   `FrobeniusMorphism::identity(&vec![])` is represented.
/// - Any [`CatgraphError`] from the underlying cospan composition.
pub fn frobenius_to_cospan<Lambda, BlackBoxLabel>(
    morphism: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
) -> Result<Cospan<Lambda>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    use crate::category::ComposableMutating;

    let mut answer = Cospan::identity(&ComposableMutating::domain(morphism));
    for layer in &morphism.layers {
        // A block-free layer is `id_I`, which is how `FrobeniusMorphism::identity`
        // represents the identity on the empty type list — legal, and the unit of
        // the fold below. A block-free layer with a *non-empty* interface is
        // malformed, and says so rather than silently contributing `id_I`.
        let mut current = match layer.blocks.first() {
            Some(first) => generator_to_cospan(&first.op)?,
            None => {
                if !layer.left_type.is_empty() || !layer.right_type.is_empty() {
                    return Err(CatgraphError::Interpret {
                        context: format!(
                            "block-free FrobeniusMorphism layer with a {}→{} interface",
                            layer.left_type.len(),
                            layer.right_type.len()
                        ),
                    });
                }
                Cospan::identity(&Vec::new())
            }
        };
        for block in layer.blocks.iter().skip(1) {
            current.monoidal(generator_to_cospan(&block.op)?);
        }
        answer = answer.compose(&current)?;
    }
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::{Composable, ComposableMutating, HasIdentity};

    // --- PartitionAlgebra ---

    #[test]
    fn partition_unit_is_empty_cospan() {
        let alg = PartitionAlgebra;
        let u: Cospan<char> = alg.unit();
        assert!(u.domain().is_empty());
        assert!(u.codomain().is_empty());
    }

    #[test]
    fn partition_identity_cospan_preserves_element() {
        let alg = PartitionAlgebra;
        // element: [] → [a, b]
        let element = Cospan::new(vec![], vec![0, 1], vec!['a', 'b']).unwrap();
        let id = Cospan::<char>::identity(&vec!['a', 'b']);
        let mapped = alg.map_cospan(&id, &element).unwrap();
        assert_eq!(mapped.domain(), element.domain());
        assert_eq!(mapped.codomain(), element.codomain());
    }

    #[test]
    fn partition_composition_is_sequential() {
        let alg = PartitionAlgebra;
        // element: [] → [a]
        let element = Cospan::new(vec![], vec![0], vec!['a']).unwrap();
        // c1: [a] → [a, a] (identity into a merge)
        let c1 = Cospan::<char>::identity(&vec!['a']);
        // Map through identity
        let mapped = alg.map_cospan(&c1, &element).unwrap();
        assert_eq!(mapped.codomain(), vec!['a']);
    }

    #[test]
    fn partition_lax_monoidal_is_tensor() {
        let alg = PartitionAlgebra;
        let a = Cospan::new(vec![], vec![0], vec!['a']).unwrap();
        let b = Cospan::new(vec![], vec![0], vec!['b']).unwrap();
        let combined = alg.lax_monoidal(&a, &b);
        assert!(combined.domain().is_empty());
        assert_eq!(combined.codomain(), vec!['a', 'b']);
    }

    // --- NameAlgebra ---

    type FM = FrobeniusMorphism<char, String>;

    #[test]
    fn name_unit_is_empty_identity() {
        let alg = NameAlgebra::<String>::new();
        let u: FM = alg.unit();
        assert!(u.domain().is_empty());
        assert!(u.codomain().is_empty());
    }

    #[test]
    fn name_identity_cospan_preserves_element() {
        let alg = NameAlgebra::<String>::new();
        // element: [] → [a] (a named morphism — use unit η: [] → [a])
        let element: FM = crate::frobenius::FrobeniusOperation::Unit('a').into();
        let id_cospan = Cospan::<char>::identity(&vec!['a']);
        let mapped = alg.map_cospan(&id_cospan, &element).unwrap();
        assert_eq!(mapped.domain(), element.domain());
        assert_eq!(mapped.codomain(), element.codomain());
    }

    #[test]
    fn name_lax_monoidal_is_tensor() {
        let alg = NameAlgebra::<String>::new();
        let a: FM = crate::frobenius::FrobeniusOperation::Unit('a').into();
        let b: FM = crate::frobenius::FrobeniusOperation::Unit('b').into();
        let combined = alg.lax_monoidal(&a, &b);
        assert!(combined.domain().is_empty());
        assert_eq!(combined.codomain(), vec!['a', 'b']);
    }

    #[test]
    fn name_map_through_merge_cospan() {
        let alg = NameAlgebra::<String>::new();
        // element: [] → [a, a] (name of something with codomain [a,a])
        let element: FM = crate::compact_closed::cup_single('a');
        assert!(element.domain().is_empty());
        assert_eq!(element.codomain(), vec!['a', 'a']);

        // merge cospan: [a, a] → [a] (both inputs map to same middle node)
        let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']).unwrap();
        let mapped = alg.map_cospan(&merge, &element).unwrap();
        assert!(mapped.domain().is_empty());
        assert_eq!(mapped.codomain(), vec!['a']);
    }

    #[test]
    fn cospan_to_frobenius_identity() {
        let types = vec!['a', 'b'];
        let id = Cospan::<char>::identity(&types);
        let morph: FM = cospan_to_frobenius(&id).unwrap();
        assert_eq!(morph.domain(), vec!['a', 'b']);
        assert_eq!(morph.codomain(), vec!['a', 'b']);
        assert!(
            morph == FM::identity(&types),
            "F(id) must be the identity morphism, not merely 2→2: depth={}",
            morph.depth()
        );
    }

    #[test]
    fn cospan_to_frobenius_merge() {
        // [a, a] → [a]: both left nodes map to middle node 0
        let merge = Cospan::new(vec![0, 0], vec![0], vec!['a']).unwrap();
        let morph: FM = cospan_to_frobenius(&merge).unwrap();
        assert_eq!(morph.domain(), vec!['a', 'a']);
        assert_eq!(morph.codomain(), vec!['a']);
        let mu: FM = crate::frobenius::FrobeniusOperation::Multiplication('a').into();
        assert!(morph == mu, "F(merge) must be μ: depth={}", morph.depth());
    }

    #[test]
    fn cospan_to_frobenius_split() {
        // [a] → [a, a]: right nodes both map to middle node 0
        let split = Cospan::new(vec![0], vec![0, 0], vec!['a']).unwrap();
        let morph: FM = cospan_to_frobenius(&split).unwrap();
        assert_eq!(morph.domain(), vec!['a']);
        assert_eq!(morph.codomain(), vec!['a', 'a']);
        let delta: FM = crate::frobenius::FrobeniusOperation::Comultiplication('a').into();
        assert!(
            morph == delta,
            "F(split) must be δ: depth={}",
            morph.depth()
        );
    }

    /// The identity fast path must not fire on an all-merged cospan (#285).
    ///
    /// `[a,a] → {•} ← [a,a]` satisfies `domain == codomain` and
    /// `left == right == [0, 0]`, yet it is the 2→2 spider. Guarding only on
    /// those two conditions returned `identity` (depth 1) for it; the correct
    /// answer is `special_frobenius_morphism(2, 2, 'a')` (depth 2). The same
    /// confusion hit `[a,a,a] → {•} ← [a,a,a]` (identity depth 1 vs spider
    /// depth 4).
    ///
    /// Scope: the four `m = n` single-apex cospans for `m ∈ {1,…,4}`. The old
    /// guard also fired at `m = 0` (the bubble — empty legs, empty boundaries);
    /// that case is left to `scalar_bubbles_survive_in_both_directions`, which
    /// pins the bubble the general route now returns there. The `m ≠ n` cases
    /// were never affected by the guard and are pinned in
    /// `tests/hypergraph_functor.rs::ctf_single_apex_cospan_is_the_spider`.
    #[test]
    fn cospan_to_frobenius_all_merged_is_not_the_identity() {
        use crate::frobenius::special_frobenius_morphism;

        for m in 1usize..=4 {
            let c = Cospan::new(vec![0; m], vec![0; m], vec!['a']).unwrap();
            let morph: FM = cospan_to_frobenius(&c).unwrap();
            let spider: FM = special_frobenius_morphism(m, m, 'a');
            assert!(
                morph == spider,
                "F(all-merged {m}→{m}) must be spider({m},{m}) (depth {}), \
                 got depth {}",
                spider.depth(),
                morph.depth(),
            );
            if m > 1 {
                let types: Vec<char> = vec!['a'; m];
                assert!(
                    morph != FM::identity(&types),
                    "F(all-merged {m}→{m}) must not be the identity: \
                     the {m} input wires are all connected to one another"
                );
            }
        }
    }

    /// A cospan whose apex carries a node neither leg hits is still handled,
    /// keeps the boundary of the identity it wraps, and — since #350 — keeps
    /// the spare node as a scalar.
    ///
    /// The **known gap this test used to carry is closed.** Until #350 it could
    /// not pin which code path produced the answer: the old fast path returned
    /// `identity(['a'])` directly, and the general route the #285 guard sends
    /// this shape down emitted `η ; ε` for the spare node which rule 3 then
    /// simplified away, so both returned `identity(['a'])` and reverting the
    /// #285 guard left this test green. With rule 3 deleted the two routes give
    /// different answers, so the `depth`/non-identity assertions below now
    /// distinguish them: reverting the #285 guard makes this test red.
    ///
    /// **Space of the claim:** the single witness `[a] → {•,•} ← [a]`, one
    /// label. It ranges over totality, the resulting boundary, and the presence
    /// of exactly the one extra layer the spare node contributes — not over
    /// multi-node leftovers, mixed labels, or the interpretation back into
    /// `Cospan` (`scalar_bubbles_survive_in_both_directions` covers that).
    #[test]
    fn cospan_to_frobenius_unhit_apex_node_keeps_the_bubble() {
        // [a] → {•, •} ← [a], right/left legs both hit node 0 only.
        let c = Cospan::new(vec![0], vec![0], vec!['a', 'a']).unwrap();
        assert_eq!(c.middle().len(), 2, "the witness has a spare apex node");
        assert_eq!(c.left_to_middle(), c.right_to_middle());
        let morph: FM = cospan_to_frobenius(&c).unwrap();
        assert_eq!(morph.domain(), vec!['a']);
        assert_eq!(morph.codomain(), vec!['a']);
        assert_eq!(
            morph.depth(),
            2,
            "the spare apex node must reach the term as η;ε: got depth {} \
             (rule 3, and the pre-#285 fast path, both gave the depth-1 identity)",
            morph.depth()
        );
        assert!(
            morph != FM::identity(&vec!['a']),
            "an unhit apex node is a scalar, not nothing: {morph:?}"
        );
    }

    // --- frobenius_to_cospan (#284) ---

    /// Every generator lands on the cospan `HypergraphCategory` names for it.
    ///
    /// **Space:** the four Frobenius generators, the identity on one and two
    /// wires, and the braiding — i.e. every `FrobeniusOperation` variant except
    /// `Spider` (covered below) and `UnSpecifiedBox` (rejected, below). One
    /// label pair `('a', 'b')` only.
    #[test]
    fn frobenius_to_cospan_sends_generators_to_generators() {
        use crate::hypergraph_category::HypergraphCategory;

        let cases: Vec<(&str, FM, Cospan<char>)> = vec![
            (
                "eta",
                FrobeniusOperation::Unit('a').into(),
                Cospan::unit('a'),
            ),
            (
                "epsilon",
                FrobeniusOperation::Counit('a').into(),
                Cospan::counit('a'),
            ),
            (
                "mu",
                FrobeniusOperation::Multiplication('a').into(),
                Cospan::multiplication('a'),
            ),
            (
                "delta",
                FrobeniusOperation::Comultiplication('a').into(),
                Cospan::comultiplication('a'),
            ),
            (
                "id_a",
                FrobeniusOperation::Identity('a').into(),
                Cospan::identity(&vec!['a']),
            ),
            (
                "braid",
                FrobeniusOperation::SymmetricBraiding('a', 'b').into(),
                Cospan::new(vec![0, 1], vec![1, 0], vec!['a', 'b']).unwrap(),
            ),
        ];
        for (label, term, expected) in cases {
            let got = frobenius_to_cospan(&term).unwrap();
            assert_eq!(
                got.canonical_form(),
                expected.canonical_form(),
                "{label}: got apex {} / {:?}, expected apex {} / {:?}",
                got.middle().len(),
                got.canonical_form().classes(),
                expected.middle().len(),
                expected.canonical_form().classes(),
            );
        }
    }

    /// Prop 3.8 as a round trip: `cospan → frobenius → cospan` returns the
    /// cospan you started with, up to apex isomorphism.
    ///
    /// This is what makes [`frobenius_to_cospan`] usable as a *semantic oracle*
    /// (see `tests/compact_closed.rs`): it is checked against
    /// [`cospan_to_frobenius`], written independently in this module, rather
    /// than against itself.
    ///
    /// **Space:** the nine cospans below, over `{'a','b'}` — identities, all
    /// four generators, a merge-and-pass, and two scalar-carrying shapes (a
    /// bare bubble, and `id_a` beside a bubble). The scalar cases were
    /// *excluded* until #350, when rule 3 stopped erasing them on the way out;
    /// `scalar_bubbles_survive_in_both_directions` below is where they are
    /// measured against the values they used to have, and this list carries
    /// them only so the round trip is not silently scalar-free. The converse
    /// round trip (`frobenius → cospan → frobenius`) is *not* claimed: it
    /// cannot hold on the nose, since many layer vectors share one cospan.
    #[test]
    fn cospan_frobenius_cospan_round_trips() {
        use crate::hypergraph_category::HypergraphCategory;

        let cases: Vec<(&str, Cospan<char>)> = vec![
            ("id_ab", Cospan::identity(&vec!['a', 'b'])),
            ("id_empty", Cospan::identity(&vec![])),
            ("eta", Cospan::unit('a')),
            ("epsilon", Cospan::counit('a')),
            ("mu", Cospan::multiplication('a')),
            ("delta", Cospan::comultiplication('a')),
            (
                "merge_and_pass",
                Cospan::new(vec![0, 0], vec![0, 1], vec!['a', 'a']).unwrap(),
            ),
            // Scalar-carrying, reachable only since #350.
            ("bubble", Cospan::new(vec![], vec![], vec!['a']).unwrap()),
            (
                "id_a_beside_bubble",
                Cospan::new(vec![0], vec![0], vec!['a', 'b']).unwrap(),
            ),
        ];
        for (label, cospan) in cases {
            let term: FM = cospan_to_frobenius(&cospan).unwrap();
            let back = frobenius_to_cospan(&term).unwrap();
            assert_eq!(
                back.canonical_form(),
                cospan.canonical_form(),
                "{label}: round trip gave apex {} (scalars {}), started from apex {} (scalars {})",
                back.canonical_form().apex_len(),
                back.canonical_form().scalar_count(),
                cospan.canonical_form().apex_len(),
                cospan.canonical_form().scalar_count(),
            );
        }
    }

    /// The **sound** direction, pinned on the pair that refuted it.
    ///
    /// `Spider(z, 0, 0)` *is* the bubble `η;ε`, and `η ; δ ; (ε ⊗ ε)` is
    /// SCFM-equal to it (counitality on one leg of `δ`). Soundness —
    /// SCFM-equal terms have equal images — therefore requires equal canonical
    /// forms here. Before the `generator_to_cospan` fix this was **false**:
    /// the spider arm interpreted `special_frobenius_morphism`'s *simplified*
    /// output, in which the then-live rule 3 had already cancelled the `η;ε`,
    /// so the spider gave `apex 0 / scalars 0` against the spelled term's
    /// `apex 1 / scalars 1`. `Cospan` is the **special**, not extra-special,
    /// theory: the bubble is a genuine `0 → 0` non-identity and must survive.
    /// Since #350 the term algebra agrees, and the `(0, 0)` arm is a measured
    /// redundancy rather than the repair (see `generator_to_cospan`'s docs).
    ///
    /// **Space:** the `0 → 0` scalar and the same scalar beside `id_a`. The
    /// second case shows the failure was not confined to the empty object: it
    /// is an `a → a` endomorphism — the interface of `id_a` and `delta_then_mu`,
    /// two of the ten `samples()` in `tests/compact_closed.rs`, which is why an
    /// interface-only check could not see a bubble beside an identity.
    /// (`samples()` itself is scalar-free; the compact_closed pins do not
    /// exercise this shape.)
    #[test]
    fn scfm_equal_scalars_have_equal_images() {
        use crate::monoidal::Monoidal;

        // η ; δ ; (ε ⊗ ε) — the bubble, spelled out.
        let mut spelled: FM = FrobeniusOperation::Unit('a').into();
        spelled
            .compose(FrobeniusOperation::Comultiplication('a').into())
            .unwrap();
        let mut counits: FM = FrobeniusOperation::Counit('a').into();
        counits.monoidal(FrobeniusOperation::Counit('a').into());
        spelled.compose(counits).unwrap();

        let bubble: FM = FrobeniusOperation::Spider('a', 0, 0).into();

        let spelled_canon = frobenius_to_cospan(&spelled).unwrap().canonical_form();
        let bubble_canon = frobenius_to_cospan(&bubble).unwrap().canonical_form();
        assert_eq!(
            bubble_canon,
            spelled_canon,
            "SCFM-equal 0→0 scalars must have equal images: \
             Spider(a,0,0) gave apex {} scalars {}, η;δ;(ε⊗ε) gave apex {} scalars {}",
            bubble_canon.apex_len(),
            bubble_canon.scalar_count(),
            spelled_canon.apex_len(),
            spelled_canon.scalar_count(),
        );
        assert_eq!(
            bubble_canon.scalar_count(),
            1,
            "the bubble is a scalar, not the empty cospan"
        );

        // Not confined to the empty object: `id_a ⊗ bubble` against
        // `id_a ⊗ (spelled bubble)`, two SCFM-equal `a → a` terms.
        let mut beside_bubble: FM = FrobeniusOperation::Identity('a').into();
        beside_bubble.monoidal(bubble);
        let mut beside_spelled: FM = FrobeniusOperation::Identity('a').into();
        beside_spelled.monoidal(spelled);

        let beside_bubble_canon = frobenius_to_cospan(&beside_bubble)
            .unwrap()
            .canonical_form();
        let beside_spelled_canon = frobenius_to_cospan(&beside_spelled)
            .unwrap()
            .canonical_form();
        assert_eq!(
            beside_bubble_canon,
            beside_spelled_canon,
            "id_a ⊗ scalar: apex {} scalars {} vs apex {} scalars {}",
            beside_bubble_canon.apex_len(),
            beside_bubble_canon.scalar_count(),
            beside_spelled_canon.apex_len(),
            beside_spelled_canon.scalar_count(),
        );
    }

    /// Spiders go through the crate's own generator decomposition, so an
    /// `(m, n)` spider is one apex vertex joining all `m + n` boundary wires.
    ///
    /// **Space:** `(m, n)` for `m, n <= 3` **except `(0, 0)`**, label `'a'` only.
    /// `(0, 0)` is the bubble: since the `generator_to_cospan` fix it interprets
    /// to apex 1 with one scalar (pinned by `scfm_equal_scalars_have_equal_images`),
    /// which the `scalar_count() == 0` assertion here does not fit (it *is* one
    /// apex vertex, so the apex assertion alone would pass), so it is pinned
    /// there rather than quietly skipped.
    #[test]
    fn frobenius_to_cospan_spiders_are_single_apex_vertices() {
        for m in 0..=3usize {
            for n in 0..=3usize {
                if m == 0 && n == 0 {
                    continue;
                }
                let spider: FM = FrobeniusOperation::Spider('a', m, n).into();
                let canon = frobenius_to_cospan(&spider).unwrap().canonical_form();
                assert_eq!(canon.dom_len(), m, "spider({m},{n}) domain");
                assert_eq!(canon.cod_len(), n, "spider({m},{n}) codomain");
                assert_eq!(
                    canon.apex_len(),
                    1,
                    "spider({m},{n}) should be one apex vertex, got {} ({:?})",
                    canon.apex_len(),
                    canon.classes()
                );
                assert_eq!(canon.scalar_count(), 0, "spider({m},{n}) scalar count");
            }
        }
    }

    /// Scalars survive **both** maps, and the two witnesses that used to refute
    /// soundness and completeness are pinned the other way up (#350).
    ///
    /// `Cospan` is the theory of **special**, not extra-special, commutative
    /// Frobenius monoids: the closed bubble `η # ε` is a genuine `0 → 0`
    /// non-identity and `k` bubbles are distinguished from `k-1` (see
    /// [`cospan_canon`](crate::cospan_canon)'s module docs). Until #350 neither
    /// `cospan_to_frobenius` nor `frobenius_to_cospan` preserved that, for two
    /// causes, both now closed:
    ///
    /// 1. ~~`cospan_to_frobenius`'s identity fast path~~ — **closed by #285.**
    ///    Its guard was `domain == codomain && left_to_middle() ==
    ///    right_to_middle()`, so it returned `identity(&domain)` and discarded
    ///    every apex vertex neither leg reaches. The guard now also requires
    ///    the common leg to be a bijection onto the apex, which reaches every
    ///    apex vertex by construction (a bubble makes `leg.len() < middle_len`,
    ///    so `0 → 0` with a bubble falls through too).
    /// 2. ~~`two_layer_simplify` rule 3~~ — **closed by #350.** It cancelled
    ///    `η(z);ε(z)` outright, the *extra-special* axiom. The decomposition
    ///    path emits one `η;ε` per unreached apex vertex and rule 3 ate each,
    ///    so a bubble interpreted to nothing and a spelled-out `η;ε` to
    ///    `apex_len() == 0`. `FrobeniusMorphism` is now the special theory and
    ///    each bubble is carried through as spelled.
    ///
    /// **This pin signals rule 3.** Restoring it turns every assertion below
    /// red, and each message carries the measured value it used to have: the
    /// two `0 → 0` cospans become the same term, the `id_a`-beside-a-bubble
    /// case comes back as `identity(['a'])`, and the spelled `η;ε` interprets
    /// to `apex 0`. The #285 fast path cannot fire on any witness here
    /// (`leg.len() < middle_len` in each), so nothing below is shared with
    /// cause 1.
    ///
    /// **Space of the claim:** six witnesses — one bubble, two bubbles, `id_a`
    /// beside a `'b'` bubble, a spelled `η;ε`, `Spider(a, 0, 0)`, and
    /// `FM::identity(&vec![])` — at one or two labels, compared through
    /// `canonical_form`. It does *not* range
    /// over scalars beside larger diagrams, over three or more bubbles, or over
    /// any non-scalar shape; and two closed counterexamples are not a proof
    /// that the maps are sound or complete (see [`frobenius_to_cospan`]'s docs).
    #[test]
    fn scalar_bubbles_survive_in_both_directions() {
        let one_bubble = Cospan::new(vec![], vec![], vec!['a']).unwrap();
        let two_bubbles = Cospan::new(vec![], vec![], vec!['a', 'a']).unwrap();
        assert_eq!(one_bubble.canonical_form().scalar_count(), 1);
        assert_eq!(two_bubbles.canonical_form().scalar_count(), 2);
        assert_ne!(
            one_bubble.canonical_form(),
            two_bubbles.canonical_form(),
            "as cospans, one bubble and two are different morphisms"
        );

        // Each bubble reaches the term algebra as `η;ε` and stays there, so the
        // two cospans have different terms and each round-trips.
        let one_term: FM = cospan_to_frobenius(&one_bubble).unwrap();
        let two_term: FM = cospan_to_frobenius(&two_bubbles).unwrap();
        assert!(
            one_term != two_term,
            "one bubble and two must not share a term (got {one_term:?} and \
             {two_term:?}); under rule 3 both were the empty term"
        );
        for (label, cospan, term) in [
            ("one_bubble", &one_bubble, &one_term),
            ("two_bubbles", &two_bubbles, &two_term),
        ] {
            let back = frobenius_to_cospan(term).unwrap().canonical_form();
            assert_eq!(
                back,
                cospan.canonical_form(),
                "{label}: round trip gave apex {} / scalars {}, the cospan has \
                 apex {} / scalars {} (rule 3 gave apex 0 for both)",
                back.apex_len(),
                back.scalar_count(),
                cospan.canonical_form().apex_len(),
                cospan.canonical_form().scalar_count(),
            );
        }

        // Away from 0 → 0: the fast path cannot fire on `id_a` beside a bubble
        // (`leg.len() == 1 < middle_len == 2` — not a bijection), so the
        // decomposition path runs and emits η('b');ε('b') in the middle.
        let id_a_and_bubble = Cospan::new(vec![0], vec![0], vec!['a', 'b']).unwrap();
        assert_eq!(id_a_and_bubble.canonical_form().apex_len(), 2);
        assert_eq!(id_a_and_bubble.canonical_form().scalar_count(), 1);
        let kept: FM = cospan_to_frobenius(&id_a_and_bubble).unwrap();
        assert!(
            kept != FM::identity(&vec!['a']),
            "an unreached apex vertex is a scalar at arity 1 too, not just at \
             0→0: got depth {} (rule 3 returned the depth-1 identity)",
            kept.depth()
        );
        let kept_back = frobenius_to_cospan(&kept).unwrap().canonical_form();
        assert_eq!(
            kept_back,
            id_a_and_bubble.canonical_form(),
            "round trip gave apex {} / scalars {}, the cospan has 2 / 1 \
             (rule 3 gave apex 1 / scalars 0)",
            kept_back.apex_len(),
            kept_back.scalar_count(),
        );

        // The soundness witness, both halves: a spelled `η;ε` and the (0,0)
        // spider are the same scalar and must have the same image.
        let mut eta_eps: FM = FrobeniusOperation::Unit('a').into();
        eta_eps
            .compose(FrobeniusOperation::Counit('a').into())
            .unwrap();
        let eta_eps_canon = frobenius_to_cospan(&eta_eps).unwrap().canonical_form();
        let spider_0_0: FM = FrobeniusOperation::Spider('a', 0, 0).into();
        let spider_canon = frobenius_to_cospan(&spider_0_0).unwrap().canonical_form();
        assert_eq!(
            (eta_eps_canon.apex_len(), eta_eps_canon.scalar_count()),
            (1, 1),
            "η;ε gave apex {} / scalars {}, the cospan bubble is 1 / 1 \
             (rule 3 cancelled it to 0 / 0)",
            eta_eps_canon.apex_len(),
            eta_eps_canon.scalar_count(),
        );
        assert_eq!(
            spider_canon,
            eta_eps_canon,
            "SCFM-equal scalars must share an image: Spider(a,0,0) gave apex \
             {} / scalars {}, η;ε gave apex {} / scalars {}",
            spider_canon.apex_len(),
            spider_canon.scalar_count(),
            eta_eps_canon.apex_len(),
            eta_eps_canon.scalar_count(),
        );

        // The completeness witness: `η;ε` and `id_I` are not SCFM-equal, and no
        // longer share an image.
        let id_empty_canon = frobenius_to_cospan(&FM::identity(&vec![]))
            .unwrap()
            .canonical_form();
        assert_eq!(id_empty_canon.apex_len(), 0, "id_I has an empty apex");
        assert_ne!(
            eta_eps_canon, id_empty_canon,
            "η;ε and id_I are not SCFM-equal and must not share an image \
             (both were apex 0 under rule 3)"
        );
    }

    /// A black box has no image in the free hypergraph category, and says so
    /// rather than inventing one.
    #[test]
    fn frobenius_to_cospan_rejects_black_boxes() {
        let boxed: FM =
            FrobeniusOperation::UnSpecifiedBox("f".to_string(), vec!['a'], vec!['b']).into();
        let err = frobenius_to_cospan(&boxed).unwrap_err();
        assert!(
            matches!(err, CatgraphError::Interpret { .. }),
            "expected Interpret, got {err:?}"
        );
    }
}
