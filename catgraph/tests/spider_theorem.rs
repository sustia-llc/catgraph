//! Fong–Spivak 2018 **Thm 6.55** (the spider theorem), pinned at two very
//! different strengths — and this header says which is which, because the two
//! claims are not interchangeable.
//!
//! # 1. Five term-level tests: hand-built composite == the builder's own term
//!
//! [`spider_2_2_via_mu_delta`], [`spider_3_1_via_double_mu`],
//! [`spider_1_3_via_double_delta`], [`spider_0_0_via_eta_epsilon`] and
//! [`mu_is_the_spider_builder_s_own_term_for_2_1`] assert *structural* equality
//! (`FrobeniusMorphism` derives `Eq`) between a hand-built composite and
//! `special_frobenius_morphism(m, n, z)`, at `(2,2)`, `(3,1)`, `(1,3)`, `(0,0)`
//! and `(2,1)`.
//!
//! **They do not verify Thm 6.55.** `FrobeniusMorphism`'s `Eq` is syntactic up
//! to `two_layer_simplify`'s three rewrite rules (identity drop, σ;σ cancel,
//! `Spider`;`Spider` fusion — the η;ε cancellation went at #350) — it is *not*
//! the Frobenius quotient,
//! so it cannot express "equals the spider" for a diagram the builder did not
//! itself build. Measured on `d6c7bd5`: `(δ ⊗ id);(id ⊗ μ)`, `σ;μ;δ`,
//! `(μ ⊗ id);(δ ⊗ id);(id ⊗ μ)`, `(δ ⊗ id);(id ⊗ σ);(μ ⊗ id)`, `(η ⊗ id);μ`
//! and the left-comb `4 → 1` are all connected and all structurally **≠**
//! `special_frobenius_morphism` at their arities. Widening the corpus under a
//! term-level assertion is therefore not available.
//!
//! What the five *do* pin is narrower and worth stating exactly: each hand-built
//! recipe is the recipe `special_frobenius_morphism` itself follows at that
//! arity — read off `catgraph/src/frobenius/operations.rs` (the
//! `special_frobenius_morphism` body): `(2,2)` recurses to `sfm(2,1);sfm(1,2)`
//! = `μ;δ`; `(3,1)` takes the odd-`m` branch to `(sfm(2,1) ⊗ id);μ` =
//! `(μ ⊗ id);μ`; `(1,3)` is `sfm(3,1)` horizontally flipped, i.e. `δ;(δ ⊗ id)`;
//! `(0,0)` is `sfm(0,1);sfm(1,0)` = `η;ε`; `(2,1)` is the generator `μ`. So the
//! five assert that a composite spelled by hand matches the builder's own
//! shape — a *builder-shape* pin, not a statement about arbitrary connected
//! diagrams.
//!
//! Falsified as such (measured on `d6c7bd5`, reverted after): mirroring
//! `special_frobenius_morphism`'s odd-`m` branch to `id ⊗ sfm(m-1, 1)` reddens
//! [`spider_3_1_via_double_mu`] and [`spider_1_3_via_double_delta`] — and
//! **only** those two, since the mirrored builder is SCFM-equal to the real one
//! and so leaves §2 green. That is exactly the division of labour between the
//! two halves of this file, and the reason the five are kept rather than
//! replaced: they see builder *shape*, §2 sees builder *meaning*.
//!
//! # 2. The semantic pin: the theorem itself, over a generated corpus
//!
//! [`connected_diagrams_denote_the_spider_in_cospan`] states Thm 6.55 in the
//! place it is actually true: the image under `frobenius_to_cospan`,
//! canonicalised by `Cospan::canonical_form`. A connected `m → n` diagram must land on a
//! cospan with **one** apex vertex carrying every domain and codomain index —
//! and that is exactly the canonical form of the spider. The oracle is
//! independent of `special_frobenius_morphism`: `apex_len`, `scalar_count` and
//! the single [`ApexClass`]'s preimages are read off the cospan, not compared
//! against a builder. The builder is then compared *to it*, so
//! `special_frobenius_morphism` rides the same claim rather than defining it.
//!
//! **What that map is, cited correctly.** `frobenius_to_cospan` is *not* a
//! direction of F&S 2019 Prop 3.8: both of Prop 3.8's directions concern
//! functors *out of* `Cospan`, and this one goes into it. What Prop 3.8
//! licenses is the construction — `Cospan_Λ` carries an SCFM structure on each
//! object (Ex 2.8), and Prop 3.8 turns that structure into the interpreting
//! functor this map computes. The anchor belongs on the construction, not on
//! the map; `cospan_algebra`'s own rustdoc says so at length, and
//! `CospanToFrobeniusFunctor` — which *is* one of Prop 3.8's directions — is
//! the neighbouring row in `FS18-AUDIT.md`.
//!
//! That rustdoc *used to* record the map as neither sound nor complete against
//! SCFM-equality **on scalars**, both witnesses measured, and that
//! incomparability is why the exclusions below were drawn where they are:
//! `m == n == 0` and every component-closing recipe are exactly the
//! scalar-shaped inputs the oracle was known to disagree with the syntax on.
//! #350 closed that gap — `two_layer_simplify`'s rule 3 is gone, both witnesses
//! are now pinned the other way up, and the rustdoc says so. The exclusions
//! therefore outlived their cause; lifting them is possible and tracked as
//! #353, and is **not** done here (see "Excluded by design" below, which states
//! the same thing at the point the exclusions are listed). Every term this file
//! does range over is additionally asserted to carry no scalar class at all.
//!
//! It is the theorem *over the corpus it ranges over*, and the shape of that
//! corpus is stated rather than implied. The measure that matters is a
//! diagram's **interior waist**: the narrowest running codomain strictly
//! *between* two blocks. A one-wire internal boundary **splits** the diagram
//! into two strictly smaller connected pieces, `A : m → 1` and `B : 1 → n`, on
//! which "one apex vertex" follows by induction. A narrow *boundary* splits
//! nothing, which is why neither the domain nor the final codomain counts as a
//! cut: a `1 → 1` diagram that δ's out to four wires, braids and μ's back is a
//! genuine instance.
//!
//! ⚠ **Read [`Built::is_wide_waist`] before quoting that count** — it is
//! evidence, not proof, and the gap runs both ways: a diagram cut is an
//! antichain and need not fall on a recipe layer boundary (under-reports), and
//! at `m == n == 1` the factorisation phrase is satisfied vacuously by
//! `id ; D ; id` without splitting anything (over-reports).
//!
//! Measured over the 1280 connected terms: **1030** have an interior waist of
//! two or more — 15 of those at `1 → 1`, where the over-reporting reading
//! applies — 205 have an internal cut of one wire, and 45 have no internal cut
//! at all (fewer than two blocks — nothing to narrow). All four counts are
//! pinned by the census, so the accounting is complete rather than stated for
//! the favourable part. [`wide_waist_permutation_family`] is what supplies the bulk
//! of the first bucket — every wiring of a δ-fan into a μ-fan, all of them
//! wide-waisted by construction — and [`MIN_CONNECTED_WIDE_WAIST`] is the floor
//! that keeps them there.
//!
//! ## Connectivity is decided by the recipe, never read off the oracle
//!
//! [`Recipe`] carries a disjoint-set over the components of the *construction*,
//! updated block by block: μ (and a merging fold) unions the components of the
//! wires it consumes, δ propagates, η starts a fresh component, ε consumes a
//! wire without destroying its component, and **σ permutes wires and unions
//! nothing** — a braiding's cospan is a permutation on a two-vertex apex, not a
//! connecting node. Filtering a corpus by its own postcondition (`apex_len ==
//! 1`) would be precisely the vacuity this file exists to remove.
//!
//! The verdict is used in **both** directions, so it is a differential rather
//! than an unchecked filter:
//! [`connected_diagrams_denote_the_spider_in_cospan`] takes the recipe's
//! *connected* terms and demands one apex vertex;
//! [`disconnected_recipes_denote_more_than_one_apex_vertex`] takes the
//! *rejected* ones and demands `apex_len() > 1` — in fact exactly the recipe's
//! own component count. Every term that second test ranges over has at least
//! **two** components: the component-free recipes are excluded below, so
//! `apex_len() > 1` really is the contrapositive over the whole arm rather than
//! over most of it.
//!
//! ## Excluded by design
//!
//! Three shapes are excluded, the first two for one shared reason:
//!
//! - **`m == n == 0`.** A `0 → 0` diagram has no boundary at all, so its single
//!   component is closed (below).
//! - **Any recipe that closes a component** — an η whose whole descendance is
//!   counit-ed off, leaving a component that touches neither boundary.
//! - **Any recipe with *no* component at all** — a random walk that started at
//!   width 0 and never drew an η, so no block could ever apply. Such a recipe is
//!   the empty term, `[] → []`, depth 0; `components == 0` makes it neither
//!   connected nor closed, and `apex_len() == components` would hold on it for
//!   free (`0 == 0`) while saying nothing. It is counted in the census
//!   ([`MEASURED_EMPTY_RECIPES`]) and asserted to be exactly that shape, but no
//!   claim test ranges over it.
//!
//! The first two sat on the *special* vs *extra-special* line, which this file
//! deliberately stayed out of: `two_layer_simplify`'s rule 3 cancelled `η;ε`
//! outright (the extra-special axiom) while `Cospan` is the **special** theory,
//! in which a closed bubble is a genuine `0 → 0` scalar. Measured on `d6c7bd5`,
//! before the rule was deleted: `(η ⊗ id);(id ⊗ id);(ε ⊗ id)` normalised to a
//! depth-1 term with the bubble gone, whose canonical form is
//! `apex_len = 1, scalars = 0`.
//!
//! **That line was decided at #350** — rule 3 is gone and `FrobeniusMorphism`
//! is the special theory, so a closed component now keeps its scalar on both
//! sides. Lifting these two exclusions is therefore possible and is tracked as
//! a follow-up (#353); it is **not** done here, and the exclusions below stand
//! exactly as they were. Both tests additionally assert `scalar_count() == 0`
//! on every term they *do* range over, so a surviving scalar class reddens the
//! pin rather than being quietly absorbed.
//!
//! ## What the corpus is, and what it is not
//!
//! Every corpus term is built from the six Frobenius generators η, ε, μ, δ, σ
//! and `id` alone — **never** from `FrobeniusOperation::Spider`, so
//! `special_frobenius_morphism` never appears inside a term under test, only on
//! the right-hand side of the comparison. All terms are at `Lambda = char`,
//! `BlackBoxLabel = String`: **one instantiation**, not "every carrier". Depth
//! is counted as *recipe layers appended*, not as the simplified term's layer
//! count — `FrobeniusMorphism::layers` is `pub(crate)` and an integration test
//! cannot see it, so the reported depth is an upper bound on the term's own.
//! The corpus is finite and seeded; it is a wide sample, not a proof. Its
//! *arity* spread is the narrow part — 24 distinct `(m, n)` pairs, none beyond
//! 4 — while its structural spread is answered by the permutation sweep (§2
//! above). Nothing here is uniform over connected diagrams — it is a corpus,
//! and the census pins exactly which one.
//!
//! The `⚠️ PARTIAL` → closed status for Thm 6.55 in
//! `catgraph-applied/docs/FS18-AUDIT.md` rests on §2, not on §1.

