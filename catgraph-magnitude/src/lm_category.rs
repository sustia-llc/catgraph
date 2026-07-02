//! [`LmCategory`] — materialized language-model transition table per
//! BV 2025 §3.
//!
//! Stores a finite set of named states, the terminating subset `T(⊥)`, and
//! the per-state next-symbol transition probabilities. The terminal mass at
//! state `x` is the implicit `1 − Σ_y transitions[x][y]`; following BV 2025
//! Eq (11), this terminal mass appears in the Tsallis-entropy sum
//! `H_t(p_x)` because `p_x` is a probability mass function on `A ∪ {†}`.
//!
//! Per umbrella Q5, v0.1.0 is "BYO-LM": callers populate the transition
//! table from their own model. No closures, no LM runtime, no inference.
//! [`LmCategory::magnitude`] consumes the table by lifting it into a
//! [`LawvereMetricSpace<NodeId>`] via the `-ln π` embedding (Lawvere 1973;
//! BV 2025 §2.17) and calling [`magnitude::<F64Rig>`](crate::magnitude::magnitude).
//!
//! # BV 2025 paper anchors
//!
//! - §2.17 "Every LM defines a `[0, ∞]`-category": distance `d(x, y) :=
//!   −ln π(y|x)`; we materialize this directly.
//! - §3.5 Eq (7): `Mag(tM) = Σ_{x,y} ζ_t⁻¹(x, y)`.
//! - §3.10 Closed form: `Mag(tM) = (t − 1) · Σ_{x ∉ T(⊥)} H_t(p_x) +
//!   #(T(⊥))`. The two acceptance tests in `tests/bv_2025_acceptance.rs`
//!   verify this against the Möbius-sum form computed by
//!   [`magnitude`] function.
//! - §3.14 Cor: `d/dt Mag(tM)|_{t=1} = Σ_{x ∉ T(⊥)} H(p_x)` (Shannon
//!   entropy). Verified by central finite difference with `h = 1e-4 >
//!   TSALLIS_SHANNON_EPS`.

use std::collections::{HashMap, HashSet};

use crate::magnitude::magnitude;
use crate::weighted_cospan::NodeId;
use crate::{CatgraphError, F64Rig, LawvereMetricSpace, Tropical};

/// Materialized language-model transition table per BV 2025 §3.
///
/// Stores:
/// - `objects`: ordered list of state names, indexed left-to-right.
/// - `terminating`: subset of state names corresponding to `T(⊥)` — the
///   theoretically terminating states. Membership is BYO-LM, not derived
///   from the transition table.
/// - `transitions`: sparse `HashMap<from, HashMap<to, prob>>`. The terminal
///   mass at state `x` is the implicit `1 − Σ_y transitions[x][y]`, which
///   is treated as the weight of the virtual `†` symbol in the Tsallis
///   sum (BV 2025 Eq 11).
///
/// **Identity axiom.** The Lawvere metric `d(x, x) = 0` (i.e.
/// `π(x|x) = 1`) is enforced by [`magnitude`](Self::magnitude) when it
/// constructs the [`LawvereMetricSpace`] — callers do not have to populate
/// self-transitions.
#[derive(Clone, Debug, PartialEq)]
pub struct LmCategory {
    objects: Vec<String>,
    terminating: HashSet<String>,
    transitions: HashMap<String, HashMap<String, f64>>,
}

impl LmCategory {
    /// Build an empty LM category over a fixed object list. Terminating set
    /// and transitions both start empty; populate via
    /// [`add_transition`](Self::add_transition) and
    /// [`mark_terminating`](Self::mark_terminating).
    ///
    /// **Object names must be unique.** Transitions are keyed by name and
    /// [`yoneda`](Self::yoneda) resolves names first-match, so a duplicated
    /// name corrupts the name→index map (last-wins) and makes the positional
    /// [`yoneda_all`](Self::yoneda_all) disagree with per-name `yoneda`.
    ///
    /// # Panics
    ///
    /// Debug-only: panics on duplicate object names.
    #[must_use]
    pub fn new(objects: Vec<String>) -> Self {
        debug_assert!(
            objects.iter().collect::<HashSet<_>>().len() == objects.len(),
            "LmCategory object names must be unique"
        );
        Self {
            objects,
            terminating: HashSet::new(),
            transitions: HashMap::new(),
        }
    }

