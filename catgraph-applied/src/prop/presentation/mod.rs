//! Prop presentations (F&S Def 5.33): equations quotienting `Free(G)`.
//!
//! A presentation `(G, s, t, E)` consists of a signature `G` with interface
//! maps `s, t` (provided via [`super::PropSignature`]) and a set `E` of
//! equations, each a pair `(lhs, rhs)` of [`super::PropExpr<G>`] that are
//! *parallel* — same source and target **words** over `Λ`, inferred by
//! [`Presentation::add_equation`]. The presented prop is `Free(G)` quotiented
//! by the smallest congruence containing `E` plus the SMC axioms. Rewriting is
//! bounded-depth (default 32): the 9 fixed **SMC-canonical-form rules** below
//! apply first, then the user equations `E` left-to-right.
//!
//! # SMC rules
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
//! — a conservative answer.

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

/// Engine selector for [`Presentation::eq_mod`], which it affects only;
/// [`Presentation::normalize`] is always bounded structural rewriting.
///
/// - [`NormalizeEngine::Structural`]: normalize both sides via bounded structural
///   rewriting and compare. May yield false negatives (`None`) on overlapping
///   equations (e.g. the 18 Thm 5.60 scalar D-group equations).
/// - [`NormalizeEngine::CongruenceClosure`] (default): decide equality via
///   bounded congruence closure over [`kb::CongruenceClosure`], which is correct
///   for any equational theory without binders, overlapping equations included.
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
/// engine). **Deserialization does not re-run [`Self::add_equation`]'s check** —
/// it reconstructs the fields directly, so a hand-crafted document can carry an
/// equation that is word-ill-formed or color-mismatched between its two sides.
/// Round-tripping a value produced by this crate is always safe; when ingesting
/// untrusted documents, re-validate by rebuilding via [`Self::add_equation`].
/// The same boundary applies to [`ColoredExpr`].
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

    /// New empty presentation with an explicit engine selector. Depth defaults
    /// to `32`.
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

    /// Add an equation `lhs = rhs`. Both sides must be **parallel morphisms over
    /// a common source word** — boundary-*word* equality, not merely matching
    /// arities. Fresh variables stand for the unknown source colors and both
    /// sides are threaded through the same variables, so a constraint discovered
    /// on either side propagates to the other; the two resulting target words are
    /// then unified pairwise. Acceptance means such a shared source word exists.
    /// Sides that are individually color-polymorphic but *jointly* constrained
    /// are accepted — `Identity(2) = Braid(1,1)` forces the two source positions
    /// to share a color — and the inferred constraint is **not stored**.
    ///
    /// # Errors
    ///
    /// - [`CatgraphError::CompositionSizeMismatch`] on any length disagreement:
    ///   a subterm handed a word of the wrong length, `rhs`'s source arity
    ///   differing from `lhs`'s, or target words of different lengths. Also when
    ///   `lhs`'s source arity reads `usize::MAX`, which is screened up front
    ///   rather than compared.
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
    #[must_use]
    pub fn equations(&self) -> &[(PropExpr<G>, PropExpr<G>)] {
        &self.equations
    }

    /// Normalize `expr` to canonical form under the SMC rules + user equations.
    /// Termination is always guaranteed by the depth bound; on a cyclic equation
    /// set the result is whichever representative was reached when the bound was
    /// hit. The returned [`NormalizeResult`] carries `.expr` (the possibly-partial
    /// normalized expression), `.converged` (`true` iff a fixpoint was reached
    /// before the depth bound) and `.steps_taken` (rewrite iterations performed).
    ///
    /// # Errors
    ///
    /// Currently infallible; returns [`CatgraphError::Presentation`] for
    /// forward-compatibility.
    pub fn normalize(&self, expr: &PropExpr<G>) -> Result<NormalizeResult<G>, CatgraphError> {
        let mut current = expr.clone();
        for step in 0..self.rewrite_depth {
            let after_smc = apply_smc_rules(&current);
            let after_user = self.apply_user_equations(&after_smc);
            if after_user == current {
                return Ok(NormalizeResult {
                    expr: current,
                    converged: true,
                    // A full iteration runs before the fixpoint check, so the
                    // 0-indexed `step` counts `step + 1` iterations performed.
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
    /// rules to a fixpoint, **without** applying user equations. Used as the CC
    /// engine's pre-pass, so the congruence-closure graph is fed
    /// SMC-canonicalized operands and seeded equations.
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

    /// Equality modulo this presentation, dispatching on [`Presentation::engine`]:
    ///
    /// - [`NormalizeEngine::Structural`]: normalize both sides via bounded
    ///   structural rewriting and compare. Returns `Ok(Some(true))` /
    ///   `Ok(Some(false))` when both sides converge, or `Ok(None)` if at least
    ///   one side hit the depth bound.
    /// - [`NormalizeEngine::CongruenceClosure`] (default): decide equality via
    ///   bounded congruence closure, over both sides and the presentation's
    ///   equations SMC-normalized first. `Ok(None)` when any of those hits the
    ///   depth bound.
    ///
    /// # The two layers
    ///
    /// Under the default engine, SMC coherence — associator, unitor, interchange,
    /// braid-naturality, `σ² = id` — is decided by [`content`], exactly: Lemma 4.1
    /// of `docs/SMC-NF-RECONCILIATION.md` §4.2 says `C(a) = C(b)` **iff** `a` and
    /// `b` are equal in the free symmetric monoidal category. The user equations
    /// `E` are decided above that layer by [`kb::CongruenceClosure`], which uses
    /// [`smc_nf::nf`] as the canonicalizer inside its `smc_refine` fixpoint. So a
    /// `true` is exact at the SMC layer and sound at the user layer; a `false` is
    /// only as complete as congruence closure is.
    ///
    /// **Sound per query, but not an equivalence relation as a decision
    /// procedure.** Every verdict is sound and definite for the pair it was asked
    /// about, but verdicts do not compose: the relation is reflexive and symmetric
    /// and **not transitive**. Under `matr_presentation(&[false, true])`,
    /// `Scalar(false)` ~ `Discard ; Zero` (the D8 user equation) and
    /// `Discard ; Zero` ~ `Discard ⊗ Zero` (the SMC layer), yet `Scalar(false)`
    /// ≁ `Discard ⊗ Zero` — and that last one is `Some(false)`, not `None`. A
    /// caller that wants a *partition* out of this must therefore take the
    /// connected components of the `Some(true)` graph.
    ///
    /// **Arity-ill-formed input.** `PropExpr`'s variants are public, so a
    /// hand-built tree can compose mismatched arities. This method stays total on
    /// those: the content layer is gated on [`content::is_arity_well_formed`], and
    /// such a tree falls through to the `nf` short-circuit and then to congruence
    /// closure. No error variant, and no panic leaking out of
    /// [`content::content_of`].
    ///
    /// **Overflowing arities.** A hand-built tree can also carry a `Braid` or
    /// `Tensor` width that sums past `usize::MAX` — a magnitude neither layer
    /// below can size from, so both reject it outright. This method screens it
    /// first, before either engine runs, and answers `Ok(Some(true))` when the two
    /// trees are structurally identical and `Ok(None)` — undecided — otherwise.
    /// `Ok(Some(false))` is never returned there.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Presentation`] if normalization fails for either
    /// side (currently unreachable).
    pub fn eq_mod(&self, a: &PropExpr<G>, b: &PropExpr<G>) -> Result<Option<bool>, CatgraphError> {
        // Neither engine is defined on an overflowing width, so screen once here.
        if !a.arities_fit() || !b.arities_fit() {
            return Ok((a == b).then_some(true));
        }
        match self.engine {
            NormalizeEngine::Structural => {
                let na = self.normalize(a)?;
                let nb = self.normalize(b)?;
                if !na.converged || !nb.converged {
                    return Ok(None);
                }
                Ok(Some(na.expr == nb.expr))
            }
            NormalizeEngine::CongruenceClosure => {
                // Settle the SMC layer first via content equality, then fall back
                // to the CC engine with the `normalize_smc_only` pre-pass.
                // `content_of` panics outside its arity-well-formed domain, so
                // the content arm is gated and an ill-formed tree takes the NF
                // short-circuit instead.
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
    /// # Examples
    ///
    /// No constructor takes a depth *and* an engine, so rebuilding a stored
    /// `(equations, depth, engine)` triple is two calls before the equations:
    ///
    /// ```ignore
    /// let mut rebuilt = Presentation::with_depth(depth);
    /// rebuilt.set_engine(engine);
    /// for (lhs, rhs) in equations { rebuilt.add_equation(lhs, rhs)?; }
    /// ```
    #[must_use]
    pub fn rewrite_depth(&self) -> usize {
        self.rewrite_depth
    }

    /// Decide equality using a semantic functor `f : Free(G) → T` that is
    /// *complete* on this presentation: the test is `f(a) == f(b)` in the
    /// functor's target type. The decision is always definite (`Ok(Some(_))`) —
    /// no depth bound, no syntactic rewriting. Completeness is an external claim
    /// carried by the functor implementation.
    ///
    /// # Examples
    ///
    /// `Free(Σ_SFG)/⟨E_{18}⟩ ≅ Mat(R)` (F&S Thm 5.60; Baez-Erbele 2015 for
    /// fields, Wadsley–Woods arXiv:1505.00048 for commutative rigs, cf. BE15 §6),
    /// so two signal-flow graphs are equivalent under the 18 Thm 5.60 equations
    /// iff their matrix images are equal:
    ///
    /// ```ignore
    /// let f = MatrixNFFunctor::<BoolRig>::new();
    /// assert_eq!(pres.eq_mod_functorial(&a, &b, &f)?, Some(true));
    /// ```
    ///
    /// # Errors
    ///
    /// Propagates [`CatgraphError`] from `f.apply` if either input expression is
    /// ill-formed.
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
    /// [`ColoredCompleteFunctor`] that is complete on this presentation — the
    /// worded sibling of [`Self::eq_mod_functorial`]. Two morphisms are equal
    /// only if they are **parallel**, so `a` and `b` compare unequal —
    /// `Ok(Some(false))`, definitely, without consulting the functor — whenever
    /// their source or target words differ, even if their images coincide. Only
    /// when the words agree is `F(a) == F(b)` consulted, and then the decision is
    /// definite.
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

    // Order matters: identity reductions and braid-involution before the
    // associators, which only rebalance structure.
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
                && f1.target() == f2.source()
                && g1.target() == g2.source()
            {
                let f12 = PropExpr::Compose(f1.clone(), f2.clone());
                let g12 = PropExpr::Compose(g1.clone(), g2.clone());
                return apply_smc_rules(&PropExpr::Tensor(Box::new(f12), Box::new(g12)));
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
                // On overflow the rule does not fire and the `Tensor` stands.
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
/// equivalence classes.
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
