//! #258 — the braiding convention, pinned **inside the core crate** (#286).
//!
//! # Why this file exists
//!
//! The #258 contract was pinned once, in
//! `catgraph-applied/tests/braiding_cross_carrier.rs`. That file is the
//! cross-carrier oracle and stays where it is — but three of the rows it
//! covers are on types defined *here*, in `catgraph`, and had no core-side pin:
//! [`CospanAlgebraMorphism`], [`FrobeniusMorphism`]'s **wiring**, and
//! [`NamedCospan`]'s **port-name direction** (it delegates its cospan to
//! `Cospan::from_permutation_on_*`, so `cospan::test::permutation_automatic` /
//! `permutatation_manual_labelled` pin that direction, and
//! `named_cospan::test::permutatation_automatic` compares a composite's legs;
//! nothing in core pinned the names — permuting them by `p` instead of `p⁻¹`,
//! in the constructor or in `permute_side`, reddens only this file).
//!
//! The measured consequence (#286): inverting the braiding direction in
//! `CospanAlgebraMorphism`'s two constructors — `p.inv()` ⇄ `p` in both the
//! forced label word and the structural cospan's right leg — left
//! `cargo test -p catgraph` **fully green** while `cargo test -p
//! catgraph-applied` went red on exactly 4 tests, all in
//! `braiding_cross_carrier.rs`. A downstream test restructure would therefore
//! have zeroed core's coverage of its own type without a single red light in
//! core. With this file present the same mutation turns 4 core tests red:
//! [`hand_written_reference_and_cam_element`] fails first on the codomain word
//! (`['B','C','A']` where the contract requires `['C','A','B']`), and
//! [`permute_side_pins_on_the_core_carriers`] reports the mutated constructor's
//! apex classes as `A:[0,5] B:[1,3] C:[2,4]` against the contract form
//! `A:[0,4] B:[1,5] C:[2,3]` that `permute_side` itself still produces.
//!
//! # What is checked, and against what
//!
//! Nothing here compares one carrier to another: a symmetric drift would
//! survive that. Every carrier is checked against a reference computed
//! **directly from `p`** — `(0..n).map(|i| p.apply(i))` for the wiring,
//! `p.inv().permute(types)` for the forced labels — and
//! [`hand_written_reference_and_cam_element`] pins that reference itself, plus
//! the `CospanAlgebraMorphism` element's whole canonical form, against values
//! written out by hand.
//!
//! # The space these claims range over
//!
//! The single-permutation sweeps run `n ∈ {3, 4}` — all `6 + 24 = 30`
//! permutations; the composition sweep runs `S₃ × S₃` (36 ordered pairs) and
//! does **not** extend to `n = 4`. Labels are **distinct** (`['A','B',…]`),
//! since uniform labels make every word assertion constant in `p`. `n ≤ 2` is
//! excluded on purpose: every permutation there is an involution, so a direction
//! flip is unobservable by construction. One algebra ([`PartitionAlgebra`]) and
//! one label type (`char`) are instantiated; a second algebra is *not* covered,
//! and neither is any `n > 4`. The `FrobeniusMorphism` rows use
//! `BlackBoxLabel = ()` only. Per-test docstrings narrow this further and are
//! the authority where they differ.

mod common;
use common::cospan_wiring;

use catgraph::category::{Composable, ComposableMutating, HasIdentity};
use catgraph::cospan_algebra::{PartitionAlgebra, frobenius_to_cospan};
use catgraph::cospan_canon::{ApexClass, CospanCanon};
use catgraph::equivalence::CospanAlgebraMorphism;
use catgraph::frobenius::FrobeniusMorphism;
use catgraph::monoidal::SymmetricMonoidalMorphism;
use catgraph::named_cospan::NamedCospan;
use catgraph_testutil::all_perms;
use permutations::Permutation;

