//! `NamedCospan` API demonstration.
//!
//! Shows construction with named ports, identity with name generation,
//! composition preserving port names, port manipulation (add/delete/connect),
//! name search by predicate, name mutation, and graph conversion.

use catgraph::category::Composable;
use catgraph::monoidal::Monoidal;
use catgraph::named_cospan::NamedCospan;
use either::Either::{Left, Right};

// ============================================================================
// Construction and Accessors
// ============================================================================

fn construction() {
    println!("=== Construction and Accessors ===\n");

    // NamedCospan<Lambda, LeftPortName, RightPortName>
    // Same as Cospan but with named boundary nodes.
    let nc: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],             // left leg (domain -> middle)
        vec![0, 2],             // right leg (codomain -> middle)
        vec!['a', 'b', 'c'],    // middle labels
        vec!["in_0", "in_1"],   // left port names
        vec!["out_0", "out_1"], // right port names
    );

    println!("left_names       = {:?}", nc.left_names());
    println!("right_names      = {:?}", nc.right_names());
    println!("cospan middle    = {:?}", nc.cospan().middle());
    println!("cospan left_leg  = {:?}", nc.cospan().left_to_middle());
    println!("cospan right_leg = {:?}", nc.cospan().right_to_middle());
    println!("domain           = {:?}", nc.domain());
    println!("codomain         = {:?}", nc.codomain());

    // Empty named cospan
    let empty: NamedCospan<char, &str, &str> = NamedCospan::empty();
    println!(
        "\nempty: left_names = {:?}, right_names = {:?}",
        empty.left_names(),
        empty.right_names()
    );
    println!();
}

// ============================================================================
// Identity with Name Generation
// ============================================================================

fn identity() {
    println!("=== Identity with Name Generation ===\n");

    // Identity needs a function that produces (LeftPortName, RightPortName)
    // from a prename. Here prenames are integers, names are strings.
    let types = vec!['a', 'b', 'c'];
    let prenames = vec![1, 2, 3];
    let id: NamedCospan<char, String, String> =
        NamedCospan::identity(&types, &prenames, |n| (format!("L{n}"), format!("R{n}")));

    println!("identity on ['a','b','c'] with prenames [1,2,3]:");
    println!("  left_names  = {:?}", id.left_names());
    println!("  right_names = {:?}", id.right_names());
    println!("  domain      = {:?}", id.domain());
    println!("  codomain    = {:?}", id.codomain());
    println!("  is_left_id  = {}", id.cospan().is_left_identity());
    println!("  is_right_id = {}", id.cospan().is_right_identity());
    println!();
}

// ============================================================================
// Composition with Named Boundaries
// ============================================================================

fn composition() {
    println!("=== Composition ===\n");

    // f: {a,b} -> {a,b} with left names ["x","y"], right names ["p","q"]
    let f: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],
        vec![0, 1],
        vec!['a', 'b'],
        vec!["x", "y"],
        vec!["p", "q"],
    );

    // g: {a,b} -> {a} with left names ["p","q"], right names ["out"]
    let g: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],
        vec![0],
        vec!['a', 'b'],
        vec!["p", "q"],
        vec!["out"],
    );

    println!(
        "f: left_names={:?}, right_names={:?}",
        f.left_names(),
        f.right_names()
    );
    println!(
        "g: left_names={:?}, right_names={:?}",
        g.left_names(),
        g.right_names()
    );

    let fg = f.compose(&g).unwrap();
    println!("\nf.compose(&g):");
    println!("  left_names  = {:?}  (from f)", fg.left_names());
    println!("  right_names = {:?}  (from g)", fg.right_names());
    println!("  domain      = {:?}", fg.domain());
    println!("  codomain    = {:?}", fg.codomain());
    println!();
}

// ============================================================================
// Port Manipulation: Add and Delete
// ============================================================================

fn port_manipulation() {
    println!("=== Port Manipulation ===\n");

    let mut nc: NamedCospan<char, &str, &str> =
        NamedCospan::new(vec![0], vec![0], vec!['a'], vec!["in_0"], vec!["out_0"]);
    println!(
        "initial: left={:?}, right={:?}, middle={:?}",
        nc.left_names(),
        nc.right_names(),
        nc.cospan().middle()
    );

    // Add left port pointing to existing middle node
    let idx = nc.add_boundary_node_known_target(0, Left("in_1"));
    println!("\nadd_boundary_node_known_target(0, Left(\"in_1\")) -> {idx:?}");
    println!("  left_names = {:?}", nc.left_names());

    // Add right port with new middle node
    let idx = nc.add_boundary_node_unknown_target('b', Right("out_1"));
    println!("\nadd_boundary_node_unknown_target('b', Right(\"out_1\")) -> {idx:?}");
    println!("  right_names = {:?}", nc.right_names());
    println!("  middle      = {:?}", nc.cospan().middle());

    // Delete a boundary node by index
    nc.delete_boundary_node(Left(0));
    println!("\ndelete_boundary_node(Left(0)):");
    println!("  left_names = {:?}", nc.left_names());

    // Delete by name
    nc.delete_boundary_node_by_name(Right("out_0"));
    println!("\ndelete_boundary_node_by_name(Right(\"out_0\")):");
    println!("  right_names = {:?}", nc.right_names());
    println!();
}

