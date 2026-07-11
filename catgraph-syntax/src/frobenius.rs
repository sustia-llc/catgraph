//! The Frobenius layer (Phase S4): the monochromatic free hypergraph category
//! as a **sum over** a user signature, plus the spider calculus, the special
//! commutative Frobenius monoid (SCFM) equations, and a sound semantic checker
//! into [`MatKron(R)`](catgraph_applied::mat_kron::MatKron).
//!
//! # The load-bearing shape: a sum type, not a second AST
//!
//! [`FrobeniusOr<G>`] adjoins the four Frobenius generators μ/η/δ/ε to a user
//! signature `G` as *extra generators of the same prop*. Because it is itself a
//! [`PropSignature`], **every** applied and textual surface — the normal-form
//! engine, [`Presentation`], `eq_mod`, the S2
//! [parser](mod@crate::text::parse)/[printer](mod@crate::text::print), and the S3
//! [`eval`](mod@crate::eval) — works over `PropExpr<FrobeniusOr<G>>` unchanged, with
//! no new interpreter or new tree type. The Frobenius structure is data in the
//! signature, not a fork of the engine.
//!
//! # Anchors (Fong & Spivak 2019, *Hypergraph Categories*, arXiv:1806.08304v3)
//!
//! - **Def 2.5** — a special commutative Frobenius monoid `(X, μ, η, δ, ε)` and
//!   its *nine* defining equations (see [`scfm_equations`]). The Hadamard SCFM
//!   on `R^dim` (Ex 2.16, realized by [`MatKron`](catgraph_applied::mat_kron))
//!   satisfies them — the S4 milestone law.
//! - **Def 2.12** — a hypergraph category: a symmetric monoidal category in
//!   which every object carries a coherent SCFM, and morphisms respect it.
//! - **Prop 3.8** — `Cospan` is the theory of SCFMs: an SCFM in a symmetric
//!   monoidal category `C` is exactly a strict SM functor `(Cospan, ⊕) → (C, ⊗)`.
//!   This is the soundness anchor for [`to_mat_kron`]: the Hadamard SCFM on
//!   `R^dim` *is* such a functor, so mapping a Frobenius term to its `MatKron`
//!   image respects all nine equations.
//! - **Thm 3.14** — `Cospan_Λ` is the free hypergraph category on `Λ`. Here the
//!   colour palette is the monochromatic `Λ = {•}` (disclaimer 2), so the free
//!   hypergraph category is `Cospan` (one spider family), and a wire `•` maps to
//!   the object `R^dim`.
//! - **Ex 2.16** — [`MatKron(R)`](catgraph_applied::mat_kron), the Hadamard-SCFM
//!   Kronecker hypergraph category, the semantic target of [`to_mat_kron`].
//!
//! # ⚠ Nine equations, not ten
//!
//! The SCFM has **nine** defining equations, per the paper itself: Ex 2.8 refers
//! to "the nine equations in Definition 2.5", and the page-10 diagrams confirm
//! it (associativity, left unitality, commutativity; coassociativity, left
//! counitality, cocommutativity; the two-equation Frobenius chain; speciality —
//! `3 + 3 + 2 + 1 = 9`). Right unitality/counitality are *derivable* from the
//! left law via commutativity/cocommutativity and are not counted. An earlier
//! design note said "ten"; the paper is the spec, so [`scfm_equations`] ships
//! nine and cites Ex 2.8's count.
//!
//! # The [#15](https://github.com/sustia-llc/catgraph/issues/15) boundary, restated
//!
//! [`hypergraph_presentation`] seeds a [`Presentation`] with the nine equations
//! `E_frob`. Deciding equality over it goes through applied's
//! [`eq_mod`](catgraph_applied::prop::presentation::Presentation::eq_mod), which
//! is **sound but syntactically incomplete** (#15): a `Some(true)` is a proof,
//! but a `None`/`Some(false)` is *not* a disproof — only that the congruence
//! closure did not establish the equation. `E_frob` overlaps heavily (spiders
//! fuse in many ways), so `None` is the expected answer for many true equalities.
//! [`to_mat_kron`] is a **sound** semantic check (Prop 3.8), **not** registered
//! as a
//! [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor):
//! no completeness theorem is claimed for `E_frob` here (the Cospan-valued
//! complete-functor spike is [#80](https://github.com/sustia-llc/catgraph/issues/80)).
//! Equal `MatKron` images witness equality *in `MatKron(R)`*; they do not
//! promote an incomplete `eq_mod None` into a syntactic decision.
//!
//! # Monochromatic scope (disclaimer 2)
//!
//! One wire colour `Λ = {•}`, one spider family. F&S 2019 Thm 3.14's full
//! **colored** generality (a distinct spider per colour) is out of scope and
//! tracked as [#79](https://github.com/sustia-llc/catgraph/issues/79).

