//! Typed-builder law tests (Phase S5).
//!
//! The heart is the **coherence family**: a set of hand-built [`Traced`]
//! pipelines over `SfgGenerator<i64>` / [`SfgModel`], one per combinator and a
//! mixed pipeline composing them all, each checked against the S5 milestone law
//!
//! ```text
//! eval(t.term(), &model, input.flatten()) == Ok(t.run(input).flatten())
//! ```
//!
//! over proptest-random input *values* (wire *shapes* are type-level, so they are
//! fixed per pipeline rather than sampled). Alongside it: [`Wires`]
//! flatten/unflatten round-trips (including the asymmetric `(Wire, (Wire, Wire))`
//! shape), the `traced_generator` arity-mismatch error paths on both the source
//! and target sides, a `Wires::unflatten` length-mismatch check, and a term-side
//! S2 print/reparse of a `Traced`-built term (tying the typed track back to the
//! textual surface).
//!
//! Scalars and input values are bounded modestly: the interpreter and the arrows
//! both do plain `i64` `+`/`*` (debug-panic / release-wrap on overflow — see the
//! `SfgModel` overflow note), so a wide range would eventually overflow along a
//! composition chain. Small bounds keep every product representable.

mod common;

use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::arrow_seam::{Arrow, Lift};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::eval;
use catgraph_syntax::text::{parse, print};
use catgraph_syntax::traced::{Traced, Wire, Wires, traced_braid_1_1, traced_generator, traced_id};
use common::sfg_model;
use proptest::prelude::*;

type Sfg = SfgGenerator<i64>;

/// The S5 milestone law for one `Traced` value on one typed input bundle: running
/// the arrow and flattening equals evaluating the paired term on the flattened
/// input. Every coherence test below funnels through this.
fn assert_coherent<A>(t: &Traced<A, Sfg>, input: A::In)
where
    A: Arrow,
    A::In: Wires<i64> + Clone,
    A::Out: Wires<i64>,
{
    let flat = input.clone().flatten();
    let ran = t.run(input).flatten();
    assert_eq!(eval(t.term(), &sfg_model(), flat), Ok(ran));
}

/// Assert a `Result` is the expected `SyntaxError::WireCount`, without requiring
/// the `Ok` payload to be `Debug` (a `Traced` is not) — the shared shape behind
/// the three arity-error checks below.
fn expect_wire_count<T>(result: Result<T, SyntaxError>, expected: usize, actual: usize) {
    match result {
        Err(SyntaxError::WireCount {
            expected: e,
            actual: a,
            ..
        }) => assert_eq!((e, a), (expected, actual)),
        Err(other) => panic!("expected WireCount ({expected}, {actual}), got {other:?}"),
        Ok(_) => panic!("expected WireCount ({expected}, {actual}), got an Ok value"),
    }
}

// ---- The SFG atom builders (one Traced per generator shape) -------------------

fn t_copy() -> Traced<impl Arrow<In = Wire<i64>, Out = (Wire<i64>, Wire<i64>)>, Sfg> {
    traced_generator(
        SfgGenerator::Copy,
        Lift::new(|Wire(x): Wire<i64>| (Wire(x), Wire(x))),
    )
    .expect("copy arrow is 1 -> 2")
}

fn t_discard() -> Traced<impl Arrow<In = Wire<i64>, Out = ()>, Sfg> {
    traced_generator(SfgGenerator::Discard, Lift::new(|Wire(_x): Wire<i64>| ()))
        .expect("discard arrow is 1 -> 0")
}

fn t_add() -> Traced<impl Arrow<In = (Wire<i64>, Wire<i64>), Out = Wire<i64>>, Sfg> {
    traced_generator(
        SfgGenerator::Add,
        Lift::new(|(Wire(a), Wire(b)): (Wire<i64>, Wire<i64>)| Wire(a + b)),
    )
    .expect("add arrow is 2 -> 1")
}

fn t_zero() -> Traced<impl Arrow<In = (), Out = Wire<i64>>, Sfg> {
    traced_generator(SfgGenerator::Zero, Lift::new(|(): ()| Wire(0i64)))
        .expect("zero arrow is 0 -> 1")
}

