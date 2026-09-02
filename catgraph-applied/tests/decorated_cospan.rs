//! Integration tests for `DecoratedCospan::compose` invoking
//! `D::pushforward` through the pushout quotient.

use catgraph::category::Composable;
use catgraph::cospan::Cospan;
use catgraph_applied::decorated_cospan::{DecoratedCospan, Decoration};

/// Edge-set decoration carrying its own apex size so the laxator can shift
/// c2's edge endpoints into the disjoint-union coordinate system.
///
/// The `Decoration` trait's `combine` signature does not receive the two
/// cardinalities, so implementations whose internal representation
/// references apex indices must carry that information inside their own
/// `Apex` type. Here we store `n`, the number of apex vertices the edge
/// indices are valid against.
#[derive(Clone, Debug, PartialEq)]
struct EdgeSet {
    n: usize,
    edges: Vec<(usize, usize)>,
}

struct Circuit;

impl Decoration for Circuit {
    type Apex = EdgeSet;
    fn empty(n: usize) -> EdgeSet {
        EdgeSet { n, edges: vec![] }
    }
    fn combine(a: EdgeSet, b: EdgeSet) -> EdgeSet {
        // Laxator φ: F(n_a) × F(n_b) → F(n_a + n_b). Shift b's endpoints
        // by n_a so they index into the disjoint union [0, n_a + n_b).
        let shift = a.n;
        let mut edges = a.edges;
        edges.extend(b.edges.into_iter().map(|(u, v)| (u + shift, v + shift)));
        EdgeSet {
            n: a.n + b.n,
            edges,
        }
    }
    fn pushforward(d: EdgeSet, quotient: &[usize]) -> EdgeSet {
        let new_n = quotient.iter().copied().max().map_or(0, |m| m + 1);
        EdgeSet {
            n: new_n,
            edges: d
                .edges
                .into_iter()
                .map(|(u, v)| (quotient[u], quotient[v]))
                .collect(),
        }
    }
}

#[test]
fn t2_1_circuit_edgeset_series_composition() {
    // Two 1-resistor cospans composed in series.
    // Each apex has two vertices sharing the same label (so the interface
    // label matches when composing). left leg points at vertex 0, right
    // leg at vertex 1, and there is one edge (0, 1).
    // After pushout, the right boundary of c1 and left boundary of c2
    // identify into a single shared vertex; the two edges must be
    // relabelled into the 3-vertex apex as [(0, 1), (1, 2)].
    let c1 = Cospan::<usize>::new(vec![0], vec![1], vec![0, 0]).unwrap();
    let circ1 = DecoratedCospan::<usize, Circuit>::new(
        c1,
        EdgeSet {
            n: 2,
            edges: vec![(0, 1)],
        },
    );

    let c2 = Cospan::<usize>::new(vec![0], vec![1], vec![0, 0]).unwrap();
    let circ2 = DecoratedCospan::<usize, Circuit>::new(
        c2,
        EdgeSet {
            n: 2,
            edges: vec![(0, 1)],
        },
    );

    let series = circ1.compose(&circ2).expect("series composition");

    assert_eq!(
        series.cospan.middle().len(),
        3,
        "3 apex vertices after pushout"
    );
    assert_eq!(series.decoration.edges.len(), 2, "2 edges after combining");
    for (u, v) in &series.decoration.edges {
        assert!(*u < 3 && *v < 3, "edge endpoint out of apex range");
    }
    let mut edges = series.decoration.edges.clone();
    edges.sort_unstable();
    assert_eq!(edges, vec![(0, 1), (1, 2)]);
}

/// Local trivial decoration — `catgraph_applied::decorated_cospan::Trivial`
/// is test-module-private, so we replicate the minimal unit decoration here.
struct LocalTrivial;

impl Decoration for LocalTrivial {
    type Apex = ();
    fn empty(_: usize) {}
    fn combine((): (), (): ()) {}
    fn pushforward((): (), _: &[usize]) {}
}

#[test]
fn t2_2_trivial_pushforward_is_unit() {
    // `pushforward` is not observably exercised here because `()` is the
    // only possible value for `LocalTrivial::Apex`. The non-vacuous
    // verification of pushforward wiring lives in T2.1 (the Circuit
    // EdgeSet test).
    let c1 = Cospan::<usize>::new(vec![0], vec![0], vec![0]).unwrap();
    let d1 = DecoratedCospan::<usize, LocalTrivial>::new(c1, ());
    let c2 = Cospan::<usize>::new(vec![0], vec![0], vec![0]).unwrap();
    let d2 = DecoratedCospan::<usize, LocalTrivial>::new(c2, ());
    let composed = d1.compose(&d2).unwrap();
    assert_eq!(composed.decoration, ());
}

