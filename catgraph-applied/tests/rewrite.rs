//! The EQ5 process surface ([#214](https://github.com/sustia-llc/catgraph/issues/214)):
//! the content cost functional (W2) and bounded convex-DPO rewriting (W3).
//!
//! What these pin, in the order the module claims it:
//!
//! - **cost is a function of the morphism** — two SMC-equal writings cost the
//!   same because both are read off content (Lemma 4.1), while a user equation
//!   *does* move the number, which is the optimization signal;
//! - **rules are validated once, at construction** — non-parallel sides, an
//!   edge-free left-hand side, and a non-mono left interface are rejected there
//!   rather than at a match site;
//! - **the serde trust boundary is re-validated at every entry point**, on both
//!   arity (including the #196 overflow class) and *words*, since a
//!   `ColoredExpr` that skipped `colored::check` can be ill-typed with
//!   perfectly good arities;
//! - **matching is convex and injective** (BGKSZ Def 3.10 / 5.4) — the negative
//!   cases are the two a naive subgraph matcher would take: a path that leaves
//!   the image and returns, and an interface node used twice;
//! - **per-step soundness against the decider** — an optimized representative
//!   is `eq_mod`-equal to its start under the presentation the rules came from;
//! - **fuel bounds the search**, and the visited set closes rule cycles;
//! - **the neutral site surface** ([#250](https://github.com/sustia-llc/catgraph/issues/250))
//!   — enumeration returns every convex match and prefers none, apply-at-a-site
//!   fires where it is pointed even where cost descent would not go, a site is
//!   re-validated against the content it is handed to rather than trusted, and
//!   `MatchSite::into_step` records a chosen site as a replayable `RewriteStep`;
//! - **a site does not outlive its content** — an apply renumbers everything, so
//!   a site from an earlier enumeration can still form a convex match at a place
//!   nobody chose. The content fingerprint rejects it, with a message that is
//!   the *stale-site* diagnosis and not the not-a-convex-match one.

use std::borrow::Cow;

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::Presentation;
#[cfg(feature = "serde")]
use catgraph_applied::prop::presentation::content::is_arity_well_formed;
use catgraph_applied::prop::presentation::content::{
    Content, canonical_key, content_eq, content_of, content_of_colored,
};
use catgraph_applied::prop::presentation::rewrite::{
    RewriteRule, apply_at, cost_of, match_sites, match_sites_of, optimize, replay, rewrite_at,
};
use catgraph_applied::prop::{Free, PropExpr, PropSignature, mono_word};

// ---- A monochromatic tool chain (Λ = {•}, spelled `()`) ----------------------

/// `A`, `B`, `C`, `D` are `1 → 1` steps; `Split : 1 → 2` and `Join : 2 → 1`
/// give the fan-out/fan-in the interchange witness needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Tool {
    A,
    B,
    C,
    D,
    Split,
    Join,
}

impl PropSignature for Tool {
    type Color = ();

    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }

    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }

    fn source(&self) -> usize {
        match self {
            Tool::Join => 2,
            _ => 1,
        }
    }

    fn target(&self) -> usize {
        match self {
            Tool::Split => 2,
            _ => 1,
        }
    }
}

fn tool(g: Tool) -> PropExpr<Tool> {
    Free::generator(g)
}

/// Compose two steps whose interfaces meet.
fn chain(parts: [Tool; 2]) -> PropExpr<Tool> {
    Free::compose(tool(parts[0]), tool(parts[1])).expect("the two interfaces meet")
}

/// Compose three `1 → 1` steps.
fn seq3(parts: [Tool; 3]) -> PropExpr<Tool> {
    Free::compose(chain([parts[0], parts[1]]), tool(parts[2])).expect("1 → 1 throughout")
}

/// Pin `width` monochromatic wires onto `expr`.
fn wired(width: usize, expr: PropExpr<Tool>) -> ColoredExpr<Tool> {
    ColoredExpr::new(vec![(); width], expr).expect("monochromatic, so only arities can fail")
}

// ---- A role-typed workflow (Λ = {Author, Reviewer}) --------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Role {
    Author,
    Reviewer,
}

const AUTHOR: &[Role] = &[Role::Author];
const REVIEWER: &[Role] = &[Role::Reviewer];

/// `Write : [Author] → [Reviewer]`, `Check : [Reviewer] → [Reviewer]`,
/// `Fast : [Author] → [Reviewer]`, `Assign : [Author] → [Author]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum Task {
    Write,
    Check,
    Fast,
    Assign,
}

impl PropSignature for Task {
    type Color = Role;

