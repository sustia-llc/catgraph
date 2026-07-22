# catgraph-magnitude

Magnitude of enriched categories + magnitude homology for the [catgraph](https://github.com/sustia-llc/catgraph) workspace.

Anchored to:
- [Bradley & Vigneaux (2025)](https://arxiv.org/abs/2501.06662) — `Mag(tM)` Tsallis decomposition (Prop 3.10), Shannon recovery (Rem 3.11), magnitude homology Euler-characteristic identity (Prop 3.14).
- [Leinster & Shulman, *Magnitude homology of enriched categories and metric spaces* (2017)](https://arxiv.org/abs/1711.00802) — §2 chain complex over Lawvere metric spaces.
- [Leinster, *The magnitude of metric spaces* (2013)](https://arxiv.org/abs/1012.5857) — Möbius / weighting / scatteredness primitives.
- [Leinster, *The Euler characteristic of a category* (2008)](https://arxiv.org/abs/math/0610260) — Cor 1.5 integer-exact Möbius for finite circuit-free categories.
- [Bradley, Terilla & Vlassopoulos, *An enriched category theory of language* (2021)](https://arxiv.org/abs/2106.07890) — representable copresheaf semantics (Yoneda embedding) + asymmetric semantic internal hom (Lemma 2 Eq 11 / §5 metric).

**Status:** workspace version `0.2.0`. Released as workspace tags `v0.1.0`
(2026-07-01: semantic + determinism + coalition layer, #19–#23), `v0.1.1`
(ULP-tolerant triangle-inequality checks, #29/#30), and `v0.2.0` (2026-07-02:
incremental coalition magnitude for the decision hot path, #31/PR #32).
Workspace-wide versioning supersedes the pre-reboot per-crate lineage; the full
BV 2025 / Leinster 2013 / LS 2017 / Leinster 2008 math stack was migrated intact
onto the DeepCausality substrate in reboot Phase 3 (#8).

## What

A pure-math Rust crate for computing the magnitude `Mag(tM)` of an enriched category over a rig
(BV 2025 §3). The headline use case is BV 2025's language-model magnitude, where `Mag(tM)` decomposes
via Tsallis q-entropy into a per-state diversity indicator that recovers Shannon entropy at `t = 1`
(BV 2025 Rem 3.11).

## Quickstart

```rust
use catgraph_magnitude::{LmCategory, magnitude::tsallis_entropy};

let mut m = LmCategory::new(vec!["⊥".into(), "⊥a".into(), "⊥a†".into()]);
m.add_transition("⊥", "⊥a", 1.0);
m.add_transition("⊥a", "⊥a†", 1.0);
m.mark_terminating("⊥a†");

let mag = m.magnitude(2.0).unwrap();
println!("Mag(2M) = {mag:.6}");   // 1.000000 (deterministic chain)
```

## Acceptance gates

Four anchor verifications must pass for any tag:

1. **BV 2025 Prop 3.10 closed form** — `Mag(tM) = (t−1) · Σ H_t(p_x) + #(T(⊥))` to `0e0` on a
   hand-computed 4-state LM.
2. **BV 2025 Rem 3.11 Shannon recovery** — `d/dt Mag|_{t=1} = Σ H(p_x)` by central finite difference
   (`h = 1e-4`) to `~6e-10`.
3. **Leinster 2013 Prop 2.1.3 chain-sum equivalence** — `mobius_function_via_chains::<F64Rig>(space) ≈ mobius_function::<F64Rig>(space)` to `1e-9` on hand-built 4-state scattered fixture + proptest n=2-5.
4. **BV 2025 Prop 3.14 magnitude-homology Euler-char identity** — `chain_complex::euler_char_identity_at(space, t, max_degree)` returns `(via_homology, via_magnitude)` agreeing within an analytical residual bound `|Δ| ≤ n · r^(max_deg+1) / (1−r) + 1e-9` where `r = (n−1) · exp(−d_min_scaled)`. 5 fixtures pass (release suite ~31s).

Two integer-exact Möbius cross-checks (Leinster 2008 Cor 1.5) ship alongside,
migrated in reboot Phase 3:
- A closed-form cross-check fixture asserting `mobius_function_via_chains_exact` matches the analytical Möbius value on a 4-element poset.
- `verify_mobius_recursion` — a bidirectional harness asserting the Leinster 2008 Def 1.1 recursion identity in both directions on integer-exact Möbius output.

The workspace `v0.1.0` (2026-07-01) semantic + determinism + coalition layer over `LmCategory` (#19–#23):
- **`yoneda`** (#19) — `LmCategory::yoneda(name)` gives the representable copresheaf `L(x, −)` as a `Copresheaf` (meaning-as-distribution over continuations, `π = exp(−d)`). `semantic_hom` / `semantic_distance` are the **asymmetric** BTV 2021 internal hom `inf_c min{1, b(c)/a(c)}` and its `−ln` distance (`semantic_distance_sym` for a non-canonical symmetric variant). BTV 2021 Lemma 2 Eq 11 / §5.
- **`determinism`** (#20) — `LmCategory::deterministic_transition_rank()` = `rank MH₁(ℓ = 0)` = the number of covering `π = 1` (deterministic) transitions: a structural memorisation count, **not** a coherence/hallucination signal. BV 2025 / LS 2017.
- **`semantic`** (#21) — comparison / clustering over the Yoneda embedding. `LmCategory::yoneda_all()` gives every meaning from one `enriched_space()` pass; `k_nearest_from` / `k_nearest_to` rank the nearest meanings to a query in **both** directions of the asymmetric BTV distance (they differ — BTV §5); `cluster_semantic_sym` is a symmetric single-linkage **convenience** over the non-canonical `semantic_distance_sym` (mutually-unreachable meanings never merge). BTV 2021 Lemma 2 Eq 11 / §5.
- **`coalition`** (#22) — enriched-coalition magnitude surface (gemini-spec §IV.5). A coalition is a **member-restricted, max-product-closed, perfectly-coupled-quotiented** cospan-weighted subgraph of an enriched category (agents = objects, couplings = `UnitInterval` homs); `coalition_magnitude(coalition, t)` is its diversity `Mag(tA|members)` via the BV 2025 §3.5 Eq 7 Möbius sum (`t = 1` canonical arm, `t = 2` collision proxy, `t → ∞` cardinality-like; Möbius sum is cyclic-safe — Prop 3.10's closed form is the acyclic special case). `Coalition::from_enriched` restricts to member homs, closes through member nodes only (couplings via non-members do not count), then skeletalizes members that are perfectly coupled both ways (distance 0) so a fully-coupled coalition reports 1 effective agent instead of a singular ζ — `effective_members()` / `member_classes()` expose the quotient. `coalition_magnitude_from_couplings` is the plain-data entry point (seed of #23 `coalition_value`). BV 2025 §3.5 Eq 7 + BTV 2021 `[0,1]` enrichment + Leinster 2008/2013 skeleton invariance.

## API surface

| Symbol | Paper anchor | Notes |
|---|---|---|
| `LmCategory` | BV 2025 §3 | Materialized BYO-LM transition table |
| `LmCategory::from_traces(traces)` | BTV 2021 §2.2 Def 4 + Eq (8) | Corpus MLE — prefix-state tree (`π(p·t\|p) = N(p·t)/N(p)`); Eq (8) exact by construction, feeds `magnitude` (#53) |
| `LmCategory::yoneda(name)` → `Copresheaf` | BTV 2021 (Yoneda) | Representable copresheaf `L(x, −)`, `π = exp(−d)` (#19) |
| `semantic_hom` / `semantic_distance` / `semantic_distance_sym` | BTV 2021 Lemma 2 Eq 11 / §5 | Asymmetric semantic internal hom + its `−ln` distance (sym variant non-canonical) (#19) |
| `LmCategory::deterministic_transition_rank()` → `usize` | BV 2025 / LS 2017 §2 | `rank MH₁(ℓ=0)` = #covering deterministic (`π=1`) transitions (#20) |
| `LmCategory::yoneda_all()` → `Vec<Copresheaf>` | BTV 2021 (Yoneda) | All meanings from one `enriched_space()` pass, object order (#21) |
| `k_nearest_from` / `k_nearest_to` | BTV 2021 §5 | Bidirectional nearest-meaning ranking over the asymmetric `semantic_distance` (#21) |
| `cluster_semantic_sym` | BTV 2021 §5 (sym convenience) | Single-linkage threshold clustering over the non-canonical `semantic_distance_sym` (#21) |
| `Coalition<O>` + `Coalition::from_enriched(cat, members)` | BV 2025 §3.5 + BTV 2021 (gemini-spec §IV.5) | Restrict-then-close + skeletalize cospan-weighted subgraph; `effective_members()` / `member_classes()` expose the perfectly-coupled quotient (#22) |
| `coalition_magnitude(coalition, t)` → `f64` | BV 2025 §3.5 Eq (7) | Coalition diversity `Mag(tA\|members)` (Möbius sum, cyclic-safe); `t=1` canonical arm (#22) |
| `coalition_magnitude_from_couplings(agents, couplings, members, t)` | BV 2025 §3.5 (gemini-spec §IV.5) | Plain-data entry point; seed of #23 `coalition_value` (#22) |
| `coalition_value(agents, couplings, members)` → `f64` | BV 2025 §3.5 (gemini-spec §IV.5) | **Stable consumer API** — the pinned `t = 1` scalar for downstream decision policies (koalisi #5 `MagnitudePolicy`); `= coalition_magnitude_from_couplings(…, 1.0)` (#23) |
| `CoalitionEvaluator` + `CoalitionEvaluator::new(agents, couplings, members, t)` | BV 2025 §3.5 Eq (7) | Caches base coalition `S` (closed table, skeletal `t`-scaled ζ⁻¹, weighting/coweighting); `value_with(candidate)` answers `Mag(S ∪ {x})` via an O(m²) closure border + bordered-Schur update, `base_value()` returns `Mag(S)` (#31) |
| `CoalitionEvaluator::value_with_scratch(candidate, &mut EvalScratch)` → `f64` | BV 2025 §3.5 Eq (7) | Allocation-free variant of `value_with` — reuses caller-owned buffers across a candidate sweep; **bit-identical** to `value_with` (#33) |
| `EvalScratch` + `EvalScratch::new()` | — | Caller-owned scratch buffers (the 7 per-call `Vec`s) for `value_with_scratch`; carries no cross-call state, reusable across candidates/evaluators (#33) |
| `coalition_value_delta(agents, couplings, members, candidate)` → `(f64, f64)` | BV 2025 §3.5 Eq (7) | One-shot `(Mag(S), Mag(S ∪ {candidate}))` pair at the pinned `t = 1` arm (#31) |
| `INCREMENTAL_REL_TOL` | numerical | Relative tolerance (`1e-9`) of the incremental value vs a fresh `coalition_value` on `S ∪ {x}`; base value is bit-identical (#31) |
| `magnitude<Q>(space, t)` | BV 2025 §3.5 Eq (7) | Möbius sum at scale `t` |
| `mobius_function<Q>(space)` | Leinster 2013 §1.1 Lemma 1.1.4 | `ζ⁻¹` via Gaussian elimination (field-fast path) |
| `mobius_function_via_chains<Q>(space)` | Leinster 2013 Prop 2.1.3 | Chain-sum via von-Neumann series (scattered-space precondition) |
| `mobius_function_via_chains_exact<N, Q: ZAlgebra>(poset)` | Leinster 2008 Cor 1.5 | **Integer-exact finite-category Möbius** over `PosetCategory<N>`; `Q: ZAlgebra` bound from `catgraph-applied` |
| `verify_mobius_recursion<N, Q: ZAlgebra>(poset, mobius)` | Leinster 2008 Def 1.1 | Bidirectional recursion-identity harness |
| `chain_count_signed_graded<Q>(space, max_chain_length)` | Leinster 2013 Prop 2.1.3 + LS 2017 §3 grading | Per-grade signed chain-count diagnostic |
| `is_mobius_invertible_at(space, t)` | Leinster 2013 Def 2.1.2 + Prop 2.1.3 | Ergonomic Möbius-existence threshold check |
| `chain_complex::{Chain, enumerate_chains, ChainIndex, boundary_matrix<Q>}` | LS 2017 §3 | Magnitude-homology chain complex `(C_{k,ℓ}, ∂_k)` over Lawvere metric |
| `chain_complex::magnitude_homology_rank<Q>(idx, k, ℓ)` | LS 2017 §3 + BV 2025 Prop 3.14 | `rank(H_{k,ℓ}(M))` via SNF over Z/p (single-prime + 2-prime cross-check) |
| `chain_complex::euler_char_identity_at(space, t, max_degree)` | BV 2025 Prop 3.14 | Headline acceptance gate |
| `snf::smith_normal_form` + `snf::{zmod, echelon, band}` + Phase-1 / Phase-2 helpers | Storjohann 2000 §7 | Custom SNF backend over `MatR<Q>` |
| `snf::smith_normal_form_integer` | Newman 1972 §1.4 Thm II.9 | Multi-prime CRT lift for invariant-factor recovery (O(r²) determinantal-divisor DP; primes from a const 16-entry table below 2^31) |
| `snf::integer::hadamard_bound` + `hadamard_bound_matr<R>` + `hadamard_bound_integer` | Hadamard | `H(A) = ∏_i ‖a_i‖₂` upper bound: f64, `MatR<R>` wrapper, and pure-integer (`isqrt`-based, float-free) variants |
| `PosetCategory<N>` | Leinster 2008 | Finite-category `LawvereMetricSpace`-compatible input type |
| `weighting<Q>(space)` | Leinster 2013 §1.1 Def 1.1.1 | Solve `ζ · w = u_I` |
| `coweighting<Q>(space)` | Leinster 2013 §1.1 Def 1.1.1 | Solve `v · ζ = u_J^T` |
| `is_scattered(space)` | Leinster 2013 Def 2.1.2 | `d(a,b) > log(#A−1)` predicate |
| `tsallis_entropy(p, t)` | BV 2025 Prop 3.10 / Tsallis 1988 | Shannon special case at `\|t−1\| < 1e-6` |
| `WeightedCospan<Λ, Q>` | F&S 2019 §1 + BV 2025 §3 | Cospan with per-edge rig weights |
| `LawvereMetricSpace<T>` (re-export) | Lawvere 1973 | Asymmetric metric space |
| `Rig`, `Ring`, `ZAlgebra`, `Z`, `BoolRig`, `UnitInterval`, `Tropical`, `F64Rig` | F&S 2018 §5.3.1 + Bourbaki *Algèbre* Ch. I §8 | Re-exports + `Ring` super-trait + `ZAlgebra` integer-exact bound (renamed from `Integer`) |
| `TSALLIS_SHANNON_EPS` | numerical | Special-case threshold `1e-6` |

## Algebraic scoping

Three Möbius paths ship with distinct trait bounds:

- **Field-fast path** — `mobius_function::<Q: Ring + Div + From<f64>>(space)` — Gaussian elimination on `[ζ | I]`. Requires multiplicative inverses (the `Div` bound).
- **Chain-sum path (von-Neumann series)** — `mobius_function_via_chains::<Q: Ring + From<f64>>(space)` — `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I`. No `Div` needed; requires the input to be **scattered** (Leinster Def 2.1.2: `d(a,b) > log(#A−1)`).
- **Integer-exact path (Leinster 2008 Cor 1.5)** — `mobius_function_via_chains_exact::<N, Q: ZAlgebra>(poset)` — direct chain-sum recursion on a `PosetCategory<N>` over any `ZAlgebra` carrier (the canonical workhorse is `Z(BigInt)`). No `Div` needed; no scatteredness precondition; produces an exact integer answer when the input poset is circuit-free (Leinster 2008 Cor 1.5 hypothesis).

The first two paths yield the same matrix on scattered invertible spaces (the chain-sum equivalence acceptance test). The integer-exact path is the natural workhorse when `f64` rounding would corrupt the answer and intermediate values can exceed `i64::MAX`.

**Out of scope: `Tropical`-valued / `BoolRig`-valued magnitude.** Per Leinster 2013 §1.3 Examples 1.3.1, the scalar rig `k` is determined by V (V = `[0,∞]` ⇒ k = ℝ). Magnitude valued in `Tropical` or `BoolRig` for our V = Lawvere `[0,∞]` setting is not paper-aligned. See `docs/BV25-AUDIT.md` §"Out of scope" for the full chain of citation.

`tsallis_entropy` is `f64`-only; lifting it to a generic rig is non-trivial (the `0/0` limit form
requires real-valued epsilon comparisons and a `ln` operation).

## Numerical scoping

Public constant `TSALLIS_SHANNON_EPS = 1e-6` is the threshold below which `tsallis_entropy` returns
`-Σ pᵢ ln pᵢ` directly, avoiding catastrophic cancellation in the `(1 − Σ pᵢᵗ)/(t − 1) ≈ 0/0`
regime. The Rem 3.11 finite-difference step `h` MUST satisfy `h > TSALLIS_SHANNON_EPS`; the
recommended `h = 1e-4` gives ~2 decimal margin above the threshold while staying near `f64`'s
`ε^(1/3) ≈ 6e-6` truncation+roundoff optimum.

## Examples

```sh
cargo run --example lm_magnitude          # BV 2025 p.4 bounds on deterministic vs. uniform LMs
cargo run --example tsallis_shannon       # Shannon recovery to exactly-0 for δt < TSALLIS_SHANNON_EPS
cargo run --example mock_coalition        # 5-agent WeightedCospan + 3-agent LmCategory diversity demo
cargo run --example prop_3_14_acceptance  # BV 2025 Prop 3.14 magnitude-homology Euler-char identity
cargo run --example integer_mobius        # Leinster 2008 Cor 1.5 integer-exact Möbius via Z(BigInt)
cargo run --example semantic_comparison   # BTV 2021 nearest-meaning ranking (both directions) + clustering (#21)
cargo run --example coalition_magnitude   # §IV.5 coalition diversity Mag(tA|members) + restrict-then-close pin (#22)
```

## Roadmap

The full magnitude stack — closed-form `Mag(tM)` on prefix-poset LMs (BV 2025
Prop 3.10 + Rem 3.11), the (co)weighting + scatteredness + chain-sum Möbius
primitives (Leinster 2013 §1.1 / Def 2.1.2 / Prop 2.1.3), the Prop 3.14
magnitude-homology Euler-characteristic identity (LS 2017 §3 chain complex +
custom Storjohann §7 SNF over `MatR<Q>`), and Leinster 2008 Cor 1.5 integer-exact
Möbius (`mobius_function_via_chains_exact` over `PosetCategory<N>` + multi-prime
CRT SNF lift) — was migrated intact onto the DeepCausality substrate in **reboot
Phase 3** (#8). Workspace-wide releases since:

- ✅ **`v0.1.0`** (2026-07-01, #19–#23): BTV 2021 Yoneda semantic embedding
  (`yoneda` — representable copresheaf `L(x, −)` + asymmetric semantic
  hom/distance, #19); `LmCategory::deterministic_transition_rank` (`rank
  MH₁(ℓ=0)` = covering deterministic transitions, #20); semantic
  comparison/clustering over the Yoneda embedding (`semantic` — `yoneda_all` +
  bidirectional `k_nearest_from`/`k_nearest_to` + `cluster_semantic_sym`, #21);
  enriched-coalition magnitude surface (`coalition` — `Coalition::from_enriched`
  restrict-then-close + skeletalize + `coalition_magnitude` /
  `coalition_magnitude_from_couplings`, gemini-spec §IV.5, #22); and the stable
  consumer entry point `coalition_value(agents, couplings, members)` — the pinned
  `t = 1` scalar the downstream koalisi `MagnitudePolicy` A/Bs (#23).
- ✅ **`v0.1.1`** (#29/#30): ULP-tolerant triangle-inequality checks in the
  coalition max-product closure.
- ✅ **`v0.2.0`** (2026-07-02, #31/PR #32): incremental coalition magnitude for
  the decision hot path — `CoalitionEvaluator` (cached base coalition + bordered-
  Schur update), `coalition_value_delta`, and the `INCREMENTAL_REL_TOL` numerical
  contract.

## License

MIT.
