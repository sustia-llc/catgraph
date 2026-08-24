//! Integration tests for self-dual compact closed structure (Fong-Spivak §3.1).
//!
//! Tests cup/cap morphisms, zigzag (snake) identities, tensor-ordered cup/cap,
//! name bijection (Prop 3.2), and composition-via-names (Props 3.3-3.4).

use catgraph::{
    category::{ComposableMutating, HasIdentity},
    compact_closed::{
        cap, cap_single, cap_tensor, compose_names, compose_names_direct, compose_names_via_unname,
        cup, cup_single, cup_tensor, name, unname,
    },
    cospan::Cospan,
    cospan_algebra::frobenius_to_cospan,
    cospan_canon::CospanCanon,
    equivalence::comp_cospan,
    frobenius::{Frobenius, FrobeniusMorphism, FrobeniusOperation},
    monoidal::Monoidal,
};

type FM = FrobeniusMorphism<char, String>;

// ---------------------------------------------------------------------------
// Semantic comparison (the content pins below all route through this)
// ---------------------------------------------------------------------------
//
// `FrobeniusMorphism` derives `Eq` on its *layer vector*, and composition only
// applies a local `two_layer_simplify`, so equal string diagrams routinely
// differ syntactically — `==` on `FM` is sound but far too fine to state the
// compact-closed laws with. `frobenius_to_cospan` sends a term to its image in
// `Cospan`, the theory of special commutative Frobenius monoids (F&S 2019
// Prop 3.8), and `canonical_form` decides isomorphism there.
//
// ⚠ **The scalar gap this header used to describe is closed (#350).** The layer
// simplifier carried a rule cancelling `η;ε` — the *extra-special* axiom, which
// `Cospan` does not satisfy — and that single fact broke both directions of
// exactness against SCFM: a spelled `η;ε` shared its image with
// `FM::identity(&vec![])` though the two are not SCFM-equal (*not complete*),
// and differed from `Spider('a', 0, 0)` though those two are (*not sound*).
// The rule was deleted at #350 and both witnesses now come out the other way,
// measured in `cospan_algebra::tests::scalar_bubbles_survive_in_both_directions`
// and `::scfm_equal_scalars_have_equal_images`.
//
// The pins below never depended on that repair: none of them ranges over a term
// carrying a boundary-adjacent `η;ε`, which was the only shape the gap touched.
//
// Consequence for everything below: `assert_same_cospan` compares apex classes
// and bubble counts (`CospanCanon::scalar_count`), so a scalar reaching
// `frobenius_to_cospan` IS caught — which, since #350, is every scalar the term
// spells. What these pins do catch above all is any change to a term's
// boundary-to-apex connectivity — which is what the audit's two measured
// vacuity modes (discard-inputs/create-outputs junk, and dropping `f̂`/`ĝ` for
// bare units) both are.

/// The semantic image of a Frobenius term: its cospan up to apex isomorphism.
fn canon(m: &FM) -> CospanCanon<char> {
    frobenius_to_cospan(m)
        .expect("the terms in this file carry no black boxes")
        .canonical_form()
}

/// A one-line digest of a canonical form, for failure messages: boundary sizes,
/// apex size, scalar (bubble) count.
fn digest(c: &CospanCanon<char>) -> String {
    format!(
        "{}→{} apex={} scalars={}",
        c.dom_len(),
        c.cod_len(),
        c.apex_len(),
        c.scalar_count()
    )
}

/// Assert two Frobenius terms are equal as morphisms of `Cospan`, reporting the
/// measured shape of both sides on failure.
fn assert_same_cospan(got: &FM, want: &FM, what: &str) {
    let (g, w) = (canon(got), canon(want));
    assert_eq!(
        g,
        w,
        "{what}: got {} vs want {}\n  got classes:  {:?}\n  want classes: {:?}",
        digest(&g),
        digest(&w),
        g.classes(),
        w.classes()
    );
}

/// Assert a Frobenius term's image is a given hand-built cospan, reporting the
/// measured shape of both sides on failure.
fn assert_image_is(got: &FM, want: &Cospan<char>, what: &str) {
    let (g, w) = (canon(got), want.canonical_form());
    assert_eq!(
        g,
        w,
        "{what}: got {} vs want {}\n  got classes:  {:?}\n  want classes: {:?}",
        digest(&g),
        digest(&w),
        g.classes(),
        w.classes()
    );
}

