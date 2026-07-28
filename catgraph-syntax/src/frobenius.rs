//! The Frobenius layer (Phase S4): the free hypergraph category on a colour
//! palette `Λ` as a **sum over** a user signature, plus the spider calculus, the
//! special commutative Frobenius monoid (SCFM) equations, and a sound semantic
//! checker into [`MatKron(R)`](catgraph_applied::mat_kron::MatKron).
//!
//! # The load-bearing shape: a sum type, not a second AST
//!
//! [`FrobeniusOr<G>`] adjoins the four Frobenius generators μ/η/δ/ε — one family
//! **per colour** — to a user signature `G` as *extra generators of the same
//! prop*. Because it is itself a [`PropSignature`], **every** applied and
//! textual surface — the normal-form engine, [`Presentation`], `eq_mod`, the S2
//! [parser](mod@crate::text::parse)/[printer](mod@crate::text::print), and the S3
//! [`eval`](mod@crate::eval) — works over `PropExpr<FrobeniusOr<G>>` unchanged, with
//! no new interpreter or new tree type. The Frobenius structure is data in the
//! signature, not a fork of the engine.
//!
//! # Anchors (Fong & Spivak 2019, *Hypergraph Categories*, arXiv:1806.08304v3)
//!
//! - **Def 2.5** — a special commutative Frobenius monoid `(X, μ, η, δ, ε)` and
//!   its *nine* defining equations (see [`scfm_equations`]). The Hadamard SCFM
//!   on `R^dim` (an extension of Ex 2.16, realized by
//!   [`MatKron`](catgraph_applied::mat_kron)) satisfies them — the S4 milestone
//!   law.
//! - **Def 2.12** — a hypergraph category is a symmetric monoidal category "in
//!   which each object `X` is equipped with a Frobenius structure
//!   `(μ_X, η_X, δ_X, ε_X)`", compatibly with `⊗`. *Each object* — hence one
//!   spider family per colour, which is why the four spider variants carry a
//!   `Λ` payload.
//! - **Lemma 3.10 / Ex 2.9 / Ex 3.12** — on an objectwise-free category
//!   (Def 3.9, `List(Λ) ≅ Ob(C)`), "assigning a Frobenius structure to each
//!   object `[l]`, for each `l ∈ Λ`, induces a unique hypergraph structure";
//!   Ex 2.9 spells out that each object of `Cospan_Λ` then carries "`m` parallel
//!   copies of the Frobenius structure on `1 ∈ Cospan`", and Ex 3.12 makes
//!   `Cospan_Λ` a hypergraph category by giving every `l ∈ Λ` the Ex 2.8
//!   structure. This is the anchor for [`hypergraph_presentation`] seeding the
//!   nine equations **per colour**: the palette's structures generate the rest.
//! - **Prop 3.8** — `Cospan` is the theory of SCFMs: an SCFM in a symmetric
//!   monoidal category `C` is exactly a strict SM functor `(Cospan, ⊕) → (C, ⊗)`.
//!   This is the soundness anchor for [`to_mat_kron`], applied *objectwise*: the
//!   Hadamard SCFM on `R^dims(c)` is such a functor for each colour `c`, and
//!   Lemma 3.10 assembles the family into one hypergraph structure.
//! - **Thm 3.14** — `Cospan_Λ` is the free hypergraph category on the set `Λ`
//!   (an adjunction `Cospan_ ⊣ Ob : Hyp → Set`). A wire of colour `c` maps to
//!   the object `R^dims(c)`.
//! - **Ex 2.16 (extended)** — the paper's example says FdVect-with-chosen-basis
//!   is a hypergraph category \[Kis15\]; the crate extends it from a field to an
//!   arbitrary rig `R` as [`MatKron(R)`](catgraph_applied::mat_kron), the
//!   Hadamard-SCFM Kronecker hypergraph category and semantic target of
//!   [`to_mat_kron`]. The rig-generality is a crate extension, not Ex 2.16's
//!   stated scope.
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
//! nine **per colour** and cites Ex 2.8's count.
//!
//! # The [#15](https://github.com/sustia-llc/catgraph/issues/15) boundary, restated
//!
//! [`hypergraph_presentation`] seeds a [`Presentation`] with the nine equations
//! `E_frob` at each palette colour. Deciding equality over it goes through
//! applied's
//! [`eq_mod`](catgraph_applied::prop::presentation::Presentation::eq_mod), which
//! is **sound but syntactically incomplete** (#15): a `Some(true)` is a proof,
//! but a `None`/`Some(false)` is *not* a disproof — only that the congruence
//! closure did not establish the equation. `E_frob` overlaps heavily (spiders
//! fuse in many ways), so `None` is the expected answer for many true equalities.
//! [`to_mat_kron`] is a **sound** semantic check (Prop 3.8), **not** registered
//! as a
//! [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor):
//! equal `MatKron` images witness equality *in `MatKron(R)`*; they do not
//! promote an incomplete `eq_mod None` into a syntactic decision. For a
//! **complete** decision over the User-free fragment, use
//! [`CospanFunctor`](crate::cospan_functor::CospanFunctor) (F&S Prop 3.8 /
//! Thm 3.14, #80).
//!
//! # Colour scope: Λ-colored end to end
//!
//! Since [#79](https://github.com/sustia-llc/catgraph/issues/79) the layer is
//! colored at every level. P3a coloured the calculus —
//! `Mu(c)`/`Eta(c)`/`Delta(c)`/`Epsilon(c)` carry their colour,
//! [`FrobeniusOr<G>`]'s own `Color` is `G`'s, and both interpreters
//! ([`to_mat_kron`], [`to_cospan`](crate::cospan_functor::to_cospan)) thread an
//! interface word top-down. P3b coloured the *text*: [`GeneratorSyntax`] for
//! `FrobeniusOr<G>` holds for any [`ColorSyntax`] palette, spiders print and
//! parse as `mu@A` / `eta@A` / `delta@A` / `epsilon@A`, and presentation files
//! carry generator declarations (`g : A B -> C`). A monochromatic signature is
//! the palette whose one letter is implicit, so its files are byte-for-byte the
//! pre-#79 ones.

