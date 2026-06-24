//! Showcase of the 4 concrete Rig instances in catgraph-applied.
//!
//! Iterates over `BoolRig`, `UnitInterval`, `Tropical`, `F64Rig` and for each
//! rig prints `zero()`, `one()`, and `a + b`, `a * b` on a sample pair.
//! Useful as a smoke-test / introductory example.
//!
//! Run: `cargo run -p catgraph-applied --example rig_showcase`

// Single-char variable names are natural for a semiring-element showcase.
#![allow(clippy::many_single_char_names)]

use catgraph_applied::rig::{BoolRig, F64Rig, Tropical, UnitInterval};
use deep_causality_num::{One, Zero};

fn main() {
    println!("=== Rig showcase ===\n");

    // BoolRig: (∨, ∧)
    println!("BoolRig:");
    println!("  zero = {:?}", BoolRig::zero());
    println!("  one  = {:?}", BoolRig::one());
    println!(
        "  BoolRig(true) + BoolRig(false)   = {:?} (∨)",
        BoolRig(true) + BoolRig(false)
    );
    println!(
        "  BoolRig(true) * BoolRig(false)   = {:?} (∧)",
        BoolRig(true) * BoolRig(false)
    );

    // UnitInterval: (max, ·) Viterbi semiring
    println!("\nUnitInterval (Viterbi):");
    println!("  zero = {:?}", UnitInterval::zero());
    println!("  one  = {:?}", UnitInterval::one());
    let p = UnitInterval::new(0.5).unwrap();
    let q = UnitInterval::new(0.3).unwrap();
    println!("  {p:?} + {q:?}   = {:?} (max)", p + q);
    println!("  {p:?} * {q:?}   = {:?} (·)", p * q);

    // Tropical: (min, +)
    println!("\nTropical (min-plus):");
    println!("  zero = Tropical(+∞)  (no connection)");
    println!("  one  = {:?}  (zero distance)", Tropical::one());
    let a = Tropical(3.0);
    let b = Tropical(2.0);
    println!("  {a:?} + {b:?} = {:?} (min)", a + b);
    println!("  {a:?} * {b:?} = {:?} (distance sum)", a * b);

    // F64Rig: plain real rig
    println!("\nF64Rig:");
    println!("  zero = {:?}", F64Rig::zero());
    println!("  one  = {:?}", F64Rig::one());
    let x = F64Rig(2.5);
    let y = F64Rig(1.5);
    println!("  {x:?} + {y:?} = {:?}", x + y);
    println!("  {x:?} * {y:?} = {:?}", x * y);

    println!("\nAll 4 rigs verify the semiring axioms via verify_rig_axioms (see tests).");
}