use std::collections::{HashMap, HashSet};

use catgraph::{
    category::{ComposableMutating, HasIdentity},
    cospan_canon::{ApexClass, CospanCanon},
    frobenius::{
        FrobeniusMorphism, FrobeniusOperation, frobenius_to_cospan, special_frobenius_morphism,
    },
    hypergraph_category::HypergraphCategory,
    monoidal::Monoidal,
};
use rand::{RngExt, SeedableRng, rngs::StdRng};

/// Morphisms with char-labelled wires and String black-box labels.
type FM = FrobeniusMorphism<char, String>;

// ===========================================================================
// §1 — the five term-level, builder-shape tests
// ===========================================================================

/// `μ;δ` is the term `special_frobenius_morphism(2, 2, z)` itself builds.
///
/// The `(2, 2)` arm of the builder recurses to `sfm(2,1);sfm(1,2)`, i.e. `μ;δ`;
/// this asserts the hand-spelled composite is structurally that term, plus
/// domain/codomain agreement.
///
/// **Space:** one term, one arity, one label, at `char`/`String`. Structural
/// equality is *not* the Frobenius quotient, so this says nothing about other
/// `2 → 2` connected diagrams — see the module header, and
/// [`connected_diagrams_denote_the_spider_in_cospan`] for the theorem.
#[test]
fn spider_2_2_via_mu_delta() {
    let z = 'z';

    // Build μ;δ : [z,z] -> [z,z] via in-place composition.
    let mut mu_then_delta = FM::multiplication(z);
    let delta = FM::comultiplication(z);
    ComposableMutating::compose(&mut mu_then_delta, delta).unwrap();

    assert_eq!(mu_then_delta.domain(), vec![z, z]);
    assert_eq!(mu_then_delta.codomain(), vec![z, z]);

    let spider: FM = special_frobenius_morphism(2, 2, z);
    assert_eq!(spider.domain(), mu_then_delta.domain());
    assert_eq!(spider.codomain(), mu_then_delta.codomain());
    assert!(
        spider == mu_then_delta,
        "s_2_2: μ;δ is not the term special_frobenius_morphism(2, 2, z) builds"
    );
}

/// `(μ ⊗ id);μ` is the term `special_frobenius_morphism(3, 1, z)` itself builds.
///
/// The builder's odd-`m` branch at `(3, 1)` is `(sfm(2,1) ⊗ id);μ`, i.e.
/// `(μ ⊗ id);μ`.
///
/// **Space:** one term, one arity, one label, at `char`/`String`. A
/// builder-shape pin, not a Thm 6.55 verification — see the module header.
///
/// **Falsification (measured, reverted):** mirroring that branch to
/// `id ⊗ sfm(m-1, 1)` takes this test red while
/// [`connected_diagrams_denote_the_spider_in_cospan`] stays green — the two
/// shapes are SCFM-equal, so only a term-level pin can separate them.
#[test]
fn spider_3_1_via_double_mu() {
    let z = 'z';

    // (μ ⊗ id) ; μ — the outer μ merges the codomain of (μ ⊗ id), which is
    // [z, z], into a single wire.
    let mut mu_id = FM::multiplication(z);
    mu_id.monoidal(<FM as HasIdentity<Vec<char>>>::identity(&vec![z]));
    let mu_outer = FM::multiplication(z);
    ComposableMutating::compose(&mut mu_id, mu_outer).unwrap();

    assert_eq!(mu_id.domain(), vec![z, z, z]);
    assert_eq!(mu_id.codomain(), vec![z]);

    let spider: FM = special_frobenius_morphism(3, 1, z);
    assert_eq!(spider.domain(), mu_id.domain());
    assert_eq!(spider.codomain(), mu_id.codomain());
    assert!(
        spider == mu_id,
        "s_3_1: (μ ⊗ id);μ is not the term special_frobenius_morphism(3, 1, z) builds"
    );
}

/// `δ;(δ ⊗ id)` is the term `special_frobenius_morphism(1, 3, z)` itself builds.
///
/// The builder reaches `(1, 3)` through `m < n`: it builds `sfm(3, 1)` —
/// `(μ ⊗ id);μ` — and horizontally flips it, which reverses the layer order and
/// dualises each generator, giving `δ;(δ ⊗ id)`.
///
/// **Space:** one term, one arity, one label, at `char`/`String`. A
/// builder-shape pin, not a Thm 6.55 verification — see the module header.
///
/// **Falsification (measured, reverted):** it inherits `(3, 1)`'s flip, so
/// mirroring the odd-`m` branch to `id ⊗ sfm(m-1, 1)` reddens this test too —
/// again while the semantic pin stays green.
#[test]
fn spider_1_3_via_double_delta() {
    let z = 'z';

    // Start with δ : [z] -> [z,z], then (δ ⊗ id) gives [z,z] -> [z,z,z].
    // So the composite is δ ; (δ ⊗ id) : [z] -> [z,z,z].
    let mut delta_first = FM::comultiplication(z);
    let mut delta_id = FM::comultiplication(z);
    delta_id.monoidal(<FM as HasIdentity<Vec<char>>>::identity(&vec![z]));
    ComposableMutating::compose(&mut delta_first, delta_id).unwrap();

    assert_eq!(delta_first.domain(), vec![z]);
    assert_eq!(delta_first.codomain(), vec![z, z, z]);

    let spider: FM = special_frobenius_morphism(1, 3, z);
    assert_eq!(spider.domain(), delta_first.domain());
    assert_eq!(spider.codomain(), delta_first.codomain());
    assert!(
        spider == delta_first,
        "s_1_3: δ;(δ ⊗ id) is not the term special_frobenius_morphism(1, 3, z) builds"
    );
}

/// `η;ε` is the term `special_frobenius_morphism(0, 0, z)` itself builds.
///
/// The builder's `(0, 0)` arm is `sfm(0,1);sfm(1,0)` = `η;ε`.
///
/// **Space:** one term, one arity, one label, at `char`/`String`.
///
/// ⚠ This assertion was **vacuous until #350**: `two_layer_simplify` cancelled
/// `η;ε` (the extra-special axiom), so both sides were the empty term and the
/// comparison held for a reason that had nothing to do with the builder. With
/// rule 3 deleted both sides are two-layer terms and the equality is a real
/// shape comparison. `(0, 0)` remains excluded from
/// [`connected_diagrams_denote_the_spider_in_cospan`] — lifting that exclusion
/// is the #353 follow-up, not this file's business here; see the module header.
#[test]
fn spider_0_0_via_eta_epsilon() {
    let z = 'z';

    let mut eta = FM::unit(z);
    let eps = FM::counit(z);
    ComposableMutating::compose(&mut eta, eps).unwrap();

    assert!(eta.domain().is_empty());
    assert!(eta.codomain().is_empty());

    let spider: FM = special_frobenius_morphism(0, 0, z);
    assert_eq!(spider.domain(), eta.domain());
    assert_eq!(spider.codomain(), eta.codomain());
    assert!(
        spider == eta,
        "s_0_0: η;ε is not the term special_frobenius_morphism(0, 0, z) builds"
    );
}

/// `μ` alone is the term `special_frobenius_morphism(2, 1, z)` builds — the
/// builder's base case, so this is the shortest possible builder-shape pin.
///
/// **Space:** one generator, one arity, one label, at `char`/`String`. It says
/// nothing about reduction: nothing is reduced here.
#[test]
fn mu_is_the_spider_builder_s_own_term_for_2_1() {
    let z = 'z';

    let mu: FM = FM::multiplication(z);
    assert_eq!(mu.domain(), vec![z, z]);
    assert_eq!(mu.codomain(), vec![z]);

    let spider: FM = special_frobenius_morphism(2, 1, z);
    assert_eq!(spider.domain(), mu.domain());
    assert_eq!(spider.codomain(), mu.codomain());
    assert!(
        spider == mu,
        "s_2_1: μ is not the term special_frobenius_morphism(2, 1, z) builds"
    );
}

// ===========================================================================
// §2 — the generated corpus
// ===========================================================================

/// The two wire labels the corpus is built on. Two of them, so a braiding can
/// cross wires of *distinct* types and not only be a same-label crossing.
const LABELS: [char; 2] = ['a', 'b'];

/// How many pseudo-random terms [`corpus`] contributes.
const RANDOM_TERMS: usize = 400;

/// The exact size of [`corpus`]. Asserted, so the space cannot shrink silently.
/// 192 scripted connected + 16 scripted wide-waist + 1488 permutation-swept
/// wide-waist (`2 · 4!` at `m = 2` plus `2 · 6!` at `m = 3`) + 9 scripted
/// disconnected + [`RANDOM_TERMS`].
const CORPUS_SIZE: usize = 192 + 16 + 1488 + 9 + RANDOM_TERMS;

/// Floor on the number of **connected** terms
/// [`connected_diagrams_denote_the_spider_in_cospan`] ranges over.
/// Measured 1280. [`wide_waist_permutation_family`] alone contributes 992 of
/// them with no RNG involved, so this floor is safe from seed drift.
const MIN_CONNECTED: usize = 900;

/// Floor on connected terms carrying at least one σ block. Measured 1105.
///
/// The permutation family supplies 992 of those *structurally*: a term of that
/// family is connected only when its permutation is non-identity, and a
/// non-identity permutation has a non-empty transposition word. Under the
/// identity the fold begins by merging each δ's two copies into their own
/// component, which is enough to leave at least one input unmerged — measured
/// disconnected in all four swept `(m, n)`. (It does *not* always end with `m`
/// components: at `(3, 2)` the fold is `Mu(0), Mu(1), Mu(0), Mu(1)` over
/// `[c0,c0,c1,c1,c2,c2]`, whose third μ joins `c0` to `c1`, ending with 2.)
const MIN_CONNECTED_WITH_BRAIDING: usize = 900;

/// Floor on connected terms whose **interior waist** — the narrowest running
/// codomain strictly between two applied blocks — is two wires or more.
/// Measured 1030.
///
/// This is the load-bearing count for the corpus's one structural bias. A
/// diagram with a one-wire internal boundary **splits** into two strictly
/// smaller connected pieces, on which the conclusion follows by induction; the
/// shapes that make the theorem say something are the ones that never narrow to
/// a single wire *inside*. A narrow boundary is not such a cut, and the metric
/// is evidence rather than proof in both directions — see
/// [`Built::is_wide_waist`], which states both gaps.
/// [`wide_waist_permutation_family`] supplies 992 of these by construction and
/// [`wide_waist_family`] 16 more, so the floor is structural rather than seeded.
const MIN_CONNECTED_WIDE_WAIST: usize = 900;

/// Floor on the deepest recipe (layers appended) among the connected terms.
/// The brief for this pin asks for depth ≥ 3; the permutation family reaches 21
/// (measured) and the floor records that rather than the minimum.
const MIN_CONNECTED_MAX_DEPTH: usize = 15;