use std::borrow::Cow;
use std::cmp::Ordering;

use catgraph::category::Composable;
use catgraph::errors::CatgraphError;

use catgraph_applied::mat_kron::MatKron;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_applied::rig::Rig;

use crate::errors::SyntaxError;
use crate::text::{AT, ColorSyntax, GeneratorSyntax};

/// Reserved token for the multiplication spider `μ`.
const KW_MU: &str = "mu";
/// Reserved token for the unit spider `η`.
const KW_ETA: &str = "eta";
/// Reserved token for the comultiplication spider `δ`.
const KW_DELTA: &str = "delta";
/// Reserved token for the counit spider `ε`.
const KW_EPSILON: &str = "epsilon";

/// A user signature `G` extended with the four special-commutative-Frobenius
/// generators **at each colour** — the generators of the free hypergraph
/// category on `Λ` laid *over* `G` (F&S 2019 Def 2.5 / Def 2.12 / Lemma 3.10).
///
/// This is a **sum type over the user signature**, not a second AST: it is a
/// [`PropSignature`], so `PropExpr<FrobeniusOr<G>>` is an ordinary free-prop term
/// and reuses every existing surface (NF, presentation, `eq_mod`, parser,
/// printer, `eval`) with no engine change. Interface *words* follow Def 2.5 at
/// the carried colour `c`:
///
/// | Variant | Diagram | Word |
/// |---|---|---|
/// | [`Mu(c)`](FrobeniusOr::Mu) | `μ_c` multiplication | `[c, c] → [c]` |
/// | [`Eta(c)`](FrobeniusOr::Eta) | `η_c` unit | `[] → [c]` |
/// | [`Delta(c)`](FrobeniusOr::Delta) | `δ_c` comultiplication | `[c] → [c, c]` |
/// | [`Epsilon(c)`](FrobeniusOr::Epsilon) | `ε_c` counit | `[c] → []` |
/// | [`User(g)`](FrobeniusOr::User) | a `G`-generator | `g.source_word() → g.target_word()` |
///
/// Choosing `G::Color = ()` collapses the palette to the monochromatic
/// `Λ = {•}`, recovering the single-sorted spider family: `Mu(())` *is* the
/// pre-P3a `Mu`.
///
/// # Bounds
///
/// The enum carries `G: PropSignature` so it can name `G::Color`. Every derive
/// is satisfied by that bound alone **except** the total order:
/// [`PropSignature::Color`] requires only `Clone + Eq + Hash + Debug`, while
/// `PropSignature` itself requires `Ord`. `Ord`/`PartialOrd` are therefore
/// hand-written under `where G::Color: Ord` (variant order
/// `Mu < Eta < Delta < Epsilon < User`, then payload order — the pre-P3a derived
/// order, extended through the payload), and the [`PropSignature`] impl carries
/// the same bound. `()` satisfies it, so no shipped signature loses an impl.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "G: serde::Serialize, G::Color: serde::Serialize",
        deserialize = "G: serde::Deserialize<'de>, G::Color: serde::Deserialize<'de>"
    ))
)]
pub enum FrobeniusOr<G: PropSignature> {
    /// Multiplication `μ_c : [c, c] → [c]` (the merge).
    Mu(G::Color),
    /// Unit `η_c : [] → [c]` (the merge's identity).
    Eta(G::Color),
    /// Comultiplication `δ_c : [c] → [c, c]` (the split).
    Delta(G::Color),
    /// Counit `ε_c : [c] → []` (the split's identity).
    Epsilon(G::Color),
    /// A generator of the underlying user signature `G`.
    User(G),
}

impl<G: PropSignature> FrobeniusOr<G> {
    /// Variant rank for the total order, `Mu < Eta < Delta < Epsilon < User` —
    /// the order a `#[derive(Ord)]` would give, kept explicit because the
    /// payloads force a hand-written impl.
    fn rank(&self) -> u8 {
        match self {
            FrobeniusOr::Mu(_) => 0,
            FrobeniusOr::Eta(_) => 1,
            FrobeniusOr::Delta(_) => 2,
            FrobeniusOr::Epsilon(_) => 3,
            FrobeniusOr::User(_) => 4,
        }
    }
}

impl<G: PropSignature> Ord for FrobeniusOr<G>
where
    G::Color: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (FrobeniusOr::Mu(a), FrobeniusOr::Mu(b))
            | (FrobeniusOr::Eta(a), FrobeniusOr::Eta(b))
            | (FrobeniusOr::Delta(a), FrobeniusOr::Delta(b))
            | (FrobeniusOr::Epsilon(a), FrobeniusOr::Epsilon(b)) => a.cmp(b),
            (FrobeniusOr::User(a), FrobeniusOr::User(b)) => a.cmp(b),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

impl<G: PropSignature> PartialOrd for FrobeniusOr<G>
where
    G::Color: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// An equation `(lhs, rhs)` over the Frobenius signature, parallel over a common
/// source word — the shape of one `E_frob` entry, of each [`scfm_equations`]
/// element, and of a user equation once lifted into `FrobeniusOr<G>`.
pub type FrobeniusEquation<G> = (PropExpr<FrobeniusOr<G>>, PropExpr<FrobeniusOr<G>>);

