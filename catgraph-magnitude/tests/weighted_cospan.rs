//! Phase 6A.1 acceptance tests for [`WeightedCospan<Lambda, Q>`].
//!
//! Three test arms:
//!
//! 1. **Round-trip (proptest, `Q = F64Rig`)** —
//!    `from_cospan_uniform(c, 1.0).as_cospan()` is structurally equal to the
//!    original cospan `c`.
//! 2. **`set_weight` idempotence (proptest, `Q = F64Rig`)** — setting the
//!    same `(i, j) → w` twice yields the same weight as setting it once.
//! 3. **`from_unit_interval` spot check** — `into_metric_space()` for a
//!    fixed 3-node `Q = UnitInterval` fixture computes `d = -ln π` per the
//!    Lawvere embedding.

use catgraph::cospan::Cospan;
use catgraph_magnitude::weighted_cospan::{NodeId, WeightedCospan};
use catgraph_magnitude::{F64Rig, Tropical, UnitInterval};
use deep_causality_num::{One, Zero};
use proptest::prelude::*;

/// Strategy: a small cospan over `Lambda = char`.
///
/// Generated as `(left, right, middle)` where:
/// - `middle` is a `Vec<char>` of length 1..=8,
/// - `left` is a `Vec<usize>` of length 0..=6 with each value < `middle.len()`,
/// - `right` is a `Vec<usize>` of length 0..=6 with each value < `middle.len()`.
fn small_cospan_strategy() -> impl Strategy<Value = Cospan<char>> {
    proptest::collection::vec(any::<char>(), 1..=8).prop_flat_map(|middle| {
        let m = middle.len();
        let left = proptest::collection::vec(0usize..m, 0..=6);
        let right = proptest::collection::vec(0usize..m, 0..=6);
        (Just(middle), left, right)
            .prop_map(|(middle, left, right)| Cospan::new(left, right, middle))
    })
}

/// Structural equality on `Cospan<Lambda>` via public accessors.
/// `Cospan` does not implement `PartialEq`, so we compare leg + middle slices.
fn cospan_eq<L: Eq + std::fmt::Debug + Copy>(a: &Cospan<L>, b: &Cospan<L>) -> bool {
    a.left_to_middle() == b.left_to_middle()
        && a.right_to_middle() == b.right_to_middle()
        && a.middle() == b.middle()
}

proptest! {
    #[test]
    fn from_cospan_uniform_roundtrip(c in small_cospan_strategy()) {
        let original = c.clone();
        let wc = WeightedCospan::from_cospan_uniform(c, F64Rig::one());
        prop_assert!(cospan_eq(wc.as_cospan(), &original));
    }

    #[test]
    fn set_weight_idempotent(
        c in small_cospan_strategy(),
        i in 0usize..8,
        j in 0usize..8,
        w in -10.0f64..10.0,
    ) {
        let mut wc_once = WeightedCospan::from_cospan_uniform(c.clone(), F64Rig::zero());
        let mut wc_twice = WeightedCospan::from_cospan_uniform(c, F64Rig::zero());

        wc_once.set_weight(i, j, F64Rig(w));
        wc_twice.set_weight(i, j, F64Rig(w));
        wc_twice.set_weight(i, j, F64Rig(w));

        prop_assert_eq!(wc_once.weight(i, j), wc_twice.weight(i, j));
    }
}

/// Fixed 3-node `Q = UnitInterval` fixture exercising the `-ln π` embedding.
///
/// Cospan structure: middle = `['x', 'y', 'z']`, every left port maps to
/// every middle index (so all `(i, j)` pairs are implied edges). After
/// uniform-construction with `prob = 1.0`, we override two probabilities and
/// verify the `LawvereMetricSpace<NodeId>` distances match `-ln(prob)`.
#[test]
fn into_metric_space_matches_minus_ln_pi() {
    // 3-node apex; left and right legs both target every node, so every
    // (i, j) pair is an implied edge.
    let middle = vec!['x', 'y', 'z'];
    let left = vec![0, 1, 2];
    let right = vec![0, 1, 2];
    let cospan = Cospan::new(left, right, middle);

    let mut wc = WeightedCospan::from_cospan_uniform(cospan, UnitInterval::one());

    // Override two specific edges. Use dyadic fractions to dodge IEEE-754
    // rounding noise in the assertion.
    let half = UnitInterval::new(0.5).unwrap();
    let quarter = UnitInterval::new(0.25).unwrap();
    wc.set_weight(0, 1, half);
    wc.set_weight(1, 2, quarter);
    // Force a "no edge" by leaving (0, 2) at uniform 1.0 (will give d = 0).
    // Force an explicit unreachable by setting (2, 0) = 0.0 → d = +∞.
    wc.set_weight(2, 0, UnitInterval::zero());

    let lms = wc.into_metric_space();

    let zero: NodeId = 0;
    let one_id: NodeId = 1;
    let two: NodeId = 2;

    // d(0, 1) = -ln(0.5)
    let d01 = lms.distance(&zero, &one_id);
    assert!(
        (d01.0 - -(0.5f64.ln())).abs() < 1e-12,
        "d(0,1) = {} expected {}",
        d01.0,
        -(0.5f64.ln())
    );

    // d(1, 2) = -ln(0.25)
    let d12 = lms.distance(&one_id, &two);
    assert!(
        (d12.0 - -(0.25f64.ln())).abs() < 1e-12,
        "d(1,2) = {} expected {}",
        d12.0,
        -(0.25f64.ln())
    );

    // d(0, 2) = -ln(1.0) = 0
    let d02 = lms.distance(&zero, &two);
    assert!(d02.0.abs() < 1e-12, "d(0,2) = {} expected 0", d02.0);

    // d(2, 0) = -ln(0.0) = +∞
    let d20 = lms.distance(&two, &zero);
    assert!(
        d20.0.is_infinite() && d20.0 > 0.0,
        "d(2,0) = {} expected +∞",
        d20.0
    );

    // Self-distances start at +∞ (since UnitInterval::zero() returned for
    // unset entries → -ln(0) = +∞). Caller is responsible for setting
    // d(x, x) = 0 if the identity axiom matters; we verify the documented
    // behaviour rather than the axiom.
    let d00 = lms.distance(&zero, &zero);
    let expected_d00 = -(1.0f64.ln()); // 0.0, since uniform_weight = 1.0
    assert!(
        (d00.0 - expected_d00).abs() < 1e-12,
        "d(0,0) = {} expected {}",
        d00.0,
        expected_d00
    );
}

