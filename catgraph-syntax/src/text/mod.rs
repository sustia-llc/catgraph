//! Textual surface for free-prop terms.
//!
//! The surface is a lossless round-trip pair over the concrete syntax
//! `expr := term (';' term)*`, `term := factor (('⊗' | '*') factor)*`,
//! `factor := id(n) | braid(m,n) | GENERATOR | '(' expr ')'`:
//!
//! - the **printer** ([`print::Pretty`] / [`print::print`]) is a structural,
//!   total renderer of [`PropExpr<G>`](catgraph_applied::prop::PropExpr). It
//!   emits ASCII only (`*`); the Unicode tensor `⊗` is an **input** synonym.
//! - the **parser** ([`parse::parse`]) is a hand-rolled recursive-descent
//!   reader accepting both `*` and `⊗`, building exclusively through the
//!   [`Free`](catgraph_applied::prop::Free) smart constructors so every parse
//!   is arity-sound by construction. Nesting depth is bounded
//!   ([`parse::MAX_NESTING_DEPTH`], untrusted input).
//! - [`presentation`] renders and reads presentation files
//!   (one `lhs = rhs` equation or `g : A B -> C` declaration per line, Seven
//!   Sketches Def 5.33).
//!
//! The round-trip target is `parse(&print(e)) == Ok(e)` structurally (same
//! tree; the printer never normalises), and holds for every term whose
//! *printed* parenthesisation stays within [`parse::MAX_NESTING_DEPTH`]:
//! printing is total, but a term nested
//! deeper (e.g. a right-fold of more than `MAX_NESTING_DEPTH` compositions,
//! each right operand of which prints parenthesised) produces text the
//! depth-bounded parser rejects. Print-then-parse pipelines that persist
//! machine-built terms of that shape must restructure them (left-fold) or
//! treat the depth bound as a format limit.
//!
//! The bridge between a signature's generators and their concrete tokens is the
//! [`GeneratorSyntax`] trait; the bridge between its colour alphabet `Λ` and
//! *their* tokens is [`ColorSyntax`].

pub mod parse;
pub mod presentation;
pub mod print;

pub use parse::{MAX_NESTING_DEPTH, parse};
pub use presentation::{parse_presentation, print_presentation};
pub use print::{Pretty, print};

use catgraph_applied::prop::PropSignature;

// ---- Concrete-syntax alphabet (shared by the printer and the parser's lexer) ----

/// Sequential-composition operator `;` (the loosest binder).
pub(crate) const SEMI: char = ';';
/// Tensor operator, ASCII form `*` (the canonical **output** token).
pub(crate) const STAR: char = '*';
/// Tensor operator, Unicode form `⊗` (U+2297) — an accepted **input** synonym
/// for [`STAR`]; never emitted by the printer.
pub(crate) const TENSOR: char = '⊗';
/// Presentation-file equation separator `=` (reserved; never valid inside an
/// expression).
pub(crate) const EQUALS: char = '=';
/// Keyword-argument separator `,` (only valid inside `braid(m,n)`).
pub(crate) const COMMA: char = ',';
/// Presentation-file declaration separator `:` in `g : A B -> C` (reserved;
/// never valid inside an expression, so — exactly like [`EQUALS`] — a raw scan
/// for it finds the same positions the lexer would).
pub(crate) const COLON: char = ':';
/// Spider-colour joiner `@` in `mu@A`. Reserved in the [`GeneratorSyntax`]
/// clause-2 alphabet but deliberately **not** a lexer delimiter: `mu@A` must
/// re-lex as one atom.
pub(crate) const AT: char = '@';
/// Declaration-line arrow separating a generator's source and target colour
/// words — a reserved *whitespace-delimited atom*, meaningful only inside a
/// declaration (`-` and `>` stay ordinary atom characters everywhere else).
pub(crate) const ARROW: &str = "->";
/// Written form of the **implicit** sort `•` (U+2022): the one colour a
/// [`ColorSyntax`] palette may leave unannotated on a spider token, spelled out
/// in a declaration word where a position must be filled.
pub(crate) const UNIT_SORT: &str = "•";
/// Reserved keyword introducing an identity `id(n)`.
pub(crate) const KW_ID: &str = "id";
/// Reserved keyword introducing a braiding `braid(m,n)`.
pub(crate) const KW_BRAID: &str = "braid";