impl<G: PropSignature> PropSignature for FrobeniusOr<G>
where
    G::Color: Ord,
{
    /// Colour-transparent: the spiders live at `G`'s own palette.
    type Color = G::Color;

    /// The user generator's own word for [`User`](FrobeniusOr::User); the Def 2.5
    /// word at the carried colour for the four spiders.
    fn source_word(&self) -> Cow<'_, [G::Color]> {
        match self {
            FrobeniusOr::Mu(c) => Cow::Owned(vec![c.clone(), c.clone()]),
            FrobeniusOr::Eta(_) => Cow::Borrowed(&[]),
            FrobeniusOr::Delta(c) | FrobeniusOr::Epsilon(c) => {
                Cow::Borrowed(std::slice::from_ref(c))
            }
            FrobeniusOr::User(g) => g.source_word(),
        }
    }

    fn target_word(&self) -> Cow<'_, [G::Color]> {
        match self {
            FrobeniusOr::Mu(c) | FrobeniusOr::Eta(c) => Cow::Borrowed(std::slice::from_ref(c)),
            FrobeniusOr::Delta(c) => Cow::Owned(vec![c.clone(), c.clone()]),
            FrobeniusOr::Epsilon(_) => Cow::Borrowed(&[]),
            FrobeniusOr::User(g) => g.target_word(),
        }
    }

    fn source(&self) -> usize {
        match self {
            FrobeniusOr::Mu(_) => 2,
            FrobeniusOr::Eta(_) => 0,
            FrobeniusOr::Delta(_) | FrobeniusOr::Epsilon(_) => 1,
            FrobeniusOr::User(g) => g.source(),
        }
    }

    fn target(&self) -> usize {
        match self {
            FrobeniusOr::Mu(_) | FrobeniusOr::Eta(_) => 1,
            FrobeniusOr::Delta(_) => 2,
            FrobeniusOr::Epsilon(_) => 0,
            FrobeniusOr::User(g) => g.target(),
        }
    }
}

// ---- Frobenius generator leaves (private smart-constructor shims) -------------
//
// One definition of each Frobenius leaf; every builder below reads from these so
// the variant-to-`Free::generator` mapping lives in exactly one place.

/// `μ_c : [c, c] → [c]` as a term.
fn mu<G>(color: G::Color) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::generator(FrobeniusOr::Mu(color))
}

/// `η_c : [] → [c]` as a term.
fn eta<G>(color: G::Color) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::generator(FrobeniusOr::Eta(color))
}

/// `δ_c : [c] → [c, c]` as a term.
fn delta<G>(color: G::Color) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::generator(FrobeniusOr::Delta(color))
}

/// `ε_c : [c] → []` as a term.
fn epsilon<G>(color: G::Color) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::generator(FrobeniusOr::Epsilon(color))
}

/// `id_n : n → n` over [`FrobeniusOr<G>`] — colour-polymorphic, it spans `n`
/// wires of whatever colours flow in.
fn id<G>(n: usize) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::identity(n)
}

/// Lift a user term into the Frobenius prop, wrapping every generator as
/// [`User`](FrobeniusOr::User) and mapping every other node identically. Takes
/// `expr` **by value** — the generator payloads are *moved* into `User(g)`, not
/// cloned (the sole caller, [`hypergraph_presentation`], owns and drops the
/// originals).
///
/// The map is **structural and arity-preserving**: `Generator(g) ↦
/// Generator(User(g))`, `Identity(n) ↦ Identity(n)`, `Braid(m, n) ↦ Braid(m,
/// n)`, and `Compose`/`Tensor` recurse. This is the inclusion `Free(G) ↪
/// Free(FrobeniusOr<G>)` of the user prop into the hypergraph prop, and it is
/// colour-preserving: `User(g)` reports `g`'s own words.
///
/// # Errors
///
/// Returns [`SyntaxError::Catgraph`] if an interior `Compose` node's interfaces
/// do not meet. A term built through the [`Free`] smart constructors is
/// arity-sound and cannot trigger this — but a `PropExpr` assembled by raw
/// variant construction (documented-legal in applied) may be ill-formed, and a
/// `Result`-returning API must surface that transparently rather than panic.
pub fn lift_user<G>(expr: PropExpr<G>) -> Result<PropExpr<FrobeniusOr<G>>, SyntaxError>
where
    G: PropSignature,
    G::Color: Ord,
{
    Ok(match expr {
        PropExpr::Identity(n) => Free::identity(n),
        PropExpr::Braid(m, n) => Free::braid(m, n),
        PropExpr::Generator(g) => Free::generator(FrobeniusOr::User(g)),
        PropExpr::Compose(f, g) => Free::compose(lift_user(*f)?, lift_user(*g)?)?,
        PropExpr::Tensor(f, g) => Free::tensor(lift_user(*f)?, lift_user(*g)?),
    })
}

/// μ-comb at colour `c` collapsing `m` legs to a single wire (`c^m → [c]`): the
/// left-nested fold `(((id ⊗ id) ; μ) ⊗ id) ; μ …`. Base cases `η_c` (`m = 0`)
/// and `id(1)` (`m = 1`). Built **iteratively** (a loop, not recursion) so a wide
/// `m` cannot overflow the stack during construction — only the O(m)-deep
/// *result* term is recursed over downstream.
fn collapse<G>(color: &G::Color, m: usize) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    if m == 0 {
        return eta::<G>(color.clone());
    }
    let mut acc = id::<G>(1);
    for _ in 1..m {
        acc = Free::compose(Free::tensor(acc, id::<G>(1)), mu::<G>(color.clone()))
            .expect("invariant: acc:k→1 tensored with id(1) is (k+1)→2, then ;μ:2→1 gives (k+1)→1");
    }
    acc
}

/// δ-comb at colour `c` expanding a single wire to `n` legs (`[c] → c^n`): the
/// right-nested fold `δ ; ((δ ; (… ⊗ id)) ⊗ id)`. Base cases `ε_c` (`n = 0`) and
/// `id(1)` (`n = 1`). Built **iteratively** (a loop, not recursion) so a wide `n`
/// cannot overflow the stack during construction.
fn expand<G>(color: &G::Color, n: usize) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    if n == 0 {
        return epsilon::<G>(color.clone());
    }
    let mut acc = id::<G>(1);
    for _ in 1..n {
        acc = Free::compose(delta::<G>(color.clone()), Free::tensor(acc, id::<G>(1)))
            .expect("invariant: δ:1→2 then (acc:1→k tensored with id(1)):2→(k+1) gives 1→(k+1)");
    }
    acc
}