/// Sanity: `weight` on an absent edge returns `Q::zero()` for `Q = Tropical`
/// (i.e. `Tropical(+∞)`) — matches the rig "no edge" convention.
#[test]
fn absent_edge_weight_is_zero_tropical() {
    let cospan = Cospan::<char>::empty();
    let wc = WeightedCospan::<char, Tropical>::from_cospan_uniform(cospan, Tropical::one());
    let w = wc.weight(0, 1);
    // Tropical::zero() = Tropical(+∞)
    assert!(w.0.is_infinite() && w.0 > 0.0);
}

/// v0.1.1: `into_validated_metric_space` returns `Ok` when the resulting
/// metric satisfies the triangle inequality. Three-node fixture with
/// dyadic probabilities, identity-axiom diagonal seeded.
#[test]
fn into_validated_metric_space_accepts_valid_lm_v0_1_1() {
    let middle = vec!['x', 'y', 'z'];
    let cospan = Cospan::new(vec![0, 1, 2], vec![0, 1, 2], middle);
    let one = UnitInterval::one();
    let mut wc = WeightedCospan::from_cospan_uniform(cospan, one);
    // Identity axiom: d(i, i) = 0 ⇒ prob(i, i) = 1.0 (uniform default).
    // Triangle-OK three-edge fixture: prob(0,1)=0.5, prob(1,2)=0.5,
    // prob(0,2)=0.25 — chain equality holds (-ln 0.25 = 2 * -ln 0.5).
    let half = UnitInterval::new(0.5).unwrap();
    let quarter = UnitInterval::new(0.25).unwrap();
    wc.set_weight(0, 1, half);
    wc.set_weight(1, 2, half);
    wc.set_weight(0, 2, quarter);

    let lms = wc.into_validated_metric_space();
    assert!(lms.is_ok(), "valid LM should pass validation, got {lms:?}");
}

/// v0.1.1: `into_validated_metric_space` returns `Err` on a triangle-
/// inequality violation. Construct an LM where d(0, 2) is much smaller than
/// d(0, 1) + d(1, 2) — i.e. prob(0, 2) is much larger than prob(0, 1) *
/// prob(1, 2), which would imply a "shortcut" path the chain product
/// cannot account for under tree-additivity semantics. **Note:** the
/// `−ln π` embedding maps prob `1.0` to distance `0` and prob `0.0` to
/// `+∞`, so to trigger d(0, 2) > d(0, 1) + d(1, 2) we need prob(0, 2) <
/// prob(0, 1) * prob(1, 2). Using prob(0, 1) = prob(1, 2) = 0.5,
/// chain product = 0.25; setting prob(0, 2) = 0.1 < 0.25 violates the
/// inequality (d(0, 2) = -ln(0.1) > 2 * -ln(0.5)).
#[test]
fn into_validated_metric_space_rejects_triangle_violation_v0_1_1() {
    let middle = vec!['x', 'y', 'z'];
    let cospan = Cospan::new(vec![0, 1, 2], vec![0, 1, 2], middle);
    let one = UnitInterval::one();
    let mut wc = WeightedCospan::from_cospan_uniform(cospan, one);
    let half = UnitInterval::new(0.5).unwrap();
    let tenth = UnitInterval::new(0.1).unwrap();
    wc.set_weight(0, 1, half);
    wc.set_weight(1, 2, half);
    wc.set_weight(0, 2, tenth);

    let lms = wc.into_validated_metric_space();
    assert!(
        lms.is_err(),
        "triangle violation should be rejected, got {lms:?}"
    );
}

/// Sanity: `from_cospan_with_weights` is honoured per-pair.
#[test]
#[allow(clippy::cast_precision_loss)]
fn from_cospan_with_weights_per_pair() {
    let middle = vec!['a', 'b'];
    let cospan = Cospan::new(vec![0, 1], vec![0, 1], middle);
    let wc = WeightedCospan::from_cospan_with_weights(cospan, |i, j| F64Rig((i * 10 + j) as f64));
    assert_eq!(wc.weight(0, 0), F64Rig(0.0));
    assert_eq!(wc.weight(0, 1), F64Rig(1.0));
    assert_eq!(wc.weight(1, 0), F64Rig(10.0));
    assert_eq!(wc.weight(1, 1), F64Rig(11.0));
}
