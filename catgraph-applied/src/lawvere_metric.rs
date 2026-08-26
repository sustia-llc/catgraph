//! Lawvere metric spaces — categories enriched over [`Tropical`] (= `[0, ∞]`
//! with min-plus semiring structure). Pedagogical references: CTFP §28.5,
//! Lawvere 1973 *Metric spaces, generalized logic, and closed categories*.
//!
//! A Lawvere metric space `(T, d)` is a set T with a distance function
//! `d: T × T → [0, ∞]` satisfying:
//! - `d(x, x) = 0` (identity / reflexivity)
//! - `d(x, z) ≤ d(x, y) + d(y, z)` (triangle inequality)
//!
//! Unlike classical metric spaces, Lawvere metrics are not required to be
//! symmetric (`d(x, y) = d(y, x)` not assumed) or have `d(x, y) = 0 → x = y`
//! (non-separation allowed). This generalisation is what lets BTV 2021
//! (arXiv:2106.07890) use Lawvere metrics as the distance-valued hom of
//! language categories, and BV 2025 (arXiv:2501.06662) compute magnitude
//! over such enrichments.

use std::collections::HashMap;
use std::hash::Hash;

use crate::{
    enriched::EnrichedCategory,
    rig::{BaseChange, One, Tropical, UnitInterval, Zero},
};

/// A Lawvere metric space enriched over [`Tropical`].
///
/// Objects live in a `Vec<T>` (insertion-ordered); distances are stored in a
/// [`HashMap`] keyed by `(T, T)`. Unset distances default to
/// `Tropical::zero() = Tropical(+∞)` — "unreachable" under shortest-path
/// semantics.
#[derive(Debug, Clone)]
pub struct LawvereMetricSpace<T: Clone + Eq + Hash> {
    objects: Vec<T>,
    distances: HashMap<(T, T), Tropical>,
}

impl<T: Clone + Eq + Hash> LawvereMetricSpace<T> {
    /// Construct an empty metric space over a fixed object list. All
    /// distances start at `Tropical(+∞)`; use
    /// [`set_distance`](Self::set_distance) to populate.
    #[must_use]
    pub fn new(objects: Vec<T>) -> Self {
        Self {
            objects,
            distances: HashMap::new(),
        }
    }

    /// Set the directed distance from `a` to `b` (overwriting any prior
    /// value). Lawvere metrics are not required to be symmetric — setting
    /// `d(a, b)` does not set `d(b, a)`.
    pub fn set_distance(&mut self, a: T, b: T, d: Tropical) {
        self.distances.insert((a, b), d);
    }

    /// Construct a metric space from an explicit distance iterator.
    ///
    /// Pairs [`new`](Self::new) with a sequence of
    /// [`set_distance`](Self::set_distance) calls in one step.
    ///
    /// **Identity axiom.** This constructor does **not** seed the diagonal
    /// `d(x, x) = 0`. To satisfy the Lawvere metric identity axiom, callers
    /// must include `((x, x), Tropical(0.0))` for every object `x` in the
    /// iterator — or rely on the [`hom`](EnrichedCategory::hom) diagonal
    /// default (returns `Tropical::one() = Tropical(0.0)`
    /// when `a == b` and no entry was set).
    ///
    /// **Duplicate keys.** Last-write-wins, mirroring [`HashMap::insert`]
    /// semantics on a duplicate `(a, b)` pair.
    #[must_use]
    pub fn from_distances<I>(objects: Vec<T>, distances: I) -> Self
    where
        I: IntoIterator<Item = ((T, T), Tropical)>,
    {
        let mut space = Self::new(objects);
        for ((a, b), d) in distances {
            space.distances.insert((a, b), d);
        }
        space
    }

    /// Distance from `a` to `b`. Returns `Tropical(+∞)` if unset.
    ///
    /// Unset distance = `Tropical::zero()` = `Tropical(+∞)` in the min-plus
    /// semiring, i.e. "no edge" / "unreachable". Under min-plus multiplication
    /// (= real addition), `+∞ + anything = +∞`, so unset distances propagate
    /// through the triangle-inequality check and shortest-path composition.
    #[must_use]
    pub fn distance(&self, a: &T, b: &T) -> Tropical {
        self.distances
            .get(&(a.clone(), b.clone()))
            .copied()
            .unwrap_or_else(Tropical::zero)
    }