type Cam = CospanAlgebraMorphism<PartitionAlgebra, char>;
type Frob = FrobeniusMorphism<char, ()>;

// ---------------------------------------------------------------------------
// Fixtures and extractors
// ---------------------------------------------------------------------------

/// The reference wiring, read straight off `p`: domain wire `i` goes to
/// codomain wire `p.apply(i)`.
fn reference_wiring(p: &Permutation) -> Vec<usize> {
    (0..p.len()).map(|i| p.apply(i)).collect()
}

/// Distinct labels, so a permuted word is distinguishable from an
/// inverse-permuted one. `['A', 'B', 'C', …]`.
fn labels(n: usize) -> Vec<char> {
    (0..n)
        .map(|i| char::from(b'A' + u8::try_from(i).expect("n < 26 in these fixtures")))
        .collect()
}

/// A second alphabet, disjoint from [`labels`], for port names.
fn names(n: usize) -> Vec<char> {
    (0..n)
        .map(|i| char::from(b'a' + u8::try_from(i).expect("n < 26 in these fixtures")))
        .collect()
}

/// The [`CospanAlgebraMorphism`] element is a cospan `[] → apex ← domain ⊕
/// codomain`, so its right leg *is* the partition. Domain wire `i` meets
/// codomain wire `k` when `right[i] == right[n + k]`.
fn cam_wiring(m: &Cam) -> Vec<usize> {
    let n = m.domain().len();
    let right = m.element().right_to_middle();
    assert_eq!(right.len(), 2 * n, "interface is domain ⊕ codomain");
    (0..n)
        .map(|i| {
            (0..n)
                .find(|k| right[n + k] == right[i])
                .expect("a braiding element must pair every domain wire with a codomain wire")
        })
        .collect()
}

/// `FrobeniusMorphism` has no leg vectors of its own; the crate's own
/// [`frobenius_to_cospan`] is the observable that turns one into wiring.
///
/// This is the row the applied oracle does **not** have: over there
/// `FrobeniusMorphism` is checked on its `domain()`/`codomain()` words only,
/// and a word can be right over an inverted wiring (as `Span` demonstrated at
/// #258).
fn frobenius_wiring(f: &Frob) -> Vec<usize> {
    let c = frobenius_to_cospan(f).expect("a pure braiding has no black boxes");
    cospan_wiring(&c)
}

// ---------------------------------------------------------------------------
// 1. The hand anchor — nothing below is computed by the code under test
// ---------------------------------------------------------------------------

