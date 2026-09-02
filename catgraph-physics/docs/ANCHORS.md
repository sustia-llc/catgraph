# catgraph-physics — paper anchors & provenance

Unlike the workspace's theorem-anchored crates, catgraph-physics is
**inspiration-anchored**: no public item claims to implement a specific
numbered theorem, and the crate deliberately has no audit doc. This note
records where each attribution comes from and its verification status against
the private papers cache (paper PDFs are not kept in-tree; fetch via the
links below). Formalized from paper-audit Phase 4 findings (#124; the audit
umbrella is #116).

Status legend:

- **✅ cache-verified** — re-checked against the cached paper text.
- **(†) cache-unverifiable** — the source is not in the papers cache; the
  attribution is carried on standard-literature grounds. (†) means "not yet
  re-checked against the source", not "suspected wrong".
- **catgraph extrapolation** — a construction this crate adds on top of the
  cited substrate; the paper is inspiration, not spec.

## Bibliography

Cached:

- **[Gor23]** J. Gorard, *A functorial perspective on (multi)computational
  irreducibility* (2023) — [arXiv:2301.04690](https://arxiv.org/abs/2301.04690).
- **[Gor20a]** J. Gorard, *Some relativistic and gravitational properties of
  the Wolfram model*, Complex Systems **29**(2) (2020) 599–654 —
  [arXiv:2004.14810](https://arxiv.org/abs/2004.14810). Cache-verified
  2026-08-17 against the full text: causal invariance ⟺ a discrete version of
  general covariance, with updating-order changes as discrete gauge
  transformations (abstract; restated in the introduction; derived through
  §2.3 — updating-order ↔ ADM gauge freedom via the discrete-ADM analogy,
  with the general-covariance identification at the section's close; the
  equivalence phrasing is the paper's abstract claim, the in-text derivation
  is stated as causal invariance ⟹ general covariance). Cache holds arXiv v2
  (2021); journal pagination not in cache.
- **[Gor20b]** J. Gorard, *Some quantum mechanical properties of the Wolfram
  model*, Complex Systems **29**(2) (2020) — multiway / branchial-graph
  substrate. Cached as `gor20b-qm-wolfram` (journal-hosted PDF, placed
  2026-07-19); branchial-hypersurface + common-ancestor-separation lineage
  verified against §3.1 (branchlike hypersurfaces, :1195-1226;
  common-ancestor descent, :1276-1278) and §3.2 (ancestry-distance metric,
  :1611-1620).
- **[Oll09]** Y. Ollivier, *Ricci curvature of Markov chains on metric
  spaces*, J. Funct. Anal. **256**(3) (2009) 810–864. Cached as
  `math_0701886` (arXiv preprint; journal pagination differs). Definition 3
  (κ(x,y) := 1 − T₁(m_x,m_y)/d(x,y)) verified against the crate's κ.

Not cached (†):

- **[Vil03]** C. Villani, *Topics in Optimal Transportation*, Graduate
  Studies in Mathematics 58, AMS, 2003.
- **[EPS73]** H. Ehrig, M. Pfender, H. J. Schneider, *Graph-grammars: an
  algebraic approach*, 14th IEEE Symposium on Switching and Automata Theory
  (1973) — the original double-pushout (DPO) paper.

## Provenance table

| Site | Content | Source | Status |
|---|---|---|---|
| `multiway/evolution_graph.rs` module header; `trace.rs` (`is_irreducible`) | irreducibility ⟺ the computation→cobordism map is a **pure** symmetric monoidal functor; reducibility = deformation away from exactness | [Gor23] | ✅ cache-verified (audit Phase 4, PR #125 — an earlier inverted gloss was fixed there) |
| `multiway/branchial.rs` | branchial graph = per-step cross-section of the multiway evolution graph | [Gor23] substrate (multiway/branchial formalism; [Gor20b] lineage) | ✅ substrate cache-verified in [Gor23]; lineage cache-verified in [Gor20b] §3.1–3.2 (2026-08-17) |
| `multiway/branchial_spectrum.rs` (λ₂ / Fiedler value, spectral gap, Fiedler bisection, spectral clustering) | algebraic connectivity as a reducibility/irreducibility proxy | **catgraph extrapolation** — [Gor23] contains no spectral/Laplacian/eigenvalue content (`rg -i 'laplacian\|spectral\|eigen\|fiedler'` over the cached text: zero hits). The branchial substrate is Gorard's; the spectral layer is ours. | in-source wording fixed under #124 |
| `multiway/branchial_analysis.rs` (`multiway_betweenness`, `multiway_katz`, `MultiwayEvolutionGraph::to_petgraph`) | betweenness (Brandes) and Katz centrality on multiway evolution graphs; eigenvector centrality deliberately NOT shipped (nilpotent adjacency of a step-graded DAG) | **catgraph extrapolation** — standard-literature algorithms via rustworkx-core; no Gorard content; rationale in CHANGELOG v0.14.0 | added v0.14.0 (#161, #162) |
| `hypergraph/gauge.rs` (`GaugeGroup`, `HypergraphRewriteGroup`, Wilson loops, plaquette action, "causal invariance ⟺ flat gauge field / holonomy = 1") | gauge-theoretic reading of hypergraph rewriting | inspired by [Gor20a]'s causal-invariance-as-gauge-covariance; the Wilson-loop / plaquette / holonomy vocabulary is standard lattice-gauge-theory machinery; the "causal invariance ⟺ flat gauge field (holonomy = 1)" equivalence is a **catgraph interpretive gloss**, not a stated theorem of any cached paper | [Gor20a] inspiration cache-verified (abstract + §2.3, 2026-08-17); the ⟺-gloss remains a catgraph interpretation |
| `hypergraph/rewrite_rule.rs`, `rewrite_span.rs` (rule as span `L ← K → R`) | double-pushout (DPO) graph rewriting | [EPS73] (classical source) | (†) — attribution only |
| `multiway/ollivier_ricci.rs`, `wasserstein.rs` | `κ(x,y) = 1 − W₁(μ_x, μ_y)/d(x,y)` with uniform neighbor measures; `W₁` by successive-shortest-path min-cost flow over equal-mass non-negative marginals | [Oll09] (definition), [Vil03] (optimal transport / `W₁`) | [Oll09] cache-verified (Def 3, 2026-08-17); [Vil03] (†) |
| `multiway/ollivier_ricci.rs` (`branchial_complexity` unit clamp) | `κ ≤ 1` holds definitionally (`W₁ ≥ 0`); the two-sided `\|κ\| ≤ 1` clamp is a **normalization convention**, not a theorem — negative Ollivier curvature on unweighted graphs is not bounded below by −1 in the standard literature | [Oll09] + standard literature | hedged in-source under #124 |

## Non-anchors

- **Mamba / state-space models** — analogy only, explicitly labelled as such
  in `evolution_graph.rs`; not part of this crate's citation surface.
- **Bradley–Vigneaux 2025** (arXiv:2501.06662) — cross-referenced in
  `evolution_graph.rs` for the discretization-functor pattern; its anchor
  home is catgraph-magnitude.
