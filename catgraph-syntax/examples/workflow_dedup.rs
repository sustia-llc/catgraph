//! **A role-typed workflow, deduplicated and then optimized** — the first
//! colored example in this directory ([#214](https://github.com/sustia-llc/catgraph/issues/214)
//! W1).
//!
//! A coalition's process is a string diagram over a palette of **roles**:
//! `Λ = {Author, Reviewer, Editor}` is [`PropSignature::Color`], a typed step is
//! a generator with a source and target *word* over it (`merge : R E -> E` is
//! the literal `g : A B -> C` shape), sequencing is [`Free::compose`], parallel
//! work is [`Free::tensor`], and per-role resource wiring is the [`FrobeniusOr`]
//! spider family — an `Author` wire cannot silently merge with a `Reviewer` one.
//!
//! The story runs in four beats:
//!
//! 1. **assemble** three workflow variants over the role palette, pinning colors
//!    with [`ColoredExpr::new`] (the only way to pin them — `id` and `braid` are
//!    color-polymorphic by design);
//! 2. **persist** the workflow theory as a presentation file — declarations
//!    `TOKEN : COLOR* -> COLOR*` plus `mu@R`-style spider tokens — and read it
//!    back;
//! 3. **deduplicate** the variants by [`canonical_key`], which collapses two
//!    writings of the same process into one bucket, cross-checked against
//!    [`ColoredExpr::eq_colored`];
//! 4. **optimize** the surviving process: price it with [`cost_of`] and search
//!    for a cheaper writing with [`optimize`], printing the trace.
//!
//! Run: `cargo run -p catgraph-syntax --example workflow_dedup`
//!
//! [`PropSignature::Color`]: catgraph_applied::prop::PropSignature::Color

use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;

use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::Presentation;
use catgraph_applied::prop::presentation::content::{canonical_key, content_of_colored};
use catgraph_applied::prop::presentation::rewrite::{RewriteRule, cost_of, optimize};
use catgraph_applied::prop::{Free, PropExpr, PropSignature};
use catgraph_syntax::frobenius::{FrobeniusOr, hypergraph_presentation};
use catgraph_syntax::text::{
    ColorSyntax, GeneratorSyntax, parse_presentation, print, print_presentation,
};

// ---- Λ = {Author, Reviewer, Editor}: the role palette ------------------------

/// The role alphabet. A fieldless enum satisfies the whole bound stack the
/// colored surface asks for: `Clone + Eq + Hash + Debug` for
/// [`PropSignature::Color`], `+ Ord` for the spiders, `+ Copy` for the cospan
/// functor, and [`ColorSyntax`] for the text layer.
///
/// **Caller-side mapping convention.** A downstream coalition indexes its
/// members by `RoleId(usize)`; that is *not* this type, and catgraph ships no
/// converter. `RoleId ↔ Role` stays a caller-side table by design — the process
/// signal (dedup, cost, an optimized representative) rides **beside** a
/// coalition's value signal rather than through it, so there is no
/// diagram→magnitude seam here and no crate edge to add.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Role {
    Author,
    Reviewer,
    Editor,
}

impl ColorSyntax for Role {
    fn print_color(&self) -> Option<String> {
        Some(
            match self {
                Role::Author => "A",
                Role::Reviewer => "R",
                Role::Editor => "E",
            }
            .to_string(),
        )
    }

    fn parse_color(token: &str) -> Option<Self> {
        match token {
            "A" => Some(Role::Author),
            "R" => Some(Role::Reviewer),
            "E" => Some(Role::Editor),
            _ => None,
        }
    }

    /// No implicit sort: every role is written out, and a bare `mu` is a parse
    /// error rather than a silently mistyped spider.
    fn implicit() -> Option<Self> {
        None
    }
}

// ---- The step signature ------------------------------------------------------

/// One typed step of the workflow. `Merge : [Reviewer, Editor] → [Editor]` is
/// the two-input shape a single arity could not express.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Step {
    Draft,
    Review,
    Revise,
    Merge,
    Publish,
    Fast,
}

