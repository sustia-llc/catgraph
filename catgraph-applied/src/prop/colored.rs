//! Λ-colored well-formedness for free-prop expressions
//! ([#79](https://github.com/sustia-llc/catgraph/issues/79) P1).
//!
//! F&S 2019 **Def 3.9** takes the objects of a Λ-colored symmetric monoidal
//! category to be the free monoid `List(Λ)` (an *objectwise-free* structure:
//! `List(Λ) ≅ Ob(C)`), and **Thm 3.14** builds `Cospan_Λ` as the free
//! hypergraph category on Λ. Interfaces are therefore **words** over Λ, not
//! bare natural numbers; `Color = ()` collapses `List(Λ)` back to `ℕ` and
//! recovers the single-sorted prop of F&S 2018 Def 5.25.
//!
//! # Why a check pass and not a smart constructor
//!
//! [`PropExpr::Identity`] and [`PropExpr::Braid`] carry only a width. They are
//! *color-polymorphic*: `id_n` spans `n` wires of whatever colors flow in, and
//! a braid permutes whatever it is handed. A bare `Identity(2)` has no
//! intrinsic word, so there is nothing for a smart constructor to check.
//! Colors instead **flow top-down**: given the diagram's source word, every
//! internal boundary word is derived by threading it through the tree. That is
//! [`check`], and it is what makes the colored morphism the *pair*
//! `(source word, expression)` — [`ColoredExpr`].
//!
//! The word discipline is the one written up Λ-generically in
//! `docs/SMC-NF-RECONCILIATION.md` **§4.1**: words live in `Λ*`, `⊗`
//! concatenates, braids are discrete cospans with a permuted anchor, and
//! identities/braids "carry whatever colors flow in". The shipped
//! monochromatic signatures are the instance `Λ = {•}`, spelled `Color = ()`
//! (see [`super::mono_word`]).
//!
//! # Equality
//!
//! [`ColoredExpr::eq_colored`] is the SMC-quotient equality: layered-normal-form
//! equality (`presentation::smc_nf::nf`) **plus** boundary-word equality. The
//! derived `PartialEq` on [`ColoredExpr`] is the *pre-quotient*, structural one
//! — same caveat as [`PropExpr`] itself (see the [module docs] of the parent
//! module).
//!
//! Cited, not re-derived: `docs/SMC-NF-RECONCILIATION.md` **§4.2** (Lemma 4.1
//! — content decides SMC-equality, stated color-generically over an arbitrary
//! Λ, so the word-level reading here inherits it) and **§4.3** (Lemma 4.2 —
//! every pipeline rewrite preserves content, giving the *unconditional*
//! soundness direction `nf(e) = nf(e′) ⇒ e =_SMC e′` on every diagram).
//! The converse is **not** established: §4.4 records that the draft
//! canonicality theorem was refuted on the fragment `𝔉` itself, and what
//! survives is probe-verified rather than proven. So a `true` from
//! `eq_colored` is sound; a `false` is not a proof of distinctness.
//!
//! [module docs]: super
//! [`PropExpr::Identity`]: super::PropExpr::Identity
//! [`PropExpr::Braid`]: super::PropExpr::Braid

use catgraph::errors::CatgraphError;

use super::presentation::smc_nf::nf;
use super::{PropExpr, PropSignature};

/// Thread `input` through `expr` top-down and return the resulting target word.
///
/// This is the colored well-formedness pass: composition requires *word*
/// equality at the interface, not merely equal arities. The arity-equal /
/// color-unequal case is exactly what a `usize`-only check cannot see.
///
/// Recursion depth is the expression height, matching the existing
/// [`PropExpr::source`] / [`PropExpr::target`] idiom.
///
/// # Errors
///
/// - [`CatgraphError::CompositionSizeMismatch`] when a subterm is handed a word
///   of the wrong *length* (including the top-level call).
/// - [`CatgraphError::Composition`] when the lengths agree but the *colors* do
///   not — the message names the generator and both words.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
/// use catgraph_applied::prop::colored::check;
/// use catgraph_applied::prop::{Free, PropSignature};
///
/// #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// struct Swap; // `Swap : A B → B A` over Λ = {A, B}
///
/// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// enum Wire { A, B }
///
/// impl PropSignature for Swap {
///     type Color = Wire;
///     fn source_word(&self) -> Cow<'_, [Wire]> { Cow::Owned(vec![Wire::A, Wire::B]) }
///     fn target_word(&self) -> Cow<'_, [Wire]> { Cow::Owned(vec![Wire::B, Wire::A]) }
/// }
///
/// let expr = Free::generator(Swap);
/// assert_eq!(check(&expr, &[Wire::A, Wire::B]).unwrap(), vec![Wire::B, Wire::A]);
/// assert!(check(&expr, &[Wire::B, Wire::A]).is_err()); // arities agree, colors do not
/// ```
pub fn check<G: PropSignature>(
    expr: &PropExpr<G>,
    input: &[G::Color],
) -> Result<Vec<G::Color>, CatgraphError> {
    match expr {
        PropExpr::Identity(n) => {
            expect_len(*n, input)?;
            Ok(input.to_vec())
        }
        PropExpr::Braid(m, n) => {
            expect_len(m + n, input)?;
            // σ_{m,n} : u ⊗ v → v ⊗ u — a block swap of the two halves.
            let mut out = Vec::with_capacity(input.len());
            out.extend_from_slice(&input[*m..]);
            out.extend_from_slice(&input[..*m]);
            Ok(out)
        }
        PropExpr::Generator(g) => {
            let want = g.source_word();
            expect_len(want.len(), input)?;
            if input != &*want {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "colored check: generator {g:?} declares source word {want:?} but received {input:?}"
                    ),
                });
            }
            Ok(g.target_word().into_owned())
        }
        PropExpr::Compose(f, h) => {
            let mid = check(f, input)?;
            check(h, &mid)
        }
        PropExpr::Tensor(f, h) => {
            // Split at `f`'s arity; each half then validates its own word.
            let split = f.source();
            expect_at_least(split, input)?;
            let (left, right) = input.split_at(split);
            let mut out = check(f, left)?;
            out.extend(check(h, right)?);
            Ok(out)
        }
    }
}