    fn source_word(&self) -> Cow<'_, [Role]> {
        Cow::Borrowed(match self {
            Task::Write | Task::Fast | Task::Assign => AUTHOR,
            Task::Check => REVIEWER,
        })
    }

    fn target_word(&self) -> Cow<'_, [Role]> {
        Cow::Borrowed(match self {
            Task::Write | Task::Fast | Task::Check => REVIEWER,
            Task::Assign => AUTHOR,
        })
    }
}

fn task(g: Task) -> PropExpr<Task> {
    Free::generator(g)
}

/// Pin a one-wire source word onto a workflow fragment.
fn at(role: Role, expr: PropExpr<Task>) -> ColoredExpr<Task> {
    ColoredExpr::new(vec![role], expr).expect("the fragment is well-typed at this role")
}

// ---- W2: the cost functional -------------------------------------------------

#[test]
fn cost_is_a_function_of_the_morphism_not_of_the_writing() {
    // Generator count, the default weighting.
    let two = chain([Tool::A, Tool::B]);
    assert_eq!(cost_of(&content_of(&two), |_| 1), 2);
    assert_eq!(
        cost_of(&content_of(&PropExpr::<Tool>::Identity(3)), |_| 1),
        0
    );

    // Interchange: `(A ⊗ B) ; (A ⊗ B)` and `(A ; A) ⊗ (B ; B)` are the same
    // morphism written two ways. Equal content (Lemma 4.1) ⇒ equal cost.
    let layered = Free::compose(
        Free::tensor(tool(Tool::A), tool(Tool::B)),
        Free::tensor(tool(Tool::A), tool(Tool::B)),
    )
    .expect("2 → 2 twice");
    let interchanged = Free::tensor(chain([Tool::A, Tool::A]), chain([Tool::B, Tool::B]));
    assert!(content_eq(
        &content_of(&layered),
        &content_of(&interchanged)
    ));
    assert_eq!(cost_of(&content_of(&layered), |_| 1), 4);
    assert_eq!(cost_of(&content_of(&interchanged), |_| 1), 4);

    // A caller-supplied price is the koalisi hook: cg owns no semantics.
    let priced = |g: &Tool| match g {
        Tool::A => 10,
        Tool::B => 3,
        _ => 1,
    };
    assert_eq!(cost_of(&content_of(&two), priced), 13);

    // Colored contents are priced the same way; the wire typing is not a cost.
    let review = at(
        Role::Author,
        Free::compose(task(Task::Write), task(Task::Check)).expect("[Author] → [Reviewer] → …"),
    );
    assert_eq!(cost_of(&content_of_colored(&review), |_| 1), 2);
    assert_eq!(
        cost_of(
            &content_of_colored(&at(Role::Author, task(Task::Fast))),
            |_| 1
        ),
        1
    );
}

// ---- W3: rule validation -----------------------------------------------------

#[test]
fn rule_construction_rejects_what_the_dpo_step_cannot_use() {
    let presentation_error =
        |result: Result<RewriteRule<Task>, CatgraphError>, needle: &str| match result {
            Err(CatgraphError::Presentation { message }) => {
                assert!(message.contains(needle), "got: {message}");
            }
            other => panic!("expected a Presentation error mentioning {needle:?}, got {other:?}"),
        };

    // Non-parallel: the source words disagree…
    presentation_error(
        RewriteRule::new(
            at(Role::Author, task(Task::Write)),
            at(Role::Reviewer, task(Task::Check)),
        ),
        "source words",
    );
    // …and the target words disagree.
    presentation_error(
        RewriteRule::new(
            at(Role::Author, task(Task::Write)),
            at(Role::Author, task(Task::Assign)),
        ),
        "target words",
    );

    // An edge-free lhs matches everywhere.
    match RewriteRule::new(
        wired(0, PropExpr::<Tool>::Identity(0)),
        wired(0, PropExpr::<Tool>::Identity(0)),
    ) {
        Err(CatgraphError::Presentation { message }) => {
            assert!(
                message.contains("no generator occurrence"),
                "got: {message}"
            );
        }
        other => panic!("expected the edge-free rejection, got {other:?}"),
    }

    // A non-mono lhs interface: the identity wire of `id₁ ⊗ A` occupies an input
    // *and* an output coordinate, so the pushout complement is not unique.
    match RewriteRule::new(
        wired(2, Free::tensor(PropExpr::Identity(1), tool(Tool::A))),
        wired(2, Free::tensor(PropExpr::Identity(1), tool(Tool::B))),
    ) {
        Err(CatgraphError::Presentation { message }) => {
            assert!(message.contains("not mono"), "got: {message}");
        }
        other => panic!("expected the mono-interface rejection, got {other:?}"),
    }

    // The shape the engine is for is accepted.
    assert!(RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D))).is_ok());
}