    /// Set the next-symbol probability `π(to | from) = prob`.
    ///
    /// Overwrites any prior value. Does NOT validate row normalization
    /// — leaky rows (`Σ_y π(y|from) < 1`) are intentional and represent
    /// the BV 2025 †-terminal mass at state `from`.
    ///
    /// **v0.1.1.** What was previously a `debug_assert!` on `prob ∈ [0, 1]`
    /// is now a release-mode `Result<(), CatgraphError>` (S1.1: prevents
    /// unbounded BFS in [`magnitude`](Self::magnitude) on `prob > 1.0`
    /// inputs that survived a release build). Membership in `objects` and
    /// non-trivial self-loop rejection (S1.2: BV 2025 §3 hypothesis forbids
    /// them — see [`magnitude`](Self::magnitude) acyclicity note) are also
    /// promoted.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if any of:
    /// - `from` is not in [`objects`](Self::objects)
    /// - `to` is not in [`objects`](Self::objects)
    /// - `prob` is not in `[0, 1]` (NaN excluded)
    /// - `from == to && prob > 0.0` (BV 2025 forbids non-trivial self-loops;
    ///   `prob == 0.0` is accepted as a no-op edge for caller convenience)
    pub fn add_transition(&mut self, from: &str, to: &str, prob: f64) -> Result<(), CatgraphError> {
        if !self.objects.iter().any(|o| o == from) {
            return Err(CatgraphError::Composition {
                message: format!("from state {from:?} not in objects"),
            });
        }
        if !self.objects.iter().any(|o| o == to) {
            return Err(CatgraphError::Composition {
                message: format!("to state {to:?} not in objects"),
            });
        }
        if !(0.0..=1.0).contains(&prob) {
            return Err(CatgraphError::Composition {
                message: format!("prob {prob} not in [0, 1]"),
            });
        }
        if from == to && prob > 0.0 {
            return Err(CatgraphError::Composition {
                message: format!(
                    "non-trivial self-loop from {from:?} to itself with prob {prob} \
                     forbidden by BV 2025 §3 acyclicity hypothesis"
                ),
            });
        }
        self.transitions
            .entry(from.to_owned())
            .or_default()
            .insert(to.to_owned(), prob);
        Ok(())
    }

    /// Mark a state as terminating (i.e. add it to `T(⊥)`).
    ///
    /// # Panics
    ///
    /// Debug-only: `state` must be in `objects`.
    pub fn mark_terminating(&mut self, state: &str) {
        debug_assert!(
            self.objects.iter().any(|o| o == state),
            "state {state:?} not in objects"
        );
        self.terminating.insert(state.to_owned());
    }

    /// Replay constructor — build an [`LmCategory`] from an explicit object
    /// list, terminating-states set, and an iterator of `(from, to, prob)`
    /// transition triples (v0.1.1).
    ///
    /// Designed for Phase 6C `magnitude_history` and `EventLogStore::replay`
    /// callers that reconstruct the LM state from an append-only audit log.
    /// Each triple is dispatched through [`add_transition`](Self::add_transition),
    /// so the same validation applies — invalid entries fail-fast with
    /// [`CatgraphError::Composition`].
    ///
    /// # Errors
    ///
    /// Returns the first [`CatgraphError`] from any failing
    /// [`add_transition`](Self::add_transition) call (unknown state,
    /// out-of-range probability, non-trivial self-loop). Invalid
    /// terminating-state names fail with a debug-only panic via
    /// [`mark_terminating`](Self::mark_terminating); release builds silently
    /// admit them but they do not affect magnitude.
    pub fn from_transition_log<I, S, T>(
        objects: Vec<String>,
        terminating: T,
        log: I,
    ) -> Result<Self, CatgraphError>
    where
        I: IntoIterator<Item = (S, S, f64)>,
        S: AsRef<str>,
        T: IntoIterator<Item = String>,
    {
        let mut m = Self::new(objects);
        for state in terminating {
            m.mark_terminating(&state);
        }
        for (from, to, prob) in log {
            m.add_transition(from.as_ref(), to.as_ref(), prob)?;
        }
        Ok(m)
    }

    /// Borrow the ordered object list.
    #[must_use]
    pub fn objects(&self) -> &[String] {
        &self.objects
    }

    /// Borrow the terminating-states set.
    #[must_use]
    pub fn terminating(&self) -> &HashSet<String> {
        &self.terminating
    }

