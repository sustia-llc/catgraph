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
//! The converse holds as **rigidity on the fragment `𝔉′`** — braid-free,
//! every `η` placement-pinned (**§4.4** Theorem 4.5) — with the `nf`-level
//! corollary there *conditional* (fixpoints must be braid-free and
//! invariant-satisfying; the blocking engine asymmetry is filed, §4.4's
//! conditional corollary), and it is open beyond `𝔉′`: §4.6's sweep still
//! finds SMC-equal pairs with distinct NFs. So a `true` from `eq_colored` is
//! sound everywhere; a `false` is **not a disproof** in general — on `𝔉′` it
//! becomes one only once the conditional corollary's engine condition is
//! discharged.
//!
//! # Equations
//!
//! An equation `lhs = rhs` has no *declared* source word — the two sides are
//! required to be parallel over *some* common one. That is the P2 inference
//! pass behind [`Presentation::add_equation`]: fresh variables stand for the
//! unknown source colors, both sides are threaded through the same variables,
//! and the two target words are unified pairwise. See `check_equation`.
//!
//! [module docs]: super
//! [`PropExpr::Identity`]: super::PropExpr::Identity
//! [`PropExpr::Braid`]: super::PropExpr::Braid
//! [`Presentation::add_equation`]: super::presentation::Presentation::add_equation

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

// ---- Equation-level word inference (#79 P2) ---------------------------------

/// A letter of an *inferred* word: a unification variable standing for an
/// as-yet-unconstrained color, or a concrete color.
#[derive(Clone, Debug)]
enum Term<C> {
    Var(usize),
    Const(C),
}

/// Union-find over an equation's source variables, one optional color binding
/// per class.
///
/// Variables may alias other variables and bind to colors; a class carries at
/// most one color, so binding a second, different one is the color conflict the
/// caller reports. Conflicts surface as the `(left, right)` pair of colors, in
/// the argument order [`Self::unify`] was called with.
struct Unifier<C> {
    parent: Vec<usize>,
    color: Vec<Option<C>>,
}

impl<C: Clone + PartialEq> Unifier<C> {
    fn new() -> Self {
        Self {
            parent: Vec::new(),
            color: Vec::new(),
        }
    }

    fn fresh(&mut self) -> usize {
        let v = self.parent.len();
        self.parent.push(v);
        self.color.push(None);
        v
    }

