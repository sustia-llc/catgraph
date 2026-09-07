//! Integration tests for `presentation::kb::CongruenceClosure` over a
//! single-sorted `1 → 1` signature.

use catgraph_applied::prop::presentation::kb::CongruenceClosure;
use catgraph_applied::prop::{Free, PropExpr, PropSignature, mono_word};
use std::borrow::Cow;

// ---- Tiny signature for testing: every generator is 1 → 1 ----

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
enum TestGen {
    A,
    B,
    C,
    D,
    E,
    F,
    X,
    Y,
    Z,
    A2,
    E2,
    X2,
    Y2,
}

impl PropSignature for TestGen {
    type Color = ();

    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
    fn source(&self) -> usize {
        1
    }
    fn target(&self) -> usize {
        1
    }
}

fn g(x: TestGen) -> PropExpr<TestGen> {
    Free::<TestGen>::generator(x)
}

fn c(f: PropExpr<TestGen>, s: PropExpr<TestGen>) -> PropExpr<TestGen> {
    Free::<TestGen>::compose(f, s).expect("every generator here is 1 → 1")
}

fn id1() -> PropExpr<TestGen> {
    Free::<TestGen>::identity(1)
}

/// Seed a fresh engine with `equations` and assert its verdict on `a` vs `b`.
fn assert_are_equal(
    equations: &[(PropExpr<TestGen>, PropExpr<TestGen>)],
    a: &PropExpr<TestGen>,
    b: &PropExpr<TestGen>,
    expected: bool,
    label: &str,
) {
    let mut engine = CongruenceClosure::new(equations);
    let observed = engine.are_equal(a, b);
    assert_eq!(
        observed, expected,
        "{label}: are_equal observed {observed}, expected {expected}"
    );
}

/// Seeded `A = Identity(1)` with `A` inserted first: the class canonical used
/// for substitution is the `Identity`, not the first-inserted member, so both
/// `A ; B` and `B ; A` reduce to `B`.
#[test]
fn atom_canonical_prefers_the_lowest_kind_not_the_first_inserted() {
    let equations = [(g(TestGen::A), id1())];
    assert_are_equal(
        &equations,
        &c(g(TestGen::A), g(TestGen::B)),
        &g(TestGen::B),
        true,
        "A ; B vs B under A = Identity(1)",
    );
    assert_are_equal(
        &equations,
        &c(g(TestGen::B), g(TestGen::A)),
        &g(TestGen::B),
        true,
        "B ; A vs B under A = Identity(1)",
    );
}

/// Merging the atom-free class `{A ; B, D ; E}` re-probes its parent
/// `(A ; B) ; C` against the signature `(D ; E) ; C` installed, so the two
/// parents' names `X` and `Y` are equal.
#[test]
fn congruence_propagates_through_a_merged_atom_free_child_class() {
    let equations = [
        (
            g(TestGen::X),
            c(c(g(TestGen::A), g(TestGen::B)), g(TestGen::C)),
        ),
        (
            g(TestGen::Y),
            c(c(g(TestGen::D), g(TestGen::E)), g(TestGen::C)),
        ),
        (
            c(g(TestGen::A), g(TestGen::B)),
            c(g(TestGen::D), g(TestGen::E)),
        ),
    ];
    assert_are_equal(
        &equations,
        &g(TestGen::X),
        &g(TestGen::Y),
        true,
        "X vs Y under (A;B);C = X, (D;E);C = Y, A;B = D;E",
    );
}

/// The same congruence with the merged atom-free class in the right-child
/// position: `C ; (A ; B)` is filed under the class of `A ; B`, so merging that
/// class re-probes it and `X` equals `Y`.
#[test]
fn a_function_node_is_registered_under_its_right_child_class() {
    let equations = [
        (
            g(TestGen::X),
            c(g(TestGen::C), c(g(TestGen::A), g(TestGen::B))),
        ),
        (
            g(TestGen::Y),
            c(g(TestGen::C), c(g(TestGen::D), g(TestGen::E))),
        ),
        (
            c(g(TestGen::A), g(TestGen::B)),
            c(g(TestGen::D), g(TestGen::E)),
        ),
    ];
    assert_are_equal(
        &equations,
        &g(TestGen::X),
        &g(TestGen::Y),
        true,
        "X vs Y under C;(A;B) = X, C;(D;E) = Y, A;B = D;E",
    );
}

/// `A ; (C ; D)` is re-probed once when `C ; D` merges into `Z ; (E ; F)`, and
/// again when refinement rewrites `Z ; (E ; F)` to `E ; F` under
/// `Z = Identity(1)`; the second re-probe reaches it only through the right
/// child's post-merge root, and matches `A ; (E ; F)`, so `X` equals `Y`.
#[test]
fn a_reprobed_node_is_refiled_under_its_second_child_root() {
    let equations = [
        (
            g(TestGen::X),
            c(g(TestGen::A), c(g(TestGen::C), g(TestGen::D))),
        ),
        (
            g(TestGen::Y),
            c(g(TestGen::A), c(g(TestGen::E), g(TestGen::F))),
        ),
        (g(TestGen::Z), id1()),
        (
            c(g(TestGen::C), g(TestGen::D)),
            c(g(TestGen::Z), c(g(TestGen::E), g(TestGen::F))),
        ),
    ];
    assert_are_equal(
        &equations,
        &g(TestGen::X),
        &g(TestGen::Y),
        true,
        "X vs Y under A;(C;D) = X, A;(E;F) = Y, Z = Identity(1), C;D = Z;(E;F)",
    );
}

/// A verdict reached only on the propagation that follows the second
/// refinement pass: refinement rewrites `Z ; (E ; F)` to `E ; F`, propagation
/// then merges `A ; (C ; D)` into the `Identity(1)` class, a second refinement
/// pass rewrites `(A ; (C ; D)) ; E2` to `E2`, and the propagation after it
/// merges `A2 ; ((A ; (C ; D)) ; E2)` with `A2 ; E2`, so `X2` equals `Y2`.
#[test]
fn propagate_fixpoint_repeats_until_refinement_reports_no_merge() {
    let equations = [
        (
            g(TestGen::X),
            c(g(TestGen::A), c(g(TestGen::C), g(TestGen::D))),
        ),
        (id1(), c(g(TestGen::A), c(g(TestGen::E), g(TestGen::F)))),
        (g(TestGen::Z), id1()),
        (
            c(g(TestGen::C), g(TestGen::D)),
            c(g(TestGen::Z), c(g(TestGen::E), g(TestGen::F))),
        ),
        (
            g(TestGen::X2),
            c(
                g(TestGen::A2),
                c(
                    c(g(TestGen::A), c(g(TestGen::C), g(TestGen::D))),
                    g(TestGen::E2),
                ),
            ),
        ),
        (g(TestGen::Y2), c(g(TestGen::A2), g(TestGen::E2))),
    ];
    assert_are_equal(
        &equations,
        &g(TestGen::X2),
        &g(TestGen::Y2),
        true,
        "X2 vs Y2 under the two-refinement-pass chain",
    );
}
