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
//! to `two_layer_simplify`'s four rewrite rules (identity drop, σ;σ cancel,
//! η;ε cancel, `Spider`;`Spider` fusion) — it is *not* the Frobenius quotient,
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
//! place it is actually true: the image under
//! `frobenius_to_cospan` (F&S 2019 Prop 3.8), canonicalised by
//! `Cospan::canonical_form`. A connected `m → n` diagram must land on a
//! cospan with **one** apex vertex carrying every domain and codomain index —
//! and that is exactly the canonical form of the spider. The oracle is
//! independent of `special_frobenius_morphism`: `apex_len`, `scalar_count` and
//! the single [`ApexClass`]'s preimages are read off the cospan, not compared
//! against a builder. The builder is then compared *to it*, so
//! `special_frobenius_morphism` rides the same claim rather than defining it.
//!
//! It is the theorem *over the corpus it ranges over*, and that corpus has a
//! structural bias the test's own "Space the assertions actually touch"
//! paragraph now states outright: most of it factors through a single wire
//! (waist ≤ 1), which is the spider's own factorisation and for which the
//! conclusion is nearly immediate. [`wide_waist_family`] exists to put the
//! non-trivial shapes — connected diagrams whose narrowest running codomain is
//! two or more wires — into the space deliberately rather than by luck, and
//! [`MIN_CONNECTED_WIDE_WAIST`] is the floor that keeps them there.
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
//! The first two sit on the *special* vs *extra-special* line, which is owned by nobody
//! here and is not this file's to decide: `two_layer_simplify`'s rule 3 cancels
//! `η;ε` outright (the extra-special axiom) while `Cospan` is the **special**
//! theory, in which a closed bubble is a genuine `0 → 0` scalar. Measured on
//! `d6c7bd5`: `(η ⊗ id);(id ⊗ id);(ε ⊗ id)` normalises to a depth-1 term with
//! the bubble gone, whose canonical form is `apex_len = 1, scalars = 0`.
//! Excluding the shape keeps this file out of that decision. Both tests
//! additionally assert `scalar_count() == 0` on every term they *do* range
//! over, so a surviving scalar class reddens the pin rather than being quietly
//! absorbed.
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
//! *structural* spread is narrower than its arity spread: 17 of the 288
//! connected terms have waist ≥ 2 (§2 above), so the remaining 271 are already
//! in the spider's own factorisation. Nothing here is uniform over connected
//! diagrams — it is a corpus, and the census pins exactly which one.
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
/// **Space:** one term, one arity, one label, at `char`/`String`. Note the
/// asymmetry this pins and does not resolve: at the *term* level
/// `two_layer_simplify` cancels `η;ε` (the extra-special axiom), so both sides
/// here are the empty term; at the *cospan* level the same diagram is a
/// non-identity scalar. That is why `(0, 0)` is excluded from
/// [`connected_diagrams_denote_the_spider_in_cospan`] — see the module header.
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
/// 192 scripted connected + 16 scripted wide-waist + 9 scripted disconnected +
/// [`RANDOM_TERMS`].
const CORPUS_SIZE: usize = 192 + 16 + 9 + RANDOM_TERMS;

/// Floor on the number of **connected** terms
/// [`connected_diagrams_denote_the_spider_in_cospan`] ranges over.
/// Measured 288; the 208 scripted ones alone would satisfy it.
const MIN_CONNECTED: usize = 200;

/// Floor on connected terms carrying at least one σ block. Measured 113.
const MIN_CONNECTED_WITH_BRAIDING: usize = 90;

/// Floor on connected terms whose **waist** — the narrowest running codomain
/// the recipe ever reaches — is two wires or more. Measured 17.
///
/// This is the load-bearing count for the corpus's one structural bias. A
/// diagram of waist ≤ 1 factors as `s_{m,1} ; (decoration) ; s_{1,n}`, which is
/// the spider's own factorisation, and Thm 6.55 is nearly immediate on it; the
/// shapes that make the theorem say something are the ones that stay two or
/// more wires wide throughout. [`wide_waist_family`] supplies 16 of them
/// (measured; a single random walk supplies the seventeenth), and this floor is
/// what notices if they leave.
const MIN_CONNECTED_WIDE_WAIST: usize = 12;

