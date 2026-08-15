//! Prop presentations (F&S Def 5.33): equations quotienting `Free(G)`.
//!
//! A presentation `(G, s, t, E)` consists of a signature `G` with interface
//! maps `s, t` (provided via [`super::PropSignature`]) and a set `E` of
//! equations, each a pair `(lhs, rhs)` of [`super::PropExpr<G>`] that are
//! *parallel* — same source and target **words** over `Λ`, inferred by
//! [`Presentation::add_equation`]. The presented prop is `Free(G)` quotiented
//! by the smallest congruence containing `E` plus the SMC axioms.
//!
//! # Implementation
//!
//! Bounded-depth (default 32) term rewriting with:
//! 1. A fixed set of 9 **SMC-canonical-form rules** applied first (interchange,
//!    unitors, associator, compose-identity, compose-associator,
//!    braid-involution, identity-coherence of ⊗). This closes the F&S Def 5.30
//!    PARTIAL gap (the syntactic quotient by SMC axioms is now explicit).
//! 2. User equations `E` applied left-to-right thereafter.
//!
//! ## SMC rules
//!
//! 1. **Interchange**: `(f1 ⊗ g1) ; (f2 ⊗ g2) → (f1 ; f2) ⊗ (g1 ; g2)` when all composable.
//! 2. **Left unitor**: `Identity(0) ⊗ f → f`.
//! 3. **Right unitor**: `f ⊗ Identity(0) → f`.
//! 4. **Associator (right-bias)**: `(f ⊗ g) ⊗ h → f ⊗ (g ⊗ h)`.
//! 5. **Compose-identity (left)**: `Identity(n) ; f → f` when `n` matches `f`'s source.
//! 6. **Compose-identity (right)**: `f ; Identity(n) → f` when `n` matches `f`'s target.
//! 7. **Compose-associator (right-bias)**: `(f ; g) ; h → f ; (g ; h)`.
//! 8. **Braid-involution**: `Braid(m,n) ; Braid(n,m) → Identity(m+n)`.
//! 9. **Identity-coherence of ⊗**:
//!    `Identity(m) ⊗ Identity(n) → Identity(m+n)`.
//!
//! # Confluence
//!
//! The 9 fixed rules are confluent on non-overlapping user equations. For
//! overlapping user equations the rewriter may yield false `eq_mod` negatives
//! — a conservative answer. Knuth-Bendix completion is out of scope.

pub mod content;
pub mod display;
pub mod functorial;
pub mod kb;
pub mod rewrite;
pub mod smc_nf;

use super::{PropExpr, PropSignature};
use catgraph::errors::CatgraphError;
use functorial::{ColoredCompleteFunctor, CompleteFunctor};

use super::colored::ColoredExpr;

/// Engine selector for [`Presentation::eq_mod`].
///
/// **Scope:** this selector affects [`Presentation::eq_mod`] only.
/// [`Presentation::normalize`] is always bounded structural rewriting —
/// congruence closure partitions into equivalence classes without producing a
/// canonical representative, so it doesn't have a meaningful `normalize`
/// semantics.
///
/// - [`NormalizeEngine::Structural`]: the structural `eq_mod` behavior — normalize
///   both sides via bounded structural rewriting and compare. Cheap and
///   deterministic for non-overlapping presentations; may yield false negatives
///   (`None`) on overlapping equations (e.g., the 18 Thm 5.60 scalar D-group
///   equations).
/// - [`NormalizeEngine::CongruenceClosure`] (default): decide
///   equality via bounded congruence closure over [`kb::CongruenceClosure`].
///   Correct decision procedure for any equational theory without binders,
///   including overlapping equations. Always returns `Some(_)` — no false
///   negatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NormalizeEngine {
    /// Structural `eq_mod` behavior: normalize both sides via bounded structural
    /// rewriting and compare structurally.
    Structural,
    /// Default: decide equality via bounded congruence closure.
    #[default]
    CongruenceClosure,
}

/// Result of [`Presentation::normalize`]. Distinguishes "fully reduced"
/// from "hit depth bound" so callers can decide how to handle partial results.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use]
pub struct NormalizeResult<G: PropSignature> {
    /// The (possibly partial) normalized expression.
    pub expr: PropExpr<G>,
    /// `true` iff normalization reached a fixpoint before the depth bound.
    pub converged: bool,
    /// Number of rewrite iterations performed (≤ `rewrite_depth`).
    pub steps_taken: usize,
}

