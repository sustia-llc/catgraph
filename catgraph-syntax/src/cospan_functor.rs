//! A **complete** decision functor for the pure-spider fragment, valued in
//! cospans (F&S 2019 Prop 3.8).
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
//! **Fragment boundary.** [`FrobeniusOr::User`] generators are opaque — they
//! have no cospan interpretation — so the functor is defined only on the
//! **User-free** (spider + scalar) fragment; a `User` node makes [`apply`] fail
//! with [`CatgraphError::Presentation`]. Mixed terms admit no completeness
//! statement (colored/multi-sorted generality is [#79]).
//!
//! **Scalars are kept.** Cospan is *special*, not *extra-special*: the closed
//! bubble `η # ε` is a genuine non-identity scalar, distinguished from `id₀`.
//! Corelations ([`Corel`](catgraph::corel::Corel)) would collapse it and so are
//! the wrong target — see the [#80] spike.
//!
//! [`apply`]: CospanFunctor::apply
//! [#79]: https://github.com/sustia-llc/catgraph/issues/79
//! [#80]: https://github.com/sustia-llc/catgraph/issues/80

use catgraph::category::{Composable, HasIdentity};
use catgraph::cospan::Cospan;
use catgraph::cospan_canon::CospanCanon;
use catgraph::errors::CatgraphError;
use catgraph::monoidal::Monoidal;

use catgraph_applied::prop::presentation::functorial::CompleteFunctor;
use catgraph_applied::prop::{PropExpr, PropSignature};

use crate::frobenius::FrobeniusOr;

/// The complete Cospan-valued functor for the User-free spider fragment
/// (F&S 2019 Prop 3.8). A zero-sized token, mirroring
/// [`MatrixNFFunctor`](catgraph_applied::prop::presentation::functorial::MatrixNFFunctor).
///
/// Its [`Target`](CompleteFunctor::Target) is
/// [`CospanCanon<()>`](catgraph::cospan_canon::CospanCanon) (one wire colour;
/// see [module docs](self) for the completeness argument and the User-free
/// fragment boundary).
#[derive(Clone, Copy, Debug, Default)]
pub struct CospanFunctor;

impl CospanFunctor {
    /// Construct the functor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Map a User-free `PropExpr<FrobeniusOr<G>>` term to its image in the free
/// monochromatic cospan category.
///
/// This is the raw functor `Free(FrobeniusOr) → Cospan`; callers that want a
/// decision should canonicalise the result via
/// [`Cospan::canonical_form`](catgraph::cospan_canon) (or go through
/// [`CospanFunctor::apply`]).
///
/// # Errors
///
/// - [`CatgraphError::Presentation`] if the term contains a
///   [`FrobeniusOr::User`] generator (outside the fragment).
/// - [`CatgraphError::Composition`] if a [`PropExpr::Compose`] node is
///   arity-mismatched (surfaced transparently from the cospan pushout).
pub fn to_cospan<G>(expr: &PropExpr<FrobeniusOr<G>>) -> Result<Cospan<()>, CatgraphError>
where
    G: PropSignature,
{
    match expr {
        // id(k): k → k ← k, one apex vertex per wire.
        PropExpr::Identity(k) => Ok(Cospan::identity(&vec![(); *k])),
        // braid(m,n): the block-swap permutation cospan on m+n wires. Domain is
        // the identity injection; codomain outputs block B (the n wires) then
        // block A (the m wires), so output position j<n carries input m+j and
        // output n+i carries input i.
        PropExpr::Braid(m, n) => {
            let total = m + n;
            let left: Vec<usize> = (0..total).collect();
            let right: Vec<usize> = (*m..total).chain(0..*m).collect();
            Ok(Cospan::new(left, right, vec![(); total]))
        }
        PropExpr::Generator(g) => match g {
            // μ: 2 → 1 ← 1, both inputs and the output share one apex vertex.
            FrobeniusOr::Mu => Ok(Cospan::new(vec![0, 0], vec![0], vec![()])),
            // η: 0 → 1 ← 1, the unit — one apex vertex, hit only by the output.
            FrobeniusOr::Eta => Ok(Cospan::new(vec![], vec![0], vec![()])),
            // δ: 1 → 1 ← 2, one input fans out to two outputs through one apex.
            FrobeniusOr::Delta => Ok(Cospan::new(vec![0], vec![0, 0], vec![()])),
            // ε: 1 → 1 ← 0, the counit — one apex vertex, hit only by the input.
            FrobeniusOr::Epsilon => Ok(Cospan::new(vec![0], vec![], vec![()])),
            // User generators are opaque — outside the pure-spider fragment.
            FrobeniusOr::User(u) => Err(CatgraphError::Presentation {
                message: format!(
                    "generator `{u:?}` is outside the pure-spider fragment; the Cospan \
                     functor is complete only for User-free terms (issue #80)"
                ),
            }),
        },
        // f ; g — pushout composition. Arity mismatch surfaces as Composition.
        PropExpr::Compose(f, g) => {
            let fc = to_cospan::<G>(f)?;
            let gc = to_cospan::<G>(g)?;
            fc.compose(&gc)
        }
        // f ⊗ g — monoidal (disjoint-union) product.
        PropExpr::Tensor(f, g) => {
            let mut fc = to_cospan::<G>(f)?;
            let gc = to_cospan::<G>(g)?;
            fc.monoidal(gc);
            Ok(fc)
        }
    }
}

impl<G> CompleteFunctor<FrobeniusOr<G>> for CospanFunctor
where
    G: PropSignature,
{
    type Target = CospanCanon<()>;

    fn apply(&self, expr: &PropExpr<FrobeniusOr<G>>) -> Result<Self::Target, CatgraphError> {
        Ok(to_cospan::<G>(expr)?.canonical_form())
    }
}
