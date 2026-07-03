//! Scaffold smoke test.
//!
//! Confirms the public API surface compiles and that the five
//! architecture wrappers, the F-(co)algebra newtypes, the free-monad
//! recursive carriers, and the `Para` type-level handle all instantiate.
//!
//! `FreeMnd<F, Z>` requires `F: EndoFunctor`
//! (GAT-based object-map encoding). The endofunctor placeholders below
//! gain trivial `EndoFunctor` impls so the type-level instantiations
//! still work — for `Stream`, `Mealy`, and `GroupAction` we use a unit
//! `Apply<X> = ()` projection because no semantics are exercised here;
//! semantics for `ListEndo` / `TreeEndo` are tested in the dedicated
//! `tests/free_monad_bijections.rs`.

#![allow(clippy::type_complexity, clippy::float_cmp)]

use catgraph_dl::algebra::{FAlgebra, FCoalgebra, MonadAlgebra};
use catgraph_dl::architectures::{FoldingRnn, MealyCell, MooreCell, RecursiveNn, UnfoldingRnn};
use catgraph_dl::free_monad::list_endo::ListEndo;
use catgraph_dl::free_monad::tree_endo::TreeEndo;
use catgraph_dl::free_monad::{CofreeCmnd, EndoFunctor, FreeMnd};

// Type-level placeholders for the F-algebra-side endofunctors. The
// `Apply<X> = ()` projections are deliberately trivial — these names are
// reserved for future semantic bodies and the smoke test only checks the
// type-level witnesses still construct.
struct StreamEndo<O>(core::marker::PhantomData<O>);
struct MealyEndo<I, O>(core::marker::PhantomData<(I, O)>);
struct GroupActionEndo<G>(core::marker::PhantomData<G>);

impl<O> EndoFunctor for StreamEndo<O> {
    type Apply<X> = ();
    fn fmap<X, Y, G>((): Self::Apply<X>, _: G) -> Self::Apply<Y>
    where
        G: Fn(X) -> Y,
    {
    }
}

impl<I, O> EndoFunctor for MealyEndo<I, O> {
    type Apply<X> = ();
    fn fmap<X, Y, G>((): Self::Apply<X>, _: G) -> Self::Apply<Y>
    where
        G: Fn(X) -> Y,
    {
    }
}

impl<G> EndoFunctor for GroupActionEndo<G> {
    type Apply<X> = ();
    fn fmap<X, Y, GFn>((): Self::Apply<X>, _: GFn) -> Self::Apply<Y>
    where
        GFn: Fn(X) -> Y,
    {
    }
}

#[test]
fn folding_rnn_constructs() {
    // Folding RNN over `1 + A × −`. Hidden state is u32, input alphabet is u8.
    let cell: FoldingRnn<f32, u32, fn(f32) -> u32, fn((f32, u8, u32)) -> u32, u8> =
        FoldingRnn::new(0.5_f32, |_p| 0_u32, |(_p, _a, s)| s);
    assert_eq!(cell.parameter, 0.5);
}

#[test]
fn unfolding_rnn_constructs() {
    let cell: UnfoldingRnn<f32, u32, fn((f32, u32)) -> u8, fn((f32, u32)) -> u32, u8> =
        UnfoldingRnn::new(0.0_f32, |(_p, _s)| 0_u8, |(_p, s)| s);
    assert_eq!(cell.parameter, 0.0);
}

#[test]
fn recursive_nn_constructs() {
    let cell: RecursiveNn<f32, u32, fn(f32) -> u32, fn((f32, u8, u32, u32)) -> u32, u8> =
        RecursiveNn::new(1.0_f32, |_p| 0_u32, |(_p, _a, _l, _r)| 0_u32);
    assert_eq!(cell.parameter, 1.0);
}

#[test]
fn mealy_cell_constructs() {
    let cell: MealyCell<f32, u32, fn((f32, u32)) -> fn(u8) -> (u8, u32), u8, u8> =
        MealyCell::new(0.25_f32, |(_p, _s)| {
            // Identity-ish next: ignored.
            |i| (i, 0_u32)
        });
    assert_eq!(cell.parameter, 0.25);
}

#[test]
fn moore_cell_constructs() {
    let cell: MooreCell<f32, u32, fn((f32, u32)) -> u8, fn((f32, u32, u8)) -> u32, u8, u8> =
        MooreCell::new(0.0_f32, |(_p, _s)| 0_u8, |(_p, s, _i)| s);
    assert_eq!(cell.parameter, 0.0);
}

#[test]
fn f_algebra_constructs() {
    let alg: FAlgebra<ListEndo<u8>, u32, fn(u32) -> u32> = FAlgebra::new(0_u32, |x| x);
    assert_eq!(alg.carrier, 0);
}

#[test]
fn f_coalgebra_constructs() {
    let coalg: FCoalgebra<StreamEndo<u8>, u32, fn(u32) -> (u8, u32)> =
        FCoalgebra::new(0_u32, |s| (0_u8, s));
    assert_eq!(coalg.carrier, 0);
}

#[test]
fn monad_algebra_constructs() {
    // Group-action monad algebra (CDL Example 2.4) — the categorical
    // recovery of GDL equivariance.
    let alg: FAlgebra<GroupActionEndo<u8>, u32, fn((u8, u32)) -> u32> =
        FAlgebra::new(0_u32, |(_g, x)| x);
    let monad_alg: MonadAlgebra<GroupActionEndo<u8>, u32, fn((u8, u32)) -> u32> =
        MonadAlgebra::new(alg);
    assert_eq!(monad_alg.algebra.carrier, 0);
}

#[test]
fn free_monad_witnesses_construct() {
    // FreeMnd(1 + A × −) ≅ List in CDL Example B.19. The `new()`
    // compatibility constructor produces `Pure(Z::default())`.
    let _free: FreeMnd<ListEndo<u8>, ()> = FreeMnd::new();
    let _free_tree: FreeMnd<TreeEndo<u8>, ()> = FreeMnd::new();

    // `CofreeCmnd::new` takes `(head, tail)`. For the smoke
    // test we use the trivial `Apply<X> = ()` projections of the stream/
    // Mealy endofunctors so `tail = ()` is a valid construction.
    let _cofree_stream: CofreeCmnd<StreamEndo<u8>, ()> = CofreeCmnd::new((), ());
    let _cofree_mealy: CofreeCmnd<MealyEndo<u8, u8>, ()> = CofreeCmnd::new((), ());
}