/// Floor on the number of distinct `(m, n)` arities among the connected terms.
/// Measured 24 — exactly the scripted grid `0..=4 × 0..=4` minus `(0, 0)`. The
/// *floor* holds by construction (the scripted grid alone supplies 24); the
/// *equality* is *measured on this seed*, not structural. Nothing in
/// [`random_term`] bounds a connected walk's arity — the start width is drawn
/// from `0..4` and up to ten δ layers may be appended, so a connected `(1, 5)`
/// is reachable in principle. It simply did not occur at seed `0x6055_0001`, so
/// a seed or `rand` change may legitimately move the exact census number.
const MIN_CONNECTED_ARITIES: usize = 24;

/// Floor on the number of **disconnected** terms with at least two components
/// and no closed component — the arm
/// [`disconnected_recipes_denote_more_than_one_apex_vertex`] ranges over.
/// Measured 716, of which [`wide_waist_permutation_family`] contributes 496 with
/// no RNG involved. (The component-free empty recipes are *not* in this arm; see
/// the module header's *Excluded by design*.)
const MIN_DISCONNECTED: usize = 400;

/// Floor on distinct canonical forms in the disconnected arm. A generator that
/// degenerated to one repeated shape would still satisfy [`MIN_DISCONNECTED`];
/// this is what notices. Measured 203.
const MIN_DISCONNECTED_DISTINCT: usize = 150;

/// Floor on σ blocks that still lie **between two distinct components** when
/// their recipe ends. Measured 2169.
///
/// This is the load-bearing count for one of the two falsification
/// perturbations: making the braiding arm a merge instead of a permutation
/// changes nothing about a connected term's image (it is already one apex
/// vertex), so only a σ that spans two components can see it. Counted against
/// the *final* disjoint-set, so a σ whose sides a later μ merges is not counted
/// — it cannot see that perturbation either. See [`Recipe::braid_pairs`].
const MIN_CROSS_COMPONENT_BRAIDINGS: usize = 1200;

/// Floor on corpus terms excluded for closing a component, so the exclusion is
/// visibly non-empty rather than a clause about nothing. Measured 62.
const MIN_CLOSED_EXCLUDED: usize = 30;

/// One wire in a recipe's running codomain: its label and which construction
/// component it belongs to.
#[derive(Clone, Copy, Debug)]
struct Wire {
    label: char,
    comp: usize,
}

/// One block spliced into the running term as `id^i ⊗ g ⊗ id^rest`.
///
/// Deliberately only the six Frobenius generators (`Id` never appears alone —
/// an identity-only layer is a no-op and is skipped). `FrobeniusOperation::Spider`
/// is **not** here: the corpus must not contain the builder it is compared
/// against.
#[derive(Clone, Copy, Debug)]
enum Block {
    /// μ at wires `(i, i+1)` — the only *merging* block, so the only one that
    /// unions components.
    Mu(usize),
    /// δ at wire `i` — one component, two wires.
    Delta(usize),
    /// η inserted before wire `i`, starting a **fresh** component.
    Eta(usize, char),
    /// ε at wire `i` — consumes the wire; the component survives (it may still
    /// hold boundary elsewhere).
    Eps(usize),
    /// σ at wires `(i, i+1)` — permutes them and unions **nothing**.
    Braid(usize),
}

/// A term under construction, together with the disjoint-set over the
/// components of its *recipe*.
///
/// The union-find is the connectivity oracle, and it is deliberately built from
/// the construction and never from the term's image: see the module header.
struct Recipe {
    parent: Vec<usize>,
    /// Whether component `c` was seeded by a domain wire — such a component
    /// touches the boundary forever, even after its wires are counit-ed off.
    domain_seeded: Vec<bool>,
    wires: Vec<Wire>,
    domain: Vec<char>,
    term: FM,
    depth: usize,
    braidings: usize,
    /// The component pair each σ was laid across, as the components stood *at
    /// that moment*. Resolved against the **final** disjoint-set in
    /// [`Recipe::finish`], so a σ whose two sides are merged by a later μ is not
    /// counted as spanning two components — it does not, once the recipe ends.
    braid_pairs: Vec<(usize, usize)>,
    /// The narrowest **internal** cut the recipe reaches: the smallest running
    /// codomain strictly *between* two applied blocks.
    ///
    /// Neither the domain nor the final codomain is a cut of this kind — that is
    /// the whole point of the metric, since a diagram is not in the spider's own
    /// `s_{m,1} ; decoration ; s_{1,n}` factorisation merely because its own
    /// boundary happens to be one wire wide. `None` for a recipe with fewer than
    /// two applied blocks, which has no internal cut at all. A rejected block
    /// does not move it.
    interior_waist: Option<usize>,
    /// Running codomain after the most recently applied block. It becomes an
    /// *internal* cut — and is folded into [`Self::interior_waist`] — exactly
    /// when a further block follows it.
    last_width: Option<usize>,
}

impl Recipe {
    fn new(domain: &[char]) -> Self {
        let wires = domain
            .iter()
            .enumerate()
            .map(|(i, &label)| Wire { label, comp: i })
            .collect();
        Self {
            parent: (0..domain.len()).collect(),
            domain_seeded: vec![true; domain.len()],
            wires,
            domain: domain.to_vec(),
            term: <FM as HasIdentity<Vec<char>>>::identity(&domain.to_vec()),
            depth: 0,
            braidings: 0,
            braid_pairs: Vec::new(),
            interior_waist: None,
            last_width: None,
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) -> usize {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[rb] = ra;
        }
        ra
    }

    /// Start a component that no domain wire seeded — an η.
    fn fresh_component(&mut self) -> usize {
        let id = self.parent.len();
        self.parent.push(id);
        self.domain_seeded.push(false);
        id
    }

    /// Splice `id^at ⊗ generator ⊗ id^rest` onto the accumulator.
    ///
    /// Must be called *before* `self.wires` is updated: the layer is typed
    /// against the accumulator's current codomain.
    fn push_layer(&mut self, at: usize, consumed: usize, generator: FM) {
        let cod: Vec<char> = self.wires.iter().map(|w| w.label).collect();
        let mut layer: FM = <FM as HasIdentity<Vec<char>>>::identity(&cod[..at].to_vec());
        layer.monoidal(generator);
        layer.monoidal(<FM as HasIdentity<Vec<char>>>::identity(
            &cod[at + consumed..].to_vec(),
        ));
        ComposableMutating::compose(&mut self.term, layer)
            .expect("invariant: the layer was built on the accumulator's own codomain");
        self.depth += 1;
    }

    /// Apply one block, or report why it does not fit, keeping
    /// [`Self::interior_waist`] up to date on success.
    ///
    /// The width left behind by the *previous* block is folded in here rather
    /// than when that block ran, because that is the moment it becomes an
    /// internal cut: a width is only internal once another block follows it.
    fn apply(&mut self, block: Block) -> Result<(), String> {
        let outcome = self.apply_block(block);
        if outcome.is_ok() {
            if let Some(previous) = self.last_width {
                self.interior_waist =
                    Some(self.interior_waist.map_or(previous, |w| w.min(previous)));
            }
            self.last_width = Some(self.wires.len());
        }
        outcome
    }

    fn apply_block(&mut self, block: Block) -> Result<(), String> {
        match block {
            Block::Mu(i) => {
                if i + 1 >= self.wires.len() {
                    return Err(format!("μ at {i}: only {} wires", self.wires.len()));
                }
                let (a, b) = (self.wires[i], self.wires[i + 1]);
                if a.label != b.label {
                    return Err(format!("μ at {i}: {} vs {}", a.label, b.label));
                }
                self.push_layer(i, 2, FrobeniusOperation::Multiplication(a.label).into());
                let root = self.union(a.comp, b.comp);
                self.wires[i] = Wire {
                    label: a.label,
                    comp: root,
                };
                self.wires.remove(i + 1);
                Ok(())
            }
            Block::Delta(i) => {
                if i >= self.wires.len() {
                    return Err(format!("δ at {i}: only {} wires", self.wires.len()));
                }
                let w = self.wires[i];
                self.push_layer(i, 1, FrobeniusOperation::Comultiplication(w.label).into());
                self.wires.insert(i + 1, w);
                Ok(())
            }
            Block::Eta(i, z) => {
                if i > self.wires.len() {
                    return Err(format!("η at {i}: only {} wires", self.wires.len()));
                }
                self.push_layer(i, 0, FrobeniusOperation::Unit(z).into());
                let comp = self.fresh_component();
                self.wires.insert(i, Wire { label: z, comp });
                Ok(())
            }
            Block::Eps(i) => {
                if i >= self.wires.len() {
                    return Err(format!("ε at {i}: only {} wires", self.wires.len()));
                }
                let w = self.wires[i];
                self.push_layer(i, 1, FrobeniusOperation::Counit(w.label).into());
                self.wires.remove(i);
                Ok(())
            }
            Block::Braid(i) => {
                if i + 1 >= self.wires.len() {
                    return Err(format!("σ at {i}: only {} wires", self.wires.len()));
                }
                let (a, b) = (self.wires[i], self.wires[i + 1]);
                self.push_layer(
                    i,
                    2,
                    FrobeniusOperation::SymmetricBraiding(a.label, b.label).into(),
                );
                // σ: [z, w] → [w, z]. A permutation of wires — no union.
                self.wires[i] = b;
                self.wires[i + 1] = a;
                self.braidings += 1;
                self.braid_pairs.push((a.comp, b.comp));
                Ok(())
            }
        }
    }

    /// Close the recipe out into a [`Built`], computing the connectivity verdict
    /// and the closed-component verdict from the disjoint-set alone.
    fn finish(mut self, name: String) -> Built {
        let codomain: Vec<char> = self.wires.iter().map(|w| w.label).collect();
        let wire_comps: Vec<usize> = self.wires.iter().map(|w| w.comp).collect();

        // A component touches the boundary iff a domain wire seeded it or one of
        // its wires survived to the codomain.
        let mut boundary: HashMap<usize, bool> = HashMap::new();
        for c in 0..self.parent.len() {
            let seeded = self.domain_seeded[c];
            let root = self.find(c);
            let entry = boundary.entry(root).or_insert(false);
            *entry = *entry || seeded;
        }
        for c in wire_comps {
            let root = self.find(c);
            boundary.insert(root, true);
        }

        let components = boundary.len();
        let closed = boundary.values().any(|&touches| !touches);

        // Resolved against the FINAL disjoint-set, not against the one that
        // stood when the σ was laid: a braiding whose two sides a later μ merges
        // does not span two components in the finished recipe, and the
        // perturbation this count guards (a merging braiding arm) cannot see it
        // either. Measured on the 5457e2d corpus: 121 of the 132 counted at
        // braid time were still cross-component at the end, 11 were not.
        let braid_pairs = std::mem::take(&mut self.braid_pairs);
        let cross_component_braidings = braid_pairs
            .into_iter()
            .filter(|&(a, b)| self.find(a) != self.find(b))
            .count();

        Built {
            name,
            term: self.term,
            domain: self.domain,
            codomain,
            components,
            connected: components == 1,
            closed,
            depth: self.depth,
            braidings: self.braidings,
            cross_component_braidings,
            interior_waist: self.interior_waist,
        }
    }
}

