//! Coalition diversity `Mag(tA|members)` over a 5-agent coupling table (#22).
//!
//! The §IV.5 (gemini-spec) reading of a coalition: agents are objects of an
//! enriched category, inter-agent couplings are `[0,1]`-valued hom-objects
//! (Bradley–Terilla–Vlassopoulos 2021, arXiv:2106.07890), and a coalition is a
//! **member-restricted, max-product-closed** cospan-weighted subgraph. Its
//! diversity is the magnitude of the induced Lawvere metric space (BV 2025 §3.5
//! Eq 7 / Thm 3.10, arXiv:2501.06662), read at several `t`:
//!
//! - `t = 1`  — canonical/default arm: Shannon-diversity reading.
//! - `t = 2`  — collision-probability proxy.
//! - `t = 10` — approaching the cardinality-like limit.
//!
//! This is the *alternative* to message-passing: coupling strengths are data
//! (however obtained), not exchanged messages. No SurrealDB, no tokio, no async.
//!
//! Reuses the `mock_coalition` agent names/weights so readers can cross-reference.
//! Two overlapping coalitions are formed — `{alice, bob, carol}` and
//! `{carol, dan, eve}` — and the restrict-then-close pin is demonstrated: a pair
//! coupled only through a non-member shows `∞` (they do not count).

use catgraph_magnitude::{
    Coalition, HomMap, UnitInterval, coalition_magnitude, coalition_magnitude_from_couplings,
};

/// 5 agents: alice, bob, carol, dan, eve.
const AGENTS: [&str; 5] = ["alice", "bob", "carol", "dan", "eve"];

/// Directed inter-agent couplings (probabilities), reusing the `mock_coalition`
/// weights:
///
/// ```text
/// alice → bob   0.7      bob   → carol 0.5      carol → dan 0.6
/// alice → eve   0.2      bob   → dan   0.4      dan   → eve 0.3
/// eve   → alice 0.1  (cycle)
/// ```
const COUPLINGS: [(&str, &str, f64); 7] = [
    ("alice", "bob", 0.7),
    ("alice", "eve", 0.2),
    ("bob", "carol", 0.5),
    ("bob", "dan", 0.4),
    ("carol", "dan", 0.6),
    ("dan", "eve", 0.3),
    ("eve", "alice", 0.1),
];

/// Build the 5-agent `HomMap<&str, UnitInterval>` coupling category.
fn build_category() -> HomMap<&'static str, UnitInterval> {
    let mut cat = HomMap::new(AGENTS.to_vec());
    for &(from, to, p) in &COUPLINGS {
        cat.set_hom(from, to, UnitInterval::new(p).unwrap());
    }
    cat
}

/// Print the diversity profile `Mag(tA|members)` at t ∈ {1, 2, 10}.
fn diversity_profile(label: &str, coalition: &Coalition<&'static str>) {
    println!(
        "=== Coalition {label}: {} members, {} effective ===",
        coalition.len(),
        coalition.effective_members()
    );
    for t in [1.0_f64, 2.0, 10.0] {
        let mag = coalition_magnitude(coalition, t).expect("t > 0 ⇒ invertible");
        let arm = match t {
            x if (x - 1.0).abs() < 1e-9 => "canonical (Shannon via f'(1))",
            x if (x - 2.0).abs() < 1e-9 => "collision proxy",
            _ => "cardinality-like limit",
        };
        println!("  Mag(t = {t:>4})  =  {mag:.6}   [{arm}]");
    }
    println!();
}

