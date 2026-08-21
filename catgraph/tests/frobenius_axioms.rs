//! The Def 2.5 special-commutative-Frobenius equations, built on **both** sides
//! and decided, for the four
//! [`HypergraphCategory`](catgraph::hypergraph_category::HypergraphCategory)
//! carriers this crate defines (Fong-Spivak 2019 Def 2.12, whose Frobenius
//! requirement *is* Def 2.5).
//!
//! # What this battery ranges over, and what it does not
//!
//! Eleven equations: Def 2.5's nine with unitality and counitality each split
//! into their left and right halves, since each half can fail alone. Each is
//! built from the carrier's own `unit` / `counit` / `multiplication` /
//! `comultiplication` / `identity` / braiding, on **one** wire type (`'z'`) at
//! the arities the equations themselves force (≤ 3 wires). It does not range
//! over several wire types, over wider spiders, over black-boxed generators, or
//! over any equation outside Def 2.5 — a carrier can pass every test here and
//! still get a 4-input spider or a heterogeneous braiding wrong.
//!
//! **The carrier limit.** "Four carriers" is four *named* carriers, not every
//! implementor. Six types implement `HypergraphCategory` in this workspace: the
//! four below plus `catgraph_applied::petri_net::PetriNet` and
//! `catgraph_applied::decorated_cospan::DecoratedCospan`, neither of which is
//! decided here or anywhere else — the only Def 2.5 equation pins in
//! `catgraph-applied` are for `MatKron`. That gap is not hypothetical for
//! `PetriNet`: `catgraph-applied/tests/braiding_cross_carrier.rs` already
//! records it as the carrier whose **constructor** is lossy —
//! `from_decorated_cospan` keeps only the apex as places and the decoration as
//! transitions, discarding both leg maps — so it is the last one a reader
//! should assume this file covers. (That file separately notes a *different*
//! fact about `permute_side`: it permutes `self.transitions`, so its `p` is
//! sized by the transition count rather than by a boundary arity. The two are
//! not the same mechanism and an earlier revision of this paragraph merged
//! them. See #272, whose ratified reading retains the boundary.)
//!
//! **The carrier count is three decision paths, not four.** `Corel` is a
//! transparent newtype over `Cospan` and delegates everything the battery
//! touches, so its row recomputes the `Cospan` row — measured in
//! [`corel_recomputes_the_cospan_battery`], not asserted. A mutant that reddens
//! both has been caught once, not twice. Nor are the remaining three fully
//! disjoint: `frobenius_to_cospan` interprets each generator *as* the
//! corresponding `Cospan` generator, so the `FrobeniusMorphism` row shares that
//! half of its path with the `Cospan` row and contributes its own composition
//! and normalizer on top. Measured with a non-merging μ and a non-splitting δ
//! on `Cospan`'s `HypergraphCategory` impl: `Cospan`, `Corel` and
//! `FrobeniusMorphism` all go red, `CospanAlgebraMorphism` — which builds its
//! generators from its own literal cospans — stays green.
//!
//! Two riders sit beside the eleven because the eleven cannot see them:
//! [`zigzag_identities_per_carrier`] (a cup that never composed leaves every
//! Def 2.5 equation intact) and [`braiding_is_a_genuine_crossing_per_carrier`]
//! (μ and δ are symmetric, so `σ ; μ == μ` holds for σ = id — measured, not
//! feared: swapping the braiding for the identity left all four carrier
//! batteries green).
//!
//! # Why each carrier is decided the way it is
//!
//! - [`Cospan`] — [`Cospan::canonical_form`], the complete invariant for
//!   parallel-cospan equality, on the nose.
//! - [`Corel`] — the *same* computation, reached through the newtype's
//!   delegation. Its own contribution is
//!   [`corel_battery_composites_stay_jointly_surjective`], the one claim
//!   `Cospan` cannot make.
//! - [`CospanAlgebraMorphism`] over [`PartitionAlgebra`] — the same, **after
//!   discarding scalar (bubble) classes**, because five of the eleven fail on
//!   the nose: `multiplication_in` and `comultiplication_in` build their
//!   structural cospan over a three-vertex apex that only one vertex is ever
//!   reached on, so each μ and each δ contributes two apex classes no boundary
//!   touches. That deviation is not hidden by the quotient; it is measured, to
//!   the bubble, in [`cospan_algebra_morphism_bubble_ledger`].
//! - [`FrobeniusMorphism`] — [`frobenius_to_cospan`] then `canonical_form`.
//!   Its own `PartialEq` compares *presentations* and separates both sides of
//!   **all eleven**; [`frobenius_structural_equality_decides_nothing_here`]
//!   pins that count rather than leaving it as a remark.

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    corel::Corel,
    cospan::Cospan,
    cospan_algebra::PartitionAlgebra,
    cospan_canon::{ApexClass, CospanCanon},
    equivalence::CospanAlgebraMorphism,
    frobenius::{FrobeniusMorphism, frobenius_to_cospan},
    hypergraph_category::HypergraphCategory,
};
use permutations::Permutation;