/// The sample terms every content pin below ranges over, as
/// `(label, f, domain, codomain)`. The two interface columns are declared data
/// beside the term; `samples_are_well_formed` checks them against `f` itself so
/// they cannot drift.
///
/// Ten terms: both identities, all four Frobenius generators, the braiding, two
/// two-layer composites, and one spider. This is the *whole* space these pins
/// quantify over — they are not statements about arbitrary morphisms, and in
/// particular no term here carries a black box or a label other than `'a'`/`'b'`.
fn samples() -> Vec<(&'static str, FM, Vec<char>, Vec<char>)> {
    let braid: FM = FrobeniusOperation::SymmetricBraiding('a', 'b').into();

    let mut delta_mu: FM = FrobeniusOperation::Comultiplication('a').into();
    delta_mu
        .compose(FrobeniusOperation::Multiplication('a').into())
        .expect("δ;μ interfaces match");

    let mut mu_delta: FM = FrobeniusOperation::Multiplication('a').into();
    mu_delta
        .compose(FrobeniusOperation::Comultiplication('a').into())
        .expect("μ;δ interfaces match");

    vec![
        ("id_a", FM::identity(&vec!['a']), vec!['a'], vec!['a']),
        (
            "id_ab",
            FM::identity(&vec!['a', 'b']),
            vec!['a', 'b'],
            vec!['a', 'b'],
        ),
        (
            "mu",
            FrobeniusOperation::Multiplication('a').into(),
            vec!['a', 'a'],
            vec!['a'],
        ),
        (
            "delta",
            FrobeniusOperation::Comultiplication('a').into(),
            vec!['a'],
            vec!['a', 'a'],
        ),
        (
            "eta",
            FrobeniusOperation::Unit('a').into(),
            vec![],
            vec!['a'],
        ),
        (
            "epsilon",
            FrobeniusOperation::Counit('a').into(),
            vec!['a'],
            vec![],
        ),
        ("braid_ab", braid, vec!['a', 'b'], vec!['b', 'a']),
        ("delta_then_mu", delta_mu, vec!['a'], vec!['a']),
        ("mu_then_delta", mu_delta, vec!['a', 'a'], vec!['a', 'a']),
        (
            "spider_2_3",
            FrobeniusOperation::Spider('a', 2, 3).into(),
            vec!['a', 'a'],
            vec!['a', 'a', 'a'],
        ),
    ]
}

/// The declared `(domain, codomain)` columns of [`samples`] are the term's own.
///
/// Without this, the codomain column is inert — every call site destructures it
/// as `_y` — and could drift out of sync with the term beside it unnoticed. The
/// domain column is load-bearing (it supplies `x.len()` to `unname`), but a
/// wrong value there would surface only as a confusing roundtrip failure.
///
/// **Space:** the ten [`samples`]. This is an interface check, not a content
/// one: it says the columns describe the term, not that the term is right.
#[test]
fn samples_are_well_formed() {
    for (label, f, x, y) in samples() {
        assert_eq!(f.domain(), x, "{label}: declared domain != f.domain()");
        assert_eq!(
            f.codomain(),
            y,
            "{label}: declared codomain != f.codomain()"
        );
    }
}

/// Bend a cospan's left leg round to the right: `X → A ← Y` becomes
/// `I → A ← X ⊕ Y`.
///
/// This is the compact-closed *name* computed directly on the cospan, without
/// going through cup/cap — the independent reference `name` is checked against.
fn bend_left_leg(c: &Cospan<char>) -> Cospan<char> {
    let mut right = c.left_to_middle().to_vec();
    right.extend_from_slice(c.right_to_middle());
    Cospan::new(vec![], right, c.middle().to_vec())
        .expect("legs are copied from a valid cospan, so they stay in the apex")
}

/// The inverse of [`bend_left_leg`]: split `I → A ← X ⊕ Y` at `x_len` back into
/// `X → A ← Y`. The independent reference `unname` is checked against.
fn unbend_right_leg(c: &Cospan<char>, x_len: usize) -> Cospan<char> {
    assert!(
        c.left_to_middle().is_empty(),
        "unbend expects a name (domain I), got a {}-wide domain",
        c.left_to_middle().len()
    );
    let right = c.right_to_middle();
    Cospan::new(
        right[..x_len].to_vec(),
        right[x_len..].to_vec(),
        c.middle().to_vec(),
    )
    .expect("legs are copied from a valid cospan, so they stay in the apex")
}

// ---------------------------------------------------------------------------
// §3.1 Prop 3.1: cup = η;δ, cap = μ;ε
// ---------------------------------------------------------------------------

#[test]
fn cup_is_unit_then_comult() {
    let z = 'a';
    let c: FM = cup_single(z);
    assert!(c.domain().is_empty(), "cup: I → X⊗X, domain = I");
    assert_eq!(c.codomain(), vec![z, z], "cup: I → X⊗X");
    assert!(
        c.depth() >= 1,
        "cup should have at least 1 layer after simplification"
    );
}

#[test]
fn cap_is_mult_then_counit() {
    let z = 'b';
    let c: FM = cap_single(z);
    assert_eq!(c.domain(), vec![z, z], "cap: X⊗X → I");
    assert!(c.codomain().is_empty(), "cap: X⊗X → I, codomain = I");
    assert!(
        c.depth() >= 1,
        "cap should have at least 1 layer after simplification"
    );
}

// ---------------------------------------------------------------------------
// §3.1 Eq. (13): Zigzag identities
// ---------------------------------------------------------------------------

#[test]
fn zigzag_right_snake_char() {
    let z = 'z';
    let mut first: FM = cup_single(z);
    first.monoidal(FM::identity(&vec![z]));
    let mut second: FM = FM::identity(&vec![z]);
    second.monoidal(cap_single(z));
    let mut snake = first;
    snake.compose(second).expect("zigzag composition");
    assert_eq!(snake.domain(), vec![z]);
    assert_eq!(snake.codomain(), vec![z]);
}

#[test]
fn zigzag_left_snake_char() {
    let z = 'z';
    let mut first: FM = FM::identity(&vec![z]);
    first.monoidal(cup_single(z));
    let mut second: FM = cap_single(z);
    second.monoidal(FM::identity(&vec![z]));
    let mut snake = first;
    snake.compose(second).expect("zigzag composition");
    assert_eq!(snake.domain(), vec![z]);
    assert_eq!(snake.codomain(), vec![z]);
}

