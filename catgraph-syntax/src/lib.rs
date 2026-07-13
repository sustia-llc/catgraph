//! # catgraph-syntax
//!
//! A textual generator/relation presentation surface for hypergraph-category
//! morphisms, expressed as terms of the free prop over a signature and printed
//! above [`catgraph-applied`](catgraph_applied)'s presentation / normal-form
//! engine (issue [#5](https://github.com/sustia-llc/catgraph/issues/5), the
//! Phase 6 milestone). The crate never re-derives the term AST or the decision
//! procedures — [`PropExpr<G>`](catgraph_applied::prop::PropExpr), the
//! [`Free`](catgraph_applied::prop::Free) smart constructors, `smc_nf`, and
//! `eq_mod` all live in applied; syntax adds the layers above that engine: the
//! *textual* surface (the [`print`](mod@text::print) / [`parse`](mod@text::parse)
//! round-trip and presentation files), the *interpreter*
//! ([`eval`] — the executable term-action of Def 5.25, with the R-linear
//! [`SfgModel`](eval::SfgModel) as its worked example), the *Frobenius layer*
//! ([`frobenius`] — the monochromatic free hypergraph category as
//! [`FrobeniusOr<G>`](frobenius::FrobeniusOr), its spider calculus, the nine
//! SCFM equations, and the sound [`to_mat_kron`](frobenius::to_mat_kron) checker,
//! F&S 2019), and the *typed builder* ([`traced`] — a
//! [`Traced<A, G>`](traced::Traced) pairs an executable haft
//! [`Arrow`](arrow_seam::Arrow) with the [`PropExpr`](catgraph_applied::prop::PropExpr)
//! term it denotes, bridged by [`Wires`](traced::Wires), so one value can be both
//! *run* and *reasoned about*; the first consumer of the [`arrow_seam`]).
//!
//! With the typed builder live, the five-layer S1–S5 surface of the
//! [#5](https://github.com/sustia-llc/catgraph/issues/5) milestone is **complete**.
//!
//! Anchors: Fong & Spivak 2018, *Seven Sketches in Compositionality*
//! (Def 5.25 = prop signature / `Free(G)`; Def 5.30 = a `G`-generated prop
//! expression; Def 5.33 = presentation; Thm 5.60 = Mat(R) normal form) and
//! Fong & Spivak 2019, *Hypergraph Categories* (the [`frobenius`] layer —
//! Def 2.5's SCFM, Prop 3.8, Thm 3.14).
//! Anchor-to-item map: [`docs/ANCHORS.md`](https://github.com/sustia-llc/catgraph/blob/main/catgraph-syntax/docs/ANCHORS.md).
//!
//! ## Two standing disclaimers
//!
//! These bound what this crate does — and, deliberately, does *not* — decide.
//! They are restated at each point of use.
//!
//! ### 1. The [#15](https://github.com/sustia-llc/catgraph/issues/15) completeness boundary
//!
//! Applied's congruence-closure decision
//! ([`Presentation::eq_mod`](catgraph_applied::prop::presentation::Presentation::eq_mod))
//! is **sound but syntactically incomplete by design**: it returns
//! `Ok(Some(true))` for a proven equality, but a `None`/`Ok(Some(false))` is
//! *not* a proof of inequality — it only means the congruence closure did not
//! establish the equation. **Complete** decisions come solely through the
//! functorial route
//! ([`Presentation::eq_mod_functorial`](catgraph_applied::prop::presentation::Presentation::eq_mod_functorial)
//! with a
//! [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor)),
//! which today means Mat(R) via
//! [`MatrixNFFunctor`](catgraph_applied::prop::presentation::functorial::MatrixNFFunctor)
//! (Seven Sketches Thm 5.60). Nothing in `catgraph-syntax` promotes an
//! incomplete `None` into a decision.
//!
//! ### 2. The monochromatic-fragment scope
//!
//! The [`frobenius`] layer (S4, live) presents the *single-sort* (monochromatic)
//! free hypergraph category — the object palette is `Λ = {•}`, one wire colour,
//! one spider family. F&S 2019 Thm 3.14's full **colored** generality (a distinct
//! spider family per colour) is out of scope here and tracked separately as
//! [#79](https://github.com/sustia-llc/catgraph/issues/79); multi-sorted /
//! Λ-colored prop expressions are an applied-side extension.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod arrow_seam;
pub mod cospan_functor;
pub mod depth;
pub mod errors;
pub mod eval;
pub mod frobenius;
pub mod sfg_syntax;
pub mod text;
pub mod traced;

/// Compile-tested README: the `parse → eval → matrix cross-check` example (and
/// any other Rust code blocks) are run as doctests via this hidden include, so
/// the milestone smoke example cannot rot. Non-Rust blocks in the README are
/// fenced as `text` so they are not compiled.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
mod readme {}
