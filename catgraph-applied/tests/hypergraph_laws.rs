//! Fong–Spivak Thm 6.77 / Def 2.12 for the two `catgraph-applied` hypergraph
//! categories, and the Def 2.12 generator table across all six carriers in the
//! workspace.
//!
//! # What each part ranges over
//!
//! - [`generator_partitions_match_the_hand_anchored_table`] — the six Frobenius
//!   generators (η, ε, μ, δ, cup, cap) on one wire type (`'z'`), for the six
//!   types implementing
//!   [`HypergraphCategory`](catgraph::hypergraph_category::HypergraphCategory)
//!   (`rg -n 'HypergraphCategory<' --type rust catgraph/src catgraph-applied/src`,
//!   2026-09-02: `catgraph/src/hypergraph_category.rs:89` and `:135`,
//!   `catgraph/src/corel.rs:564`, `catgraph/src/equivalence.rs:564`,
//!   `catgraph-applied/src/decorated_cospan.rs:286`,
//!   `catgraph-applied/src/petri_net.rs:870`). Each generator is compared
//!   against a partition table written out by hand, never against another
//!   carrier. It does not range over several wire types, wider spiders, or any
//!   morphism that is not a generator.
//! - [`decorated_cospan_battery`] / [`petri_net_battery`] — Def 2.5's nine
//!   equations with unitality and counitality split into halves, eleven in
//!   total, on one wire type at the arities the equations force (≤ 3 wires).
//! - [`decorated_cospan_zigzags`] / [`petri_net_zigzags`] — the two snake
//!   equations on the derived cup and cap, one wire type.
//! - [`petri_net_battery_transition_ledger`] — the transition counts the
//!   `PetriNet` key quotients away, per equation side, against a hand-derived
//!   table.
//!
//! The same eleven equations on `Cospan`, `Corel`, `CospanAlgebraMorphism` and
//! `FrobeniusMorphism` are `catgraph/tests/frobenius_axioms.rs`.

use catgraph::category::{Composable, HasIdentity};
use catgraph::corel::Corel;
use catgraph::cospan::Cospan;
use catgraph::cospan_algebra::PartitionAlgebra;
use catgraph::cospan_canon::CospanCanon;
use catgraph::equivalence::CospanAlgebraMorphism;
use catgraph::frobenius::{FrobeniusMorphism, frobenius_to_cospan};
use catgraph::hypergraph_category::HypergraphCategory;
use catgraph_applied::decorated_cospan::DecoratedCospan;
use catgraph_applied::petri_net::{PetriDecoration, PetriNet, Transition};
use permutations::Permutation;

/// The wire type every equation and generator is built on.
const Z: char = 'z';

type Decorated = DecoratedCospan<char, PetriDecoration<char>>;

// ---------------------------------------------------------------------------
// 1. The Def 2.12 generator table, by hand, and the six carriers against it
// ---------------------------------------------------------------------------

/// The partition a morphism induces on its boundary slots: one
/// `(domain slots, codomain slots)` pair per apex class, sorted.
type Partition = Vec<(Vec<usize>, Vec<usize>)>;

/// `(domain arity, codomain arity, partition)` — the whole comparison value.
type Shape = (usize, usize, Partition);

/// Read a [`Shape`] off a canonical form, dropping scalar (bubble) classes.
///
/// Bubbles carry no boundary slot, so they cannot change the partition; the
/// `CospanAlgebraMorphism` row is the one that has any, and their exact count
/// is measured in `catgraph/tests/frobenius_axioms.rs`'s
/// `cospan_algebra_morphism_bubble_ledger`.
fn shape(canon: &CospanCanon<char>) -> Shape {
    let mut partition: Partition = canon
        .classes()
        .iter()
        .filter(|class| !class.is_scalar())
        .map(|class| (class.dom_preimage().to_vec(), class.cod_preimage().to_vec()))
        .collect();
    partition.sort();
    (canon.dom_len(), canon.cod_len(), partition)
}

/// Def 2.12's four generators and the two derived compact-closed morphisms,
/// each as the single spider connecting all of its wires.
///
/// Written out rather than computed: this is the anchor the six carriers are
/// compared against, and nothing in the crate produces it.
fn hand_anchored_generator_table() -> Vec<(&'static str, Shape)> {
    vec![
        ("unit", (0, 1, vec![(vec![], vec![0])])),
        ("counit", (1, 0, vec![(vec![0], vec![])])),
        ("multiplication", (2, 1, vec![(vec![0, 1], vec![0])])),
        ("comultiplication", (1, 2, vec![(vec![0], vec![0, 1])])),
        ("cup", (0, 2, vec![(vec![], vec![0, 1])])),
        ("cap", (2, 0, vec![(vec![0, 1], vec![])])),
    ]
}

