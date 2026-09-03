//! The core claim, end to end.
//!
//! Every `HypergraphCategory` implementor in `catgraph/src` — `Cospan`,
//! `Corel`, `CospanAlgebraMorphism<PartitionAlgebra, _>` and
//! `FrobeniusMorphism` (`rg -n '^impl.*HypergraphCategory<' catgraph/src` → 4)
//! — satisfies the eleven Def 2.5 equations, the Def 2.12 generator table and
//! both zigzags, decided by `CospanCanon` equality, over generator words of
//! length ≤ 3 and every permutation of ≤ 4 wires; `Cospan<char>` satisfies
//! strict left/right unitality under `Cospan`'s derived `PartialEq`; `compose`
//! on each equals a union-find partition reference computed from the operand
//! wirings; and `wiring(f ⊗ g) == wiring(f) ++ shift(wiring(g))` on the 13
//! public `Monoidal` implementors in `catgraph/src`.
//!
//! # Input space
//!
//! One wire type (`'z'`) except where a fixture names more; the exhaustive
//! `Cospan` corpus is every cospan with domain, codomain and apex each at most
//! 2 (`cospan_corpus`, size asserted below); the generator-word sweep is every
//! composable word of length 2 and 3 over the nine generators `eta eps mu delta
//! id1 id2 sigma cup cap`; the permutation sweep is every element of `S₀`
//! through `S₄`.
//!
//! # References
//!
//! Each Def 2.5 equation and each zigzag builds both sides from the carrier's
//! own generators and compares them — the reference is the equation. The Def
//! 2.12 generator table is hand-written `CospanCanon` values. The composition
//! and tensor claims compare against `catgraph_testutil::wiring`, whose
//! `pushout` and `shift_concat` are `Vec<usize>` code with no catgraph edge.
//!
//! # Reach
//!
//! The two `#[cfg(test)]` `Monoidal` implementors — `Defaulting`
//! (`frobenius/trait_impl.rs:241`) and `CospanBacked` (`:322`) — are not
//! reachable from an integration test. `FrobeniusLayer`
//! (`frobenius/operations.rs:206`) is `pub(crate)` and is reached only through
//! `FrobeniusMorphism`'s layer-wise `monoidal`. The `GenericMonoidalMorphism`
//! tensor row runs two two-layer operands, asserted at equal depth below, so
//! its assertions touch `GenericMonoidalMorphism::monoidal`'s equal-depth
//! pairing and not the identity padding it applies at unequal depth.
//!
//! # covers:
//!
//! `ApexClass` `CatgraphError` `Composable` `ComposableMutating` `Corel`
//! `Cospan` `CospanAlgebra` `CospanAlgebraMorphism` `CospanCanon`
//! `Decomposition` `FinSetMap` `FinSetMorphism` `FrobeniusMorphism`
//! `FrobeniusOperation` `GenericMonoidalMorphism`
//! `GenericMonoidalMorphismLayer` `HasIdentity` `HypergraphCategory` `Monoidal`
//! `NamedCospan` `OrderPresInj` `OrderPresSurj` `PartitionAlgebra` `Rel` `Span`
//! `SymmetricMonoidalDiscreteMorphism` `SymmetricMonoidalMorphism`
//!
//! # not-covered:
//!
//! `BoundaryLeg` `Contains` `CospanToFrobeniusFunctor` `EitherExt` `Frobenius`
//! `HypergraphFunctor` `InterpretableMorphism` `MonoidalMorphism`
//! `MonoidalMutatingMorphism` `MorphismSystem` `NameAlgebra` `Operadic`
//! `RelabelingFunctor` `ResultExt` `TestContainer` `TestMorphism` `TestSystem`
//! `TryFromFinSetError` `TryFromInjError` `TryFromSurjError`

use std::sync::Arc;

use catgraph::{
    category::{Composable, ComposableMutating, HasIdentity},
    corel::Corel,
    cospan::Cospan,
    cospan_algebra::PartitionAlgebra,
    cospan_canon::{ApexClass, CospanCanon},
    equivalence::CospanAlgebraMorphism,
    finset::{Decomposition, FinSetMorphism, OrderPresInj, OrderPresSurj},
    frobenius::{FrobeniusMorphism, FrobeniusOperation, frobenius_to_cospan},
    hypergraph_category::HypergraphCategory,
    monoidal::{
        GenericMonoidalMorphism, GenericMonoidalMorphismLayer, Monoidal,
        SymmetricMonoidalDiscreteMorphism,
    },
    named_cospan::NamedCospan,
    span::{Rel, Span},
};
use catgraph_testutil::{
    all_perms,
    wiring::{CospanWiring, Leg, Wiring},
};
use permutations::Permutation;

/// The wire type every claim is built on unless a fixture names more.
const Z: char = 'z';

/// Def 2.5's nine equations, with unitality and counitality split into halves.
const EQUATION_COUNT: usize = 11;

/// The largest wire count the permutation sweep runs `Sₙ` over.
const MAX_WIRES: usize = 4;

// ---------------------------------------------------------------------------
// Wiring extraction
// ---------------------------------------------------------------------------

/// A cospan's `(apex labels, domain leg, codomain leg)`.
fn cospan_wiring(c: &Cospan<char>) -> CospanWiring<char> {
    CospanWiring::new(
        c.middle().to_vec(),
        c.left_to_middle().to_vec(),
        c.right_to_middle().to_vec(),
    )
    .expect("invariant: a Cospan's legs are in bounds of its apex")
}

/// The same wiring read off a canonical form, so the apex is in class order.
fn canon_wiring(canon: &CospanCanon<char>) -> CospanWiring<char> {
    let mut dom = vec![0usize; canon.dom_len()];
    let mut cod = vec![0usize; canon.cod_len()];
    let apex: Vec<char> = canon.classes().iter().map(|k| *k.label()).collect();
    for (class, k) in canon.classes().iter().enumerate() {
        for &index in k.dom_preimage() {
            dom[index] = class;
        }
        for &index in k.cod_preimage() {
            cod[index] = class;
        }
    }
    CospanWiring::new(apex, dom, cod)
        .expect("invariant: every preimage entry of a CospanCanon is a boundary index")
}

/// The wiring with the apex positions no leg reaches removed.
fn drop_scalars(w: &CospanWiring<char>) -> CospanWiring<char> {
    let mut reached = vec![false; w.apex().len()];
    for &target in w.dom().iter().chain(w.cod()) {
        reached[target] = true;
    }
    let mut renumbered = vec![0usize; w.apex().len()];
    let mut apex = Vec::new();
    for (position, &keep) in reached.iter().enumerate() {
        if keep {
            renumbered[position] = apex.len();
            apex.push(w.apex()[position]);
        }
    }
    CospanWiring::new(
        apex,
        w.dom().iter().map(|&v| renumbered[v]).collect(),
        w.cod().iter().map(|&v| renumbered[v]).collect(),
    )
    .expect("invariant: dropping unreached apex positions keeps every leg in bounds")
}

// ---------------------------------------------------------------------------
// Carriers
// ---------------------------------------------------------------------------

