//! Applied category theory extensions for catgraph — modules built on the
//! Fong-Spivak 2019 core (cospans, spans, Frobenius, hypergraph categories)
//! that are not part of that paper's numbered content.
//!
//! ## Modules
//!
//! - [`decorated_cospan`] — `Decoration` trait + `DecoratedCospan<Lambda, D>` (F&S Def 6.75, Thm 6.77)
//! - [`e1_operad`] — little-intervals operad (E₁)
//! - [`e2_operad`] — little-disks operad (E₂)
//! - [`enriched`] — `EnrichedCategory<V>` trait + `HomMap<O, V>` (F&S §2.4)
//! - [`graphical_linalg`] — the 18-equation Thm 5.60 presentation of `Mat(R)` (F&S §5.4)
//! - [`hypergraph`] — `Hypergraph<V, HE>` CRUD container with a `hyperedge_as_cospan`
//!   view back to [`catgraph::cospan::Cospan`]
//! - [`integer`] — sealed `ZAlgebra` trait for rigs carrying integer-exact
//!   arithmetic (Bourbaki *Algèbre* Ch. I §8)
//! - [`lawvere_metric`] — `LawvereMetricSpace<T>` over `Tropical` (Lawvere 1973)
//! - [`linear_combination`] — formal linear combinations over a coefficient ring
//! - [`mat`] — `MatR<R>`, the matrix prop over any `Rig` (F&S Def 5.50)
//! - `mat_f64` — nalgebra bridge for `MatR<F64Rig>` (feature `f64-rig`)
//! - [`mat_kron`] — `MatKron(R)`, FdVect with the Kronecker tensor (F&S 2019 Ex 2.16)
//! - [`mat_to_sfg`] — the realization `mat_to_sfg` (F&S Prop 5.56)
//! - [`operad_algebra`] — operad algebras `F : O → Set` with `CircAlgebra` (F&S Def 6.99, Ex 6.100)
//! - [`operad_functor`] — functors between operads with the `E₁ ↪ E₂` inclusion (F&S Rough Def 6.98)
//! - [`petri_net`] — place/transition nets with a cospan bridge
//! - [`prop`] — symmetric strict monoidal categories with `Ob = ℕ` and the free
//!   prop `Free(G)` on a signature (F&S Def 5.2, Def 5.25)
//! - [`rig`] — the `Rig` semiring trait with `BoolRig`, `UnitInterval`,
//!   `Tropical`, `F64Rig` (F&S Def 5.36)
//! - [`sfg`] — `SignalFlowGraph<R>`, the free prop on signal-flow generators (F&S Def 5.45)
//! - [`sfg_to_mat`] — the functor `S : SFG_R → Mat(R)` (F&S Thm 5.53)
//! - [`temperley_lieb`] — Temperley-Lieb / Brauer algebra via perfect matchings
//! - [`trace`] — partial trace `Tr_X(f) : A → B` on [`mat_kron`] (F&S 2019 §3.1)
//! - [`wiring_diagram`] — operadic substitution on named cospans
//! - [`z`] — the `Z(BigInt)` newtype implementing [`integer::ZAlgebra`]
//!
//! ## Features
//!
//! - `parallel` (default) — rayon arms in [`linear_combination`] and [`temperley_lieb`]; forwards `catgraph/parallel`
//! - `f64-rig` — exposes the `mat_f64` nalgebra bridge
//! - `serde` — `Serialize`/`Deserialize` on the term, rewrite-trace, and content-key types
//! - `internal-bench`, `internal-probes` — hooks for benches and tests; not public API
//!
//! See [`docs/FS18-AUDIT.md`](https://github.com/sustia-llc/catgraph/blob/main/catgraph-applied/docs/FS18-AUDIT.md)
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
pub mod mat_to_sfg;
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

// Short path for `integer::ZAlgebra`; the long path remains valid.
pub use integer::ZAlgebra;

// Short paths for the hypergraph container; the long paths remain valid.
pub use hypergraph::{HyperedgeIndex, Hypergraph, HypergraphError, VertexIndex};

/// The RNG supply contract for [`e1_operad::E1::random`]: `rand_core 0.10`'s
/// infallible generator trait `Rng` and its base trait `TryRng`. Re-exported so
/// callers can name the bound without a direct `rand_core` dependency; a custom
/// engine implements `TryRng<Error = Infallible>`, since `Rng` is
/// blanket-implemented over it and cannot be implemented directly.
pub use rand_core::{Rng, TryRng};