/// The `(m, n)` **spider** at colour `c`: collapse `m` legs to one wire via a
/// μ-comb, then expand that wire to `n` legs via a δ-comb (source `c^m`, target
/// `c^n`).
///
/// Construction: `spider(c, m, n) = collapse(c, m) ; expand(c, n)`, where the two
/// combs meet at the single collapsed middle wire — the composition therefore
/// always typechecks (the `.expect` states that invariant). The edge cases:
///
/// - `spider(c, 0, n)` starts from `η_c` (`collapse(c, 0) = η_c`);
/// - `spider(c, m, 0)` ends at `ε_c` (`expand(c, 0) = ε_c`);
/// - `spider(c, 0, 0) = η_c ; ε_c` (the empty spider — the scalar `η ; ε`);
/// - `spider(c, m, 1)` returns `collapse(c, m)` directly and `spider(c, 1, n)`
///   returns `expand(c, n)` directly — the other comb would be `id(1)`, and
///   composing with it only adds a dead identity leg (and, under
///   [`to_mat_kron`], a wasted identity matmul). Together these subsume the
///   identity spider: `spider(c, 1, 1) = expand(c, 1) = id(1)`, the canonical
///   form for the identity. (`id(1)` is colour-polymorphic, so the `1 → 1`
///   spider carries no colour in the term — it takes whichever colour flows in.)
///
/// Spiders **fuse**: for a *connecting wire count* `k ≥ 1`, `spider(c, m, k) ;
/// spider(c, k, n)` and `spider(c, m, n)` denote the same morphism (semantically
/// checkable via [`to_mat_kron`]) — the spider-fusion property (F&S 2018
/// Def 6.54 / Thm 6.55; the "spider" vocabulary is Seven Sketches' — F&S 2019
/// states the underlying SCFM in §2.2 without it). Fusion is *within* a colour:
/// different colours have disjoint spider families (Lemma 3.10). The condition
/// `k ≥ 1` is load-bearing: at `k = 0`
/// the composite is `spider(c, m, 0) ; spider(c, 0, n) = (… ; ε) ; (η ; …)`,
/// whose `ε ; η` scalar bridge collapses the connection (its `MatKron` image is
/// the all-ones outer product, not the identity), so it is **not**
/// `spider(c, m, n)`.
///
/// # Depth
///
/// The *result* term is O(m + n) deep; consumers that recurse over it
/// (`to_mat_kron`, [`eval`](mod@crate::eval), the [printer](mod@crate::text::print))
/// recurse once per level, so a pathologically wide spider can exhaust the stack
/// downstream — an engine-wide property of the term representation, matching the
/// note on the printer/interpreter. Construction itself is iterative and never
/// recurses.
#[must_use]
pub fn spider<G>(color: G::Color, m: usize, n: usize) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    if m == 1 {
        return expand::<G>(&color, n);
    }
    if n == 1 {
        return collapse::<G>(&color, m);
    }
    Free::compose(collapse::<G>(&color, m), expand::<G>(&color, n))
        .expect("invariant: collapse(m):m→1 meets expand(n):1→n at the single middle wire")
}

/// The compact-closed **cup** `[] → [c, c]` at colour `c`, built as `η_c ; δ_c` —
/// matching [`MatKron::cup`](catgraph_applied::mat_kron::MatKron::cup) at
/// `dims(c)` exactly under [`to_mat_kron`].
#[must_use]
pub fn cup<G>(color: G::Color) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::compose(eta::<G>(color.clone()), delta::<G>(color))
        .expect("invariant: η:0→1 ; δ:1→2 gives 0→2")
}

/// The compact-closed **cap** `[c, c] → []` at colour `c`, built as `μ_c ; ε_c` —
/// matching [`MatKron::cap`](catgraph_applied::mat_kron::MatKron::cap) at
/// `dims(c)` exactly under [`to_mat_kron`].
#[must_use]
pub fn cap<G>(color: G::Color) -> PropExpr<FrobeniusOr<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    Free::compose(mu::<G>(color.clone()), epsilon::<G>(color))
        .expect("invariant: μ:2→1 ; ε:1→0 gives 2→0")
}