/// A finished corpus entry: the term plus everything the *recipe* knows about
/// it. Nothing here is read off the term's cospan image.
struct Built {
    name: String,
    term: FM,
    domain: Vec<char>,
    codomain: Vec<char>,
    /// Number of connected components of the recipe (0-input blocks included).
    /// Zero only for the empty recipe — no domain wires and no η ever drawn, so
    /// no block could apply; excluded by design (see the module header).
    components: usize,
    connected: bool,
    /// Some component touches neither boundary — excluded by design.
    closed: bool,
    /// Recipe layers appended; an upper bound on the simplified term's depth.
    depth: usize,
    braidings: usize,
    /// σ blocks whose two sides are still distinct components when the recipe
    /// ends — see [`Recipe::braid_pairs`].
    cross_component_braidings: usize,
    /// The narrowest **internal** layer boundary the recipe reached, or `None`
    /// if it has fewer than two blocks and so has none. `Some(w)` with `w >= 2`
    /// is the file's evidence that a connected term is a non-trivial instance of
    /// Thm 6.55 — read [`Built::is_wide_waist`] for exactly how far that
    /// evidence goes, and [`MIN_CONNECTED_WIDE_WAIST`] for the floor.
    interior_waist: Option<usize>,
}

impl Built {
    /// Whether **no layer boundary of this recipe, other than its own domain and
    /// codomain, is one wire wide** (and at least one such boundary exists).
    ///
    /// That is the literal predicate. It is worth stating literally, because the
    /// property it is *evidence for* is a slightly different one and the gap
    /// runs in both directions.
    ///
    /// The property of interest is that the diagram does not **split**: a
    /// one-wire cut with blocks on both sides exhibits it as `A ; B` with `A` a
    /// connected `m → 1` and `B` a connected `1 → n`, both strictly smaller, on
    /// which "one apex vertex" follows by induction — the spider's own
    /// factorisation. Two caveats, neither of which the count above can see:
    ///
    /// - **It over-reports at the ends.** `s_{1,1}` is the identity, so *every*
    ///   `1 → 1` diagram trivially "factors" as `s_{1,1} ; D ; s_{1,1}`. That
    ///   factorisation splits nothing — `D` is not smaller than `D` — so it is
    ///   not the trivialising kind, and a `1 → 1` diagram that δ's out to four
    ///   wires, braids and μ's back is a genuine instance (its content is the
    ///   special axiom). But a reader checking "does it factor as
    ///   `s_{m,1} ; … ; s_{1,n}`?" at `m = n = 1` will find that it does, so the
    ///   phrase alone does not decide the question and the split is what does.
    ///   **15** corpus terms are counted wide on this reading — pinned as
    ///   [`MEASURED_CONNECTED_WIDE_TRIVIAL_ENDS`], and that total is the number
    ///   to quote; it was 8 if you count only [`connected_family`]'s scripted
    ///   `(1, 1)` terms and miss the random walks that also land there. The 992
    ///   of [`wide_waist_permutation_family`] all have `m, n >= 2` and do not
    ///   rest on it.
    /// - **It under-reports in general.** A cut of a string diagram is an
    ///   antichain, not necessarily a boundary between two recipe *layers*, so a
    ///   diagram can have a one-wire cut that no layer boundary of this
    ///   particular recipe realises. `interior_waist >= 2` therefore means "this
    ///   spelling never passes through one wire", which is evidence of
    ///   non-triviality and not a proof of it.
    fn is_wide_waist(&self) -> bool {
        self.interior_waist.is_some_and(|w| w >= 2)
    }
}

/// The scripted connected family: for every `(m, n)` in `0..=4 × 0..=4` except
/// `(0, 0)`, on each of the two labels, four decoration variants.
///
/// Shape: fold the `m` inputs to one wire with a left comb of μ (or seed one
/// with η when `m == 0`), decorate, then split to `n` with a comb of δ (or
/// close with ε when `n == 0`). Every variant stays connected by construction —
/// which the disjoint-set then *verifies* rather than assumes, since a scripting
/// mistake would show up as `components > 1` and land the term in the other arm.
///
/// **Note the structural restriction this shape imposes:** folding to one wire
/// puts a one-wire *internal* boundary in every term with `m >= 2`, splitting it
/// into `s_{m,1}` and the rest — the spider's own factorisation. (At `m <= 1`
/// there is no fold, so the restriction does not apply: the eight `m == 1`,
/// `n == 1` terms *of this family* count as wide-waisted, on the reading
/// [`Built::is_wide_waist`] flags as over-reporting at that arity. Corpus-wide
/// that count is 15, not 8 — [`MEASURED_CONNECTED_WIDE_TRIVIAL_ENDS`] — and 15
/// is the number to quote outside this docstring.) That is the
/// corpus's one systematic bias, and [`wide_waist_family`] and
/// [`wide_waist_permutation_family`] are what answer it.
fn connected_family() -> Vec<Built> {
    let mut out = Vec::new();
    for &z in &LABELS {
        for m in 0..=4usize {
            for n in 0..=4usize {
                if m == 0 && n == 0 {
                    continue;
                }
                for variant in 0..4usize {
                    out.push(scripted_connected(z, m, n, variant));
                }
            }
        }
    }
    out
}

fn scripted_connected(z: char, m: usize, n: usize, variant: usize) -> Built {
    let mut r = Recipe::new(&vec![z; m]);
    let step = |r: &mut Recipe, b: Block| {
        r.apply(b)
            .unwrap_or_else(|e| panic!("scripted_connected({z}, {m}, {n}, {variant}): {e}"));
    };

    if m == 0 {
        step(&mut r, Block::Eta(0, z));
    } else {
        for _ in 1..m {
            step(&mut r, Block::Mu(0));
        }
    }

    // One wire here, whatever `m` was. Decorate it.
    let decoration: Vec<Block> = match variant {
        // δ;μ — the special axiom's left-hand side, two layers deep.
        0 => vec![Block::Delta(0), Block::Mu(0)],
        // A fresh η absorbed by μ: a second recipe component that *is* merged.
        1 => vec![Block::Eta(0, z), Block::Mu(0)],
        // A braiding between two wires of the same component.
        2 => vec![Block::Delta(0), Block::Braid(0), Block::Mu(0)],
        // Deeper: split three ways, drop one, braid, merge.
        _ => vec![
            Block::Delta(0),
            Block::Delta(0),
            Block::Eps(0),
            Block::Braid(0),
            Block::Mu(0),
        ],
    };
    for b in decoration {
        step(&mut r, b);
    }

    if n == 0 {
        step(&mut r, Block::Eps(0));
    } else {
        for k in 0..n.saturating_sub(1) {
            step(&mut r, Block::Delta(k));
        }
    }

    r.finish(format!("connected_{z}_{m}_{n}_v{variant}"))
}

/// The scripted **wide-waist** connected family — the shapes that make Thm 6.55
/// non-trivial, put into the corpus deliberately rather than by luck.
///
/// [`connected_family`]'s recipes fold to one wire before decorating whenever
/// `m >= 2`, so they carry a one-wire *internal* boundary: they **split** into
/// two strictly smaller connected pieces, on which "one apex vertex" follows by
/// induction. Measured on `5457e2d`, before
/// this family existed, only 22 of the 272 connected terms had an interior
/// waist of two or more, and a further 45 had no internal cut at all — fewer
/// than two blocks, so nothing to narrow, and not an instance of the
/// distinction either way (see [`Built::is_wide_waist`]).
///
/// These sixteen never narrow below `w >= 2` wires. Three shapes, each on both
/// labels, generalising three of the composites the module header measures as
/// structurally **≠** their spider:
///
/// - **comb `w`** — for `i` in `0..w-1`: split wire `i`, merge the copy into
///   wire `i + 1`. At `w == 2` this is exactly `(δ ⊗ id);(id ⊗ μ)`. Arity
///   `(w, w)`; interior waist `w + 1` at `w == 2` and `w` above it.
/// - **braided comb `w`** — the same with a σ between the split and the merge.
///   At `w == 2`, exactly `(δ ⊗ id);(id ⊗ σ);(μ ⊗ id)`. Arity `(w, w)`; interior
///   waist as for the comb.
/// - **folded comb `w`** — one μ first, then comb `w`. At `w == 2`, exactly
///   `(μ ⊗ id);(δ ⊗ id);(id ⊗ μ)`. Arity `(w + 1, w)`, interior waist `w`.
///
/// The waist annotations are for the *interior* metric, which excludes the
/// boundary: at `w == 2` the comb's only internal boundary is the 3-wire one
/// between its δ and its μ, its final 2-wire codomain not being a cut. Under
/// the boundary-counting metric this family was written against they all read
/// `w` instead.
///
/// `w` runs `2..=4` for the first two and `2..=3` for the third, which keeps
/// every arity inside the scripted grid `0..=4 × 0..=4` (so this family does not
/// move [`MEASURED_CONNECTED_ARITIES`]). As with [`connected_family`], each is
/// connected *by construction* and the disjoint-set then verifies it: a
/// scripting mistake lands the term in the other arm and reddens the census.
///
/// **These sixteen are guarded by name, not by a count.** Measured: deleting
/// them all leaves every floor in this file green, because
/// [`wide_waist_permutation_family`] out-supplies any count worth flooring. The
/// three `_a_2` shapes are asserted present and wide in
/// [`the_corpus_is_the_space_these_pins_claim`].
fn wide_waist_family() -> Vec<Built> {
    let mut out = Vec::new();
    let step = |r: &mut Recipe, b: Block| {
        r.apply(b)
            .unwrap_or_else(|e| panic!("wide_waist_family: {e}"));
    };

    for &z in &LABELS {
        for w in 2..=4usize {
            // comb: (δ ⊗ id^{w-1}) ; (id ⊗ μ ⊗ id^{w-2}) ; …
            let mut r = Recipe::new(&vec![z; w]);
            for i in 0..w - 1 {
                step(&mut r, Block::Delta(i));
                step(&mut r, Block::Mu(i + 1));
            }
            out.push(r.finish(format!("wide_comb_{z}_{w}")));

            // braided comb: a σ between each split and its merge.
            let mut r = Recipe::new(&vec![z; w]);
            for i in 0..w - 1 {
                step(&mut r, Block::Delta(i));
                step(&mut r, Block::Braid(i + 1));
                step(&mut r, Block::Mu(i));
            }
            out.push(r.finish(format!("wide_braided_comb_{z}_{w}")));
        }

        for w in 2..=3usize {
            // folded comb: one μ first, so the arity is (w + 1, w).
            let mut r = Recipe::new(&vec![z; w + 1]);
            step(&mut r, Block::Mu(0));
            for i in 0..w - 1 {
                step(&mut r, Block::Delta(i));
                step(&mut r, Block::Mu(i + 1));
            }
            out.push(r.finish(format!("wide_folded_comb_{z}_{w}")));
        }
    }

    out
}

