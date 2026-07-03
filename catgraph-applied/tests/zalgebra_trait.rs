//! Marker test for the §1.17 `ZAlgebra` trait in catgraph-applied.
//!
//! Renamed `Integer` to `ZAlgebra` (Bourbaki Algèbre Ch. I §8 — ℤ as
//! initial object of the category of unital rings) and
//! sealed it via `crate::integer::private::Sealed`. This test exercises
//! the top-level re-export `catgraph_applied::ZAlgebra` (the canonical
//! short path) — the long path `catgraph_applied::integer::ZAlgebra`
//! remains valid as well.

use catgraph_applied::ZAlgebra;

#[test]
fn zalgebra_trait_marker_compiles() {
    // Sanity check: ZAlgebra trait can be bound on a generic.
    fn _generic<I: ZAlgebra>(_x: I) {}
}