#[test]
fn t2_3_decorated_cospan_pushforward_through_quotient() {
    // integration test pinning that DecoratedCospan::compose
    // routes the combined decoration through D::pushforward using the
    // quotient from Cospan::compose_with_quotient. Constructed so that the
    // quotient observably collapses a multi-vertex apex into a smaller one,
    // and the edge decoration must witness the collapse via relabelling.
    //
    // c1:  domain = ['x']         codomain = ['y', 'y']
    //      middle = [0_x, 1_y, 2_y]
    //      left leg → 0  (x), right leg → [1, 2]  (both y's)
    //      edges: [(0, 1), (0, 2)]  — fan-out from the x-vertex
    //
    // c2:  domain = ['y', 'y']    codomain = ['z']
    //      middle = [0_y, 1_y, 2_z]
    //      left leg → [0, 1] (the two y's), right leg → 2 (z)
    //      edges: [(0, 2), (1, 2)]  — fan-in into the z-vertex
    //
    // Composing c1 ; c2 glues c1.right ⊃ {1_y, 2_y} with c2.left ⊃ {0_y, 1_y}
    // via pushout. The quotient identifies them pairwise so the two y-vertices
    // appear once in the pushout apex (one shared y per identification).
    // Final apex: [x, y, y, z] = 4 vertices (the two y's stay distinct because
    // c1 fans out to two separate y's and c2 fans in from two separate y's).
    //
    // The apex numbering `Cospan::compose_with_quotient` chooses is not part
    // of the contract, so the expected edge multiset is named through the
    // composed cospan's own legs: the x-vertex is the domain leg's image, the
    // z-vertex the codomain leg's image, and the two remaining apex vertices
    // are the y's. That fixes all four edges exactly —
    // `[(x,y1), (x,y2), (y1,z), (y2,z)]` as a sorted multiset — under any
    // numbering, so a quotient that merged the two y's, dropped an
    // identification, or wired x straight to z is separated from the right
    // one.
    let c1 = Cospan::<char>::new(vec![0], vec![1, 2], vec!['x', 'y', 'y']).unwrap();
    let circ1 = DecoratedCospan::<char, Circuit>::new(
        c1,
        EdgeSet {
            n: 3,
            edges: vec![(0, 1), (0, 2)],
        },
    );

    let c2 = Cospan::<char>::new(vec![0, 1], vec![2], vec!['y', 'y', 'z']).unwrap();
    let circ2 = DecoratedCospan::<char, Circuit>::new(
        c2,
        EdgeSet {
            n: 3,
            edges: vec![(0, 2), (1, 2)],
        },
    );

    let composed = circ1.compose(&circ2).expect("compose should succeed");

    let apex_size = composed.cospan.middle().len();
    assert_eq!(
        apex_size, 4,
        "pushout should identify the two y interfaces, leaving 4 apex vertices"
    );
    // Decoration's `n` carries the post-pushforward apex size — must match.
    assert_eq!(
        composed.decoration.n, apex_size,
        "decoration's apex size must match cospan apex size after pushforward"
    );
    // Edges are quotient-images of the combined edge set; count is preserved.
    assert_eq!(
        composed.decoration.edges.len(),
        4,
        "all four pre-pushout edges should survive the quotient"
    );

    // Name the three roles through the composed cospan's own legs.
    let x = composed.cospan.left_to_middle()[0];
    let z = composed.cospan.right_to_middle()[0];
    assert_eq!(composed.cospan.middle()[x], 'x');
    assert_eq!(composed.cospan.middle()[z], 'z');
    let ys: Vec<usize> = (0..apex_size).filter(|v| *v != x && *v != z).collect();
    assert_eq!(ys.len(), 2, "the two y interfaces must stay distinct");
    for &y in &ys {
        assert_eq!(composed.cospan.middle()[y], 'y');
    }

    let mut expected = vec![(x, ys[0]), (x, ys[1]), (ys[0], z), (ys[1], z)];
    expected.sort_unstable();
    let mut observed = composed.decoration.edges.clone();
    observed.sort_unstable();
    assert_eq!(
        observed, expected,
        "pushforward must send the fan-out/fan-in through the quotient: \
         x={x}, y={ys:?}, z={z}"
    );
}