/// The wire type every equation is built on.
const Z: char = 'z';

/// How many equations [`equations`] returns. Def 2.5's nine, with unitality and
/// counitality split into halves.
const EQUATION_COUNT: usize = 11;

/// A carrier under test: a hypergraph category plus a *decider* for equality of
/// two parallel morphisms.
trait Carrier: HypergraphCategory<char> + Clone + Sized {
    const NAME: &'static str;

    /// The comparison value: equality of keys is the equation's verdict.
    type Key: PartialEq + std::fmt::Debug;

    fn seq(&self, other: &Self) -> Self;
    fn key(&self) -> Self::Key;
    fn dom(&self) -> Vec<char>;
    fn cod(&self) -> Vec<char>;

    fn par(&self, other: &Self) -> Self {
        let mut answer = self.clone();
        answer.monoidal(other.clone());
        answer
    }

    fn id(types: &[char]) -> Self {
        <Self as HasIdentity<Vec<char>>>::identity(&types.to_vec())
    }

    /// σ: `[z, z] → [z, z]`, the braiding on two equal wires.
    fn swap(z: char) -> Self {
        Self::from_permutation_on_domain(Permutation::transposition(2, 0, 1), &[z, z])
            .expect("invariant: a 2-element transposition matches a 2-wire word")
    }
}

impl Carrier for Cospan<char> {
    const NAME: &'static str = "Cospan<char>";
    type Key = CospanCanon<char>;

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("battery composites are type-correct by hand")
    }
    fn key(&self) -> Self::Key {
        self.canonical_form()
    }
    fn dom(&self) -> Vec<char> {
        Composable::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        Composable::codomain(self)
    }
}

impl Carrier for Corel<char> {
    const NAME: &'static str = "Corel<char>";
    type Key = CospanCanon<char>;

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("battery composites are type-correct by hand")
    }
    fn key(&self) -> Self::Key {
        self.as_cospan().canonical_form()
    }
    fn dom(&self) -> Vec<char> {
        Composable::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        Composable::codomain(self)
    }
}

/// Drop the bubble classes, keeping the partition of the two boundaries.
///
/// Legal as a `CospanCanon`: scalars have empty preimages on both legs, so
/// removing them leaves the boundary partition, the ascending preimages and the
/// class sort exactly as they were — `from_parts` re-checks all three.
fn without_scalars(canon: &CospanCanon<char>) -> CospanCanon<char> {
    let kept: Vec<ApexClass<char>> = canon
        .classes()
        .iter()
        .filter(|class| !class.is_scalar())
        .map(|class| {
            ApexClass::new(
                *class.label(),
                class.dom_preimage().to_vec(),
                class.cod_preimage().to_vec(),
            )
        })
        .collect();
    CospanCanon::from_parts(canon.dom_len(), canon.cod_len(), kept)
        .expect("invariant: dropping empty-preimage classes preserves every from_parts invariant")
}

impl Carrier for CospanAlgebraMorphism<PartitionAlgebra, char> {
    const NAME: &'static str = "CospanAlgebraMorphism<PartitionAlgebra, char>";
    type Key = CospanCanon<char>;

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("battery composites are type-correct by hand")
    }
    fn key(&self) -> Self::Key {
        without_scalars(&self.element().canonical_form())
    }
    fn dom(&self) -> Vec<char> {
        Composable::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        Composable::codomain(self)
    }
}

