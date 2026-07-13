//! Category-theoretic graph structures with source/target (cospan) semantics.
//!
//! **catgraph** implements applied category theory for compositional systems,
//! focusing on Fong-Spivak-style string diagrams and cospans. Hyperedges connect
//! *source sets* to *target sets* (cospan semantics), as opposed to path-based
//! semantics where vertices are chained sequentially.
//!
//! ## Core abstractions
//!
//! - [`category`] — composition traits: [`HasIdentity`](category::HasIdentity),
//!   [`Composable`](category::Composable), [`ComposableMutating`](category::ComposableMutating)
//! - [`cospan`] — pushout composition over union-find (nearly linear time)
//! - [`span`] — pullback composition (dual of cospan) and [`Rel`](span::Rel) relation algebra
//! - [`named_cospan`] — port-labeled cospans for wiring-style composition
//! - [`monoidal`] — tensor product, symmetric braiding, generic layered morphisms
//!
//! ## Frobenius, compact closed, hypergraph categories
//!
//! - [`frobenius`] — Frobenius algebra generators, layered morphisms, DAG-based interpretation
//! - [`compact_closed`] — self-dual compact closed structure: cup/cap morphisms (Fong-Spivak §3.1)
//! - [`cospan_algebra`] — lax monoidal functors from cospans to sets (Fong-Spivak §2.1)
//! - [`hypergraph_category`] — hypergraph category trait with Frobenius generators (Fong-Spivak §2.3)
//! - [`hypergraph_functor`] — structure-preserving maps between hypergraph categories (Fong-Spivak §2.3)
//! - [`equivalence`] — cospan-algebra morphism + Thm 1.2 per-Λ roundtrip (Fong-Spivak §4)
//! - [`operadic`] — the `Operadic` trait for substitution (concrete impls live in `catgraph-applied`)
//!
//! ## Finite sets and combinatorics
//!
//! - [`finset`] — finite set morphisms, epi-mono factorization, permutations
//!
//! ## Out of scope — see sibling crates
//!
//! - Hypergraph DPO rewriting, multiway evolution, gauge theory, branchial analysis → `catgraph-physics`
//! - Petri nets, wiring diagrams, `E_n` operads, Temperley-Lieb, linear combinations → `catgraph-applied`
//! - Persistence → `catgraph-surreal` (sibling repo)
//! - Computational irreducibility → `irreducible` (sibling repo)

pub mod category;
pub mod compact_closed;
pub mod corel;
pub mod cospan;
pub mod cospan_algebra;
pub mod cospan_canon;
pub mod equivalence;
pub mod errors;
pub mod finset;
pub mod frobenius;
pub mod hypergraph_category;
pub mod hypergraph_functor;
pub mod monoidal;
pub mod named_cospan;
pub mod operadic;
pub mod span;
pub mod utils;
