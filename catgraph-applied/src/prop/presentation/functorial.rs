//! Functorial decision procedure for prop-equality.
//!
//! When a prop presentation `(G, E)` admits a known-complete functor
//! `F : Free(G) → T` into a decidable target, equality in the quotient
//! `Free(G)/⟨E⟩` reduces to equality in `T`:
//!
//! ```text
//!   [a] = [b] in Free(G)/⟨E⟩     iff     F(a) = F(b) in T
//! ```
//!
//! [`CompleteFunctor<G>`] is the generic trait; [`ColoredCompleteFunctor<G>`] is
//! its Λ-colored sibling, consuming a
//! [`ColoredExpr<G>`](crate::prop::colored::ColoredExpr) so the functor sees the
//! interface word a bare [`PropExpr`] cannot carry; [`MatrixNFFunctor<R>`] is the
//! instance for `S : SFG_R → Mat(R)`, complete on the presentation
//! `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` by F&S Thm 5.60 (Baez-Erbele 2015 for fields;
//! Wadsley–Woods arXiv:1505.00048 for commutative rigs, cf. BE15 §6). Applied via
//! [`super::Presentation::eq_mod_functorial`].

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use catgraph::errors::CatgraphError;

use crate::{
    mat::MatR,
    prop::{PropExpr, PropSignature, colored::ColoredExpr},
    rig::Rig,
    sfg::{SfgGenerator, SignalFlowGraph},
    sfg_to_mat::sfg_to_mat,
};

/// A functor `F : Free(G) → T` that is *complete* for a particular prop
/// presentation — `F(a) = F(b)` iff `[a] = [b]` in `Free(G)/⟨E⟩`. Completeness
/// is a claim about the specific presentation, not a property of the functor
/// alone.
pub trait CompleteFunctor<G: PropSignature> {
    /// The codomain of the functor; `==` on `Target` is the decision procedure.
    type Target: Clone + Debug + PartialEq;

    /// Apply the functor to a `PropExpr<G>`.
    ///
    /// # Errors
    ///
    /// Implementations may return a [`CatgraphError`] if `expr` is ill-formed
    /// (e.g. an arity mismatch at a `Compose` node).
    fn apply(&self, expr: &PropExpr<G>) -> Result<Self::Target, CatgraphError>;
}

/// The Λ-colored sibling of [`CompleteFunctor`]: a functor `F : Free_Λ(G) → T`
/// that is *complete* for a presentation of the **colored** free prop. Consumes
/// the pair `(source word, expression)` carried by [`ColoredExpr`], which a bare
/// [`PropExpr`] cannot express.
pub trait ColoredCompleteFunctor<G: PropSignature> {
    /// The codomain of the functor; `==` on `Target` is the decision procedure.
    type Target: Clone + Debug + PartialEq;

    /// Apply the functor to a checked colored morphism.
    ///
    /// # Errors
    ///
    /// Implementations may return a [`CatgraphError`] if `expr` lies outside the
    /// functor's domain (e.g. a generator with no image).
    fn apply_colored(&self, expr: &ColoredExpr<G>) -> Result<Self::Target, CatgraphError>;
}

/// The matrix functor `S : SFG_R → Mat(R)` (F&S 2018 Thm 5.53), complete on the
/// `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` presentation by F&S Thm 5.60 (Baez-Erbele 2015
/// for fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs, cf. BE15 §6).
/// Equality of `MatR<R>` values decides equivalence of signal-flow graphs under
/// the 18 Thm 5.60 equations.
pub struct MatrixNFFunctor<R: Rig + Debug + Eq + Hash + Ord + 'static> {
    _phantom: PhantomData<R>,
}

impl<R: Rig + Debug + Eq + Hash + Ord + 'static> MatrixNFFunctor<R> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<R: Rig + Debug + Eq + Hash + Ord + 'static> Default for MatrixNFFunctor<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> CompleteFunctor<SfgGenerator<R>> for MatrixNFFunctor<R>
where
    R: Rig + Debug + Eq + Hash + Ord + 'static,
{
    type Target = MatR<R>;

    fn apply(&self, expr: &PropExpr<SfgGenerator<R>>) -> Result<MatR<R>, CatgraphError> {
        // `SignalFlowGraph<R>` is a newtype over `PropExpr<SfgGenerator<R>>`.
        let sfg = SignalFlowGraph::from_prop_expr(expr.clone());
        sfg_to_mat(&sfg)
    }
}
