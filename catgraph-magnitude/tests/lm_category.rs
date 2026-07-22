//! [`LmCategory`] unit tests + BV 2025 intro magnitude-bounds proptest.
//!
//! The two paper-anchored acceptance tests (Prop 3.10 closed form,
//! Rem 3.11 / Eq (12) Shannon recovery) live in `tests/bv_2025_acceptance.rs`
//! so they appear as
//! a distinct test binary in `cargo test` output — they are the
//! acceptance gate and visibility matters.

// `usize → f64` casts on small-state-count test fixtures are precision-safe.
#![allow(clippy::cast_precision_loss)]

use catgraph_magnitude::lm_category::LmCategory;
use catgraph_testutil::Lcg;
use proptest::prelude::*;

/// `magnitude(t)` requires no transitions at all to be well-defined: every
/// state has `d(x, x) = 0` (identity axiom) and every off-diagonal is `+∞`.
/// `ζ_t = I` ⇒ `μ_t = I` ⇒ `Mag = n` (the trace of the identity).
#[test]
fn empty_transitions_magnitude_is_n() {
    let m = LmCategory::new(vec!["a".into(), "b".into(), "c".into()]);
    let mag = m.magnitude(1.5).expect("identity zeta is invertible");
    assert!(
        (mag - 3.0).abs() < 1e-12,
        "empty-transition LM magnitude should be n=3, got {mag}"
    );
}

/// Round-trip: `add_transition` followed by `transitions().get` recovers
/// the inserted probability; `mark_terminating` followed by
/// `terminating().contains` recovers the membership.
#[test]
fn add_transition_and_mark_terminating_round_trip() {
    let mut m = LmCategory::new(vec!["A".into(), "B".into(), "C".into()]);
    m.add_transition("A", "B", 0.5).unwrap();
    m.add_transition("A", "C", 0.3).unwrap();
    m.mark_terminating("A");

    assert_eq!(
        m.transitions().get("A").and_then(|r| r.get("B")),
        Some(&0.5)
    );
    assert_eq!(
        m.transitions().get("A").and_then(|r| r.get("C")),
        Some(&0.3)
    );
    assert!(m.terminating().contains("A"));
    assert!(!m.terminating().contains("B"));
    assert_eq!(m.objects().len(), 3);
}