#[test]
fn zigzag_right_snake_unit_type() {
    #[allow(clippy::upper_case_acronyms)] // local test alias
    type UFM = FrobeniusMorphism<(), String>;
    let z = ();
    let mut first: UFM = cup_single(z);
    first.monoidal(UFM::identity(&vec![z]));
    let mut second: UFM = UFM::identity(&vec![z]);
    second.monoidal(cap_single(z));
    let mut snake = first;
    snake.compose(second).expect("zigzag composition");
    assert_eq!(snake.domain(), vec![()]);
    assert_eq!(snake.codomain(), vec![()]);
}

// ---------------------------------------------------------------------------
// Monoidal structure of cup/cap (paired ordering)
// ---------------------------------------------------------------------------

#[test]
fn cup_multi_is_monoidal_product() {
    let c: FM = cup(&['a', 'b', 'c']);
    assert!(c.domain().is_empty());
    assert_eq!(c.codomain(), vec!['a', 'a', 'b', 'b', 'c', 'c']);
}

#[test]
fn cap_multi_is_monoidal_product() {
    let c: FM = cap(&['a', 'b', 'c']);
    assert_eq!(c.domain(), vec!['a', 'a', 'b', 'b', 'c', 'c']);
    assert!(c.codomain().is_empty());
}

#[test]
fn cap_then_cup_is_bubble() {
    let z = 'm';
    let mut bubble: FM = cap_single(z);
    bubble.compose(cup_single(z)).expect("[] interface");
    assert_eq!(bubble.domain(), vec![z, z]);
    assert_eq!(bubble.codomain(), vec![z, z]);
}

#[test]
fn cup_then_cap_is_dimension() {
    let z = 'n';
    let mut dim: FM = cup_single(z);
    dim.compose(cap_single(z)).expect("[z,z] interface");
    assert!(dim.domain().is_empty());
    assert!(dim.codomain().is_empty());
}

// ---------------------------------------------------------------------------
// Frobenius trait interpretation of cup/cap
// ---------------------------------------------------------------------------

#[test]
#[allow(clippy::similar_names)] // mathematical pairings (unit/comult, mult/counit) are intentional
fn cup_cap_frobenius_interpret() {
    let z = 'f';
    let unit = FM::interpret_unit(z);
    let comult = FM::interpret_comultiplication(z);
    let mut frob_cup = unit;
    frob_cup.compose(comult).expect("η;δ");
    assert_eq!(frob_cup.domain(), cup_single::<_, String>(z).domain());
    assert_eq!(frob_cup.codomain(), cup_single::<_, String>(z).codomain());

    let mult = FM::interpret_multiplication(z);
    let counit = FM::interpret_counit(z);
    let mut frob_cap = mult;
    frob_cap.compose(counit).expect("μ;ε");
    assert_eq!(frob_cap.domain(), cap_single::<_, String>(z).domain());
    assert_eq!(frob_cap.codomain(), cap_single::<_, String>(z).codomain());
}

// ---------------------------------------------------------------------------
// Edge cases (cup/cap)
// ---------------------------------------------------------------------------

#[test]
fn cup_cap_empty_types() {
    let c: FM = cup(&[]);
    assert!(c.domain().is_empty());
    assert!(c.codomain().is_empty());
    let c: FM = cap(&[]);
    assert!(c.domain().is_empty());
    assert!(c.codomain().is_empty());
}

#[test]
fn cup_cap_single_element_slice() {
    let c: FM = cup(&['x']);
    assert!(c.domain().is_empty());
    assert_eq!(c.codomain(), vec!['x', 'x']);
    let c: FM = cap(&['x']);
    assert_eq!(c.domain(), vec!['x', 'x']);
    assert!(c.codomain().is_empty());
}

// ---------------------------------------------------------------------------
// §3.1 Prop 3.1 (tensor ordering): cup_tensor / cap_tensor
// ---------------------------------------------------------------------------

#[test]
fn cup_tensor_produces_x_tensor_x() {
    let c: FM = cup_tensor(&['a', 'b']);
    assert!(c.domain().is_empty());
    assert_eq!(
        c.codomain(),
        vec!['a', 'b', 'a', 'b'],
        "cup_tensor: I → X⊗X"
    );
}

#[test]
fn cap_tensor_accepts_x_tensor_x() {
    let c: FM = cap_tensor(&['a', 'b']);
    assert_eq!(c.domain(), vec!['a', 'b', 'a', 'b'], "cap_tensor: X⊗X → I");
    assert!(c.codomain().is_empty());
}

#[test]
fn cup_tensor_three_types_ordering() {
    let c: FM = cup_tensor(&['x', 'y', 'z']);
    assert_eq!(c.codomain(), vec!['x', 'y', 'z', 'x', 'y', 'z']);
}

#[test]
fn cup_tensor_single_matches_cup_single() {
    let tensor: FM = cup_tensor(&['a']);
    let single: FM = cup_single('a');
    assert_eq!(tensor.domain(), single.domain());
    assert_eq!(tensor.codomain(), single.codomain());
}

#[test]
fn cap_tensor_single_matches_cap_single() {
    let tensor: FM = cap_tensor(&['a']);
    let single: FM = cap_single('a');
    assert_eq!(tensor.domain(), single.domain());
    assert_eq!(tensor.codomain(), single.codomain());
}

