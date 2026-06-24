//! V-enriched categories + Lawvere metric spaces — six demonstrations covering
//! the enrichment substrate from `enriched.rs` and `lawvere_metric.rs`.
//!
//! # Paper anchors
//!
//! - **F&S 2018 Seven Sketches** (`arXiv:1803.05316`) — §1.1 / §2.4 (preorders
//!   and enriched categories); Rough Def 4.51 (V-enriched categories over a
//!   commutative monoidal poset).
//! - **Lawvere 1973** — *Metric Spaces, Generalized Logic, and Closed
//!   Categories* — the foundational identification of metric spaces with
//!   `Tropical`-enriched categories: distances `d: T × T → [0, ∞]` enrich over
//!   `([0, ∞], min, +)` (the `Tropical` rig). Identity `d(x, x) = 0` is the
//!   multiplicative identity in min-plus; triangle inequality
//!   `d(x, z) ≤ d(x, y) + d(y, z)` is `Tropical` composition.
//! - **BTV 2021** (`arXiv:2106.07890`) — language categories enriched over
//!   `UnitInterval` (Viterbi semiring); the `-ln π` embedding into `Tropical`
//!   is the substrate for BV 2025 magnitude. Demonstrated here in §4 via
//!   [`LawvereMetricSpace::from_unit_interval`].
//!
//! # Narrative arc
//!
//! 1. Dual construction paths to the same metric space (`new` + `set_distance`
//!    loop vs `from_distances` iterator) — verifies they agree pointwise.
//! 2. Triangle inequality verification — positive case + deliberately broken
//!    case.
//! 3. `size()` + `objects()` iteration over the underlying point set.
//! 4. Base-change `UnitInterval → Tropical` via `from_unit_interval` (the
//!    `-ln π` BTV21 recipe).
//! 5. Closure-driven construction via `from_distance_fn`.
//! 6. `HomMap<O, V>` + a custom `EnrichedCategory<Tropical>` wrapper — shows
//!    that the trait is the unifying abstraction over both `LawvereMetricSpace`
//!    and `HomMap`.
//!
//! Run: `cargo run -p catgraph-applied --example enriched_lawvere --release`

#![allow(clippy::float_cmp)] // f64 ops here are exact-by-construction.
#![allow(clippy::too_many_lines)] // 6 narrative sections in one main; idiomatic for examples.

use catgraph_applied::{
    enriched::{EnrichedCategory, HomMap},
    lawvere_metric::LawvereMetricSpace,
    rig::{Tropical, UnitInterval},
};
use deep_causality_num::One;

