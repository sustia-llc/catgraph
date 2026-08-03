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
//! - **fuel bounds the search**, and the visited set closes rule cycles.

use std::borrow::Cow;

use catgraph::errors::CatgraphError;
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::Presentation;
#[cfg(feature = "serde")]
use catgraph_applied::prop::presentation::content::is_arity_well_formed;
use catgraph_applied::prop::presentation::content::{
    canonical_key, content_eq, content_of, content_of_colored,
};
use catgraph_applied::prop::presentation::rewrite::{RewriteRule, cost_of, optimize, replay};
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

    for forged in [ill_typed, mislabelled] {
        screened(
            "RewriteRule::new",
            RewriteRule::new(forged.clone(), forged.clone()).map(|_| ()),
        );
        screened("optimize", optimize(&forged, &[], 8, |_| 1).map(|_| ()));
        screened("replay", replay(&forged, &[], &[]).map(|_| ()));
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
