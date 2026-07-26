//! The BTV 2021 **semantic category** `L̂ := [0,1]^L`, end-to-end (Def 8).
//!
//! Bradley–Terilla–Vlassopoulos 2021 (*An enriched category theory of
//! language*, arXiv:2106.07890) Def 8 takes the syntax category `L` — texts as
//! objects, `L(x, y) = π(y | x)` as the `[0,1]`-valued hom (Def 4) — and forms
//! the category of enriched copresheaves `L̂ = [0,1]^L`. Objects of `L̂` are
//! meanings; its hom is the asymmetric end of Lemma 2, Eq 11.
//!
//! **`L̂` is demonstrated here implicitly.** There is no reified `SemanticCategory`
//! wrapper type in this crate, and by design (the 2026-07-25 #53 decision) there
//! will not be: the category structure is exhibited by the surfaces that already
//! ship —
//!
//! | `L̂` structure                | shipped surface                            |
//! |------------------------------|--------------------------------------------|
//! | objects (meanings)           | [`Copresheaf`]                              |
//! | hom `Ĉ(f, g) ∈ [0,1]`        | [`semantic_hom`] (Lemma 2, Eq 11)           |
//! | Yoneda embedding `L → L̂`     | `LmCategory::yoneda_all` (Cor 2)            |
//! | metric view `d̂ = −ln Ĉ`      | [`semantic_distance`] (§5)                  |
//!
//! The **categorical content** of this example is step 5: the enriched
//! composition law `Ĉ(f, g) ⊗ Ĉ(g, h) → Ĉ(f, h)` — in the monoidal category
//! `([0,1], ×, 1)` that is the inequality `Ĉ(f,g) · Ĉ(g,h) ≤ Ĉ(f,h)` — checked
//! over every one of the `7³ = 343` triples of the fixture. It is *demonstrated,
//! not claimed*.
//!
//! ## The fixture
//!
//! A corpus of eight two-word texts built from BTV's own running example,
//! "red firetruck" (§1), plus a second colour so that the two colour-branches
//! separate:
//!
//! ```text
//!   red firetruck ×3   red rose ×1      blue sky ×2   blue rose ×2
//! ```
//!
//! `LmCategory::from_traces` turns this into the corpus-MLE prefix trie: states
//! are observed prefixes (space-joined, ε = `""`), and `π(p·t | p) = N(p·t)/N(p)`
//! makes the chain rule Eq 8 hold exactly. These are plain texts — BTV 2021 has
//! no production-rule taxonomy, so nothing here is a terminal/non-terminal split
//! or a grammar.

use catgraph_magnitude::lm_category::LmCategory;
use catgraph_magnitude::{Copresheaf, semantic_distance, semantic_hom};

const TOL: f64 = 1e-12;

/// Build the eight-trace corpus of the module docs and its prefix-trie category.
///
/// The resulting `L` has 7 objects in lexicographic order (ε first):
///   0 `""` (ε)  1 `"blue"`  2 `"blue rose"`  3 `"blue sky"`
///   4 `"red"`   5 `"red firetruck"`  6 `"red rose"`
fn build_corpus_category() -> LmCategory {
    let corpus: Vec<Vec<&str>> = vec![
        vec!["red", "firetruck"],
        vec!["red", "firetruck"],
        vec!["red", "firetruck"],
        vec!["red", "rose"],
        vec!["blue", "sky"],
        vec!["blue", "sky"],
        vec!["blue", "rose"],
        vec!["blue", "rose"],
    ];
    LmCategory::from_traces(&corpus).expect("corpus is non-empty with whitespace-free tokens")
}

/// Render a state name for printing — ε is the empty string.
fn show(name: &str) -> &str {
    if name.is_empty() { "ε" } else { name }
}

/// Eq 12 reachability on **names**: `y` is an extension of `x` iff `x` is a
/// prefix of `y` in the space-joined sense (`y == x`, `x` is ε, or `y` starts
/// with `x + " "`). The copresheaf support must agree with this.
fn is_extension_of(x: &str, y: &str) -> bool {
    x == y || x.is_empty() || y.strip_prefix(x).is_some_and(|rest| rest.starts_with(' '))
}

