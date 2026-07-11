//! [`GeneratorSyntax`] for the signal-flow-graph signature
//! [`SfgGenerator<R>`](catgraph_applied::sfg::SfgGenerator) (Seven Sketches
//! Def 5.45 / Eq 5.52) — the demo signature the S3 interpreter smoke test reads
//! from text.
//!
//! # Token scheme
//!
//! The four nullary-payload generators print to distinct plain tokens
//! (`copy`, `discard`, `add`, `zero`). `Scalar(r)` carries a rig element, so
//! its token is `scalar:<r>` using `R`'s [`Display`]; the argument is recovered
//! with `R`'s [`FromStr`].
//!
//! # Clause-2 compliance is conditional on `R`
//!
//! [`GeneratorSyntax`]'s clause 2 requires each token to be a single lexical
//! atom (no `;` `*` `⊗` parentheses whitespace `=`, and not the reserved
//! keywords `id` / `braid`). The plain tokens satisfy it unconditionally. The
//! `scalar:<r>` token satisfies it **iff `R`'s [`Display`] output contains no
//! grammar metacharacter** — true for the integer rigs (their `Display` is an
//! optional `-` followed by decimal digits, and `:`/`-` are not metacharacters),
//! and exercised per rig by the round-trip proptest. A rig whose `Display`
//! emitted, say, a space would break clause 2 (and the round trip) for its
//! `Scalar` values; float rigs are excluded from the round-trip guarantee for
//! this reason (the designer default recorded on issue #5).

use core::fmt::Display;
use core::str::FromStr;
use std::hash::Hash;

use catgraph_applied::rig::Rig;
use catgraph_applied::sfg::SfgGenerator;

use crate::text::GeneratorSyntax;

/// Prefix distinguishing the `Scalar(r)` token from the plain tokens.
const SCALAR_PREFIX: &str = "scalar:";

impl<R> GeneratorSyntax for SfgGenerator<R>
where
    R: Rig + core::fmt::Debug + Eq + Hash + Display + FromStr + 'static,
{
    fn print_token(&self) -> String {
        match self {
            SfgGenerator::Copy => "copy".to_string(),
            SfgGenerator::Discard => "discard".to_string(),
            SfgGenerator::Add => "add".to_string(),
            SfgGenerator::Zero => "zero".to_string(),
            SfgGenerator::Scalar(r) => format!("{SCALAR_PREFIX}{r}"),
        }
    }

    fn parse_token(token: &str) -> Option<Self> {
        match token {
            "copy" => Some(SfgGenerator::Copy),
            "discard" => Some(SfgGenerator::Discard),
            "add" => Some(SfgGenerator::Add),
            "zero" => Some(SfgGenerator::Zero),
            _ => token
                .strip_prefix(SCALAR_PREFIX)
                .and_then(|r| r.parse::<R>().ok())
                .map(SfgGenerator::Scalar),
        }
    }
}
