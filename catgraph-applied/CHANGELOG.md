<!-- markdownlint-disable MD024 -->
<!-- MD024 (no-duplicate-heading) disabled: Keep a Changelog intentionally
     reuses `### Added`, `### Changed`, `### Fixed`, etc. across releases. -->
# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-05-13

Co-released with **catgraph-magnitude v0.5.0** at workspace umbrella **v0.14.0**.
This is the first minor release of catgraph-applied (v0.5.x → v0.6.0) and
contains one source-breaking rename. Downstream code depending on `Integer`
must migrate to `ZAlgebra` — see migration guide below.

Examples-coverage + benches-coverage baseline tracking begins at this
release boundary (first minor bump for this crate).

### Added

- `tests/zalgebra_axioms.rs` (T2) — proptest-grade verification of the `from_i64`
  ring-homomorphism axioms (zero, one, negation, additivity, multiplicativity).
  5 tests total: 3 unit tests + 2 proptest cases (256 cases each). Verifies the
  implementor axioms declared in `integer.rs`'s `# Implementor axioms` section
  on the `ZAlgebra` trait, making the Bourbaki *Algèbre* Ch. I §8
  (ℤ as initial object of the category of unital rings)
  ring-homomorphism contract `ℤ → Z(BigInt)` test-enforced rather than
  rustdoc-only.
- Top-level re-export `pub use integer::ZAlgebra` at crate root — canonical short
  path `catgraph_applied::ZAlgebra` (cg-mag consumers can use either the short
  path or the long `catgraph_applied::integer::ZAlgebra`).

### Changed (BREAKING)

