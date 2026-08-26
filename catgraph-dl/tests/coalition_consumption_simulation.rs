//! A downstream-shaped `impl Actegory<SetMonoidal>` (`MockQuantale`) whose
//! `act` agrees pointwise with `SetActegory::act`, and an end-to-end
//! `tie_weights::<SetActegory, …>` call.

use catgraph_dl::para::{
    Actegory, MonoidalCategory, ParaMorphism, SetActegory, SetMonoidal, tie_weights,
};

/// Stand-in for `catgraph_coalition::Quantale`'s eventual actegory body.
///
/// Action is `(P, X) ↦ (P, X)` — Cartesian product, same shape as
/// [`SetActegory`]. In the actual coalition caller the action would carry
/// Tropical-flavoured min-weight semantics, BTV21 free-monoid concatenation,
/// or similar non-trivial structure; cg-dl is structure-agnostic so the
/// simulation uses trivial Cartesian.
///
/// Defined here as a local ZST to keep cg-dl dep-free of coalition.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct MockQuantale;

impl Actegory<SetMonoidal> for MockQuantale {
    type Object = catgraph_dl::para::SetObject;
    type Morphism = catgraph_dl::para::SetMorphism;
    type ActionResult<P, X> = (P, X);

    fn act<P, X>(&self, parameter: P, x: X) -> Self::ActionResult<P, X> {
        (parameter, x)
    }

    fn compose_action<Q, P, X>(
        &self,
        q: Q,
        p: P,
        x: X,
    ) -> Self::ActionResult<<SetMonoidal as MonoidalCategory>::Tensor<Q, P>, X> {
        ((q, p), x)
    }
}

/// Simulates the coalition caller end-to-end.
///
/// Asserts:
/// 1. The `MockQuantale` actegory implementation is well-formed — `act`
///    gives `(p, x)` and `compose_action` gives `((q, p), x)`, matching
///    the documented Cartesian shape.
/// 2. `tie_weights::<i64, _, i64, i64>(3, untied)` produces a
///    `ParaMorphism` whose action collapses the paired parameter slot — for
///    `f(((p1, p2), x)) = p1 + p2 + x` and tied value `3`, the resulting
///    action evaluated at `(3, 5)` returns `3 + 3 + 5 = 11`.
/// 3. `MockQuantale::act` and `SetActegory::act` agree pointwise on the
///    Cartesian-action shape, demonstrating the pathway is structure-
///    agnostic — the coalition caller's actegory choice does not change the
///    `tie_weights` arithmetic.
#[test]
fn tie_weights_consumption_pathway_simulation() {
    let mock = MockQuantale;
    let set_acteg = SetActegory::new();

    // (1) Sanity-check the MockQuantale actegory body matches the
    //     documented Cartesian shape.
    assert_eq!(mock.act(7_i64, 5_i64), (7, 5));
    assert_eq!(mock.compose_action(2_i64, 3_i64, 5_i64), ((2, 3), 5));

    // (2) Exercise tie_weights end-to-end. We use SetActegory here as a
    //     conservative caller choice; the actegory choice is
    //     orthogonal to the diagonal collapse. For the coalition caller this
    //     line becomes ParaMorphism<SetMonoidal, QuantaleActegory, …>.
    let untied: ParaMorphism<SetMonoidal, SetActegory, (i64, i64), _> =
        ParaMorphism::new((0_i64, 0_i64), |((p1, p2), x): ((i64, i64), i64)| {
            p1 + p2 + x
        });

    let tied = tie_weights::<SetActegory, i64, _, i64, i64>(3_i64, untied);

    assert_eq!(tied.parameter, 3_i64);
    assert_eq!((tied.action)((3_i64, 5_i64)), 11_i64);

    // Sweep — the diagonal collapse is pointwise on every (p, x).
    for (p, x) in [(0_i64, 0_i64), (1, 2), (-3, 5), (10, -7), (100, 0)] {
        let z: i64 = (tied.action)((p, x));
        assert_eq!(
            z,
            p + p + x,
            "diagonal collapse failed at (p, x) = ({p}, {x})"
        );
    }

    // (3) Cross-validate: MockQuantale::act and SetActegory::act agree
    //     pointwise. Confirms the Cartesian-action shape is uniform across
    //     any (Set, ×, 1)-flavoured actegory and the coalition caller's choice
    //     of QuantaleActegory does not perturb the tie_weights output.
    for (p, x) in [(0_i64, 0_i64), (1, 2), (-3, 5)] {
        assert_eq!(
            mock.act(p, x),
            set_acteg.act(p, x),
            "MockQuantale and SetActegory diverged at (p, x) = ({p}, {x})"
        );
    }
}