/// The **nine** defining equations of the special commutative Frobenius monoid
/// **at colour `c`** (F&S 2019 Def 2.5), as parallel `(lhs, rhs)` term pairs
/// built through [`Free`]. This is `E_frob(c)`, the equation set
/// [`hypergraph_presentation`] seeds once per palette colour.
///
/// Ex 2.8 states the count outright — "the nine equations in Definition 2.5" —
/// and the page-10 string diagrams confirm each one (all read left-to-right,
/// `σ = braid(1, 1)`, every spider at `c`):
///
/// 1. **associativity** `(μ ⊗ id) ; μ = (id ⊗ μ) ; μ` (`c³ → c`)
/// 2. **unitality** (left) `(η ⊗ id) ; μ = id(1)` (`c → c`)
/// 3. **commutativity** `σ ; μ = μ` (`c² → c`)
/// 4. **coassociativity** `δ ; (δ ⊗ id) = δ ; (id ⊗ δ)` (`c → c³`)
/// 5. **counitality** (left) `δ ; (ε ⊗ id) = id(1)` (`c → c`)
/// 6. **cocommutativity** `δ ; σ = δ` (`c → c²`)
/// 7. **Frobenius-L** `(δ ⊗ id) ; (id ⊗ μ) = μ ; δ` (`c² → c²`)
/// 8. **Frobenius-R** `(id ⊗ δ) ; (μ ⊗ id) = μ ; δ` (`c² → c²`)
/// 9. **speciality** `δ ; μ = id(1)` (`c → c`)
///
/// Equations 7 and 8 are the two halves of the paper's three-term Frobenius
/// chain `(δ ⊗ id) ; (id ⊗ μ) = μ ; δ = (id ⊗ δ) ; (μ ⊗ id)`; the right
/// unitality/counitality (Def 2.5's diagrams show only the left) are derivable
/// via commutativity/cocommutativity and are not among the nine.
///
/// The identity and braid legs are colour-polymorphic ([`PropExpr::Identity`] /
/// [`PropExpr::Braid`] carry only a width), so each pair is well-formed exactly
/// at the source word its spiders force — `c` throughout.
#[must_use]
pub fn scfm_equations<G>(color: G::Color) -> Vec<FrobeniusEquation<G>>
where
    G: PropSignature,
    G::Color: Ord,
{
    // Local composition shim: every pair below is arity-matched by the arities
    // annotated in this function's rustdoc, so `Free::compose` cannot fail here.
    let c = |f: PropExpr<FrobeniusOr<G>>, g: PropExpr<FrobeniusOr<G>>| {
        Free::compose(f, g).expect("invariant: scfm_equations builds arity-matched compositions")
    };
    // Per-leaf shims so the colour is cloned at each use site, not threaded.
    let mu = || mu::<G>(color.clone());
    let eta = || eta::<G>(color.clone());
    let delta = || delta::<G>(color.clone());
    let epsilon = || epsilon::<G>(color.clone());
    let braid = Free::<FrobeniusOr<G>>::braid(1, 1);

    vec![
        // 1. associativity: (μ ⊗ id) ; μ = (id ⊗ μ) ; μ.
        (
            c(Free::tensor(mu(), id::<G>(1)), mu()),
            c(Free::tensor(id::<G>(1), mu()), mu()),
        ),
        // 2. unitality (left): (η ⊗ id) ; μ = id(1).
        (c(Free::tensor(eta(), id::<G>(1)), mu()), id::<G>(1)),
        // 3. commutativity: braid(1,1) ; μ = μ.
        (c(braid.clone(), mu()), mu()),
        // 4. coassociativity: δ ; (δ ⊗ id) = δ ; (id ⊗ δ).
        (
            c(delta(), Free::tensor(delta(), id::<G>(1))),
            c(delta(), Free::tensor(id::<G>(1), delta())),
        ),
        // 5. counitality (left): δ ; (ε ⊗ id) = id(1).
        (c(delta(), Free::tensor(epsilon(), id::<G>(1))), id::<G>(1)),
        // 6. cocommutativity: δ ; braid(1,1) = δ.
        (c(delta(), braid), delta()),
        // 7. Frobenius-L: (δ ⊗ id) ; (id ⊗ μ) = μ ; δ.
        (
            c(
                Free::tensor(delta(), id::<G>(1)),
                Free::tensor(id::<G>(1), mu()),
            ),
            c(mu(), delta()),
        ),
        // 8. Frobenius-R: (id ⊗ δ) ; (μ ⊗ id) = μ ; δ.
        (
            c(
                Free::tensor(id::<G>(1), delta()),
                Free::tensor(mu(), id::<G>(1)),
            ),
            c(mu(), delta()),
        ),
        // 9. speciality: δ ; μ = id(1).
        (c(delta(), mu()), id::<G>(1)),
    ]
}

/// Build a [`Presentation`] of the free hypergraph category on the palette
/// `colors` over `G`: lift each user equation via [`lift_user`], then add the
/// nine [`scfm_equations`] **at every colour** (`E_frob`).
///
/// Seeding per colour is F&S 2019 **Lemma 3.10**: on an objectwise-free category
/// it suffices to give a Frobenius structure to each `l ∈ Λ`, and that induces a
/// unique hypergraph structure on all of `List(Λ)` (Ex 2.9 describes the induced
/// structure on a word as parallel copies of its letters'). So `|colors| × 9`
/// equations present the whole colored theory; a repeated colour simply seeds its
/// nine equations twice (harmless, since the congruence closure is
/// order- and multiplicity-invariant, but wasted work — pass a palette, not a
/// word).
///
/// For the monochromatic case pass the one-element palette `[()]`.
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
/// [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor),
/// or its worded sibling
/// [`eq_mod_functorial_colored`](catgraph_applied::prop::presentation::Presentation::eq_mod_functorial_colored)):
/// `Mat(R)` via Thm 5.60 for signal-flow graphs, and — for the User-free
/// spider fragment of `E_frob` —
/// [`CospanFunctor`](crate::cospan_functor::CospanFunctor) (F&S Prop 3.8 /
/// Thm 3.14, #80).
/// [`to_mat_kron`] is a separate **sound semantic** check, not a syntactic decision.
///
/// # Equation order
///
/// User equations are added **before** the `E_frob` equations, and the colours
/// are seeded in iteration order. Under the default
/// [`NormalizeEngine::CongruenceClosure`](catgraph_applied::prop::presentation::NormalizeEngine)
/// engine, insertion order is immaterial (congruence closure is order-invariant).
/// Under [`NormalizeEngine::Structural`](catgraph_applied::prop::presentation::NormalizeEngine),
/// rewrites apply in insertion order, so the order **is** observable — a caller
/// that switches the engine via
/// [`set_engine`](catgraph_applied::prop::presentation::Presentation::set_engine)
/// should be aware user rewrites fire ahead of the SCFM ones.
///
/// # Errors
///
/// Returns [`SyntaxError::Catgraph`] if any lifted user equation is
/// non-parallel or ill-formed (surfaced transparently from [`lift_user`] or from
/// [`add_equation`](catgraph_applied::prop::presentation::Presentation::add_equation),
/// whose check is boundary-*word* equality since #79 P2).
/// The built-in equations are parallel by construction; an error here
/// can only originate in a caller-supplied user equation.
pub fn hypergraph_presentation<G>(
    colors: impl IntoIterator<Item = G::Color>,
    user_eqs: impl IntoIterator<Item = (PropExpr<G>, PropExpr<G>)>,
) -> Result<Presentation<FrobeniusOr<G>>, SyntaxError>
where
    G: PropSignature,
    G::Color: Ord,
{
    let mut presentation = Presentation::<FrobeniusOr<G>>::new();
    for (lhs, rhs) in user_eqs {
        presentation.add_equation(lift_user(lhs)?, lift_user(rhs)?)?;
    }
    for color in colors {
        for (lhs, rhs) in scfm_equations::<G>(color) {
            presentation.add_equation(lhs, rhs)?;
        }
    }
    Ok(presentation)
}