#[test]
fn t2_4_petri_decoration_collapsed_quotient_preserves_transition_count() {
    // Quotient collapses both apex elements to 0, pushforward is a no-op
    // on transition count. Regression-guards the behaviour that
    // composition preserves all transitions across the pushout.
    use catgraph_applied::petri_net::{PetriApex, PetriDecoration, Transition};
    use rust_decimal::Decimal;

    let c1 = Cospan::<char>::new(vec![0], vec![0], vec!['p']).unwrap();
    let t1 = Transition::new(vec![(0, Decimal::ONE)], vec![]);
    let d1 = DecoratedCospan::<char, PetriDecoration<char>>::new(
        c1,
        PetriApex {
            n: 1,
            transitions: vec![t1],
        },
    );

    let c2 = Cospan::<char>::new(vec![0], vec![0], vec!['p']).unwrap();
    let t2 = Transition::new(vec![], vec![(0, Decimal::ONE)]);
    let d2 = DecoratedCospan::<char, PetriDecoration<char>>::new(
        c2,
        PetriApex {
            n: 1,
            transitions: vec![t2],
        },
    );

    let composed = d1.compose(&d2).unwrap();
    // Both transitions preserved — quotient collapses both apex elements
    // to 0, so pushforward is a no-op on transition count.
    assert_eq!(composed.decoration.transitions.len(), 2);
}

/// The `PetriDecoration` laxator shifts the second operand's arc place
/// indices into the right half of the tensored apex.
///
/// **What this ranges over.** One `Monoidal::monoidal` call, a one-place left
/// operand and a two-place right operand carrying one arc on its second
/// place, on `Lambda = char`. It does not sweep apex sizes or transition
/// counts.
#[test]
fn t2_5_petri_decoration_monoidal_shifts_the_second_operand() {
    use catgraph::monoidal::Monoidal;
    use catgraph_applied::petri_net::{PetriApex, PetriDecoration, Transition};
    use rust_decimal::Decimal;

    let mut left = DecoratedCospan::<char, PetriDecoration<char>>::new(
        Cospan::<char>::new(vec![0], vec![0], vec!['p']).unwrap(),
        PetriApex {
            n: 1,
            transitions: vec![Transition::new(vec![(0, Decimal::ONE)], vec![])],
        },
    );
    let right = DecoratedCospan::<char, PetriDecoration<char>>::new(
        Cospan::<char>::new(vec![0], vec![1], vec!['q', 'r']).unwrap(),
        PetriApex {
            n: 2,
            transitions: vec![Transition::new(vec![], vec![(1, Decimal::TWO)])],
        },
    );

    left.monoidal(right);

    assert_eq!(
        left.decoration.n, 3,
        "the tensored apex is 1 + 2 places, got n = {}",
        left.decoration.n
    );
    let shifted = left.decoration.transitions[1].post();
    assert_eq!(
        shifted,
        &[(2, Decimal::TWO)],
        "the right operand's arc on its place 1 must land on place 1 + 1 = 2, got: {shifted:?}"
    );
}