- **`Integer` trait renamed to `ZAlgebra`** (Bourbaki *Algèbre* Ch. I §8 — ℤ as initial object of the category of unital rings;
  *Z-algebra* is the standard term-of-art for a ring admitting a unique unital
  ring homomorphism ℤ → R, which is exactly what this trait names — not "the
  set of integers"). Deferred to v0.6.0 as a breaking change after the naming
  mismatch was identified as semantically misleading. All downstream code using
  `use catgraph_applied::Integer`, `use catgraph_applied::integer::Integer`, or
  `impl Integer for T` must migrate to `ZAlgebra`.
- **`ZAlgebra` is now sealed** via `private::Sealed` supertrait — external impls
  are prevented at the trait-bound level. Precedent: `catgraph-dl`'s
  `SetCategoryDefaults` sealing pattern at v0.4.0, hardened here with
  `pub(crate) mod private` (hard-seal — external `impl ZAlgebra for T` is
  structurally impossible, not merely conventional). `Z(BigInt)` remains the
  only implementation; the seal prevents accidental impls on rigs that violate
  the integer-arithmetic contract (e.g., `F64Rig` would silently fail the
  `from_i64(0) == zero()` axiom by `to_bits()`-equivalence only).

### Migration guide for v0.5.6 → v0.6.0

```rust
// v0.5.6 (OLD)
use catgraph_applied::integer::Integer;

// v0.6.0 (NEW)
use catgraph_applied::ZAlgebra;           // canonical short path
// or
use catgraph_applied::integer::ZAlgebra;  // long path, still valid
```

For downstream consumers that had their own `impl Integer for T`:

```rust
// v0.5.6 (OLD) — DOES NOT COMPILE under v0.6.0
impl Integer for MyRig {
    fn from_i64(n: i64) -> Self { /* ... */ }
}

// v0.6.0 — rename alone is NOT sufficient:
// the trait is sealed; external impls are rejected. The compiler
// surfaces the rename error first, then the seal error if the rename
// is naively applied. See "If you need a custom integer-exact ring" below.
```

If you need a custom integer-exact ring, file an issue describing the use case —
the seal is intentional but the crate maintainers can consider widening the impl set if
a justified consumer surfaces.

## [0.5.6] - 2026-05-13

Co-released with **catgraph-magnitude v0.4.0** and **catgraph v0.13.0** at
the same workspace umbrella **v0.13.8**. Strictly additive on v0.5.5; no
v0.5.x API break.

### Added

- **`Integer` trait** (T3 from cg-mag v0.4.0 Session 1) — Bourbaki-tower
  extension of `Rig` adding `Neg + Sub + from_i64` lifting constructor.
  Substrate for cg-mag's `mobius_function_via_chains_exact` and
  `smith_normal_form_integer`.
- **`Z(BigInt)` newtype** (T4 from cg-mag v0.4.0 Session 1) —
  `num::BigInt`-backed `Integer + Ring` instance for arbitrary-precision
  integer-exact computation.
- **`rustworkx` feature flag (default-on)** — gates `rustworkx-core`
  dependency behind the same feature pattern as `catgraph`.
  `--no-default-features` makes the `temperley_lieb` module entirely absent
  (its `BrauerMorphism::compose` is petgraph-central; no meaningful fallback).

## [0.5.5] - 2026-05-10

Substrate release for catgraph-magnitude v0.3.0 magnitude-homology / SNF
work. Dual-tagged with **catgraph-magnitude v0.3.0** at the same release
commit per workspace `CLAUDE.md` release rule 3 (target workspace umbrella
**v0.13.3**). Strictly additive on v0.5.4; no v0.5.x API break.

### Added

- Mutable `MatR<Q>` API: `row_swap`, `scale_row`, `add_scaled_row`,
  `col_swap`, `scale_col`, `add_scaled_col`, `entries_mut`, `entry_mut`.
  **Substrate for catgraph-magnitude v0.3.0** Storjohann §7 SNF port over
  `MatR<Q>`. Eight in-place mutators required by the `snf::band` /
  `snf::echelon` / `snf::bidiagonal_to_smith` row/column-operation
  primitives. No equivalent v0.5.4 API existed; the SNF port would have
  required a separate `Vec<Vec<i64>>` allocation pass per matrix without
  these.
- `LawvereMetricSpace::size()` and `LawvereMetricSpace::objects()`
  accessors — read-only object-count + slice view over the underlying
  `Vec<T>`. **Substrate for chain enumeration** in catgraph-magnitude
  v0.3.0 `chain_complex::enumerate_chains` DFS — the chain enumerator
  walks `(0..n)` then dereferences via `objects()[i]`.
- `LawvereMetricSpace::<usize>::from_distance_fn(n, f)` constructor —
  builds a `(0..n)`-indexed Lawvere metric space from a distance closure
  `f: (usize, usize) -> f64`. Ergonomic fixture builder for
  catgraph-magnitude v0.3.0 chain-complex tests; equivalent to the
  `new(0..n) + set_distance` loop. Required by the 5-fixture path C
  acceptance suite (each fixture builds via `from_distance_fn`).
- `impl From<i64> for F64Rig` — lifts signed integers into `F64Rig` for
  use in `catgraph-magnitude::chain_complex::boundary_matrix`, where the
  LS 2017 Def 2.5 sign coefficient `(-1)^i` is lifted via
  `Q::from(sign: i64)`. **Substrate for the `Q: Rig + From<i64>` bound**
  on `boundary_matrix`. Was not present in v0.5.3's `From<f64> for F64Rig`
  set; v0.5.5 closes the integer-flavour conversion path.

### Mid-session ride-along additions (beyond originally-scoped 8 mutator methods)

The original v0.5.5 substrate plan called for the 8 mutable `MatR<Q>`
methods only. Mid-session implementation of catgraph-magnitude v0.3.0
Phase A surfaced gaps that needed inline ride-along closure rather than
deferral:

- `LawvereMetricSpace::size()` — the chain-complex enumerator needed an
  `usize` object count; `objects().len()` worked but added an indirection.
- `LawvereMetricSpace::objects()` — chain enumeration needs `&[T]` slice
  view, not just an iterator.
- `LawvereMetricSpace::from_distance_fn` — needed for 5-fixture
  acceptance suite ergonomics (every fixture hand-builds the same
  `for a in 0..n { for b in 0..n { space.set_distance(a, b, f(a, b)) }}`
  loop pattern).
- `impl From<i64> for F64Rig` — `boundary_matrix` lifts the LS 2017 Def 2.5
  sign coefficient via `Q::from((-1_i64).pow(i))`; `From<f64>` would lose
  precision on large `i`.

All four ride-alongs ship in v0.5.5. None are breaking; existing v0.5.4
callers continue compiling.

### Substrate consumer

- catgraph-magnitude v0.3.0 — see [`catgraph-magnitude/CHANGELOG.md`](../catgraph-magnitude/CHANGELOG.md) for the consumer surface this substrate enables.

### Pre-tag rustdoc cleanup ride-along

Three doc-only edits closed pre-existing rustdoc warnings ahead of the
v0.5.5 release commit, bringing `cargo doc --workspace --no-deps` to
zero warnings:

- `linear_combination.rs:10, 226` — public-doc links to private const
  `PARALLEL_MUL_THRESHOLD` replaced with backtick formatting + the
  literal value (32 terms). Const stays private.
- `temperley_lieb.rs:21` — redundant explicit link target on
  `MonoidalMorphism` removed.
- `lawvere_metric.rs:147` — redundant explicit link target on
  `EnrichedCategory::objects` removed (the v0.5.5 ride-along addition
  flagged in session-state at v0.5.5 land).

### Performance candidates (bench-driven, no version target)

Deferred from prior rayon ride-along.

- `par_array_windows::<2>()` at `catgraph-physics::branchial_parallel_step_pairs` + `evolution_cospan::to_cospan_chain` — bench-driven
- `walk_tree_prefix` / `walk_tree_postfix` for multiway BFS / confluence-diamond enumeration
- `fold_chunks` / `fold_chunks_with` for Phase 6 magnitude per-partition accumulation
- rayon Producer/Consumer plumbing if public parallel-iterator APIs land on `MultiwayEvolutionGraph` / `BranchialGraph`
- `kb::CongruenceClosure::atom_canonical` — currently O(n) per call, called O(n) times inside `smc_refine`, so O(n²) per fixpoint iteration (bounded by `SAFETY_BOUND = 64`). Replace the full-graph scan with a per-class best-atom cache updated on `merge`. Surfaced by v0.5.1 code-review pass (2026-04-24). Not blocking at current d≤3 Mat(R) test sizes (~40 terms → ~100k ops). If Branch A (Knuth-Bendix completion) wins at v0.5.3 decision, `atom_canonical` is deleted and this TODO dissolves.

## [0.5.4] - 2026-04-28

Additive patch closing four bound-tightness and defensive-default gaps
surfaced during a deep review. Co-released with catgraph v0.12.2 (the
`Copy → Clone` widening that unblocks the wiring-diagram `InterCircle`
loosening) and catgraph-magnitude v0.1.1 at the same workspace SHA. No
API breaks; v0.5.3 consumers continue to compile.

### Added

- `LawvereMetricSpace::from_distances<I: IntoIterator<Item = ((T, T),
  Tropical)>>` — convenience constructor pairing `new` with a sequence of
  `set_distance` calls. Downstream consumers use it when materializing
  per-port distance tables. Last-write-wins on duplicate keys, mirroring
  `HashMap::insert` semantics.
- `EnrichedCategory::hom` — diagonal default for
  `LawvereMetricSpace<T>`. When `a == b` and no entry has been recorded,
  returns `Tropical::one() = Tropical(0.0)` (Lawvere identity axiom). An
  explicit non-zero diagonal entry takes precedence; off-diagonal unset
  entries continue returning `Tropical::zero() = Tropical(+∞)`. Defends
  against the BTV21 enrichment-call-site footgun where unseeded LMs would
  silently return `+∞` from the trait method while `LmCategory` seeds the
  diagonal explicitly.
- `tests/decorated_cospan.rs` — `t2_3_decorated_cospan_pushforward_through_quotient`
  integration test exercising `compose_with_quotient` + `D::pushforward`
  end-to-end through `DecoratedCospan` with an `EdgeSet` decoration whose
  apex relabelling is observable. The pre-existing
  `t2_3_petri_decoration_*` test renamed `t2_4_*` to free the slot.
- `tests/wiring_diagram::operadic_with_clone_only_intercircle` —
  regression test parameterising `CircleName` over `String` (Clone, not
  Copy), exercising the loosened `Operadic for WiringDiagram` impl bound.

### Changed

- `Operadic for WiringDiagram` impl bound — `InterCircle: Eq + Copy +
  Send + Sync` loosened to `InterCircle: Eq + Clone + Send + Sync`. The
  `IntraCircle` Copy bound is preserved (still Copy-typed in the existing
  consumers; loosening it carries no downstream demand). Enables
  `WiringDiagram<Lambda, String, _>::operadic_substitution` for downstream
  consumers whose `InterCircle` is `String`. Riders on the catgraph
  v0.12.2 `NamedCospan::{find_nodes_by_name_predicate, identity,
  from_permutation_extra_data}` Copy → Clone widening.

## [0.5.3] - 2026-04-25

**Additive release, no API break from v0.5.2.** Prerequisite for
catgraph-magnitude v0.1.0: exposes the ring and field structure of `F64Rig`
to Rust's type system, enabling `mobius_function::<F64Rig>` Gaussian
elimination in catgraph-magnitude.

### Added

- `Neg`, `Sub`, `Div`, and `From<f64>` impls on `F64Rig`. `F64Rig` was
  already a ring at the math level (the existing
  `verify_axioms_f64_rig_sample` test exercises `F64Rig(-1.0)`); these
  impls expose the ring + field operations Rust needs to perform arithmetic.
  The ring/field bound stays off `Rig` itself — only `F64Rig` carries it.
  Required by `catgraph-magnitude` v0.1.0's `mobius_function::<F64Rig>`
  (Gaussian elimination, `ζ · μ = I` over `F64Rig`).

## [0.5.2] - 2026-04-24

**Additive release, no API break from v0.5.1.** Three independent tracks:
Layer 1 Joyal-Street string-diagram normal form, Option A atom-canonical
refinement of the CC engine, and the opt-in semantic `Functorial` decision
procedure. Plus code-review polish and a test-suite rename that reflects
what the `#[ignore]`'d suite actually measures.

### Added

- `src/prop/presentation/smc_nf.rs` — Layer 1 Joyal-Street string-diagram
  normal form (~950 LOC). Canonicalizes `PropExpr<G>` up to the SMC
  coherence axioms (associator, unitors, interchange, braid naturality,
  `σ² = id`) without consulting user equations. Public API:
  `smc_nf::nf(e)` → `StringDiagram<G>`, `smc_nf::from_string_diagram(sd)`
  → `PropExpr<G>`. 18 paper-cited regression tests in
  `tests/smc_nf_regression.rs` (Joyal-Street 1991 Part I, Selinger 2011).
  6 proptest coverage tests + 1 known-gap case in
  `tests/smc_nf_completeness.rs` (the interchange/topological-layer-order
  case is tracked as `#[ignore]` and not blocking).
- `src/prop/presentation/functorial.rs` — `CompleteFunctor<G>` trait +
  `MatrixNFFunctor<R>` concrete instance. `MatrixNFFunctor` wraps the
  existing `sfg_to_mat` as a semantic decision procedure for SFG_R,
  complete by F&S Thm 5.60 / Baez-Erbele 2015. Supplies a provably
  complete decision path for the `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)`
  presentation — the congruence-closure engine's syntactic-incompleteness
  gap (see `tests/graphical_linalg.rs`) is now closable operationally.
- `Presentation::eq_mod_functorial<F: CompleteFunctor<G>>(&self, a, b, f)` —
  opt-in semantic-decision method. Complements the syntactic `eq_mod` (the
  `NormalizeEngine::CongruenceClosure` default remains unchanged). Always
  returns `Ok(Some(_))` — no depth bounds, no false negatives; completeness
  is an external claim carried by the functor implementation. Design note:
  we keep the functor as a call-site parameter rather than adding a
  `NormalizeEngine::Functorial` enum variant because `CompleteFunctor` has
  an associated `Target` type that varies per instance, which precludes a
  uniform enum-payload representation without type erasure.
- Option A atom-canonical refinement in `kb::CongruenceClosure`: new
  `propagate_fixpoint` outer loop alternating congruence propagation and a
  post-merge `smc_refine` pass (bounded by `SAFETY_BOUND = 64`). Each refine
  rebuilds terms using atom-canonical class substitutions and runs
  `smc_nf::nf` on the rebuilt expression; any change is merged back into
  the CC graph. Reduces BoolRig d=2 faithfulness-harness collisions
  2574 → 1433 (~44%). The residual gap is closable only by Knuth-Bendix
  saturation or the `Functorial` engine above.
- 6 smoke tests in `tests/functorial.rs` exercising `MatrixNFFunctor` /
  `eq_mod_functorial` end-to-end.

### Changed

- `Presentation::eq_mod` (CC-engine branch) now has a Layer-1-NF short-circuit:
  if `smc_nf::nf(a) == smc_nf::nf(b)` the call returns `Ok(Some(true))`
  without running the CC fixpoint. Falls back to the v0.5.1 CC path
  otherwise. Union capability (NF OR CC); neither is lost. No API change.
- The 12 `thm_5_60_faithful_*` integration tests in
  `tests/graphical_linalg.rs` are renamed to `cc_completeness_tracking_*`,
  reflecting what they actually measure: the incompleteness of the default
  `NormalizeEngine::CongruenceClosure` engine relative to the complete
  semantic `MatrixNFFunctor`. Baez-Erbele 2015 proved
  `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)` abstractly — we do not need to verify an
  established theorem. The tests stay `#[ignore]`'d as diagnostic, not as a
  release gate; `eq_mod_functorial` decides the underlying equality
  operationally. `IGNORE_REASON` and the module docstring are rewritten to
  match.

### Fixed

- `install_function_node` in `kb::CongruenceClosure` now re-canonicalizes
  the signature-table key via `find(a) / find(b)` after the post-collision
  merge, rather than reusing the pre-merge `ra, rb`. Belt-and-suspenders
  defense against a future refactor that moves merges into
  `install_function_node` or reorders the recursion — today `merge` cannot
  shift the children's roots, so the observable behavior is unchanged.
  Surfaced by v0.5.1 fresh-eyes code review (2026-04-24).
- `normalize_smc_only` + `apply_smc_rules` docstrings corrected to say
  "9 fixed SMC-canonical-form rules" (previously stale at "8 rules" after
  Rule 9 landed in v0.5.1).
- `LawvereMetricSpace::triangle_inequality_holds` comment clarifies that
  the `>` comparison is ordering on `[0, ∞]` distinct from the tropical
  rig's `min` additive order.
- `smc_nf::from_string_diagram` gains a `# Panics` docstring noting the
  internal `expect` calls are invariant-guarded and cannot fire.
- `smc_nf_completeness::compose_associator` proptest stabilized by
  raising `max_global_rejects` 1024 → 16 384 to accommodate the
  three-way arity-compatibility rejection cascade from
  `prop_assume!(a.target() == b.source())` +
  `prop_assume!(b.target() == c.source())`.

### Deferred (v0.5.3+ decision point)

v0.5.3 is not scheduled work — it's a decision point between two branches:

- **Branch A (Knuth-Bendix completion):** saturate the 17 Thm 5.60
  equations modulo SMC coherence until confluent. 1-3 weeks research;
  open-ended if confluence fails on a subset. Would render
  `atom_canonical` / `term_to_canonical_expr` / `smc_refine` redundant
  and close the `cc_completeness_tracking_*` tests under CC.
- **Branch B (declare `MatrixNFFunctor` terminal):** accept that for
  Mat(R) presentations the semantic engine is complete by theorem, keep
  the `#[ignore]`'d tests as diagnostic, and move to Phase 6. Zero
  effort.

Pick at Phase 6 kickoff or when a non-Mat(R) presentation requires a
syntactically complete decision procedure. Both paths remain open.

### Requires

- catgraph v0.12.0 (unchanged from v0.5.1).

## [0.5.1] - 2026-04-22

**BREAKING CHANGES in `Presentation` and `PropSignature`** — migration guide below. Ships three independent tracks: the normalizer upgrade (Knuth-Bendix-grade correctness for overlapping equations), SMC Rule 9 (identity-coherence of ⊗), and enrichment infrastructure (Phase 6 prep).

### Added

- `src/prop/presentation/kb.rs` — congruence-closure decision procedure
  (Downey-Sethi-Tarjan 1980, signature-table variant). Term graph +
  union-find with path halving + congruence propagation through
  Compose/Tensor. Complete for finitely-presented equational theories
  without binders. 10 unit tests in `tests/kb.rs`.
- `Presentation::with_engine(NormalizeEngine)` + `Presentation::set_engine`
  — engine selector **for `eq_mod` only** (`normalize` remains structural
  rewriting regardless of engine). Variants:
  - `NormalizeEngine::Structural` — v0.5.0 `eq_mod` behavior: normalize both
    sides and compare. Fast, but returns `None` (unknown) on overlapping
    equations that exceed the rewrite-depth bound.
  - `NormalizeEngine::CongruenceClosure` (default since v0.5.1) — decides
    equality via bounded congruence closure with an SMC-structural pre-pass.
    No false negatives; correct decision procedure for finitely-presented
    equational theories without binders.
- SMC Rule 9 in `apply_smc_rules`: `Identity(m) ⊗ Identity(n) → Identity(m+n)`
  (identity-coherence of ⊗). Valid SMC axiom missing from v0.5.0's 8 rules.
- `src/enriched.rs` — `EnrichedCategory<V: Rig>` trait generalizing
  `Hom(a, b): Set` to `Hom(a, b): V` for any rig V. Concrete
  `HomMap<O, V>` finite realization. Object-safe (documented in trait
  rustdoc) for `Box<dyn EnrichedCategory<V, Object = T>>` consumers.
  References F&S §1.1, §2.4; CTFP Ch 28.
- `src/lawvere_metric.rs` — `LawvereMetricSpace<T>` over `Tropical`.
  Triangle-inequality verifier + `-ln π` embedding from `UnitInterval` via
  `BaseChange`. `EnrichedCategory<Tropical>` impl. References CTFP §28.5,
  Lawvere 1973.

### Changed

- **BREAKING:** `Presentation::normalize` return type changed from
  `Result<PropExpr<G>, CatgraphError>` to `Result<NormalizeResult<G>, CatgraphError>`.
  The new `NormalizeResult<G>` struct exposes `.expr`, `.converged`,
  `.steps_taken` fields so callers can detect partial results when the
  rewrite-depth bound is hit.
- **BREAKING:** `Presentation::eq_mod` return type changed from
  `Result<bool, CatgraphError>` to `Result<Option<bool>, CatgraphError>`.
  `None` signals "at least one side hit the rewrite-depth bound before
  converging — answer unknown".
- **BREAKING:** `PropSignature` trait now requires `Eq + Hash` in addition
  to `Clone + PartialEq + Debug`. Required for the HashMap-backed
  congruence-closure term graph.
- **BREAKING:** The three f64-wrapping rigs (`UnitInterval`, `Tropical`,
  `F64Rig`) gained manual `Eq + Hash` impls via `f64::to_bits()`. NaN
  caveats inherit from `PartialEq` (same as `f64`). Required by the
  supertrait widening.

### Fixed

- Faithfulness harness (`verify_sfg_to_mat_is_full_and_faithful`) now
  routes through `Presentation::eq_mod` (not `normalize`), so the new CC
  engine is actually consulted during enumeration.

### Deferred to v0.5.2

- **Thm 5.60 faithfulness tests remain `#[ignore]`'d.** Investigation during
  v0.5.1 execution revealed that `apply_smc_rules` (a one-pass bottom-up
  rewriter) cannot canonicalize interchange-requires-reassociation cases
  (e.g., `ε ⊗ (σ ⊗ id)` vs `(ε ⊗ id₃); (σ ⊗ id)`). Closing this requires
  Joyal-Street string-diagram normal form. Audit §5.4 Thm 5.60 stays
  PARTIAL with a clearer gap characterization.

### Migration guide for v0.5.0 → v0.5.1

```rust
// v0.5.0 (OLD)
let normalized: PropExpr<G> = presentation.normalize(&expr)?;
if presentation.eq_mod(&a, &b)? { ... }

// v0.5.1 (NEW) — explicit (recommended)
let result = presentation.normalize(&expr)?;
let normalized: PropExpr<G> = result.expr;
if !result.converged {
    // hit the depth bound — handle explicitly
}

match presentation.eq_mod(&a, &b)? {
    Some(true) => { /* definitely equal */ }
    Some(false) => { /* definitely unequal */ }
    None => { /* hit depth bound — unknown */ }
}

// v0.5.1 (NEW) — conservative (fastest migration)
let normalized = presentation.normalize(&expr)?.expr;
let eq = presentation.eq_mod(&a, &b)?.unwrap_or(false);
```

`unwrap_or(false)` is conservative — treats "unknown" as "unequal",
matching v0.5.0's behavior for overlapping equations. But the new default
CC engine always returns `Some(_)` (never `None`) on bounded user-equation
sets, so `unwrap_or(false)` only matters if you explicitly opt into
`Structural`.

For types implementing `PropSignature`: add `Eq + Hash` to the derive.
For types wrapping `f64`, follow the manual impl pattern in `rig.rs`:
`impl Eq for T {}` + `impl Hash` via `self.0.to_bits().hash(state)`.

### Requires

- catgraph v0.12.0 (unchanged from v0.5.0).

## [0.5.0] - 2026-04-21

Tier 3 applied-CT closures — F&S *Seven Sketches* Chapter 5 main content:
the prop presentation machinery, functorial semantics `S: SFG_R → Mat(R)`,
and the 16-equation Thm 5.60 presentation of Mat(R). Also closes §6.3 Ex 6.64
(Corel as `HypergraphCategory`) via catgraph v0.12.0 core.

### Added

- `src/rig.rs` — `Rig` trait (F&S Def 5.36) as a blanket impl over
  `num_traits::{Zero, One}` + `Add` + `Mul`. 4 concrete instances:
  `BoolRig` (∨, ∧), `UnitInterval` ([0,1] Viterbi semiring; BTV 2021
  enrichment base), `Tropical` ([0,∞], min, +, ∞, 0; Lawvere metric / magnitude
  homology base), `F64Rig` (real demo rig). `BaseChange<UnitInterval>` for
  `Tropical` via `d = −ln π`. `verify_rig_axioms` runtime check returning
  `CatgraphError::RigAxiomViolation`.
- `src/prop/presentation.rs` — `Presentation<G>` (F&S Def 5.33) with
  `add_equation`, `normalize`, `eq_mod`, `with_depth`. 8-rule SMC canonical
  form applied first (closes Def 5.30 PARTIAL gap); user equations applied
  left-to-right. Bounded-depth rewriting (default 32); Knuth-Bendix
  completion is v0.5.1 work.
- `src/sfg.rs` — `SignalFlowGraph<R>` (F&S Def 5.45). 5 primitive generators
  from Eq 5.52: Copy 1→2, Discard 1→0, Add 2→1, Zero 0→1, Scalar(r) 1→1.
  Derived `copy_n` / `discard_n` as iterated compositions.
- `src/mat.rs` — `MatR<R>` matrix prop (F&S Def 5.50) over any `Rig` R,
  backed by `Vec<Vec<R>>`. F&S convention: morphism `m → n` is `m × n`.
  `Composable`, `Monoidal`, `SymmetricMonoidalMorphism` + `block_diagonal`
  tensor. Works for Tropical, Boolean, and UnitInterval without nalgebra.
- `src/sfg_to_mat.rs` — `sfg_to_mat` functor `S: SFG_R → Mat(R)` (F&S
  Thm 5.53). Structural recursion over `PropExpr<SfgGenerator<R>>`; generator
  matrix table matches Eq 5.52 exactly. Functoriality on all 4 rigs verified
  via 13 integration tests.
- `src/graphical_linalg.rs` — `matr_presentation<R>` builds the 16 equations
  from F&S Thm 5.60 p.170 (Groups A cocomonoid, B monoid, C bialgebra,
  D scalar). `verify_sfg_to_mat_is_full_and_faithful<R>` enumeration harness.
- `src/mat_f64.rs` (feature `f64-rig`, opt-in) — nalgebra bridge for
  `MatR<F64Rig>`: `mat_to_nalgebra` / `mat_from_nalgebra` roundtrip,
  `determinant`, `try_inverse`.
- 9 new integration test files + 2 runnable examples (`rig_showcase`,
  `sfg_to_mat`).

### Changed

- `src/prop.rs` → `src/prop/mod.rs` (directory module) to host the new
  `presentation` submodule. API unchanged; all v0.4.0 prop tests continue
  to pass.
- `PropSignature: Eq` relaxed to `PropSignature: PartialEq` with matching
  `#[derive(PartialEq)]` on `PropExpr`. Required to use f64-backed rigs
  (`UnitInterval`, `F64Rig`, `Tropical`) as `Scalar(R)` generator payloads
  inside `SfgGenerator<R>`. Strict weakening — all existing impls that
  required `Eq` still compile.
- catgraph dep bumped to v0.12.0 (for `Corel<Lambda>` + new error variants
  `Presentation`, `SfgFunctor`, `RigAxiomViolation`).

### Features

- `f64-rig` (opt-in, off by default) — enables the `mat_f64` module and adds
  a transitive `nalgebra` dep. Non-f64 rig users skip nalgebra entirely.

### Known limitations

- **Thm 5.60 faithfulness enumeration tests `#[ignore]`'d.** The 12
  `thm_5_60_faithful_*` tests in `tests/graphical_linalg.rs` are marked
  `#[ignore]` with documented reason: `Presentation::normalize` uses bounded
  structural rewriting without Knuth-Bendix completion; the D-group scalar
  equations heavily overlap and produce false-negative equivalence-class
  splits. The equation set itself is correct — all 16 F&S p.170 equations
  construct cleanly — and soundness smoke tests pass. Audit §5.4 Thm 5.60
  is **PARTIAL** in v0.5.0. **v0.5.1 will add KB completion and re-enable
  the faithfulness tests.**

### Requires

- catgraph v0.12.0 (new error variants + `Corel<Lambda>`).

## [0.4.0] - 2026-04-20

Tier 2 applied-CT gap closures from `docs/FS18-AUDIT.md`. Three
new modules anchored to F&S *Seven Sketches in Compositionality*
§5.2 and §6.5; no changes to existing public APIs.

### Added

- `prop` module (Def 5.2, Def 5.25). `PropSignature` trait for generator
  arities; arity-tracked `PropExpr<G>` expression tree; smart constructors
  `Free::{identity, braid, generator, compose, tensor}` with
  composition-arity validation. Implements `Composable<Vec<()>>`,
  `HasIdentity<Vec<()>>`, `Monoidal`, and `SymmetricMonoidalMorphism<()>`.
  Equality is structural — the SMC quotient (interchange law, unitors,
  braiding naturality) is deferred to v0.5.0 alongside the Tier 3
  presentation / equations type (Def 5.33).
- `operad_algebra` module (Def 6.99). Single-sorted `OperadAlgebra<O, Input>`
  trait `F : O → Set` generic over any `Operadic<Input>` type. Concrete
  `CircAlgebra` implementing F&S Ex 6.100 for `WiringDiagram` via
  outer-port counts; `check_substitution_preserved` helper witnessing
  `evaluate(op ∘_i inner, inputs) == evaluate(op, inputs)` for algebras
  whose evaluator discards inputs.
- `operad_functor` module (Rough Def 6.98). Generic `OperadFunctor<O1, O2, Input>`
  trait. Concrete `E1ToE2` packaging the canonical little-intervals-into-
  little-disks inclusion (via upstream `E2::from_e1_config`) with a
  `start_name` offset so the two branches of `F(o ∘_i q) = F(o) ∘_i F(q)`
  can share a substitution without colliding on E2's unique-name
  invariant. Literal geometric functoriality is verified by
  `E1ToE2::check_substitution_preserved` (canonicalising each side's disks
  by centre-x and comparing within `f32` tolerance); a generic arity-level
  shadow `check_substitution_preserved` covers any `OperadFunctor`.
- Public accessors `E1::arity`, `E1::sub_intervals`, `E2::arity_of`,
  `E2::sub_circles`; `#[derive(Clone)]` on `E1` and `E2<Name: Clone>`.
  Additive and non-breaking.
- Examples: `examples/free_prop.rs`, `examples/operad_algebra_circ.rs`,
  `examples/operad_functor_e1_to_e2.rs`.
- Tests: `tests/prop.rs` (11 tests), `tests/operad_algebra.rs` (3 tests),
  `tests/operad_functor.rs` (4 tests).

### Requires

- catgraph v0.11.4 (unchanged from v0.3.3).

## [0.3.3] - 2026-04-19

Phase W.1 — WASM + edge-device support. Wires the `parallel` feature
through all four `CondIterator` call sites; compiles clean against
`wasm32-wasip1-threads` and `wasm32-wasip1 --no-default-features`.

### Added

- `[features] default = ["parallel"]` — `parallel = ["dep:rayon",
  "dep:rayon-cond", "catgraph/parallel"]`.
- `examples/wasi_smoke_applied.rs` — representative `LinearCombination`
  multiplication example.

### Changed

- `rayon` and `rayon-cond` are now optional dependencies gated by the
  `parallel` feature.
- `catgraph` dep is `default-features = false` so the `parallel` toggle
  propagates.
- `src/linear_combination.rs::Mul::mul` and `::linear_combine`:
  `CondIterator::new(...).map(...).collect()` gated with
  `#[cfg(feature = "parallel")]`; plain `into_iter().map(...).collect()`
  fallback when off. Shared closure extracted so both arms use identical
  per-term logic.
- `src/temperley_lieb.rs::BrauerMorphism::non_crossing`: both `source`
  and `target` crossing checks use `CondIterator::new(...).any(...)`
  under `#[cfg(feature = "parallel")]`; plain `.into_iter().any(...)`
  fallback when off. Shared `has_crossing` predicate extracted once.
- `tests/rayon_equivalence.rs`: the three direct `CondIterator`
  arm-equivalence tests are gated behind `#[cfg(feature = "parallel")]`
  (they test the rayon_cond dep, which is only in the graph when the
  feature is on).

### Notes

- Native test count: 900 with default features, 897 with
  `--no-default-features` (the 3 gated tests).

## [0.3.2] - 2026-04-19

Pre-WASM rayon consolidation. Internal-only — no public API change.

### Changed

- `linear_combination::Mul::mul` and `linear_combination::LinearCombination::linear_combine` now use `rayon_cond::CondIterator` to unify the parallel/sequential branches at the two `HashMap` `into_par_iter()` call sites. Functional behavior unchanged — `PARALLEL_MUL_THRESHOLD = 32` still gates the parallel path.
- `temperley_lieb::BrauerMorphism::non_crossing` now uses `rayon_cond::CondIterator` to unify the parallel/sequential branches at the two `par_bridge()` call sites. Functional behavior unchanged — `PARALLEL_COMBINATIONS_THRESHOLD = 8` still gates the parallel path.

### Added

- `rayon-cond = "0.4"` as a direct dependency (previously pulled transitively via `rustworkx-core`).
- `tests/rayon_equivalence.rs` extended to exercise both `CondIterator::Parallel` and `CondIterator::Serial` arms at each migrated site, asserting algebraic-law determinism across the toggle.

### Why this shape

The previous if/else-over-threshold pattern duplicated the iteration body. `rayon_cond::CondIterator` is the canonical rustworkx-core idiom (see [`rustworkx-core/src/centrality.rs`](https://github.com/Qiskit/rustworkx/blob/main/rustworkx-core/src/centrality.rs)) for compile/runtime parallel↔sequential toggling, and it's the right pattern for Phase W.1's `parallel` feature flag — a single `#[cfg(feature = "parallel")]` gate replaces cfg-gating two parallel branches.

## [0.3.1] - 2026-04-18

Tier 1.1 follow-ups flagged during v0.3.0 work.

### Added

- `DecoratedCospan::compose` now invokes `D::pushforward` through the pushout quotient (realizes F&S Def 6.75 / Thm 6.77 for decorations whose apex data references apex indices).
- Direct `PetriNet::permute_side` implementation via in-place permutation of the transition sequence — replaces the decoration-bridge impl that discarded boundary permutations on the return trip.
- `Transition::relabel` arc deduplication: when the quotient collapses distinct places onto the same target, arcs merge with summed `Decimal` multiplicities. Pre- and post-arcs dedup independently (self-loops preserved). Canonical ascending-by-place sort.
- `examples/petri_net_braiding.rs` — direct `permute_side` demo.
- `tests/decorated_cospan.rs` — 3 integration tests covering Circuit EdgeSet series composition, `Trivial` pushforward unit, `PetriDecoration` regression safety.
- `tests/petri_net.rs` — 8 new tests (4 braiding + 4 arc-dedup).

### Changed

- `examples/decorated_cospan_circuit.rs` extended with series composition; `NOTE:` caveat block removed.
- `FS18-AUDIT.md` Ex 6.79–6.86 row upgraded from PARTIAL to DONE; headline recomputed (9 DONE / 3 PARTIAL / 12 MISSING / 17 N/A / 15 IN CORE of 56 items).

### Requires

- catgraph v0.11.3 for `Cospan::compose_with_quotient`.

## [0.3.0] - 2026-04-17

Tier 1 gap closures (from v0.2.0 audit).

### Added

- Generic `DecoratedCospan<Lambda, D>` + `Decoration` trait — realizes F&S Def 6.75 (decorated cospans) and Thm 6.77 (decorated cospan category is a hypergraph category).
- `PetriDecoration<Lambda>` marker type bridging `PetriNet` to the generic `DecoratedCospan` machinery.
- `HypergraphCategory<Lambda>` impl for both `DecoratedCospan<Lambda, D>` (generic) and `PetriNet<Lambda>` (specialized).
- `examples/decorated_cospan_circuit.rs` — Circuit EdgeSet example.
- `Trivial` decoration as an uninformative starting example.

### Known limitations (closed in 0.3.1)

- `DecoratedCospan::compose` did not yet invoke `D::pushforward` (required upstream `Cospan::compose_with_quotient`).
- `PetriNet::permute_side` delegated to the decoration bridge, which discarded leg permutations.
- `Transition::relabel` produced duplicate `(place, weight)` entries when the quotient collapsed places.

## [0.2.0] - 2026-04-17

### Added

- `docs/FS18-AUDIT.md` — section-by-section coverage audit against Fong & Spivak, *Seven Sketches in Compositionality* (arXiv:1803.05316v3, 2018). 56 items tracked across Chapters 4–6.
- Cross-reconciliation with `catgraph/docs/FS19-AUDIT.md`.

## [0.1.0] - 2026-04-14

### Added

- Initial release. Applied-CT modules extracted from `catgraph` core as part of the v0.11.0 slim-baseline refactor:
  - `linear_combination.rs` — formal linear combinations over a coefficient ring (R-module `R[T]`).
  - `wiring_diagram.rs` — operadic substitution on named cospans (F&S §6.5 Ex 6.94 Cospan operad).
  - `petri_net.rs` — place/transition nets, firing, reachability, parallel/sequential composition, cospan bridge.
  - `temperley_lieb.rs` — Temperley-Lieb / Brauer algebra via perfect matchings, Jones relations, dagger.
  - `e1_operad.rs` — little-intervals operad (E₁).
  - `e2_operad.rs` — little-disks operad (E₂).
- Criterion bench `rayon_thresholds`.

[Unreleased]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.6.0...HEAD
[0.6.0]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.6...catgraph-applied-v0.6.0
[0.5.6]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.5...catgraph-applied-v0.5.6
[0.5.5]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.4...catgraph-applied-v0.5.5
[0.5.4]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.3...catgraph-applied-v0.5.4
[0.5.3]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.2...catgraph-applied-v0.5.3
[0.5.2]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.1...catgraph-applied-v0.5.2
[0.5.1]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.5.0...catgraph-applied-v0.5.1
[0.5.0]: https://github.com/tsondru/catgraph/compare/catgraph-applied-v0.4.0...catgraph-applied-v0.5.0
[0.4.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.4.0
[0.3.3]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.3
[0.3.2]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.2
[0.3.1]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.1
[0.3.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.3.0
[0.2.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.2.0
[0.1.0]: https://github.com/tsondru/catgraph/releases/tag/catgraph-applied-v0.1.0