/// `rotation_left(3, 1)` sends `0→1, 1→2, 2→0`, so `p.inv()` sends `k → (k+2)%3`.
///
/// Two things are anchored here, both written out rather than derived:
///
/// 1. `reference_wiring(&p)`, which every sweep below compares against.
/// 2. The **whole canonical form** of `CospanAlgebraMorphism`'s `.element()`,
///    on both constructors and on the identity, as a
///    [`CospanCanon`] built by hand from [`ApexClass`]es. The element is the
///    morphism for this carrier — `domain`/`codomain` are only the words naming
///    its interface — so this is the assertion that actually reaches it, and
///    canonical form is what makes the comparison independent of how the
///    pushout happened to number the apex.
///
/// # The space this claim ranges over
///
/// One permutation (`rotation_left(3, 1)`, a 3-cycle — chosen because
/// `p != p.inv()`, so a direction flip is visible at all), one label alphabet,
/// `PartitionAlgebra` only. The exhaustive sweeps are the other tests; this one
/// is the anchor that stops those sweeps from drifting together with the code.
#[test]
fn hand_written_reference_and_cam_element() {
    let p = Permutation::rotation_left(3, 1);
    let types = ['A', 'B', 'C'];

    assert_eq!(reference_wiring(&p), vec![1, 2, 0]);
    assert_ne!(
        reference_wiring(&p),
        reference_wiring(&p.inv()),
        "a 3-cycle must not be its own inverse, or nothing here pins a direction"
    );

    // -- on_domain: apex = the domain word, right leg = (0..3) ++ p.inv() ----
    //
    // Interface slots 0..3 are the domain wires, 3..6 the codomain wires. The
    // braiding pairs domain `i` with codomain `p(i)`, i.e. slot `3 + p(i)`:
    // 0↔4, 1↔5, 2↔3.
    let a = Cam::from_permutation_on_domain(p.clone(), &types).unwrap();
    assert_eq!(a.domain(), vec!['A', 'B', 'C']);
    assert_eq!(
        a.codomain(),
        vec!['C', 'A', 'B'],
        "codomain[k] = types[p⁻¹(k)]"
    );
    assert_eq!(cam_wiring(&a), vec![1, 2, 0]);

    let on_domain_expected = CospanCanon::from_parts(
        0,
        6,
        vec![
            ApexClass::new('A', vec![], vec![0, 4]),
            ApexClass::new('B', vec![], vec![1, 5]),
            ApexClass::new('C', vec![], vec![2, 3]),
        ],
    )
    .expect("hand-written canonical form must satisfy the from_parts invariants");
    assert_eq!(
        a.element().canonical_form(),
        on_domain_expected,
        "the element must pair domain i with codomain p(i)"
    );

    // The inverted convention is a *different* canonical form, so the
    // assertion above pins a direction rather than accepting either. Under the
    // flip the right leg becomes (0..3) ++ p, giving 0↔5, 1↔3, 2↔4.
    let inverted = CospanCanon::from_parts(
        0,
        6,
        vec![
            ApexClass::new('A', vec![], vec![0, 5]),
            ApexClass::new('B', vec![], vec![1, 3]),
            ApexClass::new('C', vec![], vec![2, 4]),
        ],
    )
    .expect("hand-written canonical form must satisfy the from_parts invariants");
    assert_ne!(
        on_domain_expected, inverted,
        "the two conventions must be distinguishable at all"
    );
    assert_ne!(
        a.element().canonical_form(),
        inverted,
        "element realizes p⁻¹ — the pre-#258 direction"
    );

    // -- on_codomain: apex = the codomain word, right leg = p ++ (0..3) ------
    let b = Cam::from_permutation_on_codomain(p.clone(), &types).unwrap();
    assert_eq!(b.domain(), vec!['B', 'C', 'A'], "domain[i] = types[p(i)]");
    assert_eq!(b.codomain(), vec!['A', 'B', 'C']);
    assert_eq!(cam_wiring(&b), vec![1, 2, 0]);
    assert_eq!(
        b.element().canonical_form(),
        CospanCanon::from_parts(
            0,
            6,
            vec![
                ApexClass::new('A', vec![], vec![2, 3]),
                ApexClass::new('B', vec![], vec![0, 4]),
                ApexClass::new('C', vec![], vec![1, 5]),
            ],
        )
        .expect("hand-written canonical form must satisfy the from_parts invariants"),
    );

    // -- the identity permutation degenerates to the cup ---------------------
    let id = Cam::from_permutation_on_domain(Permutation::identity(3), &types).unwrap();
    assert_eq!(
        id.element().canonical_form(),
        CospanCanon::from_parts(
            0,
            6,
            vec![
                ApexClass::new('A', vec![], vec![0, 3]),
                ApexClass::new('B', vec![], vec![1, 4]),
                ApexClass::new('C', vec![], vec![2, 5]),
            ],
        )
        .expect("hand-written canonical form must satisfy the from_parts invariants"),
        "β(id) must be the cup, which is what identity_in builds",
    );

    // -- FrobeniusMorphism, read through the crate's own cospan functor ------
    let f = Frob::from_permutation_on_domain(p.clone(), &types).unwrap();
    assert_eq!(ComposableMutating::domain(&f), vec!['A', 'B', 'C']);
    assert_eq!(ComposableMutating::codomain(&f), vec!['C', 'A', 'B']);
    assert_eq!(
        frobenius_wiring(&f),
        vec![1, 2, 0],
        "the layered braiding must realize i ↦ p(i), not i ↦ p⁻¹(i)"
    );

    // -- NamedCospan: the port names ride along with the labels --------------
    let nc = NamedCospan::<char, char, char>::from_permutation_extra_data_on_domain(
        p,
        &types,
        &names(3),
        |z| (z, z),
    )
    .unwrap();
    assert_eq!(nc.domain(), vec!['A', 'B', 'C']);
    assert_eq!(nc.codomain(), vec!['C', 'A', 'B']);
    assert_eq!(nc.left_names(), &vec!['a', 'b', 'c']);
    assert_eq!(
        nc.right_names(),
        &vec!['c', 'a', 'b'],
        "right names reorder by p.inv(), exactly as the codomain labels do"
    );
    assert_eq!(cospan_wiring(nc.cospan()), vec![1, 2, 0]);
}

