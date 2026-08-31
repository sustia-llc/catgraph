//! A **complete** decision functor for the pure-spider fragment, valued in
//! cospans (F&S 2019 Prop 3.8 / Thm 3.14).
//!
//! [`to_mat_kron`](crate::frobenius::to_mat_kron) is a *sound* semantic checker
//! for the SCFM laws, but not a syntactic decision: unequal matrix images
//! decide nothing. This module closes that gap for the **User-free** fragment
//! of `PropExpr<FrobeniusOr<G>>` by mapping into the free cospan category and
//! canonicalising up to apex isomorphism.
//!
//! # Why it is complete (and where it stops)
//!
//! F&S 2019 **Prop 3.8**: `(Cospan, ⊕)` is the theory of **special** commutative
//! Frobenius monoids — SCFMs correspond one-to-one with strict SM functors out
//! of `Cospan`. So the strict SM functor `Free(FrobeniusOr) → Cospan` sending
//! μ/η/δ/ε to their cospan generators (Ex 2.8) identifies exactly the terms the
//! nine SCFM equations identify. Composing it with the decidable apex-iso
//! invariant [`CospanCanon`] yields a
//! genuine [`CompleteFunctor`] for `E_frob` — the second entry in the
//! completeness registry after `Mat(R)`/Thm 5.60
//! ([`MatrixNFFunctor`](catgraph_applied::prop::presentation::functorial::MatrixNFFunctor)).
//!
//! **Colored generality.** The same argument runs at every colour: **Ex 3.12**
//! makes `Cospan_Λ` a hypergraph category by assigning each `l ∈ Λ` the Ex 2.8
//! structure (legitimate by **Lemma 3.10**), and **Thm 3.14** states that
//! `Cospan_Λ` is the *free* hypergraph category on `Λ`. The colored
//! [`ColoredCompleteFunctor`] impl below therefore decides `E_frob` over a
//! palette exactly as the monochromatic one does over `{•}` — the apex labels
//! carry the colours, so cospans over different colours cannot be confused.
//!
//! **Fragment boundary.** [`FrobeniusOr::User`] generators are opaque — they
//! have no cospan interpretation — so the functor is defined only on the
//! **User-free** (spider + scalar) fragment; a `User` node makes [`apply`] fail
//! with [`CatgraphError::Presentation`]. Mixed terms admit no completeness
//! statement.
//!
//! **Scalars are kept.** Cospan is *special*, not *extra-special*: the closed
//! bubble `η # ε` is a genuine non-identity scalar, distinguished from `id₀`.
//! Corelations ([`Corel`](catgraph::corel::Corel)) would collapse it and so are
//! the wrong target.
//!
//! [`apply`]: CospanFunctor::apply

use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;
use catgraph::cospan_canon::CospanCanon;
use catgraph::errors::CatgraphError;
use catgraph::monoidal::Monoidal;

use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::functorial::{ColoredCompleteFunctor, CompleteFunctor};
use catgraph_applied::prop::{PropExpr, PropSignature, mono_word};

use crate::frobenius::{FrobeniusOr, spider_target_word};

/// The complete Cospan-valued functor for the User-free spider fragment
/// (F&S 2019 Prop 3.8 / Thm 3.14). A zero-sized token, mirroring
/// [`MatrixNFFunctor`](catgraph_applied::prop::presentation::functorial::MatrixNFFunctor).
///
/// It implements both registry traits:
///
/// - [`CompleteFunctor<FrobeniusOr<G>>`] for a monochromatic `G`, with
///   [`Target`](CompleteFunctor::Target) `CospanCanon<()>` — the word-blind
///   entry point, which supplies the monochromatic source word itself;
/// - [`ColoredCompleteFunctor<FrobeniusOr<G>>`] for any `G`, with `Target`
///   `CospanCanon<G::Color>` — the worded entry point, taking the source word
///   from the [`ColoredExpr`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CospanFunctor;