/// The **dimension of an interface word**: the product of its letters'
/// dimensions, or a [`SyntaxError::DimensionOverflow`] if that product exceeds
/// `usize`. The empty word maps to `1` (the monoidal unit `R⁰ = R`).
fn word_dim<C, D>(word: &[C], dims: &D) -> Result<usize, SyntaxError>
where
    D: Fn(&C) -> usize,
{
    let mut product: usize = 1;
    for letter in word {
        let factor = dims(letter);
        product = product
            .checked_mul(factor)
            .ok_or(SyntaxError::DimensionOverflow { product, factor })?;
    }
    Ok(product)
}

/// Guard that the **dense cell count** of a `source`-word → `target`-word
/// morphism fits `usize`, returning its `(rows, cols)`.
///
/// The dense matrix has `rows · cols` cells, where `rows` is the source word's
/// dimension and `cols` the target's; guarding the product proves `rows`, `cols`,
/// **and** their product all fit `usize`, so every downstream `MatKron`
/// constructor / `kron` / matmul allocates a `Vec` whose capacity cannot
/// overflow. This is a `usize`-overflow guard on the cell *count*, **not**
/// memory-exhaustion protection: a huge-but-representable matrix (e.g. `2^60`
/// cells) still allocates, and that is the caller's responsibility.
fn checked_cells<C, D>(source: &[C], target: &[C], dims: &D) -> Result<(usize, usize), SyntaxError>
where
    D: Fn(&C) -> usize,
{
    let rows = word_dim(source, dims)?;
    let cols = word_dim(target, dims)?;
    rows.checked_mul(cols)
        .ok_or(SyntaxError::DimensionOverflow {
            product: rows,
            factor: cols,
        })?;
    Ok((rows, cols))
}

/// A subterm was handed a word of the wrong length.
fn expect_len<C>(expected: usize, input: &[C]) -> Result<(), SyntaxError> {
    if input.len() == expected {
        Ok(())
    } else {
        Err(CatgraphError::CompositionSizeMismatch {
            expected,
            actual: input.len(),
        }
        .into())
    }
}

/// A tensor's left factor needs at least its own arity from the incoming word.
fn expect_at_least<C>(expected: usize, input: &[C]) -> Result<(), SyntaxError> {
    if input.len() >= expected {
        Ok(())
    } else {
        Err(CatgraphError::CompositionSizeMismatch {
            expected,
            actual: input.len(),
        }
        .into())
    }
}

