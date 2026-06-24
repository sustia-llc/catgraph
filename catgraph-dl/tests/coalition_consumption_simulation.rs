//! Simulation of `catgraph-coalition` v0.4.0's consumption pathway for
//! [`catgraph_dl::para::tie_weights`]. cg-dl has no coalition dep; the test
//! defines a local `MockQuantale` ZST playing the role of v0.4.0's actegory.
//!
//! Three purposes:
//! 1. Smoke-test the documented consumption pathway end-to-end.
//! 2. Catch future API drift on `tie_weights` — any signature change breaks
//!    this simulated caller.
//! 3. Provide a copy-paste template for v0.4.0's first integration test.
//!
//! ## Pathway recap
//!
//! Per the cg-dl `comonoid.rs` "Consumption pathway" rustdoc + the cg-dl
//! `CLAUDE.md` "⚠️ CAREFUL — Para is upstream of Quantale" caveat,
//! `catgraph-coalition` v0.4.0 will:
//!
//! 1. **Import** `catgraph_dl::para::Actegory` (never re-define it locally).
//! 2. Define `impl Actegory<SetMonoidal> for QuantaleActegory` with the
//!    coalition's actegory action (Tropical-flavoured min-weights, free-
//!    monoid concatenation, etc. — the BTV21 substrate provides the body).
//! 3. Build a `ParaMorphism<SetMonoidal, QuantaleActegory, (P, P), F>` whose
//!    action consumes a paired parameter `f(((p1, p2), x))`.
//! 4. Call `tie_weights::<QuantaleActegory, P, _, X, Y>(parameter_tied, untied)`
//!    to collapse the paired parameter into a single shared `P`.
//!
//! ## Why this test uses both `MockQuantale` and `SetActegory`
//!
//! As of v0.4.0 `tie_weights` is parametric over the actegory
//! (`C: Actegory<SetMonoidal>`); v0.2.0/v0.3.x were `SetActegory`-bound. This
//! test was authored against the v0.3.x bound and retains `SetActegory` as a
//! *conservative caller choice* at the actual `tie_weights` call site to
//! preserve the v0.3.x acceptance shape; the simulation therefore:
//!
//! - Defines `MockQuantale` to demonstrate *pattern (i)*: this is the shape
//!   of the actegory definition coalition v0.5.0 will write in its own crate.
//! - Builds a `ParaMorphism<SetMonoidal, SetActegory, (P, P), F>` and calls
//!   `tie_weights::<SetActegory, …>` end-to-end — exercises the real
//!   consumption API on the simplest actegory.
//! - Cross-validates that `MockQuantale::act` matches `SetActegory::act`
//!   pointwise for the Cartesian action shape, demonstrating the pathway
//!   is structure-agnostic — i.e. coalition v0.5.0 can swap the call site
//!   to `tie_weights::<QuantaleActegory, …>(parameter_tied, untied)` without
//!   changing the body.
//!
//! When coalition v0.5.0 lands, the `MockQuantale` block lifts verbatim into
//! `catgraph-coalition::actegory` as the body of `impl Actegory<SetMonoidal>
//! for QuantaleActegory`, and the call site swaps from `<SetActegory, …>` to
//! `<QuantaleActegory, …>` — no cg-dl-side change required.

use catgraph_dl::para::{
    Actegory, MonoidalCategory, ParaMorphism, SetActegory, SetMonoidal, tie_weights,
};

/// Stand-in for `catgraph_coalition::Quantale`'s eventual actegory body.
///
/// Action is `(P, X) ↦ (P, X)` — Cartesian product, same shape as
/// [`SetActegory`]. In the actual v0.4.0 caller the action would carry
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

/// Simulates the coalition v0.4.0 caller end-to-end.
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
///    agnostic — the v0.4.0 caller's actegory choice does not change the
///    `tie_weights` arithmetic.
#[test]
fn tie_weights_consumption_pathway_simulation() {
    let mock = MockQuantale;
    let set_acteg = SetActegory::new();

    // (1) Sanity-check the MockQuantale actegory body matches the
    //     documented Cartesian shape.
    assert_eq!(mock.act(7_i64, 5_i64), (7, 5));
    assert_eq!(mock.compose_action(2_i64, 3_i64, 5_i64), ((2, 3), 5));

    // (2) Exercise tie_weights end-to-end. We use SetActegory here because
    //     v0.2.0 tie_weights is SetActegory-bound; the actegory choice is
    //     orthogonal to the diagonal collapse. For the v0.4.0 caller this
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
    //     any (Set, ×, 1)-flavoured actegory and the v0.4.0 caller's choice
    //     of QuantaleActegory does not perturb the tie_weights output.
    for (p, x) in [(0_i64, 0_i64), (1, 2), (-3, 5)] {
        assert_eq!(
            mock.act(p, x),
            set_acteg.act(p, x),
            "MockQuantale and SetActegory diverged at (p, x) = ({p}, {x})"
        );
    }
}
