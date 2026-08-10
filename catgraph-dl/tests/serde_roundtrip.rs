//! Serde round-trip tests for the parameter-carrier surface (#230).
//!
//! The dl complement of `catgraph-applied`'s `tests/serde_roundtrip.rs` (#81):
//! applied persists *terms*, this persists the `RModule<S>` parameter data they
//! act on. Runs only under `--features serde`; the default build compiles
//! neither the derives nor this file. The `Dual` cases need `ad` as well, since
//! `para::ad` itself is `ad`-gated — CI runs them via `--features "serde ad"`.
//!
//! Beyond value round-trips, this file pins the wire-format claims the
//! `RModule` Serde rustdoc (the trust-boundary statement of record) makes:
//! byte-stable re-serialization (the #81 pattern), the documented
//! wrong-dimension degradation, and the non-finite-float `null` asymmetry.
#![cfg(feature = "serde")]

mod common;

use catgraph_dl::para::{DirectSum, F64Module, RModule};

/// Serialize → deserialize → re-serialize, asserting the representation is
/// byte-stable (the #81 `Presentation` pattern): a silent wire-format change
/// would otherwise strand every previously persisted document.
fn json_round_trip_stable<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let json = serde_json::to_string(value).expect("serialize");
    let back: T = serde_json::from_str(&json).expect("deserialize");
    let json2 = serde_json::to_string(&back).expect("re-serialize");
    assert_eq!(json, json2, "wire representation must be round-trip stable");
    back
}

#[test]
fn rmodule_json_round_trip_is_identity_and_byte_stable() {
    // A generic module, the zero-dimensional unit R⁰, and the zero vector of a
    // positive dimension — `dim` is carried by the coordinate count alone, so
    // the empty case is the one that could plausibly go missing.
    let cases: [F64Module; 3] = [
        RModule::new(vec![1.5, -2.0, 0.25]),
        RModule::zero_dim(),
        RModule::zeros(4),
    ];

    for module in cases {
        let back = json_round_trip_stable(&module);
        assert_eq!(module, back, "RModule must survive a JSON round-trip");
    }
}

#[test]
fn deserialized_module_still_satisfies_the_module_axioms() {
    // Run a deserialized block through the crate's own law drivers
    // (tests/common/mod.rs — the shared fixtures, not a fork): the axioms must
    // hold on loaded data exactly as on constructed data.
    let json = "[0.5,-3.25,2.0]";
    let back: F64Module = serde_json::from_str(json).expect("deserialize");
    common::assert_f64_module_axioms(back.as_slice().to_vec(), 1.75);
}

#[test]
fn wrong_dimension_document_degrades_add_to_none() {
    // The rustdoc's documented degradation, pinned: `add` is the operation
    // that rejects a mismatched dimension with `None`. (The same doc is
    // explicit that `scale`/`direct_sum`/`flatten`/`as_slice` do NOT check —
    // the consumer's `dim()` check at the entry point is the discipline.)
    let expected_dim_2 = RModule::new(vec![1.0, 2.0]);
    let forged: F64Module = serde_json::from_str("[9.0,9.0,9.0]").expect("deserialize");
    assert_eq!(forged.dim(), 3, "a document's dim is its coordinate count");
    assert!(
        expected_dim_2.add(&forged).is_none(),
        "add must reject the wrong-dimension loaded block with None",
    );
}

#[test]
fn non_finite_floats_serialize_to_null_and_do_not_load_back() {
    // The documented serde_json asymmetry, pinned: saving a non-finite state
    // succeeds (destroying the values as `null`) and loading it back fails.
    let module = RModule::new(vec![1.0, f64::NAN, f64::INFINITY]);
    let json = serde_json::to_string(&module).expect("serialize succeeds");
    assert_eq!(
        json, "[1.0,null,null]",
        "serde_json writes non-finite floats as null, indistinguishably",
    );
    assert!(
        serde_json::from_str::<F64Module>(&json).is_err(),
        "the null-bearing document must fail to deserialize as f64",
    );
}

#[test]
fn direct_sum_round_trips_and_still_flattens() {
    let left = RModule::new(vec![1.0, 2.0]);
    let right = RModule::new(vec![3.0]);
    let sum = DirectSum(left, right);

    let back = json_round_trip_stable(&sum);
    assert_eq!(sum, back, "DirectSum must survive a JSON round-trip");

    // The `⊕` realisation still holds on the deserialized value: R² ⊕ R¹ = R³,
    // left block first — and the monoid laws hold on the loaded coordinate
    // data via the shared law driver.
    assert_eq!(
        back.flatten(),
        RModule::new(vec![1.0, 2.0, 3.0]),
        "a round-tripped direct sum must flatten to the same concatenation",
    );
    common::assert_direct_sum_monoid(vec![1.0, 2.0], vec![3.0], vec![-4.0, 0.5]);
}

#[test]
fn direct_sum_with_the_empty_summand_round_trips() {
    // R⁰ ⊕ v = v is the unit law the module docs lean on; the empty summand is
    // the serialization case that could plausibly go missing (an empty array
    // in one slot of the pair).
    let sum = DirectSum(RModule::zero_dim(), RModule::new(vec![7.0]));
    let back = json_round_trip_stable(&sum);
    assert_eq!(sum, back, "the R⁰ summand must survive the round-trip");
    assert_eq!(
        back.flatten(),
        RModule::new(vec![7.0]),
        "R⁰ ⊕ v must still flatten to v after loading",
    );
}

#[cfg(feature = "ad")]
mod ad {
    use catgraph_dl::para::RModule;
    use catgraph_dl::para::ad::Dual;

    use super::json_round_trip_stable;

    #[test]
    fn dual_round_trips_both_channels() {
        for dual in [
            Dual::variable(3.0_f64),
            Dual::constant(-1.25_f64),
            Dual::new(0.5_f64, 7.0_f64),
        ] {
            let back = json_round_trip_stable(&dual);
            assert_eq!(dual, back, "Dual must survive a JSON round-trip");
        }
    }

    #[test]
    fn dual_module_round_trips_and_still_differentiates() {
        // The `DualF64Module = RModule<Dual<f64>>` instantiation: a seeded
        // parameter block, one variable and one constant coordinate.
        let module = RModule::new(vec![Dual::variable(2.0_f64), Dual::constant(5.0_f64)]);

        let back = json_round_trip_stable(&module);
        assert_eq!(
            module, back,
            "RModule<Dual<f64>> must survive a JSON round-trip",
        );

        // Arithmetic on the deserialized block still carries derivatives:
        // 3 · (x, c) has tangent (3, 0).
        let scaled = back.scale(Dual::constant(3.0_f64));
        let coords = scaled.as_slice();
        assert_eq!(coords[0].value(), 6.0);
        assert_eq!(coords[0].derivative(), 3.0);
        assert_eq!(coords[1].value(), 15.0);
        assert_eq!(coords[1].derivative(), 0.0);
    }
}
