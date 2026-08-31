//! `Free::fold` law tests (CDL Example 2.12 lists, Example 2.14 trees, over
//! the Prop B.18 carriers with an inhabited `Pure` payload): the `Pure` arm,
//! left-to-right position order, and the `Assemble` recompose path.
//!
//! Both witnesses carry an **inhabited** payload type, so `Pure` leaves are
//! reachable. The tree witness folds with a **non-commutative** algebra; the
//! list witness has arity ≤ 1, so child order is not observable there, and its
//! tests exercise the `Pure` terminator. Every expected value below is a
//! closed-form string.

use catgraph_dl::Either;
use catgraph_dl::free_monad::Free;
use catgraph_dl::free_monad::list_endo::{ListEndo, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::TreeEndo;

/// `A + (−)²` over `char` leaves with a `char` payload: `Pure` leaves and
/// `Suspend(Left(_))` leaves both inhabit this carrier.
type Tree = Free<TreeEndo<char>, char>;

/// A `Suspend(Left(a))` leaf — the `A` summand, arity 0.
fn leaf(a: char) -> Tree {
    Free::suspend(Either::Left(a))
}

/// A `Suspend(Right((l, r)))` node — the `(−)²` summand, arity 2.
fn node(l: Tree, r: Tree) -> Tree {
    Free::suspend(Either::Right((Box::new(l), Box::new(r))))
}

/// The `pure_case` of [`render`]: a `Pure` payload renders bracketed, so it is
/// distinguishable from a `Left(a)` leaf carrying the same character.
fn tree_pure(z: char) -> String {
    format!("[{z}]")
}

/// The algebra of [`render`]: `Left(a)` to `a`, `Right((l, r))` to
/// `(<l><r>)`. Concatenation is non-commutative, so the rendering of a node
/// determines the order its children were folded in.
fn tree_algebra(cell: Either<char, (String, String)>) -> String {
    match cell {
        Either::Left(a) => a.to_string(),
        Either::Right((l, r)) => format!("({l}{r})"),
    }
}

/// Fold a [`Tree`] to its parenthesized rendering. The result determines both
/// which arm ran at each cell and in which order the children were folded.
fn render(tree: Tree) -> String {
    tree.fold(&tree_pure, &tree_algebra)
}

/// `1 + A × −` over `char` labels with a `&str` terminator.
type List = Free<ListEndo<char>, &'static str>;

/// The `pure_case` of [`render_list`]: the terminator renders in angle
/// brackets, so its payload is visible in the folded result.
fn list_pure(z: &'static str) -> String {
    format!("<{z}>")
}

/// The algebra of [`render_list`]: a cons cell prepends `a:`, and the `Nil`
/// shape (`None`, arity 0) renders as `!`.
fn list_algebra(cell: Option<(char, String)>) -> String {
    match cell {
        None => "!".to_string(),
        Some((a, rest)) => format!("{a}:{rest}"),
    }
}

/// Fold a [`List`] to `a:b:…:<terminator>`.
fn render_list(list: List) -> String {
    list.fold(&list_pure, &list_algebra)
}

/// The catamorphism over `A + (−)²` (CDL Example 2.14, on the Prop B.18
/// carrier), on shapes that separate the `Pure` arm, position order, and the
/// `Assemble` recompose path.
///
/// - **`Pure` arm** — the bare `Pure` root, and the `Pure` leaves inside the
///   compound shapes, which render `[p]` / `[z]` where no other arm can.
/// - **Position order** — the mirrored pair `([p]a)` / `(a[p])`, and the
///   asymmetric tree: swapping the two children of every node renders
///   `((43)([p]1))`, not `((1[p])(34))`.
/// - **`Assemble` recompose** — the right-nested spine and the asymmetric
///   tree both reach an `Assemble` while an outer sibling's result is already
///   on the result stack, so taking the node's contents from the front of that
///   stack rather than the back renders `([z](12))` and `(4((1[p])3))`.
#[test]
fn fold_pins_the_pure_arm_child_order_and_recompose() {
    // Degenerate roots: one arm each, no `Assemble` at all.
    assert_eq!(render(Free::pure('p')), "[p]", "bare `Pure` root");
    assert_eq!(render(leaf('a')), "a", "bare `Left` leaf, arity 0");

    // The mirrored pair: same multiset of children, different renderings.
    assert_eq!(
        render(node(Free::pure('p'), leaf('a'))),
        "([p]a)",
        "`Pure` in position 0"
    );
    assert_eq!(
        render(node(leaf('a'), Free::pure('p'))),
        "(a[p])",
        "`Pure` in position 1 — a swapped child order renders `([p]a)`"
    );

    // Asymmetric, with a `Pure` leaf under the left subtree. Reversed children
    // render `((43)([p]1))`; contents taken from the front of the result stack
    // render `(4((1[p])3))`.
    let asymmetric = node(node(leaf('1'), Free::pure('p')), node(leaf('3'), leaf('4')));
    assert_eq!(render(asymmetric), "((1[p])(34))");

    // Right-nested: the inner `Assemble` runs with the outer left sibling's
    // result already on the stack. Front-taken contents render `([z](12))`.
    let right_spine = node(leaf('1'), node(leaf('2'), Free::pure('z')));
    assert_eq!(render(right_spine), "(1(2[z]))");
}

/// The same catamorphism over `1 + A × −` (CDL Example 2.12, on the Prop B.18
/// carrier), with reachable `Pure` terminators.
///
/// The two terminators give `x:y:z:<END>` and `x:y:z:<TAIL>` over identical
/// labels. The `Nil` shape is folded too: arity 0 through `recompose`,
/// rendering `a:!`.
#[test]
fn list_endo_fold_threads_the_pure_terminator() {
    let items = vec!['x', 'y', 'z'];

    assert_eq!(
        render_list(vec_to_free_mnd(items.clone(), "END")),
        "x:y:z:<END>"
    );
    assert_eq!(
        render_list(vec_to_free_mnd(items, "TAIL")),
        "x:y:z:<TAIL>",
        "the terminator payload is the tail of the rendering"
    );

    // Empty list: `vec_to_free_mnd` collapses to `Pure`, so the whole rendering
    // is the terminator.
    assert_eq!(render_list(vec_to_free_mnd(Vec::new(), "END")), "<END>");

    // A cons cell whose tail is the bare `Nil` shape — the arity-0 `None` of
    // `1 + A × −`, which `free_mnd_to_vec` panics on but `fold` interprets.
    let nil_terminated: List = Free::suspend(Some(('a', Box::new(Free::suspend(None)))));
    assert_eq!(render_list(nil_terminated), "a:!");
}
