//! Textual presentation files: one `lhs = rhs` equation or `g : A B -> C`
//! generator declaration per line (Seven Sketches Def 5.33).
//!
//! A presentation `(G, s, t, E)` quotients `Free(G)` by a set `E` of
//! boundary-matched equation pairs. This module renders and reads the equation
//! list `E` as line-oriented text, reusing the expression
//! [parser](super::parse::parse) / [printer](super::print::print) surface for
//! each side.
//!
//! ## Format
//!
//! - blank and whitespace-only lines are skipped;
//! - a line containing `=` is an **equation** `lhs = rhs`. `=` is the reserved
//!   separator (the expression lexer never emits it inside a term), so such a
//!   line must contain **exactly one** `=`; more than one is a
//!   [`SyntaxError::Parse`] carrying the line's byte offset;
//! - each side is a full expression; a lexical/structural failure on a side is
//!   a `Parse` error whose offset is relative to the whole input;
//! - the check on each equation is
//!   [`Presentation::add_equation`](catgraph_applied::prop::presentation::Presentation::add_equation),
//!   whose failures — boundary-**word** disagreement, not just arity — pass
//!   through transparently as [`SyntaxError::Catgraph`];
//! - a line with no `=` and a `:` is a **declaration**
//!   `TOKEN : COLOR* -> COLOR*` (whitespace-separated, either colour list
//!   possibly empty, exactly one `:` and exactly one `->`). `:` is reserved the
//!   same way `=` is, so the scan for it cannot land inside a token;
//! - anything else is the same "exactly one `=`" error as before.
//!
//! Declarations may appear anywhere in the file; conventionally — and in
//! everything [`print_presentation`] emits — they come first.
//!
//! Round-trip: `parse_presentation(&print_presentation(p))` recovers `p`'s
//! equation list structurally (the printer never normalises).
//!
//! ## Declarations *check* the signature, they do not build it
//!
//! Def 5.33's presentation data is `(G, s, t, E)`; the file format serialises
//! `E`, and the signature still travels as the type parameter `G`. A
//! declaration is therefore a **drift check**, not a construction: its token
//! must name a generator of `G`, its colour tokens must name letters of `Λ`,
//! and the two declared words must *equal* that generator's own
//! `source_word()` / `target_word()`. A file that disagrees with the `G` it is
//! read at is rejected rather than silently reinterpreted.
//!
//! [`print_presentation`] emits one declaration per **distinct generator
//! occurring in the equation list**, in [`Ord`] order — not one per signature
//! generator, because [`PropSignature`]
//! has no enumeration surface (`G` is a Rust type, not a listable set), so the
//! equation list is the only place the printer can learn which generators the
//! file mentions. Declarations are emitted **iff** at least one port colour of
//! at least one such generator has a token of its own
//! ([`ColorSyntax::print_color`] is `Some`); a monochromatic presentation
//! therefore prints no declarations at all, and a declaration-free file stays
//! valid input.
//!
//! ## Runtime configuration is not part of the format
//!
//! Configuration on [`Presentation`] — a non-default
//! [`NormalizeEngine`](catgraph_applied::prop::presentation::NormalizeEngine)
//! or rewrite depth — is **not** part of the format: [`parse_presentation`]
//! returns a default-configured presentation, and callers that printed a
//! specially configured one must re-apply `set_engine` / rebuild with
//! `with_depth` after parsing. Equation lists round-trip; decision-procedure
//! configuration does not.

use core::fmt::Write;
use std::collections::BTreeSet;

use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::{PropExpr, PropSignature};

use crate::errors::SyntaxError;
use crate::text::parse::parse;
use crate::text::print::Pretty;
use crate::text::{ARROW, COLON, ColorSyntax, EQUALS, GeneratorSyntax, word_token};

/// Render a presentation to text: the colour declarations (if any), then the
/// equation list, one `lhs = rhs` per line.
#[must_use]
pub fn print_presentation<G: GeneratorSyntax>(presentation: &Presentation<G>) -> String
where
    G::Color: ColorSyntax,
{
    let mut out = String::new();
    // Every line this function writes is non-empty, so "nothing written yet" is
    // exactly "this is the first line".
    for generator in declared_generators(presentation) {
        if !out.is_empty() {
            out.push('\n');
        }
        print_declaration(&generator, &mut out);
    }
    for (lhs, rhs) in presentation.equations() {
        if !out.is_empty() {
            out.push('\n');
        }
        write!(out, "{} {EQUALS} {}", Pretty(lhs), Pretty(rhs))
            .expect("invariant: fmt::Write to a String cannot fail");
    }
    out
}

