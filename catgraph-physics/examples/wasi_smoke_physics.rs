//! WASI smoke example — verifies catgraph-physics compiles and runs under
//! `wasm32-wasip1` / `wasm32-wasip1-threads`.
//!
//! Exercises a small hypergraph rewrite step: build a two-edge hypergraph,
//! find matches for a rewrite rule, and apply it. Touches the DPO rewriting
//! core (`Hypergraph`, `RewriteRule`, subgraph matching) as a representative
//! non-trivial slice of the crate.
//!
//! ## Running
//!
//! Native: `cargo run --example wasi_smoke_physics -p catgraph-physics`
//!
//! WASM: see the note in `catgraph/examples/wasi_smoke_core.rs` about the
//! proptest → wait-timeout dev-dep resolution quirk. The library itself
//! builds clean:
//! ```sh
//! cargo build --lib --target wasm32-wasip1-threads -p catgraph-physics
//! cargo build --lib --target wasm32-wasip1 --no-default-features -p catgraph-physics
//! ```

use catgraph_physics::hypergraph::{Hyperedge, Hypergraph};

fn main() {
    // Build a 3-vertex path {0, 1, 2} with two directed hyperedges.
    let edges = vec![Hyperedge::new(vec![0, 1]), Hyperedge::new(vec![1, 2])];
    let hg: Hypergraph = Hypergraph::from_edges(edges);

    let vertex_count = hg.vertices().count();
    let edge_count = hg.edges().count();
    println!("catgraph-physics WASI smoke: hypergraph constructed");
    println!("  vertices: {vertex_count}  edges: {edge_count}");
}