    /// Check the triangle inequality `d(x, z) ≤ d(x, y) + d(y, z)` over
    /// all triples `(x, y, z) ∈ objects³`.
    ///
    /// Returns `true` iff the inequality holds everywhere.
    ///
    /// Exact (zero-tolerance) check — equivalent to
    /// [`triangle_inequality_holds_within`](Self::triangle_inequality_holds_within)`(0.0)`.
    /// Callers whose distances are derived from `−ln` of floating-point
    /// products (where `−ln(a·b)` and `(−ln a)+(−ln b)` differ by ULPs) should
    /// prefer the tolerant variant — see its docs.
    ///
    /// # Complexity
    ///
    /// `O(n³)` where `n = self.objects.len()`. Intended for small finite
    /// spaces and test fixtures; not suitable for large metric spaces.
    #[must_use]
    pub fn triangle_inequality_holds(&self) -> bool {
        self.triangle_inequality_holds_within(0.0)
    }

    /// Check the triangle inequality `d(x, z) ≤ d(x, y) + d(y, z) + tol` over
    /// all triples `(x, y, z) ∈ objects³`, tolerating an absolute slack of
    /// `tol` in the distance (log) domain.
    ///
    /// Returns `true` iff `d(x, z) ≤ d(x, y) + d(y, z) + tol` everywhere; a
    /// triple is a violation iff `d(x, z) > d(x, y) + d(y, z) + tol`.
    ///
    /// The tolerance is for distances that are the `−ln` lift of `[0, 1]`-valued
    /// couplings (BTV 2021 §5): evaluating `−ln(π(x, y)·π(y, z))` as
    /// `(−ln π(x, y)) + (−ln π(y, z))` differs by a few ULPs, and on non-dyadic
    /// couplings (e.g. `1/3`, `2/5`) that noise can push `d(x, z)` a hair above
    /// the summed bound. A `tol` orders of magnitude above the ULP noise and
    /// orders below any genuine violation absorbs it.
    ///
    /// `tol` is an absolute slack in the distance / log domain (the same units
    /// as the stored `Tropical` values). Passing `tol = 0.0` reproduces the
    /// exact check of [`triangle_inequality_holds`](Self::triangle_inequality_holds).
    ///
    /// # Infinity semantics
    ///
    /// Preserved from the exact check. If `sum = +∞` (some leg unreachable),
    /// then `sum + tol = +∞`, so `d(x, z) > +∞` is never true and no triple is
    /// a violation. If `d(x, z) = +∞` while the sum is finite, `+∞ > finite`
    /// holds and the triple is a violation.
    ///
    /// # Complexity
    ///
    /// `O(n³)` where `n = self.objects.len()`. Intended for small finite
    /// spaces and test fixtures; not suitable for large metric spaces.
    #[must_use]
    #[allow(clippy::similar_names)]
    pub fn triangle_inequality_holds_within(&self, tol: f64) -> bool {
        for x in &self.objects {
            for y in &self.objects {
                for z in &self.objects {
                    let dxy = self.distance(x, y);
                    let dyz = self.distance(y, z);
                    let dxz = self.distance(x, z);
                    // Tropical multiplication is real addition, so
                    // `sum.0 = dxy.0 + dyz.0`.
                    let sum = dxy * dyz;
                    // Ordinary `≤` on the payload — distinct from the rig's
                    // additive order, which is `min`.
                    if dxz.0 > sum.0 + tol {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Number of objects.
    #[must_use]
    pub fn size(&self) -> usize {
        self.objects.len()
    }

    /// Read-only access to the underlying object list.
    ///
    /// # Name resolution
    ///
    /// This inherent method shares its name with [`EnrichedCategory::objects`],
    /// which returns a `Box<dyn Iterator<...>>`. Bare `space.objects()`
    /// resolves to *this* slice accessor; the iterator form needs UFCS:
    /// `EnrichedCategory::<Tropical>::objects(&space)`.
    #[must_use]
    pub fn objects(&self) -> &[T] {
        &self.objects
    }

    /// Build a Lawvere metric space from a [`UnitInterval`]-valued probability
    /// function via the `-ln π` embedding (see
    /// [`BaseChange<UnitInterval> for Tropical`](crate::rig::BaseChange)).
    ///
    /// Probabilities of `0` become `+∞` (unreachable); probabilities of `1`
    /// become `0` (self-identity distance).
    ///
    /// # Caller obligations
    ///
    /// To satisfy the Lawvere metric identity axiom (`d(x, x) = 0`), the
    /// caller must ensure `prob(x, x) = UnitInterval::new(1.0).unwrap()` for
    /// every object `x`. This constructor does not enforce the axiom — a
    /// closure that returns `prob(x, x) < 1.0` produces a structure where
    /// `d(x, x) > 0`, silently violating the axiom.
    /// [`triangle_inequality_holds`](Self::triangle_inequality_holds) checks
    /// only the triangle inequality; callers that want identity-axiom
    /// validation must assert it separately.
    ///
    /// # Iteration order
    ///
    /// The constructor iterates `objects × objects` in the `Vec<T>` order,
    /// not [`HashMap`] order — the `prob` closure sees a deterministic
    /// traversal.
    // Takes `Vec<T>` by value for symmetry with `new`, which stores the list.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_unit_interval<F>(objects: Vec<T>, prob: F) -> Self
    where
        F: Fn(&T, &T) -> UnitInterval,
    {
        let mut space = Self::new(objects);
        // Cloned once so the loops can mutate `space.distances` without
        // aliasing `space.objects`.
        let keys = space.objects.clone();
        for a in &keys {
            for b in &keys {
                let p = prob(a, b);
                let d = <Tropical as BaseChange<UnitInterval>>::base_change(p);
                space.distances.insert((a.clone(), b.clone()), d);
            }
        }
        space
    }
}

impl LawvereMetricSpace<usize> {
    /// Build a `usize`-indexed Lawvere metric space `(0..n)` from a distance
    /// closure. Equivalent to the `new(0..n) + set_distance` loop, but more
    /// ergonomic for fixtures.
    ///
    /// # Caller obligations
    ///
    /// - `f(a, a)` should return `0.0` for the Lawvere identity axiom.
    /// - The triangle inequality is the caller's responsibility; verify with
    ///   [`Self::triangle_inequality_holds`] if needed.
    pub fn from_distance_fn<F>(n: usize, f: F) -> Self
    where
        F: Fn(usize, usize) -> f64,
    {
        let mut space = Self::new((0..n).collect());
        for a in 0..n {
            for b in 0..n {
                space.set_distance(a, b, Tropical(f(a, b)));
            }
        }
        space
    }
}

impl<T> EnrichedCategory<Tropical> for LawvereMetricSpace<T>
where
    T: Clone + Eq + Hash + 'static,
{
    type Object = T;

    /// Hom-object `hom(a, b)` in the `Tropical`-enriched view of the metric
    /// space.
    ///
    /// **Diagonal default.** When `a == b`, returns `Tropical::one() =
    /// Tropical(0.0)` — the multiplicative identity in min-plus, which is the
    /// Lawvere metric identity axiom `d(x, x) = 0`. This default fires only
    /// when no explicit `set_distance(x, x, _)` has been recorded; an explicit
    /// non-zero diagonal entry takes precedence and surfaces in `hom` as set,
    /// without the default override.
    ///
    /// **Off-diagonal.** Falls through to [`distance`](Self::distance), which
    /// returns the recorded value or `Tropical::zero() = Tropical(+∞)` if
    /// unset. Off-diagonal unset entries remain "unreachable": the diagonal
    /// default enforces an axiom, it does not infer a transitive closure.
    fn hom(&self, a: &Self::Object, b: &Self::Object) -> Tropical {
        if a == b {
            // An explicit entry wins; otherwise the identity axiom's 0.
            self.distances
                .get(&(a.clone(), b.clone()))
                .copied()
                .unwrap_or_else(Tropical::one)
        } else {
            self.distance(a, b)
        }
    }

    fn objects(&self) -> Box<dyn Iterator<Item = Self::Object> + '_> {
        Box::new(self.objects.iter().cloned())
    }
}
