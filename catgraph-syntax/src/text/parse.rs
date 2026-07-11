//! Recursive-descent parser for the textual free-prop surface (Phase S2).
//!
//! [`parse`] is the inverse of [`print`](super::print::print): it reads the
//! concrete syntax `expr := term (';' term)*`,
//! `term := factor (('⊗' | '*') factor)*`,
//! `factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'` into a
//! [`PropExpr<G>`](catgraph_applied::prop::PropExpr). Composition `;` is the
//! loosest operator, tensor binds tighter, both are left-associative, and the
//! Unicode tensor `⊗` (U+2297) is accepted everywhere the ASCII `*` is —
//! matching the input alphabet the printer's grammar documents. The parser
//! builds **exclusively** through the [`Free`]
//! smart constructors, so every accepted term is arity-sound by construction
//! (Seven Sketches Def 5.30).
//!
//! # Lexical structure
//!
//! An *atom* is a maximal run of characters excluding the operator/grouping
//! set `;` `*` `⊗` `(` `)` `=` and whitespace. `=` is reserved so that a
//! generator token can never eat the equation separator of a presentation file
//! (`super::presentation`); the [`GeneratorSyntax`] clause-2 alphabet is
//! narrowed accordingly (a token must additionally avoid `=`). The keywords
//! `id` and `braid` are atoms whose *following* parenthesised argument list is
//! mandatory: `id(n)` and `braid(m,n)` with decimal `usize` arguments. A bare
//! `id` or `braid` with no `(` is a parse error — they are reserved and never
//! name a generator. Any other atom is handed to
//! [`G::parse_token`](GeneratorSyntax::parse_token); a `None` there is a parse
//! error naming the offending token.
//!
//! # Failure split
//!
//! Lexical and structural failures surface as
//! [`SyntaxError::Parse`] carrying the byte
//! offset of the offending input. Arity failures raised by
//! [`Free::compose`](catgraph_applied::prop::Free::compose) pass through
//! transparently as [`SyntaxError::Catgraph`].
//!
//! # Bounded nesting depth
//!
//! Parenthesised subexpressions recurse; untrusted input could otherwise drive
//! the parser to a stack overflow. Recursion into a `'(' expr ')'` body is
//! guarded by an explicit depth counter bounded at [`MAX_NESTING_DEPTH`];
//! exceeding it is a [`SyntaxError::Parse`], never a panic. Flat operator
//! chains (`a ; b ; c`, `a * b * c`) are parsed iteratively and are unaffected
//! by the bound.

use std::marker::PhantomData;

use catgraph_applied::prop::{Free, PropExpr};

use crate::errors::SyntaxError;
use crate::text::GeneratorSyntax;

/// Maximum parenthesis-nesting depth [`parse`] accepts before rejecting the
/// input with a [`SyntaxError::Parse`].
///
/// The bound caps parser recursion (one level per open parenthesis) so that
/// adversarial input such as `"(".repeat(1_000_000)` fails cleanly instead of
/// overflowing the stack.
pub const MAX_NESTING_DEPTH: usize = 256;

/// A lexical token. Operators and grouping characters are single tokens; every
/// other maximal non-whitespace run is an [`Tok::Atom`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    /// `;` — sequential composition.
    Semi,
    /// `*` or `⊗` — tensor.
    Star,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `=` — the presentation equation separator (never valid inside an
    /// expression; reserved so a generator token cannot absorb it).
    Equals,
    /// A maximal non-operator, non-whitespace run.
    Atom(String),
}

/// A token paired with the byte offset at which it starts.
#[derive(Debug, Clone)]
struct Lexeme {
    tok: Tok,
    offset: usize,
}