    /// Borrow the transition table.
    #[must_use]
    pub fn transitions(&self) -> &HashMap<String, HashMap<String, f64>> {
        &self.transitions
    }

    /// Magnitude `Mag(tM)` of the LM at scale `t`, computed via Möbius sum
    /// (BV 2025 §3.5 Eq 7).
    ///
    /// Builds an `n × n` Lawvere metric space over `0..n` (`NodeId` =
    /// position in [`objects`](Self::objects)), populating distances per
    /// the **BV 2025 §2.10–2.17 prefix-extension semantics**:
    ///
    /// - `d(i, i) = 0` (identity axiom).
    /// - For every directed extension path `i = x₀ → x₁ → … → x_k = j`
    ///   recorded in the transition table, `π(j | i) = ∏_{ℓ} π(x_{ℓ+1} |
    ///   x_ℓ)` (BV 2025 Eq 6) and `d(i, j) = −ln π(j | i)`.
    /// - When no such path exists, the distance defaults to `Tropical(+∞)`
    ///   (i.e. `ζ_t[i][j] = 0`), per the convention `π(y | x) = 0` when `y`
    ///   is not an extension of `x` (BV 2025 §2.15).
    ///
    /// The transitive-closure computation is a forward BFS from each
    /// source node, multiplying probabilities along each path. **The
    /// transition table must be acyclic** for the resulting metric to
    /// satisfy BV 2025's tree-poset structure — otherwise the BFS may
    /// loop and the magnitude will not match the closed form of Thm 3.10.
    /// Acyclicity is the caller's responsibility; a debug-only assertion
    /// catches obvious self-loop cases. (Cyclic LMs are mathematically
    /// well-defined via the chain-sum Möbius formula but fall outside the
    /// poset hypothesis of Thm 3.10 — see BV 2025 §3.7 Remark.)
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if the t-scaled zeta is
    /// singular at this scale. Per BV 2025 Prop 3.6 `ζ_t` is invertible for
    /// any `t > 0` in the LM setting; singular results indicate caller
    /// inputs that violate the LM assumptions (e.g. degenerate parametric
    /// coincidences from cyclic transitions).
    ///
    /// Also returns [`CatgraphError::Composition`] if the BFS frontier cap
    /// (`n*n` steps per source) is exhausted (v0.1.1; defense-in-depth
    /// against malformed inputs that bypass [`add_transition`](Self::add_transition)
    /// validation — e.g. table populated by a future release-mode caller
    /// that constructs `transitions` directly via the `pub` API).
    ///
    /// # Panics
    ///
    /// Debug-only: panics if `t <= 0.0`. BV 2025 §3 only studies `t > 0`;
    /// behavior at `t ≤ 0` is unspecified (Tropical(`+∞`) vs Tropical(`-∞`)
    /// semantics on unset distances diverge). Release builds skip the check.
    pub fn magnitude(&self, t: f64) -> Result<f64, CatgraphError> {
        debug_assert!(
            t > 0.0,
            "magnitude(t) requires t > 0; behavior at t = {t} is unspecified per BV 2025 §3"
        );
        let space = self.enriched_space()?;
        let mag: F64Rig = magnitude(&space, t)?;
        Ok(mag.0)
    }

