//! Arrow substrate — the single import seam for `deep_causality_haft`.
//!
//! This is the **only** file in `catgraph-syntax` that names
//! `deep_causality_haft`, mirroring catgraph-dl's `src/endofunctor.rs`
//! precedent (issue [#12](https://github.com/sustia-llc/catgraph/issues/12)):
//! every other module imports the adopted Arrow names from here
//! (`crate::arrow_seam`), so the upstream dependency surface stays pinned to one
//! reviewable place.
//!
//! # What is consumed and why
//!
//! haft's Arrow algebra is the **execution target** for the typed builder
//! ([`crate::traced`], Phase S5 — **live**, and this seam's first real consumer):
//! a [`Traced<A, G>`](crate::traced::Traced) pairs an executable haft [`Arrow`]
//! with the [`PropExpr`](catgraph_applied::prop::PropExpr) term it denotes, so
//! a morphism can be both *run* and *reasoned about* from one value.
//!
//! ## Consumed by the builder
//!
//! These are the names [`crate::traced`] actually builds on:
//!
//! - [`Arrow`] — the trait (`run` + the combinator methods).
//! - [`Compose`] — sequential composition `f >>> g` (the term-level `;`), behind
//!   [`Traced::then`](crate::traced::Traced::then).
//! - [`Split`] — the true tensor `(A, C) → (B, D)` (the term-level `⊗`), behind
//!   [`Traced::par`](crate::traced::Traced::par).
//! - [`Id`] — the identity arrow, behind [`traced_id`](crate::traced::traced_id).
//! - [`Lift`] — the pure-function lift, behind
//!   [`traced_braid_1_1`](crate::traced::traced_braid_1_1) and the caller's
//!   generator arrows.
//!
//! ## Re-exported, deliberately not (yet) consumed — reserved surface
//!
//! These names round out the Arrow algebra for downstream users but the builder
//! does not (currently) need them; they are kept live for the same
//! documented-reserved-surface reason catgraph-dl kept its `num` dep ahead of the
//! #36 R-module actegory:
//!
//! - [`arrow`] / [`ArrowBuilder`] — the fluent lift/construction path; the
//!   ergonomic way for a downstream crate to build arrows to feed
//!   [`traced_generator`](crate::traced::traced_generator).
//! - [`First`] / [`Second`] — tensor with an identity on one side; achievable
//!   through the builder anyway (`par` with a [`traced_id`](crate::traced::traced_id)),
//!   so not exposed as a dedicated combinator.
//! - [`Fanout`] — the Cartesian diagonal `A → (A, A)`; **rejected** by the builder
//!   because pairing it with a term would let the arrow duplicate a wire no term
//!   generator copied (Fanout ≠ Frobenius `δ`) — [`crate::traced`]'s *Deliberate
//!   omissions* is the canonical statement; the name stays exported so that
//!   distinction can be *named*.
//!
//! All of these have been **live public API** from this seam since Phase S1.
//!
//! # Deliberate exclusions
//!
//! Some haft Arrow-adjacent names are intentionally **not** re-exported:
//!
//! - `Free` / `FreeWitness` — haft's free monad ships **no** `Eq` / `Clone` /
//!   `Debug` (opaque by design), but applied's congruence-closure engine
//!   *requires* `Eq + Hash` on terms (see
//!   [`PropSignature`](catgraph_applied::prop::PropSignature)). So
//!   [`PropExpr<G>`](catgraph_applied::prop::PropExpr) — which derives all of
//!   them — stays the term type, and haft's `Free` is not a substitute. The
//!   `FreeMnd` interplay is tracked in catgraph
//!   [#76](https://github.com/sustia-llc/catgraph/issues/76).
//! - The `IoAction` family (`IoAction`, `IoAndThen`, `IoMap`, …) — its `run`
//!   consumes `self` and it is IO-effect specific; it is an effect executor,
//!   not a reusable, inspectable term carrier, so it cannot back the
//!   term-plus-arrow pairing.
//! - `EndoArrow` — haft's endo-arrow (iteration) stays out: the shipped S5
//!   [`Traced`](crate::traced::Traced) builder wants no fixed-point / loop
//!   combinator, so it is not re-exported. Revisit only if a future phase adds a
//!   loop combinator.

// The adopted names live in `deep_causality_haft`; re-exported here so the rest
// of the crate imports them from a single seam (`crate::arrow_seam`). See the
// module docs above for the consumption rationale and the deliberate
// exclusions (`Free`/`FreeWitness`, the `IoAction` family, `EndoArrow`).
pub use deep_causality_haft::{
    Arrow, ArrowBuilder, Compose, Fanout, First, Id, Lift, Second, Split, arrow,
};