/// Magnitude is finite and (per the p.4 intro bounds) bounded on a small tree-shaped LM.
///
/// Uses a minimal `A = {a}, N = 1` tree (4 states), the same shape as the
/// BV 2025 acceptance fixture.
#[test]
fn magnitude_smoke_tree_lm() {
    let mut m = LmCategory::new(vec!["s0".into(), "s0a".into(), "s0t".into(), "s0at".into()]);
    m.mark_terminating("s0t");
    m.mark_terminating("s0at");
    m.add_transition("s0", "s0a", 0.6).unwrap();
    m.add_transition("s0", "s0t", 0.4).unwrap();
    m.add_transition("s0a", "s0at", 1.0).unwrap();

    for &t in &[0.5_f64, 1.5, 2.0, 5.0] {
        let mag = m.magnitude(t).expect("zeta_t should be invertible");
        assert!(
            mag.is_finite(),
            "Mag(tM) at t={t} should be finite, got {mag}"
        );
        // BV 2025 p.4 intro bounds (derivable from Prop 3.10): #T(⊥) ≤ Mag(tM) ≤ #ob(M) for t ≥ 1.
        if t >= 1.0 {
            assert!(
                mag >= m.terminating().len() as f64 - 1e-9,
                "intro lower bound violated at t={t}: mag={mag}"
            );
            assert!(
                mag <= m.objects().len() as f64 + 1e-9,
                "intro upper bound violated at t={t}: mag={mag}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// add_transition Result error paths
// ---------------------------------------------------------------------------

/// `add_transition` returns `Err` when `from` is not in `objects`.
#[test]
fn add_transition_unknown_from_state_errors() {
    let mut m = LmCategory::new(vec!["A".into(), "B".into()]);
    let err = m.add_transition("ZZZ", "B", 0.5);
    assert!(err.is_err(), "unknown from-state must error, got {err:?}");
}

/// `add_transition` returns `Err` when `to` is not in `objects`.
#[test]
fn add_transition_unknown_to_state_errors() {
    let mut m = LmCategory::new(vec!["A".into(), "B".into()]);
    let err = m.add_transition("A", "ZZZ", 0.5);
    assert!(err.is_err(), "unknown to-state must error, got {err:?}");
}

/// `add_transition` returns `Err` when `prob ∉ [0, 1]` — release-mode
/// promotion of the previous debug-only assertion.
#[test]
fn add_transition_prob_out_of_range_errors() {
    let mut m = LmCategory::new(vec!["A".into(), "B".into()]);
    assert!(m.add_transition("A", "B", -0.1).is_err());
    assert!(m.add_transition("A", "B", 1.1).is_err());
    assert!(m.add_transition("A", "B", f64::NAN).is_err());
    // Endpoints are accepted.
    assert!(m.add_transition("A", "B", 0.0).is_ok());
    assert!(m.add_transition("A", "B", 1.0).is_ok());
}

/// `add_transition` rejects non-trivial self-loops by
/// construction; BV 2025 §3 hypothesis forbids them.
#[test]
fn add_transition_self_loop_errors() {
    let mut m = LmCategory::new(vec!["A".into()]);
    assert!(m.add_transition("A", "A", 0.5).is_err());
    // prob == 0.0 self-loop is accepted as a no-op for caller convenience.
    assert!(m.add_transition("A", "A", 0.0).is_ok());
}

/// `from_transition_log` reconstructs an LM from an append-only
/// log of `(from, to, prob)` triples, mirroring `magnitude_history`
/// replay-from-event-log semantics.
#[test]
fn from_transition_log_replays_lm() {
    let objects = vec!["s0".into(), "s0a".into(), "s0t".into(), "s0at".into()];
    let terminating = vec!["s0t".to_owned(), "s0at".to_owned()];
    let log: Vec<(&str, &str, f64)> =
        vec![("s0", "s0a", 0.6), ("s0", "s0t", 0.4), ("s0a", "s0at", 1.0)];
    let m = LmCategory::from_transition_log(objects, terminating, log)
        .expect("replay should succeed on valid log");
    assert_eq!(m.objects().len(), 4);
    assert!(m.terminating().contains("s0t"));
    assert!(m.terminating().contains("s0at"));
    assert_eq!(
        m.transitions().get("s0").and_then(|r| r.get("s0a")),
        Some(&0.6)
    );
}

/// `from_transition_log` propagates validation errors from
/// `add_transition` — invalid log entries fail-fast.
#[test]
fn from_transition_log_propagates_validation_error() {
    let objects = vec!["A".into(), "B".into()];
    let log: Vec<(&str, &str, f64)> = vec![("A", "B", 0.5), ("UNKNOWN", "B", 0.5)];
    let result = LmCategory::from_transition_log(objects, Vec::<String>::new(), log);
    assert!(
        result.is_err(),
        "log with unknown from-state must error, got {result:?}"
    );
}

/// The documented max-probability-path contract on cyclic tables: a
/// `prob = 1.0` two-cycle (`A ⇄ B`, both directions certain) is
/// `add_transition`-legal, must terminate (the strict-improvement relaxation
/// treats the equal-probability rederivation as a non-improvement, not an
/// oscillation), and must yield `d(A, B) = d(B, A) = −ln 1 = 0`.
#[test]
fn enriched_space_prob_one_two_cycle_terminates_with_zero_distance() {
    let mut m = LmCategory::new(vec!["A".into(), "B".into()]);
    m.add_transition("A", "B", 1.0).unwrap();
    m.add_transition("B", "A", 1.0).unwrap();

    let space = m
        .enriched_space()
        .expect("prob = 1.0 two-cycle must not exhaust the BFS frontier cap");
    let d_ab = space.distance(&0, &1).0;
    let d_ba = space.distance(&1, &0).0;
    assert_eq!(d_ab, 0.0, "d(A, B) must be −ln 1 = 0");
    assert_eq!(d_ba, 0.0, "d(B, A) must be −ln 1 = 0");
}

// ---------------------------------------------------------------------------
// from_traces — corpus MLE constructor (BTV 2021 §2.2 Def 4 + Eq 8, #53)
// ---------------------------------------------------------------------------

/// Build a name → `NodeId` (position in `objects()`) map for reading
/// `enriched_space()` distances by state name.
fn name_index(m: &LmCategory) -> std::collections::HashMap<&str, usize> {
    m.objects()
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect()
}

/// Hand-checked small corpus: exact π values, objects, and terminating set.
///
/// Corpus `["the","cat"], ["the","dog"], ["a","cat"]`. Prefix counts:
/// `N(ε)=3, N("the")=2, N("the cat")=1, N("the dog")=1, N("a")=1,
/// N("a cat")=1`.
#[test]
fn from_traces_hand_checked_corpus() {
    let traces = vec![vec!["the", "cat"], vec!["the", "dog"], vec!["a", "cat"]];
    let m = LmCategory::from_traces(&traces).expect("valid corpus");

    // Objects: ascending lexicographic order, ε ("") first.
    assert_eq!(
        m.objects(),
        &[
            String::new(),
            "a".to_owned(),
            "a cat".to_owned(),
            "the".to_owned(),
            "the cat".to_owned(),
            "the dog".to_owned(),
        ]
    );

    // π(child | parent) = N(child) / N(parent).
    let pi = |from: &str, to: &str| m.transitions().get(from).and_then(|r| r.get(to)).copied();
    assert_eq!(pi("", "the"), Some(2.0 / 3.0)); // 2/3
    assert_eq!(pi("", "a"), Some(1.0 / 3.0)); // 1/3
    assert_eq!(pi("the", "the cat"), Some(0.5)); // 1/2
    assert_eq!(pi("the", "the dog"), Some(0.5)); // 1/2
    assert_eq!(pi("a", "a cat"), Some(1.0)); // 1/1

    // Terminating = prefixes where some trace ends exactly.
    let mut term: Vec<&String> = m.terminating().iter().collect();
    term.sort();
    assert_eq!(
        term,
        vec![
            &"a cat".to_owned(),
            &"the cat".to_owned(),
            &"the dog".to_owned(),
        ]
    );
}

/// BTV 2021 §2.2 Eq (8) on a `from_traces` tree, read through
/// `enriched_space` as distances: `d(x, z) == d(x, y) + d(y, z)` within
/// `1e-12` for every realized composable pair. Corpus depth ≥ 3.
///
/// Scope: this verifies the *tree metric additivity* half of Eq (8) — on a
/// prefix tree the path is unique, so additivity holds for any edge values.
/// The other half (the MLE values themselves, whose telescoping makes Eq (8)
/// an equality of probabilities) is pinned by
/// `from_traces_hand_checked_corpus`.
#[test]
fn from_traces_eq8_chain_rule_exact() {
    let traces = vec![vec!["a", "b", "c"], vec!["a", "b", "d"], vec!["a", "e"]];
    let m = LmCategory::from_traces(&traces).expect("valid corpus");
    let space = m.enriched_space().expect("tree table has no cycles");
    let idx = name_index(&m);
    let d = |from: &str, to: &str| space.distance(&idx[from], &idx[to]).0;

    let mut pairs_checked = 0usize;
    for (x, row) in m.transitions() {
        for y in row.keys() {
            let Some(y_row) = m.transitions().get(y) else {
                continue;
            };
            for z in y_row.keys() {
                let lhs = d(x, z);
                let rhs = d(x, y) + d(y, z);
                assert!(
                    (lhs - rhs).abs() < 1e-12,
                    "Eq (8) violated for {x:?} → {y:?} → {z:?}: \
                     d(x,z)={lhs} != d(x,y)+d(y,z)={rhs}"
                );
                pairs_checked += 1;
            }
        }
    }
    assert!(
        pairs_checked >= 1,
        "corpus should realize at least one composable pair"
    );
}

/// Terminal mass at a branching-and-terminating state equals `#ends / N`.
///
/// Corpus `["the"], ["the","cat"]`: `N("the")=2`, one trace ends at `"the"`,
/// one continues to `"the cat"`. Row `"the"` = `{"the cat": 1/2}`, so
/// `1 − Σ row = 0.5 = #ends("the")/N("the") = 1/2`.
#[test]
fn from_traces_terminal_mass_matches_ends_over_n() {
    let traces = vec![vec!["the"], vec!["the", "cat"]];
    let m = LmCategory::from_traces(&traces).expect("valid corpus");

    let row_sum: f64 = m
        .transitions()
        .get("the")
        .map(|r| r.values().sum())
        .unwrap_or(0.0);
    let terminal_mass = 1.0 - row_sum;
    assert!(
        (terminal_mass - 0.5).abs() < 1e-12,
        "terminal mass at \"the\" should be #ends/N = 1/2, got {terminal_mass}"
    );
    assert!(m.terminating().contains("the"));
}

/// `magnitude(t)` smoke: a corpus builds a tree (acyclic) so ζ_t is
/// invertible and magnitude is a finite `Ok` value.
#[test]
fn from_traces_magnitude_smoke() {
    let traces = vec![vec!["the", "cat"], vec!["the", "dog"], vec!["a", "cat"]];
    let m = LmCategory::from_traces(&traces).expect("valid corpus");
    let mag = m.magnitude(2.0).expect("tree table ⇒ invertible ζ_2");
    assert!(
        mag.is_finite(),
        "magnitude at t=2.0 should be finite, got {mag}"
    );
}

/// Empty corpus is rejected.
#[test]
fn from_traces_empty_corpus_errors() {
    let traces: Vec<Vec<&str>> = Vec::new();
    assert!(LmCategory::from_traces(&traces).is_err());
}

/// A token containing an internal space is rejected (state-name collision).
#[test]
fn from_traces_token_with_space_errors() {
    let traces = vec![vec!["a b"]];
    assert!(LmCategory::from_traces(&traces).is_err());
}

/// An empty-string token is rejected.
#[test]
fn from_traces_empty_string_token_errors() {
    let traces = vec![vec!["a", ""]];
    assert!(LmCategory::from_traces(&traces).is_err());
}

/// An empty trace in a non-empty corpus makes ε terminating without panic.
#[test]
fn from_traces_empty_trace_marks_epsilon_terminating() {
    let traces = vec![vec![], vec!["a"]];
    let m = LmCategory::from_traces(&traces).expect("empty trace is a valid observation");
    assert!(
        m.terminating().contains(""),
        "an empty trace should mark ε (\"\") terminating"
    );
    // ε is an object; the non-empty trace realizes ε → "a" with π = 1/2.
    assert!(m.objects().iter().any(|o| o.is_empty()));
    assert_eq!(m.transitions().get("").and_then(|r| r.get("a")), Some(&0.5));
}

// ---------------------------------------------------------------------------
// Intro magnitude-bounds proptest — sanity check on random LMs
// ---------------------------------------------------------------------------

/// Construct a random tree-shaped `n`-state LM with strictly forward
/// transitions: state `i` may only transition to states `j > i`.
///
/// State naming: `s0, …, s{n-1}`. The last state `s{n-1}` is the only
/// terminating state. This mirrors the BV 2025 §2.15 prefix-poset shape
/// (forward-only, no cycles, single root); the intro bounds hold in this regime.
fn build_random_tree_lm(n: usize, seed: u64) -> LmCategory {
    // `| 1` seed prep stays at the call site (#33).
    let mut rng = Lcg::new(seed | 1);

    let names: Vec<String> = (0..n).map(|i| format!("s{i}")).collect();
    let mut m = LmCategory::new(names.clone());
    m.mark_terminating(&names[n - 1]);

    // Forward chain: each non-terminal state distributes mass over later
    // states + leaves a non-trivial terminal mass (renormalize to 1 below).
    for i in 0..(n - 1) {
        let mut raw: Vec<f64> = Vec::with_capacity(n - i - 1);
        for _ in (i + 1)..n {
            raw.push(rng.next_f64());
        }
        let total: f64 = raw.iter().sum();
        if total < 1e-9 {
            continue;
        }
        for (k, &r) in raw.iter().enumerate() {
            let p = r / total;
            if p > 0.0 {
                // Skip in case proptest happens to land i+1+k == i (cannot
                // happen with the strict `i+1+k > i` indexing, but defensive
                // against future refactors).
                if names[i] != names[i + 1 + k] {
                    m.add_transition(&names[i], &names[i + 1 + k], p).unwrap();
                }
            }
        }
    }
    m
}

proptest! {
    /// BV 2025 p.4 intro bounds (derivable from Prop 3.10): `#T(⊥) ≤ Mag(tM) ≤ #ob(M)` for `t ≥ 1`.
    ///
    /// At `t = 1` magnitude exactly equals `#T(⊥) + Σ entropies`, but the
    /// general bound argument is monotone and the upper bound `#ob(M)` is
    /// tight only as `t → ∞`. We test with `t ∈ {1.5, 2.0, 3.0}` to stay
    /// well inside the regime where ζ_t is invertible and the bounds apply.
    #[test]
    fn mag_bounds_intro(
        n in 2usize..=4,
        seed in any::<u64>(),
    ) {
        let m = build_random_tree_lm(n, seed);
        let n_term = m.terminating().len() as f64;
        let n_obj = m.objects().len() as f64;
        for &t in &[1.5_f64, 2.0, 3.0] {
            let Ok(mag) = m.magnitude(t) else {
                // Singular zeta on the random fixture — accept and skip.
                continue;
            };
            prop_assert!(
                mag >= n_term - 1e-6,
                "intro lower bound violated at t={t}: mag={mag}, #T(⊥)={n_term}"
            );
            prop_assert!(
                mag <= n_obj + 1e-6,
                "intro upper bound violated at t={t}: mag={mag}, #ob={n_obj}"
            );
        }
    }
}