use catgraph::category::Composable;

use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_applied::rig::Rig;

use crate::errors::SyntaxError;
use crate::text::GeneratorSyntax;

/// A user signature `G` extended with the four special-commutative-Frobenius
/// generators — the generators of the monochromatic free hypergraph category
/// laid *over* `G` (F&S 2019 Def 2.5 / Def 2.12).
///
/// This is a **sum type over the user signature**, not a second AST: it is a
/// [`PropSignature`], so `PropExpr<FrobeniusOr<G>>` is an ordinary free-prop term
/// and reuses every existing surface (NF, presentation, `eq_mod`, parser,
/// printer, `eval`) with no engine change. Arities follow Def 2.5:
///
/// | Variant | Diagram | Arity |
/// |---|---|---|
/// | [`Mu`](FrobeniusOr::Mu) | `μ` multiplication | `2 → 1` |
/// | [`Eta`](FrobeniusOr::Eta) | `η` unit | `0 → 1` |
/// | [`Delta`](FrobeniusOr::Delta) | `δ` comultiplication | `1 → 2` |
/// | [`Epsilon`](FrobeniusOr::Epsilon) | `ε` counit | `1 → 0` |
/// | [`User(g)`](FrobeniusOr::User) | a `G`-generator | `g.source() → g.target()` |
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FrobeniusOr<G> {
    /// Multiplication `μ : 2 → 1` (the merge).
    Mu,
    /// Unit `η : 0 → 1` (the merge's identity).
    Eta,
    /// Comultiplication `δ : 1 → 2` (the split).
    Delta,
    /// Counit `ε : 1 → 0` (the split's identity).
    Epsilon,
    /// A generator of the underlying user signature `G`.
    User(G),
}

/// An arity-matched `(lhs, rhs)` equation over the Frobenius signature — the
/// shape of one `E_frob` entry, of each [`scfm_equations`] element, and of a
/// user equation once lifted into `FrobeniusOr<G>`.
pub type FrobeniusEquation<G> = (PropExpr<FrobeniusOr<G>>, PropExpr<FrobeniusOr<G>>);

impl<G: PropSignature> PropSignature for FrobeniusOr<G> {
    fn source(&self) -> usize {
        match self {
            FrobeniusOr::Mu => 2,
            FrobeniusOr::Eta => 0,
            FrobeniusOr::Delta | FrobeniusOr::Epsilon => 1,
            FrobeniusOr::User(g) => g.source(),
        }
    }

    fn target(&self) -> usize {
        match self {
            FrobeniusOr::Mu | FrobeniusOr::Eta => 1,
            FrobeniusOr::Delta => 2,
            FrobeniusOr::Epsilon => 0,
            FrobeniusOr::User(g) => g.target(),
        }
    }
}

// ---- Frobenius generator leaves (private smart-constructor shims) -------------
//
// One definition of each Frobenius leaf; every builder below reads from these so
// the variant-to-`Free::generator` mapping lives in exactly one place.

/// `μ : 2 → 1` as a term.
fn mu<G: PropSignature>() -> PropExpr<FrobeniusOr<G>> {
    Free::generator(FrobeniusOr::Mu)
}

