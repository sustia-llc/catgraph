//! **P1 gate (Phase 2 re-substrate).** Proves the catgraph-native rig axioms
//! hold when `0`/`1` are sourced from `deep_causality_num::{Zero, One}` rather
//! than the `num` crate.
//!
//! This is the single structurally-novel DeepCausality contact of the cutover
//! (see the reboot cutover plan §4 critical-path risk): if DC's `Zero`/`One`
//! differed semantically from `num`'s (identity / absorbing behaviour), the
//! failure would surface downstream in magnitude's SNF / rig-axiom paths. This
//! gate fires here, in Phase 2, before Phase 3 opens.
//!
//! `Rig` is and stays a catgraph-native semiring — it is **never** a
//! `deep_causality_num::Ring` (DC's lowest ring requires `Sub`, which
//! `BoolRig` / `Tropical` lack). Only `Zero` / `One` are re-sourced from DC.

// These are the DeepCausality identity traits — the ones the re-substrate put
// behind the `Rig` bound. Bringing them into scope here makes the gate's intent
// explicit and lets us assert the identities resolve through DC.
use deep_causality_num::{One, Zero};

use catgraph_applied::rig::{BoolRig, F64Rig, Tropical, UnitInterval, verify_rig_axioms};

/// `R::zero()` / `R::one()` resolve through `deep_causality_num`, and the eight
/// semiring axioms hold over a representative sample for each catgraph rig.
#[test]
fn rig_axioms_with_dc_zero_one() {
    // BoolRig — finite, so exhaustive.
    {
        // Identities come from `deep_causality_num`, not `num`.
        assert_eq!(<BoolRig as Zero>::zero(), BoolRig(false));
        assert_eq!(<BoolRig as One>::one(), BoolRig(true));
        assert!(<BoolRig as Zero>::zero().is_zero());
        assert!(<BoolRig as One>::one().is_one());
        let universe = [BoolRig(false), BoolRig(true)];
        for a in universe {
            for b in universe {
                for c in universe {
                    verify_rig_axioms(&a, &b, &c)
                        .unwrap_or_else(|e| panic!("BoolRig {a:?},{b:?},{c:?}: {e}"));
                }
            }
        }
    }

    // F64Rig — sample including a negative (it is a ring, but only rig axioms
    // are claimed).
    {
        assert_eq!(<F64Rig as Zero>::zero(), F64Rig(0.0));
        assert_eq!(<F64Rig as One>::one(), F64Rig(1.0));
        assert!(<F64Rig as Zero>::zero().is_zero());
        assert!(<F64Rig as One>::one().is_one());
        let samples = [F64Rig(0.0), F64Rig(1.0), F64Rig(2.5), F64Rig(-1.0)];
        for a in samples {
            for b in samples {
                for c in samples {
                    verify_rig_axioms(&a, &b, &c).unwrap();
                }
            }
        }
    }

    // UnitInterval — Viterbi (max, ·). Dyadic fractions to dodge IEEE drift.
    {
        assert_eq!(
            <UnitInterval as Zero>::zero(),
            UnitInterval::new(0.0).unwrap()
        );
        assert_eq!(
            <UnitInterval as One>::one(),
            UnitInterval::new(1.0).unwrap()
        );
        assert!(<UnitInterval as Zero>::zero().is_zero());
        assert!(<UnitInterval as One>::one().is_one());
        let samples = [
            UnitInterval::new(0.0).unwrap(),
            UnitInterval::new(0.25).unwrap(),
            UnitInterval::new(0.5).unwrap(),
            UnitInterval::new(1.0).unwrap(),
        ];
        for a in samples {
            for b in samples {
                for c in samples {
                    verify_rig_axioms(&a, &b, &c).unwrap();
                }
            }
        }
    }

    // Tropical (min, +): additive zero is +∞, multiplicative one is real 0.
    {
        assert!(<Tropical as Zero>::zero().0.is_infinite());
        assert_eq!(<Tropical as One>::one(), Tropical(0.0));
        assert!(<Tropical as Zero>::zero().is_zero());
        assert!(<Tropical as One>::one().is_one());
        let samples = [
            Tropical(f64::INFINITY),
            Tropical(0.0),
            Tropical(1.5),
            Tropical(5.0),
        ];
        for a in samples {
            for b in samples {
                for c in samples {
                    verify_rig_axioms(&a, &b, &c).unwrap();
                }
            }
        }
    }
}