/// Map a Frobenius term to its image in [`MatKron(R)`](catgraph_applied::mat_kron),
/// the Hadamard-SCFM Kronecker hypergraph category — the **sound** semantic
/// checker for the SCFM laws (F&S 2019 Prop 3.8 applied objectwise; the
/// rig-general target extends Ex 2.16's FdVect-with-basis).
///
/// `source_word` is the term's declared input word (the colored morphism is the
/// pair `(word, expr)`, exactly as in
/// [`ColoredExpr`](catgraph_applied::prop::colored::ColoredExpr)); `dims` assigns
/// each colour its object dimension. Colours flow **top-down** through the term
/// the way [`check`](catgraph_applied::prop::colored::check) threads them:
/// `Identity`/`Braid` carry whatever flows in, spiders demand their declared
/// colour.
///
/// A monochromatic caller passes `&mono_word(expr.source())` and a constant
/// `&|_| dim`.
///
/// # Mapping (a wire of colour `c` ↦ the object `R^dims(c)`; a *word* ↦ the product)
///
/// | Node | Image |
/// |---|---|
/// | `Identity(k)` | [`MatKron::identity`](catgraph_applied::mat_kron::MatKron::identity) at the incoming word's dimension |
/// | `Braid(m, n)` | [`MatKron::braiding(a, b)`](catgraph_applied::mat_kron::MatKron::braiding), `a`/`b` the two blocks' dimensions |
/// | `Mu(c)` | [`MatKron::mu(dims(c))`](catgraph_applied::mat_kron::MatKron::mu) |
/// | `Eta(c)` | [`MatKron::eta(dims(c))`](catgraph_applied::mat_kron::MatKron::eta) |
/// | `Delta(c)` | [`MatKron::delta(dims(c))`](catgraph_applied::mat_kron::MatKron::delta) |
/// | `Epsilon(c)` | [`MatKron::epsilon(dims(c))`](catgraph_applied::mat_kron::MatKron::epsilon) |
/// | `Compose(f, g)` | matmul `f ; g` |
/// | `Tensor(f, g)` | Kronecker `f ⊗ g` |
/// | `User(g)` | **out of domain** — [`SyntaxError::NonFrobenius`] |
///
/// # Soundness, not completeness
///
/// For each colour `c`, the Hadamard SCFM on `R^dims(c)` is a special commutative
/// Frobenius monoid, so by **Prop 3.8** it induces a strict SM functor from
/// `Cospan` into `MatKron(R)`; **Lemma 3.10** assembles that family into a single
/// hypergraph structure, and **Thm 3.14** (`Cospan_Λ` is the free hypergraph
/// category on `Λ`) makes the extension to colored terms unique. `to_mat_kron`
/// computes that functor. It therefore respects all nine [`scfm_equations`] at
/// every colour: for a proven-equal pair, the two images are equal
/// (the S4 milestone law, machine-checked). It is deliberately **not** registered
/// as a
/// [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor):
/// no completeness theorem is claimed for `E_frob` here, so equal images witness
/// equality *in `MatKron(R)`* and must not be read as a general syntactic
/// decision. The **complete** decision for the User-free fragment is
/// [`CospanFunctor`](crate::cospan_functor::CospanFunctor) (same Prop 3.8 /
/// Thm 3.14, but valued in the free `Cospan_Λ` rather than a single model
/// `MatKron(R)`, #80).
///
/// `User(_)` generators are outside this functor's domain (the SCFM structure
/// carries no image for an arbitrary signature generator), so they are rejected
/// rather than assigned a guessed matrix.
///
/// # `dims(c) ≥ 2`
///
/// At `dims(c) = 1` every object over `c` degenerates to the monoidal unit and
/// the check is vacuous at that colour; callers validating the laws must use
/// `dims(c) ≥ 2`.
///
/// # Errors
///
/// - [`SyntaxError::NonFrobenius`] if `expr` contains a `User(g)` generator.
/// - [`SyntaxError::DimensionOverflow`] if the **dense cell count** of any node —
///   including the block product in a `Braid`, `dims(c)²` in `Mu`/`Delta`, and
///   the `kron`/matmul results — overflows `usize`. The guard is threaded through
///   every node (leaf, `kron`, and `compose`), so the sound checker never wraps
///   into a wrong matrix or attempts an overflowing `Vec` allocation. It guards
///   the cell *count*, not memory pressure: a huge-but-representable matrix still
///   allocates.
/// - [`SyntaxError::Catgraph`] carrying
///   [`CompositionSizeMismatch`](catgraph::errors::CatgraphError::CompositionSizeMismatch)
///   if a subterm (including the whole term) is handed a word of the wrong
///   length, or
///   [`Composition`](catgraph::errors::CatgraphError::Composition) if the lengths
///   agree but a spider's declared colour does not match the incoming one.
/// - [`SyntaxError::RecursionLimit`] if the term is deeper than
///   [`MAX_TERM_DEPTH`](crate::depth::MAX_TERM_DEPTH).
pub fn to_mat_kron<G, R, D>(
    expr: &PropExpr<FrobeniusOr<G>>,
    source_word: &[G::Color],
    dims: &D,
) -> Result<MatKron<R>, SyntaxError>
where
    G: PropSignature,
    G::Color: Ord,
    R: Rig,
    D: Fn(&G::Color) -> usize,
{
    // Pre-flight the recursion depth so `to_mat_kron_inner` cannot overflow the
    // stack on an unbounded programmatically-built term (#99).
    crate::depth::guard_term_depth(expr)?;
    to_mat_kron_inner::<G, R, D>(expr, source_word, dims).map(|(matrix, _target)| matrix)
}

/// The [`MatKron`] constructor a spider variant maps to
/// ([`mu`](MatKron::mu) / [`eta`](MatKron::eta) / [`delta`](MatKron::delta) /
/// [`epsilon`](MatKron::epsilon)), taking the wire's object dimension.
type SpiderCtor<R> = fn(usize) -> MatKron<R>;

/// Recursion behind [`to_mat_kron`], returning `(matrix, target_word)`.
///
/// Threading the interface *words* is what makes the overflow guard **complete**
/// and the colour check possible: every node knows both its boundary words, so it
/// checks its own dense cell count via [`checked_cells`] *before* the constructor
/// runs — covering the product dimensions a naïve per-interface guard misses
/// (`braiding`'s `a·b`, `mu`/`delta`'s `dims(c)²`, `kron`'s and matmul's results)
/// — and matches each spider's declared colour against what actually arrives.
fn to_mat_kron_inner<G, R, D>(
    expr: &PropExpr<FrobeniusOr<G>>,
    input: &[G::Color],
    dims: &D,
) -> Result<(MatKron<R>, Vec<G::Color>), SyntaxError>
where
    G: PropSignature,
    G::Color: Ord,
    R: Rig,
    D: Fn(&G::Color) -> usize,
{
    match expr {
        PropExpr::Identity(k) => {
            expect_len(*k, input)?;
            let (rows, _cols) = checked_cells(input, input, dims)?;
            Ok((MatKron::identity(rows), input.to_vec()))
        }
        PropExpr::Braid(m, n) => {
            // σ_{m,n} block-swaps the incoming word's two halves; the matrix is
            // the perfect shuffle of the two blocks' dimensions. `m + n` is
            // checked (a directly-constructed `Braid(usize::MAX, 1)` would
            // otherwise wrap): saturating to `usize::MAX` cannot match any real
            // slice length, so the mismatch is reported rather than panicked on.
            expect_len(m.checked_add(*n).unwrap_or(usize::MAX), input)?;
            let mut target = Vec::with_capacity(input.len());
            target.extend_from_slice(&input[*m..]);
            target.extend_from_slice(&input[..*m]);
            checked_cells(input, &target, dims)?;
            let a = word_dim(&input[..*m], dims)?;
            let b = word_dim(&input[*m..], dims)?;
            Ok((MatKron::braiding(a, b), target))
        }
        PropExpr::Generator(g) => {
            // The spider's colour and its `MatKron` constructor, resolved in one
            // match so the `User` case is handled without a second, unreachable
            // arm — and so the matrix is not built before the guard runs.
            let (color, build): (&G::Color, SpiderCtor<R>) = match g {
                FrobeniusOr::Mu(c) => (c, MatKron::mu),
                FrobeniusOr::Eta(c) => (c, MatKron::eta),
                FrobeniusOr::Delta(c) => (c, MatKron::delta),
                FrobeniusOr::Epsilon(c) => (c, MatKron::epsilon),
                FrobeniusOr::User(u) => {
                    return Err(SyntaxError::NonFrobenius {
                        generator: format!("{u:?}"),
                    });
                }
            };
            let target = spider_target_word(g, input)?;
            checked_cells(input, &target, dims)?;
            Ok((build(dims(color)), target))
        }
        PropExpr::Compose(f, g) => {
            let (fm, mid) = to_mat_kron_inner::<G, R, D>(f, input, dims)?;
            let (gm, target) = to_mat_kron_inner::<G, R, D>(g, &mid, dims)?;
            // matmul allocates a rows(input) × cols(target) result — guard it
            // even though each factor's own subtree already fit.
            checked_cells(input, &target, dims)?;
            // A matmul interface mismatch surfaces transparently as Catgraph.
            Ok((fm.compose(&gm)?, target))
        }
        PropExpr::Tensor(f, g) => {
            let split = f.source();
            expect_at_least(split, input)?;
            let (left, right) = input.split_at(split);
            let (fm, mut target) = to_mat_kron_inner::<G, R, D>(f, left, dims)?;
            let (gm, right_target) = to_mat_kron_inner::<G, R, D>(g, right, dims)?;
            target.extend(right_target);
            checked_cells(input, &target, dims)?;
            Ok((fm.kron(&gm), target))
        }
    }
}