/// A hypergraph category together with the wiring its verdicts are read from.
trait Carrier: HypergraphCategory<char> + Clone + Sized {
    const NAME: &'static str;

    fn seq(&self, other: &Self) -> Self;
    fn wiring(&self) -> CospanWiring<char>;
    fn dom(&self) -> Vec<char>;
    fn cod(&self) -> Vec<char>;

    /// The comparison value: `CospanCanon` of the wiring.
    fn key(&self) -> CospanCanon<char> {
        let w = self.wiring();
        Cospan::new(w.dom().to_vec(), w.cod().to_vec(), w.apex().to_vec())
            .expect("invariant: a wiring's legs are in bounds of its apex")
            .canonical_form()
    }

    fn par(&self, other: &Self) -> Self {
        let mut answer = self.clone();
        answer.monoidal(other.clone());
        answer
    }

    fn id(types: &[char]) -> Self {
        <Self as HasIdentity<Vec<char>>>::identity(&types.to_vec())
    }

    fn braiding(p: &Permutation, types: &[char]) -> Self {
        Self::from_permutation_on_domain(p.clone(), types)
            .expect("invariant: the permutation length matches the wire word")
    }

    /// σ: `[z, z] → [z, z]`, the braiding on two equal wires.
    fn swap(z: char) -> Self {
        Self::braiding(&Permutation::transposition(2, 0, 1), &[z, z])
    }
}

impl Carrier for Cospan<char> {
    const NAME: &'static str = "Cospan<char>";

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("the sweep composes type-correct pairs")
    }
    fn wiring(&self) -> CospanWiring<char> {
        cospan_wiring(self)
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

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("the sweep composes type-correct pairs")
    }
    fn wiring(&self) -> CospanWiring<char> {
        cospan_wiring(self.as_cospan())
    }
    fn dom(&self) -> Vec<char> {
        Composable::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        Composable::codomain(self)
    }
}

impl Carrier for CospanAlgebraMorphism<PartitionAlgebra, char> {
    const NAME: &'static str = "CospanAlgebraMorphism<PartitionAlgebra, char>";

    fn seq(&self, other: &Self) -> Self {
        Composable::compose(self, other).expect("the sweep composes type-correct pairs")
    }

    /// The element is a cospan `[] → apex ← dom ⊕ cod`, so the morphism's two
    /// legs are its right leg split at `dom.len()`. Scalar classes are dropped:
    /// `multiplication_in` and `comultiplication_in` build a three-vertex apex
    /// only one vertex of which any leg reaches.
    fn wiring(&self) -> CospanWiring<char> {
        let element = self.element();
        assert!(
            element.left_to_middle().is_empty(),
            "[{}] the element stopped being a cospan out of the empty object",
            Self::NAME
        );
        let split = Composable::domain(self).len();
        let right = element.right_to_middle();
        let raw = CospanWiring::new(
            element.middle().to_vec(),
            right[..split].to_vec(),
            right[split..].to_vec(),
        )
        .expect("invariant: the element's right leg is in bounds of its apex");
        drop_scalars(&raw)
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

    fn seq(&self, other: &Self) -> Self {
        let mut answer = self.clone();
        ComposableMutating::compose(&mut answer, other.clone())
            .expect("the sweep composes type-correct pairs");
        answer
    }

    fn wiring(&self) -> CospanWiring<char> {
        canon_wiring(
            &frobenius_to_cospan(self)
                .expect("no arm here builds a black box, so every term interprets")
                .canonical_form(),
        )
    }

    fn dom(&self) -> Vec<char> {
        ComposableMutating::domain(self)
    }
    fn cod(&self) -> Vec<char> {
        ComposableMutating::codomain(self)
    }
}

// ---------------------------------------------------------------------------
// Def 2.5 — the eleven equations
// ---------------------------------------------------------------------------

/// The eleven equations as `(name, lhs, rhs)`, built from the carrier's own
/// generators. `;` reads left to right, so `f.seq(&g)` is "f then g".
fn equations<C: Carrier>(z: char) -> Vec<(&'static str, C, C)> {
    let eta = || C::unit(z);
    let eps = || C::counit(z);
    let mu = || C::multiplication(z);
    let delta = || C::comultiplication(z);
    let id = || C::id(&[z]);
    let sigma = || C::swap(z);

    let table = vec![
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
        ("speciality: delta ; mu == id", delta().seq(&mu()), id()),
    ];
    assert_eq!(
        table.len(),
        EQUATION_COUNT,
        "the equation table changed size without EQUATION_COUNT following it"
    );
    table
}

/// Every failure of the eleven on one carrier, with both keys.
fn equation_failures<C: Carrier>() -> Vec<String> {
    let mut failures = Vec::new();
    for (name, lhs, rhs) in equations::<C>(Z) {
        if lhs.dom() != rhs.dom() || lhs.cod() != rhs.cod() {
            failures.push(format!(
                "  {name}\n    lhs: {:?} -> {:?}\n    rhs: {:?} -> {:?}",
                lhs.dom(),
                lhs.cod(),
                rhs.dom(),
                rhs.cod()
            ));
            continue;
        }
        let (left, right) = (lhs.key(), rhs.key());
        if left != right {
            failures.push(format!("  {name}\n    lhs = {left:?}\n    rhs = {right:?}"));
        }
    }
    failures
}

/// The eleven Def 2.5 equations on all four carriers.
#[test]
fn def_2_5_equations_on_every_carrier() {
    let rows = [
        (
            <Cospan<char> as Carrier>::NAME,
            equation_failures::<Cospan<char>>(),
        ),
        (
            <Corel<char> as Carrier>::NAME,
            equation_failures::<Corel<char>>(),
        ),
        (
            <CospanAlgebraMorphism<PartitionAlgebra, char> as Carrier>::NAME,
            equation_failures::<CospanAlgebraMorphism<PartitionAlgebra, char>>(),
        ),
        (
            <FrobeniusMorphism<char, String> as Carrier>::NAME,
            equation_failures::<FrobeniusMorphism<char, String>>(),
        ),
    ];
    let report: Vec<String> = rows
        .iter()
        .filter(|(_, failures)| !failures.is_empty())
        .map(|(name, failures)| {
            format!(
                "[{name}] {} of {EQUATION_COUNT} failed:\n{}",
                failures.len(),
                failures.join("\n")
            )
        })
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));
}

/// No composite of the eleven grows a bubble on `Cospan`: the count is zero on
/// both sides of every equation, which equality alone would not say.
#[test]
fn cospan_equations_create_no_scalars() {
    for (name, lhs, rhs) in equations::<Cospan<char>>(Z) {
        assert_eq!(
            (lhs.key().scalar_count(), rhs.key().scalar_count()),
            (0, 0),
            "{name}: a Def 2.5 composite in Cospan grew a bubble"
        );
    }
}

