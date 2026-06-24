//! Generic multiway (non-deterministic) computation infrastructure.
//!
//! Provides data structures and algorithms for branching computation systems
//! where multiple execution paths exist simultaneously. This includes:
//!
//! - [`MultiwayEvolutionGraph`]: Core graph for tracking branching state evolution
//! - [`run_multiway_bfs`]: Generic BFS explorer for any non-deterministic system
//! - [`BranchialGraph`]: Time-slice foliation (tensor product structure at each step)
//! - [`DiscreteCurvature`]: Trait for curvature backends on branchial graphs
//! - [`OllivierRicciCurvature`]: Ollivier-Ricci curvature via Wasserstein transport
//! - [`wasserstein_1`]: Transportation simplex W₁ optimal transport solver

pub mod branchial;
pub mod branchial_analysis;
pub mod branchial_spectrum;
pub mod curvature;
pub mod evolution_graph;
pub mod ollivier_ricci;
pub mod wasserstein;

pub use branchial::{
    BranchialGraph, BranchialStepStats, BranchialSummary, branchial_parallel_step_pairs,
    extract_branchial_foliation, find_all_merge_points,
};
pub use branchial_analysis::{
    branchial_articulation_points, branchial_coloring, branchial_core_numbers,
};
pub use branchial_spectrum::BranchialSpectrum;
pub use curvature::{CurvatureFoliation, DiscreteCurvature};
pub use evolution_graph::{
    BranchId, ConfluenceDiamond, MergePoint, MultiwayCycle, MultiwayEdge, MultiwayEdgeKind,
    MultiwayEvolutionGraph, MultiwayNode, MultiwayNodeId, MultiwayStatistics, run_multiway_bfs,
};
pub use ollivier_ricci::{OllivierFoliation, OllivierRicciCurvature};
pub use wasserstein::wasserstein_1;