/// The **permutation-swept** wide-waist family: every wiring of a δ-fan into a
/// μ-fan, at `m` inputs and `n` outputs.
///
/// [`wide_waist_family`]'s sixteen shapes are hand-picked, so they answer the
/// waist bias with a fixed handful of wirings. This family answers it
/// exhaustively over a small regime instead: split each of the `m` inputs with
/// δ, apply **every** permutation of the resulting `2m` middle wires (realised
/// as a word of adjacent σ's, so the diagram is braid-rich by construction),
/// then fold back down to `n` with μ. The narrowest internal cut is
/// `min(m, n) + 1` — the run widens to `2m` and comes back down, and only the
/// domain and the final codomain sit at `m` and `n`, neither of which is an
/// internal cut — so every term here is wide-waisted whatever the permutation
/// does.
///
/// The permutation is what makes it worth sweeping rather than sampling: it
/// decides whether the two halves of each split end up in the same μ-group, so
/// the *same* recipe shape lands in both arms of the differential — connected
/// for some permutations, two-component for others — with the disjoint-set, not
/// the oracle, saying which. Measured on `d6c7bd5` over the 1488 terms at
/// `m ∈ {2, 3}`, `n ∈ {2, 3}`: 992 connected (all agreeing with the spider),
/// 496 disconnected (all with `apex_len` exactly their component count),
/// interior waists exactly `{3, 4}`, and **none** structurally equal to its
/// spider — the whole family is outside what §1's term-level `Eq` can reach. The
/// last two are not left as prose: [`the_corpus_is_the_space_these_pins_claim`]
/// asserts both, in an order that keeps neither able to pass vacuously.
///
/// The label alternates with `m` rather than the family being built twice, which
/// keeps both labels exercised without doubling a 1488-term sweep. Arities stay
/// inside the scripted grid `0..=4 × 0..=4`, so this family does not move
/// [`MEASURED_CONNECTED_ARITIES`] either.
fn wide_waist_permutation_family() -> Vec<Built> {
    let mut out = Vec::new();
    for (idx, m) in (2..=3usize).enumerate() {
        let z = LABELS[idx % LABELS.len()];
        let middle = 2 * m;
        for n in 2..=3usize {
            for (p, permutation) in permutations(middle).into_iter().enumerate() {
                let mut r = Recipe::new(&vec![z; m]);
                let step = |r: &mut Recipe, b: Block| {
                    r.apply(b)
                        .unwrap_or_else(|e| panic!("wide_waist_permutation_family: {e}"));
                };

                // Split: the δ for original wire `i` sits at position `2i`,
                // because the `i` splits before it have each widened the run by
                // one.
                for i in 0..m {
                    step(&mut r, Block::Delta(2 * i));
                }
                // Permute, as a word of adjacent transpositions.
                for at in transposition_word(&permutation) {
                    step(&mut r, Block::Braid(at));
                }
                // Fold `middle` wires down to `n`, cycling the merge position so
                // the μ's are not all stacked at wire 0.
                for k in 0..middle - n {
                    step(&mut r, Block::Mu(k % n));
                }

                out.push(r.finish(format!("wide_perm_{z}_{m}_{n}_p{p}")));
            }
        }
    }
    out
}

/// Every permutation of `0..n`, in a fixed order (the corpus must be
/// deterministic).
fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut current: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    fn walk(v: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
        if k == v.len() {
            out.push(v.clone());
            return;
        }
        for i in k..v.len() {
            v.swap(k, i);
            walk(v, k + 1, out);
            v.swap(k, i);
        }
    }
    walk(&mut current, 0, &mut out);
    out
}

/// A word of adjacent transpositions realising `target`, as σ positions to apply
/// in order: after the word, the wire that started at `target[i]` sits at `i`.
fn transposition_word(target: &[usize]) -> Vec<usize> {
    let mut current: Vec<usize> = (0..target.len()).collect();
    let mut word = Vec::new();
    for (position, &want) in target.iter().enumerate() {
        let mut at = current
            .iter()
            .position(|&x| x == want)
            .expect("invariant: `target` is a permutation of `0..target.len()`");
        while at > position {
            current.swap(at - 1, at);
            word.push(at - 1);
            at -= 1;
        }
    }
    word
}

/// The two helpers [`wide_waist_permutation_family`] rests on, checked against
/// their own claims rather than trusted.
///
/// Both are load-bearing in a way nothing else in the file would notice.
/// [`permutations`] returns `n!` entries *by construction* — a version that
/// emitted duplicates would leave [`CORPUS_SIZE`] at 2105 and silently shrink
/// the sweep to fewer distinct wirings; and a [`transposition_word`] that
/// realised some *other* permutation would still yield well-typed, connected,
/// wide-waist terms that pass every other assertion in this file. So the word
/// "every" in that family's docstring is checked here: `n!` outputs, all
/// distinct, and each word applied to `0..n` reproducing its target exactly.
#[test]
fn the_permutation_sweep_really_sweeps_every_permutation() {
    // Up to 6, because `wide_waist_permutation_family` calls `permutations(2m)`
    // at `m ∈ {2, 3}` — and the 6-element case supplies 1440 of the family's
    // 1488 terms. Stopping at 5 would leave 97% of the sweep guarded by a check
    // that never reaches its arity.
    for n in 0..=6usize {
        let factorial: usize = (1..=n).product();
        let all = permutations(n);
        assert_eq!(
            all.len(),
            factorial,
            "permutations({n}) produced {} entries, not {n}! = {factorial}",
            all.len(),
        );
        let distinct: HashSet<Vec<usize>> = all.iter().cloned().collect();
        assert_eq!(
            distinct.len(),
            factorial,
            "permutations({n}) produced {} distinct entries out of {}, so the sweep repeats a \
             wiring instead of covering one more",
            distinct.len(),
            all.len(),
        );

        for target in &all {
            // Driven through `Recipe`/`Block::Braid` rather than through a
            // hand-copied `swap`, so the check is a *link* to the DSL's WIRE
            // BOOKKEEPING and not a re-spelling of it: change which wires
            // `Block::Braid` exchanges and the sweep's wirings stop being the
            // permutations their names claim, and a duplicated convention here
            // would have stayed green through it. The wires carry distinct
            // components, so tracking `comp` tracks the permutation.
            //
            // It links to that half only. `realised` is read from
            // `recipe.wires`, never from `recipe.term`, so a `Block::Braid` that
            // kept the swap and emitted the wrong *generator* — an identity, say
            // — passes here. The emitted term is what the two claim tests
            // exercise; this test's subject is the permutation, not the layer.
            let mut recipe = Recipe::new(&vec![LABELS[0]; n]);
            for at in transposition_word(target) {
                recipe
                    .apply(Block::Braid(at))
                    .unwrap_or_else(|e| panic!("transposition_word({target:?}) at {at}: {e}"));
            }
            let realised: Vec<usize> = recipe.wires.iter().map(|w| w.comp).collect();
            assert_eq!(
                &realised, target,
                "transposition_word({target:?}) realises {realised:?} through Block::Braid — every \
                 swept diagram would still be well-typed and connected, so nothing else here \
                 would notice",
            );
        }
    }
}

/// The scripted disconnected family — the arm that makes the recipe's verdict a
/// two-way differential.
///
/// Every entry keeps each component attached to a boundary (no closed
/// component), and several are separated *only* by a σ, which is the shape that
/// notices a braiding arm turned into a merge.
fn disconnected_family() -> Vec<Built> {
    let mut out = Vec::new();
    let step = |r: &mut Recipe, b: Block| {
        r.apply(b).unwrap_or_else(|e| panic!("disconnected: {e}"));
    };

    // Two same-label components crossed by a σ. σ('a','a') is identity-shaped in
    // Cospan, so nothing but the component count separates this from a wire pair.
    {
        let mut r = Recipe::new(&['a', 'a', 'a', 'a']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Mu(1));
        step(&mut r, Block::Braid(0));
        out.push(r.finish("disc_same_label_sigma_between_components".into()));
    }
    // Two components of *different* labels crossed by a σ, then split.
    {
        let mut r = Recipe::new(&['a', 'a', 'b', 'b']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Mu(1));
        step(&mut r, Block::Braid(0));
        step(&mut r, Block::Delta(0));
        out.push(r.finish("disc_mixed_label_sigma_between_components".into()));
    }
    // Three components, two σ between them.
    {
        let mut r = Recipe::new(&['a', 'a', 'a', 'a', 'a', 'a']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Mu(1));
        step(&mut r, Block::Mu(2));
        step(&mut r, Block::Braid(0));
        step(&mut r, Block::Braid(1));
        step(&mut r, Block::Delta(0));
        out.push(r.finish("disc_three_components_two_sigmas".into()));
    }
    // A second component seeded by η and crossed by a σ — never merged.
    {
        let mut r = Recipe::new(&['a']);
        step(&mut r, Block::Eta(0, 'b'));
        step(&mut r, Block::Braid(0));
        step(&mut r, Block::Delta(1));
        out.push(r.finish("disc_eta_component_crossed_by_sigma".into()));
    }
    // No σ at all: two components side by side, each decorated.
    {
        let mut r = Recipe::new(&['a', 'a', 'b', 'b']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Mu(1));
        step(&mut r, Block::Delta(0));
        step(&mut r, Block::Delta(2));
        out.push(r.finish("disc_two_components_no_sigma".into()));
    }
    // A component whose wires are all counit-ed off but which was seeded at the
    // domain: still boundary-touching, so *not* closed, and still its own apex
    // vertex with an empty codomain preimage.
    {
        let mut r = Recipe::new(&['a', 'a', 'b']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Eps(1));
        out.push(r.finish("disc_domain_only_component".into()));
    }
    // Deeper: two components, each internally δ;μ, then a σ across them.
    {
        let mut r = Recipe::new(&['a', 'a', 'b', 'b']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Mu(1));
        step(&mut r, Block::Delta(0));
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Delta(1));
        step(&mut r, Block::Mu(1));
        step(&mut r, Block::Braid(0));
        out.push(r.finish("disc_decorated_components_then_sigma".into()));
    }
    // Four one-wire components: a bare identity on four wires, then σ chains.
    {
        let mut r = Recipe::new(&['a', 'b', 'a', 'b']);
        step(&mut r, Block::Braid(0));
        step(&mut r, Block::Braid(2));
        step(&mut r, Block::Braid(1));
        out.push(r.finish("disc_four_wires_sigma_chain".into()));
    }
    // A component with codomain but no domain (η-seeded) beside one with both.
    {
        let mut r = Recipe::new(&['a', 'a']);
        step(&mut r, Block::Mu(0));
        step(&mut r, Block::Eta(1, 'a'));
        step(&mut r, Block::Delta(1));
        step(&mut r, Block::Braid(0));
        out.push(r.finish("disc_eta_component_beside_domain_component".into()));
    }

    out
}