/// The `Corel` row of the eleven is the `Cospan` row, key for key.
///
/// `Corel`'s generators are `new_unchecked(Cospan::…)` and its identity,
/// `monoidal` and `from_permutation_on_domain` delegate, so on these eleven
/// equations the two carriers compute one thing. Ranges over the eleven
/// equations at one wire type; it does not range over "`Corel` delegates" —
/// #351 overrode `Composable::compose` and this comparison stayed green,
/// because none of the eleven composites births a mid-composition bubble.
#[test]
fn corel_equations_recompute_the_cospan_equations() {
    let cospans = equations::<Cospan<char>>(Z);
    let corels = equations::<Corel<char>>(Z);
    assert_eq!(cospans.len(), corels.len(), "the two tables differ in size");

    for ((name, cospan_lhs, cospan_rhs), (_, corel_lhs, corel_rhs)) in
        cospans.into_iter().zip(corels)
    {
        assert_eq!(
            corel_lhs.key(),
            cospan_lhs.key(),
            "{name}: the Corel lhs is no longer the Cospan lhs"
        );
        assert_eq!(
            corel_rhs.key(),
            cospan_rhs.key(),
            "{name}: the Corel rhs is no longer the Cospan rhs"
        );
        assert!(
            corel_lhs.as_cospan().is_jointly_surjective()
                && corel_rhs.as_cospan().is_jointly_surjective(),
            "{name}: a side left the corelation subcategory"
        );
    }
}

/// The bubble count `CospanAlgebraMorphism`'s wiring drops, per equation side.
///
/// `multiplication_in` and `comultiplication_in` each build their structural
/// cospan over a three-vertex apex whose right leg is `[0, 0, 0]`, so each μ and
/// each δ in a term contributes two apex vertices no leg reaches and an identity
/// contributes none.
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
        (4, 4), // associativity — two μ each
        (2, 0), // left unitality — one μ against a bare identity
        (2, 0), // right unitality
        (2, 2), // commutativity
        (4, 4), // coassociativity — two δ each
        (2, 0), // left counitality
        (2, 0), // right counitality
        (2, 2), // cocommutativity
        (4, 4), // Frobenius left — one μ and one δ each side
        (4, 4), // Frobenius right
        (4, 0), // speciality — μ and δ against a bare identity
    ];

    let counts: Vec<(usize, usize)> = measured.iter().map(|&(_, lhs, rhs)| (lhs, rhs)).collect();
    assert_eq!(
        counts.as_slice(),
        expected.as_slice(),
        "the H_Part bubble ledger moved. Measured, in order:\n{}",
        measured
            .iter()
            .map(|(name, lhs, rhs)| format!("  ({lhs}, {rhs}) {name}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// `FrobeniusMorphism`'s own `PartialEq` separates both sides of all eleven
/// equations, which is what makes `frobenius_to_cospan` the decider for that
/// carrier.
#[test]
fn frobenius_structural_equality_decides_none_of_the_eleven() {
    let separated: Vec<&str> = equations::<FrobeniusMorphism<char, String>>(Z)
        .into_iter()
        .filter(|(_, lhs, rhs)| lhs != rhs)
        .map(|(name, _, _)| name)
        .collect();
    assert_eq!(
        separated.len(),
        EQUATION_COUNT,
        "structural equality now decides {} of the {EQUATION_COUNT} equations",
        EQUATION_COUNT - separated.len()
    );
}

// ---------------------------------------------------------------------------
// Def 2.12 — the generator table and the zigzags
// ---------------------------------------------------------------------------

/// A hand-written canonical form from `(dom_len, cod_len, classes)`.
fn canon(
    dom_len: usize,
    cod_len: usize,
    classes: &[(Vec<usize>, Vec<usize>)],
) -> CospanCanon<char> {
    let mut classes: Vec<ApexClass<char>> = classes
        .iter()
        .map(|(dom, cod)| ApexClass::new(Z, dom.clone(), cod.clone()))
        .collect();
    classes.sort();
    CospanCanon::from_parts(dom_len, cod_len, classes)
        .expect("the table's hand-written classes are well formed")
}

/// The Def 2.12 generator table: what each generator's wiring is, spelled out.
///
/// `(name, dom_len, cod_len, one entry per apex class as (domain preimage,
/// codomain preimage))`.
fn generator_table() -> Vec<(&'static str, CospanCanon<char>)> {
    vec![
        ("eta", canon(0, 1, &[(vec![], vec![0])])),
        ("eps", canon(1, 0, &[(vec![0], vec![])])),
        ("mu", canon(2, 1, &[(vec![0, 1], vec![0])])),
        ("delta", canon(1, 2, &[(vec![0], vec![0, 1])])),
        ("id", canon(1, 1, &[(vec![0], vec![0])])),
        (
            "sigma",
            canon(2, 2, &[(vec![0], vec![1]), (vec![1], vec![0])]),
        ),
        ("cup", canon(0, 2, &[(vec![], vec![0, 1])])),
        ("cap", canon(2, 0, &[(vec![0, 1], vec![])])),
    ]
}

/// The carrier's generators in the table's order.
fn generators_in_table_order<C: Carrier>() -> Vec<C> {
    vec![
        C::unit(Z),
        C::counit(Z),
        C::multiplication(Z),
        C::comultiplication(Z),
        C::id(&[Z]),
        C::swap(Z),
        C::cup(Z).expect("cup is defined for every carrier here"),
        C::cap(Z).expect("cap is defined for every carrier here"),
    ]
}

fn generator_table_failures<C: Carrier>() -> Vec<String> {
    let mut failures = Vec::new();
    for ((name, expected), built) in generator_table()
        .into_iter()
        .zip(generators_in_table_order::<C>())
    {
        let observed = built.key();
        if observed != expected {
            failures.push(format!(
                "  {name}\n    observed = {observed:?}\n    expected = {expected:?}"
            ));
        }
    }
    failures
}

/// Every carrier's eight Def 2.12 generators against the hand-written table.
#[test]
fn def_2_12_generator_table_on_every_carrier() {
    let rows = [
        (
            <Cospan<char> as Carrier>::NAME,
            generator_table_failures::<Cospan<char>>(),
        ),
        (
            <Corel<char> as Carrier>::NAME,
            generator_table_failures::<Corel<char>>(),
        ),
        (
            <CospanAlgebraMorphism<PartitionAlgebra, char> as Carrier>::NAME,
            generator_table_failures::<CospanAlgebraMorphism<PartitionAlgebra, char>>(),
        ),
        (
            <FrobeniusMorphism<char, String> as Carrier>::NAME,
            generator_table_failures::<FrobeniusMorphism<char, String>>(),
        ),
    ];
    let report: Vec<String> = rows
        .iter()
        .filter(|(_, failures)| !failures.is_empty())
        .map(|(name, failures)| format!("[{name}]\n{}", failures.join("\n")))
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));
}

fn zigzag_failures<C: Carrier>() -> Vec<String> {
    let cup = C::cup(Z).expect("cup is defined for every carrier here");
    let cap = C::cap(Z).expect("cap is defined for every carrier here");
    let id = C::id(&[Z]);
    let mut failures = Vec::new();

    let left = cup.par(&id).seq(&id.par(&cap));
    if left.key() != id.key() {
        failures.push(format!(
            "  left snake (cup (x) id) ; (id (x) cap)\n    observed = {:?}\n    expected = {:?}",
            left.key(),
            id.key()
        ));
    }
    let right = id.par(&cup).seq(&cap.par(&id));
    if right.key() != id.key() {
        failures.push(format!(
            "  right snake (id (x) cup) ; (cap (x) id)\n    observed = {:?}\n    expected = {:?}",
            right.key(),
            id.key()
        ));
    }
    failures
}

/// Both zigzags on all four carriers, at one wire type.
///
/// The eleven cannot see these: `cup` and `cap` are built from η # δ and μ # ε,
/// so a cup that never composed leaves every Def 2.5 equation intact.
#[test]
fn zigzag_identities_on_every_carrier() {
    let rows = [
        (
            <Cospan<char> as Carrier>::NAME,
            zigzag_failures::<Cospan<char>>(),
        ),
        (
            <Corel<char> as Carrier>::NAME,
            zigzag_failures::<Corel<char>>(),
        ),
        (
            <CospanAlgebraMorphism<PartitionAlgebra, char> as Carrier>::NAME,
            zigzag_failures::<CospanAlgebraMorphism<PartitionAlgebra, char>>(),
        ),
        (
            <FrobeniusMorphism<char, String> as Carrier>::NAME,
            zigzag_failures::<FrobeniusMorphism<char, String>>(),
        ),
    ];
    let report: Vec<String> = rows
        .iter()
        .filter(|(_, failures)| !failures.is_empty())
        .map(|(name, failures)| format!("[{name}]\n{}", failures.join("\n")))
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));
}

