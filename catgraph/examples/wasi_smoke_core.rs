//! WASI smoke example — verifies the catgraph core API compiles and runs
//! under `wasm32-wasip1` / `wasm32-wasip1-threads`.
//!
//! Exercises a representative slice: build two cospans, compose them via
//! pushout, and print the result's left/right/middle shapes. The goal is
//! to flush out any WASM-hostile code path in the core types (pushout,
//! union-find, named boundary handling).
//!
//! ## Running
//!
//! Native: `cargo run --example wasi_smoke_core -p catgraph`
//!
//! WASM build (requires temporarily removing `proptest` from
//! `catgraph/Cargo.toml` `[dev-dependencies]` — see the W.1 CHANGELOG entry
//! for the rationale: cargo resolves dev-deps for any `--example` build and
//! proptest pulls `wait-timeout` which doesn't support `wasm32-*`. The
//! library itself builds clean — `cargo build --lib --target ...` works
//! out of the box):
//! ```sh
//! cargo build --lib --target wasm32-wasip1-threads -p catgraph
//! cargo build --lib --target wasm32-wasip1 --no-default-features -p catgraph
//! ```
//!
//! To run the example itself under wasmtime, build with the workaround and
//! invoke:
//! ```sh
//! wasmtime run --wasi threads=y \
//!     target/wasm32-wasip1-threads/debug/examples/wasi_smoke_core.wasm
//! ```

use catgraph::category::Composable;
use catgraph::cospan::Cospan;

fn main() {
    // c1: left legs → {a, b}, right legs → {a, b, c}. Middle = [a, b, c].
    let c1: Cospan<char> = Cospan::new(vec![0, 1], vec![0, 1, 2], vec!['a', 'b', 'c']);
    // c2 is chosen so c1's right boundary matches c2's left boundary label-by-label:
    // c2.left → [a, b, c] (indices [0, 1, 2]), so pushout composes cleanly.
    let c2: Cospan<char> = Cospan::new(vec![0, 1, 2], vec![1, 2], vec!['a', 'b', 'c']);

    let composed = c1
        .compose(&c2)
        .expect("pushout composition should succeed on aligned boundaries");

    println!("catgraph WASI smoke: composed cospan shape");
    println!(
        "  left legs: {}  right legs: {}  middle atoms: {}",
        composed.left_to_middle().len(),
        composed.right_to_middle().len(),
        composed.middle().len(),
    );
}