/// Tokenise `input`, tracking the byte offset of each token for diagnostics.
fn lex(input: &str) -> Vec<Lexeme> {
    let mut out = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some(&(idx, c)) = chars.peek() {
        let single = match c {
            c if c.is_whitespace() => {
                chars.next();
                continue;
            }
            ';' => Some(Tok::Semi),
            '*' | '⊗' => Some(Tok::Star),
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            '=' => Some(Tok::Equals),
            _ => None,
        };
        if let Some(tok) = single {
            out.push(Lexeme { tok, offset: idx });
            chars.next();
            continue;
        }
        // Maximal atom: consume until the next operator, grouping char, or
        // whitespace.
        let start = idx;
        let mut end = idx;
        while let Some(&(i, c)) = chars.peek() {
            if c.is_whitespace() || matches!(c, ';' | '*' | '⊗' | '(' | ')' | '=') {
                break;
            }
            end = i + c.len_utf8();
            chars.next();
        }
        out.push(Lexeme {
            tok: Tok::Atom(input[start..end].to_string()),
            offset: start,
        });
    }
    out
}

/// Parse `input` into a [`PropExpr<G>`], consuming the whole string.
///
/// # Errors
///
/// Returns [`SyntaxError::Parse`] for lexical or structural failures (with the
/// byte offset of the offending input, including trailing garbage and
/// over-deep nesting) and [`SyntaxError::Catgraph`] for arity failures raised
/// by the underlying [`Free::compose`](catgraph_applied::prop::Free::compose).
///
/// ```
/// use catgraph_applied::prop::{Free, PropSignature};
/// use catgraph_syntax::text::{parse, print, GeneratorSyntax};
///
/// #[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// struct G;
/// impl PropSignature for G {
///     fn source(&self) -> usize { 1 }
///     fn target(&self) -> usize { 1 }
/// }
/// impl GeneratorSyntax for G {
///     fn print_token(&self) -> String { "g".to_string() }
///     fn parse_token(t: &str) -> Option<Self> { (t == "g").then_some(G) }
/// }
///
/// let e = Free::compose(Free::generator(G), Free::<G>::identity(1)).unwrap();
/// assert_eq!(parse::<G>(&print(&e)), Ok(e));
/// ```
pub fn parse<G: GeneratorSyntax>(input: &str) -> Result<PropExpr<G>, SyntaxError> {
    let lexemes = lex(input);
    let mut parser = Parser::<G>::new(&lexemes, input.len());
    if parser.peek().is_none() {
        return Err(SyntaxError::Parse {
            offset: 0,
            message: "empty input; expected an expression".to_string(),
        });
    }
    let expr = parser.parse_expr(0)?;
    if let Some(trailing) = lexemes.get(parser.pos) {
        return Err(SyntaxError::Parse {
            offset: trailing.offset,
            message: "unexpected trailing input".to_string(),
        });
    }
    Ok(expr)
}

/// Cursor over a lexeme stream. `end_offset` is the byte length of the source,
/// reported for end-of-input diagnostics.
struct Parser<'a, G: GeneratorSyntax> {
    lexemes: &'a [Lexeme],
    pos: usize,
    end_offset: usize,
    _marker: PhantomData<G>,
}