/// The generators to declare: the distinct ones occurring in the equation list,
/// in [`Ord`] order — or none at all when the signature has no colour to show
/// (see the module docs for why occurrence, not enumeration, is the criterion).
fn declared_generators<G: GeneratorSyntax>(presentation: &Presentation<G>) -> BTreeSet<G>
where
    G::Color: ColorSyntax,
{
    let mut seen = BTreeSet::new();
    for (lhs, rhs) in presentation.equations() {
        collect_generators(lhs, &mut seen);
        collect_generators(rhs, &mut seen);
    }
    if seen.iter().any(has_visible_color) {
        seen
    } else {
        BTreeSet::new()
    }
}

/// Collect the generator leaves of `expr`. Recurses once per level, exactly as
/// the [printer](Pretty) does over the same term.
fn collect_generators<G: PropSignature>(expr: &PropExpr<G>, out: &mut BTreeSet<G>) {
    match expr {
        PropExpr::Generator(g) => {
            out.insert(g.clone());
        }
        PropExpr::Compose(f, g) | PropExpr::Tensor(f, g) => {
            collect_generators(f, out);
            collect_generators(g, out);
        }
        PropExpr::Identity(_) | PropExpr::Braid(_, _) => {}
    }
}

/// Does any port of `g` carry a colour with a token of its own?
fn has_visible_color<G: PropSignature>(g: &G) -> bool
where
    G::Color: ColorSyntax,
{
    let source = g.source_word();
    let target = g.target_word();
    source
        .iter()
        .chain(target.iter())
        .any(|c| c.print_color().is_some())
}

/// Write one declaration line `token : A B -> C`.
fn print_declaration<G: GeneratorSyntax>(g: &G, out: &mut String)
where
    G::Color: ColorSyntax,
{
    out.push_str(&g.print_token());
    out.push(' ');
    out.push(COLON);
    for color in g.source_word().iter() {
        out.push(' ');
        out.push_str(&word_token(color));
    }
    out.push(' ');
    out.push_str(ARROW);
    for color in g.target_word().iter() {
        out.push(' ');
        out.push_str(&word_token(color));
    }
}

/// Parse a presentation file into a [`Presentation<G>`].
///
/// # Errors
///
/// Returns [`SyntaxError::Parse`] for
///
/// - a line that contains neither exactly one `=` nor (with no `=` at all) a
///   `:`, or whose either equation side fails to parse;
/// - a malformed declaration — not exactly one `:`, not exactly one `->`, or
///   more than one atom before the `:`;
/// - a declaration naming a token no generator of `G` parses, or a colour token
///   no letter of `Λ` parses;
/// - a declaration whose source or target word **disagrees** with the named
///   generator's own; the message names the generator and both words;
///
/// and [`SyntaxError::Catgraph`] for an equation whose two sides have
/// mismatched boundary *words* (surfaced from
/// [`Presentation::add_equation`](catgraph_applied::prop::presentation::Presentation::add_equation)).
pub fn parse_presentation<G: GeneratorSyntax>(input: &str) -> Result<Presentation<G>, SyntaxError>
where
    G::Color: ColorSyntax,
{
    let mut presentation = Presentation::<G>::new();
    let mut line_start = 0usize;
    for line in input.split_inclusive('\n') {
        if !line.trim().is_empty() {
            // EQUALS is a lexer delimiter, so this raw scan finds exactly the Tok::Equals positions the lexer would.
            let mut separators = line.match_indices(EQUALS).map(|(i, _)| i);
            match (separators.next(), separators.next()) {
                (Some(eq), None) => {
                    let lhs = parse::<G>(&line[..eq]).map_err(|e| shift(e, line_start))?;
                    let rhs =
                        parse::<G>(&line[eq + 1..]).map_err(|e| shift(e, line_start + eq + 1))?;
                    // A boundary-word mismatch surfaces transparently as
                    // `SyntaxError::Catgraph`.
                    presentation.add_equation(lhs, rhs)?;
                }
                // No `=` at all, but a declaration separator: a generator
                // declaration, which is checked against `G` and adds nothing.
                (None, _) if line.contains(COLON) => check_declaration::<G>(line, line_start)?,
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
            }
        }
        line_start += line.len();
    }
    Ok(presentation)
}

