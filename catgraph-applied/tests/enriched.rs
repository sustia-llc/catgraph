//! Integration tests for [`catgraph_applied::enriched`] and
//! [`catgraph_applied::lawvere_metric`].
//!
//! The `EnrichedCategory<V>` trait + `HomMap<O, V>` concrete impl are covered
//! first (6 tests); then [`LawvereMetricSpace<T>`] over [`Tropical`] (4 tests).

use catgraph_applied::{
    enriched::{EnrichedCategory, HomMap},
    lawvere_metric::LawvereMetricSpace,
    rig::{F64Rig, Tropical, UnitInterval},
};

#[test]
fn hommap_unset_hom_returns_zero() {
    let objects = vec!['a', 'b'];
    let hm: HomMap<char, F64Rig> = HomMap::new(objects);
    assert_eq!(hm.hom(&'a', &'b'), F64Rig(0.0));
}

#[test]
fn hommap_set_hom_roundtrip() {
    let objects = vec!['a', 'b'];
    let mut hm: HomMap<char, F64Rig> = HomMap::new(objects);
    hm.set_hom('a', 'b', F64Rig(3.5));
    assert_eq!(hm.hom(&'a', &'b'), F64Rig(3.5));
    // Asymmetric: b→a still zero.
    assert_eq!(hm.hom(&'b', &'a'), F64Rig(0.0));
}

#[test]
fn id_hom_is_rig_one() {
    let hm: HomMap<char, UnitInterval> = HomMap::new(vec!['a']);
    assert_eq!(hm.id_hom(&'a'), UnitInterval::new(1.0).unwrap());
}

#[test]
fn compose_hom_multiplies_in_rig() {
    let mut hm: HomMap<char, F64Rig> = HomMap::new(vec!['a', 'b', 'c']);
    hm.set_hom('a', 'b', F64Rig(2.0));
    hm.set_hom('b', 'c', F64Rig(3.0));
    // Default compose_hom: hom(a,b) * hom(b,c) = 2 * 3 = 6.
    assert_eq!(hm.compose_hom(&'a', &'b', &'c'), F64Rig(6.0));
}

#[test]
fn tropical_enriched_is_shortest_path() {
    // Tropical (min, +): compose_hom gives a+b (sum of distances).
    // Tropical zero = +∞, so unset homs = +∞.
    let mut hm: HomMap<char, Tropical> = HomMap::new(vec!['a', 'b', 'c']);
    hm.set_hom('a', 'b', Tropical(3.0));
    hm.set_hom('b', 'c', Tropical(4.0));
    assert_eq!(hm.compose_hom(&'a', &'b', &'c'), Tropical(7.0));
}

#[test]
fn objects_iterator_preserves_order() {
    let hm: HomMap<char, F64Rig> = HomMap::new(vec!['x', 'y', 'z']);
    let collected: Vec<char> = hm.objects().collect();
    assert_eq!(collected, vec!['x', 'y', 'z']);
}

// ------------------------------------------------------------------
// LawvereMetricSpace tests
// ------------------------------------------------------------------

#[test]
fn lawvere_triangle_inequality_identity_space() {
    // Every point has d(x, y) = 0. Triangle holds trivially.
    let objects = vec!['a', 'b', 'c'];
    let mut m = LawvereMetricSpace::new(objects.clone());
    for a in &objects {
        for b in &objects {
            m.set_distance(*a, *b, Tropical(0.0));
        }
    }
    assert!(m.triangle_inequality_holds());
}

#[test]
fn lawvere_triangle_inequality_fails_on_violation() {
    // Set d(a,c) = 10 but d(a,b) + d(b,c) = 2 + 3 = 5 < 10 — violation.
    let objects = vec!['a', 'b', 'c'];
    let mut m = LawvereMetricSpace::new(objects);
    m.set_distance('a', 'b', Tropical(2.0));
    m.set_distance('b', 'c', Tropical(3.0));
    m.set_distance('a', 'c', Tropical(10.0));
    // Missing distances default to +∞, so many sums will be +∞ (and +∞ is
    // always ≥ anything), but the specific triple (a, b, c) violates.
    assert!(!m.triangle_inequality_holds());
}

#[test]
fn lawvere_from_unit_interval_roundtrip() {
    let objects = vec!['a', 'b'];
    let m = LawvereMetricSpace::<char>::from_unit_interval(objects, |a, b| {
        if a == b {
            UnitInterval::new(1.0).unwrap()
        } else {
            UnitInterval::new(0.5).unwrap()
        }
    });
    // d(a, a) = -ln(1) = 0.0; d(a, b) = -ln(0.5) ≈ 0.693.
    assert!((m.distance(&'a', &'a').0 - 0.0).abs() < 1e-9);
    assert!((m.distance(&'a', &'b').0 - (-0.5_f64.ln())).abs() < 1e-9);
}

