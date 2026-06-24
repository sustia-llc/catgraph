//! Demonstrates the functor `S: SFG_R → Mat(R)` from F&S 2018 Thm 5.53.
//!
//! Builds a small signal flow graph over [`F64Rig`] — `copy ; (scalar(2.0) ⊗
//! scalar(3.0)) ; add` — and applies the S functor to see the resulting 1×1
//! matrix `[[5.0]]` (because copy-two-wires-then-scale-then-add is the
//! amplification factor `2 + 3 = 5`).
//!
//! Run: `cargo run -p catgraph-applied --example sfg_to_mat`

use catgraph_applied::{rig::F64Rig, sfg::SignalFlowGraph, sfg_to_mat::sfg_to_mat};

fn main() {
    println!("=== S: SFG_R → Mat(R) functor demo ===\n");

    // Build: copy ; (scalar(2.0) ⊗ scalar(3.0)) ; add
    // Arity flow: 1 →[copy]→ 2 →[scalar⊗scalar]→ 2 →[add]→ 1
    // Matrix: 1×2 * 2×2 * 2×1 = 1×1 with entry (1*2*1 + 1*3*1) = 5.0.
    let s2 = SignalFlowGraph::<F64Rig>::scalar(F64Rig(2.0));
    let s3 = SignalFlowGraph::<F64Rig>::scalar(F64Rig(3.0));
    let pipeline = SignalFlowGraph::<F64Rig>::copy()
        .compose(&s2.tensor(&s3))
        .unwrap()
        .compose(&SignalFlowGraph::<F64Rig>::add())
        .unwrap();

    println!("SFG: copy(1→2) ; (scalar(2) ⊗ scalar(3)) ; add(2→1)");

    let matrix = sfg_to_mat(&pipeline).unwrap();
    println!("\nS(SFG) is a {}×{} matrix:", matrix.rows(), matrix.cols());
    for row in matrix.entries() {
        println!("  {row:?}");
    }

    assert_eq!(matrix.rows(), 1);
    assert_eq!(matrix.cols(), 1);
    // entry = one*2*one + one*3*one = 5.0
    assert_eq!(matrix.entries()[0][0], F64Rig(5.0));

    // Also show the primitive generators' matrices from Eq 5.52.
    println!("\n=== Eq 5.52 generator table (F64Rig) ===");
    for (name, sfg) in [
        ("Copy    : 1→2", SignalFlowGraph::<F64Rig>::copy()),
        ("Discard : 1→0", SignalFlowGraph::<F64Rig>::discard()),
        ("Add     : 2→1", SignalFlowGraph::<F64Rig>::add()),
        ("Zero    : 0→1", SignalFlowGraph::<F64Rig>::zero()),
        (
            "Scalar(π): 1→1",
            SignalFlowGraph::<F64Rig>::scalar(F64Rig(std::f64::consts::PI)),
        ),
    ] {
        let m = sfg_to_mat(&sfg).unwrap();
        println!(
            "  {name} → {}×{} matrix: {:?}",
            m.rows(),
            m.cols(),
            m.entries()
        );
    }
}