/// Split `s` into whitespace-delimited atoms, each paired with its byte offset
/// in the whole input (`base` is `s`'s own offset there).
fn atoms(s: &str, base: usize) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        if c.is_whitespace() {
            if let Some(b) = start.take() {
                out.push((base + b, &s[b..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(b) = start {
        out.push((base + b, &s[b..]));
    }
    out
}

/// Check one declaration line `TOKEN : COLOR* -> COLOR*` against `G`.
///
/// The line contributes nothing to the presentation: it asserts what `G`
/// already says, so a disagreement is the file drifting from the signature it
/// is being read at.
fn check_declaration<G: GeneratorSyntax>(line: &str, line_start: usize) -> Result<(), SyntaxError>
where
    G::Color: ColorSyntax,
{
    // COLON is a lexer delimiter, so this raw scan agrees with the lexer's own token boundaries.
    let mut separators = line.match_indices(COLON).map(|(i, _)| i);
    let colon = separators.next().ok_or_else(|| SyntaxError::Parse {
        offset: line_start,
        message: format!("declaration line must contain exactly one `{COLON}`, found 0"),
    })?;
    if let Some(extra) = separators.next() {
        return Err(SyntaxError::Parse {
            offset: line_start + extra,
            message: format!(
                "declaration line must contain exactly one `{COLON}`, found {}",
                2 + separators.count()
            ),
        });
    }

    let head = atoms(&line[..colon], line_start);
    let (token_offset, token) = match head.as_slice() {
        [single] => *single,
        _ => {
            return Err(SyntaxError::Parse {
                offset: line_start,
                message: format!(
                    "declaration must name exactly one generator token before `{COLON}`, found {}",
                    head.len()
                ),
            });
        }
    };
    let Some(generator) = G::parse_token(token) else {
        return Err(SyntaxError::Parse {
            offset: token_offset,
            message: format!("unknown generator token `{token}` in declaration"),
        });
    };

    let tail = atoms(&line[colon + 1..], line_start + colon + 1);
    let mut arrows = tail
        .iter()
        .enumerate()
        .filter(|(_, (_, atom))| *atom == ARROW)
        .map(|(i, _)| i);
    let arrow = match (arrows.next(), arrows.next()) {
        (Some(i), None) => i,
        (first, second) => {
            let found = if first.is_none() {
                0
            } else {
                2 + arrows.count()
            };
            return Err(SyntaxError::Parse {
                offset: second.map_or(line_start + colon, |i| tail[i].0),
                message: format!("declaration must contain exactly one `{ARROW}`, found {found}"),
            });
        }
    };

    check_word(
        &generator,
        &parse_word::<G::Color>(&tail[..arrow])?,
        &generator.source_word(),
        "source",
        token,
        token_offset,
    )?;
    check_word(
        &generator,
        &parse_word::<G::Color>(&tail[arrow + 1..])?,
        &generator.target_word(),
        "target",
        token,
        token_offset,
    )
}

/// Read a whitespace-separated colour word, reporting an unknown letter at its
/// own byte offset.
fn parse_word<C: ColorSyntax>(word: &[(usize, &str)]) -> Result<Vec<C>, SyntaxError> {
    word.iter()
        .map(|&(offset, atom)| {
            C::parse_color(atom).ok_or_else(|| SyntaxError::Parse {
                offset,
                message: format!("unknown colour token `{atom}` in declaration"),
            })
        })
        .collect()
}

/// Assert a declared word equals the generator's own, naming both on failure.
fn check_word<G: PropSignature>(
    generator: &G,
    declared: &[G::Color],
    actual: &[G::Color],
    side: &str,
    token: &str,
    offset: usize,
) -> Result<(), SyntaxError> {
    if declared == actual {
        return Ok(());
    }
    Err(SyntaxError::Parse {
        offset,
        message: format!(
            "declaration of `{token}` gives {side} word {declared:?}, but generator \
             {generator:?} has {side} word {actual:?}"
        ),
    })
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