/// One random block, or `None` when the drawn kind has no legal position.
///
/// A `None` is a skipped step, which keeps the draw uniform over block *kinds*
/// rather than over (kind, position) pairs and costs only term length. The
/// corpus census floors are what keep a regression in these guards from
/// silently emptying the random half.
fn random_block(rng: &mut StdRng, wires: &[Wire]) -> Option<Block> {
    let n = wires.len();
    let equal_pairs: Vec<usize> = (0..n.saturating_sub(1))
        .filter(|&i| wires[i].label == wires[i + 1].label)
        .collect();

    match rng.random_range(0..6u8) {
        0 => Some(Block::Eta(
            rng.random_range(0..=n),
            LABELS[rng.random_range(0..LABELS.len())],
        )),
        1 if n >= 1 => Some(Block::Eps(rng.random_range(0..n))),
        // Weighted twice: μ is the only merging block, and without it the random
        // half would be almost entirely disconnected.
        2 | 3 if !equal_pairs.is_empty() => Some(Block::Mu(
            equal_pairs[rng.random_range(0..equal_pairs.len())],
        )),
        4 if n >= 1 => Some(Block::Delta(rng.random_range(0..n))),
        5 if n >= 2 => Some(Block::Braid(rng.random_range(0..n - 1))),
        _ => None,
    }
}

fn random_term(rng: &mut StdRng, index: usize, steps: usize) -> Built {
    let width = rng.random_range(0..4usize);
    let start: Vec<char> = (0..width)
        .map(|_| LABELS[rng.random_range(0..LABELS.len())])
        .collect();
    let mut r = Recipe::new(&start);
    for _ in 0..steps {
        let Some(block) = random_block(rng, &r.wires) else {
            continue;
        };
        // A drawn block can still be illegal (μ on unequal labels after an
        // intervening σ); a rejection is a skipped step, not a failure.
        let _ = r.apply(block);
    }
    r.finish(format!("random_{index}"))
}

/// The whole corpus: 192 scripted connected terms ([`connected_family`]), 16
/// scripted wide-waist ones ([`wide_waist_family`]), the 1488 of
/// [`wide_waist_permutation_family`] — 71% of the corpus, and the reason
/// [`CORPUS_SIZE`] is 2105 rather than 617 — 9 scripted disconnected terms
/// ([`disconnected_family`]), and [`RANDOM_TERMS`] pseudo-random ones seeded at
/// `0x6055_0001`.
///
/// The scripted families are appended *before* the seeded walks are drawn, so
/// adding one does not perturb the random stream.
fn corpus() -> Vec<Built> {
    let mut out = connected_family();
    out.extend(wide_waist_family());
    out.extend(wide_waist_permutation_family());
    out.extend(disconnected_family());

    let mut rng = StdRng::seed_from_u64(0x6055_0001);
    for k in 0..RANDOM_TERMS {
        let steps = 1 + (k % 10);
        out.push(random_term(&mut rng, k, steps));
    }
    out
}

/// A one-line digest of a canonical form, for failure messages.
fn digest(c: &CospanCanon<char>) -> String {
    format!(
        "{}→{} apex={} scalars={}",
        c.dom_len(),
        c.cod_len(),
        c.apex_len(),
        c.scalar_count()
    )
}

fn image(term: &FM, name: &str) -> CospanCanon<char> {
    frobenius_to_cospan(term)
        .unwrap_or_else(|e| {
            panic!("{name}: frobenius_to_cospan rejected a black-box-free term: {e}")
        })
        .canonical_form()
}

