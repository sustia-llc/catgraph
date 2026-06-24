//! Functorial decision procedure for prop-equality (v0.5.2).
//!
//! When a prop presentation `(G, E)` admits a known-complete functor
//! `F : Free(G) → T` into a decidable target, equality in the quotient
//! `Free(G)/⟨E⟩` reduces to equality in `T`:
//!
//! ```text
//!   [a] = [b] in Free(G)/⟨E⟩     iff     F(a) = F(b) in T
//! ```
//!
//! This module exposes two pieces:
//!
//! - [`CompleteFunctor<G>`] — a generic trait for any such decision functor.
//! - [`MatrixNFFunctor<R>`] — concrete instance wrapping [`crate::sfg_to_mat`]
//!   for the canonical `S: SFG_R → Mat(R)` functor. Complete on the
//!   presentation `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` by F&S Thm 5.60 /
//!   Baez-Erbele 2015. For signal-flow graphs under the 17 Thm 5.60
//!   equations, a matrix-equality check decides equivalence in the prop
//!   quotient — no congruence-closure engine needed.
//!
//! Use via [`super::Presentation::eq_mod_functorial`]. Unlike the default
//! [`super::NormalizeEngine::CongruenceClosure`] engine (sound but
//! syntactically incomplete on overlapping equation sets), a complete
//! functor is both sound and complete on its target presentation.
//!
//! # Why not a `NormalizeEngine` enum variant?
//!
//! `CompleteFunctor` has an associated `Target` type that varies per
//! functor instance, which precludes a uniform enum-payload representation
//! without type erasure. For v0.5.2 we keep the functor as a call-site
//! parameter on [`super::Presentation::eq_mod_functorial`]; the two
//! existing `NormalizeEngine` variants (`Structural`, `CongruenceClosure`)
//! continue to cover the default syntactic path.

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
// `Hash` is imported for the `MatrixNFFunctor<R>` bound only; removing it
// would break the functor's `R: Rig + Eq + Hash + 'static` constraint
// (inherited from `sfg_to_mat`'s signature). `CompleteFunctor` itself has
// no `Hash` requirement — see the module-level note.

use catgraph::errors::CatgraphError;

use crate::{
    mat::MatR,
    prop::{PropExpr, PropSignature},
    rig::Rig,
    sfg::{SfgGenerator, SignalFlowGraph},
    sfg_to_mat::sfg_to_mat,
};

// Note on trait bounds: `Target: Clone + Debug + PartialEq` is the actual
// v0.5.2 requirement (`eq_mod_functorial` only compares two values with
// `==`). The revised-scope plan §4.2 sketched `Eq + Hash`, but those
// tighter bounds would require `Eq` on `MatR<R>` (and by transitivity
// `Eq` on every rig `R`) without any call site actually needing them.
// If a future functor consumer needs to hash target values, the bound can
// be tightened with a minor version bump.

/// A functor `F : Free(G) → T` that is *complete* for a particular prop
/// presentation — `F(a) = F(b)` iff `[a] = [b]` in `Free(G)/⟨E⟩`.
///
/// Completeness is a claim about the specific presentation; it is not a
/// property of the functor alone. For example, the matrix functor
/// `S : SFG_R → Mat(R)` is complete on the `E_{17}` presentation of Thm 5.60
/// (Baez-Erbele 2015) but would NOT decide equality under a smaller
/// equation set `E' ⊂ E_{17}` — there the functor would over-identify.
///
/// Implementors are responsible for citing the source of their
/// completeness claim in their rustdoc.
pub trait CompleteFunctor<G: PropSignature> {
    /// The codomain of the functor. Equality in `Target` is the decision
    /// procedure's discriminator — hence only `PartialEq` is required.
    type Target: Clone + Debug + PartialEq;

    /// Apply the functor to a `PropExpr<G>`.
    ///
    /// # Errors
    ///
    /// Implementations may return a [`CatgraphError`] if the input
    /// expression is ill-formed (e.g., arity mismatch at a `Compose`
    /// node). For expressions built through the safe [`super::Presentation`]
    /// API this cannot occur; the error path exists for expressions
    /// constructed directly via `PropExpr`.
    fn apply(&self, expr: &PropExpr<G>) -> Result<Self::Target, CatgraphError>;
}

/// The matrix functor `S : SFG_R → Mat(R)` (F&S 2018 Thm 5.53), complete
/// on the `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` presentation by F&S Thm 5.60 /
/// Baez-Erbele 2015. Wraps the existing [`crate::sfg_to_mat`] function.
///
/// Equality of `MatR<R>` values decides equivalence of signal-flow graphs
/// under the 17 Thm 5.60 equations — no Knuth-Bendix completion or
/// congruence closure required.
pub struct MatrixNFFunctor<R: Rig + Debug + Eq + Hash + 'static> {
    _phantom: PhantomData<R>,
}

impl<R: Rig + Debug + Eq + Hash + 'static> MatrixNFFunctor<R> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<R: Rig + Debug + Eq + Hash + 'static> Default for MatrixNFFunctor<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R> CompleteFunctor<SfgGenerator<R>> for MatrixNFFunctor<R>
where
    R: Rig + Debug + Eq + Hash + 'static,
{
    type Target = MatR<R>;

    fn apply(&self, expr: &PropExpr<SfgGenerator<R>>) -> Result<MatR<R>, CatgraphError> {
        // `sfg_to_mat` takes a `&SignalFlowGraph<R>`, which is a newtype
        // over `PropExpr<SfgGenerator<R>>`. Wrap the expression and
        // delegate — this is the existing, well-tested path through the
        // Thm 5.53 functor.
        let sfg = SignalFlowGraph::from_prop_expr(expr.clone());
        sfg_to_mat(&sfg)
    }
}
