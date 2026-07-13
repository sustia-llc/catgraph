//! Serde round-trip for a full syntax term `PropExpr<FrobeniusOr<G>>` (#81,
//! syntax complement). The machine analogue of the textual parser/printer
//! round-trip in `tests/persistence.rs`.
//!
//! Runs only under `--features serde`.
#![cfg(feature = "serde")]

use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::frobenius::FrobeniusOr;

// A user generator that is itself serde-able, so the whole term is.
type G = FrobeniusOr<SfgGenerator<i64>>;

/// A term mixing Frobenius spiders with a `User` generator carrying a payload:
/// `η ; (δ ⊗ scalar(7)) ; …` shape — exercises `Mu`/`Eta`/`Delta` and
/// `User(SfgGenerator::Scalar)`.
fn sample_term() -> PropExpr<G> {
    // eta : 0 → 1 ; delta : 1 → 2  →  0 → 2
    let eta = Free::generator(FrobeniusOr::Eta);
    let delta = Free::generator(FrobeniusOr::Delta);
    let left = Free::compose(eta, delta).expect("eta(0→1) ; delta(1→2)");
    // tensor a User scalar (1 → 1) alongside — result 1 → 1, then no compose.
    let user_scalar: PropExpr<G> = Free::generator(FrobeniusOr::User(SfgGenerator::Scalar(7)));
    Free::tensor(left, user_scalar) // 1 → 3
}

#[test]
fn frobenius_term_json_round_trip_is_identity() {
    let term = sample_term();
    let json = serde_json::to_string(&term).expect("serialize");
    let back: PropExpr<G> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        term, back,
        "a FrobeniusOr term must survive a JSON round-trip"
    );
}

/// Every bare `FrobeniusOr` variant round-trips (including `User`).
#[test]
fn every_variant_round_trips() {
    let variants: [G; 5] = [
        FrobeniusOr::Mu,
        FrobeniusOr::Eta,
        FrobeniusOr::Delta,
        FrobeniusOr::Epsilon,
        FrobeniusOr::User(SfgGenerator::Add),
    ];
    for v in variants {
        let json = serde_json::to_string(&v).expect("serialize");
        let back: G = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(v, back);
    }
}
