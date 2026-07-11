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
//! # Round-trip compliance is conditional on `R`
//!
//! [`GeneratorSyntax`]'s clause 2 requires each token to be a single lexical
//! atom (no `;` `*` `⊗` parentheses whitespace `=` `,`, and not the reserved
//! keywords `id` / `braid`). The plain tokens satisfy it unconditionally. The
//! `scalar:<r>` token satisfies the contract **iff both**:
//!
//! 1. `R`'s [`Display`] output contains no grammar metacharacter (clause 2) —
//!    true for the integer rigs, whose `Display` is an optional `-` followed
//!    by decimal digits (`:`/`-` are not metacharacters); and
//! 2. `R`'s [`FromStr`] parses that `Display` output back to the same value
//!    and the output is nonempty (clause 1) — a rig whose `Display` rendered
//!    some value as `""` would print the bare token `scalar:`, which fails to
//!    reparse even though it contains no metacharacter.
//!
//! Neither condition is checked at print time; they are **exercised for
//! `i64` by the round-trip proptests** in this crate's test suite. Any other
//! rig instantiated with this impl needs its own round-trip test. A rig whose
//! `Display` emitted, say, a space would break clause 2 for its `Scalar`
//! values; float rigs cannot inhabit [`SfgGenerator`] at all (no `Eq`/`Hash`)
//! and are excluded from the round-trip guarantee regardless (the designer
//! default recorded on issue #5).

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
