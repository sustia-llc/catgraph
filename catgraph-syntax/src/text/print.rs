//! Structural pretty-printer for [`PropExpr<G>`](catgraph_applied::prop::PropExpr).
//!
//! The printer is **structural and total**: it renders a term exactly as
//! written — it never normalizes, reassociates, or applies any prop equation.
//! This is deliberate. Printing the syntactic term (rather than a normal form)
//! sidesteps applied's η-scheduling normalization question
//! ([#55](https://github.com/sustia-llc/catgraph/issues/55)) entirely: a
//! `PropExpr` prints to concrete syntax that parses back (via
//! [`parse`](super::parse::parse)) to the *same* tree, independent of any
//! decision procedure.
//!
//! # Grammar and precedence
//!
//! Output is ASCII. Two binary operators, both **left-associative**:
//!
//! - composition `;` — **lowest** precedence (binds loosest);
//! - tensor `*` — **tighter** than `;`.
//!
//! Atoms are `id(n)`, `braid(m,n)`, and generator tokens. Parentheses are
//! emitted **only where the tree structure requires them** to reparse
//! identically: a lower-precedence child inside a higher-precedence operator,
//! and the right operand of a same-operator nesting (which, under
//! left-associative printing, would otherwise reassociate). The printed
//! syntax renders a `G`-generated prop expression in the sense of Seven
//! Sketches Def 5.30.
//!
//! # Depth
//!
//! Printing recurses once per tree level with no depth bound, matching the
//! structural recursion of `PropExpr` itself (`source`/`target`, the NF
//! engine). Pathologically deep terms (tens of thousands of nested nodes)
//! can exhaust the stack; this is an engine-wide property of the term
//! representation, not specific to the printer. The S2 parser bounds input
//! nesting explicitly (untrusted text).

use std::fmt;

use catgraph_applied::prop::PropExpr;

use crate::text::{COMMA, GeneratorSyntax, KW_BRAID, KW_ID, SEMI, STAR};

/// Binding tightness of a printed node. Larger binds tighter; a child is
/// parenthesized relative to its parent by comparing these.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
    /// Sequential composition `;` — loosest.
    Compose = 1,
    /// Tensor `*` — tighter than composition.
    Tensor = 2,
    /// Atoms (`id`, `braid`, generators) and any parenthesized subterm.
    Atom = 3,
}

fn prec<G: GeneratorSyntax>(expr: &PropExpr<G>) -> Prec {
    match expr {
        PropExpr::Identity(_) | PropExpr::Braid(_, _) | PropExpr::Generator(_) => Prec::Atom,
        PropExpr::Tensor(_, _) => Prec::Tensor,
        PropExpr::Compose(_, _) => Prec::Compose,
    }
}

/// Write `child` as an operand of a binary node whose precedence is `parent`.
///
/// `on_right` marks the right operand: under left-associative printing the
/// right operand of a same-precedence operator must be parenthesized (otherwise
/// `a ; (b ; c)` would reparse as `(a ; b) ; c`), whereas the left operand of
/// the same operator needs none.
fn fmt_child<G: GeneratorSyntax>(
    child: &PropExpr<G>,
    parent: Prec,
    on_right: bool,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let cp = prec(child);
    let need_parens = if on_right { cp <= parent } else { cp < parent };
    if need_parens {
        write!(f, "(")?;
        fmt_node(child, f)?;
        write!(f, ")")
    } else {
        fmt_node(child, f)
    }
}

fn fmt_node<G: GeneratorSyntax>(expr: &PropExpr<G>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match expr {
        PropExpr::Identity(n) => write!(f, "{KW_ID}({n})"),
        PropExpr::Braid(m, n) => write!(f, "{KW_BRAID}({m}{COMMA}{n})"),
        PropExpr::Generator(g) => write!(f, "{}", g.print_token()),
        PropExpr::Compose(l, r) => {
            fmt_child(l, Prec::Compose, false, f)?;
            write!(f, " {SEMI} ")?;
            fmt_child(r, Prec::Compose, true, f)
        }
        PropExpr::Tensor(l, r) => {
            fmt_child(l, Prec::Tensor, false, f)?;
            write!(f, " {STAR} ")?;
            fmt_child(r, Prec::Tensor, true, f)
        }
    }
}

/// A [`fmt::Display`] adapter that pretty-prints a borrowed
/// [`PropExpr<G>`](catgraph_applied::prop::PropExpr).
///
/// The orphan rule forbids implementing [`Display`](fmt::Display) directly on
/// applied's `PropExpr`, so this local newtype carries the impl. See the module
/// docs for the grammar and precedence rules.
///
/// ```
/// use catgraph_applied::prop::{Free, PropSignature};
/// use catgraph_syntax::text::{GeneratorSyntax, Pretty};
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
/// let e = Free::<G>::identity(2);
/// assert_eq!(Pretty(&e).to_string(), "id(2)");
/// ```
pub struct Pretty<'a, G: GeneratorSyntax>(pub &'a PropExpr<G>);

impl<G: GeneratorSyntax> fmt::Display for Pretty<'_, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_node(self.0, f)
    }
}

/// Pretty-print a term to an owned `String` (the [`Pretty`] adapter without the
/// borrow ceremony).
///
/// # Depth
///
/// Printing recurses over the term structure, so — like the interpreters — a
/// pathologically deep programmatically-built term risks a stack overflow.
/// Unlike the interpreters this returns an infallible `String` and so carries no
/// guard; a caller handling untrusted or machine-generated terms can pre-check
/// with [`term_depth`](crate::depth::term_depth) against
/// [`MAX_TERM_DEPTH`](crate::depth::MAX_TERM_DEPTH). Parsed terms are already
/// bounded by the [parser](mod@crate::text::parse)'s `MAX_NESTING_DEPTH`.
#[must_use]
pub fn print<G: GeneratorSyntax>(expr: &PropExpr<G>) -> String {
    Pretty(expr).to_string()
}
