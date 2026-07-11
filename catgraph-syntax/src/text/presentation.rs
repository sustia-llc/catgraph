//! Textual presentation files: one `lhs = rhs` equation per line (Seven
//! Sketches Def 5.33).
//!
//! A presentation `(G, s, t, E)` quotients `Free(G)` by a set `E` of
//! arity-matched equation pairs. This module renders and reads the equation
//! list `E` as line-oriented text, reusing the expression
//! [parser](super::parse::parse) / [printer](super::print::print) surface for
//! each side. Both directions live here (rather than split across `parse.rs`
//! and `print.rs`) so the equation-file format is documented in one place.
//!
//! ## Format
//!
//! - one equation per line, `lhs = rhs`;
//! - blank and whitespace-only lines are skipped;
//! - `=` is the reserved separator (the expression lexer never emits it inside
//!   a term), so a line must contain **exactly one** `=`; zero or more than one
//!   is a [`SyntaxError::Parse`] carrying the line's byte offset;
//! - each side is a full expression; a lexical/structural failure on a side is
//!   a `Parse` error whose offset is relative to the whole input;
//! - the arity check on each equation is
//!   [`Presentation::add_equation`](catgraph_applied::prop::presentation::Presentation::add_equation),
//!   whose mismatch failures pass through transparently as
//!   [`SyntaxError::Catgraph`].
//!
//! Round-trip: `parse_presentation(&print_presentation(p))` recovers `p`'s
//! equation list structurally (the printer never normalises).
//!
//! ## The format carries only the equation list `E`
//!
//! Def 5.33's presentation data is `(G, s, t, E)`; the file format serialises
//! `E` (the signature travels as the type parameter `G`). Runtime
//! configuration on [`Presentation`] — a non-default
//! [`NormalizeEngine`](catgraph_applied::prop::presentation::NormalizeEngine)
//! or rewrite depth — is **not** part of the format: [`parse_presentation`]
//! returns a default-configured presentation, and callers that printed a
//! specially configured one must re-apply `set_engine` / rebuild with
//! `with_depth` after parsing. Equation lists round-trip; decision-procedure
//! configuration does not.

use core::fmt::Write;

use catgraph_applied::prop::presentation::Presentation;

use crate::errors::SyntaxError;
use crate::text::GeneratorSyntax;
use crate::text::parse::parse;
use crate::text::print::Pretty;

/// Render a presentation's equation list to text, one `lhs = rhs` per line.
#[must_use]
pub fn print_presentation<G: GeneratorSyntax>(presentation: &Presentation<G>) -> String {
    let mut out = String::new();
    for (i, (lhs, rhs)) in presentation.equations().iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        write!(out, "{} = {}", Pretty(lhs), Pretty(rhs))
            .expect("invariant: fmt::Write to a String cannot fail");
    }
    out
}

/// Parse a presentation file into a [`Presentation<G>`].
///
/// # Errors
///
/// Returns [`SyntaxError::Parse`] for a line that does not contain exactly one
/// `=` or whose either side fails to parse, and [`SyntaxError::Catgraph`] for
/// an equation whose two sides have mismatched arity (surfaced from
/// [`Presentation::add_equation`](catgraph_applied::prop::presentation::Presentation::add_equation)).
pub fn parse_presentation<G: GeneratorSyntax>(input: &str) -> Result<Presentation<G>, SyntaxError> {
    let mut presentation = Presentation::<G>::new();
    let mut line_start = 0usize;
    for line in input.split_inclusive('\n') {
        if !line.trim().is_empty() {
            let mut separators = line.match_indices('=').map(|(i, _)| i);
            let eq = match (separators.next(), separators.next()) {
                (Some(i), None) => i,
                (first, _) => {
                    return Err(SyntaxError::Parse {
                        offset: line_start,
                        message: format!(
                            "presentation line must contain exactly one `=`, found {}",
                            if first.is_none() {
                                0
                            } else {
                                2 + separators.count()
                            }
                        ),
                    });
                }
            };
            let lhs = parse::<G>(&line[..eq]).map_err(|e| shift(e, line_start))?;
            let rhs = parse::<G>(&line[eq + 1..]).map_err(|e| shift(e, line_start + eq + 1))?;
            // Arity mismatch surfaces transparently as `SyntaxError::Catgraph`.
            presentation.add_equation(lhs, rhs)?;
        }
        line_start += line.len();
    }
    Ok(presentation)
}

/// Shift a [`SyntaxError::Parse`] offset from side-local to whole-input
/// coordinates; non-`Parse` errors pass through unchanged.
fn shift(err: SyntaxError, delta: usize) -> SyntaxError {
    match err {
        SyntaxError::Parse { offset, message } => SyntaxError::Parse {
            offset: offset + delta,
            message,
        },
        other => other,
    }
}