/// Check a spider's declared source word against the incoming one and return its
/// target word — the shared generator arm of both interpreters
/// ([`to_mat_kron`] and [`to_cospan`](crate::cospan_functor::to_cospan)), so the
/// two cannot drift on what a colour mismatch is.
///
/// `User` generators must be filtered out by the caller: they are outside both
/// functors' domains, and this would otherwise validate them as ordinary
/// generators.
///
/// # Errors
///
/// - [`CatgraphError::CompositionSizeMismatch`] if the incoming word has the
///   wrong length (mirroring
///   [`check`](catgraph_applied::prop::colored::check)'s convention).
/// - [`CatgraphError::Composition`] if the lengths agree but the colours do not —
///   the message names the spider and both words.
pub(crate) fn spider_target_word<G>(
    g: &FrobeniusOr<G>,
    input: &[G::Color],
) -> Result<Vec<G::Color>, CatgraphError>
where
    G: PropSignature,
    G::Color: Ord,
{
    let want = g.source_word();
    if input.len() != want.len() {
        return Err(CatgraphError::CompositionSizeMismatch {
            expected: want.len(),
            actual: input.len(),
        });
    }
    if input != &*want {
        return Err(CatgraphError::Composition {
            message: format!(
                "colored Frobenius: spider {g:?} declares source word {want:?} but received {input:?}"
            ),
        });
    }
    Ok(g.target_word().into_owned())
}

/// [`GeneratorSyntax`] for [`FrobeniusOr<G>`]: the four Frobenius generators
/// print/parse as the reserved tokens `mu` / `eta` / `delta` / `epsilon`,
/// **annotated with their colour** where the palette has one to show, and
/// [`User(g)`](FrobeniusOr::User) delegates to `g`'s own token.
///
/// # Colour annotation: `mu@A` ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3b)
///
/// A spider prints `mu@A` when its colour's
/// [`print_color`](ColorSyntax::print_color) is `Some("A")`, and bare `mu` when
/// the colour is the palette's implicit sort. Parsing mirrors that: `mu@A` reads
/// the annotation through [`parse_color`](ColorSyntax::parse_color), while a bare
/// `mu` takes the palette's [`implicit`](ColorSyntax::implicit) sort — and is a
/// **parse failure** under a palette that has none, since there would be no
/// colour to give the spider. The monochromatic palette `()` is all-implicit, so
/// a single-sorted signature prints and parses exactly the pre-#79 tokens.
///
/// The joiner `@` is reserved in the [`GeneratorSyntax`] clause-2 alphabet, so a
/// user token can never be spelled `mu@…` and the split on `@` cannot cut a
/// `G`-token in two.
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
impl<G> GeneratorSyntax for FrobeniusOr<G>
where
    G: GeneratorSyntax,
    G::Color: ColorSyntax + Ord,
{
    fn print_token(&self) -> String {
        match self {
            FrobeniusOr::Mu(c) => spider_token(KW_MU, c),
            FrobeniusOr::Eta(c) => spider_token(KW_ETA, c),
            FrobeniusOr::Delta(c) => spider_token(KW_DELTA, c),
            FrobeniusOr::Epsilon(c) => spider_token(KW_EPSILON, c),
            FrobeniusOr::User(g) => g.print_token(),
        }
    }

    fn parse_token(token: &str) -> Option<Self> {
        let (name, annotation) = match token.split_once(AT) {
            Some((name, color)) => (name, Some(color)),
            None => (token, None),
        };
        let spider: fn(G::Color) -> Self = match name {
            KW_MU => FrobeniusOr::Mu,
            KW_ETA => FrobeniusOr::Eta,
            KW_DELTA => FrobeniusOr::Delta,
            KW_EPSILON => FrobeniusOr::Epsilon,
            // Not a spider name. The *whole* token goes to `G` — annotation
            // included, so a clause-2-violating `@` in a user token cannot be
            // silently trimmed away here.
            _ => return G::parse_token(token).map(FrobeniusOr::User),
        };
        let color = match annotation {
            Some(tok) => G::Color::parse_color(tok)?,
            None => G::Color::implicit()?,
        };
        Some(spider(color))
    }
}

/// A spider's token: the bare reserved `name` at the implicit sort, `name@c`
/// once the colour has a token of its own.
fn spider_token<C: ColorSyntax>(name: &str, color: &C) -> String {
    match color.print_color() {
        Some(tok) => format!("{name}{AT}{tok}"),
        None => name.to_string(),
    }
}