// ---------------------------------------------------------------------------
// Braiding — every permutation of at most four wires
// ---------------------------------------------------------------------------

/// The permutation `p` on `n` same-typed wires, as a hand-written canonical
/// form: `Cospan::from_permutation_on_domain`'s contract is that domain wire `i`
/// and codomain wire `p.apply(i)` share an apex vertex.
fn permutation_canon(p: &Permutation, n: usize) -> CospanCanon<char> {
    let classes: Vec<(Vec<usize>, Vec<usize>)> =
        (0..n).map(|i| (vec![i], vec![p.apply(i)])).collect();
    canon(n, n, &classes)
}

fn braiding_failures<C: Carrier>() -> Vec<String> {
    let mut failures = Vec::new();
    for n in 0..=MAX_WIRES {
        let word = vec![Z; n];
        for p in all_perms(n) {
            let built = C::braiding(&p, &word);
            let observed = built.key();
            let expected = permutation_canon(&p, n);
            if observed != expected {
                failures.push(format!(
                    "  n = {n}, p = {:?}\n    observed = {observed:?}\n    expected = {expected:?}",
                    (0..n).map(|i| p.apply(i)).collect::<Vec<_>>()
                ));
                continue;
            }
            let inverse = C::braiding(&p.inv(), &word);
            let round_trip = built.seq(&inverse).key();
            let identity = C::id(&word).key();
            if round_trip != identity {
                failures.push(format!(
                    "  n = {n}, p ; p^-1 != id\n    observed = {round_trip:?}\n    expected = \
                     {identity:?}"
                ));
            }
        }
    }
    failures
}

/// Every permutation of `0..n` for `n` up to four, on every carrier, against a
/// hand-written wiring, plus `σ_p ; σ_p⁻¹ == id`.
///
/// The (co)commutativity equations hold for any σ that μ and δ cannot tell from
/// the identity, so they say nothing about braiding on their own.
#[test]
fn braiding_is_the_permutation_wiring_on_every_carrier() {
    let rows = [
        (
            <Cospan<char> as Carrier>::NAME,
            braiding_failures::<Cospan<char>>(),
        ),
        (
            <Corel<char> as Carrier>::NAME,
            braiding_failures::<Corel<char>>(),
        ),
        (
            <CospanAlgebraMorphism<PartitionAlgebra, char> as Carrier>::NAME,
            braiding_failures::<CospanAlgebraMorphism<PartitionAlgebra, char>>(),
        ),
        (
            <FrobeniusMorphism<char, String> as Carrier>::NAME,
            braiding_failures::<FrobeniusMorphism<char, String>>(),
        ),
    ];
    let report: Vec<String> = rows
        .iter()
        .filter(|(_, failures)| !failures.is_empty())
        .map(|(name, failures)| format!("[{name}]\n{}", failures.join("\n")))
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));

    // Two different wire types: the braiding exchanges the words, on every
    // carrier.
    fn hetero_row<C: Carrier>() -> (&'static str, (Vec<char>, Vec<char>)) {
        let hetero = C::braiding(&Permutation::transposition(2, 0, 1), &['a', 'b']);
        (C::NAME, (Carrier::dom(&hetero), Carrier::cod(&hetero)))
    }
    let expected = (vec!['a', 'b'], vec!['b', 'a']);
    let hetero_report: Vec<String> = [
        hetero_row::<Cospan<char>>(),
        hetero_row::<Corel<char>>(),
        hetero_row::<CospanAlgebraMorphism<PartitionAlgebra, char>>(),
        hetero_row::<FrobeniusMorphism<char, String>>(),
    ]
    .into_iter()
    .filter(|(_, observed)| *observed != expected)
    .map(|(name, observed)| format!("  [{name}] observed = {observed:?}"))
    .collect();
    assert!(
        hetero_report.is_empty(),
        "the braiding did not exchange the two wire types, expected {expected:?}:\n{}",
        hetero_report.join("\n")
    );
}

// ---------------------------------------------------------------------------
// #290 — compose against the union-find partition reference
// ---------------------------------------------------------------------------

/// Every length-`len` word over `0..base`, in lexicographic order.
fn index_words(base: usize, len: usize) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new()];
    for _ in 0..len {
        let mut next = Vec::new();
        for word in &out {
            for value in 0..base {
                let mut extended = word.clone();
                extended.push(value);
                next.push(extended);
            }
        }
        out = next;
    }
    out
}

/// Every cospan over one wire type with domain, codomain and apex at most 2.
fn cospan_corpus() -> Vec<Cospan<char>> {
    let mut out = Vec::new();
    for apex in 0..=2usize {
        for dom in 0..=2usize {
            for cod in 0..=2usize {
                for left in index_words(apex, dom) {
                    for right in index_words(apex, cod) {
                        out.push(
                            Cospan::new(left.clone(), right, vec![Z; apex])
                                .expect("every corpus leg entry is below the apex size"),
                        );
                    }
                }
            }
        }
    }
    out
}

/// The corpus size, so a corpus that silently shrank is a failure and not a
/// weaker sweep.
#[test]
fn cospan_corpus_has_the_expected_size() {
    let corpus = cospan_corpus();
    // apex 0: only the 0 -> 0 shape; apex 1: 9 (dom, cod) shapes, one leg each;
    // apex 2: sum over dom, cod <= 2 of 2^dom * 2^cod = 7 * 7.
    assert_eq!(corpus.len(), 1 + 9 + 49);
    assert!(
        corpus.iter().any(|c| c.left_to_middle() == [1, 0]),
        "the corpus lost the non-monotone left leg the identity fast path turns on"
    );
}

