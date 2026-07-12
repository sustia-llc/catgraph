//! The general `catgraph-applied` consumer story: **a term language instead of
//! combinator plumbing.**
//!
//! We build one and the same non-trivial signal-flow morphism —
//! `copy ; (scalar(2) ⊗ scalar(3)) ; add : 1 → 1` (an amplify-by-5 circuit) —
//! two ways:
//!
//! 1. through the raw [`Free`] smart-constructor API (the combinator plumbing a
//!    consumer writes today), and
//! 2. by [`parse`]-ing the equivalent one-line term string through the S2
//!    textual surface.
//!
//! Because [`PropExpr`] derives `Eq` and that equality is **structural** (same
//! syntax tree, no normalization), the two constructions are literally the same
//! value — `assert_eq!(raw, parsed)` holds. We then run both through the S3
//! interpreter under [`SfgModel<i64>`] and confirm identical outputs. The pitch:
//! the term string is the same morphism, read the way a human writes it.
//!
//! Run: `cargo run -p catgraph-syntax --example programmatic_construction`

use std::error::Error;

use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::eval::{SfgModel, eval};
use catgraph_syntax::text::{parse, print};

/// Leaf shim: wrap a bare SFG generator in a `PropExpr` leaf (the house idiom
/// from `tests/common`).
fn g(s: SfgGenerator<i64>) -> PropExpr<SfgGenerator<i64>> {
    Free::generator(s)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== term language vs combinator plumbing ===\n");

    // (1) Raw combinator plumbing. Arity flow:
    //   1 →[copy]→ 2 →[scalar(2) ⊗ scalar(3)]→ 2 →[add]→ 1
    // `compose` is the only fallible constructor (it arity-checks the seam);
    // both composes below are arity-valid by construction, so `?` never fires.
    let scaled = Free::tensor(g(SfgGenerator::Scalar(2)), g(SfgGenerator::Scalar(3)));
    let raw = Free::compose(
        Free::compose(g(SfgGenerator::Copy), scaled)?,
        g(SfgGenerator::Add),
    )?;
    println!("raw  (Free::* plumbing) : {}", print(&raw));

    // (2) The same morphism, read from one line of text. `;` binds loosest and
    // `*` tighter, so `copy ; scalar:2 * scalar:3 ; add` groups as
    // `((copy ; (scalar:2 * scalar:3)) ; add)` — exactly the raw tree above.
    let source = "copy ; scalar:2 * scalar:3 ; add";
    let parsed = parse::<SfgGenerator<i64>>(source)?;
    println!("parsed(\"{source}\") : {}", print(&parsed));

    // Structural equality: PropExpr's derived `Eq` compares syntax trees, and the
    // printer never normalizes, so the two constructions are the *same value*.
    assert_eq!(raw, parsed, "raw plumbing and parsed text are one morphism");
    println!("\nstructurally equal: assert_eq!(raw, parsed) holds\n");

    // Both evaluate identically under the S3 SFG interpreter. copy duplicates,
    // scalar(2)/scalar(3) scale the two copies, add sums them: x ↦ 5x.
    let model = SfgModel::<i64>::new();
    for x in [6_i64, -2, 0] {
        let out_raw = eval(&raw, &model, vec![x])?;
        let out_parsed = eval(&parsed, &model, vec![x])?;
        assert_eq!(out_raw, out_parsed);
        println!("eval @ [{x:>3}] : raw = {out_raw:?}  parsed = {out_parsed:?}  (= [5·{x}])");
    }

    println!("\nsource/target: {} → {}", raw.source(), raw.target());
    println!("\nThe term string IS the pitch: one line replaces the nested plumbing.");
    Ok(())
}
