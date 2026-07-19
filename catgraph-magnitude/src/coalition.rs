//! [`Coalition`] — the §IV.5 enriched-coalition magnitude surface (#22).
//!
//! Bradley–Vigneaux 2025 (*The Magnitude of Categories of Texts Enriched by
//! Language Models*, arXiv:2501.06662)
//! §3.5 Eq (7) gives magnitude as the Möbius sum `Mag(tM) = Σ_{x,y} ζ_t⁻¹(x, y)`;
//! Bradley–Terilla–Vlassopoulos 2021 (*An enriched category theory of language*,
//! arXiv:2106.07890) supply the `[0,1]` enrichment via `d = −ln π`. This module
//! reads a coalition as a **cospan-weighted subgraph of an enriched category**
//! and computes its diversity as that Möbius sum on the induced Lawvere metric
//! space — a categorical alternative to message-passing where the couplings are
//! data (however obtained), not exchanged messages.
//!
//! # The §IV.5 mapping (gemini-spec)
//!
//! | §IV.5 concept          | catgraph realization                                            |
//! |------------------------|-----------------------------------------------------------------|
//! | agents                 | objects of an enriched category `A`                             |
//! | inter-agent coupling   | hom-object `A(i, j) ∈ Q` with `Q = `[`UnitInterval`] (`[0,1]`)   |
//! | coalition              | member-restricted, max-product-closed cospan-weighted subgraph  |
//! | coalition diversity    | `Mag(tA\|members)` — this module's [`coalition_magnitude`]       |
//!
//! `magnitude` computes the Möbius sum (BV 2025 §3.5 Eq 7), which is well-defined
//! for **cyclic** coupling graphs too. BV 2025 Prop 3.10's Tsallis *closed form*
//! only applies in the acyclic tree-poset case (see
//! [`LmCategory::magnitude`](crate::lm_category::LmCategory::magnitude)); coalitions
//! may be cyclic, which is fine for Eq 7.
//!
//! # Coalition semantics: restrict-then-close
//!
//! A coalition over a member subset `S ⊆ ob(A)` is the **free `[0,1]`-category
//! on the member-restricted coupling generators**. Construction (see
//! [`Coalition::from_enriched`]) is two-step:
//!
//! 1. **Restrict first.** Read the direct couplings `A(i, j)` for `i, j ∈ S`
//!    only — the generators. Couplings mediated through a **non-member** are
//!    dropped: they never enter the generator set, so they cannot contribute to
//!    the closure.
//! 2. **Then close.** `A_S(i, j)` is the highest product-of-couplings over any
//!    path `i → … → j` using **member nodes only**, computed by a dense
//!    Bellman–Ford relaxation (`m − 1` rounds; the optimum is a simple path of
//!    at most `m − 1` edges, and cycles never improve a product of weights
//!    `≤ 1`). The identity diagonal is `1.0`.
//!
//! Because the closure is a max-product (equivalently a min-sum of `−ln`
//! shortest path), composition satisfies `A_S(i, j) · A_S(j, k) ≤ A_S(i, k)`,
//! i.e. the triangle inequality `d(i, k) ≤ d(i, j) + d(j, k)` holds **by
//! construction** under the `−ln` lift.
//!
//! # Skeletalization (perfectly-coupled members)
//!
//! Two members `x, y` with `A_S(x, y) = A_S(y, x) = 1.0` are at distance `0` in
//! both directions — the same object up to the enrichment's equivalence. Left in
//! place they give ζ two identical rows, so ζ is singular at *every* `t`.
//! Magnitude is a property of the **skeleton**: it is invariant under
//! equivalence (Leinster 2008 *The Euler characteristic of a category*,
//! [arXiv:math/0610260](https://arxiv.org/abs/math/0610260) — magnitude is defined on skeletal categories
//! and invariant under equivalence), and for metric spaces the Kolmogorov
//! quotient of a pseudometric space is an equivalent `[0,∞]`-category with equal
//! magnitude (Leinster 2013). So [`Coalition::from_enriched`] quotients members
//! by `x ~ y ⟺ A_S(x, y) = A_S(y, x) = 1.0` (an equivalence relation given the
//! closure's triangle inequality) on the **closed** table and stores the
//! **skeletal** metric space (one representative per class). The full member
//! cospan is retained unchanged for the boundary story;
//! [`Coalition::effective_members`] reports the skeleton size and
//! [`Coalition::member_classes`] the per-member class index.
//!
//! Skeletalization removes *only* the perfectly-coupled degeneracy; other
//! singular ζ configurations (parametric coincidences) still surface as `Err`
//! from [`coalition_magnitude`].
//!
//! # Scale `t`
//!
//! `t = 1` is the canonical/default arm (the downstream `MagnitudePolicy` pin).
//! Its Shannon connection is via the **derivative**
//! `d/dt Mag(tM)|_{t=1} = Σ_x H(p_x)` (Shannon entropy; BV 2025 Rem 3.11 /
//! Eq (12)), not the `t = 1` value itself. `t = 2` is a collision-probability
//! proxy (the `Σ pᵢ²` regime); `t → ∞` approaches a cardinality-like limit
//! (effective member count). The API takes an explicit `t`.

use std::collections::HashMap;
use std::fmt::Debug;

use deep_causality_num::{One, Zero};

use crate::magnitude::magnitude;
use crate::weighted_cospan::{NodeId, WeightedCospan};
use crate::{CatgraphError, EnrichedCategory, F64Rig, LawvereMetricSpace, UnitInterval};

use catgraph::cospan::Cospan;