fn expect_len<C>(expected: usize, input: &[C]) -> Result<(), CatgraphError> {
    if input.len() == expected {
        Ok(())
    } else {
        Err(CatgraphError::CompositionSizeMismatch {
            expected,
            actual: input.len(),
        })
    }
}

fn expect_at_least<C>(expected: usize, input: &[C]) -> Result<(), CatgraphError> {
    if input.len() >= expected {
        Ok(())
    } else {
        Err(CatgraphError::CompositionSizeMismatch {
            expected,
            actual: input.len(),
        })
    }
}

/// A morphism of the free **Λ-colored** prop: the pair `(source word, expr)`,
/// together with the target word [`check`] derived from them.
///
/// Constructing one is the only way to pin colors onto a [`PropExpr`], whose
/// `Identity` / `Braid` nodes are color-polymorphic by design (see the module
/// docs). Every value of this type is word-well-formed by construction, with
/// the single documented exception of the serde path below.
///
/// # Equality
///
/// The derived `PartialEq` is **structural** (pre-quotient): equal source word,
/// equal target word, and syntactically identical trees.
/// [`Self::eq_colored`] is the SMC-quotient equality — normal forms plus
/// boundary words. Two expressions related by interchange are `eq_colored` but
/// not `==`.
///
/// # Serde (feature `serde`)
///
/// `Serialize` / `Deserialize` round-trip all three fields. **Deserialization
/// does not re-run [`check`]** — it reconstructs the fields directly, so a
/// hand-crafted document could carry a target word that the expression does not
/// actually produce, or an expression that is not word-well-formed at all.
/// Constructing through serde is a *trusted* path; round-tripping a value
/// produced by this crate is always safe. This mirrors the boundary already
/// documented on [`Presentation`] (#81); extending validation to the colored
/// surface belongs to #79 P2, which upgrades `add_equation`'s arity check to
/// boundary-word equality. When ingesting untrusted documents, re-validate by
/// rebuilding via [`Self::new`].
///
/// [`Presentation`]: super::presentation::Presentation
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "G: serde::Serialize, G::Color: serde::Serialize",
        deserialize = "G: serde::Deserialize<'de>, G::Color: serde::Deserialize<'de>"
    ))
)]
pub struct ColoredExpr<G: PropSignature> {
    source_word: Vec<G::Color>,
    target_word: Vec<G::Color>,
    expr: PropExpr<G>,
}

impl<G: PropSignature> ColoredExpr<G> {
    /// Check `expr` against `source_word` and pair them up.
    ///
    /// # Errors
    ///
    /// Propagates [`check`]'s errors verbatim.
    pub fn new(source_word: Vec<G::Color>, expr: PropExpr<G>) -> Result<Self, CatgraphError> {
        let target_word = check(&expr, &source_word)?;
        Ok(Self {
            source_word,
            target_word,
            expr,
        })
    }

    /// The declared source word `s ∈ Λ*`.
    #[must_use]
    pub fn source_word(&self) -> &[G::Color] {
        &self.source_word
    }

    /// The derived target word `t ∈ Λ*`.
    #[must_use]
    pub fn target_word(&self) -> &[G::Color] {
        &self.target_word
    }

    /// The underlying uncolored expression.
    #[must_use]
    pub fn expr(&self) -> &PropExpr<G> {
        &self.expr
    }

    /// Consume into `(source word, target word, expr)`.
    #[must_use]
    pub fn into_inner(self) -> (Vec<G::Color>, Vec<G::Color>, PropExpr<G>) {
        (self.source_word, self.target_word, self.expr)
    }

    /// SMC-quotient equality: equal boundary words **and** equal normal forms.
    ///
    /// Sound in the `true` direction unconditionally (§4.3 Lemma 4.2's
    /// readback). A `false` is not a disproof — see the module docs on §4.4.
    #[must_use]
    pub fn eq_colored(&self, other: &Self) -> bool {
        self.source_word == other.source_word
            && self.target_word == other.target_word
            && nf(&self.expr) == nf(&other.expr)
    }
}
