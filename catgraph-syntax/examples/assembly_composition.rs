//! The coalition-assembly story: **the term frontend as a construction API.**
//!
//! A koalisi-shaped consumer keeps a *library* of small, named generator
//! fragments — each authored once as a one-line term string — and assembles a
//! larger structure incrementally by composing (`;`) and tensoring (`⊗`) those
//! fragments through the [`Free`] smart constructors. This is the textual
//! surface used not as a one-shot whole-program parser but as an **incremental
//! build API**: parse the parts, then wire them together in Rust.
//!
//! We assemble an "amplifier" coalition from four named fragments, then run two
//! amplifiers side by side with a single [`Free::tensor`] to show parallel
//! assembly. Everything is checked by [`eval`] under [`SfgModel<i64>`].
//!
//! Run: `cargo run -p catgraph-syntax --example assembly_composition`

use std::collections::BTreeMap;
use std::error::Error;

use catgraph_applied::prop::{Free, PropExpr};
use catgraph_applied::sfg::SfgGenerator;
use catgraph_syntax::eval::{SfgModel, eval};
use catgraph_syntax::text::{parse, print};

type Sfg = PropExpr<SfgGenerator<i64>>;

/// Parse a named fragment from the library's textual catalogue. Every fragment
/// string is authored by a human in the term language, not built by hand.
fn library() -> Result<BTreeMap<&'static str, Sfg>, Box<dyn Error>> {
    let catalogue = [
        ("split", "copy"),          // 1 → 2  fan the signal out
        ("left_gain", "scalar:2"),  // 1 → 1  amplify the left copy
        ("right_gain", "scalar:3"), // 1 → 1  amplify the right copy
        ("combine", "add"),         // 2 → 1  merge the two copies
    ];
    let mut lib = BTreeMap::new();
    for (name, src) in catalogue {
        lib.insert(name, parse::<SfgGenerator<i64>>(src)?);
    }
    Ok(lib)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== assembling a coalition from named fragments ===\n");

    let lib = library()?;
    println!("fragment library (parsed from text):");
    for (name, frag) in &lib {
        println!(
            "  {name:<11}: {:<12} ({} → {})",
            print(frag),
            frag.source(),
            frag.target()
        );
    }

    // Assemble incrementally. First tensor the two gain fragments into a 2 → 2
    // parallel stage, then bracket it with split and combine. Each `?` on
    // `compose` documents the arity seam being checked as we wire parts up.
    let gains = Free::tensor(lib["left_gain"].clone(), lib["right_gain"].clone());
    let amplifier = Free::compose(
        Free::compose(lib["split"].clone(), gains)?,
        lib["combine"].clone(),
    )?;
    println!("\nassembled amplifier : {}", print(&amplifier));
    println!(
        "                      {} → {}",
        amplifier.source(),
        amplifier.target()
    );

    let model = SfgModel::<i64>::new();
    let single = eval(&amplifier, &model, vec![7])?;
    assert_eq!(single, vec![35]); // 2·7 + 3·7
    println!("eval @ [7]          : {single:?}  (= [5·7])");

    // Parallel assembly: two amplifier coalitions running side by side is one
    // more `tensor` — the construction API composes with itself.
    let parallel = Free::tensor(amplifier.clone(), amplifier);
    println!("\nparallel assembly   : {}", print(&parallel));
    let both = eval(&parallel, &model, vec![1, 2])?;
    assert_eq!(both, vec![5, 10]);
    println!("eval @ [1, 2]       : {both:?}  (two coalitions, independent inputs)");

    println!("\nThe fragments were authored as text; the wiring is ordinary Rust.");
    Ok(())
}