fn t_scalar(r: i64) -> Traced<impl Arrow<In = Wire<i64>, Out = Wire<i64>>, Sfg> {
    traced_generator(
        SfgGenerator::Scalar(r),
        Lift::new(move |Wire(x): Wire<i64>| Wire(x * r)),
    )
    .expect("scalar arrow is 1 -> 1")
}

// ---- Composite pipeline fixtures (built ONCE, reused across tests) -------------

/// `copy ; add : 1 → 1` — doubles its input.
fn double_traced() -> Traced<impl Arrow<In = Wire<i64>, Out = Wire<i64>>, Sfg> {
    t_copy().then(t_add())
}

/// `copy ; (scalar(2) *** scalar(3)) ; add : 1 → 1` — computes `x ↦ 5x`. Shared by
/// the coherence proptest and the term-round-trip test so they cannot drift.
fn five_x_traced() -> Traced<impl Arrow<In = Wire<i64>, Out = Wire<i64>>, Sfg> {
    t_copy().then(t_scalar(2).par(t_scalar(3))).then(t_add())
}

// ---- Coherence family: every combinator, proptest-random VALUES ---------------

proptest! {
    // Every pipeline here is affine (a fixed composite of `+`/`*`/copy/swap over
    // i64); coherence is a value-independent structural identity, so a handful of
    // sampled points per shape suffices — capping the cases keeps the suite quick
    // without weakening it.
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// `traced_generator` for each of the five SFG generator shapes.
    #[test]
    fn coherence_generators(x in -100i64..=100, y in -100i64..=100, r in -3i64..=3) {
        assert_coherent(&t_copy(), Wire(x));
        assert_coherent(&t_discard(), Wire(x));
        assert_coherent(&t_add(), (Wire(x), Wire(y)));
        assert_coherent(&t_zero(), ());
        assert_coherent(&t_scalar(r), Wire(x));
    }

    /// `traced_id` on several bundle shapes (0, 1, 2, and 3 wires).
    #[test]
    fn coherence_identity(a in -100i64..=100, b in -100i64..=100, c in -100i64..=100) {
        assert_coherent(&traced_id::<(), Sfg>(), ());
        assert_coherent(&traced_id::<Wire<i64>, Sfg>(), Wire(a));
        assert_coherent(
            &traced_id::<(Wire<i64>, Wire<i64>), Sfg>(),
            (Wire(a), Wire(b)),
        );
        assert_coherent(
            &traced_id::<(Wire<i64>, (Wire<i64>, Wire<i64>)), Sfg>(),
            (Wire(a), (Wire(b), Wire(c))),
        );
    }

    /// `traced_braid_1_1` — the single-wire swap matches eval's `Braid(1, 1)`
    /// block rotation `[a, b] ↦ [b, a]`. `assert_coherent` already checks the term
    /// side against eval; the extra pin fixes the concrete swap *direction*.
    #[test]
    fn coherence_braid(a in -100i64..=100, b in -100i64..=100) {
        let braid = traced_braid_1_1::<i64, Sfg>();
        assert_coherent(&braid, (Wire(a), Wire(b)));
        prop_assert_eq!(braid.run((Wire(a), Wire(b))).flatten(), vec![b, a]);
    }

    /// `then` — `copy ; add : 1 → 1` doubles its input.
    #[test]
    fn coherence_then_double(x in -100i64..=100) {
        let double = double_traced();
        assert_coherent(&double, Wire(x));
        prop_assert_eq!(double.run(Wire(x)).flatten(), vec![2 * x]);
    }

    /// `par` — `scalar(p) *** scalar(q) : (Wire, Wire) → (Wire, Wire)`.
    #[test]
    fn coherence_par(a in -100i64..=100, b in -100i64..=100, p in -3i64..=3, q in -3i64..=3) {
        let both = t_scalar(p).par(t_scalar(q));
        assert_coherent(&both, (Wire(a), Wire(b)));
        prop_assert_eq!(both.run((Wire(a), Wire(b))).flatten(), vec![a * p, b * q]);
    }

    /// A mixed pipeline composing every combinator: the shared `five_x_traced`
    /// (`copy ; (scalar(2) *** scalar(3)) ; add = 5x`) and a braid variant
    /// `(scalar(2) *** scalar(3)) ; braid ; add = 3b + 2a`.
    #[test]
    fn coherence_mixed_pipeline(x in -100i64..=100, a in -100i64..=100, b in -100i64..=100) {
        let five_x = five_x_traced();
        assert_coherent(&five_x, Wire(x));
        prop_assert_eq!(five_x.run(Wire(x)).flatten(), vec![5 * x]);

        let braided = t_scalar(2)
            .par(t_scalar(3))
            .then(traced_braid_1_1::<i64, Sfg>())
            .then(t_add());
        assert_coherent(&braided, (Wire(a), Wire(b)));
        prop_assert_eq!(braided.run((Wire(a), Wire(b))).flatten(), vec![3 * b + 2 * a]);
    }
}