/// `η : 0 → 1` as a term.
fn eta<G: PropSignature>() -> PropExpr<FrobeniusOr<G>> {
    Free::generator(FrobeniusOr::Eta)
}

/// `δ : 1 → 2` as a term.
fn delta<G: PropSignature>() -> PropExpr<FrobeniusOr<G>> {
    Free::generator(FrobeniusOr::Delta)
}

/// `ε : 1 → 0` as a term.
fn epsilon<G: PropSignature>() -> PropExpr<FrobeniusOr<G>> {
    Free::generator(FrobeniusOr::Epsilon)
}

/// `id_n : n → n` over [`FrobeniusOr<G>`].
fn id<G: PropSignature>(n: usize) -> PropExpr<FrobeniusOr<G>> {
    Free::identity(n)
}

/// Lift a user term into the Frobenius prop, wrapping every generator as
/// [`User`](FrobeniusOr::User) and mapping every other node identically.
///
/// The map is **structural and arity-preserving**: `Generator(g) ↦
/// Generator(User(g))`, `Identity(n) ↦ Identity(n)`, `Braid(m, n) ↦ Braid(m,
/// n)`, and `Compose`/`Tensor` recurse. Because `User(g)` has exactly `g`'s
/// arities, an interior `Free::compose` re-composition can never fail — the
/// `.expect` documents that invariant. This is the inclusion `Free(G) ↪
/// Free(FrobeniusOr<G>)` of the user prop into the hypergraph prop.
#[must_use]
pub fn lift_user<G: PropSignature>(expr: &PropExpr<G>) -> PropExpr<FrobeniusOr<G>> {
    match expr {
        PropExpr::Identity(n) => Free::identity(*n),
        PropExpr::Braid(m, n) => Free::braid(*m, *n),
        PropExpr::Generator(g) => Free::generator(FrobeniusOr::User(g.clone())),
        PropExpr::Compose(f, g) => Free::compose(lift_user(f), lift_user(g))
            .expect("invariant: lift_user preserves arities, so an interior compose cannot fail"),
        PropExpr::Tensor(f, g) => Free::tensor(lift_user(f), lift_user(g)),
    }
}

/// μ-comb collapsing `m` legs to a single wire (`m → 1`): the left-nested fold
/// `(((η|id) ⊗ id) ; μ) …`. Base cases `η` (`m = 0`) and `id(1)` (`m = 1`).
fn collapse<G: PropSignature>(m: usize) -> PropExpr<FrobeniusOr<G>> {
    match m {
        0 => eta(),
        1 => id(1),
        _ => Free::compose(Free::tensor(collapse::<G>(m - 1), id(1)), mu()).expect(
            "invariant: collapse(m-1):(m-1)→1 tensored with id(1) is m→2, then ;μ:2→1 gives m→1",
        ),
    }
}

/// δ-comb expanding a single wire to `n` legs (`1 → n`): the right-nested fold
/// `δ ; ((δ ; (… ⊗ id)) ⊗ id)`. Base cases `ε` (`n = 0`) and `id(1)` (`n = 1`).
fn expand<G: PropSignature>(n: usize) -> PropExpr<FrobeniusOr<G>> {
    match n {
        0 => epsilon(),
        1 => id(1),
        _ => Free::compose(delta(), Free::tensor(expand::<G>(n - 1), id(1))).expect(
            "invariant: δ:1→2 then (expand(n-1):1→(n-1) tensored with id(1)):2→n gives 1→n",
        ),
    }
}