/// The arity screens — including the [#196] overflow class — are reachable only
/// across `ColoredExpr`'s serde trust boundary, which is where they are tested.
///
/// [#196]: https://github.com/sustia-llc/catgraph/issues/196
#[cfg(feature = "serde")]
#[test]
fn rule_construction_screens_the_serde_trust_boundary() {
    // A `Compose` joining one wire to two: arity-ill-formed, so `content_of`
    // would panic rather than answer.
    let mismatched: ColoredExpr<Tool> = serde_json::from_str(
        r#"{"source_word":[null],"target_word":[null],
            "expr":{"Compose":[{"Identity":1},{"Identity":2}]}}"#,
    )
    .expect("the serde path does not re-run `check`");
    // A braid whose width sums past `usize::MAX` (#196).
    let overflowing: ColoredExpr<Tool> = serde_json::from_str(
        r#"{"source_word":[],"target_word":[],
            "expr":{"Braid":[18446744073709551615,1]}}"#,
    )
    .expect("the serde path does not re-run `check`");

    for forged in [mismatched, overflowing] {
        match RewriteRule::new(forged.clone(), forged) {
            Err(CatgraphError::Presentation { message }) => {
                assert!(message.contains("arity-well-formed"), "got: {message}");
            }
            other => panic!("expected the arity screen, got {other:?}"),
        }
    }
}

/// The **word** screen is the other half of that boundary, and the half an
/// arity screen cannot see: `Write ; Assign` joins `[Author] → [Reviewer]` to
/// `[Author] → [Author]`, so the wire *counts* line up and the colors do not.
/// A document may also simply lie about its target word. Every public entry
/// point re-runs `colored::check` against the declared source word and requires
/// the target word it derives to be the one stored — without which the colored
/// matcher would read node colors no `⟦·⟧` ever assigned.
#[cfg(feature = "serde")]
#[test]
fn every_entry_point_screens_a_color_forged_document() {
    // Colors disagree across a `Compose` whose arities agree.
    let ill_typed: ColoredExpr<Task> = serde_json::from_str(
        r#"{"source_word":["Author"],"target_word":["Author"],
            "expr":{"Compose":[{"Generator":"Write"},{"Generator":"Assign"}]}}"#,
    )
    .expect("the serde path does not re-run `check`");
    // Well-typed expression, forged target word.
    let mislabelled: ColoredExpr<Task> = serde_json::from_str(
        r#"{"source_word":["Author"],"target_word":["Author"],
            "expr":{"Generator":"Write"}}"#,
    )
    .expect("the serde path does not re-run `check`");

    // Both pass the arity screen, so only the word screen can catch them.
    assert!(is_arity_well_formed(ill_typed.expr()));
    assert!(is_arity_well_formed(mislabelled.expr()));

    let screened = |where_: &str, result: Result<(), CatgraphError>| match result {
        Err(CatgraphError::Presentation { message }) => {
            assert!(
                message.contains("word-well-formed") || message.contains("target word"),
                "{where_}: got {message}"
            );
        }
        other => panic!("{where_}: expected the word screen, got {other:?}"),
    };

    // The site surface is an entry point too, and both halves of it screen: since
    // #250's review, `match_sites_of` returns a `Result` rather than panicking
    // through `content_of_colored`, which matters because it is the *first* step
    // of the pair — a panic there aborts before `rewrite_at`'s screen is reached.
    let rule = RewriteRule::new(
        at(Role::Author, task(Task::Write)),
        at(Role::Author, task(Task::Fast)),
    )
    .expect("both [Author] → [Reviewer], one hyperedge, mono interface");
    let honest = at(Role::Author, task(Task::Write));
    let sites = match_sites_of(&honest, &rule, 1).expect("an honestly built expression");
    assert_eq!(sites.len(), 1);

    for forged in [ill_typed, mislabelled] {
        screened(
            "RewriteRule::new",
            RewriteRule::new(forged.clone(), forged.clone()).map(|_| ()),
        );
        screened("optimize", optimize(&forged, &[], 8, |_| 1).map(|_| ()));
        screened("replay", replay(&forged, &[], &[]).map(|_| ()));
        screened(
            "match_sites_of",
            match_sites_of(&forged, &rule, 8).map(|_| ()),
        );
        screened(
            "rewrite_at",
            rewrite_at(&forged, &rule, &sites[0]).map(|_| ()),
        );
    }
}