impl CospanFunctor {
    /// Construct the functor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Map a User-free `PropExpr<FrobeniusOr<G>>` term, read at the interface word
/// `source_word`, to its image in the free cospan category `Cospan_Λ`.
///
/// This is the raw functor `Free(FrobeniusOr) → Cospan_Λ`; callers that want a
/// decision should canonicalise the result via
/// [`Cospan::canonical_form`](catgraph::cospan_canon) (or go through
/// [`CospanFunctor::apply`] / [`CospanFunctor::apply_colored`]).
///
/// Colours flow **top-down**, matching
/// [`check`](catgraph_applied::prop::colored::check): `Identity(k)` gives one
/// apex vertex per incoming wire, labelled by that wire's colour; `Braid(m, n)`
/// keeps the incoming labels and permutes the codomain leg; each spider demands
/// its declared colour and contributes a single apex vertex carrying it (Ex 2.8's
/// generators, transported to colour `c` by Ex 3.12).
///
/// # Errors
///
/// - [`CatgraphError::RecursionLimit`] if the term's structural depth exceeds
///   [`MAX_TERM_DEPTH`](crate::depth::MAX_TERM_DEPTH) (the recursion guard —
///   the same variant `SyntaxError::RecursionLimit` wraps, so every interpreter
///   reports the guard with one shape even though [`CompleteFunctor`] fixes this
///   one's error type to `CatgraphError`).
/// - [`CatgraphError::Presentation`] if the term contains a
///   [`FrobeniusOr::User`] generator (outside the fragment).
/// - [`CatgraphError::CompositionSizeMismatch`] if a subterm — including the
///   whole term — is handed a word of the wrong length. This is the top-down
///   word pass, so an arity-mismatched [`PropExpr::Compose`] is caught here
///   rather than by the cospan pushout.
/// - [`CatgraphError::Composition`] if the lengths agree but a spider's declared
///   colour does not match the incoming one.
pub fn to_cospan<G>(
    expr: &PropExpr<FrobeniusOr<G>>,
    source_word: &[G::Color],
) -> Result<Cospan<G::Color>, CatgraphError>
where
    G: PropSignature,
    G::Color: Copy + Ord,
{
    // Pre-flight the recursion depth so `to_cospan_inner` cannot overflow the
    // stack on an unbounded programmatically-built term. `CompleteFunctor`
    // fixes the error type to `CatgraphError`, so this reports the shared
    // `CatgraphError::RecursionLimit` — the same shape `SyntaxError::RecursionLimit`
    // carries for the other interpreters.
    let depth = crate::depth::term_depth(expr);
    if depth > crate::depth::MAX_TERM_DEPTH {
        return Err(CatgraphError::RecursionLimit {
            depth,
            limit: crate::depth::MAX_TERM_DEPTH,
        });
    }
    to_cospan_inner::<G>(expr, source_word).map(|(cospan, _target)| cospan)
}

/// One [`to_cospan_inner`] step over a palette `C`: the subterm's cospan paired
/// with the target word it derived.
type CospanStep<C> = (Cospan<C>, Vec<C>);

/// Recursion behind [`to_cospan`], returning `(cospan, target_word)`; the depth
/// guard runs once in the public entry so this can recurse freely.
fn to_cospan_inner<G>(
    expr: &PropExpr<FrobeniusOr<G>>,
    input: &[G::Color],
) -> Result<CospanStep<G::Color>, CatgraphError>
where
    G: PropSignature,
    G::Color: Copy + Ord,
{
    match expr {
        // id(k): k → k ← k, one apex vertex per wire, labelled by that wire.
        PropExpr::Identity(k) => {
            expect_len(*k, input)?;
            let word = input.to_vec();
            Ok((Cospan::identity(&word), word))
        }
        // braid(m,n): the block-swap permutation cospan on m+n wires. Domain is
        // the identity injection, so apex vertex i carries input wire i's colour;
        // the codomain outputs block B (the n wires) then block A (the m wires),
        // so output position j<n carries input m+j and output n+i carries input i.
        PropExpr::Braid(m, n) => {
            // Checked so a directly-constructed `Braid(usize::MAX, 1)` reports a
            // mismatch instead of wrapping; `usize::MAX` matches no slice length.
            let total = m.checked_add(*n).unwrap_or(usize::MAX);
            expect_len(total, input)?;
            let left: Vec<usize> = (0..total).collect();
            let right: Vec<usize> = (*m..total).chain(0..*m).collect();
            let mut target = Vec::with_capacity(total);
            target.extend_from_slice(&input[*m..]);
            target.extend_from_slice(&input[..*m]);
            // Correct by construction: both legs range over `0..total` and
            // `expect_len` has already pinned `input.len() == total`.
            Ok((Cospan::new_unchecked(left, right, input.to_vec()), target))
        }
        PropExpr::Generator(g) => {
            let color = match g {
                FrobeniusOr::Mu(c)
                | FrobeniusOr::Eta(c)
                | FrobeniusOr::Delta(c)
                | FrobeniusOr::Epsilon(c) => *c,
                // User generators are opaque — outside the pure-spider fragment.
                FrobeniusOr::User(u) => {
                    return Err(CatgraphError::Presentation {
                        message: format!(
                            "generator `{u:?}` is outside the pure-spider fragment; the Cospan \
                             functor is complete only for User-free terms (issue #80)"
                        ),
                    });
                }
            };
            let target = spider_target_word(g, input)?;
            // Ex 2.8's four generators, at the spider's own colour: a single apex
            // vertex labelled `c`, hit by every leg the arity declares.
            //   μ_c: 2 → 1 ← 1   η_c: 0 → 1 ← 1   δ_c: 1 → 1 ← 2   ε_c: 1 → 1 ← 0
            let left = vec![0; g.source()];
            let right = vec![0; g.target()];
            // Correct by construction: every leg entry is the literal `0` and
            // the apex is the single vertex `color`.
            Ok((Cospan::new_unchecked(left, right, vec![color]), target))
        }
        // f ; g — pushout composition, over words that meet by construction.
        PropExpr::Compose(f, g) => {
            let (fc, mid) = to_cospan_inner::<G>(f, input)?;
            let (gc, target) = to_cospan_inner::<G>(g, &mid)?;
            Ok((fc.compose(&gc)?, target))
        }
        // f ⊗ g — monoidal (disjoint-union) product.
        PropExpr::Tensor(f, g) => {
            let split = f.source();
            expect_at_least(split, input)?;
            let (left, right) = input.split_at(split);
            let (mut fc, mut target) = to_cospan_inner::<G>(f, left)?;
            let (gc, right_target) = to_cospan_inner::<G>(g, right)?;
            fc.monoidal(gc);
            target.extend(right_target);
            Ok((fc, target))
        }
    }
}

/// A subterm was handed a word of the wrong length.
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

/// A tensor's left factor needs at least its own arity from the incoming word.
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

impl<G> CompleteFunctor<FrobeniusOr<G>> for CospanFunctor
where
    G: PropSignature<Color = ()>,
{
    type Target = CospanCanon<()>;

    fn apply(&self, expr: &PropExpr<FrobeniusOr<G>>) -> Result<Self::Target, CatgraphError> {
        // The word-blind trait supplies no source word; monochromatically there
        // is exactly one of the right length.
        Ok(to_cospan::<G>(expr, &mono_word(expr.source()))?.canonical_form())
    }
}

impl<G> ColoredCompleteFunctor<FrobeniusOr<G>> for CospanFunctor
where
    G: PropSignature,
    G::Color: Copy + Ord,
{
    type Target = CospanCanon<G::Color>;

    fn apply_colored(
        &self,
        expr: &ColoredExpr<FrobeniusOr<G>>,
    ) -> Result<Self::Target, CatgraphError> {
        Ok(to_cospan::<G>(expr.expr(), expr.source_word())?.canonical_form())
    }
}
