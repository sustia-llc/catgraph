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

Not cached (†):

- **[Gor20a]** J. Gorard, *Some relativistic and gravitational properties of
  the Wolfram model*, Complex Systems **29**(2) (2020) 599–654 —
  [arXiv:2004.14810](https://arxiv.org/abs/2004.14810). Abstract claim
  (arXiv-page-verified 2026-07-19): causal invariance ⟺ a discrete version of
  general covariance, with updating-order changes as discrete gauge
  transformations.
- **[Gor20b]** J. Gorard, *Some quantum mechanical properties of the Wolfram
  model*, Complex Systems **29**(2) (2020) — multiway / branchial-graph
  substrate.
- **[Oll09]** Y. Ollivier, *Ricci curvature of Markov chains on metric
  spaces*, J. Funct. Anal. **256**(3) (2009) 810–864.
- **[Vil03]** C. Villani, *Topics in Optimal Transportation*, Graduate
  Studies in Mathematics 58, AMS, 2003.
- **[EPS73]** H. Ehrig, M. Pfender, H. J. Schneider, *Graph-grammars: an
  algebraic approach*, 14th IEEE Symposium on Switching and Automata Theory
  (1973) — the original double-pushout (DPO) paper.

## Provenance table

| Site | Content | Source | Status |
|---|---|---|---|
| `multiway/evolution_graph.rs` module header; `trace.rs` (`is_wolfram_irreducible`) | irreducibility ⟺ the computation→cobordism map is a **pure** symmetric monoidal functor; reducibility = deformation away from exactness | [Gor23] | ✅ cache-verified (audit Phase 4, PR #125 — an earlier inverted gloss was fixed there) |
| `multiway/branchial.rs`, `branchial_analysis.rs` | branchial graph = per-step cross-section of the multiway evolution graph | [Gor23] substrate (multiway/branchial formalism; [Gor20b] lineage) | ✅ substrate cache-verified in [Gor23]; lineage (†) |
| `multiway/branchial_spectrum.rs` (λ₂ / Fiedler value, spectral gap, Fiedler bisection, spectral clustering) | algebraic connectivity as a reducibility/irreducibility proxy | **catgraph extrapolation** — [Gor23] contains no spectral/Laplacian/eigenvalue content (`rg -i 'laplacian\|spectral\|eigen\|fiedler'` over the cached text: zero hits). The branchial substrate is Gorard's; the spectral layer is ours. | in-source wording fixed under #124 |
| `hypergraph/gauge.rs` (`GaugeGroup`, `HypergraphRewriteGroup`, Wilson loops, plaquette action, "causal invariance ⟺ flat gauge field / holonomy = 1") | gauge-theoretic reading of hypergraph rewriting | inspired by [Gor20a]'s causal-invariance-as-gauge-covariance; the Wilson-loop / plaquette / holonomy vocabulary is standard lattice-gauge-theory machinery; the "causal invariance ⟺ flat gauge field (holonomy = 1)" equivalence is a **catgraph interpretive gloss**, not a stated theorem of any cached paper | (†) |
| `hypergraph/rewrite_rule.rs`, `rewrite_span.rs` (rule as span `L ← K → R`) | double-pushout (DPO) graph rewriting | [EPS73] (classical source) | (†) — attribution only |
| `multiway/ollivier_ricci.rs`, `wasserstein.rs` | `κ(x,y) = 1 − W₁(μ_x, μ_y)/d(x,y)` with uniform neighbor measures; `W₁` by transportation simplex | [Oll09] (definition), [Vil03] (optimal transport / `W₁`) | (†) |
| `multiway/ollivier_ricci.rs` (`branchial_complexity` unit clamp) | `κ ≤ 1` holds definitionally (`W₁ ≥ 0`); the two-sided `\|κ\| ≤ 1` clamp is a **normalization convention**, not a theorem — negative Ollivier curvature on unweighted graphs is not bounded below by −1 in the standard literature | [Oll09] + standard literature | hedged in-source under #124 |

## Non-anchors

- **Mamba / state-space models** — analogy only, explicitly labelled as such
  in `evolution_graph.rs`; not part of this crate's citation surface.
- **Bradley–Vigneaux 2025** (arXiv:2501.06662) — cross-referenced in
  `evolution_graph.rs` for the discretization-functor pattern; its anchor
  home is catgraph-magnitude.