// ---- Wires flatten/unflatten round-trips --------------------------------------

#[test]
fn wires_flatten_unflatten_round_trip() {
    // Flat two-wire bundle.
    let flat: (Wire<i64>, Wire<i64>) = (Wire(1), Wire(2));
    let values = flat.flatten();
    assert_eq!(values, vec![1, 2]);
    assert_eq!(
        <(Wire<i64>, Wire<i64>)>::unflatten(values),
        Ok((Wire(1), Wire(2)))
    );

    // The asymmetric right-nested shape flattens to the SAME canonical vector as
    // a left-nested one would, and reparses to its own shape.
    let asym: (Wire<i64>, (Wire<i64>, Wire<i64>)) = (Wire(3), (Wire(4), Wire(5)));
    assert_eq!(asym.flatten(), vec![3, 4, 5]);
    assert_eq!(
        <(Wire<i64>, (Wire<i64>, Wire<i64>))>::unflatten(vec![3, 4, 5]),
        Ok((Wire(3), (Wire(4), Wire(5))))
    );

    // The empty bundle round-trips through the empty vector.
    assert_eq!(<() as Wires<i64>>::flatten(()), Vec::<i64>::new());
    assert_eq!(<() as Wires<i64>>::unflatten(Vec::<i64>::new()), Ok(()));
}

#[test]
fn wires_unflatten_length_mismatch_is_wire_count() {
    // Too many values: the up-front length check rejects surplus (no silent
    // truncation).
    expect_wire_count(<(Wire<i64>, Wire<i64>)>::unflatten(vec![1, 2, 3]), 2, 3);
    // Too few.
    expect_wire_count(<(Wire<i64>, Wire<i64>)>::unflatten(vec![1]), 2, 1);
}

// ---- traced_generator arity-mismatch error paths ------------------------------

#[test]
fn traced_generator_source_arity_mismatch() {
    // Add declares source 2, but the arrow's input bundle is a single wire.
    let bad = traced_generator(
        SfgGenerator::<i64>::Add,
        Lift::new(|Wire(x): Wire<i64>| Wire(x)),
    );
    expect_wire_count(bad, 2, 1);
}

#[test]
fn traced_generator_target_arity_mismatch() {
    // Copy declares target 2, but the arrow's output bundle is a single wire.
    let bad = traced_generator(
        SfgGenerator::<i64>::Copy,
        Lift::new(|Wire(x): Wire<i64>| Wire(x)),
    );
    expect_wire_count(bad, 2, 1);
}

// ---- Term-side S2 round-trip of a Traced-built term ---------------------------

#[test]
fn traced_term_prints_and_reparses() {
    // The shared five_x pipeline's term is a left-nested compose chain (so it
    // prints flat, clear of the parser's nesting bound) and reparses identically —
    // the typed track hands a well-formed term straight back to the S2 surface.
    let term = five_x_traced().term().clone();
    let printed = print(&term);
    assert_eq!(parse::<Sfg>(&printed), Ok(term));
}

// ---- into_parts: the one-way door ---------------------------------------------

#[test]
fn into_parts_yields_arrow_and_matching_term() {
    let double = double_traced();
    let expected_term = double.term().clone();
    let (arrow, term) = double.into_parts();
    // The extracted term is the one the Traced denoted...
    assert_eq!(term, expected_term);
    // ...and the extracted arrow still runs (there is no way to reassemble a
    // Traced from these two halves — the sync invariant is construction-only).
    assert_eq!(arrow.run(Wire(21)).flatten(), vec![42]);
}