// ---------------------------------------------------------------------------
// 2. The three core-defined carriers, on both constructors, exhaustively
// ---------------------------------------------------------------------------

/// The sweep #286 asks for: `CospanAlgebraMorphism`, `FrobeniusMorphism`
/// (wiring, not just words) and `NamedCospan`, each against
/// `reference_wiring(&p)` — never against another carrier.
///
/// # The space this claim ranges over
///
/// All `6 + 24 = 30` permutations at `n ∈ {3, 4}`, both constructors, distinct
/// labels. `PartitionAlgebra`/`char` only for the algebra row; `()` black-box
/// labels only for the Frobenius row. Nothing here says anything about `n > 4`,
/// about a second algebra, or about `n ∈ {0, 1, 2}` (where every permutation is
/// an involution and a direction flip is unobservable by construction).
#[test]
fn core_carriers_realize_p_on_both_constructors() {
    let mut checked = 0usize;
    for n in [3usize, 4] {
        let types = labels(n);
        let prenames = names(n);
        for p in all_perms(n) {
            let want = reference_wiring(&p);
            let cod_labels: Vec<char> = p.inv().permute(&types);
            let dom_labels: Vec<char> = p.permute(&types);

            // ---- CospanAlgebraMorphism, types on the domain --------------
            let a = Cam::from_permutation_on_domain(p.clone(), &types).unwrap();
            assert_eq!(cam_wiring(&a), want, "Cam on_domain n={n} p={p:?}");
            assert_eq!(a.domain(), types, "Cam on_domain dom n={n} p={p:?}");
            assert_eq!(a.codomain(), cod_labels, "Cam on_domain cod n={n} p={p:?}");

            // ---- CospanAlgebraMorphism, types on the codomain ------------
            let a = Cam::from_permutation_on_codomain(p.clone(), &types).unwrap();
            assert_eq!(cam_wiring(&a), want, "Cam on_codomain n={n} p={p:?}");
            assert_eq!(a.domain(), dom_labels, "Cam on_codomain dom n={n} p={p:?}");
            assert_eq!(a.codomain(), types, "Cam on_codomain cod n={n} p={p:?}");

            // The relabelling law, on_codomain(p, types) == on_domain(p, p.permute(types)):
            // a consistency check restating the trait's own cross-check. It is
            // redundant with the wiring rows above, not the only witness of a
            // one-sided inversion — inverting `from_permutation_on_codomain`
            // alone (labels and leg through `p.inv()`) already fails the
            // `Cam on_codomain` wiring assertion above, measured at n=3
            // p=[1,2,0]: `[2,0,1]` vs `[1,2,0]`, with
            // `hand_written_reference_and_cam_element` red on the on_codomain
            // element's domain word (`['C','A','B']` where
            // `domain[i] = types[p(i)]` requires `['B','C','A']`) — 3 passed,
            // 2 failed; this line is never reached.
            let relabelled: Vec<char> = p.permute(&types);
            let via_domain = Cam::from_permutation_on_domain(p.clone(), &relabelled).unwrap();
            assert_eq!(
                a.element().canonical_form(),
                via_domain.element().canonical_form(),
                "on_codomain(p, types) == on_domain(p, p.permute(types)); n={n} p={p:?}"
            );

            // ---- FrobeniusMorphism, wiring on both constructors ----------
            let f = Frob::from_permutation_on_domain(p.clone(), &types).unwrap();
            assert_eq!(frobenius_wiring(&f), want, "Frob on_domain n={n} p={p:?}");
            assert_eq!(
                ComposableMutating::domain(&f),
                types,
                "Frob on_domain dom n={n} p={p:?}"
            );
            assert_eq!(
                ComposableMutating::codomain(&f),
                cod_labels,
                "Frob on_domain cod n={n} p={p:?}"
            );

            let f = Frob::from_permutation_on_codomain(p.clone(), &types).unwrap();
            assert_eq!(frobenius_wiring(&f), want, "Frob on_codomain n={n} p={p:?}");
            assert_eq!(
                ComposableMutating::domain(&f),
                dom_labels,
                "Frob on_codomain dom n={n} p={p:?}"
            );
            assert_eq!(
                ComposableMutating::codomain(&f),
                types,
                "Frob on_codomain cod n={n} p={p:?}"
            );

            // ---- NamedCospan, wiring + the port-name words ---------------
            let nc = NamedCospan::<char, char, char>::from_permutation_extra_data_on_domain(
                p.clone(),
                &types,
                &prenames,
                |z| (z, z),
            )
            .unwrap();
            assert_eq!(
                cospan_wiring(nc.cospan()),
                want,
                "NamedCospan on_domain n={n} p={p:?}"
            );
            assert_eq!(
                nc.domain(),
                types,
                "NamedCospan on_domain dom n={n} p={p:?}"
            );
            assert_eq!(
                nc.codomain(),
                cod_labels,
                "NamedCospan on_domain cod n={n} p={p:?}"
            );
            assert_eq!(
                nc.left_names(),
                &prenames,
                "NamedCospan on_domain left names n={n} p={p:?}"
            );
            assert_eq!(
                nc.right_names(),
                &p.inv().permute(&prenames),
                "NamedCospan on_domain right names n={n} p={p:?}"
            );

            let nc = NamedCospan::<char, char, char>::from_permutation_extra_data_on_codomain(
                p.clone(),
                &types,
                &prenames,
                |z| (z, z),
            )
            .unwrap();
            assert_eq!(
                cospan_wiring(nc.cospan()),
                want,
                "NamedCospan on_codomain n={n} p={p:?}"
            );
            assert_eq!(
                nc.domain(),
                dom_labels,
                "NamedCospan on_codomain dom n={n} p={p:?}"
            );
            assert_eq!(
                nc.codomain(),
                types,
                "NamedCospan on_codomain cod n={n} p={p:?}"
            );
            assert_eq!(
                nc.left_names(),
                &p.permute(&prenames),
                "NamedCospan on_codomain left names n={n} p={p:?}"
            );
            assert_eq!(
                nc.right_names(),
                &prenames,
                "NamedCospan on_codomain right names n={n} p={p:?}"
            );

            checked += 1;
        }
    }
    assert_eq!(checked, 6 + 24, "n=3 and n=4 sweeps must both have run");
}