// ============================================================================
// Connect Pair and Map-to-Same
// ============================================================================

fn connect_and_query() {
    println!("=== Connect Pair and Map-to-Same ===\n");

    let mut nc: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],
        vec![2],
        vec!['a', 'a', 'a'],
        vec!["in_0", "in_1"],
        vec!["out_0"],
    );

    // Query if two named ports map to the same middle node
    let same = nc.map_to_same(Left("in_0"), Left("in_1"));
    println!("in_0 and in_1 map_to_same = {same}");

    // Connect them (merge middle nodes)
    nc.connect_pair(Left("in_0"), Left("in_1"));
    let same_after = nc.map_to_same(Left("in_0"), Left("in_1"));
    println!("after connect_pair: in_0 and in_1 map_to_same = {same_after}");
    println!("  middle = {:?}", nc.cospan().middle());
    println!();
}

// ============================================================================
// Name Search by Predicate
// ============================================================================

fn name_search() {
    println!("=== Name Search by Predicate ===\n");

    let nc: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1, 2],
        vec![0, 1],
        vec!['a', 'b', 'c'],
        vec!["alpha", "beta", "gamma"],
        vec!["delta", "epsilon"],
    );

    // Find all nodes whose names start with a given prefix
    let results = nc.find_nodes_by_name_predicate(
        |name| name.starts_with('a'), // left predicate
        |name| name.starts_with('e'), // right predicate
        false,                        // at_most_one = false
    );
    println!("names starting with 'a' (left) or 'e' (right):");
    for r in &results {
        match r {
            Left(i) => println!("  Left({i}) = {:?}", nc.left_names()[*i]),
            Right(i) => println!("  Right({i}) = {:?}", nc.right_names()[*i]),
        }
    }

    // Find at most one (short-circuits)
    let single = nc.find_nodes_by_name_predicate(|name| name == "beta", |_| false, true);
    println!("\nfind 'beta' (at_most_one=true): {single:?}");
    println!();
}

// ============================================================================
// Name Mutation
// ============================================================================

fn name_mutation() {
    println!("=== Name Mutation ===\n");

    let mut nc: NamedCospan<char, String, String> = NamedCospan::new(
        vec![0, 1],
        vec![0],
        vec!['a', 'b'],
        vec!["in_a".to_string(), "in_b".to_string()],
        vec!["out".to_string()],
    );
    println!("before: left_names = {:?}", nc.left_names());

    // Change a single port name
    nc.change_boundary_node_name(Left(("in_a".to_string(), "input_alpha".to_string())));
    println!(
        "after rename 'in_a' -> 'input_alpha': left_names = {:?}",
        nc.left_names()
    );

    // Change all names on one side with a function
    let uppercaser = |name: &mut String| {
        *name = name.to_uppercase();
    };
    nc.change_boundary_node_names::<fn(&mut String), _>(Right(uppercaser));
    println!(
        "after uppercase all right: right_names = {:?}",
        nc.right_names()
    );
    println!();
}

// ============================================================================
// Monoidal Product
// ============================================================================

fn monoidal_product() {
    println!("=== Monoidal Product ===\n");

    let mut a: NamedCospan<char, &str, &str> =
        NamedCospan::new(vec![0], vec![0], vec!['x'], vec!["a_in"], vec!["a_out"]);
    let b: NamedCospan<char, &str, &str> =
        NamedCospan::new(vec![0], vec![0], vec!['y'], vec!["b_in"], vec!["b_out"]);

    println!("a: left={:?}, right={:?}", a.left_names(), a.right_names());
    println!("b: left={:?}, right={:?}", b.left_names(), b.right_names());

    a.monoidal(b);
    println!("\nafter a.monoidal(b):");
    println!("  left_names  = {:?}", a.left_names());
    println!("  right_names = {:?}", a.right_names());
    println!("  domain      = {:?}", a.domain());
    println!("  codomain    = {:?}", a.codomain());
    println!();
}

// ============================================================================
// Map Lambda Labels
// ============================================================================

fn map_labels() {
    println!("=== Map Lambda Labels ===\n");

    let nc: NamedCospan<char, &str, &str> = NamedCospan::new(
        vec![0, 1],
        vec![0],
        vec!['a', 'b'],
        vec!["in_0", "in_1"],
        vec!["out_0"],
    );

    let mapped = nc.map(|ch| ch.to_ascii_uppercase());
    println!("original middle = {:?}", nc.cospan().middle());
    println!("mapped middle   = {:?}", mapped.cospan().middle());
    println!(
        "names preserved: left={:?}, right={:?}",
        mapped.left_names(),
        mapped.right_names()
    );
    println!();
}

fn main() {
    construction();
    identity();
    composition();
    port_manipulation();
    connect_and_query();
    name_search();
    name_mutation();
    monoidal_product();
    map_labels();
}
