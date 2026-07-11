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
//! a morphism can be both *run* and *reasoned about* from one value. The
//! re-exports below are the combinators that builder needs:
//!
//! - [`Arrow`] / [`arrow`] — the trait and its lifting constructor.
//! - [`ArrowBuilder`] — the fluent construction surface.
//! - [`Compose`] — sequential composition `f >>> g` (the term-level `;`).
//! - [`Split`] — the true tensor `(A, C) → (B, D)` (the term-level `⊗`).
//! - [`First`] / [`Second`] — tensor with an identity on one wire bundle.
//! - [`Fanout`] — the diagonal `A → (B, C)`. **Not** Frobenius `δ`: fanout is
//!   the Cartesian diagonal (copy is free in `Set`), whereas a comonoid
//!   comultiplication is a structure map that a *model* must supply. Keeping
//!   both names available lets Phase S5 document the distinction at the type
//!   level rather than conflate them.
//! - [`Id`] / [`Lift`] — the identity arrow and pure-function lift.
//!
//! These names have been **live public API** from this seam since Phase S1 (the
//! same documented-reserved-surface pattern catgraph-dl used for its `num` dep
//! ahead of the #36 R-module actegory); as of Phase S5 the
//! [`Traced`](crate::traced::Traced) builder is their first consumer.
//!
//! [`Fanout`] is re-exported but **deliberately unused** by the builder: pairing
//! the Cartesian diagonal `A → (A, A)` with a term would let the arrow duplicate a
//! wire that no term generator copied, so the two would denote different
//! morphisms. [`crate::traced`] documents the rejection and makes the
//! Fanout-≠-Frobenius-δ discipline type-level (the diagonal is unreachable through
//! the builder); the name stays exported so that distinction can be *named*.
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