// ---- W3: matching ------------------------------------------------------------

#[test]
fn matching_is_convex_and_injective() {
    // `A ; B ⇒ D`, the connected rule.
    let sequential = RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
        .expect("parallel, mono-interfaced, two hyperedges");
    // `A ⊗ B ⇒ B ⊗ A`, the *disconnected* rule — the one whose matches have to
    // be screened for convexity, since its two hyperedges constrain nothing
    // about the path between them.
    let parallel = RewriteRule::new(
        wired(2, Free::tensor(tool(Tool::A), tool(Tool::B))),
        wired(2, Free::tensor(tool(Tool::B), tool(Tool::A))),
    )
    .expect("parallel, mono-interfaced, two hyperedges");

    let fired = |start: &ColoredExpr<Tool>, rule: &RewriteRule<Tool>| -> bool {
        optimize(start, std::slice::from_ref(rule), 64, |_| 1)
            .expect("well-formed start")
            .states_explored()
            > 1
    };

    // Positive: `A ; B` sits convexly inside `A ; B ; C` — the path out of the
    // image (into `C`) never comes back.
    let abc = wired(
        1,
        Free::compose(chain([Tool::A, Tool::B]), tool(Tool::C)).expect("1 → 1"),
    );
    assert!(fired(&abc, &sequential));

    // Positive control for the disconnected rule: two genuinely parallel arms.
    let a_par_b = wired(2, Free::tensor(tool(Tool::A), tool(Tool::B)));
    assert!(fired(&a_par_b, &parallel));

    // NEGATIVE — convexity. In `A ; C ; B` the two hyperedges the rule wants are
    // both present and their labels agree, but the directed path `A → C → B`
    // leaves the image and returns. BGKSZ Def 3.10 rules it out, and with it the
    // pushout complement.
    let acb = wired(
        1,
        Free::compose(
            Free::compose(tool(Tool::A), tool(Tool::C)).expect("1 → 1"),
            tool(Tool::B),
        )
        .expect("1 → 1"),
    );
    assert!(!fired(&acb, &parallel));

    // NEGATIVE — injectivity. In `A ; B` the rule's two interface nodes (`A`'s
    // target and `B`'s source) would have to land on the same wire.
    assert!(!fired(&wired(1, chain([Tool::A, Tool::B])), &parallel));

    // NEGATIVE — the rule fires only at its own role. `Check` is typed
    // `[Reviewer] → [Reviewer]`, and no `[Author]`-typed occurrence carries that
    // label; in a Λ-colored signature the label *determines* the tentacle
    // colors, so the color test in the matcher is a refinement subsumed by label
    // equality rather than an independent screen.
    let reviewer_rule = RewriteRule::new(
        at(
            Role::Reviewer,
            Free::compose(task(Task::Check), task(Task::Check)).expect("[Reviewer] twice"),
        ),
        at(Role::Reviewer, task(Task::Check)),
    )
    .expect("parallel over [Reviewer]");
    let author_side = at(
        Role::Author,
        Free::compose(task(Task::Assign), task(Task::Write)).expect("[Author] → [Author] → …"),
    );
    let outcome = optimize(&author_side, &[reviewer_rule], 64, |_| 1).expect("well-formed start");
    assert_eq!(outcome.states_explored(), 1);
    assert_eq!(outcome.best_cost(), 2);
}

// ---- W3: soundness against the decider --------------------------------------

#[test]
fn an_optimized_representative_is_equal_modulo_the_presentation() {
    let mut presentation = Presentation::<Tool>::new();
    presentation
        .add_equation(chain([Tool::A, Tool::B]), tool(Tool::D))
        .expect("both sides read 1 → 1");

    let rules = [
        RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
            .expect("the equation, oriented"),
    ];
    let start = wired(1, chain([Tool::A, Tool::B]));
    let outcome = optimize(&start, &rules, 64, |_| 1).expect("well-formed start");

    assert_eq!(outcome.initial_cost(), 2);
    assert_eq!(outcome.best_cost(), 1);
    assert_eq!(outcome.steps().len(), 1);
    assert_eq!(outcome.steps()[0].rule(), 0);
    assert_eq!(outcome.steps()[0].matched_edges().len(), 2);
    assert!(!outcome.fuel_exhausted());

    // The decider agrees — which is the whole soundness claim, checked rather
    // than asserted. `eq_mod` is untouched by this module.
    assert_eq!(
        presentation.eq_mod(start.expr(), outcome.best().expr()),
        Ok(Some(true))
    );

    // The trace is a witness: replaying it re-derives the very same state.
    let replayed = replay(&start, &rules, outcome.steps()).expect("the trace is legal");
    assert_eq!(
        canonical_key(&content_of_colored(&replayed)),
        canonical_key(&content_of_colored(outcome.best()))
    );

    // A forged trace is rejected rather than trusted.
    assert!(replay(&start, &rules, &[]).is_ok());
    assert!(matches!(
        replay(&start, &[], outcome.steps()),
        Err(CatgraphError::Presentation { .. })
    ));
}