/// The `(m, n)` **spider**: collapse `m` legs to one wire via a μ-comb, then
/// expand that wire to `n` legs via a δ-comb (source `m`, target `n`).
///
/// Construction: `spider(m, n) = collapse(m) ; expand(n)`, where the two combs
/// meet at the single collapsed middle wire — the composition therefore always
/// typechecks (the `.expect` states that invariant). The edge cases follow the
/// combs' bases:
///
/// - `spider(0, n)` starts from `η` (`collapse(0) = η`);
/// - `spider(m, 0)` ends at `ε` (`expand(0) = ε`);
/// - `spider(0, 0) = η ; ε` (the empty spider — the scalar `η ; ε`), matching
///   the design's chosen construction;
/// - `spider(1, 1)` is returned as the literal `id(1)` (both combs are `id(1)`,
///   so `collapse(1) ; expand(1)` would be `id(1) ; id(1)`; the single-node
///   `id(1)` is the documented canonical form for the identity spider).
///
/// Spiders **fuse**: `spider(m, k) ; spider(k, n)` and `spider(m, n)` denote the
/// same morphism (semantically checkable via [`to_mat_kron`]), the defining
/// property of the SCFM spider calculus (F&S 2019 §2.2).
#[must_use]
pub fn spider<G: PropSignature>(m: usize, n: usize) -> PropExpr<FrobeniusOr<G>> {
    if m == 1 && n == 1 {
        return id(1);
    }
    Free::compose(collapse::<G>(m), expand::<G>(n))
        .expect("invariant: collapse(m):m→1 meets expand(n):1→n at the single middle wire")
}

/// The compact-closed **cup** `0 → 2`, built as `η ; δ` — matching
/// [`MatKron::cup`](catgraph_applied::mat_kron::MatKron::cup) exactly under
/// [`to_mat_kron`].
#[must_use]
pub fn cup<G: PropSignature>() -> PropExpr<FrobeniusOr<G>> {
    Free::compose(eta(), delta()).expect("invariant: η:0→1 ; δ:1→2 gives 0→2")
}

/// The compact-closed **cap** `2 → 0`, built as `μ ; ε` — matching
/// [`MatKron::cap`](catgraph_applied::mat_kron::MatKron::cap) exactly under
/// [`to_mat_kron`].
#[must_use]
pub fn cap<G: PropSignature>() -> PropExpr<FrobeniusOr<G>> {
    Free::compose(mu(), epsilon()).expect("invariant: μ:2→1 ; ε:1→0 gives 2→0")
}