    /// Build the `[0, ∞]`-enriched Lawvere metric space `d(x, y) = −ln π(y | x)`
    /// over `0..n` (`NodeId` = position in [`objects`](Self::objects)), per the
    /// **BV 2025 §2.10–2.17 prefix-extension semantics** (BTV 2021 §5 metric view).
    ///
    /// - `d(i, i) = 0` (identity axiom).
    /// - For every directed extension path `i = x₀ → … → x_k = j` in the
    ///   transition table, `π(j | i) = ∏_ℓ π(x_{ℓ+1} | x_ℓ)` (BV 2025 Eq 6) and
    ///   `d(i, j) = −ln π(j | i)`; when several paths reach `j`, the
    ///   **highest-probability path wins** (see below).
    /// - When no such path exists the distance is left unset, and
    ///   [`LawvereMetricSpace::distance`] reports `Tropical(+∞)` (i.e.
    ///   `π(j | i) = 0`; BV 2025 §2.15).
    ///
    /// Shared substrate: [`magnitude`](Self::magnitude) lifts it through the
    /// Möbius sum; [`yoneda`](Self::yoneda) reads its rows as representable
    /// copresheaves (BTV 2021).
    ///
    /// # Cyclic tables and the max-probability-path contract
    ///
    /// **Acyclicity is required only by [`magnitude`](Self::magnitude)** — BV
    /// 2025's tree-poset hypothesis (Thm 3.10). `enriched_space` itself is
    /// well-defined on any [`add_transition`](Self::add_transition)-legal table,
    /// including cyclic (mutually-reachable) ones: `d(i, j)` is the
    /// max-probability path from `i` to `j`, computed by a strict-improvement
    /// label-correcting relaxation. Termination is guaranteed because a node is
    /// only revisited when its path probability strictly improves and edge
    /// probabilities are `≤ 1` — traversing a cycle never increases the product
    /// (a `prob = 1.0` cycle re-derives an equal value, which is *not* a strict
    /// improvement, so it does not oscillate). The semantic layer
    /// ([`yoneda_all`](Self::yoneda_all), `cluster_semantic_sym`, #21) relies on
    /// this contract to model mutual reachability (synonymy).
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] if the per-source BFS frontier cap
    /// (`n*n` steps) is exhausted — defense-in-depth against malformed inputs
    /// (`prob > 1.0` entries) that bypass
    /// [`add_transition`](Self::add_transition) validation.
    pub fn enriched_space(&self) -> Result<LawvereMetricSpace<NodeId>, CatgraphError> {
        let n = self.objects.len();
        let objects: Vec<NodeId> = (0..n).collect();
        let mut space = LawvereMetricSpace::new(objects);

        // Index each state name to its position in `self.objects`.
        let idx: HashMap<&str, usize> = self
            .objects
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i))
            .collect();

        // Identity axiom: d(i, i) = 0 ⇒ ζ[i][i] = 1.
        for i in 0..n {
            space.set_distance(i, i, Tropical(0.0));
        }

        // Forward-extension closure. For each source `i`, BFS through the
        // transition table, accumulating the multiplicative probability.
        // `best[j]` records the best (highest-probability) path so far — the
        // documented max-probability-path contract (see rustdoc above). On the
        // acyclic tree-poset tables magnitude() needs, the path is unique; on
        // DAG-with-rejoin or cyclic tables the highest weight wins, and the
        // strict `>` improvement test below is what guarantees termination
        // (equal-probability rederivations, e.g. a prob = 1.0 cycle, do not
        // re-enter the frontier).
        //
        // BFS termination cap (v0.1.1): at most `n * n` step transitions per
        // source. A well-formed acyclic LM yields O(n) steps; the n² cap is
        // defense-in-depth for callers who bypass `add_transition` validation
        // (see Errors note above).
        let frontier_cap = n.saturating_mul(n).max(1);
        for i in 0..n {
            let mut best: HashMap<usize, f64> = HashMap::new();
            best.insert(i, 1.0);
            let mut frontier: Vec<usize> = vec![i];
            let mut steps_remaining = frontier_cap;
            while let Some(cur) = frontier.pop() {
                if steps_remaining == 0 {
                    return Err(CatgraphError::Composition {
                        message: format!(
                            "LmCategory::enriched_space BFS frontier cap of {frontier_cap} \
                             steps exhausted from source {i}; check the transition \
                             table for cycles or `prob > 1.0` entries that bypass \
                             add_transition validation"
                        ),
                    });
                }
                steps_remaining -= 1;
                let cur_p = best[&cur];
                let cur_name = self.objects[cur].as_str();
                let Some(row) = self.transitions.get(cur_name) else {
                    continue;
                };
                for (next_name, &edge_p) in row {
                    if edge_p <= 0.0 {
                        continue;
                    }
                    let Some(&next) = idx.get(next_name.as_str()) else {
                        continue;
                    };
                    // Self-loops are rejected by add_transition (S1.2,
                    // v0.1.1) — this guard is unreachable on well-formed
                    // tables. Kept defensive for direct-mutation callers
                    // (none today; field is private since v0.1.0).
                    let new_p = cur_p * edge_p;
                    let prior = best.get(&next).copied().unwrap_or(0.0);
                    if new_p > prior {
                        best.insert(next, new_p);
                        frontier.push(next);
                    }
                }
            }
            // Write distances for every reached node `j != i`.
            for (j, p) in best {
                if j == i || p <= 0.0 {
                    continue;
                }
                space.set_distance(i, j, Tropical(-p.ln()));
            }
        }

        Ok(space)
    }
}