impl<'a, G: GeneratorSyntax> Parser<'a, G> {
    fn new(lexemes: &'a [Lexeme], end_offset: usize) -> Self {
        Self {
            lexemes,
            pos: 0,
            end_offset,
            _marker: PhantomData,
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.lexemes.get(self.pos).map(|l| &l.tok)
    }

    /// Byte offset of the current lexeme, or the end offset past the last one.
    fn offset(&self) -> usize {
        self.lexemes
            .get(self.pos)
            .map_or(self.end_offset, |l| l.offset)
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn parse_err<T>(&self, offset: usize, message: impl Into<String>) -> Result<T, SyntaxError> {
        Err(SyntaxError::Parse {
            offset,
            message: message.into(),
        })
    }

    /// `expr := term (';' term)*` — left-associative composition.
    fn parse_expr(&mut self, depth: usize) -> Result<PropExpr<G>, SyntaxError> {
        let mut acc = self.parse_term(depth)?;
        while matches!(self.peek(), Some(Tok::Semi)) {
            self.bump();
            let rhs = self.parse_term(depth)?;
            // Arity failures surface transparently as `SyntaxError::Catgraph`.
            acc = Free::compose(acc, rhs)?;
        }
        Ok(acc)
    }

    /// `term := factor (('⊗' | '*') factor)*` — left-associative tensor.
    fn parse_term(&mut self, depth: usize) -> Result<PropExpr<G>, SyntaxError> {
        let mut acc = self.parse_factor(depth)?;
        while matches!(self.peek(), Some(Tok::Star)) {
            self.bump();
            let rhs = self.parse_factor(depth)?;
            acc = Free::tensor(acc, rhs);
        }
        Ok(acc)
    }

    /// `factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'`.
    fn parse_factor(&mut self, depth: usize) -> Result<PropExpr<G>, SyntaxError> {
        let (tok, offset) = match self.lexemes.get(self.pos) {
            Some(l) => (l.tok.clone(), l.offset),
            None => {
                return self.parse_err(
                    self.end_offset,
                    "unexpected end of input; expected a factor",
                );
            }
        };
        match tok {
            Tok::LParen => {
                if depth >= MAX_NESTING_DEPTH {
                    return self.parse_err(
                        offset,
                        format!("nesting deeper than MAX_NESTING_DEPTH ({MAX_NESTING_DEPTH})"),
                    );
                }
                self.bump();
                let inner = self.parse_expr(depth + 1)?;
                if matches!(self.peek(), Some(Tok::RParen)) {
                    self.bump();
                    Ok(inner)
                } else {
                    self.parse_err(self.offset(), "expected `)`")
                }
            }
            Tok::Atom(atom) => {
                self.bump();
                self.parse_atom(&atom, offset)
            }
            _ => self.parse_err(
                offset,
                "expected a factor (`id(n)`, `braid(m,n)`, a generator token, or `(`)",
            ),
        }
    }

    /// Resolve an atom to an `id`/`braid` keyword or a generator token.
    fn parse_atom(&mut self, atom: &str, offset: usize) -> Result<PropExpr<G>, SyntaxError> {
        match atom {
            "id" => {
                let arg = self.read_paren_args("id")?;
                let n = parse_usize(&arg.text, arg.offset)?;
                Ok(Free::identity(n))
            }
            "braid" => {
                let arg = self.read_paren_args("braid")?;
                let (m, n) = match arg.text.split_once(',') {
                    Some((a, b)) => (
                        parse_usize(a.trim(), arg.offset)?,
                        parse_usize(b.trim(), arg.offset)?,
                    ),
                    None => {
                        return self.parse_err(
                            arg.offset,
                            "expected `braid(m,n)` with a comma between the two arities",
                        );
                    }
                };
                Ok(Free::braid(m, n))
            }
            _ => match G::parse_token(atom) {
                Some(g) => Ok(Free::generator(g)),
                None => self.parse_err(offset, format!("unknown generator token `{atom}`")),
            },
        }
    }

    /// Consume `'(' atom* ')'` after a keyword, returning the joined argument
    /// text and its start offset. Concatenating the atoms tolerates internal
    /// whitespace (`id( 2 )`, `braid(1, 2)`) while still building the exact
    /// `m,n` string the argument grammar expects.
    fn read_paren_args(&mut self, kw: &str) -> Result<ParenArgs, SyntaxError> {
        if matches!(self.peek(), Some(Tok::LParen)) {
            self.bump();
        } else {
            return self.parse_err(
                self.offset(),
                format!("`{kw}` requires a parenthesised argument list `{kw}(...)`"),
            );
        }
        let start = self.offset();
        let mut text = String::new();
        loop {
            match self.lexemes.get(self.pos).map(|l| l.tok.clone()) {
                Some(Tok::Atom(s)) => {
                    text.push_str(&s);
                    self.bump();
                }
                Some(Tok::RParen) => {
                    self.bump();
                    break;
                }
                _ => {
                    return self.parse_err(
                        self.offset(),
                        format!("expected an argument or `)` in `{kw}(...)`"),
                    );
                }
            }
        }
        Ok(ParenArgs {
            text,
            offset: start,
        })
    }
}

/// Joined text of a keyword's parenthesised argument list plus its start offset.
struct ParenArgs {
    text: String,
    offset: usize,
}

/// Parse a decimal `usize`, rejecting overflow and malformed input as a
/// [`SyntaxError::Parse`] rather than panicking.
fn parse_usize(s: &str, offset: usize) -> Result<usize, SyntaxError> {
    s.parse::<usize>().map_err(|_| SyntaxError::Parse {
        offset,
        message: format!("expected a decimal usize, found `{s}`"),
    })
}