/// The **nine** defining equations of a special commutative Frobenius monoid
/// (F&S 2019 Def 2.5), as arity-matched `(lhs, rhs)` term pairs built through
/// [`Free`]. This is `E_frob`, the equation set [`hypergraph_presentation`]
/// seeds.
///
/// Ex 2.8 states the count outright — "the nine equations in Definition 2.5" —
/// and the page-10 string diagrams confirm each one (all read left-to-right,
/// `σ = braid(1, 1)`):
///
/// 1. **associativity** `(μ ⊗ id) ; μ = (id ⊗ μ) ; μ` (`3 → 1`)
/// 2. **unitality** (left) `(η ⊗ id) ; μ = id(1)` (`1 → 1`)
/// 3. **commutativity** `σ ; μ = μ` (`2 → 1`)
/// 4. **coassociativity** `δ ; (δ ⊗ id) = δ ; (id ⊗ δ)` (`1 → 3`)
/// 5. **counitality** (left) `δ ; (ε ⊗ id) = id(1)` (`1 → 1`)
/// 6. **cocommutativity** `δ ; σ = δ` (`1 → 2`)
/// 7. **Frobenius-L** `(δ ⊗ id) ; (id ⊗ μ) = μ ; δ` (`2 → 2`)
/// 8. **Frobenius-R** `(id ⊗ δ) ; (μ ⊗ id) = μ ; δ` (`2 → 2`)
/// 9. **speciality** `δ ; μ = id(1)` (`1 → 1`)
///
/// Equations 7 and 8 are the two halves of the paper's three-term Frobenius
/// chain `(δ ⊗ id) ; (id ⊗ μ) = μ ; δ = (id ⊗ δ) ; (μ ⊗ id)`; the right
/// unitality/counitality (Def 2.5's diagrams show only the left) are derivable
/// via commutativity/cocommutativity and are not among the nine.
#[must_use]
pub fn scfm_equations<G: PropSignature>() -> Vec<FrobeniusEquation<G>> {
    // Local composition shim: every pair below is arity-matched by the arities
    // annotated in this function's rustdoc, so `Free::compose` cannot fail here.
    let c = |f: PropExpr<FrobeniusOr<G>>, g: PropExpr<FrobeniusOr<G>>| {
        Free::compose(f, g).expect("invariant: scfm_equations builds arity-matched compositions")
    };
    let braid = Free::<FrobeniusOr<G>>::braid(1, 1);

    vec![
        // 1. associativity: (μ ⊗ id) ; μ = (id ⊗ μ) ; μ.
        (
            c(Free::tensor(mu(), id(1)), mu()),
            c(Free::tensor(id(1), mu()), mu()),
        ),
        // 2. unitality (left): (η ⊗ id) ; μ = id(1).
        (c(Free::tensor(eta(), id(1)), mu()), id(1)),
        // 3. commutativity: braid(1,1) ; μ = μ.
        (c(braid.clone(), mu()), mu()),
        // 4. coassociativity: δ ; (δ ⊗ id) = δ ; (id ⊗ δ).
        (
            c(delta(), Free::tensor(delta(), id(1))),
            c(delta(), Free::tensor(id(1), delta())),
        ),
        // 5. counitality (left): δ ; (ε ⊗ id) = id(1).
        (c(delta(), Free::tensor(epsilon(), id(1))), id(1)),
        // 6. cocommutativity: δ ; braid(1,1) = δ.
        (c(delta(), braid), delta()),
        // 7. Frobenius-L: (δ ⊗ id) ; (id ⊗ μ) = μ ; δ.
        (
            c(Free::tensor(delta(), id(1)), Free::tensor(id(1), mu())),
            c(mu(), delta()),
        ),
        // 8. Frobenius-R: (id ⊗ δ) ; (μ ⊗ id) = μ ; δ.
        (
            c(Free::tensor(id(1), delta()), Free::tensor(mu(), id(1))),
            c(mu(), delta()),
        ),
        // 9. speciality: δ ; μ = id(1).
        (c(delta(), mu()), id(1)),
    ]
}

/// Build a [`Presentation`] of the monochromatic free hypergraph category over
/// `G`: lift each user equation via [`lift_user`], then add the nine
/// [`scfm_equations`] (`E_frob`).
///
/// The resulting presentation quotients `Free(FrobeniusOr<G>)` by the user
/// theory *plus* the SCFM laws. Deciding equality over it is applied's
/// [`eq_mod`](catgraph_applied::prop::presentation::Presentation::eq_mod), which
/// is **sound but incomplete** (#15): `Some(true)` is a proof, but a
/// `None`/`Some(false)` is not a disproof. `E_frob` overlaps (spiders fuse many
/// ways), so `None` is the *expected* answer for many genuine equalities.
/// **Complete** decisions come only through the functorial route
/// ([`eq_mod_functorial`](catgraph_applied::prop::presentation::Presentation::eq_mod_functorial)
/// with a
/// [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor));
/// today that is Mat(R) via Thm 5.60, and no complete functor is claimed for
/// `E_frob` (the Cospan-valued spike is
/// [#80](https://github.com/sustia-llc/catgraph/issues/80)). [`to_mat_kron`] is a
/// separate **sound semantic** check, not a syntactic decision.
///
/// # Errors
///
/// Returns [`SyntaxError::Catgraph`] if any lifted user equation or any
/// `E_frob` equation is arity-mismatched (surfaced transparently from
/// [`add_equation`](catgraph_applied::prop::presentation::Presentation::add_equation)).
/// The nine built-in equations are arity-matched by construction; an error here
/// can only originate in a caller-supplied user equation.
pub fn hypergraph_presentation<G: PropSignature>(
    user_eqs: impl IntoIterator<Item = (PropExpr<G>, PropExpr<G>)>,
) -> Result<Presentation<FrobeniusOr<G>>, SyntaxError> {
    let mut presentation = Presentation::<FrobeniusOr<G>>::new();
    for (lhs, rhs) in user_eqs {
        presentation.add_equation(lift_user(&lhs), lift_user(&rhs))?;
    }
    for (lhs, rhs) in scfm_equations::<G>() {
        presentation.add_equation(lhs, rhs)?;
    }
    Ok(presentation)
}