const AUTHOR: &[Role] = &[Role::Author];
const REVIEWER: &[Role] = &[Role::Reviewer];
const EDITOR: &[Role] = &[Role::Editor];
const REVIEWER_EDITOR: &[Role] = &[Role::Reviewer, Role::Editor];

impl PropSignature for Step {
    type Color = Role;

    fn source_word(&self) -> Cow<'_, [Role]> {
        Cow::Borrowed(match self {
            Step::Draft | Step::Fast => AUTHOR,
            Step::Review | Step::Revise => REVIEWER,
            Step::Merge => REVIEWER_EDITOR,
            Step::Publish => EDITOR,
        })
    }

    fn target_word(&self) -> Cow<'_, [Role]> {
        Cow::Borrowed(match self {
            Step::Draft | Step::Revise => REVIEWER,
            Step::Review | Step::Fast | Step::Merge | Step::Publish => EDITOR,
        })
    }
}

impl GeneratorSyntax for Step {
    fn print_token(&self) -> String {
        match self {
            Step::Draft => "draft",
            Step::Review => "review",
            Step::Revise => "revise",
            Step::Merge => "merge",
            Step::Publish => "publish",
            Step::Fast => "fast",
        }
        .to_string()
    }

    fn parse_token(token: &str) -> Option<Self> {
        match token {
            "draft" => Some(Step::Draft),
            "review" => Some(Step::Review),
            "revise" => Some(Step::Revise),
            "merge" => Some(Step::Merge),
            "publish" => Some(Step::Publish),
            "fast" => Some(Step::Fast),
            _ => None,
        }
    }
}

/// The working signature: workflow steps plus one spider family per role.
type Gen = FrobeniusOr<Step>;

fn step(s: Step) -> PropExpr<Gen> {
    Free::generator(FrobeniusOr::User(s))
}

fn spider(g: Gen) -> PropExpr<Gen> {
    Free::generator(g)
}

/// Compose left-to-right, reporting the first interface that does not meet.
fn pipeline(parts: Vec<PropExpr<Gen>>) -> Result<PropExpr<Gen>, Box<dyn Error>> {
    let mut parts = parts.into_iter();
    let mut acc = parts.next().ok_or("a pipeline needs at least one stage")?;
    for next in parts {
        acc = Free::compose(acc, next)?;
    }
    Ok(acc)
}

