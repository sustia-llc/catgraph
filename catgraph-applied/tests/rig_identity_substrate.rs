//! **Identity-substrate gate.** Proves the rig axioms hold when `0`/`1` are
//! the catgraph-native [`Zero`]/[`One`] (#219) — the identity traits the `Rig`
//! bound is written against.
//!
//! The gate is a semantic one, not a bookkeeping one: if the identities ever
//! differed in identity or absorbing behaviour from the `num`-crate ones this
//! substrate has been swapped with twice (`num` → `deep_causality_num` at the
//! Phase-2 re-substrate, then → native here), the failure would surface
//! downstream in magnitude's SNF / rig-axiom paths rather than at the swap.
//! Compiling this file is also what pins *where* `R::zero()`/`R::one()`
//! resolve — the CI grep-guard against stray `num::{Zero, One}` is only a
//! supplement to it.
//!
//! `Rig` is and stays a catgraph-native semiring — it is never anyone else's
//! `Ring`, whose lowest form requires `Sub` (which `BoolRig`/`Tropical` lack).

use catgraph_applied::rig::{
    BoolRig, Checked, F64Rig, One, Tropical, UnitInterval, Zero, verify_rig_axioms,
};

/// `R::zero()` / `R::one()` resolve through the native identity traits, and the
/// eight semiring axioms hold over a representative sample for each catgraph rig.
#[test]
fn rig_axioms_with_native_zero_one() {
    // BoolRig — finite, so exhaustive.
    {
        // Identities come from `catgraph_applied::rig`, not `num`.
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

/// Every primitive integer and float carries the native identities, so the
/// blanket `Rig` impl still lifts them (the module docs claim exactly this).
///
/// Written against a generic helper so each assertion is the *trait* method,
/// not an inherent one — a missing impl is a compile error, not a silent pass.
#[test]
fn primitives_carry_the_native_identities() {
    fn check<T: Zero + One + Clone + PartialEq + std::fmt::Debug>(zero: T, one: T) {
        assert_eq!(T::zero(), zero);
        assert_eq!(T::one(), one);
        assert!(T::zero().is_zero());
        assert!(T::one().is_one());
        assert!(!T::one().is_zero());
        assert!(!T::zero().is_one());
        // Blanket `Rig` lift — the point of implementing the identities at all.
        assert!(verify_rig_axioms(&zero, &one, &one).is_ok());
    }

    check::<i8>(0, 1);
    check::<i16>(0, 1);
    check::<i32>(0, 1);
    check::<i64>(0, 1);
    check::<i128>(0, 1);
    check::<isize>(0, 1);
    check::<u8>(0, 1);
    check::<u16>(0, 1);
    check::<u32>(0, 1);
    check::<u64>(0, 1);
    check::<u128>(0, 1);
    check::<usize>(0, 1);
    check::<f32>(0.0, 1.0);
    check::<f64>(0.0, 1.0);
}

/// `-0.0` is the additive identity, matching IEEE `-0.0 == 0.0` (the float
/// `is_zero` doc comment claims this, and `F64Rig`'s `-0.0`-normalizing
/// `Eq`/`Hash` posture from #58 depends on the two agreeing).
#[test]
fn negative_zero_is_zero_for_floats() {
    assert!((-0.0f64).is_zero());
    assert!((-0.0f32).is_zero());
    assert!(F64Rig(-0.0).is_zero());
    assert_eq!(F64Rig(-0.0), <F64Rig as Zero>::zero());
}

/// The generic wrappers over a primitive still resolve their identities through
/// the wrapped type's native impls — `Checked<T>` is the shipped case, and the
/// one whose `is_zero`/`is_one` deliberately answer `false` for `⊥`.
#[test]
fn checked_identities_delegate_and_reject_poison() {
    assert_eq!(<Checked<i64> as Zero>::zero(), Checked::new(0_i64));
    assert_eq!(<Checked<i64> as One>::one(), Checked::new(1_i64));
    assert!(<Checked<i64> as Zero>::zero().is_zero());
    assert!(<Checked<i64> as One>::one().is_one());

    let poisoned = Checked::new(4_000_000_000_i64) * Checked::new(4_000_000_000_i64);
    assert!(poisoned.is_poisoned());
    assert!(!poisoned.is_zero());
    assert!(!poisoned.is_one());
}