#[test]
fn cup_tensor_cap_tensor_compose() {
    let types = &['a', 'b', 'c'];
    let mut dim: FM = cup_tensor(types);
    dim.compose(cap_tensor(types)).expect("X⊗X interface");
    assert!(dim.domain().is_empty());
    assert!(dim.codomain().is_empty());
}

#[test]
fn cap_tensor_cup_tensor_compose() {
    let types = &['a', 'b'];
    let mut bubble: FM = cap_tensor(types);
    bubble.compose(cup_tensor(types)).expect("I interface");
    assert_eq!(bubble.domain(), vec!['a', 'b', 'a', 'b']);
    assert_eq!(bubble.codomain(), vec!['a', 'b', 'a', 'b']);
}

// ---------------------------------------------------------------------------
// §3.1 Prop 3.2: Name bijection
// ---------------------------------------------------------------------------

#[test]
fn name_of_identity_single() {
    let id: FM = FM::identity(&vec!['a']);
    let named = name(&id).unwrap();
    assert!(named.domain().is_empty(), "name: I → X⊗Y");
    assert_eq!(named.codomain(), vec!['a', 'a'], "name(id_a): I → a⊗a");
}

#[test]
fn name_of_identity_multi() {
    let id: FM = FM::identity(&vec!['a', 'b']);
    let named = name(&id).unwrap();
    assert!(named.domain().is_empty());
    assert_eq!(named.codomain(), vec!['a', 'b', 'a', 'b']);
}

#[test]
fn name_of_unit() {
    // η: [] → [z], name(η) = cup_[] ; (id_[] ⊗ η) = η
    let unit: FM = FrobeniusOperation::Unit('a').into();
    let named = name(&unit).unwrap();
    assert!(named.domain().is_empty());
    assert_eq!(named.codomain(), vec!['a']);
}

#[test]
fn name_of_counit() {
    // ε: [z] → [], name(ε) = cup_z ; (id_z ⊗ ε) : I → [z]
    let counit: FM = FrobeniusOperation::Counit('a').into();
    let named = name(&counit).unwrap();
    assert!(named.domain().is_empty());
    assert_eq!(named.codomain(), vec!['a']);
}

#[test]
fn name_of_multiplication() {
    // μ: [z,z] → [z], name(μ): I → [z,z,z]
    let mult: FM = FrobeniusOperation::Multiplication('a').into();
    let named = name(&mult).unwrap();
    assert!(named.domain().is_empty());
    assert_eq!(named.codomain(), vec!['a', 'a', 'a']);
}

/// Roundtrip: unname(name(f)) has same domain/codomain as f.
#[test]
fn unname_name_roundtrip_identity() {
    let id: FM = FM::identity(&vec!['x']);
    let named = name(&id).unwrap();
    let recovered = unname(&named, 1).unwrap();
    assert_eq!(recovered.domain(), id.domain());
    assert_eq!(recovered.codomain(), id.codomain());
}

#[test]
fn unname_name_roundtrip_multi_type() {
    let types = vec!['a', 'b'];
    let id: FM = FM::identity(&types);
    let named = name(&id).unwrap();
    let recovered = unname(&named, 2).unwrap();
    assert_eq!(recovered.domain(), types);
    assert_eq!(recovered.codomain(), types);
}

#[test]
fn unname_name_roundtrip_multiplication() {
    let mult: FM = FrobeniusOperation::Multiplication('a').into();
    let named = name(&mult).unwrap();
    let recovered = unname(&named, 2).unwrap();
    assert_eq!(recovered.domain(), vec!['a', 'a']);
    assert_eq!(recovered.codomain(), vec!['a']);
}

#[test]
fn unname_rejects_nonempty_domain() {
    let f: FM = FM::identity(&vec!['a']);
    assert!(unname(&f, 1).is_err());
}

#[test]
fn unname_rejects_x_len_overflow() {
    let g: FM = cup_single('a');
    assert!(unname(&g, 5).is_err());
}

// ---------------------------------------------------------------------------
// §3.1 Props 3.3-3.4: Composition via names
// ---------------------------------------------------------------------------

/// `compose_names(name(id)`, name(id)) = name(id;id) = name(id)
#[test]
fn compose_names_identities() {
    let id: FM = FM::identity(&vec!['a']);
    let f_hat = name(&id).unwrap();
    let g_hat = name(&id).unwrap();
    let result = compose_names(&f_hat, &g_hat, 1, 1).unwrap();
    assert!(result.domain().is_empty());
    assert_eq!(result.codomain(), vec!['a', 'a']);
}

/// `compose_names` matches name(f;g) in domain/codomain.
#[test]
fn compose_names_matches_direct_composition() {
    let f: FM = FrobeniusOperation::Comultiplication('a').into(); // [a] → [a,a]
    let g: FM = FrobeniusOperation::Multiplication('a').into(); // [a,a] → [a]

    // Direct: name(f;g)
    let mut fg = f.clone();
    fg.compose(g.clone()).unwrap();
    let direct = name(&fg).unwrap();

    // Via names: compose_names(name(f), name(g))
    let f_hat = name(&f).unwrap(); // I → [a, a, a]
    let g_hat = name(&g).unwrap(); // I → [a, a, a]
    let via_names = compose_names(&f_hat, &g_hat, 1, 2).unwrap();

    assert_eq!(via_names.domain(), direct.domain());
    assert_eq!(via_names.codomain(), direct.codomain());
}

