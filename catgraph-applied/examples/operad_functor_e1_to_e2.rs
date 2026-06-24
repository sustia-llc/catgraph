//! F&S *Seven Sketches* Rough Def 6.98 — operad functor demo.
//!
//! Exhibits the canonical inclusion `E₁ ↪ E₂` of the little-intervals
//! operad into the little-disks operad, and verifies
//! `F(outer ∘₀ inner) ≡ F(outer) ∘₀ F(inner)` geometrically (up to disk
//! names).

use catgraph_applied::e1_operad::E1;
use catgraph_applied::operad_functor::{E1ToE2, OperadFunctor};

fn main() {
    let outer = E1::new(vec![(0.0, 0.5), (0.5, 1.0)], true).expect("outer well-formed");
    let inner = E1::new(vec![(0.0, 0.3), (0.3, 0.6), (0.6, 1.0)], true).expect("inner well-formed");

    // Map each E1 configuration to its E2 image (disjoint name ranges).
    let f_outer = E1ToE2::default().map_operation(&outer).unwrap();
    let f_inner = E1ToE2::with_offset(outer.arity())
        .map_operation(&inner)
        .unwrap();

    println!(
        "F(outer) has arity {} — x-axis disks along [-1, +1]",
        f_outer.arity_of()
    );
    println!(
        "F(inner) has arity {} — x-axis disks along [-1, +1]",
        f_inner.arity_of()
    );

    // Witness the functoriality law on two different substitution slots.
    E1ToE2::check_substitution_preserved(
        || E1::new(vec![(0.0, 0.5), (0.5, 1.0)], true).unwrap(),
        0,
        || E1::new(vec![(0.0, 0.3), (0.3, 0.6), (0.6, 1.0)], true).unwrap(),
    )
    .expect("slot 0 — geometric functoriality");

    E1ToE2::check_substitution_preserved(
        || E1::new(vec![(0.0, 0.4), (0.4, 1.0)], true).unwrap(),
        1,
        || E1::new(vec![(0.1, 0.5), (0.5, 0.9)], true).unwrap(),
    )
    .expect("slot 1 — geometric functoriality");

    println!("Functoriality F(o ∘_i q) = F(o) ∘_i F(q) verified on slots 0 and 1.");
}