/// A coalition: the §IV.5 "cospan-weighted subgraph" of an enriched category.
///
/// Wraps a [`WeightedCospan<O, UnitInterval>`] whose apex/middle is the coalition
/// members (in local [`NodeId`] order — index `i` ↔ `NodeId(i)`) and whose
/// weights are the **max-product-closed** couplings `A_S(i, j)` (see the module
/// docs). The legs are the discrete/identity cospan over the members for v0 —
/// the boundary-port story is carried by the cospan type but richer legs (shared
/// state ports between overlapping coalitions) are future work.
///
/// Alongside the cospan, the coalition stores a **derived** [`LawvereMetricSpace`]
/// — the skeletal metric space (perfectly-coupled members quotiented; see the
/// module docs) — built once at construction. It is an immutable cache of the
/// cospan: `Coalition` exposes no mutators, so the cache never goes stale.
///
/// Construct with [`Coalition::from_enriched`] (reads couplings from any
/// [`EnrichedCategory`]`<UnitInterval>`) or via the plain-data entry point
/// [`coalition_magnitude_from_couplings`].
#[derive(Clone, Debug)]
pub struct Coalition<O>
where
    O: Copy + Eq + Debug,
{
    /// Member-restricted, closed couplings as cospan weights (diagonal = `1.0`).
    /// Middle = the members in local `NodeId` order; retained for the boundary
    /// story.
    cospan: WeightedCospan<O, UnitInterval>,
    /// Derived cache: the **skeletal** Lawvere metric space (one object per
    /// `~`-class). `magnitude` is computed from this, not from the full cospan.
    space: LawvereMetricSpace<NodeId>,
    /// Per-member class index (`0..effective_members()`); `members()[i]` belongs
    /// to skeleton class `member_classes()[i]`.
    member_classes: Vec<usize>,
}

impl<O> Coalition<O>
where
    O: Copy + Eq + Debug,
{
    /// Build a coalition over `members` by reading couplings from an enriched
    /// category, applying the **restrict-then-close** max-product closure, and
    /// **skeletalizing** perfectly-coupled members (see the module docs).
    ///
    /// The direct generators are `cat.hom(members[i], members[j])` for
    /// `i, j ∈ members` (off-diagonal); the closure records the highest
    /// product-of-couplings path through member nodes only. The full member
    /// cospan (diagonal `1.0`) is retained; the stored metric space is the
    /// skeleton.
    ///
    /// `t = 1` is the canonical arm downstream — see [`coalition_magnitude`].
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if:
    /// - `members` is empty,
    /// - `members` contains a duplicate (each `NodeId` must be a distinct agent),
    ///   or
    /// - some member is not an object of `cat`.
    pub fn from_enriched<C>(cat: &C, members: &[O]) -> Result<Self, CatgraphError>
    where
        C: EnrichedCategory<UnitInterval, Object = O>,
    {
        if members.is_empty() {
            return Err(CatgraphError::Composition {
                message: "Coalition::from_enriched: empty member set".to_owned(),
            });
        }

        // Members must be distinct — a repeated member would seed two NodeIds
        // for one agent (∞-separated) and inflate the magnitude. O(m²) Eq scan
        // (no Hash bound available on `O`).
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                if members[i] == members[j] {
                    return Err(CatgraphError::Composition {
                        message: format!(
                            "Coalition::from_enriched: duplicate member {:?}",
                            members[i]
                        ),
                    });
                }
            }
        }

        // Validate membership: every coalition member must be an object of `cat`.
        let cat_objects: Vec<O> = cat.objects().collect();
        for m in members {
            if !cat_objects.contains(m) {
                return Err(CatgraphError::Composition {
                    message: format!(
                        "Coalition::from_enriched: member {m:?} is not an object of the category"
                    ),
                });
            }
        }

        let m = members.len();

        // Step 1 — restrict: direct generators among members only. Couplings
        // routed through non-members are absent by construction (we only ever
        // read hom(member, member)). The generator diagonal is written but never
        // read: the closure seeds `best[s] = 1.0` and skips `next == cur`.
        let generators: Vec<Vec<f64>> = (0..m)
            .map(|i| {
                (0..m)
                    .map(|j| cat.hom(&members[i], &members[j]).value())
                    .collect()
            })
            .collect();

        // Step 2 — close: dense max-product transitive closure within members.
        let closed = bellman_ford_closure(&generators, m);

        // Materialize the full member cospan (discrete legs on `0..m`).
        let legs: Vec<NodeId> = (0..m).collect();
        let cospan_inner = Cospan::new(legs.clone(), legs, members.to_vec());
        let mut wc = WeightedCospan::from_cospan_uniform(cospan_inner, UnitInterval::zero());
        for i in 0..m {
            wc.set_weight(i, i, UnitInterval::one()); // identity axiom d(i,i) = 0
        }
        for (i, row) in closed.iter().enumerate() {
            for (j, &p) in row.iter().enumerate() {
                if i != j && p > 0.0 {
                    // A product of couplings each in [0, 1] stays in [0, 1].
                    let ui = UnitInterval::new(p)
                        .expect("invariant: product of [0,1] couplings stays in [0,1]");
                    wc.set_weight(i, j, ui);
                }
            }
        }

        // Step 3 — skeletalize on the CLOSED table: quotient members by
        // x ~ y ⟺ closed(x,y) = closed(y,x) = 1.0 (d = 0 both directions).
        let (member_classes, reps) = skeletal_classes(&closed, m);
        let space = build_skeletal_space(&closed, &reps);

        debug_assert!(
            space.triangle_inequality_holds_within(crate::TRIANGLE_FLOAT_TOL),
            "coalition closure must satisfy the triangle inequality by construction \
             (within float tolerance TRIANGLE_FLOAT_TOL — the −ln lift of a max-product \
             closure differs from the summed-log bound by ULPs)"
        );

        Ok(Self {
            cospan: wc,
            space,
            member_classes,
        })
    }

    /// Borrow the underlying cospan-weighted subgraph (weights = closed
    /// couplings, diagonal = `1.0`, full member set — no skeletalization).
    #[must_use]
    pub fn as_weighted_cospan(&self) -> &WeightedCospan<O, UnitInterval> {
        &self.cospan
    }

    /// The coalition members in local [`NodeId`] order (`members()[i]` ↔
    /// `NodeId(i)`).
    #[must_use]
    pub fn members(&self) -> &[O] {
        self.cospan.as_cospan().middle()
    }

    /// Number of members (the full set, before skeletalization). Always `≥ 1`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cospan.as_cospan().middle().len()
    }

    /// Always `false` — a [`Coalition`] cannot be constructed empty. Present for
    /// the clippy `len`/`is_empty` pairing convention.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cospan.as_cospan().middle().is_empty()
    }

    /// Number of **effective** agents: the skeleton size after quotienting
    /// perfectly-coupled (`A(x,y) = A(y,x) = 1.0`) members. A coalition of 5
    /// members all mutually coupled at `1.0` reports `1` (see the module docs).
    #[must_use]
    pub fn effective_members(&self) -> usize {
        self.space.size()
    }

    /// Per-member skeleton class index: `member_classes()[i]` is the
    /// `~`-equivalence class of `members()[i]`, in `0..effective_members()`.
    /// Members sharing a class are perfectly coupled (distance `0` both ways).
    #[must_use]
    pub fn member_classes(&self) -> &[usize] {
        &self.member_classes
    }

    /// Borrow the derived **skeletal** Lawvere metric space (one object per
    /// `~`-class) that [`coalition_magnitude`] inverts.
    ///
    /// `pub(crate)` so [`crate::coalition_eval`] (#31) can cache its base `μ`
    /// off this exact space instead of re-skeletalizing the extracted closed
    /// table — the two would agree, but reuse keeps the base value bit-identical
    /// and saves the rebuild.
    #[must_use]
    pub(crate) fn space(&self) -> &LawvereMetricSpace<NodeId> {
        &self.space
    }
}