/// Prop 3.4: (`id_X` ⊕ f̂) ; comp = f — recovery of f from its name.
#[test]
fn recovery_from_name() {
    let f: FM = FrobeniusOperation::Comultiplication('a').into(); // [a] → [a,a]
    let f_hat = name(&f).unwrap();
    let recovered = unname(&f_hat, 1).unwrap();
    assert_eq!(recovered.domain(), f.domain());
    assert_eq!(recovered.codomain(), f.codomain());
}

/// Prop 3.4 literal form — build the recovery explicitly without relying on `unname`.
///
/// For `f: X → Y`, construct `f̂: I → X ⊗ Y` via `name`, then build the composition
/// cospan `comp^X_{∅,Y} = cap_X ⊗ id_Y: X ⊗ X ⊗ Y → Y` from scratch, and verify that
///
/// ```text
/// (id_X ⊗ f̂) ; comp^X_{∅,Y} = f
/// ```
///
/// This exercises the paper's formula structurally rather than going through the
/// `unname` helper, so a regression in either `name` or the comp cospan would
/// surface here even if `unname` is defined to short-circuit.
///
/// Since #284 the final assertion compares the *content* — the image in `Cospan`
/// up to apex isomorphism — not just `f`'s domain and codomain, which the
/// discard-inputs/create-outputs junk the audit substituted also reproduced.
fn prop_3_4_recover_via_explicit_comp(f: &FM, x: &[char], y: &[char]) {
    let f_hat = name(f).unwrap();
    assert!(f_hat.domain().is_empty(), "f̂ must have domain I");
    assert_eq!(
        f_hat.codomain(),
        [x, y].concat(),
        "f̂ codomain must be X ⊗ Y"
    );

    // (id_X ⊗ f̂): X → X ⊗ X ⊗ Y
    let mut lhs: FM = FM::identity(&x.to_vec());
    lhs.monoidal(f_hat);
    assert_eq!(lhs.domain(), x.to_vec());
    assert_eq!(lhs.codomain(), [x, x, y].concat());

    // comp^X_{∅,Y} = cap_X ⊗ id_Y: X ⊗ X ⊗ Y → Y
    let mut comp: FM = cap_tensor(x);
    comp.monoidal(FM::identity(&y.to_vec()));
    assert_eq!(comp.domain(), [x, x, y].concat());
    assert_eq!(comp.codomain(), y.to_vec());

    // (id_X ⊗ f̂) ; comp^X_{∅,Y}
    lhs.compose(comp).expect("Prop 3.4 interfaces align");

    // Result must be f: X → Y — same interface *and* same morphism.
    assert_eq!(lhs.domain(), f.domain());
    assert_eq!(lhs.codomain(), f.codomain());
    assert_same_cospan(&lhs, f, "Prop 3.4: (id_X ⊗ f̂) ; comp != f");
}

#[test]
fn prop_3_4_identity_single() {
    let f: FM = FM::identity(&vec!['a']);
    prop_3_4_recover_via_explicit_comp(&f, &['a'], &['a']);
}

#[test]
fn prop_3_4_identity_multi() {
    let f: FM = FM::identity(&vec!['a', 'b']);
    prop_3_4_recover_via_explicit_comp(&f, &['a', 'b'], &['a', 'b']);
}

#[test]
fn prop_3_4_multiplication() {
    // Multiplication: [a, a] → [a]
    let f: FM = FrobeniusOperation::Multiplication('a').into();
    prop_3_4_recover_via_explicit_comp(&f, &['a', 'a'], &['a']);
}

#[test]
fn prop_3_4_comultiplication() {
    // Comultiplication: [a] → [a, a]
    let f: FM = FrobeniusOperation::Comultiplication('a').into();
    prop_3_4_recover_via_explicit_comp(&f, &['a'], &['a', 'a']);
}

#[test]
fn prop_3_4_unit_to_mult() {
    // Unit ; Comult: [] → [a] → [a, a]
    let unit: FM = FrobeniusOperation::Unit('a').into();
    let comult: FM = FrobeniusOperation::Comultiplication('a').into();
    let mut f = unit;
    f.compose(comult).unwrap();
    prop_3_4_recover_via_explicit_comp(&f, &[], &['a', 'a']);
}

/// `compose_names` rejects non-empty domain inputs.
#[test]
fn compose_names_rejects_nonempty_domain() {
    let id: FM = FM::identity(&vec!['a']);
    let named = name(&id).unwrap();
    assert!(compose_names(&id, &named, 1, 1).is_err());
    assert!(compose_names(&named, &id, 1, 1).is_err());
}

// ---------------------------------------------------------------------------
// Prop 3.3 literal formula: compose_names_direct vs compose_names_via_unname
// ---------------------------------------------------------------------------

