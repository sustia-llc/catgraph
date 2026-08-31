//! # catgraph-syntax
//!
//! A textual generator/relation presentation surface for hypergraph-category
//! morphisms, expressed as terms of the free prop over a signature and printed
//! above [`catgraph-applied`](catgraph_applied)'s presentation / normal-form
//! engine. The crate does not re-derive the term AST or the decision
//! procedures — [`PropExpr<G>`](catgraph_applied::prop::PropExpr), the
//! [`Free`](catgraph_applied::prop::Free) smart constructors, `smc_nf`, and
//! `eq_mod` all live in applied; syntax adds the layers above that engine: a
//! *textual* surface (the [`print`](mod@text::print) / [`parse`](mod@text::parse)
//! round-trip and presentation files), an *interpreter*
//! ([`eval`] — the term-action of Def 5.25, with the R-linear
//! [`SfgModel`](eval::SfgModel) as its worked example), a *Frobenius layer*
//! ([`frobenius`] — a presentation of the hypergraph theory on a colour palette
//! `Λ` as
//! [`FrobeniusOr<G>`](frobenius::FrobeniusOr), its spider calculus, the nine
//! SCFM equations per colour, and the sound
//! [`to_mat_kron`](frobenius::to_mat_kron) checker), and a *typed builder*
//! ([`traced`] — a
//! [`Traced<A, G>`](traced::Traced) pairs an executable
//! [`Arrow`](arrow_seam::Arrow) with the [`PropExpr`](catgraph_applied::prop::PropExpr)
//! term it denotes, bridged by [`Wires`](traced::Wires), so one value can be both
//! *run* and *reasoned about*).
//!
//! Anchors: Fong & Spivak 2018, *Seven Sketches in Compositionality*
//! (Def 5.25 = prop signature / `Free(G)`; Def 5.30 = a `G`-generated prop
//! expression; Def 5.33 = presentation; Thm 5.60 = Mat(R) normal form) and
//! Fong & Spivak 2019, *Hypergraph Categories* (the [`frobenius`] layer —
//! Def 2.5's SCFM, Prop 3.8, Thm 3.14).
//! Anchor-to-item map: [`docs/ANCHORS.md`](https://github.com/sustia-llc/catgraph/blob/main/catgraph-syntax/docs/ANCHORS.md).
//!
//! ## Completeness boundary
//!
//! Applied's congruence-closure decision
//! ([`Presentation::eq_mod`](catgraph_applied::prop::presentation::Presentation::eq_mod))
//! is sound but syntactically incomplete: it returns
//! `Ok(Some(true))` for a proven equality, but a `None`/`Ok(Some(false))` is
//! not a proof of inequality — it only means the congruence closure did not
//! establish the equation. Complete decisions come solely through the
//! functorial route
//! ([`Presentation::eq_mod_functorial`](catgraph_applied::prop::presentation::Presentation::eq_mod_functorial)
//! with a
//! [`CompleteFunctor`](catgraph_applied::prop::presentation::functorial::CompleteFunctor)),
//! which today means Mat(R) via
//! [`MatrixNFFunctor`](catgraph_applied::prop::presentation::functorial::MatrixNFFunctor)
//! (Thm 5.60). Nothing in `catgraph-syntax` promotes an
//! incomplete `None` into a decision.
//!
//! ## Colour palettes
//!
//! The crate is colored at every layer. The [`frobenius`] calculus carries a
//! colour on each spider variant, [`FrobeniusOr`](frobenius::FrobeniusOr) is
//! colour-transparent, and both interpreters ([`to_mat_kron`](frobenius::to_mat_kron),
//! [`to_cospan`](cospan_functor::to_cospan)) thread an interface word top-down,
//! realising Thm 3.14's colored `Cospan_Λ`. The textual surface mirrors it: a
//! palette implementing
//! [`ColorSyntax`](text::ColorSyntax) gives its letters tokens, spiders print and
//! parse as `mu@A`, and presentation files carry generator declarations
//! `g : A B -> C`. A monochromatic signature is the palette whose single letter
//! is implicit, so its terms and files are byte-for-byte the single-palette form.

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

/// Runs the README's Rust code blocks as doctests via this hidden include.
/// Non-Rust blocks in the README are fenced as `text` so they are not
/// compiled.
#[doc = include_str!("../README.md")]
#[cfg(doctest)]
mod readme {}
