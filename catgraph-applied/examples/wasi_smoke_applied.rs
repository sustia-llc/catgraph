//! WASI smoke example — verifies catgraph-applied compiles and runs under
//! `wasm32-wasip1` / `wasm32-wasip1-threads`.
//!
//! Exercises a representative applied-CT slice: construct two
//! `LinearCombination`s and multiply them through the convolution product.
//! With `--features parallel` (default, plus `wasm32-wasip1-threads`), above
//! `PARALLEL_MUL_THRESHOLD` the rayon_cond::CondIterator arm fires; with
//! `--no-default-features` (plain `wasm32-wasip1`) the same path runs
//! sequentially with no rayon in the dep graph.
//!
//! ## Running
//!
//! Native: `cargo run --example wasi_smoke_applied -p catgraph-applied`
//!
//! WASM: see the note in `catgraph/examples/wasi_smoke_core.rs` about the
//! proptest → wait-timeout dev-dep resolution quirk. The library itself
//! builds clean:
//! ```sh
//! cargo build --lib --target wasm32-wasip1-threads -p catgraph-applied
//! cargo build --lib --target wasm32-wasip1 --no-default-features -p catgraph-applied
//! ```

use catgraph_applied::linear_combination::LinearCombination;

fn main() {
    // Build two ~40-term linear combinations — above the parallel threshold
    // (32), so the CondIterator parallel arm would activate on native with
    // the `parallel` feature on. Sequential arm on `--no-default-features`.
    let a: LinearCombination<i64, i64> = (0..40).map(|i| (i, 1_i64)).collect();
    let b: LinearCombination<i64, i64> = (0..40).map(|i| (i, 2_i64)).collect();

    let product = a * b;
    let all_positive = product.all_terms_satisfy(|t| *t >= 0);
    println!("catgraph-applied WASI smoke: LinearCombination product");
    println!("  all basis terms non-negative in a*b: {all_positive}");
}