// ---------------------------------------------------------------------------
// 3. `permute_side` on the same three carriers
// ---------------------------------------------------------------------------

/// `permute_side` splices `β(p)` on the codomain and `β(p⁻¹)` on the domain —
/// the asymmetry the trait documents. Two consequences are checked:
///
/// - **on an identity**, the result is the corresponding constructor's
///   braiding, wiring for wiring;
/// - **conjugation**: permuting *both* sides of an identity by the same `p`
///   returns the identity on the relabelled word. Under the symmetric misreading
///   (`β(p)` on both sides) the result is `β(p²)`, which is the identity only
///   when `p` is an involution — so this is the assertion the domain rule lives
///   or dies on, and it is why the sweep starts at `n = 3` (every element of
///   `S₂` is an involution).
///
/// # The space this claim ranges over
///
/// All 30 permutations at `n ∈ {3, 4}`, on `CospanAlgebraMorphism` and
/// `NamedCospan` (identity sweep **and** conjugation), and on
/// `FrobeniusMorphism` in the conjugation block only — words and wiring there,
/// but no single-sided `FrobeniusMorphism` sweep. Nothing here covers a
/// *non*-identity starting morphism on any of the three: that separates
/// "splices the right braiding" from "rebuilds one from scratch", and it is
/// `catgraph-applied/tests/braiding_cross_carrier.rs` §9 that makes that claim,
/// not this file.
#[test]
fn permute_side_pins_on_the_core_carriers() {
    let mut checked = 0usize;
    for n in [3usize, 4] {
        let types = labels(n);
        let prenames = names(n);
        let identity_wiring: Vec<usize> = (0..n).collect();
        for p in all_perms(n) {
            let p_inv = p.inv();
            let want_cod = reference_wiring(&p);
            let want_dom = reference_wiring(&p_inv);
            let permuted: Vec<char> = p_inv.permute(&types);
            let permuted_names: Vec<char> = p_inv.permute(&prenames);

            // ---- CospanAlgebraMorphism: identity ; β --------------------
            let mut a = <Cam as HasIdentity<Vec<char>>>::identity(&types);
            a.permute_side(&p, true);
            assert_eq!(cam_wiring(&a), want_cod, "Cam cod n={n} p={p:?}");
            assert_eq!(a.domain(), types, "Cam cod dom n={n} p={p:?}");
            assert_eq!(a.codomain(), permuted, "Cam cod cod n={n} p={p:?}");
            assert_eq!(
                a.element().canonical_form(),
                Cam::from_permutation_on_domain(p.clone(), &types)
                    .unwrap()
                    .element()
                    .canonical_form(),
                "id.permute_side(p, true) must be the on_domain braiding; n={n} p={p:?}"
            );

            let mut a = <Cam as HasIdentity<Vec<char>>>::identity(&types);
            a.permute_side(&p, false);
            assert_eq!(cam_wiring(&a), want_dom, "Cam dom n={n} p={p:?}");
            assert_eq!(a.domain(), permuted, "Cam dom dom n={n} p={p:?}");
            assert_eq!(a.codomain(), types, "Cam dom cod n={n} p={p:?}");

            // ---- CospanAlgebraMorphism: conjugation ---------------------
            let mut a = <Cam as HasIdentity<Vec<char>>>::identity(&types);
            a.permute_side(&p, true);
            a.permute_side(&p, false);
            assert_eq!(
                cam_wiring(&a),
                identity_wiring,
                "conjugating an identity by p must give the identity back, not β(p²); n={n} p={p:?}"
            );
            assert_eq!(a.domain(), permuted, "Cam conj dom n={n} p={p:?}");
            assert_eq!(a.codomain(), permuted, "Cam conj cod n={n} p={p:?}");
            assert_eq!(
                a.element().canonical_form(),
                <Cam as HasIdentity<Vec<char>>>::identity(&permuted)
                    .element()
                    .canonical_form(),
                "Cam conjugation must land on the identity element; n={n} p={p:?}"
            );

            // ---- NamedCospan: names travel with their wires -------------
            let mut nc = NamedCospan::<char, char, char>::identity(&types, &prenames, |z| (z, z));
            nc.permute_side(&p, true);
            assert_eq!(
                cospan_wiring(nc.cospan()),
                want_cod,
                "NamedCospan cod n={n} p={p:?}"
            );
            assert_eq!(nc.domain(), types, "NamedCospan cod dom n={n} p={p:?}");
            assert_eq!(nc.codomain(), permuted, "NamedCospan cod cod n={n} p={p:?}");
            assert_eq!(
                nc.right_names(),
                &permuted_names,
                "right names must move with the right leg; n={n} p={p:?}"
            );
            assert_eq!(
                nc.left_names(),
                &prenames,
                "left names untouched; n={n} p={p:?}"
            );

            let mut nc = NamedCospan::<char, char, char>::identity(&types, &prenames, |z| (z, z));
            nc.permute_side(&p, false);
            assert_eq!(
                cospan_wiring(nc.cospan()),
                want_dom,
                "NamedCospan dom n={n} p={p:?}"
            );
            assert_eq!(nc.domain(), permuted, "NamedCospan dom dom n={n} p={p:?}");
            assert_eq!(nc.codomain(), types, "NamedCospan dom cod n={n} p={p:?}");
            assert_eq!(
                nc.left_names(),
                &permuted_names,
                "left names must move with the left leg; n={n} p={p:?}"
            );
            assert_eq!(
                nc.right_names(),
                &prenames,
                "right names untouched; n={n} p={p:?}"
            );

            // ---- FrobeniusMorphism: conjugation, on the words -----------
            let mut f: Frob = FrobeniusMorphism::identity(&types);
            f.permute_side(&p, true);
            f.permute_side(&p, false);
            assert_eq!(
                ComposableMutating::domain(&f),
                permuted,
                "Frob conj dom n={n} p={p:?}"
            );
            assert_eq!(
                ComposableMutating::codomain(&f),
                permuted,
                "Frob conj cod n={n} p={p:?}"
            );
            assert_eq!(
                frobenius_wiring(&f),
                identity_wiring,
                "Frob conjugation must be the identity wiring, not β(p²); n={n} p={p:?}"
            );

            checked += 1;
        }
    }
    assert_eq!(checked, 6 + 24, "n=3 and n=4 sweeps must both have run");
}