/// Floor on the deepest recipe (layers appended) among the connected terms.
/// The brief for this pin asks for depth ≥ 3; the scripted family reaches 11
/// (measured) and the floor records that rather than the minimum.
const MIN_CONNECTED_MAX_DEPTH: usize = 8;

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
/// Measured 220. (The component-free empty recipes are *not* in this arm; see
/// the module header's *Excluded by design*.)
const MIN_DISCONNECTED: usize = 150;

/// Floor on distinct canonical forms in the disconnected arm. A generator that
/// degenerated to one repeated shape would still satisfy [`MIN_DISCONNECTED`];
/// this is what notices. Measured 184.
const MIN_DISCONNECTED_DISTINCT: usize = 120;

/// Floor on σ blocks laid **between two distinct components** across the
/// disconnected arm. Measured 132.
///
/// This is the load-bearing count for one of the two falsification
/// perturbations: making the braiding arm a merge instead of a permutation
/// changes nothing about a connected term's image (it is already one apex
/// vertex), so only a σ that spans two components can see it.
const MIN_CROSS_COMPONENT_BRAIDINGS: usize = 60;

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
    cross_component_braidings: usize,
    /// The narrowest running codomain the recipe has reached so far, counting
    /// the starting domain and the codomain after every applied block — the
    /// diagram's *waist*. A rejected block does not move it.
    min_waist: usize,
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
            cross_component_braidings: 0,
            min_waist: domain.len(),
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

    /// Apply one block, or report why it does not fit, keeping [`Self::min_waist`]
    /// up to date on success.
    fn apply(&mut self, block: Block) -> Result<(), String> {
        let outcome = self.apply_block(block);
        if outcome.is_ok() {
            self.min_waist = self.min_waist.min(self.wires.len());
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
                if self.find(a.comp) != self.find(b.comp) {
                    self.cross_component_braidings += 1;
                }
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
            cross_component_braidings: self.cross_component_braidings,
            min_waist: self.min_waist,
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
    cross_component_braidings: usize,
    /// The narrowest running codomain the recipe ever reached — the diagram's
    /// waist. `min_waist >= 2` is what makes a connected term a non-trivial
    /// instance of Thm 6.55; see [`MIN_CONNECTED_WIDE_WAIST`].
    min_waist: usize,
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
/// makes every term here a `s_{m,1} ; decoration ; s_{1,n}`, i.e. of *waist* ≤ 1,
/// which is the spider's own factorisation. That is the corpus's one systematic
/// bias, and [`wide_waist_family`] is what answers it.
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
/// [`connected_family`]'s recipes all fold to one wire before decorating, so
/// their *waist* — the narrowest running codomain the recipe reaches — is at
/// most 1. Such a diagram is already in the spider's own
/// `s_{m,1} ; decoration ; s_{1,n}` shape, and "one apex vertex" follows almost
/// immediately from the recipe. Measured on `5457e2d`, before this family
/// existed, the waist histogram over the 272 connected terms was
/// `{0: 107, 1: 164, 2: 1}` — 271 of 272 factored through at most one wire, and
/// the single exception (`random_263`, `2 → 2`,
/// `[Delta(0), Delta(2), Eps(0), Mu(0)]`) was an accident of the seed.
///
/// These sixteen never narrow below `w >= 2` wires. Three shapes, each on both
/// labels, generalising three of the composites the module header measures as
/// structurally **≠** their spider:
///
/// - **comb `w`** — for `i` in `0..w-1`: split wire `i`, merge the copy into
///   wire `i + 1`. At `w == 2` this is exactly `(δ ⊗ id);(id ⊗ μ)`. Arity
///   `(w, w)`, waist `w`.
/// - **braided comb `w`** — the same with a σ between the split and the merge.
///   At `w == 2`, exactly `(δ ⊗ id);(id ⊗ σ);(μ ⊗ id)`. Arity `(w, w)`,
///   waist `w`.
/// - **folded comb `w`** — one μ first, then comb `w`. At `w == 2`, exactly
///   `(μ ⊗ id);(δ ⊗ id);(id ⊗ μ)`. Arity `(w + 1, w)`, waist `w`.
///
/// `w` runs `2..=4` for the first two and `2..=3` for the third, which keeps
/// every arity inside the scripted grid `0..=4 × 0..=4` (so this family does not
/// move [`MEASURED_CONNECTED_ARITIES`]). As with [`connected_family`], each is
/// connected *by construction* and the disjoint-set then verifies it: a
/// scripting mistake lands the term in the other arm and reddens the census.
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

/// The whole corpus: 192 scripted connected terms, 16 scripted wide-waist ones,
/// 9 scripted disconnected terms, and [`RANDOM_TERMS`] pseudo-random ones seeded
/// at `0x6055_0001`.
///
/// The scripted families are appended *before* the seeded walks are drawn, so
/// adding one does not perturb the random stream.
fn corpus() -> Vec<Built> {
    let mut out = connected_family();
    out.extend(wide_waist_family());
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
/// 16 of [`wide_waist_family`], plus whichever of the [`RANDOM_TERMS`] random
/// walks came out connected. All at `Lambda = char`, `BlackBoxLabel = String` —
/// **one instantiation**. Every term is built from η, ε, μ, δ, σ, `id` only;
/// none contains a `Spider` block. Arities beyond 4, three or more distinct
/// labels, and black boxes are outside it. This is a wide finite sample, not a
/// proof of the theorem.
///
/// **And one structural restriction, stated because it dominates the space.**
/// A diagram's *waist* is the narrowest running codomain its recipe reaches.
/// Waist ≤ 1 means the diagram factors as `s_{m,1} ; decoration ; s_{1,n}` —
/// the spider's own factorisation, on which the conclusion is nearly immediate.
/// [`connected_family`] is waist ≤ 1 by construction, and measured on `5457e2d`
/// the whole connected arm had waist histogram `{0: 107, 1: 164, 2: 1}`: 271 of
/// 272 terms factored through at most one wire, the exception being a single
/// random walk. [`wide_waist_family`] was added for exactly that reason and
/// [`MIN_CONNECTED_WIDE_WAIST`] is asserted below, so the non-trivial shapes are
/// in the space on purpose; they are still a minority of it (17 of 288 measured,
/// waists 2–4). Waists above 4 are outside the space entirely.
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
/// | `generator_to_cospan`'s `Comultiplication(z)` arm → the disconnected `Cospan::new_unchecked(vec![0], vec![0, 1], vec![z, z])` | red, **173 of 288** connected terms disagree — first witness `connected_a_0_1_v3`: `apex=2 scalars=1` where the spider is `apex=1 scalars=0`. **All 16** of [`wide_waist_family`] are among them (e.g. `wide_comb_a_2`, `2 → 2`: `apex=2`, classes `[(a, dom [0], cod [0]), (a, dom [1], cod [1])]`), so the wide-waist arm is load-bearing rather than decorative |
/// | `SymmetricBraiding` arm made a same-label merge | **green** — see below |
/// | `special_frobenius_morphism`'s odd-`m` branch mirrored to `id ⊗ sfm(m-1, 1)` | **green** — see below |
/// | [`wide_waist_family`] dropped from [`corpus`] (with `CORPUS_SIZE` followed down, so the size assert still passes) | red on the waist floor: **1 of 272** connected terms have waist ≥ 2 — the pin above cannot be satisfied by the seed alone |
///
/// The middle two are the honest statement of what this test *cannot* see, and
/// each is covered elsewhere in this file. A merging σ cannot change a
/// connected term's image: it is already one apex vertex, so unioning two of
/// its own wires moves nothing —
/// [`disconnected_recipes_denote_more_than_one_apex_vertex`] is what reddens (29
/// of 220). A mirrored spider builder is *SCFM-equal* to the real one, so both
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
        if built.min_waist >= 2 {
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
         {connected} terms have waist ≥ 2, floor {} (measured {} when this pin was written) — \
         without them every term factors through a single wire and Thm 6.55 is nearly immediate",
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
/// with **two or more** recipe components — the nine scripted ones plus
/// whichever random walks came out that way — at `Lambda = char`,
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
/// | the same merge **restricted to `z == w`**, so every term stays type-correct | red, **29 of 220** disconnected recipes disagree — `disc_same_label_sigma_between_components`: recipe 2 components, image `apex=1` (one class, dom `[0,1,2,3]`, cod `[0,1]`) |
/// | `generator_to_cospan`'s `Comultiplication(z)` arm → the disconnected `Cospan::new_unchecked(vec![0], vec![0, 1], vec![z, z])` | red, **84 of 220** disagree — `disc_mixed_label_sigma_between_components`: recipe 2 components, image `apex=3` |
///
/// Both numerators are unchanged from the measurement taken when the arm still
/// admitted the 47 component-free recipes — re-measured, not inferred: such a
/// recipe carries no block at all, so no perturbation of a generator's cospan
/// can move it. Only the denominators moved, 267 → 220.
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
// current shape. All ten are asserted exactly in
// `the_corpus_is_the_space_these_pins_claim`. Eight of them additionally appear
// in a failure message above or below, so a reader can tell a real regression
// from fixture drift without rerunning anything; the two that do not —
// `MEASURED_CLOSED_EXCLUDED` and `MEASURED_EMPTY_RECIPES` — belong to arms no
// claim test ranges over, and are reported only by the census tuple and by the
// floor guards at the end of it.
const MEASURED_CONNECTED: usize = 288;
const MEASURED_CONNECTED_WITH_BRAIDING: usize = 113;
const MEASURED_CONNECTED_WIDE_WAIST: usize = 17;
const MEASURED_CONNECTED_MAX_DEPTH: usize = 11;
const MEASURED_CONNECTED_ARITIES: usize = 24;
const MEASURED_DISCONNECTED: usize = 220;
const MEASURED_DISCONNECTED_DISTINCT: usize = 184;
const MEASURED_CROSS_COMPONENT_BRAIDINGS: usize = 132;
const MEASURED_CLOSED_EXCLUDED: usize = 62;
const MEASURED_EMPTY_RECIPES: usize = 47;

/// The corpus census, asserted exactly rather than by floor — so a change to the
/// generator has to restate what it produced instead of drifting inside the
/// floors the two claim tests use.
///
/// It also pins that the **excluded** arms are non-empty: terms whose recipe
/// closes a component do exist in the corpus, and so do component-free empty
/// recipes, so the exclusions documented in the module header are clauses about
/// something. Almost nothing is asserted *about* the closed terms — deciding
/// whether a closed bubble survives is the special vs extra-special question
/// this file deliberately does not answer. The empty ones are pinned to be
/// exactly the empty term (`[] → []`, depth 0), which is the whole of what
/// "no component at all" can mean.
///
/// **Space:** the corpus of [`corpus`] at `char`/`String` on the pinned seed
/// `0x6055_0001`.
///
/// **Nine of the ten numbers are properties of the generator**, not of the
/// production code under test, and for those this test goes red when the
/// *generator* drifts — which is exactly its job. The tenth is not, and the
/// docstring says so rather than pointing a future maintainer at the wrong
/// cause: `MEASURED_DISCONNECTED_DISTINCT` counts distinct **canonical forms**,
/// so it is computed by [`image`] — `frobenius_to_cospan` + `canonical_form`,
/// production code — and moves when *that* changes. The falsification record
/// proves it (both measured, both reverted): perturbing `generator_to_cospan`'s
/// `Comultiplication` arm moves this count 184 → 189 and reddens this test, and
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
            if built.min_waist >= 2 {
                wide_waist += 1;
            }
            max_depth = max_depth.max(built.depth);
            arities.insert((built.domain.len(), built.codomain.len()));
        } else {
            disconnected += 1;
            cross += built.cross_component_braidings;
            disconnected_images.insert(image(&built.term, &built.name));
        }
    }

    assert_eq!(
        (
            connected,
            disconnected,
            closed,
            empty,
            with_braiding,
            wide_waist,
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
            MEASURED_CROSS_COMPONENT_BRAIDINGS,
            MEASURED_CONNECTED_MAX_DEPTH,
            MEASURED_CONNECTED_ARITIES,
            MEASURED_DISCONNECTED_DISTINCT,
        ),
        "(connected, disconnected, closed, empty, connected-with-σ, connected-with-waist-≥-2, \
         cross-component σ, deepest connected recipe, distinct connected arities, distinct \
         disconnected canonical forms)"
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