impl Carrier for FrobeniusMorphism<char, String> {
    const NAME: &'static str = "FrobeniusMorphism<char, String>";
    type Key = CospanCanon<char>;

    fn seq(&self, other: &Self) -> Self {
        let mut answer = self.clone();
        ComposableMutating::compose(&mut answer, other.clone())
            .expect("battery composites are type-correct by hand");
        answer
    }
    fn key(&self) -> Self::Key {
        frobenius_to_cospan(self)
            .expect("the battery builds no black boxes, so every term interprets")
            .canonical_form()
    }
    fn dom(&self) -> Vec<char> {
        ComposableMutating::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        ComposableMutating::codomain(self)
    }
}

/// The eleven equations, each as `(name, lhs, rhs)` built from the carrier's own
/// generators. `;` reads left to right, so `f.seq(&g)` is "f then g".
fn equations<C: Carrier>(z: char) -> Vec<(&'static str, C, C)> {
    let eta = || C::unit(z);
    let eps = || C::counit(z);
    let mu = || C::multiplication(z);
    let delta = || C::comultiplication(z);
    let id = || C::id(&[z]);
    let sigma = || C::swap(z);

    let table = vec![
        // --- commutative monoid (μ, η) ---
        (
            "associativity: (mu (x) id) ; mu == (id (x) mu) ; mu",
            mu().par(&id()).seq(&mu()),
            id().par(&mu()).seq(&mu()),
        ),
        (
            "left unitality: (eta (x) id) ; mu == id",
            eta().par(&id()).seq(&mu()),
            id(),
        ),
        (
            "right unitality: (id (x) eta) ; mu == id",
            id().par(&eta()).seq(&mu()),
            id(),
        ),
        ("commutativity: sigma ; mu == mu", sigma().seq(&mu()), mu()),
        // --- cocommutative comonoid (δ, ε) ---
        (
            "coassociativity: delta ; (delta (x) id) == delta ; (id (x) delta)",
            delta().seq(&delta().par(&id())),
            delta().seq(&id().par(&delta())),
        ),
        (
            "left counitality: delta ; (eps (x) id) == id",
            delta().seq(&eps().par(&id())),
            id(),
        ),
        (
            "right counitality: delta ; (id (x) eps) == id",
            delta().seq(&id().par(&eps())),
            id(),
        ),
        (
            "cocommutativity: delta ; sigma == delta",
            delta().seq(&sigma()),
            delta(),
        ),
        // --- Frobenius law, both handednesses ---
        (
            "Frobenius left: (delta (x) id) ; (id (x) mu) == mu ; delta",
            delta().par(&id()).seq(&id().par(&mu())),
            mu().seq(&delta()),
        ),
        (
            "Frobenius right: (id (x) delta) ; (mu (x) id) == mu ; delta",
            id().par(&delta()).seq(&mu().par(&id())),
            mu().seq(&delta()),
        ),
        // --- special ---
        ("speciality: delta ; mu == id", delta().seq(&mu()), id()),
    ];
    assert_eq!(
        table.len(),
        EQUATION_COUNT,
        "the equation table changed size without EQUATION_COUNT following it"
    );
    table
}