/// A presentation of a prop: generators `G` with arity maps plus equations `E`.
///
/// # Serde (feature `serde`)
///
/// `Serialize`/`Deserialize` round-trip the full state (equations, depth,
/// engine). **Deserialization does not re-run [`Self::add_equation`]'s check**
/// — it reconstructs the fields directly. Since #79 P2 that check is
/// boundary-*word* equality, so a hand-crafted document could carry an equation
/// that is word-ill-formed (an inner subterm handed the wrong number of wires)
/// or color-mismatched between its two sides, not merely arity-mismatched.
/// Round-tripping a value produced by this crate is always safe; when ingesting
/// untrusted documents, re-validate by rebuilding via [`Self::add_equation`].
/// The same boundary applies to
/// [`ColoredExpr`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Presentation<G: PropSignature> {
    equations: Vec<(PropExpr<G>, PropExpr<G>)>,
    rewrite_depth: usize,
    engine: NormalizeEngine,
}

impl<G: PropSignature> Default for Presentation<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: PropSignature> Presentation<G> {
    /// New empty presentation with default `rewrite_depth = 32` and default
    /// [`NormalizeEngine::CongruenceClosure`] engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            equations: Vec::new(),
            rewrite_depth: 32,
            engine: NormalizeEngine::default(),
        }
    }

    /// New empty presentation with a custom rewrite-depth bound. Engine
    /// defaults to [`NormalizeEngine::CongruenceClosure`].
    #[must_use]
    pub fn with_depth(rewrite_depth: usize) -> Self {
        Self {
            equations: Vec::new(),
            rewrite_depth,
            engine: NormalizeEngine::default(),
        }
    }

    /// New empty presentation with an explicit engine selector. Depth
    /// defaults to `32`.
    ///
    /// Use this to opt into the [`NormalizeEngine::Structural`]
    /// behavior on an overlapping presentation (for regression testing or
    /// performance comparison).
    #[must_use]
    pub fn with_engine(engine: NormalizeEngine) -> Self {
        Self {
            equations: Vec::new(),
            rewrite_depth: 32,
            engine,
        }
    }

    /// Set the engine after construction.
    pub fn set_engine(&mut self, engine: NormalizeEngine) {
        self.engine = engine;
    }

    /// Add an equation `lhs = rhs`. Both sides must be **parallel morphisms
    /// over a common source word** — boundary-*word* equality, not merely
    /// matching arities ([#79](https://github.com/sustia-llc/catgraph/issues/79)
    /// P2).
    ///
    /// The check is the word-inference pass sibling of
    /// [`crate::prop::colored::check`]: fresh variables stand for the unknown
    /// source colors, and *both* sides are threaded through the same variables,
    /// so a constraint discovered on either side propagates to the other. The
    /// two resulting target words are then unified pairwise. Acceptance means
    /// such a shared source word exists (the most general one, given the
    /// inferred constraints).
    ///
    /// # Monochromatic signatures
    ///
    /// With `Color = ()` every unification succeeds and the pass reduces to
    /// length checks — but it is still **stronger** than the pre-P2 check,
    /// which compared only the two sides' top-level [`PropExpr::source`] /
    /// [`PropExpr::target`]. [`PropExpr`]'s variants are public, so a
    /// hand-built ill-composed tree — `Identity(1) ; (Identity(2) ; Identity(1))`
    /// reads `1 → 1` at the top while its inner `Identity(2)` is handed one
    /// wire — used to be accepted and is now rejected with
    /// [`CatgraphError::CompositionSizeMismatch`]. Terms built through
    /// [`Free`](crate::prop::Free) are unaffected.
    ///
    /// # Polymorphic equations
    ///
    /// Sides that are individually color-polymorphic but *jointly* constrained
    /// are accepted: `Identity(2) = Braid(1,1)` forces the two source positions
    /// to share a color, and is well-formed at every word that does. The
    /// inferred constraint is **not stored**, and rewriting by user equations
    /// ([`Self::normalize`], [`Self::eq_mod`]) is word-blind — it operates on
    /// [`PropExpr`], and no in-tree API applies user equations to a
    /// [`ColoredExpr`] — so there is no
    /// rewrite site that could observe the omission.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::CompositionSizeMismatch`] on any length
    ///   disagreement: a subterm handed a word of the wrong length, `rhs`'s
    ///   source arity differing from `lhs`'s, or target words of different
    ///   lengths. Also when `lhs`'s source arity reads `usize::MAX` — the check
    ///   *sizes* a fresh source word from it, so that one is screened up front
    ///   rather than compared, since the sizing would abort before any
    ///   comparison could reject it (#196).
    /// - [`CatgraphError::Composition`] when the lengths agree but the colors
    ///   conflict — the message names the generator and position, or the target
    ///   position, and both colors.
    pub fn add_equation(
        &mut self,
        lhs: PropExpr<G>,
        rhs: PropExpr<G>,
    ) -> Result<(), CatgraphError> {
        super::colored::check_equation(&lhs, &rhs)?;
        self.equations.push((lhs, rhs));
        Ok(())
    }

    /// Borrow the equation list (LHS-RHS pairs) for external inspection.
    ///
    /// Primarily intended for soundness/faithfulness testing: callers can
    /// iterate every `(lhs, rhs)` pair and assert a chosen semantic
    /// interpretation (e.g. matrix equality under a functor) holds on every
    /// equation.
    #[must_use]
    pub fn equations(&self) -> &[(PropExpr<G>, PropExpr<G>)] {
        &self.equations
    }

    /// Normalize `expr` to canonical form under the SMC rules + user equations.
    ///
    /// Termination is always guaranteed by the depth bound; on a cyclic
    /// equation set the result is whichever representative was reached when
    /// the bound was hit.
    ///
    /// Returns a [`NormalizeResult`] exposing `.expr` (the possibly-partial
    /// normalized expression), `.converged` (`true` iff a fixpoint was reached
    /// before the depth bound), and `.steps_taken` (the number of rewrite
    /// iterations performed).
    ///
    /// Callers that only need the expression can write
    /// `p.normalize(&e)?.expr`.
    ///
    /// # Errors
    ///
    /// Currently infallible, but returns [`CatgraphError::Presentation`] for
    /// forward-compatibility (future well-formedness checks may fire during
    /// rewriting).
    pub fn normalize(&self, expr: &PropExpr<G>) -> Result<NormalizeResult<G>, CatgraphError> {
        let mut current = expr.clone();
        for step in 0..self.rewrite_depth {
            let after_smc = apply_smc_rules(&current);
            let after_user = self.apply_user_equations(&after_smc);
            if after_user == current {
                return Ok(NormalizeResult {
                    expr: current,
                    converged: true,
                    // `step` is 0-indexed but a complete iteration (one SMC
                    // pass + one user-equations pass) runs BEFORE the
                    // fixpoint check, so the number of iterations performed
                    // is `step + 1`. Matches the rustdoc contract and the
                    // depth-bound branch (which returns `self.rewrite_depth`,
                    // the count of full iterations run).
                    steps_taken: step + 1,
                });
            }
            current = after_user;
        }
        // Depth bound reached; return whatever we have.
        Ok(NormalizeResult {
            expr: current,
            converged: false,
            steps_taken: self.rewrite_depth,
        })
    }

    /// SMC-only bounded normalization: apply the 9 fixed SMC-canonical-form
    /// rules (including Rule 9 — identity-coherence of ⊗ `Identity(m) ⊗
    /// Identity(n) → Identity(m+n)`) to a fixpoint, **without**
    /// applying user equations. Used by the
    /// CC engine's pre-pass so the congruence-closure graph is fed
    /// SMC-canonicalized operands and seeded equations without pre-consuming
    /// the user equations themselves.
    ///
    /// Returns `Result` for forward-compatibility (matches `normalize`'s
    /// signature; future well-formedness checks may fire during rewriting).
    #[allow(clippy::unnecessary_wraps)]
    fn normalize_smc_only(&self, expr: &PropExpr<G>) -> Result<NormalizeResult<G>, CatgraphError> {
        let mut current = expr.clone();
        for step in 0..self.rewrite_depth {
            let after_smc = apply_smc_rules(&current);
            if after_smc == current {
                return Ok(NormalizeResult {
                    expr: current,
                    converged: true,
                    steps_taken: step + 1,
                });
            }
            current = after_smc;
        }
        Ok(NormalizeResult {
            expr: current,
            converged: false,
            steps_taken: self.rewrite_depth,
        })
    }

    /// Equality modulo this presentation.
    ///
    /// Dispatches on [`Presentation::engine`]:
    ///
    /// - [`NormalizeEngine::Structural`]: normalize both sides via bounded
    ///   structural rewriting and compare. Returns `Ok(Some(true))` /
    ///   `Ok(Some(false))` when both sides converge, or `Ok(None)` if at least
    ///   one side hit the depth bound.
    /// - [`NormalizeEngine::CongruenceClosure`] (default):
    ///   decide equality via bounded congruence closure. Always returns
    ///   `Ok(Some(_))` — no false negatives on overlapping equations.
    ///
    /// # The two layers, and which decides what
    ///
    /// Under the default engine this is a two-layer question, and the two layers
    /// have different decision procedures
    /// ([#57](https://github.com/sustia-llc/catgraph/issues/57) a1):
    ///
    /// - **SMC coherence** — associator, unitor, interchange, braid-naturality,
    ///   `σ² = id`. Decided by [`content`], exactly: Lemma 4.1 of
    ///   `docs/SMC-NF-RECONCILIATION.md` §4.2 says `C(a) = C(b)` **iff** `a` and
    ///   `b` are equal in the free symmetric monoidal category. Content equality
    ///   is therefore the equality of record at this layer. That matters because
    ///   [`smc_nf::nf`] is not complete here: it separates SMC-equal writings
    ///   (§4.4's `η` placement slack, §4.6's ledger), and every one of those
    ///   pairs is now decided `Ok(Some(true))`.
    /// - **User equations** — the `E` of this presentation, e.g. the 18 Thm 5.60
    ///   equations. Decided by [`kb::CongruenceClosure`] above the SMC layer,
    ///   with the same bounded-completeness caveats as before.
    ///
    /// **Where `nf` still decides.** It no longer decides the SMC layer on the
    /// well-formed path, but it has not been reduced to display: it is still the
    /// canonicalizer *inside* [`kb::CongruenceClosure`]'s `smc_refine` fixpoint,
    /// which rebuilds every term with atom-canonical substitutions, normalizes it
    /// with `nf`, and merges the term's class with the normal form's — so `nf`'s
    /// quality still affects which user-equation classes close — and it is still
    /// the fallback below, outside content's domain. Its other jobs, canonical
    /// display and readback, are unchanged.
    ///
    /// So a `true` is exact at the SMC layer and sound at the user layer; a
    /// `false` is still only as complete as congruence closure is. Cocommutativity
    /// is the standing illustration of the split: `Copy ; Add` and
    /// `Copy ; σ ; Add` are *not* SMC-equal, so the content layer declines them,
    /// and whether they come back equal is up to the presentation's equations.
    ///
    /// **Sound per query, but not an equivalence relation as a decision
    /// procedure** ([#189](https://github.com/sustia-llc/catgraph/issues/189)).
    /// Every verdict is sound and definite for the pair it was asked about, but
    /// verdicts do not compose: the relation is reflexive and symmetric and
    /// **not transitive**. Under `matr_presentation(&[false, true])`,
    /// `Scalar(false)` ~ `Discard ; Zero` (the D8 user equation) and
    /// `Discard ; Zero` ~ `Discard ⊗ Zero` (the SMC layer), yet
    /// `Scalar(false)` ≁ `Discard ⊗ Zero` — and that last one is `Some(false)`,
    /// not `None`. Measured in #189 on a 120-expression pool of parallel
    /// `1 → 1` arrows (the pool is recorded there):
    /// 10 490 ordered violating triples with zero `None` verdicts, so this is
    /// congruence closure's incompleteness showing through rather than a
    /// depth-bound artifact. A caller that wants a *partition* out of this must
    /// therefore take the connected components of the `Some(true)` graph — a
    /// scan against class representatives computes a statistic of the
    /// enumeration order, not a function of the relation. That is what
    /// [`crate::graphical_linalg::verify_sfg_to_mat_is_full_and_faithful`] does.
    ///
    /// **Arity-ill-formed input.** `PropExpr`'s variants are public, so a
    /// hand-built tree can compose mismatched arities. This method stays total on
    /// those, as it always has: the content layer is gated on
    /// [`content::is_arity_well_formed`] and such a tree falls through to the
    /// pre-#57 `nf` short-circuit and then to congruence closure, reaching
    /// exactly the verdict it reached before. No new error variant, and no panic
    /// leaking out of [`content::content_of`].
    ///
    /// **Overflowing arities (#196).** A hand-built tree can also carry a `Braid`
    /// or `Tensor` width that sums past `usize::MAX`. That is not the same class:
    /// a mismatched `Compose` still has real arities on both sides, whereas an
    /// overflowed width is a magnitude *neither* layer below can size from —
    /// [`content::content_of`] would allocate from it and [`smc_nf::nf`] would
    /// decompose a `σ` of `usize::MAX + 1` wires — so both reject it outright.
    /// This method screens it first, before either engine runs, and answers
    /// `Ok(Some(true))` when the two trees are structurally identical (the one
    /// verdict still soundly available) and `Ok(None)` — undecided — otherwise.
    /// `Ok(Some(false))` is never returned there: it would claim a disproof no
    /// layer established.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Presentation`] if normalization fails for either
    /// side (currently unreachable; future-proofing).
    pub fn eq_mod(&self, a: &PropExpr<G>, b: &PropExpr<G>) -> Result<Option<bool>, CatgraphError> {
        // #196: neither engine is defined on an overflowing width — `nf` rejects,
        // `content_of` rejects, and `apply_smc_rules` declines to fuse. Screen
        // once here rather than leaving each of them to fail differently.
        if !a.arities_fit() || !b.arities_fit() {
            return Ok((a == b).then_some(true));
        }
        match self.engine {
            NormalizeEngine::Structural => {
                // Structural behavior: normalize both sides + compare; None if
                // either side hit the depth bound.
                let na = self.normalize(a)?;
                let nb = self.normalize(b)?;
                if !na.converged || !nb.converged {
                    return Ok(None);
                }
                Ok(Some(na.expr == nb.expr))
            }
            NormalizeEngine::CongruenceClosure => {
                // Hybrid: settle the SMC layer FIRST, then fall back to the CC
                // engine with the `normalize_smc_only` pre-pass.
                //
                // Why the hybrid:
                // - Content equality is *exact* for SMC coherence (associator,
                //   unitor, interchange, braid-naturality, σ²=id): by Lemma 4.1
                //   `C(a) = C(b)` iff `a` and `b` are SMC-equal, so this arm
                //   catches every such equality without consulting user
                //   equations. It replaced an `nf(a) == nf(b)` check in #57 a1,
                //   which was sound but incomplete — the NF still separates
                //   SMC-equal pairs (all 183 published divergences, and 1153 in
                //   braid mode; 253 / 1162 before #185), and content closes
                //   every one of them.
                // - The CC engine handles user-equation congruence (e.g., the
                //   18 Thm 5.60 equations) but doesn't know SMC axioms.
                // - Replacing CC's pre-pass entirely with NF was tried and
                //   regressed the faithfulness-test collision counts at
                //   BoolRig d2 (2574 → 3763, both measured under the pre-#189
                //   greedy partition) because NF reshapes seeded-
                //   equation LHS/RHS into forms CC's structural hash no
                //   longer matches the query against. That is a fact about the
                //   *pre-pass*, which is unchanged here; this arm only adds
                //   `Some(true)` verdicts ahead of CC.
                // - The union (SMC OR CC) captures both capabilities without
                //   the reshaping problem.
                //
                // Perf note: the content check is cheap (no equation
                // enumeration, same cost order as the `nf` call it replaced)
                // and short-circuits a large fraction of queries.
                //
                // `content_of` panics outside its arity-well-formed domain,
                // where `eq_mod` is total today, so the content arm is gated and
                // an ill-formed tree keeps the pre-#57 NF short-circuit
                // verbatim. On the gated-in path the NF check would be dead
                // weight: `nf` preserves content (Lemma 4.2), so
                // `nf(a) == nf(b)` implies `C(a) = C(b)` and the content arm has
                // already returned.
                if content::is_arity_well_formed(a) && content::is_arity_well_formed(b) {
                    if content::content_eq(&content::content_of(a), &content::content_of(b)) {
                        return Ok(Some(true));
                    }
                } else if smc_nf::nf(a) == smc_nf::nf(b) {
                    return Ok(Some(true));
                }
                // Fall back to the CC engine with SMC pre-pass.
                let na = self.normalize_smc_only(a)?;
                let nb = self.normalize_smc_only(b)?;
                if !na.converged || !nb.converged {
                    return Ok(None);
                }
                let normalized_equations: Vec<(PropExpr<G>, PropExpr<G>)> = {
                    let mut out = Vec::with_capacity(self.equations.len());
                    for (lhs, rhs) in &self.equations {
                        let nl = self.normalize_smc_only(lhs)?;
                        let nr = self.normalize_smc_only(rhs)?;
                        if !nl.converged || !nr.converged {
                            return Ok(None);
                        }
                        out.push((nl.expr, nr.expr));
                    }
                    out
                };
                let mut engine = kb::CongruenceClosure::new(&normalized_equations);
                Ok(Some(engine.are_equal(&na.expr, &nb.expr)))
            }
        }
    }

    /// Borrow the engine selector.
    #[must_use]
    pub fn engine(&self) -> NormalizeEngine {
        self.engine
    }

    /// Borrow the depth bound [`Self::normalize`] and [`Self::eq_mod`] run to.
    ///
    /// The sibling of [`Self::engine`], and needed for the same reason: a
    /// consumer that stores a presentation as its `(equations, depth, engine)`
    /// parts and rebuilds it later can read the engine back but, until this
    /// accessor, could not read the depth. A rebuild that goes through
    /// [`Self::new`] plus [`Self::add_equation`] therefore restored the **default
    /// 32** silently — no error, no warning, just a different bound than the one
    /// configured, and with it a different `converged` verdict on any
    /// presentation whose normalization needed the longer budget.
    ///
    /// # Rebuilding: both parts, or the same bug in the other slot
    ///
    /// No constructor takes a depth *and* an engine, so restoring both is two
    /// calls: [`Self::with_depth`] for the depth, then [`Self::set_engine`] for
    /// the engine, then the equations.
    ///
    /// ```ignore
    /// let mut rebuilt = Presentation::with_depth(depth);
    /// rebuilt.set_engine(engine);
    /// for (lhs, rhs) in equations { rebuilt.add_equation(lhs, rhs)?; }
    /// ```
    ///
    /// The `set_engine` line is not optional. [`Self::with_depth`] carries the
    /// depth and *only* the depth — its engine is `NormalizeEngine::default()`,
    /// i.e. [`NormalizeEngine::CongruenceClosure`]
    /// — so a rebuild that stops after `with_depth` silently restores
    /// `CongruenceClosure` over a stored [`NormalizeEngine::Structural`],
    /// reintroducing in the engine slot exactly the silent-default bug this
    /// accessor was added to fix in the depth slot. Symmetrically,
    /// [`Self::with_engine`] carries the engine and defaults the depth to 32.
    #[must_use]
    pub fn rewrite_depth(&self) -> usize {
        self.rewrite_depth
    }

    /// Decide equality using a semantic functor `f : Free(G) → T` that is
    /// *complete* on this presentation.
    ///
    /// For any [`CompleteFunctor`] `f`, the test is `f(a) == f(b)` in the
    /// functor's target type. The decision is always definite
    /// (`Ok(Some(_))`) — there is no depth bound, no syntactic rewriting,
    /// no false negatives. Completeness is an external claim carried by
    /// the functor implementation (see [`CompleteFunctor`] rustdoc).
    ///
    /// # Example — Thm 5.60 Mat(R) via [`functorial::MatrixNFFunctor`]
    ///
    /// `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` (F&S Thm 5.60; proof via Baez-Erbele
    /// 2015 for fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs,
    /// cf. BE15 §6). Two signal-flow graphs are equivalent under the 18 Thm
    /// 5.60 equations iff their matrix images are equal:
    ///
    /// ```ignore
    /// let f = MatrixNFFunctor::<BoolRig>::new();
    /// assert_eq!(pres.eq_mod_functorial(&a, &b, &f)?, Some(true));
    /// ```
    ///
    /// Complements [`Self::eq_mod`], which uses the syntactic
    /// [`NormalizeEngine::CongruenceClosure`] default and may return
    /// `None` or `Some(false)` where the functorial engine would return
    /// `Some(true)` (CC is sound but syntactically incomplete on
    /// overlapping equation sets).
    ///
    /// # Errors
    ///
    /// Propagates [`CatgraphError`] from `f.apply` if either input
    /// expression is ill-formed.
    pub fn eq_mod_functorial<F>(
        &self,
        a: &PropExpr<G>,
        b: &PropExpr<G>,
        f: &F,
    ) -> Result<Option<bool>, CatgraphError>
    where
        F: CompleteFunctor<G>,
    {
        let fa = f.apply(a)?;
        let fb = f.apply(b)?;
        Ok(Some(fa == fb))
    }

    /// Decide equality of two **colored** morphisms using a
    /// [`ColoredCompleteFunctor`] that is complete on this presentation
    /// ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3a) — the
    /// worded sibling of [`Self::eq_mod_functorial`].
    ///
    /// # Parallel first, images second
    ///
    /// Two morphisms are equal only if they are **parallel**: same source word
    /// and same target word. `a` and `b` therefore compare unequal —
    /// `Ok(Some(false))`, definitely, without consulting the functor — whenever
    /// their boundary words differ, even if their images coincide. That is not a
    /// redundancy check: [`ColoredCompleteFunctor`] asks only that `Target`
    /// equality decide equality *within* a hom-set, and nothing in the trait
    /// makes a target separate the hom-sets from each other, so image equality
    /// alone could identify morphisms with different interfaces. (A given
    /// functor may of course be finer — the cospan canonical form, for one,
    /// records each boundary index against its apex label, so it happens to
    /// recover the words.) Only when the words agree is `F(a) == F(b)`
    /// consulted, and then the decision is definite.
    ///
    /// # Errors
    ///
    /// Propagates [`CatgraphError`] from `f.apply_colored` if either expression
    /// lies outside the functor's domain.
    pub fn eq_mod_functorial_colored<F>(
        &self,
        a: &ColoredExpr<G>,
        b: &ColoredExpr<G>,
        f: &F,
    ) -> Result<Option<bool>, CatgraphError>
    where
        F: ColoredCompleteFunctor<G>,
    {
        if a.source_word() != b.source_word() || a.target_word() != b.target_word() {
            return Ok(Some(false));
        }
        let fa = f.apply_colored(a)?;
        let fb = f.apply_colored(b)?;
        Ok(Some(fa == fb))
    }

    fn apply_user_equations(&self, expr: &PropExpr<G>) -> PropExpr<G> {
        let mut current = expr.clone();
        for (lhs, rhs) in &self.equations {
            current = rewrite_once_top(&current, lhs, rhs);
        }
        current
    }
}