/// Every term the corpus's recipe calls connected — Thm 6.55, stated at the
/// semantics.
///
/// **Claim.** For a connected `m → n` Frobenius diagram on a single wire type
/// `z`, `frobenius_to_cospan(·).canonical_form()` has exactly one apex vertex,
/// labelled `z`, whose domain preimage is all of `0..m` and whose codomain
/// preimage is all of `0..n`, and no scalar classes — and that canonical form is
/// the one `special_frobenius_morphism(m, n, z)` lands on.
///
/// **Space the assertions actually touch.** The connected, non-closed terms of
/// [`corpus`]: the 192 scripted terms of [`connected_family`] (both labels ×
/// `(m, n)` in `0..=4 × 0..=4` minus `(0, 0)` × four decoration variants), the
/// 16 of [`wide_waist_family`], the 992 connected members of
/// [`wide_waist_permutation_family`], plus whichever of the [`RANDOM_TERMS`]
/// random walks came out connected — 1280 terms measured. All at
/// `Lambda = char`, `BlackBoxLabel = String` — **one instantiation**. Every term
/// is built from η, ε, μ, δ, σ, `id` only; none contains a `Spider` block.
/// Arities beyond 4, three or more distinct labels, and black boxes are outside
/// it. This is a wide finite sample, not a proof of the theorem.
///
/// **And the structural spread, stated in full rather than for its favourable
/// part.** A diagram's *interior waist* is the narrowest running codomain
/// strictly between two of its blocks; a one-wire internal boundary **splits**
/// it into two strictly smaller connected pieces, on which the conclusion
/// follows by induction. ⚠ The metric is evidence and not proof in either
/// direction — [`Built::is_wide_waist`] states both gaps, and 15 of the wide
/// terms below sit at the `1 → 1` arity where it over-reports. Over
/// the 1280 connected terms the interior-waist histogram is
/// `{None: 45, 1: 205, 2: 23, 3: 619, 4: 388}` — **1030** with a cut of two or
/// more, 205 with a one-wire cut, and 45 with no internal cut at all (fewer than
/// two blocks). All three buckets are pinned by
/// [`the_corpus_is_the_space_these_pins_claim`], so no part of the split is left
/// to be inferred. [`connected_family`] contributes the narrow bucket by
/// construction at `m >= 2`; [`wide_waist_permutation_family`] contributes 992
/// of the wide one, also by construction. Interior waists above 4 are outside
/// the space entirely.
///
/// **The oracle is independent of the builder.** `apex_len`, `scalar_count` and
/// the [`ApexClass`] preimages are read off the cospan image; the comparison
/// against `special_frobenius_morphism` is an *additional* assertion, so a
/// regression in the builder reddens this test rather than being compared
/// against itself.
///
/// **Falsification (production at `d6c7bd5`, corpus at this file's current
/// shape; every perturbation reverted after).**
///
/// | perturbation | result |
/// |---|---|
/// | `generator_to_cospan`'s `Comultiplication(z)` arm → the disconnected `Cospan::new_unchecked(vec![0], vec![0, 1], vec![z, z])` | red, **1165 of 1280** connected terms disagree — first witness `connected_a_0_1_v3`: `apex=2 scalars=1` where the spider is `apex=1 scalars=0`. **All 992** connected members of [`wide_waist_permutation_family`] are among them (counted), so the sweep is load-bearing rather than decorative |
/// | `SymmetricBraiding` arm made a same-label merge | **green** — see below |
/// | `special_frobenius_morphism`'s odd-`m` branch mirrored to `id ⊗ sfm(m-1, 1)` | **green** — see below |
/// | [`wide_waist_permutation_family`] dropped from [`corpus`] (with `CORPUS_SIZE` followed down, so the size assert still passes) | red on [`MIN_CONNECTED`]: **288 connected terms over 617**, floor 900. The wide bucket falls to 38 of 288 in the same run — so the sweep, not the seed, is what carries the structural spread |
///
/// The middle two are the honest statement of what this test *cannot* see, and
/// each is covered elsewhere in this file. A merging σ cannot change a
/// connected term's image: it is already one apex vertex, so unioning two of
/// its own wires moves nothing —
/// [`disconnected_recipes_denote_more_than_one_apex_vertex`] is what reddens
/// (**397 of 716**). A mirrored spider builder is *SCFM-equal* to the real one, so both
/// sides of the `canon != spider_canon` comparison move together and this test
/// stays green by rights; the term-level [`spider_3_1_via_double_mu`] and
/// [`spider_1_3_via_double_delta`] are what go red. That division of labour is
/// the reason the five §1 tests are kept rather than replaced.
#[test]
fn connected_diagrams_denote_the_spider_in_cospan() {
    let terms = corpus();
    assert_eq!(
        terms.len(),
        CORPUS_SIZE,
        "the corpus changed size without CORPUS_SIZE following it"
    );

    let mut failures: Vec<String> = Vec::new();
    let mut connected = 0usize;
    let mut with_braiding = 0usize;
    let mut wide_waist = 0usize;
    let mut max_depth = 0usize;
    let mut arities: HashSet<(usize, usize)> = HashSet::new();

    for built in &terms {
        if !built.connected || built.closed {
            continue;
        }
        connected += 1;
        if built.braidings > 0 {
            with_braiding += 1;
        }
        if built.is_wide_waist() {
            wide_waist += 1;
        }
        max_depth = max_depth.max(built.depth);

        let (m, n) = (built.domain.len(), built.codomain.len());
        arities.insert((m, n));

        // A connected term's boundary carries one label: μ is the only merging
        // block and it is typed, so a component never spans two labels.
        let mut boundary = built.domain.iter().chain(built.codomain.iter());
        let z = *boundary
            .next()
            .expect("invariant: m == n == 0 terms are excluded, so the boundary is non-empty");
        if built
            .domain
            .iter()
            .chain(built.codomain.iter())
            .any(|&c| c != z)
        {
            failures.push(format!(
                "  {}: connected but its boundary mixes labels: {:?} → {:?}",
                built.name, built.domain, built.codomain
            ));
            continue;
        }

        let canon = image(&built.term, &built.name);
        let expected_dom: Vec<usize> = (0..m).collect();
        let expected_cod: Vec<usize> = (0..n).collect();
        let spider_canon = image(
            &special_frobenius_morphism::<char, String>(m, n, z),
            "special_frobenius_morphism",
        );

        if canon.apex_len() != 1 || canon.scalar_count() != 0 {
            failures.push(format!(
                "  {}: connected {m}→{n} on '{z}' but its image is {} (want apex=1 scalars=0); \
                 classes {:?}",
                built.name,
                digest(&canon),
                canon.classes(),
            ));
            continue;
        }
        let class: &ApexClass<char> = &canon.classes()[0];
        if *class.label() != z
            || class.dom_preimage() != expected_dom.as_slice()
            || class.cod_preimage() != expected_cod.as_slice()
        {
            failures.push(format!(
                "  {}: connected {m}→{n} on '{z}' but its single apex class is \
                 (label '{}', dom {:?}, cod {:?}); want (label '{z}', dom {expected_dom:?}, cod \
                 {expected_cod:?})",
                built.name,
                class.label(),
                class.dom_preimage(),
                class.cod_preimage(),
            ));
            continue;
        }
        if canon != spider_canon {
            failures.push(format!(
                "  {}: connected {m}→{n} on '{z}' denotes {} but special_frobenius_morphism({m}, \
                 {n}, '{z}') denotes {}; classes {:?} vs {:?}",
                built.name,
                digest(&canon),
                digest(&spider_canon),
                canon.classes(),
                spider_canon.classes(),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {connected} connected diagrams do not denote their spider:\n{}",
        failures.len(),
        failures.join("\n"),
    );

    // Non-vacuity floors. Agreement over an empty (or degenerate) connected arm
    // would be agreement about nothing.
    assert!(
        connected >= MIN_CONNECTED,
        "the connected arm collapsed: {connected} connected terms over {} corpus terms, floor {} \
         (measured {} when this pin was written)",
        terms.len(),
        MIN_CONNECTED,
        MEASURED_CONNECTED,
    );
    assert!(
        with_braiding >= MIN_CONNECTED_WITH_BRAIDING,
        "the connected arm lost its braidings: {with_braiding} of {connected} carry a σ, floor {} \
         (measured {} when this pin was written)",
        MIN_CONNECTED_WITH_BRAIDING,
        MEASURED_CONNECTED_WITH_BRAIDING,
    );
    assert!(
        wide_waist >= MIN_CONNECTED_WIDE_WAIST,
        "the connected arm collapsed onto the spider's own factorisation: {wide_waist} of \
         {connected} terms have interior waist ≥ 2, floor {} (measured {} when this pin was \
         written) — without them every term that has an internal boundary at all splits at a \
         single wire into two smaller connected pieces, on which Thm 6.55 follows by induction \
         (the rest have fewer than two blocks and are instances of nothing)",
        MIN_CONNECTED_WIDE_WAIST,
        MEASURED_CONNECTED_WIDE_WAIST,
    );
    assert!(
        max_depth >= MIN_CONNECTED_MAX_DEPTH,
        "the connected arm went shallow: deepest recipe {max_depth} layers, floor {} (measured {} \
         when this pin was written)",
        MIN_CONNECTED_MAX_DEPTH,
        MEASURED_CONNECTED_MAX_DEPTH,
    );
    assert!(
        arities.len() >= MIN_CONNECTED_ARITIES,
        "the connected arm narrowed: {} distinct (m, n) arities, floor {} (measured {} when this \
         pin was written)",
        arities.len(),
        MIN_CONNECTED_ARITIES,
        MEASURED_CONNECTED_ARITIES,
    );
}

/// The rejected half of the same verdict: a recipe the disjoint-set calls
/// disconnected denotes **exactly** its own component count of apex vertices.
///
/// This is what makes the connectivity verdict a differential instead of an
/// unchecked filter. `apex_len() > 1` is the contrapositive the brief for this
/// pin asks for; the assertion here is the sharper `apex_len() == components`,
/// which on this arm is the same statement plus the count — *because* the arm
/// is restricted to `components >= 2`. That restriction is load-bearing and not
/// decoration: a recipe with `components == 0` would satisfy
/// `apex_len() == components` as `0 == 0`, which is true and says nothing, and
/// would not satisfy `apex_len() > 1` at all. Measured on `5457e2d`, when the
/// arm still admitted them, 47 of its 267 terms were that shape — all of them
/// the identical empty term, `[] → []`, depth 0, produced by walks that started
/// at width 0 and never drew an η. They are excluded at the filter below and
/// counted separately in the census; see the module header's *Excluded by
/// design*.
///
/// **Space the assertions actually touch.** The non-closed terms of [`corpus`]
/// with **two or more** recipe components — the nine scripted ones, the 496
/// disconnected members of [`wide_waist_permutation_family`], plus whichever
/// random walks came out that way; 716 measured — at `Lambda = char`,
/// `BlackBoxLabel = String`. Closed-component terms are excluded here for the
/// same reason as in the connected arm (the special vs extra-special line; see
/// the module header), which is why `scalar_count() == 0` is asserted rather
/// than assumed.
///
/// **Falsification (production at `d6c7bd5`, corpus at this file's current
/// shape; every perturbation reverted after).**
///
/// | perturbation | result |
/// |---|---|
/// | `generator_to_cospan`'s `SymmetricBraiding` arm → the merge `Cospan::new_unchecked(vec![0, 0], vec![0, 0], vec![z])` for **every** `σ` | red, but on a *type* error: `disc_mixed_label_sigma_between_components` is rejected by the layer fold with `'b' vs 'a'` at a common interface, because a merged apex cannot retype `[z, w] → [w, z]`. Worth recording — on distinct labels the permutation is the only well-typed reading, so this arm is not free to be wrong there |
/// | the same merge **restricted to `z == w`**, so every term stays type-correct | red, **397 of 716** disconnected recipes disagree — `disc_same_label_sigma_between_components`: recipe 2 components, image `apex=1` (one class, dom `[0,1,2,3]`, cod `[0,1]`) |
/// | `generator_to_cospan`'s `Comultiplication(z)` arm → the disconnected `Cospan::new_unchecked(vec![0], vec![0, 1], vec![z, z])` | red, **380 of 716** disagree — first witness `wide_perm_a_2_3_p0`: recipe 2 components, image `apex=3` |
///
/// The middle row's numerator was 29 of 220 before
/// [`wide_waist_permutation_family`] existed. The sweep is what moved it: every
/// one of its permutations lays σ's across a δ-fan whose two halves may or may
/// not end up in the same component, which is exactly the shape a merging
/// braiding arm gets wrong.
///
/// The second row is the one this test exists for, and it reddens **only** here:
/// [`connected_diagrams_denote_the_spider_in_cospan`] stayed green under it,
/// because a connected term is already one apex vertex and unioning two of its
/// own wires moves nothing. The scripted `disc_*_sigma_between_components` terms
/// are separated by nothing but a σ, and [`MIN_CROSS_COMPONENT_BRAIDINGS`] is
/// the floor that keeps that shape in the space.
#[test]
fn disconnected_recipes_denote_more_than_one_apex_vertex() {
    let terms = corpus();
    assert_eq!(
        terms.len(),
        CORPUS_SIZE,
        "the corpus changed size without CORPUS_SIZE following it"
    );

    let mut failures: Vec<String> = Vec::new();
    let mut disconnected = 0usize;
    let mut cross_component_braidings = 0usize;
    let mut distinct: HashSet<CospanCanon<char>> = HashSet::new();

    for built in &terms {
        // `components < 2` covers both the connected arm and the component-free
        // empty recipes, on which `apex_len() == components` is `0 == 0` and
        // pins nothing.
        if built.components < 2 || built.closed {
            continue;
        }
        disconnected += 1;
        cross_component_braidings += built.cross_component_braidings;

        let canon = image(&built.term, &built.name);
        distinct.insert(canon.clone());

        if canon.apex_len() != built.components || canon.scalar_count() != 0 {
            failures.push(format!(
                "  {}: the recipe has {} components and no closed one, but its image is {} \
                 ({}→{} wires); classes {:?}",
                built.name,
                built.components,
                digest(&canon),
                built.domain.len(),
                built.codomain.len(),
                canon.classes(),
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {disconnected} disconnected recipes do not denote their own component count:\n{}",
        failures.len(),
        failures.join("\n"),
    );

    assert!(
        disconnected >= MIN_DISCONNECTED,
        "the disconnected arm collapsed: {disconnected} terms, floor {} (measured {} when this \
         pin was written)",
        MIN_DISCONNECTED,
        MEASURED_DISCONNECTED,
    );
    assert!(
        distinct.len() >= MIN_DISCONNECTED_DISTINCT,
        "the disconnected arm degenerated: {} distinct canonical forms over {disconnected} terms, \
         floor {} (measured {} when this pin was written)",
        distinct.len(),
        MIN_DISCONNECTED_DISTINCT,
        MEASURED_DISCONNECTED_DISTINCT,
    );
    assert!(
        cross_component_braidings >= MIN_CROSS_COMPONENT_BRAIDINGS,
        "no σ spans two components any more ({cross_component_braidings}, floor {}, measured {} \
         when this pin was written) — a braiding arm turned into a merge would now pass",
        MIN_CROSS_COMPONENT_BRAIDINGS,
        MEASURED_CROSS_COMPONENT_BRAIDINGS,
    );
}

// The values measured on `d6c7bd5` (production) with the corpus at this file's
// current shape. All twelve are asserted exactly in
// `the_corpus_is_the_space_these_pins_claim`.
//
// Eight of them additionally appear in a failure message above or below, so a
// reader can tell a real regression from fixture drift without rerunning
// anything. Four do not, and here is exactly where each is reported, since a
// wrong answer to that is the same over-quantification this file exists to
// remove: `MEASURED_CLOSED_EXCLUDED` is reported by the census tuple and by the
// floor-vs-census loop (as `MIN_CLOSED_EXCLUDED`'s partner); `MEASURED_EMPTY_RECIPES`,
// `MEASURED_CONNECTED_NO_INTERNAL_CUT` and
// `MEASURED_CONNECTED_WIDE_TRIVIAL_ENDS` are reported by the census tuple
// **alone** — no floor guards any of them, because none bounds a claim test's
// space. They are census bookkeeping, and each does a different job. The
// connected arm's partition is `MEASURED_CONNECTED` (1280) = wide 1030 + narrow
// 205 + `MEASURED_CONNECTED_NO_INTERNAL_CUT` 45; `MEASURED_EMPTY_RECIPES` (47)
// is **not** part of it — those terms have `components == 0` and are skipped
// before the connected branch is reached, so adding them into that sum is a
// misread, not a drift. `MEASURED_CONNECTED_WIDE_TRIVIAL_ENDS` bounds how much
// of the wide bucket rests on `Built::is_wide_waist`'s over-reporting arity, and
// `MEASURED_EMPTY_RECIPES` is the non-emptiness pin of an excluded arm. Between
// them the corpus's structural bias is stated in full rather than for the
// favourable part.
const MEASURED_CONNECTED: usize = 1280;
const MEASURED_CONNECTED_WITH_BRAIDING: usize = 1105;
const MEASURED_CONNECTED_WIDE_WAIST: usize = 1030;
const MEASURED_CONNECTED_NO_INTERNAL_CUT: usize = 45;
/// Connected wide-waist terms at `m == n == 1` — the arity where
/// [`Built::is_wide_waist`] over-reports, since `s_{1,1}` is the identity and
/// the `s_{m,1} ; … ; s_{1,n}` phrase is then satisfied without splitting
/// anything. Pinned so the prose can *bound* that reading rather than estimate
/// it: 15 of the 1030 wide terms, and none of them in the permutation sweep,
/// whose terms all have `m, n >= 2`.
const MEASURED_CONNECTED_WIDE_TRIVIAL_ENDS: usize = 15;
const MEASURED_CONNECTED_MAX_DEPTH: usize = 21;
const MEASURED_CONNECTED_ARITIES: usize = 24;
const MEASURED_DISCONNECTED: usize = 716;
const MEASURED_DISCONNECTED_DISTINCT: usize = 203;
const MEASURED_CROSS_COMPONENT_BRAIDINGS: usize = 2169;
const MEASURED_CLOSED_EXCLUDED: usize = 62;
const MEASURED_EMPTY_RECIPES: usize = 47;

/// The corpus census, asserted exactly rather than by floor — so a change to the
/// generator has to restate what it produced instead of drifting inside the
/// floors the two claim tests use.
///
/// It also pins that the **excluded** arms are non-empty: terms whose recipe
/// closes a component do exist in the corpus, and so do component-free empty
/// recipes, so the exclusions documented in the module header are clauses about
/// something. Almost nothing is asserted *about* the closed terms — the
/// special vs extra-special question was answered elsewhere (#350: the closed
/// bubble survives), and widening this file's claims onto those terms is the
/// #353 follow-up rather than something this census does. The empty ones are
/// pinned to be
/// exactly the empty term (`[] → []`, depth 0), which is the whole of what
/// "no component at all" can mean.
///
/// **Space:** the corpus of [`corpus`] at `char`/`String` on the pinned seed
/// `0x6055_0001`.
///
/// **Eleven of the twelve numbers are properties of the generator**, not of the
/// production code under test, and for those this test goes red when the
/// *generator* drifts — which is exactly its job. The twelfth is not, and the
/// docstring says so rather than pointing a future maintainer at the wrong
/// cause: `MEASURED_DISCONNECTED_DISTINCT` counts distinct **canonical forms**,
/// so it is computed by [`image`] — `frobenius_to_cospan` + `canonical_form`,
/// production code — and moves when *that* changes. The falsification record
/// proves it (both measured, both reverted): perturbing `generator_to_cospan`'s
/// `Comultiplication` arm moves this count 203 → 220 and reddens this test, and
/// making its `SymmetricBraiding` arm an unrestricted merge takes this test down
/// with [`image`]'s **panic** — `disc_mixed_label_sigma_between_components:
/// frobenius_to_cospan rejected a black-box-free term` — rather than with an
/// assertion. A red on that one component means "read the diff", not "the
/// fixture drifted".
#[test]
fn the_corpus_is_the_space_these_pins_claim() {
    let terms = corpus();
    assert_eq!(terms.len(), CORPUS_SIZE, "corpus size");

    let mut connected = 0usize;
    let mut disconnected = 0usize;
    let mut closed = 0usize;
    let mut empty = 0usize;
    let mut with_braiding = 0usize;
    let mut wide_waist = 0usize;
    let mut no_internal_cut = 0usize;
    let mut wide_with_trivial_ends = 0usize;
    let mut perm_family_waists: HashSet<Option<usize>> = HashSet::new();
    let mut perm_family_structurally_equal = 0usize;
    let mut cross = 0usize;
    let mut max_depth = 0usize;
    let mut arities: HashSet<(usize, usize)> = HashSet::new();
    let mut disconnected_images: HashSet<CospanCanon<char>> = HashSet::new();

    for built in &terms {
        if built.components == 0 {
            empty += 1;
            // "No component at all" has exactly one witness: the recipe that
            // never applied a block. Pinned so the bucket cannot quietly grow a
            // second meaning.
            assert!(
                built.domain.is_empty() && built.codomain.is_empty() && built.depth == 0,
                "{}: components == 0 but it is not the empty term ({:?} → {:?}, depth {})",
                built.name,
                built.domain,
                built.codomain,
                built.depth,
            );
            continue;
        }
        if built.closed {
            closed += 1;
            continue;
        }
        if built.connected {
            connected += 1;
            if built.braidings > 0 {
                with_braiding += 1;
            }
            if built.is_wide_waist() {
                wide_waist += 1;
            }
            if built.interior_waist.is_none() {
                no_internal_cut += 1;
            }
            // The wide terms that rest on `is_wide_waist`'s over-reporting end:
            // at `m == n == 1` the `s_{m,1} ; … ; s_{1,n}` phrase is satisfied
            // vacuously by `id ; D ; id`, so "wide" there means only "this
            // spelling never passes through one wire". Counted so the docs can
            // bound the reading instead of estimating it.
            if built.is_wide_waist() && built.domain.len() == 1 && built.codomain.len() == 1 {
                wide_with_trivial_ends += 1;
            }
            if built.name.starts_with("wide_perm_") {
                let (m, n) = (built.domain.len(), built.codomain.len());
                // Per TERM, not pooled: a set of the waists seen cannot tell a
                // (3,3) term that regressed to 3 from a (2,2) term that
                // regressed to 4 — the pooled set is `{3, 4}` either way.
                assert_eq!(
                    built.interior_waist,
                    Some(m.min(n) + 1),
                    "{}: swept {m}→{n} term has interior waist {:?}, not min(m, n) + 1 = {}",
                    built.name,
                    built.interior_waist,
                    m.min(n) + 1,
                );
                perm_family_waists.insert(built.interior_waist);
                let spider: FM = special_frobenius_morphism(m, n, built.domain[0]);
                if built.term == spider {
                    perm_family_structurally_equal += 1;
                }
            }
            max_depth = max_depth.max(built.depth);
            arities.insert((built.domain.len(), built.codomain.len()));
        } else {
            disconnected += 1;
            cross += built.cross_component_braidings;
            disconnected_images.insert(image(&built.term, &built.name));
        }
    }

    // ⚠ Twelve elements is the ceiling for std's tuple `PartialEq`/`Debug`
    // impls. A thirteenth census bucket does not fail here with an arity
    // message — it fails with an unsatisfied trait bound. Split into a named
    // struct at that point rather than puzzling over the error.
    assert_eq!(
        (
            connected,
            disconnected,
            closed,
            empty,
            with_braiding,
            wide_waist,
            no_internal_cut,
            wide_with_trivial_ends,
            cross,
            max_depth,
            arities.len(),
            disconnected_images.len(),
        ),
        (
            MEASURED_CONNECTED,
            MEASURED_DISCONNECTED,
            MEASURED_CLOSED_EXCLUDED,
            MEASURED_EMPTY_RECIPES,
            MEASURED_CONNECTED_WITH_BRAIDING,
            MEASURED_CONNECTED_WIDE_WAIST,
            MEASURED_CONNECTED_NO_INTERNAL_CUT,
            MEASURED_CONNECTED_WIDE_TRIVIAL_ENDS,
            MEASURED_CROSS_COMPONENT_BRAIDINGS,
            MEASURED_CONNECTED_MAX_DEPTH,
            MEASURED_CONNECTED_ARITIES,
            MEASURED_DISCONNECTED_DISTINCT,
        ),
        "(connected, disconnected, closed, empty, connected-with-σ, \
         connected-with-interior-waist-≥-2, connected-with-no-internal-cut, \
         connected-wide-at-1→1, cross-component σ, deepest connected recipe, distinct connected \
         arities, distinct disconnected canonical forms)"
    );

    // `wide_waist_family`'s three named witnesses, guarded by NAME rather than
    // by a count. Measured: deleting all 16 of that family leaves every floor in
    // this file green — the permutation sweep alone keeps `MIN_CONNECTED` (1264
    // ≥ 900), `MIN_CONNECTED_WITH_BRAIDING` (1099) and
    // `MIN_CONNECTED_WIDE_WAIST` (1014) satisfied — so before this check the
    // exact census tuple was the only thing that noticed, and it would read as
    // fixture drift. A count floor would not fix that, since the sweep
    // out-supplies any number worth setting. What actually earns these 16 their
    // place is that three of them *are* the composites §1 measures as
    // structurally ≠ their spider, so those three are pinned by name.
    for name in [
        "wide_comb_a_2",
        "wide_braided_comb_a_2",
        "wide_folded_comb_a_2",
    ] {
        let found = terms
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("{name} left the corpus — see wide_waist_family"));
        assert!(
            found.connected && !found.closed && found.is_wide_waist(),
            "{name} is no longer a connected, non-closed, wide-waist term ({} components, closed \
             {}, interior waist {:?})",
            found.components,
            found.closed,
            found.interior_waist,
        );
        // …and the property that is the stated reason these three are here: each
        // is a connected diagram the builder does not itself build. Asserting
        // only "connected and wide" above would leave the justification
        // unchecked, so a change to `special_frobenius_morphism` or
        // `two_layer_simplify` that collapsed one onto its spider would pass.
        let (m, n) = (found.domain.len(), found.codomain.len());
        let spider: FM = special_frobenius_morphism(m, n, found.domain[0]);
        assert!(
            found.term != spider,
            "{name} is now structurally EQUAL to special_frobenius_morphism({m}, {n}, '{}') — the \
             reason this shape is in the corpus is that §1's term-level Eq cannot reach it",
            found.domain[0],
        );
    }

    // `wide_waist_permutation_family`'s two claims about its own terms, asserted
    // rather than left as prose. The per-term `min(m, n) + 1` law is checked in
    // the loop above, where the arity is in hand; these two are the corpus-wide
    // riders: both waists are actually *reached* (a sweep that lost the `m = 3`
    // regime would satisfy the per-term law over the survivors), and **none** of
    // the terms is structurally equal to its spider, which is why the sweep says
    // something §1's term-level `Eq` cannot.
    //
    // Neither can pass vacuously, and the order is what guarantees it: a
    // selector that stops matching leaves both accumulators empty, and an empty
    // set is not `{3, 4}`, so the waist assert fires first (measured: prefixing
    // the selector with `XX` gives `left: {}`). Only once it has passed is the
    // `== 0` below known to range over 992 terms rather than over none.
    let mut expected_waists: HashSet<Option<usize>> = HashSet::new();
    expected_waists.insert(Some(3));
    expected_waists.insert(Some(4));
    assert_eq!(
        perm_family_waists, expected_waists,
        "the permutation sweep no longer reaches both interior waists {{3, 4}}"
    );
    assert_eq!(
        perm_family_structurally_equal, 0,
        "{perm_family_structurally_equal} of the permutation sweep's connected terms are now \
         structurally equal to their spider — the sweep no longer lies outside §1's reach"
    );

    // Every floor constant in this file must sit at or below the census value it
    // guards, or the claim test asserting it is already red on the very corpus
    // the census above pins. All nine are checked, not just the ones that happen
    // to be convenient: eight are asserted by the two claim tests, and
    // `MIN_CLOSED_EXCLUDED` is asserted only here, as the non-emptiness pin of
    // the excluded arm — which is why it is listed last rather than first.
    //
    // What this can and cannot catch, stated so nobody reads more into it: the
    // exact census above has already pinned every `measured` value to its
    // `MEASURED_*` constant, so by the time this loop runs it is comparing two
    // constants. It fires when somebody *raises a floor* past its census value —
    // never on corpus drift, which the assert above catches first. That is the
    // whole of its job.
    for (name, floor, measured) in [
        ("MIN_CONNECTED", MIN_CONNECTED, connected),
        (
            "MIN_CONNECTED_WITH_BRAIDING",
            MIN_CONNECTED_WITH_BRAIDING,
            with_braiding,
        ),
        (
            "MIN_CONNECTED_WIDE_WAIST",
            MIN_CONNECTED_WIDE_WAIST,
            wide_waist,
        ),
        (
            "MIN_CONNECTED_MAX_DEPTH",
            MIN_CONNECTED_MAX_DEPTH,
            max_depth,
        ),
        (
            "MIN_CONNECTED_ARITIES",
            MIN_CONNECTED_ARITIES,
            arities.len(),
        ),
        ("MIN_DISCONNECTED", MIN_DISCONNECTED, disconnected),
        (
            "MIN_DISCONNECTED_DISTINCT",
            MIN_DISCONNECTED_DISTINCT,
            disconnected_images.len(),
        ),
        (
            "MIN_CROSS_COMPONENT_BRAIDINGS",
            MIN_CROSS_COMPONENT_BRAIDINGS,
            cross,
        ),
        ("MIN_CLOSED_EXCLUDED", MIN_CLOSED_EXCLUDED, closed),
    ] {
        assert!(
            floor <= measured,
            "{name} = {floor} sits above its census value {measured}, so the test that asserts it \
             is already red on the corpus this census pins — lower the floor or restate the census"
        );
    }

    // Nothing in the corpus escapes the four-way split.
    assert_eq!(
        connected + disconnected + closed + empty,
        CORPUS_SIZE,
        "connected + disconnected + closed + empty must exhaust the corpus"
    );
}