/// Run all eleven on one carrier, reporting *every* failure with both keys
/// rather than dying on the first.
fn run<C: Carrier>() {
    let mut failures: Vec<String> = Vec::new();
    for (name, lhs, rhs) in equations::<C>(Z) {
        assert_eq!(lhs.dom(), rhs.dom(), "[{}] {name}: domains differ", C::NAME);
        assert_eq!(
            lhs.cod(),
            rhs.cod(),
            "[{}] {name}: codomains differ",
            C::NAME
        );
        let (left, right) = (lhs.key(), rhs.key());
        if left != right {
            failures.push(format!("  {name}\n    lhs = {left:?}\n    rhs = {right:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "[{}] {} of the {EQUATION_COUNT} Def 2.5 equations failed:\n{}",
        C::NAME,
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn cospan_battery() {
    run::<Cospan<char>>();
}

/// `Cospan` decides the battery on the nose, with **no** bubbles on either side
/// of any equation.
///
/// Split out from [`cospan_battery`] because the equality there would stay green
/// if both sides ever grew the *same* spurious scalar; this says the count is
/// zero, which is the claim the paper makes about these composites.
#[test]
fn cospan_battery_creates_no_scalars() {
    for (name, lhs, rhs) in equations::<Cospan<char>>(Z) {
        assert_eq!(
            (lhs.key().scalar_count(), rhs.key().scalar_count()),
            (0, 0),
            "{name}: a Def 2.5 composite in Cospan grew a bubble"
        );
    }
}

/// The eleven on `Corel`, which is **not** an independent verdict.
///
/// See [`corel_recomputes_the_cospan_battery`]: `Corel` delegates every
/// operation the battery uses, and its key is the wrapped `Cospan`'s key, so
/// this test cannot go red unless [`cospan_battery`] does. It stays because the
/// delegation is what makes that true, and a `Corel` that stopped delegating
/// should be caught here rather than assumed. The `Corel`-specific content of
/// this file is [`corel_battery_composites_stay_jointly_surjective`].
#[test]
fn corel_battery() {
    run::<Corel<char>>();
}

/// Measured: the `Corel` row of the battery is the `Cospan` row, key for key.
///
/// `Corel<Lambda>` is a transparent newtype. `unit` / `counit` /
/// `multiplication` / `comultiplication` / `cup` / `cap` are
/// `new_unchecked(Cospan::…)`, `identity` / `compose` / `monoidal` /
/// `from_permutation_on_domain` delegate to the wrapped value, and
/// [`Carrier::key`] is `self.as_cospan().canonical_form()` — the same function
/// `Cospan`'s key calls. So "eleven equations on four carriers" is really
/// eleven on **three** decision paths, and counting `corel_battery` and
/// `cospan_battery` as two independent reds under one mutant double-counts one
/// computation.
///
/// Written to go red if that ever stops being true, which is the only way it
/// could become news: a `Corel` that overrides any of those operations makes
/// the row independent and this test is where that shows up.
#[test]
fn corel_recomputes_the_cospan_battery() {
    let cospans = equations::<Cospan<char>>(Z);
    let corels = equations::<Corel<char>>(Z);
    assert_eq!(
        cospans.len(),
        corels.len(),
        "the two tables disagree in size"
    );

    for ((name, cospan_lhs, cospan_rhs), (_, corel_lhs, corel_rhs)) in
        cospans.into_iter().zip(corels)
    {
        assert_eq!(
            corel_lhs.key(),
            cospan_lhs.key(),
            "{name}: the Corel lhs stopped being the Cospan lhs, so the Corel row is now an \
             independent verdict and the audit doc may stop calling it a recomputation"
        );
        assert_eq!(
            corel_rhs.key(),
            cospan_rhs.key(),
            "{name}: the Corel rhs stopped being the Cospan rhs"
        );
    }
}

/// Every composite in the battery stays a corelation — jointly surjective — so
/// the `Corel` row is decided on genuine `Corel` values and not on wrappers that
/// `Corel::new` would have rejected.
///
/// This is the one assertion in the file that `Cospan` cannot make, and so the
/// one that earns `Corel` a row: `Corel::compose` and `Corel::monoidal` both go
/// through `new_unchecked`, so nothing else in the crate would notice if a
/// composite left the subcategory.
#[test]
fn corel_battery_composites_stay_jointly_surjective() {
    for (name, lhs, rhs) in equations::<Corel<char>>(Z) {
        assert!(
            lhs.as_cospan().is_jointly_surjective(),
            "{name}: lhs left the corelation subcategory"
        );
        assert!(
            rhs.as_cospan().is_jointly_surjective(),
            "{name}: rhs left the corelation subcategory"
        );
    }
}

#[test]
fn cospan_algebra_morphism_battery() {
    run::<CospanAlgebraMorphism<PartitionAlgebra, char>>();
}

/// The measured deviation the `CospanAlgebraMorphism` key quotients away.
///
/// The bubbles are born with the generators, not made by composition:
/// `multiplication_in` and `comultiplication_in` each build their structural
/// cospan over a **three**-vertex apex `[z, z, z]` whose right leg is `[0, 0, 0]`,
/// so vertices 1 and 2 are reached by nothing and the element carries two
/// scalars from the start. Every μ or δ in a term contributes two, and an
/// identity contributes none — which is exactly the table below, and is why the
/// five equations that compare a term against a bare identity are the five that
/// fail on the nose. `Cospan`'s own μ and δ use a one-vertex apex and have no
/// such classes ([`cospan_battery_creates_no_scalars`]).
///
/// The numbers are measurements, not targets. They are what makes this a ledger
/// rather than a shrug: narrowing those apexes to one vertex — the fix that
/// would let this carrier's key drop [`without_scalars`] — moves every entry to
/// `(0, 0)` and trips this test rather than passing silently.
#[test]
fn cospan_algebra_morphism_bubble_ledger() {
    let measured: Vec<(&str, usize, usize)> =
        equations::<CospanAlgebraMorphism<PartitionAlgebra, char>>(Z)
            .into_iter()
            .map(|(name, lhs, rhs)| {
                (
                    name,
                    lhs.element().canonical_form().scalar_count(),
                    rhs.element().canonical_form().scalar_count(),
                )
            })
            .collect();

    let expected: [(usize, usize); EQUATION_COUNT] = [
        (4, 4), // associativity — two μ each, so the counts cancel
        (2, 0), // left unitality — one μ against a bare identity
        (2, 0), // right unitality
        (2, 2), // commutativity — one μ on each side
        (4, 4), // coassociativity — two δ each
        (2, 0), // left counitality
        (2, 0), // right counitality
        (2, 2), // cocommutativity — one δ on each side
        (4, 4), // Frobenius left — one μ and one δ on each side
        (4, 4), // Frobenius right
        (4, 0), // speciality — μ and δ against a bare identity
    ];

    let counts: Vec<(usize, usize)> = measured.iter().map(|&(_, lhs, rhs)| (lhs, rhs)).collect();
    assert_eq!(
        counts.as_slice(),
        expected.as_slice(),
        "the H_Part bubble ledger moved. Measured, in order:\n{}\nEither H_Part stopped \
         stranding spent internal wires — in which case this carrier's key can drop \
         `without_scalars` — or something changed how many it strands.",
        measured
            .iter()
            .map(|(name, lhs, rhs)| format!("  ({lhs}, {rhs}) {name}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn frobenius_morphism_battery() {
    run::<FrobeniusMorphism<char, String>>();
}

/// `FrobeniusMorphism`'s own `PartialEq` separates both sides of **all eleven**
/// equations — the measurement that forces [`frobenius_to_cospan`] to be the
/// decider for that carrier.
///
/// This is a record of a limitation, so it is written to go red when the
/// limitation *improves*: `two_layer_simplify`'s four rules normalize identity
/// layers, braiding pairs, `η`-into-`ε` loops and `Spider`-block chains, and
/// none of them fires on a Def 2.5 equation between the lettered generators.
/// Anything that widens it lowers this count and should be met by narrowing the
/// carrier's key, not by editing the number.
#[test]
fn frobenius_structural_equality_decides_nothing_here() {
    let separated: Vec<&str> = equations::<FrobeniusMorphism<char, String>>(Z)
        .into_iter()
        .filter(|(_, lhs, rhs)| lhs != rhs)
        .map(|(name, _, _)| name)
        .collect();
    assert_eq!(
        separated.len(),
        EQUATION_COUNT,
        "structural equality now decides {} of the {EQUATION_COUNT} equations; still separated: \
         {separated:?}",
        EQUATION_COUNT - separated.len()
    );
}

/// One place [`frobenius_to_cospan`] does not commute with `FrobeniusMorphism`'s
/// own composition, pinned with both values.
///
/// `two_layer_simplify`'s unit/counit rule deletes an `η` feeding an `ε`, which
/// is the *extra*-special axiom `η # ε = id_I`. `Cospan` is the theory of the
/// **special** monoids and keeps that closed bubble as a genuine scalar. So the
/// presented composite interprets to a cospan with **0** apex vertices while the
/// two interpretations composed in `Cospan` give **1**, all of it a scalar.
///
/// Narrow by construction: this is the `η # ε` pattern at one wire type, not a
/// survey of everything `two_layer_simplify` does — and "the only divergence"
/// is a claim this test cannot make. It was in fact false when this file was
/// written: see [`spider_fusion_needs_a_wire_between_the_two_spiders`], which
/// covers a second one, in the spider-fusion rule, that changed *connectivity*.
#[test]
fn frobenius_scalar_loop_is_erased_before_interpretation() {
    let eta = FrobeniusMorphism::<char, String>::unit(Z);
    let eps = FrobeniusMorphism::<char, String>::counit(Z);

    let mut presented = eta.clone();
    ComposableMutating::compose(&mut presented, eps.clone()).unwrap();
    let after_fm_compose = frobenius_to_cospan(&presented).unwrap().canonical_form();

    let semantic = Composable::compose(
        &frobenius_to_cospan(&eta).unwrap(),
        &frobenius_to_cospan(&eps).unwrap(),
    )
    .unwrap()
    .canonical_form();

    assert_eq!(
        (after_fm_compose.apex_len(), after_fm_compose.scalar_count()),
        (0, 0),
        "FrobeniusMorphism::compose no longer erases the eta;eps bubble — if that is deliberate, \
         this carrier is now sound for the special theory and the caveat on frobenius_to_cospan \
         should go"
    );
    assert_eq!(
        (semantic.apex_len(), semantic.scalar_count()),
        (1, 1),
        "Cospan stopped keeping the eta;eps scalar; it is the theory of *special*, not \
         extra-special, commutative Frobenius monoids"
    );
    assert_ne!(
        after_fm_compose, semantic,
        "the two routes agree now, so the documented non-commutation is stale"
    );
}

/// The normalizer's spider-fusion rule must not fuse across **zero** wires.
///
/// `Spider(z, m, n) ; Spider(z, n, k) = Spider(z, m, k)` is the spider theorem,
/// and it fuses the two spiders *along the n wires that join them*. At `n = 0`
/// there are no such wires: `Spider(z, 2, 0) ; Spider(z, 0, 2)` is the monoidal
/// product of a sink and a source, two components with nothing between them.
/// Fusing it to `Spider(z, 2, 2)` connects them — a change of **connectivity**,
/// not of a scalar, and reachable from the public API (`FrobeniusOperation` and
/// its `From` impl are both `pub`).
///
/// Rule 4 carries an `n >= 1` guard for exactly this, and the pre-fix tree was
/// fully green without it, which is why this test exists rather than a remark.
///
/// ⚠ **What this pins is the conjunction of the two defenses, not either one.**
/// Rule 4's `&& *n1 > 0` and the `target_size() > 0` filter on the
/// `target_side_placement` lookup are redundant with each other; measured,
/// deleting either alone leaves `cargo test -p catgraph` green and only
/// deleting both reddens this test. Read a MISSED cargo-mutants score on either
/// single deletion as accurate rather than stale.
///
/// The oracle is the semantics: interpret each factor and compose in `Cospan`,
/// versus interpret the presented composite. Both are named below, so a failure
/// says which side moved.
///
/// Scope: one wire type, the arities `(2, 0)` and `(0, 2)`. It says nothing
/// about the rule at `n >= 1`, which `frobenius_laws.rs::spider_fusion` pins.
#[test]
fn spider_fusion_needs_a_wire_between_the_two_spiders() {
    use catgraph::frobenius::FrobeniusOperation;
    type Fm = FrobeniusMorphism<char, String>;

    let sink: Fm = FrobeniusOperation::Spider(Z, 2, 0).into();
    let source: Fm = FrobeniusOperation::Spider(Z, 0, 2).into();

    let mut presented = sink.clone();
    ComposableMutating::compose(&mut presented, source.clone())
        .expect("[z, z] -> [] then [] -> [z, z] is composable");

    let after_fm_compose = frobenius_to_cospan(&presented)
        .expect("no black boxes here")
        .canonical_form();
    let semantic = Composable::compose(
        &frobenius_to_cospan(&sink).expect("no black boxes here"),
        &frobenius_to_cospan(&source).expect("no black boxes here"),
    )
    .expect("the two generator cospans compose")
    .canonical_form();

    // Two disjoint classes: {dom [0, 1], cod []} and {dom [], cod [0, 1]}.
    assert_eq!(
        semantic.classes().len(),
        2,
        "Cospan stopped keeping the sink and the source apart; got {semantic:?}"
    );

    // The connectivity claim goes first, so that reverting both defenses (Rule
    // 4's `*n1 > 0` guard and the zero-output filter on the lookup — either
    // alone leaves the crate green, see the docstring) fails *here* — on the
    // semantics — and not merely on the presentation shape. Measured with both
    // removed: the presented composite becomes the single block
    // `Spider('z', 2, 2)` and interprets to ONE class, {dom [0, 1], cod [0, 1]},
    // against the semantics' two.
    assert_eq!(
        after_fm_compose,
        semantic,
        "FrobeniusMorphism::compose fused two spiders that share no wire. The presented \
         composite interprets to {} apex class(es) where the semantics has {}; one class means \
         the sink's inputs and the source's outputs were wired together. Presentation: \
         {presented:?}",
        after_fm_compose.classes().len(),
        semantic.classes().len()
    );

    // And the presentation itself: the two components stay two layers.
    assert_eq!(
        presented.depth(),
        2,
        "the sink and the source fused into one block. Presentation: {presented:?}"
    );
}

/// `frobenius_to_cospan` on the six lettered generators is the generator cospan
/// the trait impl on `Cospan` names — the base case the battery's FM row rests
/// on, so it is pinned separately from the equations that consume it.
///
/// Ranges over the four Frobenius generators plus the identity and the
/// same-type braiding at one wire type; it says nothing about `Spider(z, m, n)`
/// at other arities or about heterogeneous braidings.
#[test]
fn frobenius_to_cospan_agrees_with_the_cospan_generators() {
    type Fm = FrobeniusMorphism<char, String>;

    let pairs: [(&str, Fm, Cospan<char>); 5] = [
        ("unit", Fm::unit(Z), Cospan::unit(Z)),
        ("counit", Fm::counit(Z), Cospan::counit(Z)),
        (
            "multiplication",
            Fm::multiplication(Z),
            Cospan::multiplication(Z),
        ),
        (
            "comultiplication",
            Fm::comultiplication(Z),
            Cospan::comultiplication(Z),
        ),
        (
            "identity",
            <Fm as HasIdentity<Vec<char>>>::identity(&vec![Z]),
            <Cospan<char> as HasIdentity<Vec<char>>>::identity(&vec![Z]),
        ),
    ];

    for (name, fm, cospan) in pairs {
        assert_eq!(
            frobenius_to_cospan(&fm).unwrap().canonical_form(),
            cospan.canonical_form(),
            "{name}: the FrobeniusMorphism generator and the Cospan generator disagree"
        );
    }

    // The braiding, whose two carriers build it through different code paths.
    assert_eq!(
        frobenius_to_cospan(&<Fm as Carrier>::swap(Z))
            .unwrap()
            .canonical_form(),
        <Cospan<char> as Carrier>::swap(Z).canonical_form(),
        "braiding: the FrobeniusMorphism braiding and the Cospan braiding disagree"
    );
}

/// The braiding the (co)commutativity equations use is a genuine crossing.
///
/// **Why this exists.** μ and δ are symmetric, so `σ ; μ == μ` and
/// `δ ; σ == δ` hold for *any* `σ: [z, z] → [z, z]` — the identity included.
/// Replacing [`Carrier::swap`] with `from_permutation_on_domain(identity(2), ..)`
/// was measured to leave all four carrier batteries green, so those two
/// equations say nothing about braiding on their own. This closes that:
///
/// 1. σ differs from the identity on two wires of the *same* type, which is the
///    exact mutant above;
/// 2. σ ; σ is the identity, so it is a crossing and not junk;
/// 3. on two *different* types it exchanges the words, `['a','b'] → ['b','a']`.
///
/// It ranges over 2-wire braidings only; wider permutations are
/// `catgraph-applied`'s `braiding_cross_carrier.rs`.
fn braiding_is_a_genuine_crossing<C: Carrier>() {
    let sigma = C::swap(Z);
    let id_two = C::id(&[Z, Z]);

    assert_ne!(
        sigma.key(),
        id_two.key(),
        "[{}] the braiding on two same-typed wires is the identity, so `sigma ; mu == mu` and \
         `delta ; sigma == delta` are vacuous for this carrier",
        C::NAME
    );
    assert_eq!(
        sigma.seq(&sigma).key(),
        id_two.key(),
        "[{}] sigma ; sigma != id: the braiding is not an involution",
        C::NAME
    );

    let hetero = C::from_permutation_on_domain(Permutation::transposition(2, 0, 1), &['a', 'b'])
        .expect("a 2-element transposition matches a 2-wire word");
    assert_eq!(
        hetero.dom(),
        vec!['a', 'b'],
        "[{}] braiding domain",
        C::NAME
    );
    assert_eq!(
        hetero.cod(),
        vec!['b', 'a'],
        "[{}] the braiding did not exchange the two wire types",
        C::NAME
    );
}

#[test]
fn braiding_is_a_genuine_crossing_per_carrier() {
    braiding_is_a_genuine_crossing::<Cospan<char>>();
    braiding_is_a_genuine_crossing::<Corel<char>>();
    braiding_is_a_genuine_crossing::<CospanAlgebraMorphism<PartitionAlgebra, char>>();
    braiding_is_a_genuine_crossing::<FrobeniusMorphism<char, String>>();
}

/// `Cospan`'s braiding, spelled out: wire 0 lands on the apex vertex the
/// codomain reads back at slot 1, and wire 1 at slot 0.
///
/// The generic pin above compares σ against the identity and against itself; it
/// would still pass on a σ that crossed *and* did something else. This names the
/// whole value, for the one carrier the others are checked against.
#[test]
fn cospan_braiding_canonical_form_is_the_crossing() {
    let sigma = <Cospan<char> as Carrier>::swap(Z);
    let expected = CospanCanon::from_parts(
        2,
        2,
        vec![
            ApexClass::new(Z, vec![0], vec![1]),
            ApexClass::new(Z, vec![1], vec![0]),
        ],
    )
    .expect("hand-built crossing is a valid canonical form");
    assert_eq!(sigma.canonical_form(), expected);
}

/// The two snake equations on the **derived** cup and cap, for every carrier.
///
/// Def 2.12's self-dual compact closed structure is what
/// [`HypergraphCategory::cup`] and [`HypergraphCategory::cap`] exist for, and
/// neither appears in the eleven equations above — they are built from η # δ and
/// μ # ε, so a cup that forgot to compose (two disconnected wires instead of one
/// bent one) leaves every Def 2.5 equation intact. Both snakes are decided by
/// the carrier's own key, like everything else here.
///
/// Ranges over one wire type and the two snakes; it does not check cup/cap on
/// multi-wire words (`compact_closed::cup`/`cap` build those by tensoring, and
/// are pinned in `tests/compact_closed.rs`).
fn zigzags<C: Carrier>() {
    let z = Z;
    let cup = C::cup(z).expect("cup is defined for every carrier in the battery");
    let cap = C::cap(z).expect("cap is defined for every carrier in the battery");
    let id = C::id(&[z]);

    let left_snake = cup.par(&id).seq(&id.par(&cap));
    assert_eq!(
        left_snake.key(),
        id.key(),
        "[{}] left snake (cup (x) id) ; (id (x) cap) != id",
        C::NAME
    );

    let right_snake = id.par(&cup).seq(&cap.par(&id));
    assert_eq!(
        right_snake.key(),
        id.key(),
        "[{}] right snake (id (x) cup) ; (cap (x) id) != id",
        C::NAME
    );
}

#[test]
fn zigzag_identities_per_carrier() {
    zigzags::<Cospan<char>>();
    zigzags::<Corel<char>>();
    zigzags::<CospanAlgebraMorphism<PartitionAlgebra, char>>();
    zigzags::<FrobeniusMorphism<char, String>>();
}

/// A black box has no cospan, and `frobenius_to_cospan` says so instead of
/// inventing one.
#[test]
fn frobenius_to_cospan_rejects_black_boxes() {
    use catgraph::frobenius::FrobeniusOperation;

    let boxed: FrobeniusMorphism<char, String> =
        FrobeniusOperation::UnSpecifiedBox("f".to_string(), vec![Z], vec![Z, Z]).into();
    let error = frobenius_to_cospan(&boxed).expect_err("a black box denotes nothing");
    assert!(
        format!("{error}").contains("UnSpecifiedBox"),
        "the error should name what could not be interpreted, got: {error}"
    );
}