/// Apply the 9 fixed SMC-axiom rules once bottom-up, recursing into Compose/Tensor.
fn apply_smc_rules<G: PropSignature>(expr: &PropExpr<G>) -> PropExpr<G> {
    // First, recurse into children (bottom-up).
    let expr = match expr {
        PropExpr::Compose(f, g) => {
            let f_norm = apply_smc_rules(f);
            let g_norm = apply_smc_rules(g);
            PropExpr::Compose(Box::new(f_norm), Box::new(g_norm))
        }
        PropExpr::Tensor(f, g) => {
            let f_norm = apply_smc_rules(f);
            let g_norm = apply_smc_rules(g);
            PropExpr::Tensor(Box::new(f_norm), Box::new(g_norm))
        }
        other => other.clone(),
    };

    // Now apply top-level rules. Order matters — more-specific rules first
    // (identity reductions and braid-involution) before associators, which
    // only rebalance structure.
    match expr {
        // Rule 5: Identity(n) ; f → f
        PropExpr::Compose(ref f, ref g) if matches!(f.as_ref(), PropExpr::Identity(_)) => {
            if let PropExpr::Identity(n) = f.as_ref()
                && *n == g.source()
            {
                return apply_smc_rules(g);
            }
            PropExpr::Compose(f.clone(), g.clone())
        }
        // Rule 6: f ; Identity(n) → f
        PropExpr::Compose(ref f, ref g) if matches!(g.as_ref(), PropExpr::Identity(_)) => {
            if let PropExpr::Identity(n) = g.as_ref()
                && *n == f.target()
            {
                return apply_smc_rules(f);
            }
            PropExpr::Compose(f.clone(), g.clone())
        }
        // Rule 8: Braid(m,n) ; Braid(n,m) → Identity(m+n)
        PropExpr::Compose(ref f, ref g)
            if matches!(f.as_ref(), PropExpr::Braid(_, _))
                && matches!(g.as_ref(), PropExpr::Braid(_, _)) =>
        {
            if let (PropExpr::Braid(m1, n1), PropExpr::Braid(m2, n2)) = (f.as_ref(), g.as_ref())
                && *m1 == *n2
                && *n1 == *m2
            {
                return PropExpr::Identity(m1 + n1);
            }
            PropExpr::Compose(f.clone(), g.clone())
        }
        // Rule 1: Interchange (f1 ⊗ g1) ; (f2 ⊗ g2) → (f1 ; f2) ⊗ (g1 ; g2)
        PropExpr::Compose(ref left, ref right)
            if matches!(left.as_ref(), PropExpr::Tensor(_, _))
                && matches!(right.as_ref(), PropExpr::Tensor(_, _)) =>
        {
            if let (PropExpr::Tensor(f1, g1), PropExpr::Tensor(f2, g2)) =
                (left.as_ref(), right.as_ref())
            {
                // Composability check: f1.target == f2.source and g1.target == g2.source.
                if f1.target() == f2.source() && g1.target() == g2.source() {
                    let f12 = PropExpr::Compose(f1.clone(), f2.clone());
                    let g12 = PropExpr::Compose(g1.clone(), g2.clone());
                    return apply_smc_rules(&PropExpr::Tensor(Box::new(f12), Box::new(g12)));
                }
            }
            PropExpr::Compose(left.clone(), right.clone())
        }
        // Rule 7: (f ; g) ; h → f ; (g ; h)
        PropExpr::Compose(ref outer_left, ref outer_right)
            if matches!(outer_left.as_ref(), PropExpr::Compose(_, _)) =>
        {
            if let PropExpr::Compose(f, g) = outer_left.as_ref() {
                let inner = PropExpr::Compose(g.clone(), outer_right.clone());
                return apply_smc_rules(&PropExpr::Compose(f.clone(), Box::new(inner)));
            }
            PropExpr::Compose(outer_left.clone(), outer_right.clone())
        }
        // Rule 2: Identity(0) ⊗ f → f
        PropExpr::Tensor(ref f, ref g) if matches!(f.as_ref(), PropExpr::Identity(0)) => {
            apply_smc_rules(g)
        }
        // Rule 3: f ⊗ Identity(0) → f
        PropExpr::Tensor(ref f, ref g) if matches!(g.as_ref(), PropExpr::Identity(0)) => {
            apply_smc_rules(f)
        }
        // Rule 9: Identity(m) ⊗ Identity(n) → Identity(m+n)
        //                  (identity-coherence of the monoidal product)
        PropExpr::Tensor(ref f, ref g)
            if matches!(f.as_ref(), PropExpr::Identity(_))
                && matches!(g.as_ref(), PropExpr::Identity(_)) =>
        {
            if let (PropExpr::Identity(m), PropExpr::Identity(n)) = (f.as_ref(), g.as_ref())
                // #196: the fused width is a magnitude — every consumer of an
                // `Identity(k)` sizes from `k` — so on overflow the rule simply
                // does not fire and the `Tensor` stands. Declining a rewrite is
                // always sound; wrapping onto `m + n - usize::MAX` is not.
                && let Some(fused) = m.checked_add(*n)
            {
                return PropExpr::Identity(fused);
            }
            PropExpr::Tensor(f.clone(), g.clone())
        }
        // Rule 4: (f ⊗ g) ⊗ h → f ⊗ (g ⊗ h)
        PropExpr::Tensor(ref outer_left, ref outer_right)
            if matches!(outer_left.as_ref(), PropExpr::Tensor(_, _)) =>
        {
            if let PropExpr::Tensor(f, g) = outer_left.as_ref() {
                let inner = PropExpr::Tensor(g.clone(), outer_right.clone());
                return apply_smc_rules(&PropExpr::Tensor(f.clone(), Box::new(inner)));
            }
            PropExpr::Tensor(outer_left.clone(), outer_right.clone())
        }
        other => other,
    }
}