/// Dense max-product transitive closure of the `m × m` generator matrix.
///
/// `closed[s][j]` is the highest product of couplings over any path `s → … → j`
/// through member nodes, with `closed[s][s] = 1.0`. Per source `s`, run at most
/// `m − 1` Bellman–Ford rounds — each round relaxes every ordered pair
/// `best[next] = max(best[next], best[cur] · g[cur][next])` — with early exit on
/// a no-change round. This is exact for max-product with weights `≤ 1`: the
/// optimal path is simple (at most `m − 1` edges, since traversing a cycle
/// multiplies by a factor `≤ 1` and never improves), so `m − 1` rounds suffice.
/// `O(m³)` worst case, deterministic. (An earlier LIFO frontier with an `m·m`
/// *pop* cap was defective — dense near-`1.0` couplings can require more than
/// `m²` pops, silently truncating the closure.)
fn bellman_ford_closure(generators: &[Vec<f64>], m: usize) -> Vec<Vec<f64>> {
    let mut closed = vec![vec![0.0_f64; m]; m];
    for s in 0..m {
        let mut best = vec![0.0_f64; m];
        best[s] = 1.0;
        for _round in 0..m.saturating_sub(1) {
            let mut changed = false;
            for cur in 0..m {
                let cp = best[cur];
                if cp <= 0.0 {
                    continue;
                }
                for next in 0..m {
                    if next == cur {
                        continue;
                    }
                    let g = generators[cur][next];
                    if g <= 0.0 {
                        continue;
                    }
                    let np = cp * g;
                    if np > best[next] {
                        best[next] = np;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        closed[s] = best;
    }
    closed
}

/// Quotient members by `x ~ y ⟺ closed(x,y) = closed(y,x) = 1.0` (perfect
/// mutual coupling ⇒ distance `0` both ways). Returns `(member_classes, reps)`
/// where `member_classes[x]` is the compact class index of member `x` and
/// `reps[c]` is the representative member (smallest index) of class `c`.
///
/// `~` is an equivalence relation given the closure's triangle inequality
/// (`d(x,z) ≤ d(x,y) + d(y,z) = 0`), so union-find over the mutual-`1.0` pairs
/// yields a well-defined partition. The `== 1.0` test is exact: a product of
/// couplings equals `1.0` iff every factor is `1.0`.
///
/// # Preconditions
///
/// `closed` must be a max-product **closed** `m × m` table: diagonal exactly
/// `1.0`, entries in `[0, 1]`, and the triangle inequality holding under the
/// `−ln` lift. Callers pass either [`bellman_ford_closure`]'s output or a
/// bordered extension of it ([`crate::coalition_eval`]'s slow path); the exact
/// `== 1.0` equivalence test relies on the closedness (a raw generator table,
/// where a `1.0` two-hop product isn't yet materialized, would mis-quotient).
// The symmetric pair scan reads BOTH `closed[i][j]` and `closed[j][i]`, and the
// second loop indexes `parent`/`member_classes` while consuming a union-find
// `find` — an iterator-by-value rewrite doesn't apply (mirrors the module-level
// allow in `magnitude.rs`).
//
// `pub(crate)` for reuse by [`crate::coalition_eval`] (#31): the incremental
// slow path re-skeletalizes the bordered table with this exact helper.
#[allow(clippy::needless_range_loop)]
pub(crate) fn skeletal_classes(closed: &[Vec<f64>], m: usize) -> (Vec<usize>, Vec<usize>) {
    let mut parent: Vec<usize> = (0..m).collect();
    for i in 0..m {
        for j in (i + 1)..m {
            if closed[i][j] == 1.0 && closed[j][i] == 1.0 {
                let (ri, rj) = (uf_find(&mut parent, i), uf_find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }
    // Compact roots to `0..k` in first-seen order; representative = first member.
    let mut root_to_class: HashMap<usize, usize> = HashMap::new();
    let mut member_classes = vec![0usize; m];
    let mut reps: Vec<usize> = Vec::new();
    for x in 0..m {
        let r = uf_find(&mut parent, x);
        let c = *root_to_class.entry(r).or_insert_with(|| {
            reps.push(x);
            reps.len() - 1
        });
        member_classes[x] = c;
    }
    (member_classes, reps)
}

/// Iterative union-find `find` with path halving.
fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Build the skeletal Lawvere metric space over `0..reps.len()` from the closed
/// table, using each class's representative row: `d(a, b) = −ln closed[rep_a][rep_b]`
/// (`+∞` when the coupling is `0`). Quotient distances are well-defined — any
/// representative gives the same value (Kolmogorov quotient of a pseudometric).
///
/// `pub(crate)` for reuse by [`crate::coalition_eval`] (#31): the incremental
/// slow path builds its skeletal space with this exact helper, so incremental
/// evaluation shares the fresh-path metric.
///
/// # Preconditions
///
/// `closed` must be a max-product **closed** table (diagonal exactly `1.0`,
/// entries in `[0, 1]`, triangle inequality holding under the `−ln` lift), and
/// `reps` the class representatives [`skeletal_classes`] returned for it — each
/// `reps[a]` a valid row/column index into `closed`. The Kolmogorov-quotient
/// well-definedness (any representative gives the same distance) holds only for
/// such a closed table.
pub(crate) fn build_skeletal_space(
    closed: &[Vec<f64>],
    reps: &[usize],
) -> LawvereMetricSpace<NodeId> {
    let k = reps.len();
    LawvereMetricSpace::from_distance_fn(k, |a, b| {
        let p = closed[reps[a]][reps[b]];
        if p > 0.0 { -p.ln() } else { f64::INFINITY }
    })
}

/// Coalition diversity `Mag(tA|members)` — the magnitude of the coalition's
/// **skeletal** Lawvere metric space at scale `t` (BV 2025 §3.5 Eq 7 Möbius sum).
///
/// Reads the derived skeletal space cached at construction (no per-call
/// allocation) and calls [`magnitude::<F64Rig>`](crate::magnitude::magnitude).
/// The Möbius sum is well-defined for cyclic coupling graphs (Prop 3.10's Tsallis
/// closed form is the acyclic tree-poset special case — see
/// [`LmCategory::magnitude`](crate::lm_category::LmCategory::magnitude)).
///
/// See the module [`Scale t`](crate::coalition#scale-t) section for the
/// `t = 1` (canonical) / `t = 2` (collision) / `t → ∞` (cardinality) arms.
///
/// # Panics
///
/// Debug-only: panics if `t ≤ 0` (BV 2025 §3 studies `t > 0`; behavior at
/// `t ≤ 0` is unspecified — mirrors
/// [`LmCategory::magnitude`](crate::lm_category::LmCategory::magnitude)).
///
/// # Errors
///
/// Returns [`CatgraphError::Composition`] if the `t`-scaled zeta is singular.
/// Skeletalization already removes the perfectly-coupled degeneracy (identical
/// ζ rows); other singular configurations (parametric coincidences) still
/// surface here — callers may pre-check with
/// [`is_mobius_invertible_at`](crate::magnitude::is_mobius_invertible_at).
pub fn coalition_magnitude<O>(coalition: &Coalition<O>, t: f64) -> Result<f64, CatgraphError>
where
    O: Copy + Eq + Debug,
{
    debug_assert!(
        t > 0.0,
        "coalition_magnitude(t) requires t > 0; behavior at t = {t} is unspecified per BV 2025 §3"
    );
    let mag: F64Rig = magnitude(&coalition.space, t)?;
    Ok(mag.0)
}

/// Plain-data entry point: coalition diversity from an agent list, a sparse
/// coupling table, and a member index subset.
///
/// Builds an internal [`HomMap`](crate::HomMap)`<O, UnitInterval>` over `agents`, forms the
/// [`Coalition`] via [`Coalition::from_enriched`] (restrict-then-close +
/// skeletalize), and returns [`coalition_magnitude`] at scale `t`. Couplings are
/// `(from_idx, to_idx, prob)` triples with `prob` validated into `[0, 1]` via
/// [`UnitInterval::new`]; `members` are indices into `agents`.
///
/// This is the seed of C3's stable `coalition_value` (#23) — the signature is
/// deliberately plain-data (no enriched-category type in the caller's hands) so
/// it can back a stable public API without exposing the enrichment substrate.
///
/// # Errors
///
/// Returns [`CatgraphError`] if:
/// - any member index is out of range for `agents` (validated first),
/// - any coupling index is out of range for `agents`,
/// - a coupling is a self-loop `(i, i, _)` (the identity axiom fixes the
///   diagonal to `1.0`; a self-coupling would be silently ignored — rejected
///   instead),
/// - some `prob` is outside `[0, 1]` (via [`UnitInterval::new`]), or
/// - the coalition is empty / has a duplicate / a member is not an agent
///   (from [`Coalition::from_enriched`]).
pub fn coalition_magnitude_from_couplings<O>(
    agents: &[O],
    couplings: &[(usize, usize, f64)],
    members: &[usize],
    t: f64,
) -> Result<f64, CatgraphError>
where
    O: Copy + Eq + std::hash::Hash + Debug + 'static,
{
    let (cat, member_objs, _map) = build_coupling_category(
        agents,
        couplings,
        members,
        "coalition_magnitude_from_couplings",
    )?;
    let coalition = Coalition::from_enriched(&cat, &member_objs)?;
    coalition_magnitude(&coalition, t)
}

/// Shared validation + [`HomMap`](crate::HomMap) construction for the plain-data
/// coalition entry points (#31 dedup).
///
/// Validates in the order [`coalition_magnitude_from_couplings`] fixed — member
/// indices first, then per coupling: index range, self-loop rejection, and
/// `prob ∈ [0, 1]` via [`UnitInterval::new`] — using `ctx` as the error-message
/// prefix so each caller keeps its own message text. Returns the built
/// `HomMap`, the resolved member objects, and a `(from, to) → prob` map of the
/// validated couplings **incident to a member** (either endpoint in `members`),
/// last-write-wins on duplicates (matching [`HomMap::set_hom`](crate::HomMap)'s
/// overwrite). [`coalition_magnitude_from_couplings`] ignores the map;
/// [`crate::coalition_eval`] reads it to border a candidate against a member.
///
/// # Errors
///
/// [`CatgraphError::Composition`] for an out-of-range member index, an
/// out-of-range coupling index, a self-coupling, or (propagated) a probability
/// outside `[0, 1]`.
#[allow(clippy::type_complexity)]
pub(crate) fn build_coupling_category<O>(
    agents: &[O],
    couplings: &[(usize, usize, f64)],
    members: &[usize],
    ctx: &str,
) -> Result<
    (
        crate::HomMap<O, UnitInterval>,
        Vec<O>,
        HashMap<(usize, usize), f64>,
    ),
    CatgraphError,
>
where
    O: Copy + Eq + std::hash::Hash + Debug + 'static,
{
    let n = agents.len();

    // Validate member indices FIRST — cheap, and avoids building the HomMap on
    // invalid input.
    let member_objs: Vec<O> = members
        .iter()
        .map(|&idx| {
            agents
                .get(idx)
                .copied()
                .ok_or_else(|| CatgraphError::Composition {
                    message: format!("{ctx}: member index {idx} out of range for {n} agents"),
                })
        })
        .collect::<Result<_, _>>()?;

    let member_set: std::collections::HashSet<usize> = members.iter().copied().collect();

    let mut cat = crate::HomMap::<O, UnitInterval>::new(agents.to_vec());
    let mut coupling_map: HashMap<(usize, usize), f64> = HashMap::new();
    for &(i, j, p) in couplings {
        if i >= n || j >= n {
            return Err(CatgraphError::Composition {
                message: format!("{ctx}: coupling index ({i}, {j}) out of range for {n} agents"),
            });
        }
        if i == j {
            return Err(CatgraphError::Composition {
                message: format!(
                    "{ctx}: self-coupling ({i}, {i}, {p}) — the identity axiom fixes the diagonal to 1.0, so a self-coupling would be silently ignored"
                ),
            });
        }
        let ui = UnitInterval::new(p)?;
        cat.set_hom(agents[i], agents[j], ui);
        // Only member-incident couplings are ever read downstream (a candidate
        // borders against members), so store just those — O(m·n), not O(n²).
        if member_set.contains(&i) || member_set.contains(&j) {
            coupling_map.insert((i, j), ui.value());
        }
    }

    Ok((cat, member_objs, coupling_map))
}

/// **The stable consumer entry point** (#23): coalition diversity as a single
/// scalar, at the pinned canonical scale `t = 1`.
///
/// Equivalent to [`coalition_magnitude_from_couplings`]`(agents, couplings,
/// members, 1.0)`. This is the stability-contracted scalar downstream decision
/// policies call — koalisi #5's `MagnitudePolicy` A/Bs it against tira/aif's
/// `−G`. The semantics are **effective-member diversity**: the magnitude of the
/// coalition's *skeletal* Lawvere metric space (perfectly-coupled members
/// quotiented — see the [module docs](crate::coalition)).
///
/// `t = 1` is the pinned canonical arm (#22 pins it; its Shannon tie is the
/// derivative `d/dt Mag|_{t=1} = Σ_x H(p_x)`, BV 2025 Rem 3.11 / Eq (12)). The `t`-sweep
/// (`t = 2` collision proxy, `t → ∞` cardinality limit) is an experiment axis of
/// the downstream A/B harness, **not** a knob on this stable API — callers who
/// need other scales reach for [`coalition_magnitude_from_couplings`] directly.
///
/// Couplings are `(from_idx, to_idx, prob)` triples over `agents`; `members` are
/// indices into `agents`.
///
/// # Errors
///
/// Inherited verbatim from [`coalition_magnitude_from_couplings`]: out-of-range
/// member/coupling indices, self-couplings, probabilities outside `[0, 1]`,
/// empty/duplicate/unknown members, or a singular `t`-scaled zeta.
pub fn coalition_value<O>(
    agents: &[O],
    couplings: &[(usize, usize, f64)],
    members: &[usize],
) -> Result<f64, CatgraphError>
where
    O: Copy + Eq + std::hash::Hash + Debug + 'static,
{
    coalition_magnitude_from_couplings(agents, couplings, members, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HomMap;
    use crate::lm_category::LmCategory;

    const EPS: f64 = 1e-9;

    /// Build a `HomMap<&str, UnitInterval>` from `(from, to, prob)` name triples.
    fn hommap(
        agents: &[&'static str],
        edges: &[(&'static str, &'static str, f64)],
    ) -> HomMap<&'static str, UnitInterval> {
        let mut cat = HomMap::new(agents.to_vec());
        for &(a, b, p) in edges {
            cat.set_hom(a, b, UnitInterval::new(p).unwrap());
        }
        cat
    }

    /// Shared a → b → c (0.7, 0.5) chain fixture — closure gives A(a,c) = 0.35.
    fn chain_abc() -> HomMap<&'static str, UnitInterval> {
        hommap(&["a", "b", "c"], &[("a", "b", 0.7), ("b", "c", 0.5)])
    }

    // -----------------------------------------------------------------------
    // Chain: closure A(a,c) = 0.35; the induced space equals the mock_coalition
    // 3-agent sub-coalition, so coalition_magnitude must match
    // LmCategory::magnitude on the same topology.
    // -----------------------------------------------------------------------
    #[test]
    fn chain_matches_lmcategory_magnitude() {
        let cat = chain_abc();
        let coalition = Coalition::from_enriched(&cat, &["a", "b", "c"]).unwrap();

        // Closed coupling A(a, c) = 0.7 * 0.5 = 0.35.
        assert!((coalition.as_weighted_cospan().weight(0, 2).value() - 0.35).abs() < EPS);
        // No perfect coupling ⇒ 3 effective members.
        assert_eq!(coalition.effective_members(), 3);

        // LmCategory over the same topology (magnitude is computed from the
        // identical space either way).
        let mut lm = LmCategory::new(vec!["a".into(), "b".into(), "c".into()]);
        lm.add_transition("a", "b", 0.7).unwrap();
        lm.add_transition("b", "c", 0.5).unwrap();
        lm.mark_terminating("c");

        for t in [1.0_f64, 2.0] {
            let via_coalition = coalition_magnitude(&coalition, t).unwrap();
            let via_lm = lm.magnitude(t).unwrap();
            assert!(
                (via_coalition - via_lm).abs() < EPS,
                "t = {t}: coalition {via_coalition} != LmCategory {via_lm}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Diamond: a → {b, c} → d with two paths of different weight.
    //   a→b 0.6, b→d 0.5  (product 0.30)
    //   a→c 0.4, c→d 0.9  (product 0.36)  ⇒ A(a,d) = max = 0.36.
    // Closed ζ at t=1 (order a,b,c,d) is upper-triangular:
    //   [[1, 0.6, 0.4, 0.36], [0,1,0,0.5], [0,0,1,0.9], [0,0,0,1]]
    // Solving ζ·w = 1 (back-substitution):
    //   w_d = 1; w_c = 1-0.9 = 0.1; w_b = 1-0.5 = 0.5;
    //   w_a = 1 - 0.6·0.5 - 0.4·0.1 - 0.36·1 = 0.30.
    //   Mag(1) = Σ w = 0.30 + 0.5 + 0.1 + 1 = 1.90.
    // -----------------------------------------------------------------------
    #[test]
    fn diamond_closed_table_and_hand_derived_magnitude() {
        let cat = hommap(
            &["a", "b", "c", "d"],
            &[
                ("a", "b", 0.6),
                ("a", "c", 0.4),
                ("b", "d", 0.5),
                ("c", "d", 0.9),
            ],
        );
        let coalition = Coalition::from_enriched(&cat, &["a", "b", "c", "d"]).unwrap();
        let wc = coalition.as_weighted_cospan();

        // Closed couplings: the winning a→d path is a→c→d = 0.36 (> a→b→d = 0.30).
        assert!((wc.weight(0, 1).value() - 0.6).abs() < EPS); // a→b
        assert!((wc.weight(0, 2).value() - 0.4).abs() < EPS); // a→c
        assert!((wc.weight(0, 3).value() - 0.36).abs() < EPS); // a→d (closed, max path)
        assert!((wc.weight(1, 3).value() - 0.5).abs() < EPS); // b→d
        assert!((wc.weight(2, 3).value() - 0.9).abs() < EPS); // c→d
        assert_eq!(coalition.effective_members(), 4);

        // Hand-derived magnitude at t = 1 (see comment above).
        let mag1 = coalition_magnitude(&coalition, 1.0).unwrap();
        assert!(
            (mag1 - 1.90).abs() < EPS,
            "diamond Mag(1) = {mag1}, expected 1.90"
        );
    }

    // -----------------------------------------------------------------------
    // Restrict-before-close pin: a → m → b with m NOT a member ⇒ A(a,b) = 0.
    // -----------------------------------------------------------------------
    #[test]
    fn restrict_before_close_drops_nonmember_mediation() {
        let cat = hommap(&["a", "m", "b"], &[("a", "m", 0.9), ("m", "b", 0.9)]);
        let coalition = Coalition::from_enriched(&cat, &["a", "b"]).unwrap();
        let wc = coalition.as_weighted_cospan();

        // Local index: a = 0, b = 1. No member-only path ⇒ A(a,b) = 0.
        assert!(
            wc.weight(0, 1).value() == 0.0,
            "non-member mediation must not count"
        );
        let space = wc.clone().into_metric_space();
        assert!(space.distance(&0, &1).0.is_infinite());
        assert_eq!(coalition.effective_members(), 2);
    }

    // -----------------------------------------------------------------------
    // Cyclic couplings a ⇄ b at 0.5: terminates; d = −ln 0.5 both ways; not
    // merged (not perfect). ζ = [[1,0.5],[0.5,1]] ⇒ Mag(1) = 1/0.75 = 4/3.
    // -----------------------------------------------------------------------
    #[test]
    fn cyclic_couplings_terminate_and_magnitude_defined() {
        let cat = hommap(&["a", "b"], &[("a", "b", 0.5), ("b", "a", 0.5)]);
        let coalition = Coalition::from_enriched(&cat, &["a", "b"]).unwrap();
        assert_eq!(coalition.effective_members(), 2);
        let space = coalition.as_weighted_cospan().clone().into_metric_space();

        let want = -0.5_f64.ln();
        assert!((space.distance(&0, &1).0 - want).abs() < EPS);
        assert!((space.distance(&1, &0).0 - want).abs() < EPS);

        let mag1 = coalition_magnitude(&coalition, 1.0).unwrap();
        assert!(
            (mag1 - 4.0 / 3.0).abs() < EPS,
            "cyclic Mag(1) = {mag1}, expected 4/3"
        );
    }

    // -----------------------------------------------------------------------
    // Skeletalization — perfectly-coupled members collapse.
    // -----------------------------------------------------------------------

    /// Mutual-1.0 pair a ⇄ b: both d = 0 ⇒ one effective member ⇒ Mag = 1 at
    /// all t (a single-point skeleton). Without skeletalization ζ would have two
    /// identical rows and be singular at every t.
    #[test]
    fn mutual_one_pair_collapses_to_single_agent() {
        let cat = hommap(&["a", "b"], &[("a", "b", 1.0), ("b", "a", 1.0)]);
        let coalition = Coalition::from_enriched(&cat, &["a", "b"]).unwrap();
        assert_eq!(coalition.effective_members(), 1);
        assert_eq!(coalition.member_classes(), &[0, 0]);
        for t in [0.5_f64, 1.0, 2.0, 10.0] {
            let mag = coalition_magnitude(&coalition, t).unwrap();
            assert!(
                (mag - 1.0).abs() < EPS,
                "mutual-1.0 pair Mag({t}) = {mag}, expected 1"
            );
        }
    }

    /// 1.0 three-cycle a→b→c→a: the CLOSED table makes every pair 1.0 (e.g.
    /// A(a,c) = a→b→c = 1.0), so all three collapse ⇒ Mag = 1. Verifies the
    /// quotient is computed on the closed table, not the raw generators (where
    /// a→c is 0).
    #[test]
    fn one_three_cycle_collapses_via_closed_table() {
        let cat = hommap(
            &["a", "b", "c"],
            &[("a", "b", 1.0), ("b", "c", 1.0), ("c", "a", 1.0)],
        );
        let coalition = Coalition::from_enriched(&cat, &["a", "b", "c"]).unwrap();
        assert_eq!(coalition.effective_members(), 1);
        assert_eq!(coalition.member_classes(), &[0, 0, 0]);
        for t in [1.0_f64, 2.0] {
            let mag = coalition_magnitude(&coalition, t).unwrap();
            assert!(
                (mag - 1.0).abs() < EPS,
                "3-cycle Mag({t}) = {mag}, expected 1"
            );
        }
    }

    /// Two mutual-1.0 clones a ⇄ b plus a distinct c (a→c 0.5): {a,b} merge into
    /// one class, so the 3-member coalition's magnitude equals the 2-member
    /// coalition {a,c} with a→c 0.5 (the merged clone class ≡ the single a).
    #[test]
    fn two_clones_plus_one_equals_two_member_coalition() {
        let cat3 = hommap(
            &["a", "b", "c"],
            &[("a", "b", 1.0), ("b", "a", 1.0), ("a", "c", 0.5)],
        );
        let coalition3 = Coalition::from_enriched(&cat3, &["a", "b", "c"]).unwrap();
        assert_eq!(coalition3.effective_members(), 2);
        assert_eq!(coalition3.member_classes(), &[0, 0, 1]); // a,b in class 0; c in class 1

        // Reference 2-member coalition {a, c} with the same a→c coupling.
        let cat2 = hommap(&["a", "c"], &[("a", "c", 0.5)]);
        let coalition2 = Coalition::from_enriched(&cat2, &["a", "c"]).unwrap();

        for t in [1.0_f64, 2.0] {
            let m3 = coalition_magnitude(&coalition3, t).unwrap();
            let m2 = coalition_magnitude(&coalition2, t).unwrap();
            assert!(
                (m3 - m2).abs() < EPS,
                "t = {t}: clone-merged {m3} != 2-member {m2}"
            );
        }
    }

    /// Asymmetric a→b = 1.0, b→a = 0.5: NOT mutual-1.0 ⇒ not merged ⇒ still 2
    /// effective members, finite magnitude.
    #[test]
    fn asymmetric_one_not_merged() {
        let cat = hommap(&["a", "b"], &[("a", "b", 1.0), ("b", "a", 0.5)]);
        let coalition = Coalition::from_enriched(&cat, &["a", "b"]).unwrap();
        assert_eq!(coalition.effective_members(), 2);
        assert_eq!(coalition.member_classes(), &[0, 1]);
        let mag = coalition_magnitude(&coalition, 1.0).unwrap();
        assert!(mag.is_finite(), "asymmetric Mag(1) = {mag} must be finite");
    }

    // -----------------------------------------------------------------------
    // Singleton coalition: one object, d = 0 diagonal ⇒ Mag = 1 at any t.
    // -----------------------------------------------------------------------
    #[test]
    fn singleton_magnitude_is_one_at_any_t() {
        let cat = hommap(&["solo"], &[]);
        let coalition = Coalition::from_enriched(&cat, &["solo"]).unwrap();
        assert_eq!(coalition.len(), 1);
        assert_eq!(coalition.effective_members(), 1);
        for t in [0.5_f64, 1.0, 2.0, 10.0] {
            let mag = coalition_magnitude(&coalition, t).unwrap();
            assert!(
                (mag - 1.0).abs() < EPS,
                "singleton Mag({t}) = {mag}, expected 1"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Construction errors: empty members, unknown member, duplicate member.
    // -----------------------------------------------------------------------
    #[test]
    fn construction_errors() {
        let cat = hommap(&["a", "b"], &[("a", "b", 0.5)]);
        assert!(
            Coalition::from_enriched(&cat, &[]).is_err(),
            "empty members must error"
        );
        assert!(
            Coalition::from_enriched(&cat, &["a", "ghost"]).is_err(),
            "unknown member must error"
        );
        assert!(
            Coalition::from_enriched(&cat, &["a", "a"]).is_err(),
            "duplicate member must error"
        );
    }

    // -----------------------------------------------------------------------
    // Monotonicity sanity: chain at t ∈ {0.5, 1, 2, 10}, magnitude within
    // [1, n] for t ≥ 1 (BV 2025 p.4 bound holds for t ≥ 1 only).
    // -----------------------------------------------------------------------
    #[test]
    fn chain_magnitude_within_bounds_for_t_ge_1() {
        let cat = chain_abc();
        let coalition = Coalition::from_enriched(&cat, &["a", "b", "c"]).unwrap();
        let n = coalition.len() as f64;
        for t in [0.5_f64, 1.0, 2.0, 10.0] {
            let mag = coalition_magnitude(&coalition, t).unwrap();
            assert!(mag.is_finite(), "Mag({t}) must be finite");
            if t >= 1.0 {
                assert!(
                    mag >= 1.0 - EPS && mag <= n + EPS,
                    "Mag({t}) = {mag} out of bounds [1, {n}]"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Plain-data entry point: agrees with the enriched path; validates member
    // indices, coupling indices, self-couplings, and probabilities.
    // -----------------------------------------------------------------------
    #[test]
    fn from_couplings_matches_enriched_path_and_validates() {
        let agents = ["a", "b", "c"];
        let couplings = [(0usize, 1usize, 0.7_f64), (1, 2, 0.5)];
        let members = [0usize, 1, 2];

        let via_data =
            coalition_magnitude_from_couplings(&agents, &couplings, &members, 1.0).unwrap();

        let cat = chain_abc();
        let coalition = Coalition::from_enriched(&cat, &["a", "b", "c"]).unwrap();
        let via_cat = coalition_magnitude(&coalition, 1.0).unwrap();
        assert!((via_data - via_cat).abs() < EPS);

        // Out-of-range coupling and member indices error.
        assert!(
            coalition_magnitude_from_couplings(&agents, &[(0, 9, 0.5)], &members, 1.0).is_err()
        );
        assert!(coalition_magnitude_from_couplings(&agents, &couplings, &[9], 1.0).is_err());
        // Self-coupling triple errors (identity axiom fixes the diagonal).
        assert!(
            coalition_magnitude_from_couplings(&agents, &[(1, 1, 0.5)], &members, 1.0).is_err()
        );
        // Out-of-[0,1] probability errors via UnitInterval::new.
        assert!(
            coalition_magnitude_from_couplings(&agents, &[(0, 1, 1.5)], &members, 1.0).is_err()
        );
    }

    // -----------------------------------------------------------------------
    // Issue #29 regression: non-dyadic couplings must not trip the
    // triangle-inequality debug_assert in `from_enriched`. The skeletal space
    // stores d = −ln(closed[i][j]); for a multi-hop closure closed[a][c] =
    // closed[a][b]·closed[b][c], so d(a,c) = −ln(product) while the summed
    // bound is (−ln closed[a][b]) + (−ln closed[b][c]) — the two differ by ULPs
    // of `ln`/multiplication rounding. Pre-fix, the STRICT `dxz > sum` assert
    // panics in DEBUG on realistic couplings (e.g. 1/7·1/9 chains); post-fix
    // the tolerant `triangle_inequality_holds_within(TRIANGLE_FLOAT_TOL)` check
    // accepts them. This test runs under `cargo test` (debug), so it exercises
    // the debug_assert directly — pre-fix it panics inside `from_enriched`.
    // -----------------------------------------------------------------------
    #[test]
    fn non_dyadic_couplings_do_not_trip_triangle_assert() {
        let agents = ["a", "b", "c", "d"];
        // Grid of non-dyadic couplings (i/7, j/9, k/11), all in (0, 1), over a
        // 4-agent chain a→b→c→d so the closure builds genuine multi-hop −ln
        // products (closed a→c, a→d, b→d).
        for i in 1..7u32 {
            for j in 1..9u32 {
                for k in 1..11u32 {
                    let p = f64::from(i) / 7.0;
                    let q = f64::from(j) / 9.0;
                    let r = f64::from(k) / 11.0;
                    let cat = hommap(&agents, &[("a", "b", p), ("b", "c", q), ("c", "d", r)]);

                    // Construction runs the (post-fix tolerant) triangle-
                    // inequality debug_assert on the closed table.
                    let coalition = Coalition::from_enriched(&cat, &agents)
                        .expect("non-dyadic chain coalition must construct");

                    // Never a panic: magnitude is either finite-Ok or a
                    // well-formed Err (singular ζ) — but construction reaching
                    // here at all is the regression under test.
                    if let Ok(mag) = coalition_magnitude(&coalition, 1.0) {
                        assert!(
                            mag.is_finite(),
                            "coupling ({i}/7, {j}/9, {k}/11): Mag(1) = {mag} must be finite"
                        );
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Stable entry point coalition_value == coalition_magnitude_from_couplings
    // at the pinned t = 1 arm (chain fixture).
    // -----------------------------------------------------------------------
    #[test]
    fn coalition_value_is_t1_from_couplings() {
        let agents = ["a", "b", "c"];
        let couplings = [(0usize, 1usize, 0.7_f64), (1, 2, 0.5)];
        let members = [0usize, 1, 2];

        let via_value = coalition_value(&agents, &couplings, &members).unwrap();
        let via_t1 =
            coalition_magnitude_from_couplings(&agents, &couplings, &members, 1.0).unwrap();
        assert!((via_value - via_t1).abs() < EPS);
    }
}