/// Compute `dim^exponent`, or a [`SyntaxError::DimensionOverflow`] if the power
/// exceeds `usize` — no panic, no runaway allocation. A `k`-wire interface maps
/// to the object of dimension `dim^k` (`dim^0 = 1`, the monoidal unit).
fn dim_pow(dim: usize, exponent: usize) -> Result<usize, SyntaxError> {
    u32::try_from(exponent)
        .ok()
        .and_then(|e| dim.checked_pow(e))
        .ok_or(SyntaxError::DimensionOverflow { dim, exponent })
}

/// Map a Frobenius term to its image in [`MatKron(R)`](catgraph_applied::mat_kron),
/// the Hadamard-SCFM Kronecker hypergraph category on `R^dim` — the **sound**
/// semantic checker for the SCFM laws (F&S 2019 Prop 3.8 / Ex 2.16).
///
/// # Mapping (a wire `•` ↦ the object `R^dim`, so a `k`-wire interface ↦ `dim^k`)
///
/// | Node | Image |
/// |---|---|
/// | `Identity(k)` | [`MatKron::identity(dim^k)`](catgraph_applied::mat_kron::MatKron::identity) |
/// | `Braid(m, n)` | [`MatKron::braiding(dim^m, dim^n)`](catgraph_applied::mat_kron::MatKron::braiding) |
/// | `Mu` | [`MatKron::mu(dim)`](catgraph_applied::mat_kron::MatKron::mu) (`dim² → dim`) |
/// | `Eta` | [`MatKron::eta(dim)`](catgraph_applied::mat_kron::MatKron::eta) (`1 → dim`) |
/// | `Delta` | [`MatKron::delta(dim)`](catgraph_applied::mat_kron::MatKron::delta) (`dim → dim²`) |
/// | `Epsilon` | [`MatKron::epsilon(dim)`](catgraph_applied::mat_kron::MatKron::epsilon) (`dim → 1`) |
/// | `Compose(f, g)` | matmul `f ; g` |
/// | `Tensor(f, g)` | Kronecker `f ⊗ g` |
/// | `User(g)` | **out of domain** — [`SyntaxError::NonFrobenius`] |
///
/// # Soundness, not completeness
///
/// The Hadamard SCFM on `R^dim` is a special commutative Frobenius monoid, so by
/// **Prop 3.8** it induces a strict SM functor from `Cospan` — the free
/// monochromatic hypergraph category (**Thm 3.14**, `Λ = {•}`) — into
/// `MatKron(R)`. `to_mat_kron` computes that functor. It therefore respects all
/// nine [`scfm_equations`]: for a proven-equal pair, the two images are equal
/// (the S4 milestone law, machine-checked). It is deliberately **not** registered
/// as a
/// [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor):
/// no completeness theorem is claimed for `E_frob` here, so equal images witness
/// equality *in `MatKron(R)`* and must not be read as a general syntactic
/// decision (the Cospan-valued complete-functor spike is
/// [#80](https://github.com/sustia-llc/catgraph/issues/80)).
///
/// `User(_)` generators are outside this functor's domain (the SCFM structure
/// carries no image for an arbitrary signature generator), so they are rejected
/// rather than assigned a guessed matrix.
///
/// # `dim ≥ 2`
///
/// At `dim = 1` every object `dim^k = 1` degenerates to the monoidal unit and
/// the check is vacuous; callers validating the laws must use `dim ≥ 2`.
///
/// # Errors
///
/// - [`SyntaxError::NonFrobenius`] if `expr` contains a `User(g)` generator.
/// - [`SyntaxError::DimensionOverflow`] if a `dim^k` interface dimension
///   overflows `usize` (e.g. a very wide `Identity`/`Braid` for the chosen
///   `dim`).
/// - [`SyntaxError::Catgraph`] if an interior composition's interface does not
///   meet (surfaced transparently from the `MatKron` matmul); this cannot arise
///   from a well-formed `PropExpr` built through [`Free`], only from a
///   directly-constructed ill-formed term.
pub fn to_mat_kron<G, R>(
    expr: &PropExpr<FrobeniusOr<G>>,
    dim: usize,
) -> Result<MatKron<R>, SyntaxError>
where
    G: PropSignature,
    R: Rig,
{
    match expr {
        PropExpr::Identity(k) => Ok(MatKron::identity(dim_pow(dim, *k)?)),
        PropExpr::Braid(m, n) => Ok(MatKron::braiding(dim_pow(dim, *m)?, dim_pow(dim, *n)?)),
        PropExpr::Generator(g) => match g {
            FrobeniusOr::Mu => Ok(MatKron::mu(dim)),
            FrobeniusOr::Eta => Ok(MatKron::eta(dim)),
            FrobeniusOr::Delta => Ok(MatKron::delta(dim)),
            FrobeniusOr::Epsilon => Ok(MatKron::epsilon(dim)),
            FrobeniusOr::User(u) => Err(SyntaxError::NonFrobenius {
                generator: format!("{u:?}"),
            }),
        },
        PropExpr::Compose(f, g) => {
            let fm = to_mat_kron::<G, R>(f, dim)?;
            let gm = to_mat_kron::<G, R>(g, dim)?;
            // A matmul interface mismatch surfaces transparently as Catgraph.
            Ok(fm.compose(&gm)?)
        }
        PropExpr::Tensor(f, g) => {
            let fm = to_mat_kron::<G, R>(f, dim)?;
            let gm = to_mat_kron::<G, R>(g, dim)?;
            Ok(fm.kron(&gm))
        }
    }
}