/// Rewrite `expr`: if the whole tree matches `lhs` structurally, return
/// `rhs.clone()`; otherwise recurse into Compose/Tensor children so equations
/// can match subterms.
fn rewrite_once_top<G: PropSignature>(
    expr: &PropExpr<G>,
    lhs: &PropExpr<G>,
    rhs: &PropExpr<G>,
) -> PropExpr<G> {
    if expr == lhs {
        rhs.clone()
    } else {
        match expr {
            PropExpr::Compose(f, g) => PropExpr::Compose(
                Box::new(rewrite_once_top(f, lhs, rhs)),
                Box::new(rewrite_once_top(g, lhs, rhs)),
            ),
            PropExpr::Tensor(f, g) => PropExpr::Tensor(
                Box::new(rewrite_once_top(f, lhs, rhs)),
                Box::new(rewrite_once_top(g, lhs, rhs)),
            ),
            other => other.clone(),
        }
    }
}

/// A presented prop: wraps a [`Presentation`] with methods for operating on
/// equivalence classes. Surfaces [`PresentedProp::presentation`]
/// and [`PresentedProp::quotient_representative`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PresentedProp<G: PropSignature> {
    presentation: Presentation<G>,
}

impl<G: PropSignature> PresentedProp<G> {
    /// Wrap a presentation as a presented prop.
    #[must_use]
    pub fn new(presentation: Presentation<G>) -> Self {
        Self { presentation }
    }

    /// Borrow the underlying presentation.
    #[must_use]
    pub fn presentation(&self) -> &Presentation<G> {
        &self.presentation
    }

    /// Returns the canonical representative of the equivalence class of `expr`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Presentation`] on normalize failure
    /// (currently unreachable).
    pub fn quotient_representative(
        &self,
        expr: &PropExpr<G>,
    ) -> Result<NormalizeResult<G>, CatgraphError> {
        self.presentation.normalize(expr)
    }
}