/// Assert both `compose_names` implementations agree *as morphisms* for a given
/// `(f, g)` pair, and that both equal `name(f;g)`.
///
/// `compose_names_direct` implements Prop 3.3's literal formula
/// `(f̂ ⊗ ĝ) ; comp^Y_{X,Z}`. `compose_names_via_unname` factors through the
/// name bijection as `name(unname(f̂); unname(ĝ))`. They are mathematically
/// equal, so their images in `Cospan` must be isomorphic — not merely their
/// codomains, which is all this helper compared before #284. Gutting either
/// leg (the audit gutted the direct one) leaves every codomain unchanged.
fn assert_compose_names_equivalent(f: &FM, g: &FM, x_len: usize, y_len: usize) {
    let f_hat = name(f).unwrap();
    let g_hat = name(g).unwrap();
    let direct = compose_names_direct(&f_hat, &g_hat, x_len, y_len).unwrap();
    let via = compose_names_via_unname(&f_hat, &g_hat, x_len, y_len).unwrap();
    assert!(direct.domain().is_empty());
    assert!(via.domain().is_empty());
    assert_eq!(
        direct.codomain(),
        via.codomain(),
        "codomain mismatch between direct and via_unname"
    );
    assert_same_cospan(&direct, &via, "compose_names_direct vs via_unname");

    let mut fg = f.clone();
    fg.compose(g.clone()).unwrap();
    let expected = name(&fg).unwrap();
    assert_eq!(
        direct.codomain(),
        expected.codomain(),
        "compose_names_direct codomain disagrees with name(f;g)"
    );
    assert_same_cospan(
        &direct,
        &expected,
        "compose_names_direct vs name(f;g) (Prop 3.3)",
    );
}

#[test]
fn compose_names_direct_identities_single() {
    let f: FM = FM::identity(&vec!['a']);
    let g: FM = FM::identity(&vec!['a']);
    assert_compose_names_equivalent(&f, &g, 1, 1);
}

#[test]
fn compose_names_direct_identities_multi() {
    let f: FM = FM::identity(&vec!['a', 'b']);
    let g: FM = FM::identity(&vec!['a', 'b']);
    assert_compose_names_equivalent(&f, &g, 2, 2);
}

#[test]
fn compose_names_direct_comult_mult() {
    // f = Δ: [a] → [a,a], g = μ: [a,a] → [a]
    let f: FM = FrobeniusOperation::Comultiplication('a').into();
    let g: FM = FrobeniusOperation::Multiplication('a').into();
    // f_hat codomain = [a] ++ [a, a] = [a, a, a], split x=1, y=2
    // g_hat codomain = [a, a] ++ [a] = [a, a, a], split y=2, z=1
    assert_compose_names_equivalent(&f, &g, 1, 2);
}

#[test]
fn compose_names_direct_unit_to_identity() {
    // f = η: [] → [a], g = id: [a] → [a]
    let f: FM = FrobeniusOperation::Unit('a').into();
    let g: FM = FM::identity(&vec!['a']);
    // f_hat codomain = [] ++ [a] = [a], split x=0, y=1
    // g_hat codomain = [a] ++ [a] = [a, a], split y=1, z=1
    assert_compose_names_equivalent(&f, &g, 0, 1);
}

#[test]
fn compose_names_direct_rejects_nonempty_domain() {
    let id: FM = FM::identity(&vec!['a']);
    let named = name(&id).unwrap();
    assert!(compose_names_direct(&id, &named, 1, 1).is_err());
    assert!(compose_names_direct(&named, &id, 1, 1).is_err());
}

#[test]
fn compose_names_direct_rejects_mismatched_y() {
    // f̂: I → [a, b] (x=[a], y=[b])
    // ĝ: I → [c, d] (y=[c], z=[d]) — b ≠ c, should reject
    let f: FM = FM::identity(&vec!['a']);
    let mut g_raw: FM = FrobeniusOperation::Unit('c').into();
    g_raw.monoidal(FrobeniusOperation::Unit('d').into());
    let mut f_raw: FM = FrobeniusOperation::Unit('a').into();
    f_raw.monoidal(FrobeniusOperation::Unit('b').into());
    // Here f_raw: I → [a, b] already has domain I, so treat it as f_hat directly.
    assert!(compose_names_direct(&f_raw, &g_raw, 1, 1).is_err());
    // Silence unused warning for f.
    let _ = f;
}

// ---------------------------------------------------------------------------
// Content pins (#284 / WI-C02)
//
// Everything above this line asserts domain/codomain (and, in a few places,
// `depth() >= 1`). That is a type-level check: the audit measured that
// replacing `unname` with discard-inputs/create-outputs junk, and
// `compose_names_direct` with one that drops f̂ and ĝ for bare units, both left
// all 44 tests green. The pins below compare the *content* — the image in
// `Cospan` up to apex isomorphism, i.e. SCFM-equality of the *images* (`Cospan`
// is the free SCFM prop, F&S 2019 Prop 3.8); on *terms* this relation is
// incomparable with SCFM-equality on scalars, see the header above — against
// references built without the function under test.
// ---------------------------------------------------------------------------

/// `cup_tensor`/`cap_tensor` are the bent identity, not merely a morphism of the
/// right shape.
///
/// Reference: the cospan whose apex *is* `X` and whose one non-empty leg is
/// `[0..n] ++ [0..n]` — built by hand, with no call to cup/cap. A `cup` that
/// created `2n` unconnected vertices instead of `n` shared ones has the same
/// domain and codomain and is caught only here.
///
/// **Space:** `X` of length 0–3 over the two labels below, i.e. the `n <= 1`
/// short-circuit and the deinterleave-permutation path for `n = 2, 3`. Nothing
/// is claimed for longer `X` or for other label sets.
#[test]
fn cup_cap_tensor_are_the_bent_identity() {
    for types in [
        vec![],
        vec!['a'],
        vec!['a', 'b'],
        vec!['a', 'a'],
        vec!['a', 'b', 'a'],
    ] {
        let n = types.len();
        let doubled: Vec<usize> = (0..n).chain(0..n).collect();

        let cup_ref = Cospan::new(vec![], doubled.clone(), types.clone())
            .expect("both legs index the apex by construction");
        assert_image_is(
            &cup_tensor::<char, String>(&types),
            &cup_ref,
            &format!("cup_tensor({types:?})"),
        );

        let cap_ref = Cospan::new(doubled, vec![], types.clone())
            .expect("both legs index the apex by construction");
        assert_image_is(
            &cap_tensor::<char, String>(&types),
            &cap_ref,
            &format!("cap_tensor({types:?})"),
        );
    }
}

