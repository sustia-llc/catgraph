//! Hypergraph rewriting for compositional systems.
//!
//! Provides hypergraphs, DPO (Double-Pushout) rewrite rules, multiway
//! evolution tracking, gauge theory, and categorical span/cospan bridges.
//!
//! [`Hyperedge`] is an ordered vertex sequence generalizing graph edges.
//! [`Hypergraph`] stores vertices and hyperedges with pattern matching.

pub mod causal_graph;
pub mod evolution;
pub mod evolution_cospan;
pub mod gauge;
pub mod hyperedge;
#[allow(clippy::module_inception)]
pub mod hypergraph;
pub mod multiway_cospan;
pub mod rewrite_rule;
pub mod rewrite_span;

pub use causal_graph::{CausalComparison, CausalEvent, CausalGraph, EdgeId, EventId};
pub use evolution::{
    CausalInvarianceResult, HypergraphEvolution, HypergraphNode, HypergraphStep, WilsonLoop,
};
pub use gauge::{
    GaugeGroup, HypergraphLattice, HypergraphRewriteGroup, plaquette_action, total_action,
};
pub use hyperedge::Hyperedge;
pub use hypergraph::Hypergraph;
pub use multiway_cospan::{
    CospanInvarianceResult, CospanMergeDetail, MultiwayCospan, MultiwayCospanExt,
    MultiwayCospanGraph,
};
pub use rewrite_rule::{RewriteEffect, RewriteMatch, RewriteRule, RewriteSpan};
