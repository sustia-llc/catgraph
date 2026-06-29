# catgraph-magnitude

Magnitude of enriched categories + magnitude homology for the [catgraph](https://github.com/tsondru/catgraph) workspace.

Anchored to:
- [Bradley & Vigneaux (2025)](https://arxiv.org/abs/2501.06662) — `Mag(tM)` Tsallis decomposition (Prop 3.10), Shannon recovery (Rem 3.11), magnitude homology Euler-characteristic identity (Prop 3.14).
- [Leinster & Shulman, *Magnitude homology of enriched categories and metric spaces* (2017)](https://arxiv.org/abs/1711.00802) — §2 chain complex over Lawvere metric spaces.
- [Leinster, *The magnitude of metric spaces* (2013)](https://arxiv.org/abs/1012.5857) — Möbius / weighting / scatteredness primitives.
- [Leinster, *The Euler characteristic of a category* (2008)](https://arxiv.org/abs/math/0610260) — Cor 1.5 integer-exact Möbius for finite circuit-free categories.
- [Bradley, Terilla & Vlassopoulos, *An enriched category theory of language* (2021)](https://arxiv.org/abs/2106.07890) — representable copresheaf semantics (Yoneda embedding) + asymmetric semantic internal hom (Lemma 2 Eq 11 / §5 metric).

**Status:** v0.5.0 (ZAlgebra consumer-side rename + closed-form Möbius cross-check fixture + bidirectional `verify_mobius_recursion` harness, co-released with catgraph-applied v0.6.0 at workspace umbrella v0.14.0). v0.4.0 shipped Leinster 2008 Cor 1.5 integer-exact Möbius + multi-prime CRT integer SNF lift. v0.3.0 shipped BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity.

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

Four anchor verifications must pass for any v0.5.x tag:

1. **BV 2025 Prop 3.10 closed form** — `Mag(tM) = (t−1) · Σ H_t(p_x) + #(T(⊥))` to `0e0` on a
   hand-computed 4-state LM.
2. **BV 2025 Rem 3.11 Shannon recovery** — `d/dt Mag|_{t=1} = Σ H(p_x)` by central finite difference
   (`h = 1e-4`) to `~6e-10`.
3. **Leinster 2013 Prop 2.1.3 chain-sum equivalence** — `mobius_function_via_chains::<F64Rig>(space) ≈ mobius_function::<F64Rig>(space)` to `1e-9` on hand-built 4-state scattered fixture + proptest n=2-5 (v0.2.0).
4. **BV 2025 Prop 3.14 magnitude-homology Euler-char identity** — `chain_complex::euler_char_identity_at(space, t, max_degree)` returns `(via_homology, via_magnitude)` agreeing within an analytical residual bound `|Δ| ≤ n · r^(max_deg+1) / (1−r) + 1e-9` where `r = (n−1) · exp(−d_min_scaled)`. 5 fixtures pass (release suite ~31s).

v0.5.0 also adds:
- A closed-form cross-check fixture asserting `mobius_function_via_chains_exact` matches the analytical Möbius value on a 4-element poset.
- `verify_mobius_recursion` — a bidirectional harness asserting the Leinster 2008 Def 1.1 recursion identity in both directions on integer-exact Möbius output.

On `main` (unreleased) — a semantic + determinism layer over `LmCategory`:
- **`yoneda`** (#19) — `LmCategory::yoneda(name)` gives the representable copresheaf `L(x, −)` as a `Copresheaf` (meaning-as-distribution over continuations, `π = exp(−d)`). `semantic_hom` / `semantic_distance` are the **asymmetric** BTV 2021 internal hom `inf_c min{1, b(c)/a(c)}` and its `−ln` distance (`semantic_distance_sym` for a non-canonical symmetric variant). BTV 2021 Lemma 2 Eq 11 / §5.
- **`determinism`** (#20) — `LmCategory::deterministic_transition_rank()` = `rank MH₁(ℓ = 0)` = the number of covering `π = 1` (deterministic) transitions: a structural memorisation count, **not** a coherence/hallucination signal. BV 2025 / LS 2017.

## API surface (v0.5.0)

| Symbol | Paper anchor | Notes |
|---|---|---|
| `LmCategory` | BV 2025 §3 | Materialized BYO-LM transition table |
| `LmCategory::yoneda(name)` → `Copresheaf` | BTV 2021 (Yoneda) | Representable copresheaf `L(x, −)`, `π = exp(−d)` (unreleased, #19) |
| `semantic_hom` / `semantic_distance` / `semantic_distance_sym` | BTV 2021 Lemma 2 Eq 11 / §5 | Asymmetric semantic internal hom + its `−ln` distance (sym variant non-canonical) (unreleased, #19) |
| `LmCategory::deterministic_transition_rank()` → `usize` | BV 2025 / LS 2017 §2 | `rank MH₁(ℓ=0)` = #covering deterministic (`π=1`) transitions (unreleased, #20) |
| `magnitude<Q>(space, t)` | BV 2025 §3.5 Eq (7) | Möbius sum at scale `t` |
| `mobius_function<Q>(space)` | Leinster 2013 §1.1 Lemma 1.1.4 | `ζ⁻¹` via Gaussian elimination (field-fast path) |
| `mobius_function_via_chains<Q>(space)` | Leinster 2013 Prop 2.1.3 | Chain-sum via von-Neumann series (scattered-space precondition) |
| `mobius_function_via_chains_exact<N, Q: ZAlgebra>(poset)` | Leinster 2008 Cor 1.5 | **Integer-exact finite-category Möbius** over `PosetCategory<N>` (v0.4.0); `Q: ZAlgebra` bound from cg-applied v0.6.0 |
| `verify_mobius_recursion<N, Q: ZAlgebra>(poset, mobius)` | Leinster 2008 Def 1.1 | Bidirectional recursion-identity harness (v0.5.0) |
| `chain_count_signed_graded<Q>(space, max_chain_length)` | Leinster 2013 Prop 2.1.3 + LS 2017 §2 grading | Per-grade signed chain-count diagnostic |
| `is_mobius_invertible_at(space, t)` | Leinster 2013 Prop 2.4.17 | Ergonomic Möbius-existence threshold check |
| `chain_complex::{Chain, enumerate_chains, ChainIndex, boundary_matrix<Q>}` | LS 2017 §2 | Magnitude-homology chain complex `(C_{k,ℓ}, ∂_k)` over Lawvere metric (v0.3.0) |
| `chain_complex::magnitude_homology_rank<Q>(idx, k, ℓ)` | LS 2017 §2 + BV 2025 Prop 3.14 | `rank(H_{k,ℓ}(M))` via SNF over Z/p (single-prime + 2-prime cross-check) (v0.3.0) |
| `chain_complex::euler_char_identity_at(space, t, max_degree)` | BV 2025 Prop 3.14 | Headline acceptance gate (v0.3.0) |
| `snf::smith_normal_form` + `snf::{zmod, echelon, band}` + Phase-1 / Phase-2 helpers | Storjohann 2000 §7 | Custom SNF backend over `MatR<Q>` (v0.3.0) |
| `snf::smith_normal_form_integer` | Newman 1972 §1.4 Thm II.9 | Multi-prime CRT lift for invariant-factor recovery (v0.4.0) |
| `PosetCategory<N>` | Leinster 2008 | Finite-category `LawvereMetricSpace`-compatible input type (v0.4.0) |
| `weighting<Q>(space)` | Leinster 2013 §1.1 Def 1.1.1 | Solve `ζ · w = u_I` (v0.2.0) |
| `coweighting<Q>(space)` | Leinster 2013 §1.1 Def 1.1.1 | Solve `v · ζ = u_J^T` (v0.2.0) |
| `is_scattered(space)` | Leinster 2013 Def 2.1.2 | `d(a,b) > log(#A−1)` predicate (v0.2.0) |
| `tsallis_entropy(p, t)` | BV 2025 Prop 3.10 / Tsallis 1988 | Shannon special case at `\|t−1\| < 1e-6` |
| `WeightedCospan<Λ, Q>` | F&S 2019 §1 + BV 2025 §3 | Cospan with per-edge rig weights |
| `LawvereMetricSpace<T>` (re-export) | Lawvere 1973 | Asymmetric metric space |
| `Rig`, `Ring`, `ZAlgebra`, `Z`, `BoolRig`, `UnitInterval`, `Tropical`, `F64Rig` | F&S 2018 §5.3.1 + Bourbaki *Algèbre* Ch. I §8 | Re-exports + `Ring` super-trait + `ZAlgebra` integer-exact bound (v0.5.0 rename of `Integer`) |
| `TSALLIS_SHANNON_EPS` | numerical | Special-case threshold `1e-6` |

## Algebraic scoping

Three Möbius paths ship with distinct trait bounds:

- **Field-fast path** — `mobius_function::<Q: Ring + Div + From<f64>>(space)` (v0.1.x) — Gaussian elimination on `[ζ | I]`. Requires multiplicative inverses (the `Div` bound).
- **Chain-sum path (von-Neumann series)** — `mobius_function_via_chains::<Q: Ring + From<f64>>(space)` (v0.2.0) — `μ = Σ (−1)ᵏ Mᵏ` with `M = ζ − I`. No `Div` needed; requires the input to be **scattered** (Leinster Def 2.1.2: `d(a,b) > log(#A−1)`).
- **Integer-exact path (Leinster 2008 Cor 1.5)** — `mobius_function_via_chains_exact::<N, Q: ZAlgebra>(poset)` (v0.4.0) — direct chain-sum recursion on a `PosetCategory<N>` over any `ZAlgebra` carrier (the canonical workhorse is `Z(BigInt)`). No `Div` needed; no scatteredness precondition; produces an exact integer answer when the input poset is circuit-free (Leinster 2008 Cor 1.5 hypothesis).

The first two paths yield the same matrix on scattered invertible spaces (v0.2.0 acceptance test #3). The integer-exact path is the natural workhorse when `f64` rounding would corrupt the answer and intermediate values can exceed `i64::MAX`.

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
cargo run --example prop_3_14_acceptance  # BV 2025 Prop 3.14 magnitude-homology Euler-char identity (v0.3.0)
cargo run --example integer_mobius        # Leinster 2008 Cor 1.5 integer-exact Möbius via Z(BigInt) (v0.4.0)
```

## Roadmap

- ✅ **v0.1.0** (2026-04-25): closed-form magnitude on prefix-poset LMs (BV 2025 Prop 3.10 + Rem 3.11 acceptance gate).
- ✅ **v0.1.1** (2026-04-28): `LmCategory::add_transition` validation, `LmCategory::from_transition_log` replay constructor, `WeightedCospan::into_validated_metric_space`, BFS frontier cap.
- ✅ **v0.2.0** (2026-05-04): Leinster 2013 §1.1 (co)weighting primitives + Def 2.1.2 scatteredness predicate + Prop 2.1.3 chain-sum Möbius via von-Neumann series.
- ✅ **v0.2.1** (2026-05-04): post-shipping review patch pass; strictly additive.
- ✅ **v0.3.0** (2026-05-09): BV 2025 Prop 3.14 magnitude-homology Euler-characteristic identity `Mag(tM) = Σ_ℓ e^(−tℓ) · Σ_k (−1)ᵏ rank(H_{k,ℓ}(M))`. LS 2017 §2 chain complex + custom Storjohann §7 SNF port over `MatR<Q>` + single-prime + 2-prime rank-recovery cross-check + path C analytical-bound acceptance across 5 fixtures.
- ✅ **v0.3.1** (2026-05-10): Phase G review patch — `snf_rank_over_zp` panic → `Result`; `RankQ` rename + generic-mono coupling docs; citation corrections.
- ✅ **v0.4.0** (2026-05-13): Leinster 2008 Cor 1.5 integer-exact Möbius (`mobius_function_via_chains_exact<N, Q: Ring + Integer>` over `PosetCategory<N>`) + multi-prime CRT SNF lift (`smith_normal_form_integer` with Newman 1972 §1.4 Thm II.9 chain rebalance) + `Integer` trait + `Z(BigInt)` newtype substrate from cg-applied v0.5.6.
- ✅ **v0.5.0** (2026-05-13): `Integer` → `ZAlgebra` consumer-side rename (co-release with cg-applied v0.6.0 at workspace umbrella v0.14.0) + closed-form Möbius cross-check fixture + bidirectional `verify_mobius_recursion` harness + path β + path γ first walks.

## License

MIT.