// ---------------------------------------------------------------------------
// 4. Composition of braidings, over every ordered pair
// ---------------------------------------------------------------------------

/// `β(p₁) ; β(p₂) == β(p₁ ; p₂)` on `CospanAlgebraMorphism` and
/// `FrobeniusMorphism`, over **all 36 ordered pairs of `S₃`** with distinct
/// labels.
///
/// The load-bearing assertion is the **wiring**: the composite is compared
/// against `reference_wiring(&(p1 * p2))`, computed from the two permutations
/// directly rather than from the constructor under test, so a drift affecting
/// both sides alike cannot cancel. The element-level
/// `canonical_form() == from_permutation_on_domain(p1 * p2, …)` assertion below
/// it *does* call that constructor a third time, and so is a self-consistency
/// check (a composite must land where the direct construction does) riding on
/// the reference-based wiring check — not an independent oracle.
///
/// # The space this claim ranges over
///
/// `S₃ × S₃` only (36 pairs), two carriers, one label alphabet. `n = 4` is not
/// swept here: the pair count is quadratic and the direction question is
/// already decided at `n = 3`, where `S₃` contains non-involutions.
#[test]
fn braiding_composition_over_all_s3_pairs() {
    let n = 3usize;
    let types = labels(n);
    let perms = all_perms(n);
    assert_eq!(perms.len(), 6, "S3 has 6 elements");

    let mut checked = 0usize;
    for p1 in &perms {
        for p2 in &perms {
            let want = reference_wiring(&(p1 * p2));

            // The middle word is forced: c1's codomain is c2's domain.
            let mid: Vec<char> = p1.inv().permute(&types);

            let a1 = Cam::from_permutation_on_domain(p1.clone(), &types).unwrap();
            let a2 = Cam::from_permutation_on_domain(p2.clone(), &mid).unwrap();
            let a12 = a1.compose(&a2).expect("interfaces agree by construction");
            assert_eq!(cam_wiring(&a12), want, "Cam n={n} p1={p1:?} p2={p2:?}");
            assert_eq!(a12.domain(), types, "Cam dom n={n} p1={p1:?} p2={p2:?}");
            assert_eq!(
                a12.codomain(),
                (p1 * p2).inv().permute(&types),
                "Cam cod n={n} p1={p1:?} p2={p2:?}"
            );
            assert_eq!(
                a12.element().canonical_form(),
                Cam::from_permutation_on_domain(p1 * p2, &types)
                    .unwrap()
                    .element()
                    .canonical_form(),
                "β(p1) ; β(p2) must be β(p1 ; p2) as an element; n={n} p1={p1:?} p2={p2:?}"
            );

            let mut f1 = Frob::from_permutation_on_domain(p1.clone(), &types).unwrap();
            let f2 = Frob::from_permutation_on_domain(p2.clone(), &mid).unwrap();
            f1.compose(f2).expect("interfaces agree by construction");
            assert_eq!(
                frobenius_wiring(&f1),
                want,
                "Frob n={n} p1={p1:?} p2={p2:?}"
            );
            assert_eq!(
                ComposableMutating::codomain(&f1),
                (p1 * p2).inv().permute(&types),
                "Frob cod n={n} p1={p1:?} p2={p2:?}"
            );

            checked += 1;
        }
    }
    assert_eq!(checked, 36, "all 36 ordered S3 pairs must have run");
}