/// `compose` on the exhaustive corpus against the union-find pushout reference.
///
/// Ranges over every composable ordered pair of the corpus at one wire type, so
/// the label a merge keeps is not under test here.
#[test]
fn cospan_compose_agrees_with_the_partition_reference() {
    let corpus = cospan_corpus();
    let mut pairs = 0usize;
    for f in &corpus {
        for g in &corpus {
            if Composable::codomain(f) != Composable::domain(g) {
                continue;
            }
            pairs += 1;
            let observed = cospan_wiring(
                &Composable::compose(f, g).expect("the boundary words were just checked equal"),
            )
            .signature();
            let expected = cospan_wiring(f)
                .pushout(&cospan_wiring(g))
                .expect("the boundary words were just checked equal")
                .signature();
            assert_eq!(
                observed, expected,
                "f = {f:?}\ng = {g:?}\nobserved {observed:?}\nexpected {expected:?}"
            );
        }
    }
    assert_eq!(pairs, 1371, "the composable-pair count moved");
}

/// `Corel::compose` on the jointly-surjective corpus against the same reference
/// with the bubble classes dropped — the extra-special quotient.
#[test]
fn corel_compose_agrees_with_the_bubble_free_partition_reference() {
    let corpus: Vec<Corel<char>> = cospan_corpus()
        .into_iter()
        .filter(Cospan::is_jointly_surjective)
        .map(|c| Corel::new(c).expect("filtered to jointly surjective"))
        .collect();
    assert_eq!(corpus.len(), 41, "the corelation corpus size moved");

    let mut pairs = 0usize;
    for f in &corpus {
        for g in &corpus {
            if Composable::codomain(f) != Composable::domain(g) {
                continue;
            }
            pairs += 1;
            let observed = cospan_wiring(
                Composable::compose(f, g)
                    .expect("the boundary words were just checked equal")
                    .as_cospan(),
            )
            .signature();
            let expected = drop_scalars(
                &cospan_wiring(f.as_cospan())
                    .pushout(&cospan_wiring(g.as_cospan()))
                    .expect("the boundary words were just checked equal"),
            )
            .signature();
            assert_eq!(
                observed,
                expected,
                "f = {:?}\ng = {:?}\nobserved {observed:?}\nexpected {expected:?}",
                f.as_cospan(),
                g.as_cospan()
            );
        }
    }
    assert_eq!(pairs, 771, "the composable-pair count moved");
}

/// The nine generators every carrier builds, in one order.
fn generator_alphabet<C: Carrier>() -> Vec<(&'static str, C)> {
    vec![
        ("eta", C::unit(Z)),
        ("eps", C::counit(Z)),
        ("mu", C::multiplication(Z)),
        ("delta", C::comultiplication(Z)),
        ("id1", C::id(&[Z])),
        ("id2", C::id(&[Z, Z])),
        ("sigma", C::swap(Z)),
        (
            "cup",
            C::cup(Z).expect("cup is defined for every carrier here"),
        ),
        (
            "cap",
            C::cap(Z).expect("cap is defined for every carrier here"),
        ),
    ]
}

/// Every composable generator word of length 2 and 3, composed left to right,
/// against the reference pushout of the operand wirings.
fn word_sweep_failures<C: Carrier>(quotient_scalars: bool) -> (usize, Vec<String>) {
    let alphabet = generator_alphabet::<C>();
    let mut failures = Vec::new();
    let mut words = 0usize;

    let compare = |name: &str,
                   left: &C,
                   right: &C,
                   failures: &mut Vec<String>|
     -> Option<(C, CospanWiring<char>)> {
        let composite = left.seq(right);
        let observed = composite.wiring().signature();
        let reference = left
            .wiring()
            .pushout(&right.wiring())
            .expect("the boundary words were just checked equal");
        let reference = if quotient_scalars {
            drop_scalars(&reference)
        } else {
            reference
        };
        if observed != reference.signature() {
            failures.push(format!(
                "  {name}\n    observed = {observed:?}\n    expected = {:?}",
                reference.signature()
            ));
            return None;
        }
        Some((composite, reference))
    };

    for (a_name, a) in &alphabet {
        for (b_name, b) in &alphabet {
            if a.cod() != b.dom() {
                continue;
            }
            words += 1;
            let name = format!("{a_name} ; {b_name}");
            let Some((ab, ab_reference)) = compare(&name, a, b, &mut failures) else {
                continue;
            };
            for (c_name, c) in &alphabet {
                if ab.cod() != c.dom() {
                    continue;
                }
                words += 1;
                let name = format!("{a_name} ; {b_name} ; {c_name}");
                let composite = ab.seq(c);
                let observed = composite.wiring().signature();
                let reference = ab_reference
                    .pushout(&c.wiring())
                    .expect("the boundary words were just checked equal");
                let reference = if quotient_scalars {
                    drop_scalars(&reference)
                } else {
                    reference
                };
                if observed != reference.signature() {
                    failures.push(format!(
                        "  {name}\n    observed = {observed:?}\n    expected = {:?}",
                        reference.signature()
                    ));
                }
            }
        }
    }
    (words, failures)
}

/// Every carrier's `compose` on generator words of length 2 and 3, against the
/// union-find partition reference computed from the operand wirings.
///
/// `Corel` and `CospanAlgebraMorphism` quotient bubbles, so the reference does
/// too for those two rows.
#[test]
fn compose_agrees_with_the_partition_reference_on_every_carrier() {
    let (cospan_words, cospan_failures) = word_sweep_failures::<Cospan<char>>(false);
    let (corel_words, corel_failures) = word_sweep_failures::<Corel<char>>(true);
    let (h_part_words, h_part_failures) =
        word_sweep_failures::<CospanAlgebraMorphism<PartitionAlgebra, char>>(true);
    let (frobenius_words, frobenius_failures) =
        word_sweep_failures::<FrobeniusMorphism<char, String>>(false);

    let rows = [
        (<Cospan<char> as Carrier>::NAME, cospan_failures),
        (<Corel<char> as Carrier>::NAME, corel_failures),
        (
            <CospanAlgebraMorphism<PartitionAlgebra, char> as Carrier>::NAME,
            h_part_failures,
        ),
        (
            <FrobeniusMorphism<char, String> as Carrier>::NAME,
            frobenius_failures,
        ),
    ];
    let report: Vec<String> = rows
        .iter()
        .filter(|(_, failures)| !failures.is_empty())
        .map(|(name, failures)| format!("[{name}]\n{}", failures.join("\n")))
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n"));

    assert_eq!(
        [cospan_words, corel_words, h_part_words, frobenius_words],
        [122, 122, 122, 122],
        "the swept word count moved"
    );
}

// ---------------------------------------------------------------------------
// #346 — strict unitality
// ---------------------------------------------------------------------------