fn main() {
    let cat = build_category();

    // Two overlapping coalitions (both share carol).
    let coalition_abc =
        Coalition::from_enriched(&cat, &["alice", "bob", "carol"]).expect("valid members");
    let coalition_cde =
        Coalition::from_enriched(&cat, &["carol", "dan", "eve"]).expect("valid members");

    println!("BTV 2021 [0,1] enrichment + BV 2025 §3.5 magnitude — coalition diversity\n");
    diversity_profile("{alice, bob, carol}", &coalition_abc);
    diversity_profile("{carol, dan, eve}", &coalition_cde);

    // -----------------------------------------------------------------------
    // Restrict-then-close pin. In {alice, carol} the only alice→carol coupling
    // is mediated through bob, a NON-member. So it does not count: A(alice,
    // carol) = 0 ⇒ d = ∞.
    // -----------------------------------------------------------------------
    println!("=== Restrict-then-close pin: coalition {{alice, carol}} ===");
    let coalition_ac = Coalition::from_enriched(&cat, &["alice", "carol"]).expect("valid members");
    let space_ac = coalition_ac
        .as_weighted_cospan()
        .clone()
        .into_metric_space();
    // Local index: alice = 0, carol = 1.
    let d_ac = space_ac.distance(&0, &1);
    println!(
        "  d(alice, carol) = {}   (alice→carol only via bob, a non-member ⇒ dropped)",
        if d_ac.0.is_infinite() {
            "∞".to_owned()
        } else {
            format!("{:.6}", d_ac.0)
        }
    );
    println!();

    // -----------------------------------------------------------------------
    // Plain-data entry point (seed of #23 `coalition_value`): same answer.
    // -----------------------------------------------------------------------
    let idx = |name: &str| AGENTS.iter().position(|a| *a == name).unwrap();
    let couplings_idx: Vec<(usize, usize, f64)> = COUPLINGS
        .iter()
        .map(|&(f, t, p)| (idx(f), idx(t), p))
        .collect();
    let members_abc = [idx("alice"), idx("bob"), idx("carol")];
    let via_data =
        coalition_magnitude_from_couplings(&AGENTS, &couplings_idx, &members_abc, 1.0).unwrap();

    // -----------------------------------------------------------------------
    // Assertions — this example doubles as an integration check.
    // -----------------------------------------------------------------------

    // Restrict-then-close: alice↔carol are ∞-separated inside {alice, carol}.
    assert!(
        d_ac.0.is_infinite(),
        "non-member (bob)-mediated coupling must not count"
    );

    // Plain-data path agrees with the enriched-category path for {alice,bob,carol}.
    let via_cat = coalition_magnitude(&coalition_abc, 1.0).unwrap();
    assert!(
        (via_data - via_cat).abs() < 1e-9,
        "plain-data path {via_data} != enriched path {via_cat}"
    );

    // Diversity bounds (BV 2025 p.4): #members-normalized, 1 ≤ Mag(t) ≤ n for t ≥ 1.
    // These couplings never reach 1.0, so no members collapse (effective == full).
    for coalition in [&coalition_abc, &coalition_cde] {
        assert_eq!(
            coalition.effective_members(),
            coalition.len(),
            "no perfect coupling ⇒ no skeletal collapse"
        );
        let n = coalition.len() as f64;
        for t in [1.0_f64, 2.0, 10.0] {
            let mag = coalition_magnitude(coalition, t).unwrap();
            assert!(
                mag >= 1.0 - 1e-9 && mag <= n + 1e-9,
                "Mag({t}) = {mag} out of bounds [1, {n}]"
            );
        }
    }

    // Skeletalization: a perfectly-coupled pair (dana ⇄ dane at 1.0) is ONE
    // effective agent — magnitude 1, where a naive ζ would be singular.
    let clones = coalition_magnitude_from_couplings(
        &["dana", "dane"],
        &[(0, 1, 1.0), (1, 0, 1.0)],
        &[0, 1],
        1.0,
    )
    .expect("perfectly-coupled coalition is well-defined after skeletalization");
    assert!(
        (clones - 1.0).abs() < 1e-9,
        "perfectly-coupled pair Mag(1) = {clones}, expected 1"
    );

    println!("All coalition_magnitude assertions hold.");
}