#[test]
fn lawvere_enriched_category_impl() {
    // Verify LawvereMetricSpace implements EnrichedCategory<Tropical>
    // and delegates `hom` to `distance`.
    let objects = vec!['a', 'b'];
    let mut m = LawvereMetricSpace::new(objects);
    m.set_distance('a', 'b', Tropical(2.5));
    // hom via trait
    let d = EnrichedCategory::<Tropical>::hom(&m, &'a', &'b');
    assert_eq!(d, Tropical(2.5));
}

#[test]
#[allow(clippy::similar_names)]
fn lawvere_hom_diagonal_default_v0_5_4() {
    // hom(x, x) returns Tropical::one() = Tropical(0.0) when no
    // explicit diagonal entry was recorded (Lawvere identity-axiom default).
    let objects = vec!['a', 'b'];
    let m = LawvereMetricSpace::new(objects);
    // No set_distance calls.
    let d_aa = EnrichedCategory::<Tropical>::hom(&m, &'a', &'a');
    let d_bb = EnrichedCategory::<Tropical>::hom(&m, &'b', &'b');
    assert_eq!(
        d_aa,
        Tropical(0.0),
        "diagonal default should be Tropical(0.0)"
    );
    assert_eq!(d_bb, Tropical(0.0));
    // Off-diagonal unset entries still return Tropical(+∞).
    let d_ab = EnrichedCategory::<Tropical>::hom(&m, &'a', &'b');
    assert!(d_ab.0.is_infinite() && d_ab.0 > 0.0);
}

#[test]
fn lawvere_hom_diagonal_explicit_takes_precedence() {
    // An explicit diagonal entry is preserved by hom (no override).
    let objects = vec!['a'];
    let mut m = LawvereMetricSpace::new(objects);
    m.set_distance('a', 'a', Tropical(1.5));
    let d = EnrichedCategory::<Tropical>::hom(&m, &'a', &'a');
    assert_eq!(d, Tropical(1.5));
}

#[test]
fn lawvere_from_distances_v0_5_4() {
    // from_distances builds a metric space from an explicit
    // distance iterator.
    let objects = vec!['a', 'b', 'c'];
    let entries = vec![
        (('a', 'a'), Tropical(0.0)),
        (('a', 'b'), Tropical(2.0)),
        (('b', 'c'), Tropical(3.0)),
        (('a', 'c'), Tropical(4.0)),
    ];
    let m = LawvereMetricSpace::from_distances(objects, entries);
    assert_eq!(m.distance(&'a', &'a'), Tropical(0.0));
    assert_eq!(m.distance(&'a', &'b'), Tropical(2.0));
    assert_eq!(m.distance(&'b', &'c'), Tropical(3.0));
    assert_eq!(m.distance(&'a', &'c'), Tropical(4.0));
    // Triangle inequality holds: 4 ≤ 2 + 3.
    assert!(m.triangle_inequality_holds());
}

// ------------------------------------------------------------------
// Trait-contract test: compose_hom override path
// ------------------------------------------------------------------

/// A hom-table that overrides `compose_hom` to use `V::one()` regardless of the
/// stored hom values — verifies the default-override path is wired correctly.
/// Not semantically meaningful; this is a trait-contract test fixture.
struct IdentityCompose<O, V>
where
    O: Clone + Eq + std::hash::Hash,
    V: catgraph_applied::rig::Rig,
{
    inner: HomMap<O, V>,
}

impl<O, V> EnrichedCategory<V> for IdentityCompose<O, V>
where
    O: Clone + Eq + std::hash::Hash + 'static,
    V: catgraph_applied::rig::Rig + 'static,
{
    type Object = O;

    fn hom(&self, a: &Self::Object, b: &Self::Object) -> V {
        self.inner.hom(a, b)
    }

    // Override — ignore stored homs, always return the rig identity. The
    // unused `_a/_b/_c` params document the signature shape; the point of
    // this fixture is exactly that they are ignored.
    #[allow(clippy::used_underscore_binding)]
    fn compose_hom(&self, _a: &Self::Object, _b: &Self::Object, _c: &Self::Object) -> V {
        V::one()
    }

    fn objects(&self) -> Box<dyn Iterator<Item = Self::Object> + '_> {
        self.inner.objects()
    }
}

#[test]
fn compose_hom_override_wins_over_default() {
    let mut inner = HomMap::<char, F64Rig>::new(vec!['a', 'b', 'c']);
    inner.set_hom('a', 'b', F64Rig(2.0));
    inner.set_hom('b', 'c', F64Rig(3.0));
    let ec = IdentityCompose { inner };
    // Default would return 2.0 * 3.0 = 6.0; the override always returns 1.0.
    assert_eq!(ec.compose_hom(&'a', &'b', &'c'), F64Rig(1.0));
}
