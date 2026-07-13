//! Serde round-trip tests for the term-persistence surface (#81).
//!
//! Runs only under `--features serde`; the default build compiles neither the
//! derives nor this file.
#![cfg(feature = "serde")]

use catgraph_applied::prop::presentation::{NormalizeEngine, Presentation};
use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;

type G = SfgGenerator<i64>;

/// A non-trivial `1 → 1` signal-flow term: `copy ; (scalar(3) ⊗ id) ; add`,
/// exercising every `PropExpr` variant (`Generator`, `Identity`, `Compose`,
/// `Tensor`) and a generator carrying an `R` payload (`Scalar`).
fn sample_term() -> PropExpr<G> {
    let copy = Free::generator(SfgGenerator::Copy); // 1 → 2
    let scaled = Free::tensor(
        Free::generator(SfgGenerator::Scalar(3_i64)), // 1 → 1
        Free::<G>::identity(1),                       // 1 → 1
    ); // 2 → 2
    let add = Free::generator(SfgGenerator::Add); // 2 → 1
    let left = Free::compose(copy, scaled).expect("copy(1→2) ; (2→2)");
    Free::compose(left, add).expect("(1→2) ; add(2→1)")
}

#[test]
fn propexpr_json_round_trip_is_identity() {
    let term = sample_term();
    let json = serde_json::to_string(&term).expect("serialize");
    let back: PropExpr<G> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(term, back, "PropExpr must survive a JSON round-trip");
}

#[test]
fn normalize_engine_round_trips() {
    for engine in [
        NormalizeEngine::Structural,
        NormalizeEngine::CongruenceClosure,
    ] {
        let json = serde_json::to_string(&engine).expect("serialize");
        let back: NormalizeEngine = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(engine, back);
    }
}

#[test]
fn presentation_round_trips_and_still_decides() {
    // A presentation with one equation: copy ; add = id(1).
    let mut pres = Presentation::<G>::new();
    let lhs = Free::compose(
        Free::generator(SfgGenerator::Copy),
        Free::generator(SfgGenerator::Add),
    )
    .expect("copy(1→2) ; add(2→1)");
    let rhs = Free::<G>::identity(1);
    pres.add_equation(lhs.clone(), rhs.clone())
        .expect("arities match");

    // Serialize → deserialize → serialize: the representation is stable.
    let json = serde_json::to_string(&pres).expect("serialize");
    let back: Presentation<G> = serde_json::from_str(&json).expect("deserialize");
    let json2 = serde_json::to_string(&back).expect("re-serialize");
    assert_eq!(json, json2, "Presentation JSON must be round-trip stable");

    // The deserialized presentation still decides the equation it carries.
    assert_eq!(
        back.eq_mod(&lhs, &rhs).expect("eq_mod runs"),
        Some(true),
        "a round-tripped presentation must still prove its own equation",
    );
}