/// Eq. (13): both snakes really are `id_X`, with no leftover bubble.
///
/// The existing `zigzag_*` tests assert only that the snake is an endomorphism
/// of `X`; so is the constant-`ε;η` map, and so is `id_X ⊗ (a bubble)`. Here the
/// snake's cospan must equal `id_X`'s on the nose: same apex size (so no scalar
/// is left dangling) and the same wire-for-wire connectivity.
///
/// **Space:** `X` of length 0–3 over `{'a','b'}`, both snake orientations —
/// but **length 0 pins the empty short-circuit only**, not Eq. (13):
/// `cup_tensor(&[])` and `cap_tensor(&[])` both return through `cup`/`cap`'s
/// `types.is_empty()` early return to `FM::identity(&vec![])`, so both snakes
/// are `id_I ; id_I` and nothing about cup/cap connectivity can fail there.
/// The law itself is exercised at lengths 1–3.
/// Specialness itself (`δ;μ = id`) is pinned in `frobenius_laws.rs`; this test
/// claims only that the two zigzags reduce. A bubble reaching
/// `frobenius_to_cospan` *is* caught here, since it changes the apex size —
/// and since #350 the simplifier no longer removes any before it gets there
/// (see the header).
#[test]
fn zigzag_snakes_reduce_to_the_identity() {
    for types in [
        vec![],
        vec!['a'],
        vec!['a', 'b'],
        vec!['a', 'a'],
        vec!['a', 'b', 'a'],
    ] {
        let id: FM = FM::identity(&types);

        // (cup_X ⊗ id_X) ; (id_X ⊗ cap_X)
        let mut right_snake: FM = cup_tensor(&types);
        right_snake.monoidal(FM::identity(&types));
        let mut second: FM = FM::identity(&types);
        second.monoidal(cap_tensor(&types));
        right_snake
            .compose(second)
            .expect("zigzag interfaces align");
        assert_same_cospan(
            &right_snake,
            &id,
            &format!("right snake on {types:?} is not id"),
        );

        // (id_X ⊗ cup_X) ; (cap_X ⊗ id_X)
        let mut left_snake: FM = FM::identity(&types);
        left_snake.monoidal(cup_tensor(&types));
        let mut second: FM = cap_tensor(&types);
        second.monoidal(FM::identity(&types));
        left_snake.compose(second).expect("zigzag interfaces align");
        assert_same_cospan(
            &left_snake,
            &id,
            &format!("left snake on {types:?} is not id"),
        );
    }
}

/// Prop 3.2, forward: `name(f)` bends `f`'s left leg round to the right.
///
/// Reference: [`bend_left_leg`] applied to `f`'s own cospan — the compact-closed
/// name computed on the cospan side, touching neither `cup_tensor` nor `name`.
/// Equality of the two says `name` transports `f` intact; a `name` that dropped
/// `f` would keep the codomain `X ⊗ Y` and fail only here.
///
/// **Space:** the ten [`samples`], each with its own `X`.
#[test]
fn name_bends_the_left_leg() {
    for (label, f, _x, _y) in samples() {
        let image = frobenius_to_cospan(&f).expect("no black boxes");
        let reference = bend_left_leg(&image);
        assert_image_is(
            &name(&f).expect("name of a well-formed morphism"),
            &reference,
            &format!("name({label})"),
        );
    }
}

/// Prop 3.2, backward: `unname(ĝ, |X|)` unbends the right leg at `|X|`.
///
/// Reference: [`unbend_right_leg`] on `ĝ`'s own cospan. This is the pin the
/// audit's first measurement needs — `unname` replaced by
/// discard-inputs/create-outputs junk keeps every domain/codomain in the file
/// and is caught here, because the junk's apex has `|X| + |Y|` singleton
/// vertices where the real one has `f`'s.
///
/// **Space:** the names of the ten [`samples`]. Names of morphisms that are not
/// themselves in the image of `name` are not exercised.
#[test]
fn unname_unbends_the_right_leg() {
    for (label, f, x, _y) in samples() {
        let f_hat = name(&f).expect("name of a well-formed morphism");
        let hat_image = frobenius_to_cospan(&f_hat).expect("no black boxes");
        let reference = unbend_right_leg(&hat_image, x.len());
        assert_image_is(
            &unname(&f_hat, x.len()).expect("unname of a name"),
            &reference,
            &format!("unname(name({label}), {})", x.len()),
        );
    }
}

/// Prop 3.2 as a bijection: `unname(name(f), |X|) = f`, by content.
///
/// The existing `unname_name_roundtrip_*` tests compare domain and codomain,
/// which the identity-shaped junk also satisfies. This compares the cospans.
///
/// **Space:** the ten [`samples`]. Round-tripping the other way
/// (`name(unname(ĝ))`) is covered for names only, in the test above.
#[test]
fn name_unname_roundtrip_preserves_content() {
    for (label, f, x, _y) in samples() {
        let f_hat = name(&f).expect("name of a well-formed morphism");
        let recovered = unname(&f_hat, x.len()).expect("unname of a name");
        assert_same_cospan(&recovered, &f, &format!("roundtrip on {label}"));
    }
}