fn main() {
    println!("=== V-enriched categories + Lawvere metric spaces ===\n");

    // -------- §1. Dual construction paths converging to the same metric --------
    //
    // We build a 4-point fully-connected metric on `0..4` realising the
    // 1-D integer-line geometry d(i, j) = |i - j| (symmetric, triangular).
    // All 16 entries are set explicitly so the triangle inequality
    // verifier in §2 holds without relying on the unset-defaults-to-`+∞`
    // convention (which only satisfies the inequality on graphs that
    // genuinely have no shorter path through intermediate nodes).
    println!("§1. Two paths to the same `LawvereMetricSpace<usize>`");

    // Helper closure: line-distance d(i, j) = |i - j| as Tropical.
    #[allow(clippy::cast_precision_loss)]
    let line_d = |i: usize, j: usize| -> Tropical { Tropical(((i as f64) - (j as f64)).abs()) };

    // Path A — imperative `new` + `set_distance` loop. Common when distances
    // arrive from a stateful source (e.g. streaming observations).
    let mut space_a = LawvereMetricSpace::<usize>::new(vec![0, 1, 2, 3]);
    for i in 0..4 {
        for j in 0..4 {
            space_a.set_distance(i, j, line_d(i, j));
        }
    }

    // Path B — `from_distances` iterator. Common when distances arrive as a
    // precomputed table.
    let space_b = LawvereMetricSpace::<usize>::from_distances(
        vec![0, 1, 2, 3],
        (0..4).flat_map(|i| (0..4).map(move |j| ((i, j), line_d(i, j)))),
    );

    for i in 0..4 {
        for j in 0..4 {
            let da = space_a.distance(&i, &j);
            let db = space_b.distance(&i, &j);
            assert_eq!(da, db, "paths disagree at d({i}, {j})");
        }
    }
    println!("  Paths A and B agree pointwise on all 16 entries (d(i, j) = |i - j|).\n");

    // -------- §2. Triangle inequality — positive + defensive negative -------
    println!("§2. Triangle inequality verification");
    assert!(space_a.triangle_inequality_holds());
    println!("  space_a satisfies d(x, z) ≤ d(x, y) + d(y, z) for all triples.");

    // Construct a deliberately-broken metric: declare d(0, 2) = 5.0 while
    // d(0, 1) + d(1, 2) = 1.0 + 2.0 = 3.0. The check must return false.
    let mut broken = LawvereMetricSpace::<usize>::new(vec![0, 1, 2]);
    for i in 0..3 {
        broken.set_distance(i, i, Tropical::one());
    }
    broken.set_distance(0, 1, Tropical(1.0));
    broken.set_distance(1, 2, Tropical(2.0));
    broken.set_distance(0, 2, Tropical(5.0)); // > 1.0 + 2.0 — violates ≤.
    assert!(!broken.triangle_inequality_holds());
    println!("  Deliberately-broken metric (d(0,2)=5 > 1+2) correctly fails.\n");

    // -------- §3. `size()` + `objects()` iteration ------------------------
    println!("§3. Underlying point set");
    println!("  space_a.size()    = {}", space_a.size());
    print!("  space_a.objects() = [");
    for (i, o) in space_a.objects().iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{o}");
    }
    println!("]");
    assert_eq!(space_a.size(), 4);
    assert_eq!(space_a.objects(), &[0, 1, 2, 3]);
    println!();

    // -------- §4. Base-change `UnitInterval → Tropical` via `-ln π` --------
    //
    // BTV21 enriches a language category over the Viterbi semiring
    // `(UnitInterval, max, ·)`. BV25 magnitude operates over `Tropical`.
    // The bridge is `d(p) = -ln p`, a semiring *anti*-homomorphism: it sends
    // `·` to `+` and `1` to `0`. Concretely, p = e^{-1} ≈ 0.368 maps to d = 1.
    println!("§4. BTV21 base-change `UnitInterval → Tropical` via `-ln π`");

    let p_self = UnitInterval::new(1.0).expect("1.0 ∈ [0, 1]");
    let p_close = UnitInterval::new((-1.0_f64).exp()).expect("e^-1 ∈ [0, 1]"); // ≈ 0.368
    let p_far = UnitInterval::new(0.0).expect("0.0 ∈ [0, 1]");

    // Probability table for a 3-point metric: 0 ↔ 0 has prob 1 (identity);
    // 0 ↔ 1 has prob e^{-1}; 0 ↔ 2 has prob 0 (unreachable). All other entries
    // get the diagonal-on-identity rule + e^{-1} elsewhere for non-trivial
    // distances.
    let prob_space =
        LawvereMetricSpace::<usize>::from_unit_interval(vec![0_usize, 1, 2], |a, b| {
            if a == b {
                p_self
            } else if matches!((*a, *b), (0, 2) | (2, 0)) {
                p_far
            } else {
                p_close
            }
        });

    let d_self = prob_space.distance(&0, &0);
    let d_close = prob_space.distance(&0, &1);
    let d_far = prob_space.distance(&0, &2);
    // IEEE-754 quirk: `-ln(1.0) = -0.0`, not `+0.0`. `Tropical(-0.0)` prints
    // with the leading minus sign, but `assert_eq!(d_self, Tropical(0.0))`
    // passes because IEEE-754 `PartialEq` treats `-0.0 == +0.0` as `true`.
    // We name the quirk openly rather than normalize via `.abs()` — the goal
    // is to teach the reader the enrichment math AND its f64-substrate
    // realities. `Tropical(-0.0)` is semantically `0` (zero distance, identity
    // axiom satisfied); the sign carries no metric meaning.
    println!("  d(0, 0) = {d_self:?}    (p = 1 → d = -ln(1) = -0.0 ≡ 0 under IEEE-754)");
    println!("  d(0, 1) = {d_close:?}   (p = e^-1 → d = 1.0)");
    println!("  d(0, 2) = {d_far:?}      (p = 0 → d = +∞)");
    assert_eq!(d_self, Tropical(0.0));
    assert!((d_close.0 - 1.0).abs() < 1e-12, "expected -ln(e^-1) = 1");
    assert!(d_far.0.is_infinite() && d_far.0 > 0.0);
    println!();

    // -------- §5. Closure-driven construction via `from_distance_fn` -------
    //
    // 1-D integer-line metric: d(a, b) = |a - b|. Symmetric, triangular,
    // self-zero — a textbook Lawvere metric (in fact a classical metric here).
    // Scoped block so `const N` lives at the top of an inner scope (clippy
    // `items_after_statements` clean).
    {
        const N: usize = 5;
        println!("§5. `LawvereMetricSpace::from_distance_fn` closure constructor");

        #[allow(clippy::cast_precision_loss)]
        let line = LawvereMetricSpace::from_distance_fn(N, |a, b| ((a as f64) - (b as f64)).abs());
        println!("  5-point integer line d(a, b) = |a - b|:");
        for a in 0..N {
            print!("    d({a}, ·) =");
            for b in 0..N {
                print!(" {:.0}", line.distance(&a, &b).0);
            }
            println!();
        }
        assert!(line.triangle_inequality_holds());
        assert_eq!(line.distance(&0, &4), Tropical(4.0));
        println!("  triangle inequality holds; size = {}\n", line.size());
    }

    // -------- §6. `HomMap` + custom `EnrichedCategory<Tropical>` wrapper ----
    //
    // The enrichment substrate covers more than metric spaces. Here we build
    // a 3-object `HomMap<&'static str, Tropical>` directly, then wrap it in a
    // newtype to demonstrate that `EnrichedCategory<Tropical>` is the trait
    // through which both `LawvereMetricSpace` and `HomMap` are consumed.
    println!("§6. `HomMap<&'static str, Tropical>` + custom wrapper");

    let mut hm = HomMap::<&'static str, Tropical>::new(vec!["x", "y", "z"]);
    hm.set_hom("x", "y", Tropical(1.0));
    hm.set_hom("y", "z", Tropical(2.0));
    hm.set_hom("x", "z", Tropical(3.0)); // direct edge, equal to the via-y path.

    let wrap = TraceCategory(hm);

    let hxy = EnrichedCategory::<Tropical>::hom(&wrap, &"x", &"y");
    let hyz = EnrichedCategory::<Tropical>::hom(&wrap, &"y", &"z");
    let hxz = EnrichedCategory::<Tropical>::hom(&wrap, &"x", &"z");
    let id_x = EnrichedCategory::<Tropical>::id_hom(&wrap, &"x");
    let compose_xyz = EnrichedCategory::<Tropical>::compose_hom(&wrap, &"x", &"y", &"z");

    println!("  hom(x, y)               = {hxy:?}");
    println!("  hom(y, z)               = {hyz:?}");
    println!("  hom(x, z) (direct)      = {hxz:?}");
    println!("  id_hom(x)               = {id_x:?} (Tropical::one() = Tropical(0))");
    println!("  compose_hom(x, y, z)    = {compose_xyz:?} (= hom(x,y) · hom(y,z) = 1 + 2)");

    assert_eq!(id_x, Tropical::one()); // category identity axiom.
    assert_eq!(compose_xyz, Tropical(3.0)); // 1.0 + 2.0 in Tropical = 3.0.
    assert_eq!(compose_xyz, hxz); // composition matches the direct edge — the
    // category is "triangle-saturated" here.

    // Iterate via the trait method (UFCS — `wrap.0.objects()` would resolve
    // to `HomMap`'s `objects` if it had one; this routes through the trait).
    let names: Vec<&'static str> = EnrichedCategory::<Tropical>::objects(&wrap).collect();
    println!("  objects(): {names:?}");
    assert_eq!(names, vec!["x", "y", "z"]);

    // Cross-instance check: the metric space `space_a` from §1 is *also* an
    // `EnrichedCategory<Tropical>` — same trait, different concrete type.
    let cross: Tropical = EnrichedCategory::<Tropical>::hom(&space_a, &0, &1);
    assert_eq!(cross, Tropical(1.0));
    println!("  LawvereMetricSpace::hom(0, 1) via trait = {cross:?} (same trait, different impl)");

    println!("\nAll demonstrations green.");
}

/// Newtype wrapping a `HomMap` to show that any structure implementing
/// `EnrichedCategory<Tropical>` can stand in for a metric space wherever the
/// trait bound is required. The trait method dispatch goes through this
/// wrapper transparently.
struct TraceCategory(HomMap<&'static str, Tropical>);

impl EnrichedCategory<Tropical> for TraceCategory {
    type Object = &'static str;

    fn hom(&self, a: &Self::Object, b: &Self::Object) -> Tropical {
        self.0.hom(a, b)
    }

    fn objects(&self) -> Box<dyn Iterator<Item = Self::Object> + '_> {
        self.0.objects()
    }
}
