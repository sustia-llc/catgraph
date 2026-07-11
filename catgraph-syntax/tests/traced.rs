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

use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::arrow_seam::{Arrow, Lift};
use catgraph_syntax::errors::SyntaxError;
use catgraph_syntax::eval::{SfgModel, eval};
use catgraph_syntax::text::{parse, print};
use catgraph_syntax::traced::{Traced, Wire, Wires, traced_braid_1_1, traced_generator, traced_id};
use proptest::prelude::*;

type Sfg = SfgGenerator<i64>;

fn model() -> SfgModel<i64> {
    SfgModel::<i64>::new()
}

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
    assert_eq!(eval(t.term(), &model(), flat), Ok(ran));
}

// ---- The SFG atom builders (one Traced per generator shape) -------------------

fn t_copy() -> Traced<impl Arrow<In = Wire<i64>, Out = (Wire<i64>, Wire<i64>)>, Sfg> {
    traced_generator::<i64, _, _>(
        SfgGenerator::Copy,
        Lift::new(|Wire(x): Wire<i64>| (Wire(x), Wire(x))),
    )
    .expect("copy arrow is 1 -> 2")
}

fn t_discard() -> Traced<impl Arrow<In = Wire<i64>, Out = ()>, Sfg> {
    traced_generator::<i64, _, _>(SfgGenerator::Discard, Lift::new(|Wire(_x): Wire<i64>| ()))
        .expect("discard arrow is 1 -> 0")
}

fn t_add() -> Traced<impl Arrow<In = (Wire<i64>, Wire<i64>), Out = Wire<i64>>, Sfg> {
    traced_generator::<i64, _, _>(
        SfgGenerator::Add,
        Lift::new(|(Wire(a), Wire(b)): (Wire<i64>, Wire<i64>)| Wire(a + b)),
    )
    .expect("add arrow is 2 -> 1")
}

fn t_zero() -> Traced<impl Arrow<In = (), Out = Wire<i64>>, Sfg> {
    traced_generator::<i64, _, _>(SfgGenerator::Zero, Lift::new(|(): ()| Wire(0i64)))
        .expect("zero arrow is 0 -> 1")
}

fn t_scalar(r: i64) -> Traced<impl Arrow<In = Wire<i64>, Out = Wire<i64>>, Sfg> {
    traced_generator::<i64, _, _>(
        SfgGenerator::Scalar(r),
        Lift::new(move |Wire(x): Wire<i64>| Wire(x * r)),
    )
    .expect("scalar arrow is 1 -> 1")
}

// ---- Coherence family: every combinator, proptest-random VALUES ---------------

proptest! {
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
        assert_coherent(&traced_id::<i64, (), Sfg>(), ());
        assert_coherent(&traced_id::<i64, Wire<i64>, Sfg>(), Wire(a));
        assert_coherent(
            &traced_id::<i64, (Wire<i64>, Wire<i64>), Sfg>(),
            (Wire(a), Wire(b)),
        );
        assert_coherent(
            &traced_id::<i64, (Wire<i64>, (Wire<i64>, Wire<i64>)), Sfg>(),
            (Wire(a), (Wire(b), Wire(c))),
        );
    }

    /// `traced_braid_1_1` — the single-wire swap matches eval's `Braid(1, 1)`
    /// block rotation `[a, b] ↦ [b, a]`.
    #[test]
    fn coherence_braid(a in -100i64..=100, b in -100i64..=100) {
        let braid = traced_braid_1_1::<i64, Sfg>();
        assert_coherent(&braid, (Wire(a), Wire(b)));
        // Pin the concrete swap direction against the interpreter.
        prop_assert_eq!(braid.run((Wire(a), Wire(b))).flatten(), vec![b, a]);
        prop_assert_eq!(eval(braid.term(), &model(), vec![a, b]), Ok(vec![b, a]));
    }

    /// `then` — `copy ; add : 1 → 1` doubles its input.
    #[test]
    fn coherence_then_double(x in -100i64..=100) {
        let double = t_copy().then(t_add());
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

    /// A mixed pipeline composing every combinator:
    /// `copy ; (scalar(2) *** scalar(3)) ; add : 1 → 1` computes `x ↦ 5x`, and a
    /// braid variant `(scalar(2) *** scalar(3)) ; braid ; add` computes
    /// `(a, b) ↦ 3b + 2a`.
    #[test]
    fn coherence_mixed_pipeline(x in -100i64..=100, a in -100i64..=100, b in -100i64..=100) {
        let five_x = t_copy().then(t_scalar(2).par(t_scalar(3))).then(t_add());
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
    match <(Wire<i64>, Wire<i64>)>::unflatten(vec![1]) {
        Err(SyntaxError::WireCount {
            expected, actual, ..
        }) => assert_eq!((expected, actual), (2, 1)),
        other => panic!("expected WireCount, got {other:?}"),
    }
}

// ---- traced_generator arity-mismatch error paths ------------------------------

#[test]
fn traced_generator_source_arity_mismatch() {
    // Add declares source 2, but the arrow's input bundle is a single wire.
    let bad = traced_generator::<i64, _, _>(
        SfgGenerator::<i64>::Add,
        Lift::new(|Wire(x): Wire<i64>| Wire(x)),
    );
    match bad {
        Err(SyntaxError::WireCount {
            expected, actual, ..
        }) => assert_eq!((expected, actual), (2, 1)),
        Err(other) => panic!("expected a WireCount on the source side, got {other:?}"),
        Ok(_) => panic!("expected a source-side arity error, got a Traced value"),
    }
}

#[test]
fn traced_generator_target_arity_mismatch() {
    // Copy declares target 2, but the arrow's output bundle is a single wire.
    let bad = traced_generator::<i64, _, _>(
        SfgGenerator::<i64>::Copy,
        Lift::new(|Wire(x): Wire<i64>| Wire(x)),
    );
    match bad {
        Err(SyntaxError::WireCount {
            expected, actual, ..
        }) => assert_eq!((expected, actual), (2, 1)),
        Err(other) => panic!("expected a WireCount on the target side, got {other:?}"),
        Ok(_) => panic!("expected a target-side arity error, got a Traced value"),
    }
}

// ---- Term-side S2 round-trip of a Traced-built term ---------------------------

#[test]
fn traced_term_prints_and_reparses() {
    // The mixed pipeline's term is a left-nested compose chain (so it prints flat,
    // clear of the parser's nesting bound) and reparses identically — the typed
    // track hands a well-formed term straight back to the S2 textual surface.
    let pipeline = t_copy().then(t_scalar(2).par(t_scalar(3))).then(t_add());
    let term = pipeline.term().clone();
    let printed = print(&term);
    assert_eq!(parse::<Sfg>(&printed), Ok(term));
}

// ---- into_parts: the one-way door ---------------------------------------------

#[test]
fn into_parts_yields_arrow_and_matching_term() {
    let double = t_copy().then(t_add());
    let expected_term = double.term().clone();
    let (arrow, term) = double.into_parts();
    // The extracted term is the one the Traced denoted...
    assert_eq!(term, expected_term);
    // ...and the extracted arrow still runs (there is no way to reassemble a
    // Traced from these two halves — the sync invariant is construction-only).
    assert_eq!(arrow.run(Wire(21)).flatten(), vec![42]);
}