// ---------------------------------------------------------------------------
// 5. Arity mismatch is an error, on the carriers defined here
// ---------------------------------------------------------------------------

/// Both constructors of the **three carriers this file covers** reject a length
/// mismatch rather than panicking, and `NamedCospan`'s trait methods refuse
/// loudly and name their replacement.
///
/// `Cospan`, `Span` and `Corel` are not re-checked here — they already have
/// core-side pins (`cospan.rs`, `span.rs`) and the applied oracle sweeps all
/// nine carriers.
#[test]
fn arity_mismatch_and_named_cospan_refusal() {
    let p = Permutation::rotation_left(3, 1);
    let long = labels(4); // deliberately one longer than p

    assert!(Cam::from_permutation_on_domain(p.clone(), &long).is_err());
    assert!(Cam::from_permutation_on_codomain(p.clone(), &long).is_err());
    assert!(Frob::from_permutation_on_domain(p.clone(), &long).is_err());
    assert!(Frob::from_permutation_on_codomain(p.clone(), &long).is_err());
    assert!(
        NamedCospan::<char, char, char>::from_permutation_extra_data_on_domain(
            p.clone(),
            &long,
            &names(4),
            |z| (z, z)
        )
        .is_err(),
        "p shorter than types"
    );
    assert!(
        NamedCospan::<char, char, char>::from_permutation_extra_data_on_codomain(
            p.clone(),
            &labels(3),
            &names(4),
            |z| (z, z)
        )
        .is_err(),
        "prenames longer than types"
    );

    let refusals = [
        (
            <NamedCospan<char, u8, u8> as SymmetricMonoidalMorphism<char>>::from_permutation_on_domain(
                p.clone(),
                &labels(3),
            )
            .err(),
            "from_permutation_extra_data_on_domain",
        ),
        (
            <NamedCospan<char, u8, u8> as SymmetricMonoidalMorphism<char>>::from_permutation_on_codomain(
                p,
                &labels(3),
            )
            .err(),
            "from_permutation_extra_data_on_codomain",
        ),
    ];
    for (err, expected) in refusals {
        let err = err.expect("NamedCospan cannot build a braiding without port names");
        let rendered = format!("{err}");
        assert!(
            rendered.contains(expected),
            "the refusal must name {expected}, got: {rendered}"
        );
    }
}
