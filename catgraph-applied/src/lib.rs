//! Applied category theory extensions for catgraph.
//!
//! This crate packages modules that build on catgraph's Fong-Spivak 2019 core
//! (cospans, spans, Frobenius, hypergraph categories) but are **not** part of
//! the F&S 2019 paper's numbered content. It is the applied-CT complement to
//! the strict core crate.
//!
//! ## Modules
//!
//! - [`wiring_diagram`] — operadic substitution on named cospans
//! - [`petri_net`] — place/transition nets with cospan bridge
//! - [`temperley_lieb`] — Temperley-Lieb / Brauer algebra via perfect matchings
//!   (composition connectivity via the zero-dep `ultragraph` substrate)
//! - [`linear_combination`] — formal linear combinations over a coefficient ring
//!   (used internally by `temperley_lieb`)
//! - [`e1_operad`] — little-intervals operad (E₁)
//! - [`e2_operad`] — little-disks operad (E₂)
//! - [`decorated_cospan`] — generic `DecoratedCospan<F>` over a `Decoration` functor
//!   (Fong–Spivak Def 6.75 + Thm 6.77; v0.3.0/v0.3.1)
//! - [`prop`] — symmetric strict monoidal categories with `Ob = ℕ` and the
//!   free prop `Free(G)` on a signature (F&S Def 5.2, Def 5.25; v0.4.0)
//! - [`operad_algebra`] — operad algebras `F : O → Set` with `CircAlgebra`
//!   (F&S Def 6.99, Ex 6.100; v0.4.0)
//! - [`operad_functor`] — functors between operads with the canonical
//!   `E₁ ↪ E₂` inclusion (F&S Rough Def 6.98; v0.4.0)
//! - [`enriched`] — `EnrichedCategory<V>` trait + `HomMap<O, V>` concrete impl
//!   (F&S §2.4, CTFP Ch 28; v0.5.1)
//! - [`lawvere_metric`] — `LawvereMetricSpace<T>` over `Tropical` with triangle
//!   inequality verifier + `-ln π` embedding from `UnitInterval`
//!   (Lawvere 1973, BTV 2021; v0.5.1)
//! - [`integer`] — `ZAlgebra` trait (sealed; Bourbaki Algèbre Ch. I §8 — ℤ as initial object of the category of unital rings)
//!   for rigs carrying integer-exact arithmetic (substrate for
//!   catgraph-magnitude §1.17 Leinster 2008 Cor 1.5 chain-sum Möbius;
//!   `ZAlgebra` is also re-exported at the crate root as
//!   [`ZAlgebra`]; renamed from `Integer` and sealed at v0.6.0
//!   (introduced as `Integer` in v0.5.6))
//! - [`z`] — `Z(BigInt)` newtype, the canonical [`integer::ZAlgebra`]
//!   implementor for catgraph-magnitude §1.17 integer-exact Möbius
//!   inversion (v0.5.6)
//! - [`mat_kron`] — `MatKron(R)` FdVect with the Kronecker tensor: a genuine
//!   hypergraph category with the Hadamard SCFM as inherent generators
//!   (F&S 2019 Ex 2.16); concrete re-expression on the native
//!   `Monoidal`/`Composable`/`SymmetricMonoidalMorphism` traits
//! - [`trace`] — partial trace `Tr_X(f) : A → B` from the compact-closed
//!   structure of [`mat_kron`] (F&S 2019 §2.6); strict tensor, no
//!   associators/unitors required
//! - [`hypergraph`] — a CRUD hypergraph container (`Hypergraph<V, HE>`), the
//!   zero-dependency K1 backend for the downstream koalisi coalition layer
//!   (sustia-llc/koalisi#4), with a `hyperedge_as_cospan` categorical view (the
//!   identity cospan over the member index list) back to
//!   [`catgraph::cospan::Cospan`] (v0.6.x)
//!
//! ## Relationship to catgraph
//!
//! All modules depend on catgraph's public API:
//! - `Cospan`, `NamedCospan`, `Span`, `Rel` — pushout/pullback composition
//! - `Frobenius` generators — operadic composition of SMCs (Prop 3.8)
//! - `HypergraphCategory` trait — target of applied semantic functors
//! - `Operadic` trait — abstract interface for substitution
//! - `compact_closed` cup/cap — string-diagram rewriting (TL, wiring)
//!
//! See [`docs/FS18-AUDIT.md`](https://github.com/tsondru/catgraph/blob/main/catgraph-applied/docs/FS18-AUDIT.md)
//! for alignment with Fong & Spivak, *Seven Sketches in Compositionality*
//! (arXiv:1803.05316v3, 2018), Chapters 4–6.

/// Numerical epsilon for f32 geometric comparisons in operads.
pub(crate) const F32_EPSILON: f32 = 1e-6;

pub mod decorated_cospan;
pub mod e1_operad;
pub mod e2_operad;
pub mod enriched;
pub mod graphical_linalg;
pub mod hypergraph;
pub mod integer;
pub mod lawvere_metric;
pub mod linear_combination;
pub mod mat;
#[cfg(feature = "f64-rig")]
pub mod mat_f64;
pub mod mat_kron;
pub mod operad_algebra;
pub mod operad_functor;
pub mod petri_net;
pub mod prop;
pub mod rig;
pub mod sfg;
pub mod sfg_to_mat;
pub mod temperley_lieb;
pub mod trace;
pub mod wiring_diagram;
pub mod z;

// Convenience re-export: the canonical short path is
// `use catgraph_applied::ZAlgebra;` (parallels the cg-mag-side
// convenience-path convention). Long path `use
// catgraph_applied::integer::ZAlgebra;` remains valid.
pub use integer::ZAlgebra;

// Convenience re-exports for the K1 hypergraph container (koalisi#4 consumer
// surface). Long paths `catgraph_applied::hypergraph::…` remain valid.
pub use hypergraph::{HyperedgeIndex, Hypergraph, HypergraphError, VertexIndex};