/// `id ; f == f` and `f ; id == f` on the nose over the exhaustive corpus.
///
/// Strict structural equality, not equality up to apex isomorphism: the two
/// numberings `perform_pushout` can give an identity composite differ exactly
/// by a permutation of the apex, which a canonical form cannot see. The corpus
/// contains legs that are not monotone-injective-from-0 (`[1, 0]`), which is
/// where the fast path and the union-find body disagree.
#[test]
fn strict_unitality_over_the_cospan_corpus() {
    let corpus = cospan_corpus();
    let mut failures = Vec::new();
    for f in &corpus {
        let left_unit = <Cospan<char> as HasIdentity<Vec<char>>>::identity(&Composable::domain(f));
        let observed = Composable::compose(&left_unit, f).expect("identity composes on the left");
        if observed != *f {
            failures.push(format!("  id ; f: observed {observed:?}, expected {f:?}"));
        }
        let right_unit =
            <Cospan<char> as HasIdentity<Vec<char>>>::identity(&Composable::codomain(f));
        let observed = Composable::compose(f, &right_unit).expect("identity composes on the right");
        if observed != *f {
            failures.push(format!("  f ; id: observed {observed:?}, expected {f:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} corpus unitality checks failed:\n{}",
        failures.len(),
        2 * corpus.len(),
        failures.join("\n")
    );
}

/// The same two laws on every braiding of at most four wires, where the leg is a
/// permutation and so is non-monotone for every non-identity element.
#[test]
fn strict_unitality_on_every_braiding_of_at_most_four_wires() {
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for n in 0..=MAX_WIRES {
        let word = vec![Z; n];
        for p in all_perms(n) {
            let sigma = <Cospan<char> as Carrier>::braiding(&p, &word);
            let unit = <Cospan<char> as HasIdentity<Vec<char>>>::identity(&word);
            checked += 2;
            let left = Composable::compose(&unit, &sigma).expect("identity composes on the left");
            if left != sigma {
                failures.push(format!(
                    "  id ; sigma_{:?}: observed {left:?}, expected {sigma:?}",
                    (0..n).map(|i| p.apply(i)).collect::<Vec<_>>()
                ));
            }
            let right = Composable::compose(&sigma, &unit).expect("identity composes on the right");
            if right != sigma {
                failures.push(format!(
                    "  sigma_{:?} ; id: observed {right:?}, expected {sigma:?}",
                    (0..n).map(|i| p.apply(i)).collect::<Vec<_>>()
                ));
            }
        }
    }
    assert_eq!(checked, 2 * (1 + 1 + 2 + 6 + 24), "the swept count moved");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------------------------
// #291 — the tensor law on every public Monoidal implementor
// ---------------------------------------------------------------------------

/// A wire type as a `Leg::word` code.
fn code(c: char) -> usize {
    c as usize
}

/// The tensor law on one implementor: `wiring(f ⊗ g) == wiring(f) ++
/// shift(wiring(g))`. Takes both operands by value, since four of the thirteen
/// are not `Clone`.
fn check_tensor<T: Monoidal>(
    name: &'static str,
    mut f: T,
    g: T,
    wiring: impl Fn(&T) -> Wiring,
    failures: &mut Vec<String>,
) {
    let expected = wiring(&f).shift_concat(&wiring(&g));
    f.monoidal(g);
    let observed = wiring(&f);
    if observed != expected {
        failures.push(format!(
            "  {name}\n    observed = {observed:?}\n    expected = {expected:?}"
        ));
    }
}

/// The `Monoidal` implementors this file's tensor arm ranges over, in the order
/// `rg -nU --multiline-dotall 'impl\b[^{]*?\bMonoidal\s+for\b' catgraph/src`
/// reports their files.
const TENSOR_IMPLEMENTORS: [&str; 13] = [
    "FinSetMorphism",
    "OrderPresSurj",
    "OrderPresInj",
    "Decomposition",
    "NamedCospan",
    "GenericMonoidalMorphismLayer",
    "GenericMonoidalMorphism",
    "FrobeniusMorphism",
    "CospanAlgebraMorphism",
    "Span",
    "Rel",
    "Cospan",
    "Corel",
];

/// A block type for the two `Generic*` carriers: a wire type with an identity.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WireBox(char);

impl HasIdentity<char> for WireBox {
    fn identity(on_this: &char) -> Self {
        Self(*on_this)
    }
}

/// The `OrderPresSurj` encoding read as a map: codomain element `i` has
/// `preimage_cardinalities()[i]` preimages, in order.
fn surj_map(s: &OrderPresSurj) -> Vec<usize> {
    s.preimage_cardinalities()
        .into_iter()
        .enumerate()
        .flat_map(|(target, count)| std::iter::repeat_n(target, count))
        .collect()
}

/// The `OrderPresInj` encoding read as a map: alternating identity runs and
/// gaps, the runs carrying the domain elements.
fn inj_map(i: &OrderPresInj) -> Vec<usize> {
    let counts = i.iden_unit_counts();
    let mut map = Vec::new();
    let mut position = 0usize;
    for (index, run) in counts.iter().enumerate() {
        if index % 2 == 0 {
            for _ in 0..*run {
                map.push(position);
                position += 1;
            }
        } else {
            position += run;
        }
    }
    map
}

/// `wiring(f ⊗ g) == wiring(f) ++ shift(wiring(g))` on all thirteen public
/// `Monoidal` implementors in `catgraph/src`.
///
/// Each row names its wiring: a `FinSet` map with its codomain size for the four
/// `finset` rows; the two boundary legs into the apex for the cospan-backed
/// rows; the two apex-indexed legs into the boundaries for `Span` and `Rel`;
/// the layer type words for the two `Generic*` rows, which the tensor does not
/// renumber. The `FrobeniusMorphism` row reads the wiring off
/// `frobenius_to_cospan`; `FrobeniusMorphism::monoidal` is the only route to
/// `FrobeniusLayer::monoidal` from an integration test.
#[test]
fn tensor_is_shift_concat_on_every_public_monoidal_implementor() {
    let mut failures = Vec::new();

    // --- FinSetMorphism ---
    let finset_wiring = |m: &FinSetMorphism| {
        Wiring::new(vec![Leg::new(
            Composable::<usize>::codomain(m),
            m.0.clone(),
        )])
    };
    check_tensor(
        "FinSetMorphism",
        (vec![1, 0, 1], 0),
        (vec![0, 0], 1),
        finset_wiring,
        &mut failures,
    );
    check_tensor(
        "FinSetMorphism (empty left operand)",
        (vec![], 2),
        (vec![1, 0], 0),
        finset_wiring,
        &mut failures,
    );

    // --- OrderPresSurj ---
    let surj_wiring =
        |s: &OrderPresSurj| Wiring::new(vec![Leg::new(Composable::codomain(s), surj_map(s))]);
    check_tensor(
        "OrderPresSurj",
        OrderPresSurj::from([2, 0]),
        OrderPresSurj::from([1, 1, 0]),
        surj_wiring,
        &mut failures,
    );

    // --- OrderPresInj ---
    let inj_wiring =
        |i: &OrderPresInj| Wiring::new(vec![Leg::new(Composable::codomain(i), inj_map(i))]);
    // The left operand's run-length encoding has odd length — it ends on an
    // identity run — which is the shape the tensor has to pad before appending.
    let inj_a = OrderPresInj::try_from((vec![1, 2], 0usize))
        .expect("[1, 2] into a 3-element codomain is order preserving and injective");
    let inj_b = OrderPresInj::try_from((vec![0, 2], 0usize))
        .expect("[0, 2] into a 3-element codomain is order preserving and injective");
    check_tensor("OrderPresInj", inj_a, inj_b, inj_wiring, &mut failures);

    // --- Decomposition ---
    let decomposition_wiring = |d: &Decomposition| {
        let (map, _) = d.to_finset_morphism();
        Wiring::new(vec![Leg::new(Composable::codomain(d), map)])
    };
    let decomposition = |map: Vec<usize>, extra: usize| {
        Decomposition::try_from((map, extra)).expect("every FinSet morphism factors epi-mono")
    };
    // Both operands miss a codomain element, on either side of the image, so a
    // tensor that dropped either factor's gaps would move the map.
    check_tensor(
        "Decomposition",
        decomposition(vec![1, 0, 1], 1),
        decomposition(vec![2, 2], 1),
        decomposition_wiring,
        &mut failures,
    );
    check_tensor(
        "Decomposition (braiding)",
        Decomposition::from_permutation(Permutation::transposition(2, 0, 1), 2),
        decomposition(vec![2, 2], 1),
        decomposition_wiring,
        &mut failures,
    );
    // The extra-codomain component of each of the three operands, hand-derived
    // as `codomain - (max(map) + 1)`: `[1, 0, 1]` into 3 leaves 1, `[2, 2]` into
    // 4 leaves 1, and the two-wire braiding `[1, 0]` into 2 leaves 0.
    let decomposition_extras = (
        decomposition(vec![1, 0, 1], 1).to_finset_morphism().1,
        decomposition(vec![2, 2], 1).to_finset_morphism().1,
        Decomposition::from_permutation(Permutation::transposition(2, 0, 1), 2)
            .to_finset_morphism()
            .1,
    );
    assert_eq!(
        decomposition_extras,
        (1, 1, 0),
        "Decomposition: to_finset_morphism's extra-codomain component moved"
    );

    // --- NamedCospan ---
    let named_wiring = |n: &NamedCospan<char, u8, u8>| cospan_wiring(n.cospan()).to_wiring();
    let named_a = || {
        NamedCospan::<char, u8, u8>::new(vec![1, 0], vec![0], vec![Z, Z], vec![10, 11], vec![20])
            .expect("two apex vertices, two named domain ports, one named codomain port")
    };
    let named_b = || {
        NamedCospan::<char, u8, u8>::new(vec![0], vec![0, 0], vec![Z], vec![12], vec![21, 22])
            .expect("one apex vertex, one named domain port, two named codomain ports")
    };
    check_tensor(
        "NamedCospan",
        named_a(),
        named_b(),
        named_wiring,
        &mut failures,
    );
    let mut named_tensored = named_a();
    named_tensored.monoidal(named_b());
    assert_eq!(
        (
            named_tensored.left_names().clone(),
            named_tensored.right_names().clone()
        ),
        (vec![10, 11, 12], vec![20, 21, 22]),
        "NamedCospan: the port names are not the two name lists concatenated"
    );

    // --- GenericMonoidalMorphismLayer ---
    let layer_wiring = |l: &GenericMonoidalMorphismLayer<WireBox, char>| {
        Wiring::new(vec![
            Leg::word(l.left_type.iter().copied().map(code).collect()),
            Leg::word(l.right_type.iter().copied().map(code).collect()),
            Leg::word(l.blocks.iter().map(|b| code(b.0)).collect()),
        ])
    };
    // Neither operand is an identity — the two type words differ from each
    // other and from the block word — so a tensor that paired the wrong pair of
    // words moves the wiring.
    check_tensor(
        "GenericMonoidalMorphismLayer",
        GenericMonoidalMorphismLayer::<WireBox, char> {
            blocks: vec![WireBox('p')],
            left_type: vec!['a', 'b'],
            right_type: vec!['c'],
        },
        GenericMonoidalMorphismLayer::<WireBox, char> {
            blocks: vec![WireBox('q'), WireBox('r')],
            left_type: vec!['d'],
            right_type: vec!['e', 'f'],
        },
        layer_wiring,
        &mut failures,
    );

    // --- GenericMonoidalMorphism, at equal depth ---
    let morphism_wiring = |m: &GenericMonoidalMorphism<WireBox, char>| {
        let legs = m
            .clone()
            .extract_layers()
            .iter()
            .flat_map(|l| layer_wiring(l).legs)
            .collect();
        Wiring::new(legs)
    };
    fn generic_layer(
        blocks: &[char],
        left: &[char],
        right: &[char],
    ) -> GenericMonoidalMorphismLayer<WireBox, char> {
        GenericMonoidalMorphismLayer {
            blocks: blocks.iter().copied().map(WireBox).collect(),
            left_type: left.to_vec(),
            right_type: right.to_vec(),
        }
    }
    fn generic_morphism(
        first: GenericMonoidalMorphismLayer<WireBox, char>,
        second: GenericMonoidalMorphismLayer<WireBox, char>,
    ) -> GenericMonoidalMorphism<WireBox, char> {
        let mut morphism = GenericMonoidalMorphism::new();
        morphism
            .append_layer(first)
            .expect("an empty morphism accepts any layer");
        morphism
            .append_layer(second)
            .expect("the second layer's left_type is the first layer's right_type");
        morphism
    }
    let generic_a = generic_morphism(
        generic_layer(&['p'], &['a', 'b'], &['c']),
        generic_layer(&['q'], &['c'], &['d', 'e']),
    );
    let generic_b = generic_morphism(
        generic_layer(&['s', 't'], &['g'], &['h', 'i']),
        generic_layer(&['u'], &['h', 'i'], &['j']),
    );
    assert_eq!(
        (generic_a.depth(), generic_b.depth()),
        (2, 2),
        "the GenericMonoidalMorphism fixtures are not at equal depth, where the tensor pads with \
         identity layers instead of concatenating"
    );
    check_tensor(
        "GenericMonoidalMorphism",
        generic_a,
        generic_b,
        morphism_wiring,
        &mut failures,
    );

    // --- FrobeniusMorphism (and, through it, FrobeniusLayer) ---
    let frobenius_wiring = |m: &FrobeniusMorphism<char, String>| {
        canon_wiring(
            &frobenius_to_cospan(m)
                .expect("the fixtures build no black box")
                .canonical_form(),
        )
        .to_wiring()
    };
    let frobenius_a: FrobeniusMorphism<char, String> = FrobeniusOperation::Multiplication(Z).into();
    let frobenius_b: FrobeniusMorphism<char, String> =
        FrobeniusOperation::Comultiplication(Z).into();
    check_tensor(
        "FrobeniusMorphism",
        frobenius_a,
        frobenius_b,
        frobenius_wiring,
        &mut failures,
    );

    // --- CospanAlgebraMorphism ---
    let algebra = Arc::new(PartitionAlgebra);
    let h_part_wiring =
        |m: &CospanAlgebraMorphism<PartitionAlgebra, char>| Carrier::wiring(m).to_wiring();
    check_tensor(
        "CospanAlgebraMorphism",
        CospanAlgebraMorphism::multiplication_in(Arc::clone(&algebra), Z),
        CospanAlgebraMorphism::comultiplication_in(Arc::clone(&algebra), Z),
        h_part_wiring,
        &mut failures,
    );

    // --- Span ---
    let span_wiring = |s: &Span<char>| {
        Wiring::new(vec![
            Leg::new(s.left().len(), s.middle_to_left()),
            Leg::new(s.right().len(), s.middle_to_right()),
        ])
    };
    let span_a = Span::new(vec![Z, Z], vec![Z], vec![(1, 0), (0, 0)])
        .expect("both components of every pair are in bounds");
    let span_b =
        Span::new(vec![Z], vec![Z, Z], vec![(0, 1)]).expect("both components are in bounds");
    check_tensor("Span", span_a, span_b, span_wiring, &mut failures);

    // --- Rel ---
    let rel_wiring = |r: &Rel<char>| span_wiring(r.as_span());
    let rel_a = Rel::new(Span::new(vec![Z, Z], vec![Z], vec![(0, 0)]).expect("in bounds"))
        .expect("a single pair is jointly injective");
    let rel_b = Rel::new(Span::new(vec![Z], vec![Z, Z], vec![(0, 1)]).expect("in bounds"))
        .expect("a single pair is jointly injective");
    check_tensor("Rel", rel_a, rel_b, rel_wiring, &mut failures);

    // --- Cospan ---
    let raw_cospan_wiring = |c: &Cospan<char>| cospan_wiring(c).to_wiring();
    check_tensor(
        "Cospan",
        Cospan::new(vec![1, 0], vec![0], vec![Z, Z]).expect("in bounds"),
        Cospan::new(vec![0], vec![0, 0], vec![Z]).expect("in bounds"),
        raw_cospan_wiring,
        &mut failures,
    );

    // --- Corel ---
    let corel_wiring = |c: &Corel<char>| cospan_wiring(c.as_cospan()).to_wiring();
    check_tensor(
        "Corel",
        Corel::new(Cospan::new(vec![1, 0], vec![0, 1], vec![Z, Z]).expect("in bounds"))
            .expect("jointly surjective"),
        Corel::new(Cospan::new(vec![0], vec![0, 0], vec![Z]).expect("in bounds"))
            .expect("jointly surjective"),
        corel_wiring,
        &mut failures,
    );

    assert!(
        failures.is_empty(),
        "{} of the {} implementors failed:\n{}",
        failures.len(),
        TENSOR_IMPLEMENTORS.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// The `FrobeniusMorphism` interpretation the carrier's verdicts rest on
// ---------------------------------------------------------------------------

/// `frobenius_to_cospan` on the six lettered generators is the `Cospan`
/// generator of the same name.
///
/// Ranges over the four Frobenius generators plus the identity and the
/// same-type braiding at one wire type.
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
            frobenius_to_cospan(&fm)
                .expect("no black boxes here")
                .canonical_form(),
            cospan.canonical_form(),
            "{name}: the FrobeniusMorphism generator and the Cospan generator disagree"
        );
    }

    assert_eq!(
        frobenius_to_cospan(&<Fm as Carrier>::swap(Z))
            .expect("no black boxes here")
            .canonical_form(),
        <Cospan<char> as Carrier>::swap(Z).canonical_form(),
        "braiding: the FrobeniusMorphism braiding and the Cospan braiding disagree"
    );
}

/// The `η ; ε` bubble survives `FrobeniusMorphism::compose` to interpretation.
///
/// `Cospan` is the theory of the *special*, not extra-special, commutative
/// Frobenius monoids: the closed loop is a genuine scalar, and both the
/// presented composite and the two interpretations composed in `Cospan` keep it.
#[test]
fn frobenius_scalar_loop_survives_to_interpretation() {
    let eta = FrobeniusMorphism::<char, String>::unit(Z);
    let eps = FrobeniusMorphism::<char, String>::counit(Z);

    let mut presented = eta.clone();
    ComposableMutating::compose(&mut presented, eps.clone()).expect("[] -> [z] -> [] composes");
    let after_compose = frobenius_to_cospan(&presented)
        .expect("no black boxes here")
        .canonical_form();

    let semantic = Composable::compose(
        &frobenius_to_cospan(&eta).expect("no black boxes here"),
        &frobenius_to_cospan(&eps).expect("no black boxes here"),
    )
    .expect("the two generator cospans compose")
    .canonical_form();

    assert_eq!(
        (after_compose.apex_len(), after_compose.scalar_count()),
        (1, 1),
        "FrobeniusMorphism::compose erased the eta ; eps bubble"
    );
    assert_eq!(
        (semantic.apex_len(), semantic.scalar_count()),
        (1, 1),
        "Cospan stopped keeping the eta ; eps scalar"
    );
    assert_eq!(
        after_compose, semantic,
        "the presented route and the semantic route disagree on the eta ; eps scalar"
    );
}

/// The normalizer's spider-fusion rule does not fuse across zero wires.
///
/// `Spider(z, 2, 0) ; Spider(z, 0, 2)` is a sink beside a source, two components
/// with nothing between them; fusing it to `Spider(z, 2, 2)` changes
/// connectivity. Ranges over one wire type and the arities `(2, 0)` and
/// `(0, 2)`.
#[test]
fn spider_fusion_needs_a_wire_between_the_two_spiders() {
    type Fm = FrobeniusMorphism<char, String>;

    let sink: Fm = FrobeniusOperation::Spider(Z, 2, 0).into();
    let source: Fm = FrobeniusOperation::Spider(Z, 0, 2).into();

    let mut presented = sink.clone();
    ComposableMutating::compose(&mut presented, source.clone())
        .expect("[z, z] -> [] then [] -> [z, z] composes");

    let after_compose = frobenius_to_cospan(&presented)
        .expect("no black boxes here")
        .canonical_form();
    let semantic = Composable::compose(
        &frobenius_to_cospan(&sink).expect("no black boxes here"),
        &frobenius_to_cospan(&source).expect("no black boxes here"),
    )
    .expect("the two generator cospans compose")
    .canonical_form();

    assert_eq!(
        semantic.classes().len(),
        2,
        "Cospan stopped keeping the sink and the source apart; got {semantic:?}"
    );
    assert_eq!(
        after_compose,
        semantic,
        "FrobeniusMorphism::compose fused two spiders that share no wire: {} apex class(es) \
         against the semantics' {}. Presentation: {presented:?}",
        after_compose.classes().len(),
        semantic.classes().len()
    );
    assert_eq!(
        presented.depth(),
        2,
        "the sink and the source fused into one block. Presentation: {presented:?}"
    );
}

/// A black box denotes no cospan, and `frobenius_to_cospan` names it rather than
/// inventing one.
#[test]
fn frobenius_to_cospan_rejects_black_boxes() {
    let boxed: FrobeniusMorphism<char, String> =
        FrobeniusOperation::UnSpecifiedBox("f".to_string(), vec![Z], vec![Z, Z]).into();
    let error = frobenius_to_cospan(&boxed).expect_err("a black box denotes nothing");
    assert!(
        format!("{error}").contains("UnSpecifiedBox"),
        "the error should name what could not be interpreted, got: {error}"
    );
}
