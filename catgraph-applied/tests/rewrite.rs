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
//!   nobody chose. The content fingerprint rejects it, with the *stale-site*
//!   variant and not the not-a-convex-match one.
//! - **the `sfg_to_colored_expr` bridge carries a measured exhibit into the
//!   engine** — `mat_to_sfg(A);mat_to_sfg(A) ⇒ mat_to_sfg(A·A)` over `F64Rig`
//!   pins 42 → 21 at ℓ = 2 and 63 → 42 at ℓ = 3, with the ℓ = 3 match sites at
//!   the two overlapping positions.

use std::borrow::Cow;

use catgraph::errors::{CatgraphError, RewriteBoundary, RewriteRejection, RewriteSide};
use catgraph_applied::mat::MatR;
use catgraph_applied::mat_to_sfg::mat_to_sfg;
use catgraph_applied::prop::colored::ColoredExpr;
use catgraph_applied::prop::presentation::Presentation;
#[cfg(feature = "serde")]
use catgraph_applied::prop::presentation::content::is_arity_well_formed;
use catgraph_applied::prop::presentation::content::{
    Content, canonical_key, content_eq, content_of, content_of_colored,
};
#[cfg(feature = "serde")]
use catgraph_applied::prop::presentation::rewrite::RewriteStep;
use catgraph_applied::prop::presentation::rewrite::{
    RewriteRule, apply_at, cost_of, match_sites, match_sites_of, optimize, replay, rewrite_at,
};
use catgraph_applied::prop::{Free, PropExpr, PropSignature, mono_word};
use catgraph_applied::rig::F64Rig;
use catgraph_applied::sfg_to_colored::sfg_to_colored_expr;

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
    // Each condition is its own `RewriteRejection` variant, so a caller reads the
    // violated clause off the value rather than off the message.
    let not_parallel = |result: Result<RewriteRule<Task>, CatgraphError>,
                        expected: RewriteBoundary| match result {
        Err(CatgraphError::Rewrite(RewriteRejection::SidesNotParallel { boundary })) => {
            assert_eq!(
                boundary, expected,
                "observed {boundary}, expected {expected}"
            );
        }
        other => panic!("expected SidesNotParallel on the {expected} words, got {other:?}"),
    };

    // Non-parallel: the source words disagree…
    not_parallel(
        RewriteRule::new(
            at(Role::Author, task(Task::Write)),
            at(Role::Reviewer, task(Task::Check)),
        ),
        RewriteBoundary::Source,
    );
    // …and the target words disagree.
    not_parallel(
        RewriteRule::new(
            at(Role::Author, task(Task::Write)),
            at(Role::Author, task(Task::Assign)),
        ),
        RewriteBoundary::Target,
    );

    // An edge-free lhs matches everywhere.
    match RewriteRule::new(
        wired(0, PropExpr::<Tool>::Identity(0)),
        wired(0, PropExpr::<Tool>::Identity(0)),
    ) {
        Err(CatgraphError::Rewrite(RewriteRejection::EmptyLhs)) => {}
        other => panic!("expected EmptyLhs, got {other:?}"),
    }

    // A non-mono lhs interface: the identity wire of `id₁ ⊗ A` occupies an input
    // *and* an output coordinate, so the pushout complement is not unique. The
    // rejection names *which* node, so the field is asserted rather than the
    // variant alone.
    match RewriteRule::new(
        wired(2, Free::tensor(PropExpr::Identity(1), tool(Tool::A))),
        wired(2, Free::tensor(PropExpr::Identity(1), tool(Tool::B))),
    ) {
        Err(CatgraphError::Rewrite(RewriteRejection::LhsInterfaceNotMono { node })) => {
            assert_eq!(
                node, 0,
                "observed node {node}, expected the identity wire 0"
            );
        }
        other => panic!("expected LhsInterfaceNotMono, got {other:?}"),
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

    // A well-formed lhs against the mismatched rhs names the right-hand side.
    let well_formed: ColoredExpr<Tool> = serde_json::from_str(
        r#"{"source_word":[null],"target_word":[null],"expr":{"Identity":1}}"#,
    )
    .expect("the serde path does not re-run `check`");
    match RewriteRule::new(well_formed, mismatched.clone()) {
        Err(CatgraphError::Rewrite(RewriteRejection::IllFormed { side, message })) => {
            assert_eq!(side, RewriteSide::Rhs, "observed {side}, expected the rhs");
            assert!(message.contains("arity-well-formed"), "got: {message}");
        }
        other => panic!("expected IllFormed on the rhs, got {other:?}"),
    }

    for forged in [mismatched, overflowing] {
        match RewriteRule::new(forged.clone(), forged) {
            Err(CatgraphError::Rewrite(RewriteRejection::IllFormed { side, message })) => {
                assert_eq!(side, RewriteSide::Lhs, "observed {side}, expected the lhs");
                assert!(message.contains("arity-well-formed"), "got: {message}");
            }
            other => panic!("expected IllFormed on the lhs, got {other:?}"),
        }
    }
}