    fn find(&mut self, v: usize) -> usize {
        let mut root = v;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cur = v;
        while self.parent[cur] != cur {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    /// Bind `v`'s class to `c`. `Err((found, c))` if it already carries another.
    fn bind(&mut self, v: usize, c: &C) -> Result<(), (C, C)> {
        let root = self.find(v);
        match &self.color[root] {
            Some(found) if found != c => Err((found.clone(), c.clone())),
            Some(_) => Ok(()),
            None => {
                self.color[root] = Some(c.clone());
                Ok(())
            }
        }
    }

    fn unify(&mut self, a: &Term<C>, b: &Term<C>) -> Result<(), (C, C)> {
        match (a, b) {
            (Term::Var(x), Term::Var(y)) => {
                let (rx, ry) = (self.find(*x), self.find(*y));
                if rx == ry {
                    return Ok(());
                }
                match (self.color[rx].clone(), self.color[ry].clone()) {
                    (Some(cx), Some(cy)) if cx != cy => return Err((cx, cy)),
                    (Some(cx), None) => self.color[ry] = Some(cx),
                    _ => {}
                }
                self.parent[rx] = ry;
                Ok(())
            }
            (Term::Var(x), Term::Const(c)) => self.bind(*x, c),
            // `bind` reports `(found, given)`; `found` is the *right* term here.
            (Term::Const(c), Term::Var(y)) => {
                self.bind(*y, c).map_err(|(found, given)| (given, found))
            }
            (Term::Const(x), Term::Const(y)) => {
                if x == y {
                    Ok(())
                } else {
                    Err((x.clone(), y.clone()))
                }
            }
        }
    }
}

/// Thread `input` through `expr`, recording the color constraints it implies.
///
/// The variable-threaded sibling of [`check`]: identical control flow, with
/// [`Term`]s in place of colors and unification in place of the generator's
/// word equality test. Errors follow [`check`]'s convention.
fn infer<G: PropSignature>(
    expr: &PropExpr<G>,
    input: &[Term<G::Color>],
    u: &mut Unifier<G::Color>,
) -> Result<Vec<Term<G::Color>>, CatgraphError> {
    match expr {
        PropExpr::Identity(n) => {
            expect_len(*n, input)?;
            Ok(input.to_vec())
        }
        PropExpr::Braid(m, n) => {
            expect_len(m + n, input)?;
            let mut out = Vec::with_capacity(input.len());
            out.extend_from_slice(&input[*m..]);
            out.extend_from_slice(&input[..*m]);
            Ok(out)
        }
        PropExpr::Generator(g) => {
            let want = g.source_word();
            expect_len(want.len(), input)?;
            for (i, (have, declared)) in input.iter().zip(want.iter()).enumerate() {
                u.unify(have, &Term::Const(declared.clone()))
                    .map_err(|(found, _)| CatgraphError::Composition {
                        message: format!(
                            "colored inference: generator {g:?} declares source color {declared:?} at position {i} but the incoming word carries {found:?}"
                        ),
                    })?;
            }
            Ok(g.target_word().iter().cloned().map(Term::Const).collect())
        }
        PropExpr::Compose(f, h) => {
            let mid = infer(f, input, u)?;
            infer(h, &mid, u)
        }
        PropExpr::Tensor(f, h) => {
            let split = f.source();
            expect_at_least(split, input)?;
            let (left, right) = input.split_at(split);
            let mut out = infer(f, left, u)?;
            out.extend(infer(h, right, u)?);
            Ok(out)
        }
    }
}

/// Check that `lhs` and `rhs` are parallel morphisms over a **common source
/// word** — the boundary-word equality test behind
/// [`Presentation::add_equation`].
///
/// Both sides are threaded through the *same* fresh source variables, so a
/// constraint discovered on either side propagates to the other; the two target
/// words are then unified pairwise. Success means such a shared word exists
/// (the most general one under the inferred constraints). The constraint itself
/// is not returned — see the caller's rustdoc for why that is sound today.
///
/// # Errors
///
/// - [`CatgraphError::CompositionSizeMismatch`] on any length disagreement: a
///   subterm handed a word of the wrong length, `rhs`'s source arity differing
///   from `lhs`'s, or target words of different lengths.
/// - [`CatgraphError::Composition`] when the lengths agree but the colors
///   conflict — the message names the generator and position, or the target
///   position, and both colors.
///
/// [`Presentation::add_equation`]: super::presentation::Presentation::add_equation
pub(crate) fn check_equation<G: PropSignature>(
    lhs: &PropExpr<G>,
    rhs: &PropExpr<G>,
) -> Result<(), CatgraphError> {
    let mut u = Unifier::new();
    let source: Vec<Term<G::Color>> = (0..lhs.source()).map(|_| Term::Var(u.fresh())).collect();
    let lhs_target = infer(lhs, &source, &mut u)?;
    let rhs_target = infer(rhs, &source, &mut u)?;
    if lhs_target.len() != rhs_target.len() {
        return Err(CatgraphError::CompositionSizeMismatch {
            expected: lhs_target.len(),
            actual: rhs_target.len(),
        });
    }
    for (i, (l, r)) in lhs_target.iter().zip(rhs_target.iter()).enumerate() {
        u.unify(l, r)
            .map_err(|(left, right)| CatgraphError::Composition {
                message: format!(
                    "colored inference: target position {i} is {left:?} on the lhs but {right:?} on the rhs"
                ),
            })?;
    }
    Ok(())
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
/// produced by this crate is always safe. This mirrors the boundary documented
/// on [`Presentation`] (#81), whose [`add_equation`] check is boundary-word
/// equality since #79 P2 and is likewise not re-run on deserialization. When
/// ingesting untrusted documents, re-validate by rebuilding via [`Self::new`].
///
/// [`add_equation`]: super::presentation::Presentation::add_equation
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
    /// readback). A `false` is not a disproof — §4.4's canonicality
    /// corollary on `𝔉′` is currently *conditional*; see the module docs.
    #[must_use]
    pub fn eq_colored(&self, other: &Self) -> bool {
        self.source_word == other.source_word
            && self.target_word == other.target_word
            && nf(&self.expr) == nf(&other.expr)
    }
}