#[test]
fn the_readback_re_checks_and_preserves_the_rewritten_content() {
    let rules = [
        RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
            .expect("A ; B ⇒ D"),
    ];
    let start = wired(
        1,
        Free::compose(chain([Tool::A, Tool::B]), tool(Tool::C)).expect("1 → 1"),
    );
    let outcome = optimize(&start, &rules, 64, |_| 1).expect("well-formed start");
    assert_eq!(outcome.best_cost(), 2);

    // The readback re-checks as a colored morphism *and* against the state's
    // own content — the two together are the engine's output validation, the
    // second of them discharging `expr_of_content`'s corpus-verified round
    // trip at runtime. So the result is the content the step produced, up to
    // cospan iso under both feet.
    let expected = wired(
        1,
        Free::compose(tool(Tool::D), tool(Tool::C)).expect("1 → 1"),
    );
    assert_eq!(outcome.best().source_word(), start.source_word());
    assert_eq!(outcome.best().target_word(), start.target_word());
    assert!(content_eq(
        &content_of_colored(outcome.best()),
        &content_of_colored(&expected)
    ));
    assert!(outcome.best().eq_colored(&expected));
}

#[test]
fn a_pass_through_right_hand_side_glues_the_interface() {
    // `Split ; Join ⇒ id₁`. The right-hand side threads its wire straight
    // through, so the step has to *merge* the two interface nodes the deleted
    // hyperedges sat between rather than only re-attach them — the one gluing
    // shape a rule with a mono right interface never exercises.
    let rules = [RewriteRule::new(
        wired(1, chain([Tool::Split, Tool::Join])),
        wired(1, PropExpr::Identity(1)),
    )
    .expect("both sides read 1 → 1; only the *left* interface must be mono")];
    let start = wired(
        1,
        Free::compose(
            Free::compose(tool(Tool::A), chain([Tool::Split, Tool::Join])).expect("1 → 1"),
            tool(Tool::B),
        )
        .expect("1 → 1"),
    );
    let outcome = optimize(&start, &rules, 64, |_| 1).expect("well-formed start");
    assert_eq!(outcome.initial_cost(), 4);
    assert_eq!(outcome.best_cost(), 2);
    assert!(
        outcome
            .best()
            .eq_colored(&wired(1, chain([Tool::A, Tool::B])))
    );
}

// ---- W3: fuel, dedup, and the monochromatic instance -------------------------

#[test]
fn fuel_bounds_the_search_and_the_visited_set_closes_rule_cycles() {
    let forward = RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
        .expect("A ; B ⇒ D");
    let backward = RewriteRule::new(wired(1, tool(Tool::D)), wired(1, chain([Tool::A, Tool::B])))
        .expect("D ⇒ A ; B");
    let start = wired(1, chain([Tool::A, Tool::B]));

    // Fuel 0 explores nothing and hands the start back — as its canonical
    // readback, so the identity to assert is SMC-equality, not `==`.
    let none = optimize(&start, std::slice::from_ref(&forward), 0, |_| 1).expect("well-formed");
    assert!(none.steps().is_empty());
    assert_eq!(none.states_explored(), 1);
    assert_eq!(none.best_cost(), none.initial_cost());
    assert!(none.best().eq_colored(&start));
    assert!(
        none.fuel_exhausted(),
        "a match was available and unaffordable"
    );

    // With no rules there is nothing to afford, so the budget is not the reason
    // the search stopped.
    let ruleless = optimize(&start, &[], 0, |_| 1).expect("well-formed");
    assert!(!ruleless.fuel_exhausted());

    // The cyclic pair terminates against the visited set rather than looping:
    // `D ⇒ A ; B` regenerates a state already seen.
    let cyclic = optimize(&start, &[forward, backward], 64, |_| 1).expect("well-formed");
    assert_eq!(cyclic.states_explored(), 2);
    assert_eq!(cyclic.best_cost(), 1);
    assert!(!cyclic.fuel_exhausted());
}

