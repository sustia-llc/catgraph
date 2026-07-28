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
//! This module exposes three pieces:
//!
//! - [`CompleteFunctor<G>`] — a generic trait for any such decision functor.
//! - [`ColoredCompleteFunctor<G>`] — its Λ-colored sibling, consuming a
//!   [`ColoredExpr<G>`](crate::prop::colored::ColoredExpr) so the functor sees
//!   the interface word a bare [`PropExpr`] cannot carry.
//! - [`MatrixNFFunctor<R>`] — concrete instance wrapping [`crate::sfg_to_mat`]
//!   for the canonical `S: SFG_R → Mat(R)` functor. Complete on the
//!   presentation `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` by F&S Thm 5.60 (proof via
//!   Baez-Erbele 2015 for fields; the commutative-rig case — covering all rigs
//!   instantiated here — is Wadsley–Woods arXiv:1505.00048, cf. BE15 §6). For
//!   signal-flow graphs under the 18 Thm 5.60 equations, a matrix-equality
//!   check decides equivalence in the prop quotient — no congruence-closure
//!   engine needed.
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
//! without type erasure. We keep the functor as a call-site
//! parameter on [`super::Presentation::eq_mod_functorial`]; the two
//! existing `NormalizeEngine` variants (`Structural`, `CongruenceClosure`)
//! continue to cover the default syntactic path.

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
// `Hash` is imported for the `MatrixNFFunctor<R>` bound only; removing it
// would break the functor's `R: Rig + Eq + Hash + Ord + 'static` constraint
// (inherited from `sfg_to_mat`'s signature). `CompleteFunctor` itself has
// no `Hash` requirement — see the module-level note.

use catgraph::errors::CatgraphError;

use crate::{
    mat::MatR,
    prop::{PropExpr, PropSignature, colored::ColoredExpr},
    rig::Rig,
    sfg::{SfgGenerator, SignalFlowGraph},
    sfg_to_mat::sfg_to_mat,
};

// Note on trait bounds: `Target: Clone + Debug + PartialEq` is the actual
// requirement (`eq_mod_functorial` only compares two values with `==`).
// Tighter `Eq + Hash` bounds would require `Eq` on `MatR<R>` (and by
// transitivity `Eq` on every rig `R`) without any call site actually
// needing them.
// If a future functor consumer needs to hash target values, the bound can
// be tightened with a minor version bump.

/// A functor `F : Free(G) → T` that is *complete* for a particular prop
/// presentation — `F(a) = F(b)` iff `[a] = [b]` in `Free(G)/⟨E⟩`.
///
/// Completeness is a claim about the specific presentation; it is not a
/// property of the functor alone. For example, the matrix functor
/// `S : SFG_R → Mat(R)` is complete on the `E_{18}` presentation of Thm 5.60
/// (proof via Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for
/// commutative rigs, cf. BE15 §6) but would NOT decide equality under a smaller
/// equation set `E' ⊂ E_{18}` — there the functor would over-identify.
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

/// The Λ-colored sibling of [`CompleteFunctor`]: a functor
/// `F : Free_Λ(G) → T` that is *complete* for a presentation of the **colored**
/// free prop ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3a).
///
/// # Why a second trait rather than a bound relaxation
///
/// [`CompleteFunctor::apply`] takes a bare [`PropExpr<G>`], which is word-blind
/// *by shape*: `Identity(n)` and `Braid(m, n)` carry only a width, so a colored
/// functor handed one has no way to know which colours flow through it (see
/// [`crate::prop::colored`]). The colored morphism is the pair
/// `(source word, expression)` — [`ColoredExpr`] — so the colored functor must
/// consume that pair. A monochromatic functor can implement both: the word-blind
/// entry point recovers the unique word of the right length via
/// [`mono_word`](crate::prop::mono_word).
///
/// Completeness is, as for [`CompleteFunctor`], a claim about a particular
/// presentation; implementors cite its source in their rustdoc.
pub trait ColoredCompleteFunctor<G: PropSignature> {
    /// The codomain of the functor. Equality in `Target` is the decision
    /// procedure's discriminator — hence only `PartialEq` is required, matching
    /// [`CompleteFunctor::Target`].
    type Target: Clone + Debug + PartialEq;

    /// Apply the functor to a checked colored morphism.
    ///
    /// # Errors
    ///
    /// Implementations may return a [`CatgraphError`] if the expression lies
    /// outside the functor's domain (e.g. a generator with no image). A
    /// [`ColoredExpr`] is word-well-formed by construction, so the arity/colour
    /// failures [`CompleteFunctor`] documents cannot arise here — except through
    /// the documented serde trust boundary on [`ColoredExpr`] itself.
    fn apply_colored(&self, expr: &ColoredExpr<G>) -> Result<Self::Target, CatgraphError>;
}

/// The matrix functor `S : SFG_R → Mat(R)` (F&S 2018 Thm 5.53), complete
/// on the `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` presentation by F&S Thm 5.60 (proof
/// via Baez-Erbele 2015 for fields, Wadsley–Woods arXiv:1505.00048 for
/// commutative rigs, cf. BE15 §6). Wraps the existing [`crate::sfg_to_mat`]
/// function.
///
/// Equality of `MatR<R>` values decides equivalence of signal-flow graphs
/// under the 18 Thm 5.60 equations — no Knuth-Bendix completion or
/// congruence closure required.
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
        // `sfg_to_mat` takes a `&SignalFlowGraph<R>`, which is a newtype
        // over `PropExpr<SfgGenerator<R>>`. Wrap the expression and
        // delegate — this is the existing, well-tested path through the
        // Thm 5.53 functor.
        let sfg = SignalFlowGraph::from_prop_expr(expr.clone());
        sfg_to_mat(&sfg)
    }
}