/// [`GeneratorSyntax`] for [`FrobeniusOr<G>`]: the four Frobenius generators
/// print/parse as the reserved tokens `mu` / `eta` / `delta` / `epsilon`, and
/// [`User(g)`](FrobeniusOr::User) delegates to `g`'s own token.
///
/// # `mu`/`eta`/`delta`/`epsilon` are reserved *within* `FrobeniusOr`
///
/// [`parse_token`](GeneratorSyntax::parse_token) tries the four Frobenius names
/// **first**, so a user generator whose token is spelled `mu` / `eta` / `delta`
/// / `epsilon` is **shadowed** inside `FrobeniusOr<G>`: its printed token
/// reparses to the Frobenius generator, not `User(g)`, breaking the trait's
/// clause-1 round-trip for that `G`. Treat these four names the way the grammar
/// treats `id` / `braid` (clause 2's reserved keywords): a signature intended to
/// be wrapped in `FrobeniusOr` must not use them as generator tokens. This is an
/// impl-level restriction on top of the trait's clause-2 alphabet, not a change
/// to clause 2 itself. Every other `G`-token round-trips unchanged (the four
/// Frobenius names each contain no grammar metacharacter, so they satisfy
/// clause 2).
impl<G: GeneratorSyntax> GeneratorSyntax for FrobeniusOr<G> {
    fn print_token(&self) -> String {
        match self {
            FrobeniusOr::Mu => "mu".to_string(),
            FrobeniusOr::Eta => "eta".to_string(),
            FrobeniusOr::Delta => "delta".to_string(),
            FrobeniusOr::Epsilon => "epsilon".to_string(),
            FrobeniusOr::User(g) => g.print_token(),
        }
    }

    fn parse_token(token: &str) -> Option<Self> {
        match token {
            "mu" => Some(FrobeniusOr::Mu),
            "eta" => Some(FrobeniusOr::Eta),
            "delta" => Some(FrobeniusOr::Delta),
            "epsilon" => Some(FrobeniusOr::Epsilon),
            _ => G::parse_token(token).map(FrobeniusOr::User),
        }
    }
}