fn main() {
    // -----------------------------------------------------------------------
    // 1. Corpus → the syntax category L (Def 4 hom, corpus-MLE π).
    // -----------------------------------------------------------------------
    let m = build_corpus_category();
    let names = m.objects().to_vec();

    println!("=== BTV 2021 Def 8: the semantic category L̂ = [0,1]^L ===\n");
    println!("1. Syntax category L (Def 4), corpus-MLE over 8 traces:\n");
    for (i, name) in names.iter().enumerate() {
        println!("     {i}  {:?}", show(name));
    }
    println!(
        "\n   π(red|ε) = π(blue|ε) = 0.5;  π(red firetruck|red) = 0.75,\n   \
         π(red rose|red) = 0.25;  π(blue sky|blue) = π(blue rose|blue) = 0.5\n"
    );

    let expected_names = [
        "",
        "blue",
        "blue rose",
        "blue sky",
        "red",
        "red firetruck",
        "red rose",
    ];
    assert_eq!(
        m.objects(),
        &expected_names[..],
        "unexpected prefix-trie object list"
    );

    // -----------------------------------------------------------------------
    // 2. Yoneda embedding L → L̂ (Cor 2): every meaning, in one pass.
    // -----------------------------------------------------------------------
    let meanings: Vec<Copresheaf> = m
        .yoneda_all()
        .expect("prefix-trie category yields a well-formed enriched space");

    println!("2. Yoneda embedding y: L → L̂, x ↦ h^x = L(x, −)  (Cor 2)");
    println!("   {} objects ↦ {} meanings\n", names.len(), meanings.len());

    assert_eq!(meanings.len(), 7, "one copresheaf per object");
    for (i, f) in meanings.iter().enumerate() {
        assert_eq!(f.base(), i, "yoneda_all rows are in object order");
    }

    // -----------------------------------------------------------------------
    // 3. Eq 12 — the support of h^x is exactly x's extensions.
    // -----------------------------------------------------------------------
    println!("3. Support of each meaning (Eq 12: h^x(y) > 0 ⟺ x is a prefix of y)\n");
    for (x, f) in meanings.iter().enumerate() {
        let support: Vec<&str> = f.support().map(|y| show(&names[y])).collect();
        let label = format!("h^{:?}", show(&names[x]));
        println!("     {label:<20} → {support:?}");
    }

    for (x, f) in meanings.iter().enumerate() {
        for y in 0..names.len() {
            let expected = is_extension_of(&names[x], &names[y]);
            let observed = f.extension_to(y) > 0.0;
            assert_eq!(
                expected,
                observed,
                "Eq 12 support mismatch for h^{:?} at {:?}",
                show(&names[x]),
                show(&names[y])
            );
        }
    }

    // Two MLE spot-checks; the first telescopes two hops (Eq 8 chain rule):
    // h^ε("red firetruck") = π(red|ε)·π(red firetruck|red) = 0.5·0.75 = 3/8.
    let h_eps = &meanings[0];
    let h_red = &meanings[4];
    println!(
        "\n   h^ε(\"red firetruck\") = {:.4}   (0.5 · 0.75 = 3/8, Eq 8 telescoping)",
        h_eps.extension_to(5)
    );
    println!(
        "   h^red(\"red rose\")     = {:.4}   (1 of the 4 red traces)\n",
        h_red.extension_to(6)
    );
    assert!((h_eps.extension_to(5) - 0.375).abs() < TOL);
    assert!((h_red.extension_to(6) - 0.25).abs() < TOL);

    // -----------------------------------------------------------------------
    // 4. The hom of L̂ (Lemma 2, Eq 11): Ĉ(f, g) = inf_c min{1, g(c)/f(c)}.
    // -----------------------------------------------------------------------
    let h_blue = &meanings[1];
    let h_red_blue = semantic_hom(h_red, h_blue);
    let h_red_eps = semantic_hom(h_red, h_eps);
    let h_eps_red = semantic_hom(h_eps, h_red);

    println!("4. Semantic homs Ĉ(f, g) ∈ [0,1]  (Lemma 2, Eq 11)\n");
    println!(
        "     Ĉ(h^red,  h^blue) = {h_red_blue:.4}   red reaches contexts blue cannot\n     \
         Ĉ(h^red,  h^ε)    = {h_red_eps:.4}   ε's meaning contains red's, discounted by π(red|ε)\n     \
         Ĉ(h^ε,    h^red)  = {h_eps_red:.4}   ε reaches the blue branch, red does not\n"
    );
    assert!(h_red_blue.abs() < TOL);
    assert!((h_red_eps - 0.5).abs() < TOL);
    assert!(h_eps_red.abs() < TOL);

    // Identity of the [0,1]-category: Ĉ(f, f) = 1 is the unit id_f : I → Ĉ(f,f).
    for f in &meanings {
        assert!(
            (semantic_hom(f, f) - 1.0).abs() < TOL,
            "identity Ĉ(f, f) = 1 fails for h^{:?}",
            show(&names[f.base()])
        );
    }
    println!("     Ĉ(f, f) = 1 for all 7 meanings — the unit of the [0,1]-category ✓\n");

    // -----------------------------------------------------------------------
    // 5. THE CATEGORICAL CONTENT — enriched composition (Def 8).
    //
    //    In ([0,1], ×, 1) the composition morphism Ĉ(f,g) ⊗ Ĉ(g,h) → Ĉ(f,h)
    //    is the inequality Ĉ(f,g) · Ĉ(g,h) ≤ Ĉ(f,h). Checked exhaustively.
    // -----------------------------------------------------------------------
    println!("5. Enriched composition Ĉ(f,g) ⊗ Ĉ(g,h) → Ĉ(f,h)  (Def 8)\n");
    let mut checked = 0usize;
    let mut strict = 0usize;
    let mut equalities = 0usize;
    for f in &meanings {
        for g in &meanings {
            for h in &meanings {
                let lhs = semantic_hom(f, g) * semantic_hom(g, h);
                let rhs = semantic_hom(f, h);
                assert!(
                    lhs <= rhs + TOL,
                    "composition law violated: Ĉ(h^{:?}, h^{:?})·Ĉ(h^{:?}, h^{:?}) = {lhs} > {rhs}",
                    show(&names[f.base()]),
                    show(&names[g.base()]),
                    show(&names[g.base()]),
                    show(&names[h.base()])
                );
                checked += 1;
                if (rhs - lhs).abs() <= TOL {
                    equalities += 1;
                } else {
                    strict += 1;
                }
            }
        }
    }
    assert_eq!(checked, 343, "expected 7³ triples");
    println!(
        "     {checked} triples checked: {equalities} hold with equality, {strict} strictly.\n     \
         Composition is lax — that laxity IS the enrichment.\n"
    );

    // -----------------------------------------------------------------------
    // 6. Metric view (§5): d̂ = −ln Ĉ turns composition into the triangle
    //    inequality over the Lawvere generalized metric [0, ∞].
    // -----------------------------------------------------------------------
    let d_red_eps = semantic_distance(h_red, h_eps);
    println!("6. Metric view d̂(f, g) = −ln Ĉ(f, g) ∈ [0, ∞]  (§5)\n");
    println!("     d̂(h^red, h^ε) = −ln {h_red_eps:.1} = {d_red_eps:.4}\n");
    assert!((d_red_eps - (-0.5_f64.ln())).abs() < TOL);

    // Triangle inequality is the −ln image of step 5's composition law. An
    // infinite RHS makes it hold trivially; the converse cannot arise, since
    // Ĉ(f,g) > 0 ∧ Ĉ(g,h) > 0 ⇒ Ĉ(f,h) ≥ Ĉ(f,g)·Ĉ(g,h) > 0, so a finite RHS
    // forces a finite LHS. No ∞ − ∞ (hence no NaN) is ever formed.
    let mut triangles = 0usize;
    for f in &meanings {
        for g in &meanings {
            for h in &meanings {
                let lhs = semantic_distance(f, h);
                let rhs = semantic_distance(f, g) + semantic_distance(g, h);
                assert!(
                    lhs <= rhs + TOL,
                    "triangle inequality violated: d̂(h^{:?}, h^{:?}) = {lhs} > {rhs}",
                    show(&names[f.base()]),
                    show(&names[h.base()])
                );
                triangles += 1;
            }
        }
    }
    assert_eq!(triangles, 343, "expected 7³ triples");
    println!(
        "     {triangles} triangle inequalities d̂(f,h) ≤ d̂(f,g) + d̂(g,h) hold ✓\n     \
         (the −ln image of the composition law — same content, additive form)"
    );

    println!("\nAll semantic_category assertions hold.");
}