#[test]
fn the_colored_workflow_and_the_monochromatic_instance_both_run_end_to_end() {
    // Λ = {Author, Reviewer}: "write then review" collapses to the fast path.
    let rules = [RewriteRule::new(
        at(
            Role::Author,
            Free::compose(task(Task::Write), task(Task::Check)).expect("[Author] → [Reviewer]"),
        ),
        at(Role::Author, task(Task::Fast)),
    )
    .expect("parallel over [Author] → [Reviewer]")];
    let workflow = at(
        Role::Author,
        Free::compose(
            Free::compose(task(Task::Assign), task(Task::Write)).expect("[Author] → [Reviewer]"),
            task(Task::Check),
        )
        .expect("[Author] → [Reviewer]"),
    );
    let outcome = optimize(&workflow, &rules, 64, |_| 1).expect("well-formed start");
    assert_eq!(outcome.initial_cost(), 3);
    assert_eq!(outcome.best_cost(), 2);
    assert_eq!(outcome.best().source_word(), AUTHOR);
    assert_eq!(outcome.best().target_word(), REVIEWER);
    let expected = at(
        Role::Author,
        Free::compose(task(Task::Assign), task(Task::Fast)).expect("[Author] → [Reviewer]"),
    );
    assert!(outcome.best().eq_colored(&expected));

    // `Color = ()` is the same surface with one letter — no separate path.
    let mono_rules =
        [
            RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
                .expect("A ; B ⇒ D"),
        ];
    let mono_start = wired(
        1,
        Free::compose(tool(Tool::C), chain([Tool::A, Tool::B])).expect("1 → 1"),
    );
    let mono = optimize(&mono_start, &mono_rules, 64, |_| 1).expect("well-formed start");
    assert_eq!(mono.best_cost(), 2);
    assert!(mono.best().eq_colored(&wired(
        1,
        Free::compose(tool(Tool::C), tool(Tool::D)).expect("1 → 1")
    )));
}

// ---- #250: the neutral site surface ------------------------------------------
//
// `optimize` is a *policy* — descend on cost, stop on fuel — over two things:
// enumerate every convex match, and fire one chosen match. These pin that the
// two are now available on their own terms, that the applier re-validates what
// the enumerator handed out rather than trusting it, and that a site a caller
// picked itself records as an ordinary `RewriteStep`.

#[test]
fn enumeration_returns_every_site_and_the_applier_accepts_each_one() {
    // `A ⇒ D` has a one-hyperedge left-hand side, so `A ; C ; A` offers two legal
    // sites and nothing in the rule prefers either.
    let rule = RewriteRule::new(wired(1, tool(Tool::A)), wired(1, tool(Tool::D)))
        .expect("parallel 1 → 1, one hyperedge, mono interface");
    let start = wired(1, seq3([Tool::A, Tool::C, Tool::A]));
    let content = content_of_colored(&start);

    let sites = match_sites(&content, &rule, 8);
    assert_eq!(sites.len(), 2, "both `A` occurrences are legal sites");
    assert_ne!(
        sites[0].matched_edges(),
        sites[1].matched_edges(),
        "two sites, not one reported twice"
    );
    for site in &sites {
        // One `lhs` hyperedge, and the two wires it sits between.
        assert_eq!(site.matched_edges().len(), 1);
        assert_eq!(site.matched_nodes().len(), 2);
        assert!(site.matched_edges()[0] < content.edges().len());
        assert!(
            site.matched_nodes()
                .iter()
                .all(|&x| x < content.node_count()),
            "the indices are the target's own"
        );
    }

    // `limit` truncates the enumeration; it does not rank it.
    assert_eq!(match_sites(&content, &rule, 1).len(), 1);
    assert!(match_sites(&content, &rule, 0).is_empty());

    // Nothing the enumerator hands out is refused by the applier's own
    // re-validation: the two halves agree on what a site is.
    let mut rewritten: Vec<ColoredExpr<Tool>> = Vec::new();
    for site in &sites {
        let next = apply_at(&content, &rule, site).expect("an enumerated site is a convex match");
        assert_eq!(cost_of(&next, |_| 1), 3, "`A ⇒ D` is cost-neutral here");
        rewritten.push(rewrite_at(&start, &rule, site).expect("the same site, one level up"));
    }

    // Which site was chosen is visible in the result, and both choices are
    // legal — the surface fires where it is pointed.
    let leading = wired(1, seq3([Tool::D, Tool::C, Tool::A]));
    let trailing = wired(1, seq3([Tool::A, Tool::C, Tool::D]));
    assert!(!leading.eq_colored(&trailing));
    assert!(
        rewritten.iter().any(|e| e.eq_colored(&leading)),
        "one site rewrites the leading `A`"
    );
    assert!(
        rewritten.iter().any(|e| e.eq_colored(&trailing)),
        "the other rewrites the trailing one"
    );
}