/// Prop 3.3: `(f̂ ⊗ ĝ) ; comp^Y_{X,Z} = (f;g)^`, by content.
///
/// This is the pin the audit's second measurement needs: a
/// `compose_names_direct` that discards f̂ and ĝ and returns bare units has the
/// right codomain `X ⊗ Z` and passes every pre-existing assertion. Here the
/// result must be `name(f;g)` as a cospan — apex, connectivity and all.
///
/// **Space:** the nine composable pairs below, over the labels `'a'`/`'b'`,
/// including one pair whose `X` is empty and one spider pair.
#[test]
fn compose_names_direct_is_the_name_of_the_composite() {
    for (label, f, g, x_len, y_len) in composable_pairs() {
        let f_hat = name(&f).expect("name of f");
        let g_hat = name(&g).expect("name of g");
        let via_prop_3_3 =
            compose_names_direct(&f_hat, &g_hat, x_len, y_len).expect("Prop 3.3 formula");

        let mut fg = f.clone();
        fg.compose(g.clone()).expect("f;g interfaces align");
        let expected = name(&fg).expect("name of f;g");

        assert_same_cospan(
            &via_prop_3_3,
            &expected,
            &format!("compose_names_direct != name(f;g) for {label}"),
        );
    }
}

/// Ex. 3.5 / Eq. (14): the comp cospan `compact_closed` builds out of
/// `id ⊗ cap ⊗ id` is the one [`comp_cospan`] builds out of index arithmetic.
///
/// Two independent implementations in two modules that never call each other —
/// this is the cross-check that keeps `compose_names_direct`'s middle factor
/// honest even if every name-level test were satisfied by a coincidence.
///
/// **Space:** the eight `(X, Y, Z)` shapes below, with `|Y|` from 0 to 3 — so
/// both the `n <= 1` short-circuit and the permutation path inside `cap_tensor`
/// are covered. Labels are `'a'`/`'b'`/`'c'` only.
#[test]
fn frobenius_comp_matches_comp_cospan() {
    let shapes: [(Vec<char>, Vec<char>, Vec<char>); 8] = [
        (vec![], vec![], vec![]),
        (vec!['a'], vec![], vec!['c']),
        (vec![], vec!['b'], vec![]),
        (vec!['a'], vec!['b'], vec!['c']),
        (vec!['a'], vec!['b', 'b'], vec!['c']),
        (vec!['a', 'a'], vec!['b', 'c'], vec![]),
        (vec![], vec!['b', 'c', 'b'], vec!['c']),
        (vec!['a', 'b'], vec!['b', 'c', 'a'], vec!['c', 'a']),
    ];
    for (x, y, z) in shapes {
        let mut comp: FM = FM::identity(&x);
        comp.monoidal(cap_tensor(&y));
        comp.monoidal(FM::identity(&z));
        assert_image_is(
            &comp,
            &comp_cospan(&x, &y, &z),
            &format!("id ⊗ cap_{y:?} ⊗ id vs comp_cospan({x:?}, {y:?}, {z:?})"),
        );
    }
}

/// The `(f, g, x_len, y_len)` pairs the Prop 3.3 pins range over. `x_len` is
/// `|dom f|` and `y_len` is `|cod f| = |dom g|`, the two splits Prop 3.3 needs.
fn composable_pairs() -> Vec<(&'static str, FM, FM, usize, usize)> {
    let mut delta_mu: FM = FrobeniusOperation::Comultiplication('a').into();
    delta_mu
        .compose(FrobeniusOperation::Multiplication('a').into())
        .expect("δ;μ interfaces match");

    vec![
        (
            "id;id",
            FM::identity(&vec!['a']),
            FM::identity(&vec!['a']),
            1,
            1,
        ),
        (
            "id_ab;id_ab",
            FM::identity(&vec!['a', 'b']),
            FM::identity(&vec!['a', 'b']),
            2,
            2,
        ),
        (
            "delta;mu",
            FrobeniusOperation::Comultiplication('a').into(),
            FrobeniusOperation::Multiplication('a').into(),
            1,
            2,
        ),
        (
            "mu;delta",
            FrobeniusOperation::Multiplication('a').into(),
            FrobeniusOperation::Comultiplication('a').into(),
            2,
            1,
        ),
        (
            "eta;id",
            FrobeniusOperation::Unit('a').into(),
            FM::identity(&vec!['a']),
            0,
            1,
        ),
        (
            "mu;epsilon",
            FrobeniusOperation::Multiplication('a').into(),
            FrobeniusOperation::Counit('a').into(),
            2,
            1,
        ),
        (
            "braid;braid",
            FrobeniusOperation::SymmetricBraiding('a', 'b').into(),
            FrobeniusOperation::SymmetricBraiding('b', 'a').into(),
            2,
            2,
        ),
        (
            "delta_mu;delta",
            delta_mu,
            FrobeniusOperation::Comultiplication('a').into(),
            1,
            1,
        ),
        (
            "spider_2_3;spider_3_1",
            FrobeniusOperation::Spider('a', 2, 3).into(),
            FrobeniusOperation::Spider('a', 3, 1).into(),
            2,
            3,
        ),
    ]
}