#[test]
fn t2_6_clone_and_debug_over_petri_decoration_marker() {
    // `PetriDecoration` derives `Debug` and not `Clone`; both calls below
    // resolve through the impls on `DecoratedCospan`, which bound
    // `D: Decoration` alone.
    use catgraph_applied::petri_net::{PetriDecoration, PetriNet, Transition};
    use rust_decimal::Decimal;

    let pn: PetriNet<char> = PetriNet::new(
        vec!['a', 'b'],
        vec![Transition::new(
            vec![(0, Decimal::ONE)],
            vec![(1, Decimal::ONE)],
        )],
        vec![0],
        vec![1],
    )
    .expect("invariant: arcs and legs index the two declared places");

    let original: DecoratedCospan<char, PetriDecoration<char>> = pn.to_decorated_cospan();
    let cloned = original.clone();

    assert_eq!(
        cloned, original,
        "the clone must compare equal to the original through the shipped \
         PartialEq; original = {original:?}, clone = {cloned:?}"
    );
    assert_eq!(
        cloned.cospan.middle(),
        original.cospan.middle(),
        "the clone carries the same apex: expected {:?}, got {:?}",
        original.cospan.middle(),
        cloned.cospan.middle()
    );
    assert_eq!(
        cloned.decoration, original.decoration,
        "the clone carries the same decoration: expected {:?}, got {:?}",
        original.decoration, cloned.decoration
    );

    let rendered = format!("{original:?}");
    assert!(
        rendered.starts_with("DecoratedCospan {"),
        "Debug names the struct, got: {rendered}"
    );
    assert!(
        rendered.contains("cospan:"),
        "Debug names the `cospan` field, got: {rendered}"
    );
    assert!(
        rendered.contains("decoration:"),
        "Debug names the `decoration` field, got: {rendered}"
    );
    assert!(
        rendered.contains("PetriApex"),
        "Debug delegates to the apex value's own Debug, got: {rendered}"
    );

    let pretty = format!("{original:#?}");
    assert!(
        pretty.starts_with("DecoratedCospan {\n"),
        "the alternate spec opens the struct on its own line, got: {pretty}"
    );
    assert!(
        pretty.contains("\n    cospan: "),
        "the alternate spec indents `cospan` by four spaces, got: {pretty}"
    );
    assert!(
        pretty.contains("\n    decoration: PetriApex"),
        "the alternate spec indents `decoration` by four spaces and delegates \
         to the apex value's own Debug, got: {pretty}"
    );
    assert!(
        pretty.ends_with("\n}"),
        "the alternate spec closes the struct on its own line, got: {pretty}"
    );
}

#[test]
fn t2_7_clone_and_debug_over_marker_with_no_derives() {
    // `LocalTrivial` derives nothing at all.
    let cospan = Cospan::<usize>::new(vec![0], vec![0], vec![7]).unwrap();
    let original = DecoratedCospan::<usize, LocalTrivial>::new(cospan, ());
    let cloned = original.clone();

    assert_eq!(
        cloned, original,
        "the clone must compare equal to the original through the shipped \
         PartialEq; original = {original:?}, clone = {cloned:?}"
    );
    assert_eq!(
        cloned.cospan.middle(),
        &[7],
        "the clone carries the same apex: expected [7], got {:?}",
        cloned.cospan.middle()
    );

    let rendered = format!("{original:?}");
    assert!(
        rendered.starts_with("DecoratedCospan {"),
        "Debug names the struct, got: {rendered}"
    );
    assert!(
        rendered.contains("cospan:"),
        "Debug names the `cospan` field, got: {rendered}"
    );
    assert!(
        rendered.contains("decoration: ()"),
        "Debug renders the unit apex under the `decoration` field, got: {rendered}"
    );

    let pretty = format!("{original:#?}");
    assert!(
        pretty.starts_with("DecoratedCospan {\n"),
        "the alternate spec opens the struct on its own line, got: {pretty}"
    );
    assert!(
        pretty.contains("\n    cospan: "),
        "the alternate spec indents `cospan` by four spaces, got: {pretty}"
    );
    assert!(
        pretty.contains("\n    decoration: ()"),
        "the alternate spec indents `decoration` by four spaces, got: {pretty}"
    );
    assert!(
        pretty.ends_with("\n}"),
        "the alternate spec closes the struct on its own line, got: {pretty}"
    );

    let padded = format!("{original:>90?}");
    let (shadow_rendered, shadow_pretty, shadow_padded) = {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct DecoratedCospan {
            cospan: Cospan<usize>,
            decoration: (),
        }
        let shadow = DecoratedCospan {
            cospan: original.cospan.clone(),
            decoration: original.decoration,
        };
        (
            format!("{shadow:?}"),
            format!("{shadow:#?}"),
            format!("{shadow:>90?}"),
        )
    };

    assert_eq!(
        rendered, shadow_rendered,
        "under `{{:?}}` the shipped rendering must be byte-identical to a \
         derived-`Debug` struct of the same name and fields: shipped = \
         {rendered:?}, derived = {shadow_rendered:?}"
    );
    assert_eq!(
        pretty, shadow_pretty,
        "under `{{:#?}}` the shipped rendering must be byte-identical to a \
         derived-`Debug` struct of the same name and fields: shipped = \
         {pretty:?}, derived = {shadow_pretty:?}"
    );
    assert_eq!(
        padded, shadow_padded,
        "under `{{:>90?}}` the shipped rendering must be byte-identical to a \
         derived-`Debug` struct of the same name and fields: shipped = \
         {padded:?}, derived = {shadow_padded:?}"
    );
}