/// A prop signature whose generators have a concrete textual token.
///
/// This extends [`PropSignature`] (Seven Sketches Def 5.25 — the source/target
/// arities of each generator) with the lexical layer the textual surface needs:
/// each generator prints to a token, and a token parses back to a generator.
///
/// # Round-trip contract
///
/// Implementors **must** satisfy, for every generator `g`:
///
/// 1. `Self::parse_token(&g.print_token()) == Some(g)` — printing a generator
///    and parsing the result recovers the original generator; and
/// 2. **`print_token` returns a single lexical atom**: it must be non-empty,
///    contain no `;`, `*`, `⊗`, `=`, `,`, `:`, `@`, parenthesis, or
///    whitespace, and must equal none of the reserved words **`id`**,
///    **`braid`**, **`->`**. (`=` is reserved as the presentation-file
///    equation separator, `,` as the keyword-argument separator of
///    `braid(m,n)`, and `:` as the declaration separator of `g : A B -> C`;
///    the parser's lexer treats all three as delimiters, so a token containing
///    one cannot re-lex as one atom. `@` is **not** a delimiter — it is the
///    colour joiner of the spider token `mu@A`, reserved here so that no user
///    token can collide with a colour-annotated one. `->` is a reserved
///    whitespace-delimited atom inside declaration lines.)
///
/// Both clauses are load-bearing. The printer emits tokens **verbatim, with no
/// validation or escaping** — a token violating clause 2 produces output that
/// re-lexes as multiple tokens (or as the built-in `id(n)` / `braid(m,n)`
/// atoms), so the printed term does **not** reparse to the same tree even
/// though the token-level clause 1 may hold.
pub trait GeneratorSyntax: PropSignature {
    /// The concrete token for this generator (e.g. `"copy"`, `"add"`).
    fn print_token(&self) -> String;

    /// Parse a token back into a generator, or `None` if it names no generator
    /// of this signature.
    fn parse_token(token: &str) -> Option<Self>
    where
        Self: Sized;
}

/// A colour alphabet `Λ` whose letters have a concrete textual token.
///
/// This is [`GeneratorSyntax`]'s counterpart for a signature's
/// [`Color`](PropSignature::Color): it is what lets the textual surface spell a
/// **colored** prop — the annotated spider tokens `mu@A` / `eta@A` / `delta@A` /
/// `epsilon@A` of [`FrobeniusOr`](crate::frobenius::FrobeniusOr), and the colour
/// words of a presentation-file declaration `g : A B -> C`.
///
/// # The implicit sort
///
/// One letter of the palette may be **implicit**: it carries no annotation on a
/// spider token (bare `mu`), and where a declaration word must fill the position
/// anyway it is written `•` (U+2022). [`print_color`](ColorSyntax::print_color)
/// returns `None` for it and [`implicit`](ColorSyntax::implicit) names it.
///
/// The monochromatic palette `()` is exactly this case: a fully monochromatic
/// presentation prints no declarations and no `@` annotations at all. A
/// palette with **no** implicit letter
/// (`implicit() == None`) makes a bare `mu` a parse error — under it there is no
/// colour to default to, and inventing one would silently mistype a spider.
///
/// # Round-trip contract
///
/// Implementors **must** satisfy:
///
/// 1. `Self::parse_color(&t) == Some(c)` whenever `c.print_color() == Some(t)` —
///    printing a colour and parsing the result recovers it;
/// 2. **at most one** letter has `print_color() == None`; if one does, then
///    [`implicit`](ColorSyntax::implicit) returns exactly that letter and
///    `Self::parse_color("•")` recovers it (so the implicit sort still has a
///    written form in a declaration word), and otherwise `implicit()` is `None`;
/// 3. every `Some` token is a **single lexical atom** over
///    [`GeneratorSyntax`]'s clause-2 alphabet: non-empty, free of `;` `*` `⊗`
///    `=` `,` `:` `@` parentheses and whitespace, and equal to none of `id`,
///    `braid`, `->`.
///
/// Clause 3 is load-bearing for the same reason clause 2 is there: tokens are
/// emitted verbatim, so a violating colour makes `mu@<colour>` re-lex as several
/// atoms (or makes a declaration word unreadable) even though clause 1 holds at
/// the token level.
pub trait ColorSyntax: Sized {
    /// The concrete token for this colour, or `None` for the implicit sort.
    fn print_color(&self) -> Option<String>;

    /// Parse a token back into a colour, or `None` if it names no letter of
    /// this palette.
    fn parse_color(token: &str) -> Option<Self>;

    /// The palette's implicit sort — the colour a bare `mu` denotes — or `None`
    /// if the palette has none.
    fn implicit() -> Option<Self>;
}

/// The monochromatic palette `Λ = {•}`: its single letter is the implicit sort,
/// so colored syntax degenerates to the uncoloured surface.
impl ColorSyntax for () {
    fn print_color(&self) -> Option<String> {
        None
    }

    fn parse_color(token: &str) -> Option<Self> {
        (token == UNIT_SORT).then_some(())
    }

    fn implicit() -> Option<Self> {
        Some(())
    }
}

/// The token a colour takes **inside a declaration word**, where every position
/// must be spelled: its own token, or [`UNIT_SORT`] for the implicit sort.
pub(crate) fn word_token<C: ColorSyntax>(color: &C) -> String {
    color.print_color().unwrap_or_else(|| UNIT_SORT.to_string())
}