/// Pin the source word onto an expression — the only way colors get fixed.
fn from_author(expr: PropExpr<Gen>) -> Result<ColoredExpr<Gen>, Box<dyn Error>> {
    Ok(ColoredExpr::new(vec![Role::Author], expr)?)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== a role-typed workflow over Λ = {{A, R, E}} ===\n");

    // ---- 1. Assembly ---------------------------------------------------------
    //
    // "Draft, fan out to two reviewers, merge their verdicts": the δ_R spider
    // splits one Reviewer wire, the μ_E spider merges two Editor wires. Spider
    // fusion is colour-disjoint, so neither can absorb a wire of another role.
    let two_reviewers = pipeline(vec![
        step(Step::Draft),
        spider(FrobeniusOr::Delta(Role::Reviewer)),
        Free::tensor(step(Step::Review), step(Step::Review)),
        spider(FrobeniusOr::Mu(Role::Editor)),
    ])?;

    // The same morphism, written with interchange: `(f ⊗ id) ; (id ⊗ g)` instead
    // of `f ⊗ g`. A different *tree*, an identical process.
    let two_reviewers_interchanged = pipeline(vec![
        step(Step::Draft),
        spider(FrobeniusOr::Delta(Role::Reviewer)),
        Free::compose(
            Free::tensor(step(Step::Review), Free::identity(1)),
            Free::tensor(Free::identity(1), step(Step::Review)),
        )?,
        spider(FrobeniusOr::Mu(Role::Editor)),
    ])?;

    // A genuinely different process: one reviewer's raw output is merged with
    // the other's verdict by the two-input `merge : R E -> E`.
    let merge_variant = pipeline(vec![
        step(Step::Draft),
        spider(FrobeniusOr::Delta(Role::Reviewer)),
        Free::tensor(Free::identity(1), step(Step::Review)),
        step(Step::Merge),
    ])?;

    let variants = [
        ("two_reviewers", from_author(two_reviewers)?),
        (
            "two_reviewers_interchanged",
            from_author(two_reviewers_interchanged)?,
        ),
        ("merge_variant", from_author(merge_variant)?),
    ];
    for (name, workflow) in &variants {
        println!(
            "  {name:<27}: {:<58} {:?} → {:?}  ({})",
            print(workflow.expr()),
            workflow.source_word(),
            workflow.target_word(),
            cost_of(&content_of_colored(workflow), |_| 1)
        );
    }
    println!("  (the trailing count is cost_of at weight 1 — spiders are occurrences too)");

    // ---- 2. The theory, as a presentation file -------------------------------
    //
    // `TOKEN : COLOR* -> COLOR*` declarations are a **drift check** against the
    // signature `Gen`, not a construction of it, and every spider prints its
    // role (`mu@R`) because the palette has no implicit sort.
    let mut theory = Presentation::<Gen>::new();
    theory.add_equation(
        Free::compose(step(Step::Draft), step(Step::Review))?,
        step(Step::Fast),
    )?;
    theory.add_equation(
        Free::compose(step(Step::Publish), step(Step::Publish))?,
        step(Step::Publish),
    )?;
    theory.add_equation(
        Free::compose(
            spider(FrobeniusOr::Delta(Role::Reviewer)),
            spider(FrobeniusOr::Mu(Role::Reviewer)),
        )?,
        Free::identity(1),
    )?;

    let text = print_presentation(&theory);
    println!("\nworkflow theory as a presentation file:");
    for line in text.lines() {
        println!("  {line}");
    }
    let reparsed = parse_presentation::<Gen>(&text)?;
    assert_eq!(reparsed.equations(), theory.equations());
    println!("  (round-trips: printed, re-read, same equation list)");

    // Seeding the *whole* hypergraph theory is one call: F&S 2019 Lemma 3.10
    // says a Frobenius structure per role induces the rest, so
    // `hypergraph_presentation` adds nine SCFM equations per colour on top of
    // the user equations it is handed.
    let user_equations = [
        (
            Free::<Step>::compose(Free::generator(Step::Draft), Free::generator(Step::Review))?,
            Free::<Step>::generator(Step::Fast),
        ),
        (
            Free::<Step>::compose(
                Free::generator(Step::Publish),
                Free::generator(Step::Publish),
            )?,
            Free::<Step>::generator(Step::Publish),
        ),
    ];
    let full = hypergraph_presentation::<Step>(
        [Role::Author, Role::Reviewer, Role::Editor],
        user_equations,
    )?;
    assert_eq!(full.equations().len(), 2 + 3 * 9);
    println!(
        "  full hypergraph theory: {} equations (2 user + 9 SCFM × 3 roles)",
        full.equations().len()
    );

    // ---- 3. Dedup by canonical key ------------------------------------------
    //
    // Seams worth stating once, because a downstream harness will otherwise meet
    // them the hard way:
    //
    // * **like with like** — a `content_of` content and a `content_of_colored`
    //   one differ on any wire no generator touches, so a dedup table uses ONE
    //   entry point throughout. This table uses `content_of_colored`.
    // * **`ContentKey` is deliberately not `Ord`** (`Color` need not be), so the
    //   buckets are unordered: never depend on iteration order for a report.
    // * **serde does not re-run `check`** — a `ColoredExpr` rebuilt from a
    //   document is trusted, not validated. Re-validate untrusted input by
    //   rebuilding through `ColoredExpr::new`.
    let mut table: HashMap<_, Vec<&str>> = HashMap::new();
    for (name, workflow) in &variants {
        table
            .entry(canonical_key(&content_of_colored(workflow)))
            .or_default()
            .push(name);
    }
    let mut buckets: Vec<Vec<&str>> = table.into_values().collect();
    // Sorted here only so the printout is stable — the table itself is unordered.
    buckets.sort_unstable();
    println!(
        "\ndedup by canonical_key: {} distinct processes",
        buckets.len()
    );
    for bucket in &buckets {
        println!("  {bucket:?}");
    }
    assert_eq!(buckets.len(), 2);

    // `eq_colored` decides the same question in both directions on
    // word-well-formed values, so the table and the verdicts agree by
    // construction.
    assert!(variants[0].1.eq_colored(&variants[1].1));
    assert!(!variants[0].1.eq_colored(&variants[2].1));
    println!("  eq_colored agrees: interchange yes, merge-variant no");

    // ---- 4. Optimize the surviving process -----------------------------------
    //
    // Two rules, oriented left to right out of the same theory: "draft then
    // review is the fast path", and "publishing twice is publishing once".
    let rules = [
        RewriteRule::new(
            from_author(Free::compose(step(Step::Draft), step(Step::Review))?)?,
            from_author(step(Step::Fast))?,
        )?,
        RewriteRule::new(
            ColoredExpr::new(
                vec![Role::Editor],
                Free::compose(step(Step::Publish), step(Step::Publish))?,
            )?,
            ColoredExpr::new(vec![Role::Editor], step(Step::Publish))?,
        )?,
    ];

    let long_way = from_author(pipeline(vec![
        step(Step::Draft),
        step(Step::Review),
        step(Step::Publish),
        step(Step::Publish),
    ])?)?;
    println!("\nstarting process     : {}", print(long_way.expr()));

    let outcome = optimize(&long_way, &rules, 64, |_| 1)?;
    println!(
        "step count           : {} → {}{}",
        outcome.initial_cost(),
        outcome.best_cost(),
        if outcome.fuel_exhausted() {
            "  (fuel exhausted — best *found*, not best)"
        } else {
            ""
        }
    );
    println!("optimized process    : {}", print(outcome.best().expr()));
    println!("states explored      : {}", outcome.states_explored());
    for (index, applied) in outcome.steps().iter().enumerate() {
        println!(
            "  step {index}: rule {} at hyperedges {:?}",
            applied.rule(),
            applied.matched_edges()
        );
    }
    assert_eq!(outcome.initial_cost(), 4);
    assert_eq!(outcome.best_cost(), 2);
    assert_eq!(outcome.steps().len(), 2);

    // The objective is the caller's: a per-step price makes the *same* search
    // minimize wall-clock minutes instead of step count. catgraph owns none of
    // that semantics.
    let minutes = |g: &Gen| match g {
        FrobeniusOr::User(Step::Draft) => 90,
        FrobeniusOr::User(Step::Review) => 240,
        FrobeniusOr::User(Step::Fast) => 150,
        FrobeniusOr::User(Step::Publish) => 20,
        _ => 0,
    };
    let priced = optimize(&long_way, &rules, 64, minutes)?;
    println!(
        "\npriced in minutes    : {} → {}",
        priced.initial_cost(),
        priced.best_cost()
    );
    assert_eq!(priced.initial_cost(), 90 + 240 + 20 + 20);
    assert_eq!(priced.best_cost(), 150 + 20);
    println!(
        "  cost is read off *content*, so it is a function of the process, not of\n  \
         the writing — but it is deliberately NOT invariant under the theory's\n  \
         equations, and that difference is the whole optimization signal."
    );

    // The optimizer is not a decider. `eq_mod` is, and it confirms the result
    // independently — soundly, in the `Some(true)` direction only (#15). Note
    // also that `eq_mod` is **not transitive** (#189): a caller that wants
    // equivalence *classes* out of it must take connected components of the
    // `Some(true)` graph rather than scan against representatives.
    let verdict = theory.eq_mod(long_way.expr(), outcome.best().expr())?;
    println!("\neq_mod(start, best)  : {verdict:?}   (the decider, consulted separately)");
    assert_eq!(verdict, Some(true));

    println!("\nThe roles are types; the dedup is exact; the optimization is bounded.");
    Ok(())
}
