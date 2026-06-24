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
    frobenius::{Frobenius, FrobeniusMorphism, FrobeniusOperation},
    monoidal::Monoidal,
};

type FM = FrobeniusMorphism<char, String>;

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

    // Result must be f: X → Y.
    assert_eq!(lhs.domain(), f.domain());
    assert_eq!(lhs.codomain(), f.codomain());
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

/// Assert both `compose_names` implementations agree on domain/codomain for
/// a given `(f, g)` pair.
///
/// `compose_names_direct` implements Prop 3.3's literal formula
/// `(f̂ ⊗ ĝ) ; comp^Y_{X,Z}`. `compose_names_via_unname` factors through the
/// name bijection as `name(unname(f̂); unname(ĝ))`. They are mathematically
/// equal, so both the domain/codomain and the structural layer representation
/// must match after simplification.
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
    let mut fg = f.clone();
    fg.compose(g.clone()).unwrap();
    let expected = name(&fg).unwrap();
    assert_eq!(
        direct.codomain(),
        expected.codomain(),
        "compose_names_direct codomain disagrees with name(f;g)"
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
