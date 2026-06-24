//! Integration tests for the v0.5.2 Functorial decision engine
//! ([`Presentation::eq_mod_functorial`] + [`CompleteFunctor`] +
//! [`MatrixNFFunctor`]).
//!
//! The Functorial engine is complete by theorem on the `Free(Σ_SFG)/⟨E_{17}⟩
//! ≅ Mat(R)` presentation (F&S Thm 5.60 / Baez-Erbele 2015): two signal-flow
//! graphs are equivalent under the 17 Thm 5.60 equations iff their matrix
//! images are equal. These tests exercise the API surface; the underlying
//! functor `sfg_to_mat` has its own coverage in `tests/graphical_linalg.rs`.

use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::presentation::functorial::{CompleteFunctor, MatrixNFFunctor};
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;

/// `Identity(n)` over `SfgGenerator<R>`.
fn identity_sfg<R>(n: usize) -> PropExpr<SfgGenerator<R>>
where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    Free::<SfgGenerator<R>>::identity(n)
}

/// `Scalar(r)` over `SfgGenerator<R>`.
fn scalar_sfg<R>(r: R) -> PropExpr<SfgGenerator<R>>
where
    R: catgraph_applied::rig::Rig + std::fmt::Debug + Eq + std::hash::Hash + 'static,
{
    Free::<SfgGenerator<R>>::generator(SfgGenerator::Scalar(r))
}

// ----- Smoke tests (plan task #8 acceptance) -----

#[test]
fn same_expression_equals_itself() {
    // Reflexivity: any expression is functorially equal to itself.
    let pres = Presentation::<SfgGenerator<BoolRig>>::new();
    let f = MatrixNFFunctor::<BoolRig>::new();
    let e = identity_sfg::<BoolRig>(1);
    assert_eq!(pres.eq_mod_functorial(&e, &e, &f).unwrap(), Some(true));
}

#[test]
fn identity_and_scalar_one_have_same_matrix_image() {
    // `Scalar(1) : 1 → 1` maps to `[[1]]` under `S` (Eq 5.52).
    // `Identity(1)` also maps to `[[1]]` (the 1×1 identity matrix).
    // Their matrix images are bit-identical, so the functorial engine
    // decides equality as `Some(true)` without consulting the E_{17}
    // presentation.
    let pres = Presentation::<SfgGenerator<BoolRig>>::new();
    let f = MatrixNFFunctor::<BoolRig>::new();
    let id = identity_sfg::<BoolRig>(1);
    let s_one = scalar_sfg::<BoolRig>(BoolRig(true)); // BoolRig::one()
    assert_eq!(pres.eq_mod_functorial(&id, &s_one, &f).unwrap(), Some(true));
}

#[test]
fn different_arities_have_distinct_matrices() {
    // `Identity(1)` is 1×1; `Identity(2)` is 2×2. Different shapes ⇒
    // `MatR::eq` returns `false`.
    let pres = Presentation::<SfgGenerator<BoolRig>>::new();
    let f = MatrixNFFunctor::<BoolRig>::new();
    let id_1 = identity_sfg::<BoolRig>(1);
    let id_2 = identity_sfg::<BoolRig>(2);
    assert_eq!(
        pres.eq_mod_functorial(&id_1, &id_2, &f).unwrap(),
        Some(false),
    );
}

#[test]
fn distinct_scalars_have_distinct_matrices() {
    // `Scalar(true)` → `[[1]]`, `Scalar(false)` → `[[0]]`. In BoolRig the
    // two matrices are unequal, so the functorial engine rejects.
    let pres = Presentation::<SfgGenerator<BoolRig>>::new();
    let f = MatrixNFFunctor::<BoolRig>::new();
    let s_true = scalar_sfg::<BoolRig>(BoolRig(true));
    let s_false = scalar_sfg::<BoolRig>(BoolRig(false));
    assert_eq!(
        pres.eq_mod_functorial(&s_true, &s_false, &f).unwrap(),
        Some(false),
    );
}

// ----- Independence from the user equation set -----

#[test]
fn functorial_engine_ignores_presentation_equations() {
    // `eq_mod_functorial` is pure semantics — the user `equations` list
    // is irrelevant. Even if a presentation *declared* two unrelated
    // expressions equal syntactically, the functor's verdict depends
    // only on `f(a) == f(b)`.
    let mut pres = Presentation::<SfgGenerator<BoolRig>>::new();
    // Seed a bogus equation `Identity(1) = Identity(2)` (arity would
    // normally block this via `add_equation` — but the mismatch check is
    // on source+target not dimension equality; skip the test if arity
    // check fires). Use matching-arity expressions instead: seed
    // `Identity(1) = Scalar(false)`. The functor should still report
    // the two as *distinct* (matrix [[1]] ≠ [[0]]).
    let lhs = identity_sfg::<BoolRig>(1);
    let rhs = scalar_sfg::<BoolRig>(BoolRig(false));
    pres.add_equation(lhs.clone(), rhs.clone())
        .expect("same arity 1→1");
    let f = MatrixNFFunctor::<BoolRig>::new();
    assert_eq!(
        pres.eq_mod_functorial(&lhs, &rhs, &f).unwrap(),
        Some(false),
        "functorial engine decides by matrix equality, not by `pres.equations`"
    );
}

// ----- Trait object-ability (sanity check on API shape) -----

#[test]
fn functor_usable_as_concrete_generic_param() {
    // Compile-time check that the method's `F: CompleteFunctor<G>` bound
    // accepts concrete functor structs (no trait-object indirection
    // needed). Regression guard for future refactors.
    fn eq_via<F: CompleteFunctor<SfgGenerator<BoolRig>>>(
        pres: &Presentation<SfgGenerator<BoolRig>>,
        a: &PropExpr<SfgGenerator<BoolRig>>,
        b: &PropExpr<SfgGenerator<BoolRig>>,
        f: &F,
    ) -> Option<bool> {
        pres.eq_mod_functorial(a, b, f).unwrap()
    }
    let pres = Presentation::<SfgGenerator<BoolRig>>::new();
    let f = MatrixNFFunctor::<BoolRig>::new();
    let id = identity_sfg::<BoolRig>(1);
    assert_eq!(eq_via(&pres, &id, &id, &f), Some(true));
}