/// The six generators of one carrier, in the table's order.
fn generator_shapes<C, F>(to_shape: F) -> Vec<Shape>
where
    C: HypergraphCategory<char>,
    F: Fn(&C) -> Shape,
{
    let generators = [
        C::unit(Z),
        C::counit(Z),
        C::multiplication(Z),
        C::comultiplication(Z),
        C::cup(Z).expect("cup is defined on every carrier here"),
        C::cap(Z).expect("cap is defined on every carrier here"),
    ];
    generators.iter().map(to_shape).collect()
}

/// `CospanAlgebraMorphism`'s element is a *structural* cospan
/// `[] → apex ← domain ⊕ codomain`, so its own canonical form has an empty
/// domain and a codomain of `|dom| + |cod|` slots. The morphism's boundary
/// partition is read by splitting that leg at `m.domain().len()`.
fn cospan_algebra_shape(m: &CospanAlgebraMorphism<PartitionAlgebra, char>) -> Shape {
    let n_dom = Composable::domain(m).len();
    let n_cod = Composable::codomain(m).len();
    let right = m.element().right_to_middle();
    assert_eq!(
        right.len(),
        n_dom + n_cod,
        "the element's interface is domain ⊕ codomain"
    );

    let mut partition: Partition = (0..m.element().middle().len())
        .map(|apex| {
            (
                (0..n_dom).filter(|i| right[*i] == apex).collect::<Vec<_>>(),
                (0..n_cod)
                    .filter(|k| right[n_dom + k] == apex)
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(dom, cod)| !(dom.is_empty() && cod.is_empty()))
        .collect();
    partition.sort();
    (n_dom, n_cod, partition)
}

/// Every `HypergraphCategory` implementor realizes Def 2.12's generators as
/// the hand-written partition table above.
#[test]
fn generator_partitions_match_the_hand_anchored_table() {
    let table = hand_anchored_generator_table();

    let rows: Vec<(&'static str, Vec<Shape>)> = vec![
        (
            "Cospan<char>",
            generator_shapes::<Cospan<char>, _>(|c| shape(&c.canonical_form())),
        ),
        (
            "Corel<char>",
            generator_shapes::<Corel<char>, _>(|c| shape(&c.as_cospan().canonical_form())),
        ),
        (
            "CospanAlgebraMorphism<PartitionAlgebra, char>",
            generator_shapes::<CospanAlgebraMorphism<PartitionAlgebra, char>, _>(
                cospan_algebra_shape,
            ),
        ),
        (
            "FrobeniusMorphism<char, String>",
            generator_shapes::<FrobeniusMorphism<char, String>, _>(|f| {
                shape(
                    &frobenius_to_cospan(f)
                        .expect("the generators build no black boxes")
                        .canonical_form(),
                )
            }),
        ),
        (
            "DecoratedCospan<char, PetriDecoration<char>>",
            generator_shapes::<Decorated, _>(|d| shape(&d.cospan.canonical_form())),
        ),
        (
            "PetriNet<char>",
            generator_shapes::<PetriNet<char>, _>(|n| {
                shape(&n.to_decorated_cospan().cospan.canonical_form())
            }),
        ),
    ];
    assert_eq!(rows.len(), 6, "one row per HypergraphCategory implementor");

    for (name, shapes) in rows {
        assert_eq!(shapes.len(), table.len(), "[{name}] generator count");
        for ((generator, expected), observed) in table.iter().zip(shapes) {
            assert_eq!(
                &observed, expected,
                "[{name}] {generator}: Def 2.12 partition"
            );
        }
    }
}

/// The four cospan-family carriers and `FrobeniusMorphism` build their
/// generators with no bubble classes at all, so
/// [`generator_partitions_match_the_hand_anchored_table`]'s scalar filter
/// discards nothing on those five rows.
///
/// `CospanAlgebraMorphism` is deliberately absent: its μ and δ carry bubbles by
/// construction, ledgered in `catgraph/tests/frobenius_axioms.rs`.
#[test]
fn the_five_bubble_free_rows_have_no_scalars() {
    fn assert_no_scalars<C, F>(name: &str, to_canon: F)
    where
        C: HypergraphCategory<char>,
        F: Fn(&C) -> CospanCanon<char>,
    {
        for (generator, morphism) in [
            ("unit", C::unit(Z)),
            ("counit", C::counit(Z)),
            ("multiplication", C::multiplication(Z)),
            ("comultiplication", C::comultiplication(Z)),
            ("cup", C::cup(Z).expect("cup is defined")),
            ("cap", C::cap(Z).expect("cap is defined")),
        ] {
            assert_eq!(
                to_canon(&morphism).scalar_count(),
                0,
                "[{name}] {generator} grew a bubble"
            );
        }
    }

    assert_no_scalars::<Cospan<char>, _>("Cospan<char>", Cospan::canonical_form);
    assert_no_scalars::<Corel<char>, _>("Corel<char>", |c| c.as_cospan().canonical_form());
    assert_no_scalars::<FrobeniusMorphism<char, String>, _>(
        "FrobeniusMorphism<char, String>",
        |f| {
            frobenius_to_cospan(f)
                .expect("the generators build no black boxes")
                .canonical_form()
        },
    );
    assert_no_scalars::<Decorated, _>("DecoratedCospan<char, PetriDecoration<char>>", |d| {
        d.cospan.canonical_form()
    });
    assert_no_scalars::<PetriNet<char>, _>("PetriNet<char>", |n| {
        n.to_decorated_cospan().cospan.canonical_form()
    });
}

// ---------------------------------------------------------------------------
// 2. The eleven Def 2.5 equations on the two applied carriers
// ---------------------------------------------------------------------------

/// How many equations [`equations`] returns: Def 2.5's nine, with unitality and
/// counitality split into halves.
const EQUATION_COUNT: usize = 11;

/// A carrier under test: a hypergraph category plus a decider for equality of
/// two parallel morphisms.
///
/// `par` consumes both operands rather than cloning them: `DecoratedCospan`'s
/// derived `Clone` demands `D: Clone`, which no shipped decoration marker
/// satisfies ([#348](https://github.com/sustia-llc/catgraph/issues/348)).
trait Carrier: HypergraphCategory<char> + Sized {
    const NAME: &'static str;

    /// The comparison value: equality of keys is the equation's verdict.
    type Key: PartialEq + std::fmt::Debug;

    fn seq(&self, other: &Self) -> Self;
    fn key(&self) -> Self::Key;
    fn dom(&self) -> Vec<char>;
    fn cod(&self) -> Vec<char>;

    fn par(mut self, other: Self) -> Self {
        self.monoidal(other);
        self
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

impl Carrier for Decorated {
    const NAME: &'static str = "DecoratedCospan<char, PetriDecoration<char>>";
    /// The whole value: the underlying cospan up to apex isomorphism, and the
    /// decoration on the nose.
    type Key = (CospanCanon<char>, Vec<Transition>);

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("battery composites are type-correct by hand")
    }
    fn key(&self) -> Self::Key {
        (self.cospan.canonical_form(), self.decoration.clone())
    }
    fn dom(&self) -> Vec<char> {
        Composable::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        Composable::codomain(self)
    }
}

impl Carrier for PetriNet<char> {
    const NAME: &'static str = "PetriNet<char>";
    /// The underlying cospan up to apex isomorphism.
    ///
    /// The decoration is **not** part of the key: a `PetriNet` generator is
    /// `PetriNet::from_cospan` of the corresponding `Cospan` generator, which
    /// carries one transition, and composition concatenates transition lists —
    /// so the two sides of an equation relating a three-generator composite to
    /// a bare identity hold different transition lists by construction. That
    /// difference is measured, per equation and per side, in
    /// [`petri_net_battery_transition_ledger`].
    type Key = CospanCanon<char>;

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("battery composites are type-correct by hand")
    }
    fn key(&self) -> Self::Key {
        self.to_decorated_cospan().cospan.canonical_form()
    }
    fn dom(&self) -> Vec<char> {
        Composable::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        Composable::codomain(self)
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
            mu().par(id()).seq(&mu()),
            id().par(mu()).seq(&mu()),
        ),
        (
            "left unitality: (eta (x) id) ; mu == id",
            eta().par(id()).seq(&mu()),
            id(),
        ),
        (
            "right unitality: (id (x) eta) ; mu == id",
            id().par(eta()).seq(&mu()),
            id(),
        ),
        ("commutativity: sigma ; mu == mu", sigma().seq(&mu()), mu()),
        // --- cocommutative comonoid (δ, ε) ---
        (
            "coassociativity: delta ; (delta (x) id) == delta ; (id (x) delta)",
            delta().seq(&delta().par(id())),
            delta().seq(&id().par(delta())),
        ),
        (
            "left counitality: delta ; (eps (x) id) == id",
            delta().seq(&eps().par(id())),
            id(),
        ),
        (
            "right counitality: delta ; (id (x) eps) == id",
            delta().seq(&id().par(eps())),
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
            delta().par(id()).seq(&id().par(mu())),
            mu().seq(&delta()),
        ),
        (
            "Frobenius right: (id (x) delta) ; (mu (x) id) == mu ; delta",
            id().par(delta()).seq(&mu().par(id())),
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
fn decorated_cospan_battery() {
    run::<Decorated>();
}

/// Every composite in the `DecoratedCospan` battery carries the empty
/// decoration, so the decoration half of its key decides nothing here.
///
/// Stated rather than left implicit: the `PetriDecoration` generators are
/// `D::empty(_)`, `combine` concatenates and `pushforward` maps an empty list
/// to an empty list, so [`decorated_cospan_battery`]'s verdict rests on the
/// cospan half alone. A decoration that stopped being empty would show up
/// here, not there.
#[test]
fn decorated_cospan_battery_decorations_stay_empty() {
    for (name, lhs, rhs) in equations::<Decorated>(Z) {
        assert!(lhs.decoration.is_empty(), "{name}: lhs decoration");
        assert!(rhs.decoration.is_empty(), "{name}: rhs decoration");
    }
}

#[test]
fn petri_net_battery() {
    run::<PetriNet<char>>();
}

/// The transition counts [`Carrier::key`] leaves out of the `PetriNet`
/// verdict, per equation and per side.
///
/// Each of η, ε, μ, δ and `id` is `PetriNet::from_cospan` of a `Cospan`
/// generator and so carries exactly one transition; `monoidal` and `compose`
/// both concatenate the lists, and the braiding σ carries none. So a term's
/// count is its number of generator occurrences — which is what the table
/// below writes out, and what an equation relating a composite to a bare
/// identity necessarily breaks.
#[test]
fn petri_net_battery_transition_ledger() {
    let expected: Vec<(&'static str, usize, usize)> = vec![
        ("associativity: (mu (x) id) ; mu == (id (x) mu) ; mu", 3, 3),
        ("left unitality: (eta (x) id) ; mu == id", 3, 1),
        ("right unitality: (id (x) eta) ; mu == id", 3, 1),
        ("commutativity: sigma ; mu == mu", 1, 1),
        (
            "coassociativity: delta ; (delta (x) id) == delta ; (id (x) delta)",
            3,
            3,
        ),
        ("left counitality: delta ; (eps (x) id) == id", 3, 1),
        ("right counitality: delta ; (id (x) eps) == id", 3, 1),
        ("cocommutativity: delta ; sigma == delta", 1, 1),
        (
            "Frobenius left: (delta (x) id) ; (id (x) mu) == mu ; delta",
            4,
            2,
        ),
        (
            "Frobenius right: (id (x) delta) ; (mu (x) id) == mu ; delta",
            4,
            2,
        ),
        ("speciality: delta ; mu == id", 2, 1),
    ];
    assert_eq!(expected.len(), EQUATION_COUNT);

    for ((name, lhs, rhs), (expected_name, want_lhs, want_rhs)) in
        equations::<PetriNet<char>>(Z).into_iter().zip(expected)
    {
        assert_eq!(name, expected_name, "the ledger rows drifted out of order");
        assert_eq!(
            (lhs.transition_count(), rhs.transition_count()),
            (want_lhs, want_rhs),
            "{name}: transition counts"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The snake equations on the derived cup and cap
// ---------------------------------------------------------------------------

/// Both snakes on one carrier, decided by that carrier's own key.
///
/// Neither cup nor cap appears in the eleven equations — they are built from
/// η # δ and μ # ε — so a cup that forgot to compose leaves the battery
/// intact.
fn zigzags<C: Carrier>() {
    let cup = || C::cup(Z).expect("cup is defined for every carrier in the battery");
    let cap = || C::cap(Z).expect("cap is defined for every carrier in the battery");
    let id = || C::id(&[Z]);

    let left_snake = cup().par(id()).seq(&id().par(cap()));
    assert_eq!(
        left_snake.key(),
        id().key(),
        "[{}] left snake (cup (x) id) ; (id (x) cap) != id",
        C::NAME
    );

    let right_snake = id().par(cup()).seq(&cap().par(id()));
    assert_eq!(
        right_snake.key(),
        id().key(),
        "[{}] right snake (id (x) cup) ; (cap (x) id) != id",
        C::NAME
    );
}

#[test]
fn decorated_cospan_zigzags() {
    zigzags::<Decorated>();
}

#[test]
fn petri_net_zigzags() {
    zigzags::<PetriNet<char>>();
}

// ---------------------------------------------------------------------------
// 4. The braiding is a genuine crossing on both applied carriers
// ---------------------------------------------------------------------------

/// σ on two same-typed wires is not the identity, is an involution, and
/// exchanges two *different* wire types.
///
/// Without the first assertion the battery's `sigma ; mu == mu` and
/// `delta ; sigma == delta` rows are satisfied by σ = id.
fn braiding_is_a_genuine_crossing<C: Carrier>() {
    let sigma = C::swap(Z);
    let id_two = C::id(&[Z, Z]);

    assert_ne!(
        sigma.key(),
        id_two.key(),
        "[{}] the braiding on two same-typed wires is the identity",
        C::NAME
    );
    assert_eq!(
        sigma.seq(&sigma).key(),
        id_two.key(),
        "[{}] sigma ; sigma != id",
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
fn braiding_is_a_genuine_crossing_on_the_applied_carriers() {
    braiding_is_a_genuine_crossing::<Decorated>();
    braiding_is_a_genuine_crossing::<PetriNet<char>>();
}

// ---------------------------------------------------------------------------
// 5. The bridge the `PetriNet` row is decided through
// ---------------------------------------------------------------------------

/// `PetriNet::to_decorated_cospan` and `PetriNet::from_decorated_cospan` are
/// mutually inverse on places, both legs and the transition list, and the
/// generators of the two carriers agree as cospans.
///
/// **What this ranges over.** The six Def 2.12 generators plus the identity on
/// `['z','z']`, one wire type. It does not sweep arbitrary nets.
#[test]
fn petri_net_and_decorated_cospan_agree_through_the_bridge() {
    let nets: Vec<(&str, PetriNet<char>)> = vec![
        ("unit", PetriNet::unit(Z)),
        ("counit", PetriNet::counit(Z)),
        ("multiplication", PetriNet::multiplication(Z)),
        ("comultiplication", PetriNet::comultiplication(Z)),
        ("cup", PetriNet::cup(Z).expect("cup is defined")),
        ("cap", PetriNet::cap(Z).expect("cap is defined")),
        (
            "identity",
            <PetriNet<char> as HasIdentity<Vec<char>>>::identity(&vec![Z, Z]),
        ),
    ];

    let decorated: Vec<(&str, Decorated)> = vec![
        ("unit", Decorated::unit(Z)),
        ("counit", Decorated::counit(Z)),
        ("multiplication", Decorated::multiplication(Z)),
        ("comultiplication", Decorated::comultiplication(Z)),
        ("cup", Decorated::cup(Z).expect("cup is defined")),
        ("cap", Decorated::cap(Z).expect("cap is defined")),
        (
            "identity",
            <Decorated as HasIdentity<Vec<char>>>::identity(&vec![Z, Z]),
        ),
    ];

    for ((name, net), (decorated_name, other)) in nets.iter().zip(&decorated) {
        assert_eq!(name, decorated_name);

        let round = PetriNet::from_decorated_cospan(net.to_decorated_cospan());
        assert_eq!(round.places(), net.places(), "{name}: places");
        assert_eq!(round.left_to_place(), net.left_to_place(), "{name}: left");
        assert_eq!(
            round.right_to_place(),
            net.right_to_place(),
            "{name}: right"
        );
        assert_eq!(
            round.transitions(),
            net.transitions(),
            "{name}: transitions"
        );

        assert_eq!(
            net.to_decorated_cospan().cospan.canonical_form(),
            other.cospan.canonical_form(),
            "{name}: the two carriers' generators are the same cospan"
        );
    }
}