#[test]
fn a_site_applies_where_cost_descent_would_never_go() {
    // `D ⇒ A ; B` strictly *raises* the generator count, so the optimizer has no
    // reason to fire it — and does not: it keeps the start.
    let expand = RewriteRule::new(wired(1, tool(Tool::D)), wired(1, chain([Tool::A, Tool::B])))
        .expect("parallel 1 → 1; only the *left* interface must be mono");
    let start = wired(1, chain([Tool::D, Tool::C]));

    let outcome =
        optimize(&start, std::slice::from_ref(&expand), 64, |_| 1).expect("well-formed start");
    assert_eq!(outcome.initial_cost(), 2);
    assert_eq!(outcome.best_cost(), 2);
    assert!(
        outcome.steps().is_empty(),
        "no cheaper writing exists under this rule, so cost descent records nothing"
    );
    assert!(outcome.best().eq_colored(&start));

    // The neutral surface fires it anyway. This is the gap #250 closes: a caller
    // whose objective is not "cheaper" — a walk, an externally scored choice, a
    // detour through a dearer writing — was locked out of the engine entirely.
    let sites = match_sites_of(&start, &expand, 8).expect("an honestly built expression");
    assert_eq!(sites.len(), 1);
    let widened = rewrite_at(&start, &expand, &sites[0]).expect("the site is a convex match");
    assert!(widened.eq_colored(&wired(1, seq3([Tool::A, Tool::B, Tool::C]))));
    assert_eq!(
        cost_of(&content_of_colored(&widened), |_| 1),
        3,
        "dearer than the start, and applied regardless"
    );
}

#[test]
fn a_site_is_re_validated_against_the_content_it_is_handed_to() {
    let sequential = RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
        .expect("A ; B ⇒ D");
    let gluing = RewriteRule::new(
        wired(1, chain([Tool::Split, Tool::Join])),
        wired(1, PropExpr::Identity(1)),
    )
    .expect("Split ; Join ⇒ id₁");

    // The site's provenance is the enumerator — the only way a caller can get
    // one. Everything below hands that honest value somewhere it does not belong.
    let here = wired(1, chain([Tool::A, Tool::B]));
    let content_here = content_of_colored(&here);
    let sites = match_sites(&content_here, &sequential, 8);
    assert_eq!(sites.len(), 1);
    let site = sites[0].clone();

    // Two *different* rejections, and the test pins which one fires where: a
    // site from another content is a stale-or-foreign site, while a site whose
    // content is right but whose assignment is not convex for the rule is a bad
    // pairing. Collapsing them would hide the location bug the fingerprint
    // exists to catch.
    let rejected = |what: &str, needle: &str, result: Result<(), CatgraphError>| match result {
        Err(CatgraphError::Presentation { message }) => {
            assert!(message.contains(needle), "{what}: got {message}");
        }
        other => panic!("{what}: expected the site screen, got {other:?}"),
    };
    let foreign = "enumerated from a different content";
    let not_a_match = "convex match";

    // A different content, whose hyperedges carry other labels…
    let elsewhere = content_of_colored(&wired(1, chain([Tool::C, Tool::C])));
    rejected(
        "other labels",
        foreign,
        apply_at(&elsewhere, &sequential, &site).map(|_| ()),
    );
    // …one too small to hold the assignment at all…
    let smaller = content_of_colored(&wired(1, tool(Tool::A)));
    rejected(
        "out of range",
        foreign,
        apply_at(&smaller, &sequential, &site).map(|_| ()),
    );
    // …and the right content under the wrong rule, which is the *other*
    // diagnosis: the site does belong here, it is simply not a match of this
    // rule.
    rejected(
        "wrong rule",
        not_a_match,
        apply_at(&content_here, &gluing, &site).map(|_| ()),
    );

    // The expression-level wrapper carries the same screen.
    rejected(
        "expression level",
        foreign,
        rewrite_at(&wired(1, chain([Tool::C, Tool::C])), &sequential, &site).map(|_| ()),
    );

    // The site that does belong still fires, and to the expected content.
    let applied = apply_at(&content_here, &sequential, &site).expect("its own content");
    assert!(content_eq(
        &applied,
        &content_of_colored(&wired(1, tool(Tool::D)))
    ));
}

