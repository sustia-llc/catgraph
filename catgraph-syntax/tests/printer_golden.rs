//! Golden-output tests for the structural pretty-printer (Phase S1).
//!
//! Every term is built through [`Free`] smart constructors only; `compose` is
//! fallible (arity-checked) so its `Result` is unwrapped in-test. The goldens
//! pin the grammar/precedence rules documented on
//! [`catgraph_syntax::text::print`].

mod common;

use catgraph_applied::prop::{Free, PropExpr};
use catgraph_syntax::text::{Pretty, print};
use common::Sig;

fn g(s: Sig) -> PropExpr<Sig> {
    Free::generator(s)
}

#[test]
fn atoms_print_as_written() {
    assert_eq!(print(&Free::<Sig>::identity(2)), "id(2)");
    assert_eq!(print(&Free::<Sig>::braid(1, 2)), "braid(1,2)");
    assert_eq!(print(&g(Sig::Copy)), "copy");
    // The `Pretty` Display adapter agrees with the free `print` function.
    assert_eq!(Pretty(&g(Sig::Add)).to_string(), "add");
}

#[test]
fn empty_identity_prints() {
    assert_eq!(print(&Free::<Sig>::identity(0)), "id(0)");
}

#[test]
fn tensor_binds_tighter_than_compose() {
    // `a ; b * c` == Compose(copy, Tensor(add, unit)): copy.target = 2 =
    // add.source + unit.source = 2 + 0. No parens: tensor already binds tighter.
    let inner = Free::tensor(g(Sig::Add), g(Sig::Unit));
    let a_then_bc = Free::compose(g(Sig::Copy), inner).unwrap();
    assert_eq!(print(&a_then_bc), "copy ; add * unit");

    // `(a ; b) * c` == Tensor(Compose(copy, add), unit): the compose is a
    // looser-binding child of the tensor, so it MUST be parenthesized.
    let a_b = Free::compose(g(Sig::Copy), g(Sig::Add)).unwrap();
    let ab_tensor_c = Free::tensor(a_b, g(Sig::Unit));
    assert_eq!(print(&ab_tensor_c), "(copy ; add) * unit");

    // Same atoms, structurally distinct terms, distinct rendering.
    assert_ne!(print(&a_then_bc), print(&ab_tensor_c));

    // `c * (a ; b)` == Tensor(unit, Compose(copy, add)): a looser-binding
    // compose as the RIGHT operand of the tensor must also be parenthesized.
    // Pins the right-operand mixed-precedence branch (previously only the
    // left-operand variant was covered) — a regression that parenthesized
    // only left operands would print the ambiguous `unit * copy ; add`.
    let a_b2 = Free::compose(g(Sig::Copy), g(Sig::Add)).unwrap();
    let c_tensor_ab = Free::tensor(g(Sig::Unit), a_b2);
    assert_eq!(print(&c_tensor_ab), "unit * (copy ; add)");
}

#[test]
fn left_associated_chains_need_no_parens() {
    // ((copy ; add) ; copy) — left-nested composition prints flat.
    let copy_add = Free::compose(g(Sig::Copy), g(Sig::Add)).unwrap();
    let chain = Free::compose(copy_add, g(Sig::Copy)).unwrap();
    assert_eq!(print(&chain), "copy ; add ; copy");

    // ((copy * copy) * copy) — left-nested tensor prints flat.
    let cc = Free::tensor(g(Sig::Copy), g(Sig::Copy));
    let ccc = Free::tensor(cc, g(Sig::Copy));
    assert_eq!(print(&ccc), "copy * copy * copy");
}

#[test]
fn right_nested_same_operator_needs_parens() {
    // copy ; (add ; copy) — right-nested composition would reassociate under
    // left-associative printing, so the right operand is parenthesized.
    let add_copy = Free::compose(g(Sig::Add), g(Sig::Copy)).unwrap();
    let right_nested = Free::compose(g(Sig::Copy), add_copy).unwrap();
    assert_eq!(print(&right_nested), "copy ; (add ; copy)");

    // copy * (copy * copy) — likewise for the tensor.
    let cc = Free::tensor(g(Sig::Copy), g(Sig::Copy));
    let right_tensor = Free::tensor(g(Sig::Copy), cc);
    assert_eq!(print(&right_tensor), "copy * (copy * copy)");
}