/// The rendered text of a rejection carries the wrapper prefix, the boundary
/// name and the side name.
#[test]
fn rejections_render_the_boundary_and_side_names() {
    let not_parallel = CatgraphError::Rewrite(RewriteRejection::SidesNotParallel {
        boundary: RewriteBoundary::Source,
    });
    assert_eq!(
        not_parallel.to_string(),
        "rewrite rejected: the two sides declare different source words"
    );
    let ill_formed = CatgraphError::Rewrite(RewriteRejection::IllFormed {
        side: RewriteSide::Input,
        message: "x".to_string(),
    });
    assert_eq!(
        ill_formed.to_string(),
        "rewrite rejected: the input morphism is ill-formed: x"
    );
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

    // The typed side is the other half of what the rejection now carries: a rule
    // constructor names the side it screened, an entry point names its input.
    let screened =
        |where_: &str, expected: RewriteSide, result: Result<(), CatgraphError>| match result {
            Err(CatgraphError::Rewrite(RewriteRejection::IllFormed { side, message })) => {
                assert_eq!(
                    side, expected,
                    "{where_}: observed {side}, expected {expected}"
                );
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
            RewriteSide::Lhs,
            RewriteRule::new(forged.clone(), forged.clone()).map(|_| ()),
        );
        screened(
            "optimize",
            RewriteSide::Input,
            optimize(&forged, &[], 8, |_| 1).map(|_| ()),
        );
        screened(
            "replay",
            RewriteSide::Input,
            replay(&forged, &[], &[]).map(|_| ()),
        );
        screened(
            "match_sites_of",
            RewriteSide::Input,
            match_sites_of(&forged, &rule, 8).map(|_| ()),
        );
        screened(
            "rewrite_at",
            RewriteSide::Input,
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

/// The convexity sweep relaxes past the first hop out of the image: in
/// `A ; C ; D ; B` the directed path `A → C → D → B` leaves the image two
/// hyperedges before it returns, so `A ⊗ B ⇒ B ⊗ A` has no convex match there
/// and the search stands still.
#[test]
fn a_return_path_two_hops_outside_the_image_is_not_convex() {
    let parallel = RewriteRule::new(
        wired(2, Free::tensor(tool(Tool::A), tool(Tool::B))),
        wired(2, Free::tensor(tool(Tool::B), tool(Tool::A))),
    )
    .expect("parallel, mono-interfaced, two hyperedges");
    let acdb = wired(
        1,
        Free::compose(chain([Tool::A, Tool::C]), chain([Tool::D, Tool::B])).expect("1 → 1"),
    );

    let outcome =
        optimize(&acdb, std::slice::from_ref(&parallel), 64, |_| 1).expect("well-formed start");
    assert_eq!(outcome.states_explored(), 1);
    assert_eq!(outcome.best_cost(), outcome.initial_cost());
    assert!(
        outcome.steps().is_empty(),
        "no convex match, so the trace must be empty, got {:?}",
        outcome.steps()
    );
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

    // A forged trace is rejected rather than trusted, and the rejection carries
    // the three numbers a caller needs to place the fault: which step, which
    // rule index it named, and how many rules it was replayed against.
    assert!(replay(&start, &rules, &[]).is_ok());
    let named = outcome.steps()[0].rule();
    match replay(&start, &[], outcome.steps()) {
        Err(CatgraphError::Rewrite(RewriteRejection::UnknownRule { step, rule, rules })) => {
            assert_eq!(
                (step, rule, rules),
                (0, named, 0),
                "observed (step, rule, rules) = ({step}, {rule}, {rules}), expected (0, {named}, 0)"
            );
        }
        other => panic!("expected UnknownRule against an empty rules slice, got {other:?}"),
    }
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

/// A recorded step names the rule that fired at it, not the first rule of the
/// slice: under `[A ⇒ D, C ⇒ D]` the start `C` is only reachable by the second.
#[test]
fn a_step_records_the_index_of_the_rule_that_fired() {
    let rules = [
        RewriteRule::new(wired(1, tool(Tool::A)), wired(1, tool(Tool::D))).expect("A ⇒ D"),
        RewriteRule::new(wired(1, tool(Tool::C)), wired(1, tool(Tool::D))).expect("C ⇒ D"),
    ];
    let start = wired(1, tool(Tool::C));
    // `D` is free, so the rewrite is the descent the search takes.
    let cheap_d = |g: &Tool| u64::from(*g != Tool::D);

    let outcome = optimize(&start, &rules, 64, cheap_d).expect("well-formed start");
    assert_eq!(outcome.initial_cost(), 1);
    assert_eq!(outcome.best_cost(), 0);
    assert_eq!(outcome.steps().len(), 1);
    assert_eq!(outcome.steps()[0].rule(), 1);

    let replayed = replay(&start, &rules, outcome.steps()).expect("the trace is legal");
    assert!(
        replayed.eq_colored(&wired(1, tool(Tool::D))),
        "the trace must replay to D, reached {replayed:?}"
    );
}

/// `A ⊗ A` under `A ⇒ D` has two sites; a budget of 1 affords the first
/// application and reads exhausted at the second.
#[test]
fn each_application_spends_one_unit_of_fuel() {
    let rules =
        [RewriteRule::new(wired(1, tool(Tool::A)), wired(1, tool(Tool::D))).expect("A ⇒ D")];
    let start = wired(2, Free::tensor(tool(Tool::A), tool(Tool::A)));
    let cheap_d = |g: &Tool| u64::from(*g != Tool::D);

    let outcome = optimize(&start, &rules, 1, cheap_d).expect("well-formed start");
    assert_eq!(outcome.initial_cost(), 2);
    assert_eq!(outcome.best_cost(), 1);
    assert_eq!(outcome.steps().len(), 1);
    assert_eq!(outcome.states_explored(), 2);
    assert!(
        outcome.fuel_exhausted(),
        "the second site was matched and unaffordable, so the budget must read \
         exhausted, got false"
    );
}

/// Among states of equal cost the first reached is kept. Both rules rewrite
/// `A ⊗ B` to a state costing 3, and the earlier rule's is the one returned.
#[test]
fn an_equal_cost_successor_does_not_displace_the_first_reached() {
    let a_par_b = || wired(2, Free::tensor(tool(Tool::A), tool(Tool::B)));
    let rules = [
        RewriteRule::new(
            a_par_b(),
            wired(2, Free::tensor(tool(Tool::C), tool(Tool::B))),
        )
        .expect("A ⊗ B ⇒ C ⊗ B"),
        RewriteRule::new(
            a_par_b(),
            wired(2, Free::tensor(tool(Tool::A), tool(Tool::D))),
        )
        .expect("A ⊗ B ⇒ A ⊗ D"),
    ];
    // Two-wide left-hand sides, so neither result matches either rule again.
    let cost = |g: &Tool| match g {
        Tool::A | Tool::B => 2,
        _ => 1,
    };

    let outcome = optimize(&a_par_b(), &rules, 64, cost).expect("well-formed start");
    assert_eq!(outcome.initial_cost(), 4);
    assert_eq!(outcome.best_cost(), 3);
    assert_eq!(outcome.steps().len(), 1);
    assert_eq!(outcome.steps()[0].rule(), 0);
    let expected = wired(2, Free::tensor(tool(Tool::C), tool(Tool::B)));
    assert!(
        outcome.best().eq_colored(&expected),
        "the tie must keep C ⊗ B, got {:?}",
        outcome.best()
    );
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
    let rejected =
        |what: &str, expected: &RewriteRejection, result: Result<(), CatgraphError>| match result {
            Err(CatgraphError::Rewrite(rejection)) => {
                assert_eq!(
                    &rejection, expected,
                    "{what}: observed {rejection:?}, expected {expected:?}"
                );
            }
            other => panic!("{what}: expected the site screen, got {other:?}"),
        };
    let foreign = RewriteRejection::StaleSite;
    // An apply entry point carries no trace position, so `step` is `None` — the
    // field that separates this rejection from `replay`'s.
    let not_a_match = RewriteRejection::NotAMatch { step: None };

    // A different content, whose hyperedges carry other labels…
    let elsewhere = content_of_colored(&wired(1, chain([Tool::C, Tool::C])));
    rejected(
        "other labels",
        &foreign,
        apply_at(&elsewhere, &sequential, &site).map(|_| ()),
    );
    // …one too small to hold the assignment at all…
    let smaller = content_of_colored(&wired(1, tool(Tool::A)));
    rejected(
        "out of range",
        &foreign,
        apply_at(&smaller, &sequential, &site).map(|_| ()),
    );
    // …and the right content under the wrong rule, which is the *other*
    // diagnosis: the site does belong here, it is simply not a match of this
    // rule.
    rejected(
        "wrong rule",
        &not_a_match,
        apply_at(&content_here, &gluing, &site).map(|_| ()),
    );

    // The expression-level wrapper carries the same screen.
    rejected(
        "expression level",
        &foreign,
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
    // rejection is the stale-content variant rather than the convexity one.
    for (which, site) in stale.iter().enumerate() {
        match apply_at(&c1, &rule, site) {
            Err(CatgraphError::Rewrite(rejection)) => assert_eq!(
                rejection,
                RewriteRejection::StaleSite,
                "site {which}: observed {rejection:?}, expected StaleSite"
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

/// `replay` re-derives each recorded assignment rather than trusting it, so a
/// forged trace against `A ; B ⇒ D` is rejected on all three of: a hyperedge
/// index past the running state, one index used for both left-hand sides, and an
/// index carrying a label the left-hand side does not have there.
///
/// The steps are built through serde because the fields are private — this is
/// the untrusted-document path `RewriteStep`'s serde section describes.
#[cfg(feature = "serde")]
#[test]
fn replay_rejects_an_out_of_range_repeated_or_mislabeled_assignment() {
    let rules = [
        RewriteRule::new(wired(1, chain([Tool::A, Tool::B])), wired(1, tool(Tool::D)))
            .expect("A ; B ⇒ D"),
    ];
    // A replay rejection carries the trace position the assignment sits at, which
    // is what separates it from the same rejection raised by `apply_at`.
    let rejected = |what: &str, start: &ColoredExpr<Tool>, doc: &str| {
        let steps: Vec<RewriteStep> = serde_json::from_str(doc)
            .unwrap_or_else(|e| panic!("{what}: the forged document must deserialize: {e}"));
        match replay(start, &rules, &steps) {
            Err(CatgraphError::Rewrite(rejection)) => assert_eq!(
                rejection,
                RewriteRejection::NotAMatch { step: Some(0) },
                "{what}: observed {rejection:?}, expected NotAMatch {{ step: Some(0) }}"
            ),
            other => panic!("{what}: a forged assignment must not replay, got {other:?}"),
        }
    };

    // `A ; B` has two hyperedges, `A` at index 0 and `B` at index 1.
    let ab = wired(1, chain([Tool::A, Tool::B]));
    rejected("out of range", &ab, r#"[{"rule":0,"matched_edges":[0,7]}]"#);
    rejected(
        "repeated index",
        &ab,
        r#"[{"rule":0,"matched_edges":[0,0]}]"#,
    );

    // `C ; B` has the arity the assignment claims and `B` where the rule wants
    // it, but `C` at index 0 where the rule's left-hand side reads `A`.
    let cb = wired(1, chain([Tool::C, Tool::B]));
    rejected(
        "mislabeled edge",
        &cb,
        r#"[{"rule":0,"matched_edges":[0,1]}]"#,
    );
}

// ---- the mat_to_sfg exhibit through the sfg_to_colored_expr bridge -------------

/// The `mat_to_sfg(A);mat_to_sfg(A) ⇒ mat_to_sfg(A·A)` exhibit, bridged into
/// the engine by [`sfg_to_colored_expr`] and pinned at its measured values:
/// costs 42 → 21 at ℓ = 2 and 63 → 42 at ℓ = 3, one match site at ℓ = 2, and
/// the two overlapping positions at ℓ = 3.
///
/// The pin ranges over one matrix (`A` = the 3×3 path-graph P3 adjacency),
/// one rig (`F64Rig`), and two host depths (`mat_to_sfg(A)^{;ℓ}`, ℓ ∈ {2, 3}),
/// under `optimize` with `fuel = 64` and `per_gen = |_| 1`. It is a record of
/// this exhibit's behaviour, not a general claim about the matcher.
#[test]
fn the_mat_to_sfg_exhibit_pins_costs_and_sites_at_depths_two_and_three() {
    // A = P3 adjacency [[0,1,0],[1,0,1],[0,1,0]]; the rule squares it.
    let a = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(0.0), F64Rig(1.0), F64Rig(0.0)],
            vec![F64Rig(1.0), F64Rig(0.0), F64Rig(1.0)],
            vec![F64Rig(0.0), F64Rig(1.0), F64Rig(0.0)],
        ],
    )
    .expect("invariant: the 3×3 P3 fixture is rectangular");
    let a2 = a.matmul(&a).expect("invariant: 3×3 composes with 3×3");
    let a2_expected = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(1.0), F64Rig(0.0), F64Rig(1.0)],
            vec![F64Rig(0.0), F64Rig(2.0), F64Rig(0.0)],
            vec![F64Rig(1.0), F64Rig(0.0), F64Rig(1.0)],
        ],
    )
    .expect("invariant: the expected A·A fixture is rectangular");
    assert_eq!(
        a2.entries(),
        a2_expected.entries(),
        "A·A: observed {:?}, expected {:?}",
        a2.entries(),
        a2_expected.entries()
    );

    let g_a = mat_to_sfg(&a).expect(
        "invariant: mat_to_sfg is arity-safe for a MatR built through its own constructors",
    );
    let g_a2 = mat_to_sfg(&a2).expect(
        "invariant: mat_to_sfg is arity-safe for a MatR built through its own constructors",
    );
    let lhs = sfg_to_colored_expr(
        &g_a.compose(&g_a)
            .expect("invariant: 3→3 composes with 3→3"),
    )
    .expect("invariant: a Free-built composite with the mono word over its domain is word-well-formed");
    let rhs = sfg_to_colored_expr(&g_a2).expect(
        "invariant: a Free-built term with the mono word over its domain is word-well-formed",
    );
    let rule = RewriteRule::new(lhs, rhs)
        .expect("invariant: the two sides are parallel with a mono left interface");

    // The exhibit at depth ℓ: the outcome and the convex match sites of the
    // rule in the host.
    let at_depth = |ell: usize| {
        let mut host = g_a.clone();
        for _ in 1..ell {
            host = host
                .compose(&g_a)
                .expect("invariant: 3→3 self-composition composes at every step");
        }
        let start = sfg_to_colored_expr(&host).expect(
            "invariant: a Free-built composite with the mono word over its domain is word-well-formed",
        );
        let outcome = optimize(&start, std::slice::from_ref(&rule), 64, |_| 1)
            .expect("invariant: the start is word-well-formed");
        let sites =
            match_sites_of(&start, &rule, 65).expect("invariant: the start is word-well-formed");
        (outcome, sites)
    };

    // ℓ = 2 — the host is the rule's own lhs: one site, the identity match.
    let (two, sites_two) = at_depth(2);
    assert_eq!(
        two.initial_cost(),
        42,
        "ℓ=2 initial_cost: observed {}, expected 42",
        two.initial_cost()
    );
    assert_eq!(
        two.best_cost(),
        21,
        "ℓ=2 best_cost: observed {}, expected 21",
        two.best_cost()
    );
    assert_eq!(
        two.states_explored(),
        2,
        "ℓ=2 states_explored: observed {}, expected 2",
        two.states_explored()
    );
    assert_eq!(
        two.steps().len(),
        1,
        "ℓ=2 steps: observed {}, expected 1",
        two.steps().len()
    );
    assert!(
        !two.fuel_exhausted(),
        "ℓ=2 fuel_exhausted: observed true, expected false"
    );
    assert_eq!(
        sites_two.len(),
        1,
        "ℓ=2 match sites: observed {}, expected 1",
        sites_two.len()
    );
    assert_eq!(
        sites_two[0].matched_edges().to_vec(),
        (0..42).collect::<Vec<_>>(),
        "ℓ=2 site[0] matched_edges: observed {:?}, expected every host edge 0..=41",
        sites_two[0].matched_edges()
    );

    // ℓ = 3 — the first depth with a proper convex subterm: two sites, the
    // overlapping positions.
    let (three, sites_three) = at_depth(3);
    assert_eq!(
        three.initial_cost(),
        63,
        "ℓ=3 initial_cost: observed {}, expected 63",
        three.initial_cost()
    );
    assert_eq!(
        three.best_cost(),
        42,
        "ℓ=3 best_cost: observed {}, expected 42",
        three.best_cost()
    );
    assert_eq!(
        three.states_explored(),
        3,
        "ℓ=3 states_explored: observed {}, expected 3",
        three.states_explored()
    );
    assert_eq!(
        three.steps().len(),
        1,
        "ℓ=3 steps: observed {}, expected 1",
        three.steps().len()
    );
    assert!(
        !three.fuel_exhausted(),
        "ℓ=3 fuel_exhausted: observed true, expected false"
    );
    assert_eq!(
        sites_three.len(),
        2,
        "ℓ=3 match sites: observed {}, expected 2",
        sites_three.len()
    );
    let mut observed_sites: Vec<Vec<usize>> = sites_three
        .iter()
        .map(|site| site.matched_edges().to_vec())
        .collect();
    observed_sites.sort();
    let expected_sites = vec![(0..42).collect::<Vec<_>>(), (21..63).collect::<Vec<_>>()];
    assert_eq!(
        observed_sites, expected_sites,
        "ℓ=3 matched_edges: observed {:?}, expected the two overlapping positions (edges 0..=41 and 21..=62)",
        observed_sites
    );
}