#[test]
fn a_chosen_site_records_as_a_step_the_replay_re_derives() {
    let rule = RewriteRule::new(wired(1, tool(Tool::A)), wired(1, tool(Tool::D))).expect("A ⇒ D");
    let rules = [rule.clone()];
    let start = wired(1, seq3([Tool::A, Tool::C, Tool::A]));
    let content = content_of_colored(&start);

    let sites = match_sites(&content, &rule, 8);
    assert_eq!(sites.len(), 2);
    // The expression-level enumerator is the same enumeration one level up —
    // same sites, and the same content fingerprint on each, since
    // `match_sites_of` enumerates against exactly this content.
    assert_eq!(
        match_sites_of(&start, &rule, 8).expect("an honestly built expression"),
        sites
    );

    for site in &sites {
        // The expression pair is the content pair with `content_of_colored` in
        // front and the readback behind — nothing else.
        let by_content = apply_at(&content, &rule, site).expect("an enumerated site");
        let by_expr = rewrite_at(&start, &rule, site).expect("the same site, one level up");
        assert!(content_eq(&content_of_colored(&by_expr), &by_content));
        assert_eq!(by_expr.source_word(), start.source_word());
        assert_eq!(by_expr.target_word(), start.target_word());

        // A step recorded from a site replays to exactly that state — the
        // coherence claim `into_step` rests on: the trace surface and the site
        // surface agree on which match "this" is.
        let step = site.clone().into_step(0);
        assert_eq!(step.rule(), 0);
        assert_eq!(step.matched_edges(), site.matched_edges());
        let replayed = replay(&start, &rules, std::slice::from_ref(&step))
            .expect("a step built from a real site is a legal trace");
        assert!(replayed.eq_colored(&by_expr));
        assert_eq!(
            canonical_key(&content_of_colored(&replayed)),
            canonical_key(&by_content)
        );
    }
}

#[test]
fn a_stale_site_is_rejected_rather_than_applied_at_the_wrong_place() {
    // The #250 review probe, pinned. `apply_at` renumbers the surviving nodes and
    // appends the rhs, so after one apply the indices an earlier enumeration
    // handed out name *different* hyperedges. In a repetitive content they can
    // still form a perfectly convex match there, so re-deriving the assignment
    // alone accepts the stale site and rewrites the wrong occurrence in silence:
    //
    //     c0 = A ; A ; A            sites: three, one per occurrence
    //     c1 = apply_at(c0, s[0])   one A is now a B, and everything renumbered
    //     apply_at(c1, s[1])        used to succeed — at a location nobody chose
    //
    // The site's content fingerprint is what turns that second call into an
    // error, and into the *right* error: stale site, not "no convex match".
    let rule = RewriteRule::new(wired(1, tool(Tool::A)), wired(1, tool(Tool::B))).expect("A ⇒ B");
    let start = wired(1, seq3([Tool::A, Tool::A, Tool::A]));
    let c0 = content_of_colored(&start);

    let count = |content: &Content<Tool>, g: Tool| cost_of(content, |x| u64::from(*x == g));
    assert_eq!(count(&c0, Tool::A), 3);

    let stale = match_sites(&c0, &rule, 8);
    assert_eq!(stale.len(), 3, "three interchangeable `A` occurrences");

    let c1 = apply_at(&c0, &rule, &stale[0]).expect("a site applied to its own content");
    assert_eq!((count(&c1, Tool::A), count(&c1, Tool::B)), (2, 1));

    // Every site of the now-stale enumeration is refused against `c1` — including
    // the two that were never fired, which are the dangerous ones — and the
    // message names the stale-content case rather than convexity.
    for (which, site) in stale.iter().enumerate() {
        match apply_at(&c1, &rule, site) {
            Err(CatgraphError::Presentation { message }) => assert!(
                message.contains("enumerated from a different content"),
                "site {which}: got {message}"
            ),
            other => panic!("site {which}: a stale site must not apply, got {other:?}"),
        }
    }

    // The correct loop re-enumerates after each apply. Three steps, three `B`s,
    // and no site ever outlives the content it was enumerated from.
    let mut content = c0.clone();
    for step in 0..3 {
        let sites = match_sites(&content, &rule, 8);
        assert_eq!(sites.len(), 3 - step, "one fewer `A` to match each round");
        content = apply_at(&content, &rule, &sites[0]).expect("a freshly enumerated site");
    }
    assert_eq!((count(&content, Tool::A), count(&content, Tool::B)), (0, 3));
    assert!(match_sites(&content, &rule, 8).is_empty());
    assert!(content_eq(
        &content,
        &content_of_colored(&wired(1, seq3([Tool::B, Tool::B, Tool::B])))
    ));
}
