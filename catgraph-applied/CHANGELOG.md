<!-- markdownlint-disable MD024 -->
<!-- MD024 (no-duplicate-heading) disabled: Keep a Changelog intentionally
     reuses `### Added`, `### Changed`, `### Fixed`, etc. across releases. -->
# Changelog

All notable changes to this crate are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Lineage note:** pre-reboot version links below (`catgraph-applied-v0.x`
> tags) point at the private predecessor repo `tsondru/catgraph` and will not
> resolve publicly; they are kept as an honest record of the crate's history.

## [Unreleased]

## [workspace-v0.13.0] - 2026-08-15

### Added

- **`prop::presentation::rewrite` — rewrite traces cross a process boundary**
  ([#249](https://github.com/sustia-llc/catgraph/issues/249)). `RewriteStep` and
  `RewriteOutcome<G>` gain `Serialize`/`Deserialize` behind the existing
  off-by-default `serde` feature — the #81 pattern `PropExpr` / `ColoredExpr` /
  `Presentation` already use. Purely additive: no new type, no signature change,
  no behaviour change, and **no new dependency on the default build** — the
  crate's own `serde` edge stays optional. (Not the same as a serde-free default
  graph: `serde` is already there transitively, via `rust_decimal`. What the
  feature gates is *this crate's* edge and the derives.)

  - **Why the derive is safe on a step, which is the whole point.** A
    `RewriteStep`'s fields are private and it has no validating constructor —
    and it does not need one, because `replay` re-derives every step's
    assignment with the private `match_at` against the state it has *actually
    reached*, and rejects one whose recorded hyperedges are not a convex match
    of the named rule there. A deserialized trace is **checked, not trusted**:
    a hand-crafted document buys an `Err`, not a silent rewrite at a place
    nobody chose. This is the same trust boundary `apply_at` applies to a
    caller-supplied `MatchSite` (#250), one level up.
  - **Why `RewriteRule<G>` deliberately gains nothing.** Its four `new`
    conditions — parallel sides, both sides re-validated on arity *and* words,
    a non-empty `lhs`, and a mono left interface (which is what keeps the DPO
    pushout complement unique) — are checked **once** and never re-checked; the
    matcher and the DPO step read the compiled span
    (`lhs`/`rhs`/`interior`/`order`) as
    established fact. A `Deserialize` derive would hand all of that over
    unchecked. The supported round trip is the pair the rule was built from:
    persist the `(lhs, rhs)` `ColoredExpr`s, which already serialize, and
    rebuild through `RewriteRule::new` at load — **in the same order**, since a
    step binds a rule *index* and not a rule identity, and a reordered slice
    replays a stored trace to a different endpoint with no error to mark it.
    Now stated in `replay`'s docs as well as the rule's.
  - **What is not validated, said plainly.** `RewriteOutcome`'s trust boundary
    is per-field, not per-type. `steps` is checked at use (by `replay`); but
    `initial_cost`, `best_cost`, `states_explored` and `fuel_exhausted` have
    **no validator anywhere in the crate** and can have none — they are facts
    about a search that already happened. A forged document may claim a
    `best_cost` above its `initial_cost`, a shape `optimize` cannot produce.
    Recompute with `cost_of` if the number has to be right. Asserted in the
    honest direction by `tests/serde_roundtrip.rs`, which pins that a forged
    number *survives*.
    - **`best` is not certified by a successful `replay` either**, and that is
      easy to misread: nothing in the crate compares `best` against what the
      trace replays to, so an edited document can pair an honest trace with an
      unrelated `best` and `replay` still returns `Ok`. Unlike the numbers, it
      is exactly re-derivable — `optimize` builds `best` as the readback of the
      state its trace reaches, which is what `replay` returns — so compare the
      two, or just use `replay`'s result.
    - **A trace is only meaningful against the start it was recorded from**, by
      the same binding-by-position that makes rule *order* matter:
      `matched_edges` are indices into the running content. Replayed against a
      different well-formed start whose indices happen to line up, `replay`
      returns `Ok` with an unrelated endpoint. `RewriteOutcome` does **not**
      carry a start, so persist one alongside it.
    - **Neither type promises a stable wire format.** The serialized shape is
      the private field layout with no version tag — the caveat `ContentKey`
      already carries below. A field added in a later version is additive by
      this crate's standards but would fail every stored document on load, so
      durable storage wants a consumer-controlled version or a rebuildable
      cache.
  - **Oracle.** A trace serialized to JSON, reloaded, and replayed against rules
    rebuilt from round-tripped `(lhs, rhs)` pairs reaches the same
    `canonical_key` as the in-process trace — the end-to-end consumer story, not
    a derive smoke test. Three tampered documents (an out-of-range rule index, a
    permuted match, a match slid onto a same-labelled hyperedge elsewhere) are
    each rejected. Every one of those assertions was falsified against a
    deliberately broken build before being kept.

- **`prop::presentation::content::ContentKey<G>` is persistable**
  ([#255](https://github.com/sustia-llc/catgraph/issues/255)), under the same
  feature. The complete content-equality key the rewrite engine dedups on can
  now be stored, logged, and re-encoded across a process boundary, so a
  downstream store no longer has to reimplement the anchored canonicalization
  behind `canonical_key` — a reimplementation that drifted would disagree
  *silently*, which is the failure the issue exists to prevent. The two private
  component types (`EdgeRecord`, `ClosedBlock`) carry the derive too.

  - **It is a key, not a term.** Deserialization does not re-run `canonical_key`
    and nothing here can, since the content is not carried; a hand-crafted
    document can be a key of no content at all. Round-tripping a value this
    crate produced is always sound, and that is the contract.
  - **Still not `Ord`**, unchanged: `Color` need not be, so `Eq + Hash` remains
    the whole contract. The serialized shape is the private field layout and is
    **not** a stable wire format across versions.
  - **`Option` is lossy here, and the naive derive was wrong.** `colors` is
    `Vec<Option<G::Color>>`, and in a self-describing format `Some(c)` is
    indistinguishable from `None` whenever `C` itself serializes as `null` —
    which is exactly the monochromatic `Color = ()` case that most of this crate
    (`SfgGenerator` included) uses. A plain derive brought every key home with
    all colors erased. The field is now written through a private `color_slots`
    module as an explicit `Untyped` / `Typed(c)` tag. Found by the round-trip
    test, not by inspection.
  - **Oracle.** Both color regimes are pinned. The equal-by-a-different-writing
    fixture is `content_equality.rs`'s `trapped_closed_block` witness (§4.6(c)),
    chosen because its two writings disagree on raw indices *and* its canonical
    form has a non-empty closed part — so the round trip exercises the
    `ClosedBlock` half and the anchored renumbering is doing real work. (The
    shorter candidates do not: `dead_braid_prefix` and `eta_layer_slack` both
    survive a reseeding to raw index order, so they would have asserted nothing
    about canonicalization.) The foil differs only *inside* the closed
    component. The round-tripped key must be `==`, hash-equal, and still usable
    as a live `HashMap` key against a freshly computed one.

- The `serde` feature comment in `Cargo.toml` now enumerates the full derived
  surface. It was already stale before this change — `ColoredExpr` (#79) was
  missing.

## [workspace-v0.12.0] - 2026-08-15

### Added

- **`prop::presentation::rewrite` — the match-site surface: enumeration and
  apply-at-a-chosen-site, as public primitives**
  ([#250](https://github.com/sustia-llc/catgraph/issues/250)). `optimize` is one
  *policy* — descend on `cost_of`, stop on fuel — and until now it was the only
  way to fire a rule at all, so a caller whose objective was not "cheaper" (a
  neutral walk, an externally scored choice, a detour through a dearer writing
  to reach a cheaper one) was locked out of the engine entirely. The two things
  the search does underneath are now callable on their own terms. Purely
  additive: no existing signature or behaviour moved, `optimize` and `replay`
  are unchanged, and the private matcher is the same code both surfaces run.

  - `match_sites(&Content<G>, &RewriteRule<G>, limit) -> Vec<MatchSite>`
    enumerates every convex match (BGKSZ [arXiv:1602.06771] Def 3.10 / 5.4) of
    the rule's left-hand side, preferring none — no cost functional is consulted
    and no fuel is spent. `limit` truncates the enumeration; it does not rank
    it. The **cost warning carries over unchanged**: backtracking over
    hyperedges, worst case exponential in `|lhs|`, under the explicit small-rule
    assumption that a rule is the handful of generators a presentation's
    equations actually are.
  - `apply_at(&Content<G>, &RewriteRule<G>, &MatchSite) -> Result<Content<G>,
    CatgraphError>` fires one chosen site. A `MatchSite` is an ordinary value —
    it can be enumerated against one content and handed to another, kept across
    an apply, or paired with a rule it never matched — so it is re-validated in
    **two stages**, with two distinct `CatgraphError::Presentation` messages
    because they are different diagnoses calling for different fixes:
    - *is this the site's content at all?* Each site carries a private
      **fingerprint** of the content it was enumerated from — a structural hash
      of that content as a *representation*, not up to iso. A **stale** site is
      the dangerous case and re-deriving the assignment does not catch it:
      `apply_at` renumbers the surviving nodes and appends the rhs, so a site
      held across one apply names different hyperedges afterwards and, in a
      repetitive content, can still form a perfectly convex match there
      (`A ; A ; A` under `A ⇒ B` is the two-line demonstration). Rejected here,
      naming the stale-content case; the fix is to re-enumerate.
    - *is this assignment a convex match of this rule here?* The hyperedge
      assignment is then re-run through every condition the enumerator enforces
      before anything is glued. That is the same trust boundary `replay` already
      applies to a caller-supplied `RewriteStep`, and it is the point of the
      primitive: an unchecked apply is what the caller would otherwise have
      written themselves.
  - `MatchSite` exposes `matched_edges()` and `matched_nodes()` (the target's own
    indices, in **`lhs`-hyperedge index order** — position `i` is the image of
    `lhs` hyperedge `i`, *not* the matcher's connectivity-first traversal order)
    and `into_step(rule) -> RewriteStep`. The edge assignment is what a site
    *is* — node images are forced by it, since tentacle positions are content
    invariants — so `into_step` loses nothing, and a search a caller drove
    itself produces traces `replay` re-derives exactly like `optimize`'s. The
    fingerprint is deliberately **not** carried into a `RewriteStep`: a step is
    index-based against a replay's *running* content, which differs at every
    position. No serde impl: that is a separate decision.
  - `match_sites_of(&ColoredExpr<G>, …) -> Result<Vec<MatchSite>,
    CatgraphError>` and `rewrite_at(&ColoredExpr<G>, …)` are the same pair one
    level up, doing the `content_of_colored` / readback round trip for a single
    named step. Both levels exist because the round trip is not free: an
    `n`-step search through the expression level pays `n` contents and `n`
    readbacks, while the content level pays the readback **once**, at the end —
    which is how `optimize` is written. Both re-validate their expression at
    entry, with the same message: `match_sites_of` is the *first* step of the
    pair, so leaving it to panic through `content_of_colored` would abort across
    the documented serde trust boundary before `rewrite_at`'s screen was ever
    reached. A malformed term is an `Err`, never an empty `Vec` — "not a term"
    and "no sites" are different answers. The content-level `match_sites` still
    returns a bare `Vec`: it takes an already-built `Content` and has nothing to
    screen.

- **`Presentation::rewrite_depth()` — the depth bound is readable**
  (the A11(iii) serde-rebuild hazard, found in `catgraph-surreal` step-2). The
  sibling of the existing `engine()` accessor, and missing for no reason: a
  consumer that stores a presentation as its `(equations, depth, engine)` parts
  and rebuilds it later could read the engine back but not the depth, so a
  rebuild through `new()` + `add_equation` silently restored the **default 32** —
  no error, no warning, just a different bound than the one configured, and with
  it a different `converged` verdict on any presentation whose normalization
  needed the longer budget. Carry the value across such a round trip and rebuild
  through `with_depth(depth)` **followed by `set_engine(engine)`** — no
  constructor takes both, and `with_depth` defaults the engine, so a rebuild that
  stops there restores `CongruenceClosure` over a stored `Structural` and
  reintroduces the same silent-default bug one slot over.

### Fixed

- **`PropExpr::from_permutation` now realizes the permutation instead of
  returning the identity**
  ([#252](https://github.com/sustia-llc/catgraph/issues/252)).
  `<PropExpr<G> as SymmetricMonoidalMorphism<()>>::from_permutation` rejected a
  length mismatch and then returned `PropExpr::Identity(n)` for **every**
  correct-length permutation, discarding the permutation's action on wires. It
  was documented as a placeholder, but it type-checks and returns `Ok`, so any
  caller reaching it through the trait — including generic code written against
  `SymmetricMonoidalMorphism` with no idea which carrier it lands on — silently
  received the wrong morphism. The body is now the faithful decomposition:
  `perm[i] = p.apply(i)` is bubble-sorted by the `adjacent_swaps` core shared
  with `mat_to_sfg`, and each adjacent swap at position `t` contributes one
  braid layer `Identity(t) ⊗ Braid(1, 1) ⊗ Identity(n-t-2)`, composed in swap
  order (`O(n²)` layers).
  - **Convention** (unchanged from the sibling `permutation_sfg` in
    `mat_to_sfg` and from `MatR::permutation_matrix`): a wire entering at
    position `i` exits at position `p.apply(i)`. The perm is input-indexed, so
    no reversal is applied — unlike `presentation::smc_nf`'s `decompose_braid`,
    whose perms are output-indexed.
  - **Oracle.** `sfg_to_mat(from_permutation(p))` is asserted equal to
    `MatR::permutation_matrix(&p)` for *every* permutation of `n = 3` and
    `n = 4` (6 + 24 cases), plus named shapes at other widths. Every one of
    those comparisons collapsed to the identity matrix under the old body.
  - The length-mismatch rejection, its error message, and the
    `SymmetricMonoidalMorphism` trait are all unchanged. `types_as_on_domain`
    stays ignored, as in this crate's other single-sorted impls (`MatR`,
    `MatKron`): objects of a prop are natural numbers, the result is an
    endomorphism of `n`, and both sides carry the same object either way — now
    stated in the rustdoc and pinned by a test.

- **`PropExpr::permute_side` now actually permutes the requested side**
  ([#252](https://github.com/sustia-llc/catgraph/issues/252)). The same defect
  class as `from_permutation` above, in the sibling method of the same trait
  impl, and found by review of that fix.
  `<PropExpr<G> as SymmetricMonoidalMorphism<()>>::permute_side` spliced
  `PropExpr::Braid(0, n)` — that is `σ_{0,n}`, the **identity** on `n` wires,
  as `presentation::smc_nf`'s Step 0 rewrite (`Braid(0, n) → Identity(n)`)
  states outright. So `f.permute_side(&p, side)` handed back `f` wrapped in a
  vacuous composition for **every** `p`, on both sides: the same
  silent-wrong-answer shape, reaching the same generic
  `SymmetricMonoidalMorphism` callers. The body now splices the
  `from_permutation` network, which is faithful as of the entry above.
  - **Direction.** The two sides are *not* symmetric. The codomain side
    postcomposes `from_permutation(p)`; the domain side precomposes
    `from_permutation(p.inv())`. This follows `MatR::permute_side`, the
    semantics of record ("right-mul by `P` permutes columns (codomain side);
    left-mul by `Pᵀ` permutes rows (domain side)") together with
    `sfg_to_mat`'s `Compose(f, g) → S(f).matmul(S(g))`: `Pᵀ = P⁻¹` for a
    permutation matrix, and `permutation_matrix(p.inv())` *is* `Pᵀ`. The
    inversion is applied to the permutation, not to `types_as_on_domain` —
    that flag stays ignored, as above.
  - **Oracle.** The `S`-functor square
    `sfg_to_mat(f.permute_side(p, side)) == sfg_to_mat(f).permute_side(p, side)`
    is asserted for **both** sides over *every* permutation of `n = 3` and
    `n = 4` (12 + 48 cases), against a deliberately non-symmetric, invertible
    witness `f` — a symmetric or identity `f` can hide an inverted convention.
    The right-hand side runs the faithful `MatR::permute_side`, so the square
    cannot pass under a flipped direction. A direct `assert_ne!` against the
    input covers the no-op itself; the only prior test asserted that source and
    target were unchanged, which a no-op satisfies vacuously.
  - **Cost.** One `from_permutation` network per call: `O(n²)` braid layers,
    and `O(n²)` *deep* — the same left-nested `Compose` spine hazard
    `from_permutation` documents. Bounding arity magnitude stays the caller's
    obligation ([#197](https://github.com/sustia-llc/catgraph/issues/197)).
  - The defensive no-op on a length mismatch (a caller bug, and the trait
    signature is non-fallible) is unchanged, and now pinned by a test on both
    sides. `MatR`, `MatKron`, `mat_to_sfg`, `sfg_to_mat`, `adjacent_swaps`, and
    the `SymmetricMonoidalMorphism` trait are all untouched.

## [workspace-v0.11.0] - 2026-08-10

### Changed

- **`E1::random` now takes any `rand_core 0.10` generator, and the published
  randomness edge shrinks to `rand_core` alone**
  ([#239](https://github.com/sustia-llc/catgraph/issues/239)). The signature
  is `E1::random(cur_arity, rng: &mut impl Rng)` with `Rng` = `rand_core
  0.10`'s infallible generator trait (note the upstream 0.10 rename:
  `RngCore` is now a deprecated stub pointing at `Rng`). The uniform \[0, 1)
  sampling moved in-tree — a 24-bit `f32` ladder over `next_u32`, exact on
  the `f32` grid, maximum `1 − 2⁻²⁴`, genuinely half-open — so this crate's
  `[dependencies]` carry `rand_core` instead of `rand`: no distributions, no
  engines, no OS-entropy path anywhere in `src`. `rand` itself is dev-only
  now, workspace-wide, enforced by a CI guard
  (`scripts/check_rand_dev_only.py`). Downstream lib graphs of this crate
  (and of magnitude / dl / syntax through it) shed `rand` entirely.
  - **Source-compatible.** `rand 0.10`'s `RngExt` is a subtrait of the same
    `rand_core::Rng`, so existing call sites — engines passed directly, or
    generic code bounded `R: RngExt` — compile unchanged. The one
    requirement: the engine must sit on the rand_core **0.10** line. An
    0.9-line engine (`rand 0.9`'s `StdRng`, `rand_chacha 0.9`'s
    `ChaCha20Rng`) fails the bound with a two-same-named-traits diagnostic;
    the fix is bumping the caller's own rand-family crates to the 0.10 line.
  - **Supply contract.** `catgraph_applied::{Rng, TryRng}` are re-exported so
    callers can *name* the bound — a generic wrapper, or a custom engine via
    `TryRng<Error = Infallible>` (`Rng` is blanket-implemented over `TryRng`
    and cannot be implemented directly) — without a direct `rand_core`
    dependency. Engines themselves still come from the caller's own RNG
    crate. Note the coupling this creates: rand_core's major version is now
    part of this crate's public API, so a future rand_core 0.11 adoption is
    a breaking change here.
  - **Behavioral note.** Draws remain i.i.d. uniform on \[0, 1), but the
    stream differs from `rand`'s `random_range(0.0..1.0)`: a seeded sequence
    of `E1::random` configurations is not bit-identical across this change.
    Nothing in-tree pins exact drawn values (the seeded suites assert
    structural invariants only); new direct tests pin the ladder's exact
    endpoints, and a custom `TryRng` engine test pins the supply contract
    end to end.
  - The [#232](https://github.com/sustia-llc/catgraph/issues/232)
    feature-unification caveat (any build graph containing `catgraph-physics`
    re-enables `rand`'s defaults through `rustworkx-core`) is unchanged — see
    the #232 entry below; it is now a statement about physics-containing
    graphs only.

## [workspace-v0.10.0] - 2026-08-09

### Fixed

- **Browser-wasm lib builds of this crate no longer fail in `getrandom`**
  ([#232](https://github.com/sustia-llc/catgraph/issues/232)). The workspace
  `rand` entry drops its default features entirely: the default
  `sys_rng`/`thread_rng` pair pulled `getrandom`, whose `compile_error!` aborts
  every browser-wasm build that reaches it — and this crate's `rand` edge was
  the sole path to it for `catgraph-magnitude`, `catgraph-dl`, and
  `catgraph-syntax` too. The verified claim, exactly:
  `cargo check --lib -p <crate> --target wasm32-unknown-unknown` now passes for
  all four crates. Dev graphs still reach `getrandom` through
  `proptest`, so `--all-targets`/`--tests` forms on that target still fail —
  the guarantee is the lib/normal-dependency graph.
  - No API or behaviour change. The only non-test `rand` surface this crate
    exposes is `E1::random(cur_arity, rng: &mut impl RngExt)`, whose RNG is
    caller-supplied; nothing here reads OS entropy. The lib edge now carries
    **no** rand features at all — the seeded `StdRng` tests and `mat_ops_bench`
    fixtures moved their `std`/`std_rng` needs to this crate's own
    dev-dependency edge, so downstream lib graphs also shed `chacha20`.
  - **Feature-unification caveat.** `rustworkx-core` (behind
    `catgraph-physics`' default `rustworkx` feature) declares `rand` WITH its
    default features, and cargo unifies features per crate version across a
    build graph. Any invocation that includes `catgraph-physics` — a
    `--workspace` build, or a downstream depending on both crates — re-enables
    `sys_rng`/`thread_rng`, and `getrandom` is back. The slim graph is real
    only physics-free: per-package `-p` builds, or physics with
    `--no-default-features`. Native workspace builds and compile times are
    unchanged for the same reason. The per-package CI wasm lane guarding this
    is [#233](https://github.com/sustia-llc/catgraph/issues/233).
  - **Downstream note.** The published manifest now declares featureless
    `rand`, so this crate no longer contributes rand's defaults to a consumer's
    feature union — a consumer that was getting `thread_rng`/`sys_rng` "for
    free" through this crate must now enable them itself. Callers of
    `E1::random` supply the RNG from their own `rand 0.10` dependency (this
    crate re-exports nothing from `rand`).
  - **Runtime caveat for browser builds.** The default `parallel` feature
    compiles `rayon` cleanly on `wasm32-unknown-unknown`, but that target
    cannot spawn threads at runtime — browser consumers should build with
    `--no-default-features`, matching the README's wasip1 recipes.
  - **Not a change of supported-target policy.** Browsers stay out of scope
    (the policy statement lives in catgraph's README, WASM section);
    `wasm32-wasip1` remains the wasm lane. This removes an accidental blocker,
    it does not add a tier.

## [workspace-v0.9.0] - 2026-08-04

### Added

- **`rig::Zero` and `rig::One` — the identity traits are now catgraph's own**
  ([#219](https://github.com/sustia-llc/catgraph/issues/219), D1 of the
  [#218](https://github.com/sustia-llc/catgraph/issues/218) dependency
  streamlining). Defined in `src/rig.rs` beside the `Rig` they pair with, with
  the same shape they replace (`zero`/`is_zero`, `one`/`is_one`, each with the
  corresponding `Add`/`Mul` supertrait), and implemented for every primitive
  integer and float — so the blanket `Rig` lift over primitives is unchanged.
  The whole rig substrate is now owned end to end.

### Changed

- **`deep_causality_num` is no longer a dependency of this crate**
  (#219). `Rig`'s `Zero`/`One` bounds resolve to the traits above; every impl
  (`BoolRig`, `UnitInterval`, `Tropical`, `F64Rig`, `Checked<T>`, `Z`) moved
  across unchanged, and no rig's arithmetic, identities, or axiom results
  differ. `num` stays, still narrowed to BigInt / Complex / ToPrimitive / pow.
  Note that the crate is not gone from the lockfile: it remains transitively
  reachable under `deep_causality_haft`, which catgraph-dl still uses.
  - **BREAKING for downstream scalars.** A crate implementing `Rig` for its own
    type by implementing `deep_causality_num::{Zero, One}` must now implement
    `catgraph_applied::rig::{Zero, One}` instead. The method signatures are
    identical, so the change is the import path. Types reaching `Rig` only
    through the shipped instances or primitives need no change.
  - `tests/rig_dc_substrate.rs` renamed to `tests/rig_identity_substrate.rs`,
    and extended: the primitive impls, the `-0.0`-is-zero float behaviour, and
    `Checked<T>`'s poison-rejecting `is_zero`/`is_one` are now covered directly.

- **`temperley_lieb`'s composition connectivity is a union-find pass, and the
  `ultragraph` dependency is gone**
  ([#220](https://github.com/sustia-llc/catgraph/issues/220), D2 of the
  [#218](https://github.com/sustia-llc/catgraph/issues/218) dependency
  streamlining). Brauer diagram composition reads the glued endpoint matching
  and the closed-loop (δ-power) count off the connected components of the glued
  diagram; that is now `union-find`'s `QuickUnionUf<UnionBySize>` — the same
  substrate `catgraph` uses for its cospan/corel pushouts — instead of a
  strongly-connected-components pass over a directed graph. Results are
  unchanged; the Temperley-Lieb, symmetric-algebra, and tangle relation suites
  all still hold.
  - The rewrite is a simplification, not a transliteration. Diagram arcs are
    undirected, so the old code had to add each one in *both* directions for
    SCC-count to equal the undirected component count; that workaround is gone.
    So is the lazily-populated `Vec<Option<usize>>` node-index map — points now
    use their diagram ids directly, with `rhs`'s offset by `self_dom`, which is
    the gluing written down rather than constructed.
  - `connectivity::resolve` is total, so `<ExtendedPerfectMatching as Mul>::mul`
    no longer carries an `.expect()` for an error path that could not occur.

## [workspace-v0.8.0] - 2026-08-03

### Added

- **`prop::presentation::rewrite` — a process cost functional and bounded
  convex-DPO rewriting** ([#214](https://github.com/sustia-llc/catgraph/issues/214)
  W2 + W3; [#57](https://github.com/sustia-llc/catgraph/issues/57) a2, in its
  reframed optimizer form). A new module beside `content` / `display`; the
  engine (`smc_nf`, `kb`, `eq_mod`, `normalize`, `eq_colored`) is untouched, and
  no pin moved.

  - `cost_of(&Content<G>, per_gen: impl Fn(&G) -> u64) -> u64` sums a
    caller-supplied per-generator price over the content's hyperedges (default
    `|_| 1` = generator count). Defined on **content**, so it is a function of
    the morphism's SMC-iso class (Lemma 4.1, `docs/SMC-NF-RECONCILIATION.md`
    §4.2) rather than of the writing — which is what §4.6's recorded rejection
    of the `out_min` comparator asked for ("lower is not the objective; a
    comparator that is a function of the morphism is"). Deliberately **not**
    invariant under user equations: that difference is the optimization signal.
    No paper anchor exists for a cost functional on string diagrams; marked an
    **extension**.
  - `RewriteRule::new(lhs: ColoredExpr<G>, rhs: ColoredExpr<G>)` compiles an
    oriented equation to a content-level span, rejecting — never panicking —
    non-parallel sides, an ill-formed side, an edge-free left-hand side, and a
    **non-mono left interface** (a repeated boundary node, where the pushout
    complement stops being unique).
  - **The serde trust boundary is re-validated at every entry point.** Each of
    `RewriteRule::new`, `optimize` and `replay` re-checks its `ColoredExpr`
    inputs once, on both clauses: arity (including the
    [#196](https://github.com/sustia-llc/catgraph/issues/196) overflow screen)
    *and* words — `colored::check` is re-run against the declared source word
    and the target word it derives must equal the stored one, which is the
    re-validation `colored`'s documented trust boundary prescribes. An arity
    screen alone admits a forged document whose `Compose` joins matching widths
    with mismatched colors, and the colored matcher would then read node colors
    no `⟦·⟧` ever assigned.
  - `optimize(start, rules, fuel, per_gen) -> Result<RewriteOutcome<G>,
    CatgraphError>` searches best-first over states keyed by `canonical_key` —
    the sanctioned dedup use — applying convex matches (BGKSZ
    [arXiv:1602.06771] Def 3.10 / 5.4–5.5) and spending at most `fuel` rewrite
    applications. `RewriteOutcome` reports the best expression, the initial and
    best cost, a replayable trace, whether fuel was exhausted, and how many
    states were seen. `replay` re-derives the state a trace describes,
    re-validating each step as a convex match — relative to `rules` as given,
    since a trace binds rule *indices*, not rule identities.
  - **Output validation is two checks, not one.** The best state is read back
    via `display::expr_of_content`, re-checked through `ColoredExpr::new`, and
    then compared back against the state it came from with `content_eq`. The
    second check is what discharges `expr_of_content`'s round-trip property at
    runtime — that property is corpus-verified, not proven — so a readback that
    lost the content is an error rather than a silently wrong answer.
  - **Claims discipline.** Per-step soundness is anchored to BGKSZ **Thm 5.6**
    (convex-DPO adequacy for plain SMC): each applied step is a rewriting step
    modulo SMC structure with the given rules. There is **no termination claim**
    (§4.7's Lafont correction of record stands — his proof covers PROP `F` only,
    and the commutative case is unproven in the cached anchors; fuel is the
    honest bound) and **no confluence / normal-form / canonicality claim** —
    best found under fuel. `eq_mod` and `eq_colored` remain the deciders. The
    Λ-colored lift of BGKSZ's single-sorted setting is marked an **extension**;
    the colored conclusion leans on the color-generic lifts of Lemma 3.11 /
    Thm 3.12 recorded in `docs/SMC-NF-RECONCILIATION.md` §4.2's Lemma 4.1 proof,
    which realize Thm 5.6's factorization contexts as colored terms, and color
    matching strictly refines matches, so soundness is inherited. Spiders
    participate as opaque generators only: the Frobenius unrestricted-DPO substrate
    (Thm 4.6) and MPZ23's commutative-(co)monoid middle case are out of scope.

  [arXiv:1602.06771]: https://arxiv.org/abs/1602.06771

### Changed

- **Internal:** `prop::presentation::content` gains a `pub(super)`
  `from_parts` constructor — the DPO step above cannot be phrased as "walk an
  expression", so its parts arrive raw and the constructor re-establishes the
  BGKSZ Thm 3.12 image characterization by *checking* it, in four clauses:
  index/length ranges; tentacle counts against the label's declared words
  **and the colors those words declare at each tentacle position** (mirroring
  `content_of`'s typing discipline, so a mis-transported color cannot reach the
  engine — and so `minimal_block`'s color-consistency assertion is upheld in
  release builds too); **monogamy** in all three of Def 3.6's clauses — no node
  with two producers or two consumers, each **anchor leg mono** (no node in two
  `input` coordinates, none in two `output` coordinates; one of each is legal
  and is exactly `id₁`), and the boundary biconditional; and **acyclicity**.
  Not public API; `Content`'s fields and public surface are unchanged.

## [workspace-v0.6.0] - 2026-08-02

### Added

- **An interleave-biased third tier in the SMC-NF differential sweep**
  ([#183](https://github.com/sustia-llc/catgraph/issues/183)), so residual (a)
  — marked (interleaved) components, the one lettered residual of
  `docs/SMC-NF-RECONCILIATION.md` §4.6 still open, tracked on
  [#174](https://github.com/sustia-llc/catgraph/issues/174) — has a tracker
  that stresses it rather than reaching it incidentally.
  `published_interleave_mode_figures_reproduce` in
  `tests/smc_nf_differential_sweep.rs` (`internal-probes`, `#[ignore]`d,
  100 000 pairs, seed `0x94D0_49BB_1331_11EB`) sweeps a corpus whose generator
  forces an `A…B…A` input owner word **by construction** and **braid-free**:
  one component's two `1 → 1` arms flank terminal `Discard` splitters and are
  rejoined by a `μ` in the next layer, so `analyze_components`' union-find puts
  the two arms in one component and each splitter — target-empty, hence joined
  to nothing below — in its own, and `mark_interleaved` fires on 100% of cases.
  New two-sided pins **745 divergent / 0 in-`𝔉` / 745 marked**, over
  **100 000** marked cases. Against §4.6's motivating deficit that is 32× the
  default corpus's 23 marked divergences and 3.1× the braid corpus's 237, and
  unlike the braid tier — whose marking a coarsening `Braid` makes an upper
  bound on content-level marking (§4.4) — every case here is marked at the
  content level. The two existing tiers' pins were unchanged by *this* tier
  (253/128/23 and 1162/634/237 at the time); §4.6(a) now names both trackers.
  Both were re-pinned later in this same unreleased cycle by
  [#185](https://github.com/sustia-llc/catgraph/issues/185) — see the Fixed
  entry below — while this tier's own 745 / 0 / 745 stayed put.

### Changed

- **The faithfulness tracker counts connected components, not greedy classes**
  ([#189](https://github.com/sustia-llc/catgraph/issues/189)).
  `graphical_linalg::verify_sfg_to_mat_is_full_and_faithful` partitioned each
  matrix bucket by scanning every expression against the current class
  *representatives*. That is the component partition only when the relation is
  transitive, and `Presentation::eq_mod` is not: `Scalar(false)` ~
  `Discard ; Zero` (the D8 user equation) ~ `Discard ⊗ Zero` (the SMC layer)
  while `Scalar(false)` ≁ `Discard ⊗ Zero` — a `Some(false)`, not a `None`.
  Measured in #189 on a 120-expression pool of parallel `1 → 1` arrows (the
  pool is recorded there): 10 490 ordered violating triples, zero `None`
  verdicts, so this is congruence closure's incompleteness, not a depth-bound
  artefact. Non-transitivity is pre-existing and was found by the #57-a1
  adversarial review. The bucket partition is now the **connected components**
  of the graph whose edges are exactly the
  `Some(true)` pairs, taken by a plain union-find; a pair already inside one
  component is skipped, which is exact (an intra-component edge cannot change
  the partition) and is what keeps the all-pairs pass affordable.

  **CC collision baselines re-pinned, all four down:** BoolRig 952 → **748**,
  UnitInterval 1397 → **1114**, Tropical 1974 → **1594**, F64Rig 1969 →
  **1590**. Explain-the-delta: the greedy partition **over-counted relative to
  connected components** — every greedy class sits inside one component, so
  components are coarser-or-equal and all four rigs had to fall. Nothing about
  the relation moved: the only executable change in this release entry is the
  tracker's partition loop, so `eq_mod`, `nf`, the presentation and every corpus
  behave exactly as before. This is a change of *metric* alone, and its
  direction was forced rather than measured. What the switch buys is that the
  count is now a **function of the relation** — components are the transitive
  closure of the same edge set, whereas a greedy class count depends on
  enumeration order — which restores the monotonicity argument the pins are
  read under: a relation that only grows can only gain edges, hence only merge
  components, hence only lower the count. The #57-a1 re-pin had to appeal to the empirical direction
  for want of exactly this. Canonicality is unaffected and still judged by
  `smc_canonicality_probes`, not by these counts.

  Cost: depth-2 bucket sizes top out at 682 (BoolRig) and 1602 (Tropical,
  F64Rig), an all-pairs ceiling of 1.6M–4.5M `eq_mod` calls per rig; with the
  intra-component skip the BoolRig depth-2 tracker runs ≈2 min against the
  greedy scan's ≈11 s. The trackers stay `#[ignore]`'d, so CI time is
  unchanged; the `cc_incompleteness_count` bench groups' profiled wall times
  were re-measured (≈120 s / ≈129 s per call) — a cost step that then led to
  the groups' removal, recorded in the Removed entry below.

  `Presentation::eq_mod`'s rustdoc now records the non-transitivity directly:
  sound and definite per query, but not an equivalence relation as a decision
  procedure, so a caller wanting a partition must take components.

### Removed

- **Both `functor::cc_incompleteness_count::{bool, f64rig}/2` bench groups**
  ([#189](https://github.com/sustia-llc/catgraph/issues/189), owner decision
  2026-08-02). The #189 all-pairs component partition moved one `d=2`
  `verify_sfg_to_mat_is_full_and_faithful` call from ≈7 s to ≈120 s (`bool`) /
  ≈129 s (`f64rig`), putting `cargo bench -- cc_incompleteness` at ≈45 min
  wall with `sample_size(10)` already at criterion's floor — no configuration
  brings the pair back under a minute. The witness-count signal of record was
  never the bench: the `#[ignore]`'d `cc_completeness_tracking_*` trackers in
  `tests/graphical_linalg.rs` carry the pinned counts (748/1114/1594/1590,
  post-#189) and stay. `benches/functor_bench.rs` keeps the two `sfg_to_mat`
  groups and records the removal in its module doc.

### Fixed

- **Step 6½'s column cuts are symmetric — both runs local**
  ([#185](https://github.com/sustia-llc/catgraph/issues/185)). The shipped seed
  test took the *left* column as a maximal **local** run but demanded the
  *right* component's **whole layer presence** be one contiguous run. Neither
  `docs/SMC-NF-RECONCILIATION.md` §1's Step-6½ invariant clause nor §4.5's own
  column definition ever said that, so a fragment-symmetric, interval-aligned,
  strictly-commuting column pair whose right column belonged to a component
  that also owned an atom elsewhere in the row (`[L, B, L]`) was **declined**,
  and the inverted, non-excepted pair survived to the fixpoint.

  `adjacent_column_cuts` is replaced by `adjacent_column_cuts_at`: the seed is
  an adjacency **at a position**, and its two columns are the two maximal local
  runs meeting there. Because a row can now hold several `(c1, c2)`
  adjacencies, widening the interval became a search — it takes the
  neighbouring row's adjacency whose three cuts *meet* the current edge layer's
  (`cuts_meet`, the same internal-boundary test `column_pair_is_transposable`
  applies), scanning positions leftmost-first. Cut coordinates are
  non-decreasing across a row and `cuts_meet` reads one side per row (`tgt`
  above a boundary, `src` below), so two adjacencies align identically exactly
  when every atom in the span between them — their own runs included — has zero
  width **on the side being read** there; one-sided-zero atoms (`η`, `ε`)
  qualify, and a `0 → 0` atom is only the example that ties both readings.
  Leftmost-first decides that case deterministically, and it costs completeness
  nothing: a tie boundary has all six cut coordinates equal, which *is* strict
  commutation at that boundary, so any violating pair's tie-free segment is
  still aligned, admissible, inverted and transposable, and the sub-interval
  search still returns it (the **truncation lemma**, §4.4). Empirically the tie
  also never fired: Phase A's temporary widening instrumentation (since removed)
  over the 600 000 normalizations of the three sweep corpora (3 × 100 000 pairs,
  two writings each) counted **0** two-aligned-candidate ties, with 145 widening
  steps seeing more than one candidate at all and 18 widening steps having to
  skip a *misaligned* leftmost one — per-widening-step event counts kept as
  provenance, not numbers-of-record; the skip is pinned by the new
  `column_widening_picks_the_interval_aligned_adjacency`. No guard, no
  direction and no equal-key decline changed;
  `tests/pass_disjointness_probes.rs` is green with no pin moved.

  **Pass-disjointness probes, post-review (2026-08-02).** Seeding is strictly
  *wider*; among the newly-seeded split-presence sites, the both-readings ones
  are where the carve and the #186 exact-cancellation exit are newly exercised
  (the rest fire ordinary productive moves). `pass_disjointness_probes.rs` gains
  `split_presence_both_readings_pair_is_newly_seeded` (**P1**): shipped
  generators, inside `𝔉`, a `[Id_L, Zero_B, Discard_L]` row whose contested
  `Zero | Discard` boundary admits both readings and which the pre-#185 seed
  test declined outright. Its fixpoint is Step-6-sorted, idempotent, and equal
  to the column-pass-ablated normal form — the newly-seeded fight cancels
  exactly. The file's `column_violations_len1` checker is extended to the same
  symmetric local-run reading: it had kept the pre-#185 whole-presence
  `contiguous_run`, so it could not see split-presence violations at all — P1's
  is the first it flags. No existing probe's verdict moved.

  **Measured outcome — every move is a convergence.** Across the three
  100 000-pair sweep corpora, **79** SMC-equal pairs stopped diverging (70
  default, 9 braid) and **none** became divergent in any mode. No surviving
  divergence changed bucket, the interleave tier's divergent index set is
  bit-identical, and the four congruence-closure collision pins are unmoved
  (748 / 1114 / 1594 / 1590). Health over the same 600 000 normalizations:
  0 idempotence failures, 0 content losses. Content never moved either — all 79
  pairs were already `content_eq`- and `canonical_key`-equal, and already
  convergent under `canonical_display`.

  **Pins re-pinned, in lockstep with the docs that quote them:**
  `smc_nf_differential_sweep` **253 / 128 / 23 → 183 / 93 / 23** (default),
  **1162 / 634 / 237 → 1153 / 630 / 237** (braid), 745 / 0 / 745 unmoved
  (interleave), smoke prefix 16 → 14; `content_equality_corpus`
  253/253 → 183/183 and 1162/1162 → 1153/1153; `canonical_display_corpus`
  churn 5 334 → 4 585 and `layer_pinned_agree` 39 991 → 40 075 (default),
  churn 19 098 → 18 977 and `layer_pinned_agree` 32 270 → 32 287 (braid), with
  every other gate field, both correctness columns and every tier denominator
  verified unmoved.

  **Docs and witnesses.** §4.4's canonicality corollary on `𝔉′` is no longer
  *conditional*: its invariant-satisfaction condition is discharged by an
  argument written out there in three steps — the seed predicate now coincides
  with the §1 clause's column pair, `find_column_transposition` scans every
  `(layer, position)` so every clause-violating pair is reachable, and Step 6½
  runs to its own fixpoint inside a loop that exits only on a whole-pass
  no-change, so a fixpoint holds no *non-excepted* violation. Step two carries
  the **truncation lemma** (#185 adversarial review, 2026-08-02), which replaced
  the first draft's empirical-only qualification: the leftmost tie-break can no
  longer leave a violating pair standing even in principle, so the discharge is
  unconditional rather than "exact wherever the tie does not fire". That is a
  statement about the corollary's **side conditions only** — it still inherits
  Theorem 4.5's proof-sketch density and its **two flagged-open induction
  steps**, which #185 does not touch. §1's "Known deviation" note is deleted,
  §4.4's pass-disjointness note records that seeding is strictly wider (the
  carve and #186's exit newly exercised at the both-readings subset of the new
  sites, P1 among them),
  §4.5 gains a dated symmetrization entry (and withdraws its "complements, not
  overlaps" gloss — the new pair is in `𝔉′` and the pass fires on it), §4.6
  records the tracker movement and the two motivating indices (5417, 51534),
  and §2.4 records that local runs do not weaken `column_inversion_count`.
  The F1 witness `cut_asymmetry_separates_smc_equal_writings_inside_f_prime`
  is flipped to `assert_eq!` and renamed
  `split_presence_nesting_converges_with_free_writing` per the
  residual-(b)/(c)/(d) precedent; F2 (`braid_prefix_is_not_content_derived`)
  stays divergent with its rename instruction live. The ablation table grows
  5 → 7 (`column_pass_decides_exactly_the_seven_documented_witnesses`), and
  the 5 780-rejection alignment figure in §4.5 is relabelled a pre-fix
  (2026-07-28) measurement.

- **Wire-count sums saturate instead of overflowing**
  ([#180](https://github.com/sustia-llc/catgraph/issues/180)):
  `prop::colored::check` / `infer` and the `PropExpr::source` /
  `PropExpr::target` arity fold now saturate `Braid` and `Tensor` sums at
  `usize::MAX` instead of overflowing. Raw variant construction is
  documented-legal, so a `Braid(usize::MAX, 1)` was reachable and used to
  panic in debug builds and wrap onto a small, spuriously valid arity in
  release. `usize::MAX` matches no real wire bundle, so the saturated value is
  reported as `CompositionSizeMismatch` — reject-don't-wrap, matching the
  hardened syntax-crate interpreters (`to_cospan` / `to_mat_kron`; `eval` joins
  them in the #196 entry below).

- **The deeper passes reject an overflowing arity instead of consuming it**
  ([#196](https://github.com/sustia-llc/catgraph/issues/196)) — the residue
  #180 left, and with it the policy is uniform across the workspace.
  Saturating is the right answer only where a sum is *compared* against a real
  slice length. `content_of` sizes a node vector from a braid's `m + n`,
  `check_equation` sizes its fresh source word from `lhs.source()`, `smc_nf::nf`
  decomposes a `σ` of that many wires, `sfg_to_mat` allocates a `dim × dim`
  matrix, and the `Identity(m) ⊗ Identity(n) → Identity(m + n)` rewrite stores
  it — at each, `usize::MAX` is an allocation abort or a non-terminating loop,
  not a rejectable sentinel. Concretely:
  - `PropExpr::checked_arities` / `PropExpr::arities_fit` (**new**) are the
    exact companions to the saturating `source` / `target`, and the domain test
    the passes below screen with.
  - `content::is_arity_well_formed` now answers `false` on an overflowing
    `Braid` or `Tensor` width as well as on a mismatched `Compose`; the
    `expect` inside `content_of` makes a release build reject rather than wrap.
    (The screen covers overflowing *sums*; an infeasibly huge arity written
    literally — `Identity(usize::MAX)` — involves no sum, passes it, and stays
    the caller's obligation, with the derived layer-width class:
    [#197](https://github.com/sustia-llc/catgraph/issues/197).)
  - `smc_nf::nf` gains a documented `# Panics`: it rejects an overflowing width
    at its entry point, in both build profiles. A *mismatched* `Compose` is
    unaffected and still normalizes — `eq_mod`'s fallback depends on it.
  - `Presentation::add_equation` reports `CompositionSizeMismatch` when the LHS
    arity is `usize::MAX`, before it sizes anything from it. (PR #195's
    regression test put its overflowing braid on the RHS to avoid exactly this;
    the RHS verdict is unchanged.)
  - `sfg_to_mat` reports `SfgFunctor` on an overflowing braid width, through the
    error arm it already had for direct-`PropExpr` misuse.
  - **Behavioral, on input no constructor can produce:**
    `Presentation::eq_mod` screens the overflow class ahead of either engine and
    answers `Ok(Some(true))` on structurally identical trees, `Ok(None)` —
    undecided — otherwise; `ColoredExpr::eq_colored` (reachable only across the
    serde trust boundary) falls back to structural equality. Neither returns a
    disproof no layer established. Both previously panicked.

## [workspace-v0.5.0] - 2026-07-30

### Changed

- **BREAKING (behavioral): the SMC layer of `Presentation::eq_mod` is now
  decided by content, not by the normal form**
  ([#57](https://github.com/sustia-llc/catgraph/issues/57), a1 PR2). Under the
  default `NormalizeEngine::CongruenceClosure`, the `nf(a) == nf(b)`
  short-circuit is replaced by `content_eq(content_of(a), content_of(b))`.
  Content decides SMC-equality *exactly* (Lemma 4.1, `SMC-NF-RECONCILIATION.md`
  §4.2) where `nf` was sound but incomplete, so **`eq_mod` returns
  `Ok(Some(true))` on strictly more pairs than before** — every SMC-equal pair
  the normal form separates, which is all 253 divergences of the published
  default corpus and all 1162 in braid mode. No pair that was equal becomes
  unequal: `nf` preserves content (§4.3 Lemma 4.2), so the content relation
  *contains* the NF relation, and the decided-equal relation only grows
  (`tests/content_equality_corpus.rs` pins that containment on 2000 unrelated
  pairs; `nf_preserves_content_across_the_corpus` is the direct Lemma 4.2
  check). `ColoredExpr::eq_colored` gets the same treatment via
  `content_of_colored`, and with it a strengthened contract: on
  word-well-formed values it now **decides** colored SMC-equality, so a `false`
  is a disproof where previously it was not.
  `NormalizeEngine::Structural` is untouched. The user-equation layer is
  untouched: `Copy ; Add` and `Copy ; σ ; Add` are still unequal without the
  Thm 5.60 equations, because cocommutativity is a user equation and content
  quotients by SMC coherence alone.

  **Totality is preserved.** `content_of` panics outside its arity-well-formed
  domain, where `eq_mod` has always answered rather than panicked, so the
  content arm is gated on the new `content::is_arity_well_formed` and an
  ill-formed tree falls through to the pre-#57 `nf` short-circuit and then to
  congruence closure — reaching exactly its previous verdict. No new error
  variant.

  **CC collision baselines re-pinned, all four down:** BoolRig 980 → **952**,
  UnitInterval 1433 → **1397**, Tropical 2018 → **1974**, F64Rig 2013 →
  **1969**. Explain-the-delta: the metric buckets by matrix image first, and
  Thm 5.60 makes the matrix ground truth, so a bucket splitting into `k`
  `eq_mod`-classes contributes `k − 1` and the count measures equalities
  `eq_mod` *fails to prove*. What is **forced** is that the underlying relation
  only grew — the content relation contains the NF relation it replaced (`nf`
  preserves content, §4.3 Lemma 4.2), so no previously provable equality became
  unprovable. The *count* carries no such guarantee: it is a greedy-partition
  statistic over a **non-transitive** `eq_mod` (`Scalar(false)` ~
  `Discard ; Zero` ~ `Discard ⊗ Zero` while `Scalar(false)` ≁
  `Discard ⊗ Zero`), and over a non-transitive relation the greedy class count
  is not a function of the relation, so enlargement is not provably monotone.
  The observed direction is **empirical** — all four fell. A
  union-find-component tracker would restore monotonicity at the cost of new
  baselines: filed as
  [#189](https://github.com/sustia-llc/catgraph/issues/189), deferred — its
  body carries the non-transitivity measurement and the pool it was taken on.
  **The metric also narrowed in meaning:** the short-circuit conflation is gone
  (no residual is attributable to NF incompleteness at the SMC layer, which is
  now decided exactly), but `nf` still reaches the count through
  `kb::CongruenceClosure`'s `smc_refine`, so an NF change can still move these
  pins. [#173](https://github.com/sustia-llc/catgraph/issues/173)'s
  "conflation" note is partially addressed; #173 stays open.

### Added

- **Canonical string-diagram display — `prop::presentation::display`**
  ([#187](https://github.com/sustia-llc/catgraph/issues/187), PR1).
  `canonical_display(e) = nf(expr_of_content(content_of(e)))`, plus the
  readback `expr_of_content` it is built from. Two SMC-equal expressions get
  the **same display on every diagram**, including the families the normal form
  separates — §4.6's ledger, the marked residual-(a) cases, and the dead
  braid-prefix shapes no NF-level fix can reach. Canonicality is by
  construction rather than by a uniqueness theorem: the readback reads only the
  canonical relabeling of the content, which factors through the complete
  invariant `canonical_key`, so SMC-equal inputs reach *identical* expressions
  before `nf` is called at all.

  **Display only, and nothing else moves.** `nf` is untouched — same code, same
  fixpoints, same separations — and so are `content_of`, `content_eq`,
  `canonical_key`, `kb::smc_refine`, `Presentation::eq_mod`, every CC collision
  baseline (952 / 1397 / 1974 / 1969) and both differential-sweep trackers
  (253 / 1162). Equality was already decided exactly by content (#57 a1); what
  was missing was a canonical *picture*.

  Measured on the frozen 100 000-pair corpora (`tests/canonical_display_corpus.rs`,
  `internal-probes`). Correctness: the readback preserves content on all
  200 000 contents of each mode, and all 100 000 pairs of each mode converge —
  zero failures on both counts, both modes. Quality: the readback inserts a
  crossing on 1 628 of the 100 000 default-corpus cases (1.6 %; braid-free by
  construction, so those crossings are the sweep's own), and the display
  differs from `nf` on 5 334 (5.3 %; 19.1 % in braid mode, where `nf` keeps the
  dead braid prefixes the display drops). Cost is **reported, not asserted** —
  a wall-clock ratio is not a pin, and the test says so in its own docstring: a
  `canonical_display` call measured roughly 2.4× an `nf` call (≈ 21 µs against
  ≈ 8.6 µs on the corpus's shape distribution, release build, one machine).

  **Theorem 4.5 probe** (`SMC-NF-RECONCILIATION.md` §4.4): where the theorem's
  hypotheses are fully checked — §4.4's `η`-free corollary, 16 103 default-corpus
  cases and 11 756 braid-mode cases — `nf` and the display agree **exactly**,
  on every case, in both modes. Where slot-pinnedness has to be *assumed*
  because it is realization-quantified (the layer-pinned tier), 324 of 40 315
  disagree on the default corpus and 54 of 32 324 in braid mode. Narrowing to
  connected contents leaves **three distinct contents** (measured 2026-07-30):
  default cases 2996 and 22872 are one content, `Copy ; (id₁ ⊗ Discard ⊗ Zero) ; Add`,
  four hyperedges, pinned as the unit witness
  `display::tests::a_layer_pinned_eta_can_still_take_two_layers`; default case
  45412 is a second, six hyperedges; braid-mode case 96178 is a third, seven.
  Each was checked individually and each is **slot-slack** — the `η`'s wire
  coordinate below its own layer differs between the two realizations — hence
  outside `𝔉′`, hence not a counterexample. The multi-component remainder of
  the tier is *unexamined*; that it is the slot-pinnedness assumption failing
  there is an expectation, not a finding. What the pinned witness adds to §4.4
  is **non-uniqueness of `λ` under layer-pinnedness** (one content, two
  realizable layers), strengthening the layer-pinnedness caution, which
  established only that `ceil = 1` does not force `λ = 0`. It does *not* bear
  on the flagged-open induction step (a), which is discharged under
  slot-pinnedness — a hypothesis this content fails.

- **Display-convergence witness siblings, and the ratified semantics written
  down** ([#187](https://github.com/sustia-llc/catgraph/issues/187), PR2 — the
  closing PR of the arc). `tests/smc_nf_completeness.rs`'s `eta_slack_residual`
  module gains `display_converges_on_the_eta_slack_writings` and
  `nf_separates_where_the_display_converges`: the three writings `nf` holds in
  two classes share one `canonical_display`. `beyond_eta_slack_residuals` gains
  `display_converges_on_both_beyond_eta_witnesses`, including the F2 dead braid
  prefix no `nf`-level fix reaches.

  **This is not flip-and-rename.** The `nf`-level witnesses keep their names
  and their `assert_ne!`s, because they are now **permanent facts about `nf`**
  rather than defects awaiting repair: "#187 fixed" was ratified to mean
  *display convergence*, and #187 left `nf` untouched by construction. The
  superseded instruction in `eta_layer_slack_separates_smc_equal_writings`
  ("rename these witnesses per the residual-(b)/(c)/(d) precedent") presumed an
  `nf`-level fix and is retired; the assert message now says so and points at
  the sibling. The rename instructions in `beyond_eta_slack_residuals` stay
  **live** — those guard engine facts (the filed `adjacent_column_cuts` defect,
  the per-writing braid-freeness property), which the display converging does
  not discharge.

  Docs: `SMC-NF-RECONCILIATION.md` gains four dated 2026-07-30 notes — §4.7
  (the readback the section anticipated is shipped; `eq_mod` adoption stays a
  separate #173-adjacent decision), §4.4 (the layer-pinnedness caution
  strengthened to non-uniqueness of `λ`, with the cross-width caveat on the
  slot-slack comparison; the Theorem 4.5 probe now exists operationally, tiers
  and scope stated; and the `free_slot` far-side divergence from the design
  note's leftmost convention **owner-RATIFIED far-side at the #187 PR2 review
  (2026-07-30)**, with the measured 87-vs-88 tie recorded as a convention
  choice, not a quality result)
  and §4.6 (the 253/1162
  figures are hereafter `nf`-display trackers: equality closed by content,
  display closed by PR1, the counts retained as engine drift detectors).
  `tests/smc_nf_differential_sweep.rs`'s module docs carry the same re-label.
  **No pin moves in any of it** — doc text and new assertions only.

- **`prop::presentation::content::is_arity_well_formed`** — the domain
  predicate for `content_of`, public so callers that must stay total on
  hand-built or deserialized `PropExpr` trees can ask before building a content
  rather than catching a panic they cannot catch.

- **Abstract content and content equality —
  `prop::presentation::content`** ([#57](https://github.com/sustia-llc/catgraph/issues/57),
  a1 PR1): the `SMC-NF-RECONCILIATION.md` §4.1 content function `C`, in tree.
  `content_of` computes the anchored cospan of Λ-typed directed hypergraphs
  (BGKSZ arXiv:1602.06771v2 `⟦·⟧`) by structural recursion with union-find
  gluing at `;`; `content_eq` decides isomorphism **under both feet**; and
  `canonical_key` produces a hashable form with `canonical_key(a) ==
  canonical_key(b)` iff `content_eq(a, b)`. By Lemma 4.1 (§4.2) this decides
  SMC-equality *exactly*, on every diagram — fragment or not — which is
  strictly more than `nf` equality: the new
  `tests/content_equality_corpus.rs` scores it at **253/253** divergent pairs
  on the published default corpus and **1 162/1 162** in braid mode
  (`internal-probes`-gated, both 100k sweeps `#[ignore]`d, 5k smoke tier and
  cross-corpus negative controls in CI). The decision is exact and free of
  search: anchors force node images pointwise, monogamy (BGKSZ Def 3.6) plus
  ordered tentacles propagate each forcing deterministically, and the closed
  components — which no anchor reaches, and which are coupled to nothing else —
  are settled by comparing a complete iso invariant rather than by matching
  them against one another. So there is no comparator, no component order, no
  writing-dependent coordinate, and no backtracking anywhere in it; the cost is
  linear in the content apart from the per-closed-component serialization.
  **Word-generic from day one:** nodes carry the color their generator tentacle
  *word* declares, and `content_of_colored` pins the one remaining kind (a wire
  no generator touches, which monogamy makes boundary-anchored) from a
  `ColoredExpr`'s source word, so the boundary words are readable off the
  content and content equality on that path decides colored SMC-equality,
  parallelism included. Layering is deliberate and tested: `C` quotients by SMC
  coherence and nothing else, so `Copy ; Add` and `Copy ; σ ; Add` are correctly
  **unequal** — cocommutativity is a Thm 5.60 *user* equation and stays with
  `eq_mod`'s congruence closure above this layer. Nothing is wired to `eq_mod`
  yet; `nf` is untouched. (PR2, above, wires it.)

### Fixed

- **`SMC-NF-RECONCILIATION.md` doc corrections** (2026-07-29): §4.1 and §4.7
  gain dated status notes — the content function is no longer a specification
  of something unbuilt, and §4.7 records the a1/a2 split with the equality half
  landed. §4.7 also carries a **correction of record**: an earlier #57
  knowledge-base report's claim that "Lafont proves termination for the
  bialgebra structure" is refuted against the cached anchor — Lafont states it
  as a *conjecture* for the bialgebra-bearing system and documents the
  obstruction (`ε : 1 → 0` admits no strictly monotone interpretation into
  `ℕ*⁰`); the nearest proof in the anchors is BGKSZ Thm 6.1, for the
  *non-commutative* bimonoid. The false claim never reached this document. The
  provenance header gains a recovery note: the original working note, recorded
  there as never committed and unrecoverable, was recovered 2026-07-29 in a
  private archive, and a fidelity diff found no correction to make to the
  reconstruction — it also established that the known Selinger "Thm 3.12 p. 17"
  slip originated in the working note, so the audit's p. 18 correction stands.

- **SMC NF rigidity/canonicality theorem v2 — Theorem 4.5 on the fragment
  `𝔉′`** ([#174](https://github.com/sustia-llc/catgraph/issues/174) PR-B):
  `SMC-NF-RECONCILIATION.md` §4.4 rewritten from a refuted-status ledger into
  a proven theorem with an honest scope. New content: Lemma 4.3 (*column
  pinning* — braid-free components of the identity-split refinement are
  exactly content components, so rule-(i) keys and the direction of every
  Step-6½/7 move are content-invariants at any loop point; the braid guard is
  thereby load-bearing, not hygiene), Lemma 4.4 (*layout freedom* — a
  braid-free invariant-satisfying diagram is determined by content plus
  `(λ, ι)`: layer assignment and `η` insertion slots), content-intrinsic
  `ldepth`/placement-slack definitions, and **Theorem 4.5**: rigidity on
  `𝔉′` = braid-free diagrams whose every `η` is layer- and slot-pinned, by
  top-down induction from the input foot (proof-sketch density; two steps
  stay **flagged open** — a first discharge attempt was itself refuted in
  the delta review and is recorded in the sketch, with the
  `layer_pinned_eta_sits_below_layer_zero` witness pinning the false
  premise). Canonicality-via-`nf` on
  `𝔉′` is **conditional** — the review refuted the unconditional corollary
  twice, with both counterexamples committed as witnesses: fixpoints can
  violate a non-excepted §1 clause through the `adjacent_column_cuts`
  right-column asymmetry
  (`cut_asymmetry_separates_smc_equal_writings_inside_f_prime`; engine
  defect, filed — its fix is the discharge path), and NF braid-freeness is
  per-writing, not content (`braid_prefix_is_not_content_derived`), so `𝔉′`
  reads `nf(e)` braid-free and both corollaries condition on it. The
  `η`-free special case is proved under the same braid-free conditioning
  (all three ordering passes provably inert; the 16 103-pair `η`-free
  sub-corpus diverges nowhere). **Withdrawn, with a three-generator
  witness:** rigidity
  on the original `𝔉` — a sweep characterization showed *all* 253 divergent
  pairs (128 in-`𝔉`) are one mechanism, `η` placement slack, which no pass
  canonicalizes; canonicalizing `ι` is now the single named gap, and the
  sharpest framing yet for
  [#57](https://github.com/sustia-llc/catgraph/issues/57) (a content-level
  engine never chooses `ι` at all). New witnesses: the `eta_slack_residual`
  module (documented divergences, flip-and-rename on any future fix) and
  `tests/pass_disjointness_probes.rs` (below); both wired into the
  `internal-probes` CI step.

- **SMC NF Step 6½ — zero-arity column transposition
  (`reorder_zero_arity_columns`)**
  ([#174](https://github.com/sustia-llc/catgraph/issues/174)): the move strictly
  between Step 6 (adjacent *atoms*) and Step 7 (whole *components*). Over the
  identity-split refinement, two adjacent **interval-aligned columns** transpose
  when their block arities strictly commute at the interval's own boundaries —
  `(src X = 0 ∨ src B = 0) ∧ (tgt X = 0 ∨ tgt B = 0)`, Step 6's criterion read at
  column granularity. Direction is rule (i)'s plain `CompKey`, the same core
  Step 7 reads, so the two *rewriting* passes cannot disagree with each other;
  the guards are Step 7's (no braid-carrying or marked component, at least one
  multi-atom), plus a carve leaving the closed↔closed tie to Step 7's reading
  key. Termination adds `column_inversion_count` to the lexicographic measure,
  between `block_inversion_count` and `tied_inversion_count`
  (`docs/SMC-NF-RECONCILIATION.md` §2.4, §4.5). Steps 7 and 6½ now share one
  identity-split refinement and one component analysis per fixpoint iteration
  rather than building the same pair twice.

- **`internal-probes` feature — SMC-NF test hooks + the differential-sweep
  tracker** ([#174](https://github.com/sustia-llc/catgraph/issues/174)):
  opt-in, test-only, **NOT public API** (not in `default`). Two hooks guard
  numbers in `docs/SMC-NF-RECONCILIATION.md` that were hand-measured and could
  therefore drift silently: `smc_nf::nf_without_column_pass` — the pipeline
  with Step 6½ skipped — pins the §4.5 ablation attribution via
  `smc_canonicality_probes::column_pass_ablation` (exactly five witnesses are
  pass-dependent; CE-A3 is not); and `smc_nf::fragment_status` — per-diagram
  marking/closedness against the fragment `𝔉` — feeds
  `tests/smc_nf_differential_sweep.rs`, the #174 design round's 100 000-pair
  corpus driver ported in-tree (same generator, rewritings and seed
  `0x9E37_79B9_7F4A_7C15`), whose `published_divergence_figures_reproduce`
  pins §4.6's calibration column (253 total / 128 in-`𝔉` / 23 marked) and
  whose `published_braid_mode_figures_reproduce` is the residual-(a) tracker
  (braid-injecting corpus, 1162/634/237 — a different corpus, not comparable
  to the calibration table). CI runs the 5 000-pair smoke tier plus a clippy
  pass under the feature; both 100 000-pair sweeps stay `#[ignore]`d and are
  re-run with `--ignored` when the normal form changes.

- **Worded complete-functor surface: `ColoredCompleteFunctor` +
  `Presentation::eq_mod_functorial_colored`**
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P3a):
  `CompleteFunctor::apply` takes a bare `PropExpr`, which is word-blind *by
  shape* — `Identity(n)`/`Braid(m, n)` carry only a width — so a colored
  decision functor cannot be expressed through it. `ColoredCompleteFunctor<G>`
  consumes a `ColoredExpr<G>` instead, and `eq_mod_functorial_colored` compares
  images **only after** checking that the two morphisms are parallel (equal
  source *and* target words): the trait asks a target to decide equality within
  a hom-set, not to separate hom-sets from each other, so image equality alone
  could over-identify. Differing boundary words therefore decide `Some(false)`
  without consulting the functor. The word-blind `CompleteFunctor` and every
  existing impl are unchanged; catgraph-syntax's `CospanFunctor` now implements
  both.
- **Λ-colored prop surface: `prop::colored` check-pass + `ColoredExpr`**
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P1): top-down
  word-flow validation `check(expr, input_word) -> Result<target_word>`
  (identities and braids are color-polymorphic — colors derive from the
  ambient word; braids emit the block swap; generators match their
  `source_word` by value), and `ColoredExpr<G>` — the colored morphism as the
  checked pair `(source_word, expr)` — with `eq_colored` = layered-NF
  equality + boundary-word equality (soundness via
  `SMC-NF-RECONCILIATION.md` §4.2/§4.3 Lemmas 4.1/4.2; canonicality caveat
  per §4.4 stated in the docs). Serde derives behind the existing opt-in
  `serde` feature with the #81-style documented trust boundary (Deserialize
  does not re-run `check`; re-validate by rebuilding via `ColoredExpr::new`).
- **SMC NF content framework + canonicality status —
  `SMC-NF-RECONCILIATION.md` §4**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) proof phase,
  2026-07-27): the diagram's **abstract content** (anchored monogamous
  directed acyclic cospan of Λ-typed hypergraphs — BGKSZ arXiv:1602.06771v2
  Def 3.6/3.9, Prop 3.4, Thm 3.12) is defined **color-generically** over an
  arbitrary color set `Λ` (so the
  [#79](https://github.com/sustia-llc/catgraph/issues/79) word-generalized
  engine inherits it), with two lemmas proven: content decides SMC-equality
  (Lemma 4.1), and `nf` preserves content — so NF-equality *implies*
  SMC-equality unconditionally (Lemma 4.2). The converse (full canonicality
  on the fragment `𝔉`) is **probe-verified but open**: the draft theorem was
  refuted in adversarial review (see Fixed below); §4.4 records exactly what
  is proven, verified, and open, and §4.5 the missing move (zero-arity-bounded
  *column* transposition) with both repair paths. §4.7 frames the content
  function as the [#57](https://github.com/sustia-llc/catgraph/issues/57) DPO
  substrate (BGKSZ §5 convex DPO, Thm 5.6; MPZ CALCO 2023 Def 7 / Thm 21 /
  Thm 28 for the commutative-(co)monoid refinement). The §1 invariant list
  gains three previously implicit clauses (intra-layer identity fusion, no
  pure-identity layer beside a non-identity layer, canonical braid runs).
- **`Checked<T>` poison-on-overflow rig wrapper**
  ([#88](https://github.com/sustia-llc/catgraph/issues/88)): `Checked::Value(T)
  | Checked::Poison` in `src/rig.rs`, satisfying the `Rig` blanket impl so
  `Checked<i64>` drops unchanged into `MatR::matmul`, `sfg_to_mat`,
  `MatrixNFFunctor`, and catgraph-syntax's `eval`/`SfgModel`. Overflow becomes
  the sentinel `⊥` and propagates to the result, where callers test it with
  `is_poisoned()`. `⊥` is **fully absorbing, including `⊥ × 0 = ⊥`** — the
  zero special-case would erase a detected overflow and break distributivity in
  ring extensions. That costs exactly one rig axiom (absorbing zero, only in
  the poisoned cone); associativity, commutativity and both distributive laws
  survive, so `verify_rig_axioms` passes on unpoisoned samples and fails with
  precisely `"absorbing zero"` on a poisoned one. `Display`/`FromStr` render
  and read `⊥` as a single lexical atom, keeping `scalar_⊥` a valid
  catgraph-syntax token. No serde derives (rig types deliberately carry none;
  the [#81](https://github.com/sustia-llc/catgraph/issues/81) serde surface is
  the term layer only).
- **`CheckedOps` trait** ([#88](https://github.com/sustia-llc/catgraph/issues/88)):
  `checked_add`/`checked_mul` returning `Option<Self>`, macro-implemented for
  the twelve primitive integer types by delegating to the std inherent methods.
  Catgraph-local rather than a `num-traits` import — only `Zero`/`One` are
  sourced externally.

### Changed

- **BREAKING: `Presentation::add_equation` checks boundary-*word* equality**
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P2): the two sides
  of an equation must now be parallel morphisms over a **common source word**,
  not merely have matching top-level arities. An equation declares no source
  word, so one is *inferred*: a variable-threaded sibling of P1's
  `prop::colored::check` runs both sides through the same fresh source
  variables (so a constraint found on either side propagates to the other) and
  unifies the two target words pairwise. Acceptance means such a shared word
  exists — the most general one under the inferred constraints — which keeps
  jointly-constrained polymorphic equations like `Identity(2) = Braid(1,1)`
  valid; the inferred constraint is not stored, and user-equation rewriting
  stays word-blind (no in-tree API rewrites a `ColoredExpr`). Two breaking
  consequences: the rejection error is now
  `CatgraphError::CompositionSizeMismatch` (lengths) or
  `CatgraphError::Composition` (colors) instead of
  `CatgraphError::Presentation`; and the check is **stronger even at
  `Color = ()`** — `PropExpr`'s variants are public, so a hand-built
  ill-composed tree (`Identity(1) ; (Identity(2) ; Identity(1))` reads `1 → 1`
  at the top while its inner `Identity(2)` is handed one wire) is now
  rejected. Terms built through `Free` are unaffected; every shipped
  presentation (Thm 5.60's `E_18`, catgraph-syntax's `E_frob`) is accepted
  unchanged. `Presentation`'s serde trust boundary (#81) is restated in
  word terms: `Deserialize` still does not re-run the check, so an untrusted
  document may now carry a word-ill-formed or color-mismatched equation.
- **BREAKING: `PropSignature` is Λ-colored and `Ord`-bounded**
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P1): the trait
  gains `type Color: Clone + Eq + Hash + Debug` and **required**
  `source_word`/`target_word` returning `Cow<'_, [Self::Color]>`;
  `source()`/`target()` are now **provided** (word length — existing impls
  keep their overrides, which **must** stay equal to the word lengths; a
  documented invariant, not runtime-enforced); supertraits
  gain `Ord`. Migration per impl: `type Color = ();`, two one-line word
  methods via the new `mono_word` helper (ZST-backed, never allocates), and
  `Ord` in the derive. `SfgGenerator<R>` requires `R: Ord`; the three
  f64 rig newtypes (`UnitInterval`, `Tropical`, `F64Rig`) gain **lawful**
  manual total orders — `-0.0`-normalized `f64::total_cmp` with `PartialOrd`
  re-derived from `Ord` (NOT `to_bits`, which inverts negatives against IEEE
  order). Note: the rig-module NaN non-reflexivity caveat is **unchanged**
  (it lives in the derived IEEE `PartialEq`); `Ord` orders NaN equal to
  itself — the narrow disagreement is documented in the module docs.
  `Checked<T>` gains derived `PartialOrd + Ord` (canonical sort key only).
- **SMC NF: residual (b) closed — closed↔closed blocks sort by content**
  ([#79](https://github.com/sustia-llc/catgraph/issues/79) P1,
  [#174](https://github.com/sustia-llc/catgraph/issues/174) residual 1):
  Step 7 breaks equal rule-(i) keys (two closed blocks) by the
  **lexicographic in-situ reading** of each block (layer-by-layer,
  left-to-right, kind tag / widths / generator by `Ord` — offset-independent,
  invariant under the pass's own swaps; equal readings = identical blocks);
  Step 6 sorts equal-class `0→0` scalars by generator order (vacuous for all
  shipped signatures — none has a `0→0` generator — so behaviorally inert);
  `component_slot` uses the same comparator. Witness un-ignored and renamed
  `closed_closed_order_is_ord_less_residual` →
  `closed_blocks_sort_by_content_key`; new probe family
  (`three_closed_blocks_converge_in_reading_key_order`,
  `tied_scalars_sort_by_generator_order`, …). The four CC baseline pins
  (979/1432/2017/2012) are **unmoved** — the d = 2 enumeration cannot express
  two distinct closed components, and no shipped signature has a `0→0`
  scalar. Residuals (a)/(c)/(d) unchanged (three `#[ignore]`d witnesses).

  > **Superseded within this release by the #174 design round (2026-07-28):**
  > `component_slot` is deleted, so "`component_slot` uses the same comparator"
  > no longer applies — no free decision site reads the component order. The
  > pins quoted here as unmoved were re-baselined to 980/1433/2018/2013 by the
  > same round.
- **SMC NF PR2: component-anchored η placement (rule (i)) + Step 7
  component-block reorder**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR2, design of
  record 2026-07-25 + proof-phase addendum 2026-07-26): the point-span η sift
  is re-cut at **connected-component granularity** — an η's insertion slot is
  derived from its component's boundary attachment (union-find over the wire
  arithmetic; fused identities pre-split pass-locally), never from the
  incidental sift-time cursor, eliminating the presentation-dependent anchor
  found in the PR2-STOP diagnosis (`A;B` vs `A⊗B` block transposition). New
  `reorder_component_blocks` pass (Step 7) orders wire-disjoint component
  blocks by class — **closed (0→0) < input-anchored < output-only** — each
  anchored class by least attached boundary coordinate (the block-level
  extension of Decision 1's scalars-leftmost). Interleaved output-only
  components are conservatively NOT sifted (documented residual, strictly
  narrower than the pre-PR2 gap); closed↔closed order is stable but `Ord`-less
  (residual). Both residuals tracked in
  [#174](https://github.com/sustia-llc/catgraph/issues/174).

  > **Superseded within this release by the #174 design round (2026-07-28):**
  > the slot derivation described here is **retired**. An `η`'s insertion slot is
  > no longer derived from its component's boundary attachment — it is the
  > leftmost slot its wire coordinate admits — because importing rule (i)'s
  > coordinates into a free choice made the normal form writing-dependent
  > (CE-R1). Guard 3 correspondingly no longer gates the sift, so the
  > "interleaved output-only components are not sifted" clause no longer holds
  > either; the guard survives in Steps 7 and 6½ only. What does survive from
  > this entry: `reorder_component_blocks` (Step 7) and rule (i)'s class order
  > as *its* comparator. The pins quoted in the sibling entries were
  > re-baselined to 980/1433/2018/2013 by the same round.
- **`smc_canonicality_probes` — the canonicality gate of record**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR2): new test
  module asserting SMC-equal pairs NF-equal **directly** (the diagnosis
  counterexample `Add;Discard` / `Zero;Copy`, full three-member families,
  the mid-layer η interchange pair, closed-block transpositions). This is the
  unconfoundable metric: the CC collision count conflates canonicality with
  bounded-depth E_18 equational reach, so NF changes are judged by the
  probes; the pins only catch unexplained deltas. Bonus: **scalar centrality
  is now an NF theorem** — the catgraph-syntax #80 CC-gap test migrated to a
  counit-law witness.
- **CC depth-2 collision baselines re-pinned (a deliberate rise)**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR2): BoolRig
  972 → **979**, UnitInterval 1400 → **1432**, Tropical 1930 → **2017**,
  F64Rig 1925 → **2012** (release, deterministic, exact two-sided pins).
  Witness-diff analysis (proof-phase diagnosis, 2026-07-26): the rise is
  **equational-reach churn** — depth-2 E_18 congruence bridges are co-adapted
  to the previous exact NFs, and no NF improvement can beat pins measured
  against them (mechanism tracked in
  [#173](https://github.com/sustia-llc/catgraph/issues/173)). The old "a rise
  is a STOP" gate is retired (owner call A′,
  2026-07-27); `scripts/check_audit_counts.py` now scans the prose pin sites
  against the `BASELINE_*_D2` consts so a re-pin can never miss a doc site.
  Must-sync sites updated: `tests/graphical_linalg.rs` consts + docstring,
  FS18-AUDIT Thm 5.60 row + §15 resolution note, README `kb` row,
  `functor_bench` docs, `mat_operations` / `prop_presentation_nf` examples.
- **SMC NF Step 6: within-layer η-before-ε canonical reorder**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR1, design of
  record 2026-07-25): new `reorder_tied_zero_arity` pass in `smc_nf::nf`'s
  fixpoint — adjacent atoms that commute **strictly** (`(src A = 0 ∨ src B = 0)
  ∧ (tgt A = 0 ∨ tgt B = 0)`, both connecting braids `σ_{0,n} = id`) are
  bubble-sorted to the canonical class order `scalar (0→0) < η (0→n) <
  ε (n→0) < solid` — the greedy (lex-least) normal form of the layer's word in
  the trace monoid. Closes the zero-arity **tensor-order** split
  (`nf(ε ⊗ η) == nf(η ⊗ ε)`); the layer-assignment half (`ε ; η` compose-forms)
  remains for PR2's sift rebase. Termination measure gains a trailing
  `tied_inversion_count` component (all-pairs class inversions;
  `SMC-NF-RECONCILIATION.md` §2.4 + new §2.5). Scalar order is stable (no
  shipped signature has a `0 → 0` generator; a total order awaits one).
- **CC depth-2 collision baselines re-pinned after the drop**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) PR1): BoolRig
  1142 → **972**, UnitInterval 1634 → **1400**, Tropical 2234 → **1930**,
  F64Rig 2229 → **1925** (release, deterministic, exact two-sided pins). The
  drop is confirmed KB-like progress, not CC over-merge: every Step-6 swap is
  an on-the-nose SMC identity (both connecting braids degenerate), and the
  distinct-morphism regressions (`η ⊗ η′ ≠ η′ ⊗ η`) pin the non-merges.
  Must-sync sites updated: `tests/graphical_linalg.rs` consts + docstring,
  FS18-AUDIT Thm 5.60 row + resolution note, README `kb` row,
  `functor_bench` docs, `mat_operations` / `prop_presentation_nf` examples.

### Fixed

- **Pass-disjointness obligation resolved as FALSE-as-stated; §1 invariant
  clauses restated with the *both-readings* carve**
  ([#174](https://github.com/sustia-llc/catgraph/issues/174) PR-B):
  adversarial probes (`tests/pass_disjointness_probes.rs`, three shapes, two
  with shipped generators inside `𝔉`) construct adjacencies of
  strictly-commuting atoms whose components also pass a rewriting pass's
  guards, with the class order and the component order opposed — at which the
  old §1 transposition clauses were violated at real `nf` fixpoints (and
  jointly unsatisfiable). The engine cycles inside each pass and exits on the
  whole-pass `sd == prev` check by exact cancellation, always landing
  Step-6-sorted; the restated clauses ratify exactly that (class order wins;
  no engine change, no pin movement). §2.4's per-step non-increase claim is
  corrected accordingly — the measure is the termination proof everywhere
  except these adjacencies, where completing it (or gating the passes, the
  engine-side alternative) is tracked on #174. No divergence or
  non-termination is attributable to these conflicts (the 1 502 adversarial
  writings all terminate *and converge*; the 100 000-pair sweep terminates —
  its 253 divergences are `η` placement slack, a different mechanism). Also
  corrected in the same sweep:
  §4.4's "marking is content-level" narrowed to braid-free diagrams
  (`braid_coarsening_marks_content_clear_diagram`); §4.5 Path 1's sufficiency
  hardened from "unproven" to **refuted** (12 of the 128 satisfy it and
  diverge); §4.6's case-7079 exemplar retracted (it converges on the shipped
  engine — its divergence was the review-round engine's, closed by the
  free-site retirement); `eq_colored`/`smc_nf` rustdoc canonicality claims
  re-scoped to `𝔉′`.

- **SMC NF residuals (c) and (d) closed**
  ([#174](https://github.com/sustia-llc/catgraph/issues/174)) — the two nesting
  residuals of `docs/SMC-NF-RECONCILIATION.md` §4.6, by two different means. A
  **closed** block written strictly inside another component's wire span now
  extracts to its free writing (residual (c)), and so does a nested **sink**
  block solid on the side facing its encloser's opening — both by the Step 6½
  column pass. The **source** form of (d), CE-A3, turned out *not* to be a column
  residual at all: it was blocked by Step 6's tied comparator ranking components
  ahead of the class order, and it converges once that branch is retired
  (below). Ablating Step 6½ leaves CE-A3 converging and re-breaks the other five
  column witnesses — the attribution is measured, not assumed. All three formerly
  `#[ignore]`d witnesses in `tests/smc_nf_completeness.rs` are live regressions,
  renamed to describe behaviour (`trapped_closed_block_extracts`,
  `nested_sink_block_converges_with_free_writing`,
  `nested_source_block_converges_with_free_writing`) per the residual-(b)
  precedent.
- **Free decision sites no longer read component order**
  ([#174](https://github.com/sustia-llc/catgraph/issues/174)) — the design
  round's central change. `component_slot` (the Step 4(c) `η` slot walk) is
  **deleted**, and `tie_sorts_before`'s rule-(i) branch with it: the `η` sift
  takes the leftmost slot its coordinate admits, and a tied adjacency is decided
  by the Decision-1 class order and `G::cmp` alone. Rule (i)'s boundary
  coordinates now live only in the two *rewriting* passes, Steps 7 and 6½, which
  verify braid-freedom at the boundaries they span before moving anything.
  Rationale: a free choice is not pinned by the diagram, so importing
  writing-dependent coordinates into it makes the normal form
  writing-dependent — witnessed by **CE-R1**, an SMC-equal pair inside `𝔉` that
  the imported order separated. Guard 3 correspondingly stops gating the sift and
  the tied comparator (it survives in Steps 7 and 6½): a marked component's `η`
  now sifts, and marked-case divergences on the differential corpus fell 888 →
  23. In-`𝔉` divergences fell 192 → 128 and total divergences 1311 → 253 over the
  same 100 000-pair corpus.
  CE-R1 and CE-R2 are committed as regression witnesses.
- **Depth-2 collision pins re-baselined `+1` on every rig** — BoolRig 979 →
  **980**, UnitInterval 1432 → **1433**, Tropical 2017 → **2018**, F64Rig 2012 →
  **2013** (release, deterministic, exact two-sided pins). One new collision
  pair, none lost, attributable to the tie-branch retirement rather than to the
  column pass — the same act that closes CE-A3, so the collision and the closure
  are inseparable. The witness pair and the full lineage are in the
  `tests/graphical_linalg.rs` module docstring; must-sync sites updated
  (`BASELINE_*_D2`, docstring table, per-test comments, FS18-AUDIT §15 + the Thm
  5.60 row, `functor_bench` docs, applied README, both examples).
- **Fragment claims corrected: two new NF residuals (four total)**
  ([#55](https://github.com/sustia-llc/catgraph/issues/55) /
  [#174](https://github.com/sustia-llc/catgraph/issues/174)). Residual (c),
  found in drafting: a closed (0→0) component written strictly *inside*
  another component's wire span does not extract (its η's gap-closer is
  foreign; Step 7 never sees an adjacent free pair) — SMC-equal nested and
  free writings reach different fixpoints while sharing identical content, so
  the residual is irreducibly presentation-level. Residual (d), found in
  adversarial review and **refuting the draft §4 theorem**: a multi-atom
  zero-arity block, solid on the side facing its enclosing wall's opening
  (solid-headed sink / solid-tailed source), written nested converges with
  none of its free writings even when boundary-attached and unmarked — i.e. *inside* the
  fragment `𝔉` (probe-verified CE-A family; the draft enclosure lemma missed
  the wall-opening-at-η/ε escape). Canonicality claims re-scoped from "the
  non-interleaved fragment" to probe-verified-on-`𝔉` in the reconciliation
  doc, `smc_nf` module docs, and both test-suite docstrings; three new
  `#[ignore]`d witnesses (`trapped_closed_block_is_nesting_residual`,
  `nested_sink_block_is_column_residual`,
  `nested_source_block_is_column_residual`). The depth-2 collision pins are
  unaffected (both traps need expression depth ≥ 3, structurally outside the
  d = 2 enumeration).

  > **Superseded within this release by the #174 design round (2026-07-28):**
  > residuals (c) and (d) are both closed — (c) and (d)'s sink form by the Step
  > 6½ column pass, (d)'s source form by the free-site retirement — and the
  > three witnesses named above are un-ignored and renamed
  > (`trapped_closed_block_extracts`,
  > `nested_sink_block_converges_with_free_writing`,
  > `nested_source_block_converges_with_free_writing`). The framing "four
  > residuals (total)" is also withdrawn: §4.6 is a ledger of *named* residuals,
  > not a bound, and a differential sweep finds in-`𝔉` divergences outside all
  > four letters on every build the project has shipped. The pins named here as
  > unaffected did move `+1` — for the retirement, not for the column pass.

### Documentation

- **`rig` module docs gain the workspace overflow policy of record**
  ([#88](https://github.com/sustia-llc/catgraph/issues/88)): a per-rig-family
  matrix (Boolean/`[0,1]` cannot overflow; `Tropical`/`F64Rig` keep IEEE
  `inf`/NaN semantics; `Z(BigInt)` is exact; primitive integers inherit Rust's
  debug-panic / release-wrap; `Checked<T>` is the opt-in detection story) plus
  the recorded rejection of saturating arithmetic (silently wrong values and
  broken distributivity — not offered even as an opt-in). The policy lives
  once, here; downstream crates cite it. `README.md` carries the same note as a
  design entry.

## [workspace-v0.4.0] - 2026-07-25

### Changed

- **E1 test RNG seeds standardized to the bench convention**
  ([#141](https://github.com/sustia-llc/catgraph/issues/141) follow-up, PR #146):
  the inline seed literals in `src/e1_operad.rs`'s test module and
  `tests/operad_boundary.rs` are now documented file/module-level
  `const SEED: u64` values with `SEED + 1` offsets for independent streams
  (matching `benches/mat_ops_bench.rs`). Streams are byte-identical or
  provably unused; test behavior unchanged.
- **`LinearCombination::linear_combine` gains par-vs-seq equivalence coverage**
  ([#48](https://github.com/sustia-llc/catgraph/issues/48)): `linear_combine` is
  a second, independent `rayon_cond::CondIterator` dispatch point (it duplicates
  the `Mul::mul` dispatch rather than delegating to it) and previously had no
  parallel-vs-sequential test — its functional tests ran far below
  `PARALLEL_MUL_THRESHOLD` (32). `tests/rayon_equivalence.rs` now compares
  `linear_combine` against an independent nested-loop sequential reference at
  sizes straddling the threshold (both operands ≥ 32 to reach the parallel arm),
  including a non-injective combiner that forces coefficient collisions.
- **Shared `adjacent_swaps` bubble-sort core extracted**
  ([#138](https://github.com/sustia-llc/catgraph/issues/138)): the adjacent-
  transposition decomposition duplicated in `mat_to_sfg`'s `permutation_sfg`
  and `prop::presentation::smc_nf`'s `decompose_braid` **and
  `canonicalize_run`** (a third copy surfaced in review) now lives in one
  `pub(crate)` helper (`crate::prop::adjacent_swaps`); each call site maps the
  returned swap list into its own representation (SFG braid layers in swap
  order vs. reversed `Layer<G>` values — the smc_nf perms are output-indexed,
  so the sort word undoes the braid and its reversal rebuilds it).
  Behavior-preserving — all call sites' existing tests pass unchanged.
- **`functor_bench` cc groups re-budgeted from unmeasured design-doc estimates**
  ([#59](https://github.com/sustia-llc/catgraph/issues/59)): the
  `cc_incompleteness_count::bool/3` bench was dropped — one `d=3` verifier call
  exceeds 590 s in release, un-runnable under any criterion config; depth-3/4
  ground truth stays on the `#[ignore]`'d `cc_completeness_tracking_*_depth_{3,4}`
  trackers in `tests/graphical_linalg.rs`. Both remaining cc groups
  (`bool/2`, `f64rig/2`) now run at criterion's `sample_size(10)` minimum with
  measured wall times documented in the bench module rustdoc (per-call ≈ 6.9 s
  bool / ≈ 6.7 s f64rig; ≈ 2 min 31 s for both groups together), replacing the
  never-profiled "60 s budget" and retiring the criterion-defaults config whose
  ~13-min cost had already been measured as ground truth.
- **All Joyal–Street SMC-NF anchors verified; every (†)/(‡) mark retired**
  ([#117](https://github.com/sustia-llc/catgraph/issues/117) option (b),
  completes the issue): with the JS-I and JS-Braided journal scans now in
  the private cache, all 9 JS-I and all 6 JS-Braided page/theorem locators
  in `docs/SMC-NF-RECONCILIATION.md` were verified from page images —
  **every one exact as written**; no code or test citation needed changes.
  One quirk documented: JS-I prints two theorems headed "Theorem 1.2"
  (p. 66 planar-deformation — the one Selinger cites; p. 71 𝔽(𝒟)-freeness —
  the one catgraph cites, whose heading is a misprint per the paper's own
  p. 81 cross-references to "Theorem 1.3"). The interim (†)
  cache-unverifiable and (‡ Sel / ‡ MMR86) cross-check scaffolding is
  removed; the header now records the full provenance trail.
- **JS-Braided precursor report cached; `(‡ MMR86 …)` cross-checks added**
  ([#117](https://github.com/sustia-llc/catgraph/issues/117)): Ross
  Street's publication list designates the author-hosted scan of
  *Braided monoidal categories* (Macquarie Math. Reports 860081, 1986)
  as the earlier version of the 1993 Adv. Math. paper; it is now in the
  private papers cache (`js-braided-860081.pdf`/`.txt`).
  `docs/SMC-NF-RECONCILIATION.md` gains content cross-checks against
  it: condition **S** verbatim (`c_BA c_AB = 1_{A⊗B}`, pp. i/2), axiom
  **B2** = exactly the `c_{A⊗B,C}` decomposition the NF uses (B1 noted
  as its mirror via `c⁻¹`, p. 2), Yang-Baxter as braid-group relation
  **BG1** (p. 5), and **Theorem 4** freeness (`𝔹` free braided on one
  object, p. 17). The 1993 page/theorem locators keep their (†) — the
  report's numbering and pagination differ, and some 1993 content
  (§6 "balanced") does not exist in it, so Elsevier access (option (b))
  is still the only full-verification path.
- **Selinger + JS-II SMC-NF anchors verified and de-daggered**
  ([#117](https://github.com/sustia-llc/catgraph/issues/117) step 2):
  every Selinger (arXiv:0908.3347) and JS-II anchor in
  `docs/SMC-NF-RECONCILIATION.md` re-checked against the private papers
  cache and its (†) removed. One correction: symmetric-coherence
  **Thm 3.12 sits on p. 18, not p. 17** (p. 17 is §3.5's self-inverse
  symmetry definition) — fixed in the doc and in
  `tests/smc_nf_regression.rs`. JS-I / JS-Braided anchors keep (†)
  pending Elsevier access (#117 option (b)); statements Selinger's
  survey restates now carry a `(‡ Sel …)` cross-check mark
  (#117 option (a)), including Selinger's own attributions
  `[22, Thm. 1.2]` / `[22, Thm. 2.3]` corroborating the JS-I theorem
  numbers.
- **Selinger 2011 citation now carries its arXiv id** ([#117](https://github.com/sustia-llc/catgraph/issues/117)
  step 1): `docs/SMC-NF-RECONCILIATION.md` cites the survey as
  arXiv:0908.3347 so the private papers-cache `fetch-papers.sh` (arXiv-id
  auto-discovery) can fetch it, making the ~12 Selinger-anchored SMC-NF
  lines cache-verifiable.
- **`docs/FS18-AUDIT.md` completeness rows added (owner decision, audit
  Phase 7)** — §5.2 gains the two previously untracked prop examples:
  **Ex 5.7** (the prop Corel — 🔗 IN CORE, `catgraph::corel::Corel<Lambda>`
  carries the listed identity/symmetry/composition/monoidal structure) and
  **Ex 5.8** (the prop Rel — 🔗 IN CORE, `catgraph::span::Rel<Lambda>`,
  already mapped to this example in the cross-paper table). Summary
  `[27,3,3,12,16] of 61 → [27,3,3,12,18] of 63` (implementable count
  unchanged at 33); count-guard green.

- **Thm 5.60 presentation completed to E_18** ([#114](https://github.com/sustia-llc/catgraph/issues/114)):
  `matr_presentation<R>` now builds all **18** equation schemas of F&S Thm 5.60
  (p.170) / BE15 Theorem 2 relations (1)–(18), up from 16. The two missing rig-
  structure relations were added: **D7** scalar addition `Δ ; (r_a ⊗ r_b) ; μ =
  r_{a+b}` (BE15 (12), iterated over `rig_samples` pairs like D1) and **D8** zero
  scalar `r_0 = ε ; η` (BE15 (14), emitted once like D2). The presentation is
  renamed `E_17 → E_18` workspace-wide (the old `17` count matched neither paper;
  the figure and BE15 both have 18). Completing the presentation gives the CC
  engine more equations to identify with, so the pinned `cc_completeness_tracking_*`
  depth-2 collision baselines dropped: BoolRig 1301 → **1142**, UnitInterval
  1856 → **1634**, Tropical 2526 → **2234**, F64Rig jitter band `2770..=2790` →
  **`2468..=2488`** (observed 2478–2480); the `prop_presentation_nf` example's
  BoolRig expansion count 23 → **28**. All four `thm_5_60_soundness_*` tests
  confirm every new equation is a matrix equality under `S = sfg_to_mat`.
- **FS18-AUDIT summary recount** (paper-audit Phase 2): the summary table had
  drifted from its own detail tables since before the earliest tracked commit —
  §5.2 7→8 rows (Def 5.13 ⚠️ was uncounted), §5.3 6→7 (Prop 5.56 ❌ added in the
  2026-07-13 reconciliation was never summed), §6.3 10→9; TOTAL 26/2/2/15/15 of
  60 → **27/3/3/12/16 of 61**, implementable 30→33, headline 87%→82% DONE. The
  released [0.3.1]/[0.2.0] "56 items" entries below record the audit's size at
  those dates and are left untouched. Citation fixes in the same pass: F&S 2019
  "§2.6" → §2.3 (mat_kron) / §3.1 (trace), BTV 2021 "§1.4" → §5
  (lawvere_metric), §4.5 page range, `Ring + ZAlgebra` bound in z/integer docs.

### Fixed

- **`E1::random` now guarantees a valid configuration**
  ([#141](https://github.com/sustia-llc/catgraph/issues/141)): the generator drew
  `2·arity` uniform samples, sorted them, and paired consecutive values without
  ensuring adjacent coordinates were separated, so a draw could produce a
  zero-width or sub-epsilon interval (observed width ≈ 1e-7) that `E1::new`
  rejects — the terminal `.unwrap()` then panicked. `random` now resamples the
  whole batch until every adjacent pair of sorted coordinates is separated by more
  than `2·F32_EPSILON`, making construction infallible (`.unwrap()` → `.expect`
  documenting the invariant). The signature is generalized from
  `&mut ThreadRng` to `&mut impl RngExt` (API-affecting; callers passing a
  `ThreadRng` are unaffected).

### Added

- **`mat_to_sfg` — FS18 Prop 5.56 realization** ([#126](https://github.com/sustia-llc/catgraph/issues/126)):
  the constructive converse of the shipped `sfg_to_mat` functor (Thm 5.53). For an
  `m × n` matrix `M`, `mat_to_sfg(M)` builds the Prop 5.56 / Exercise 5.59 four-layer
  composite — copy/discard (`copy_n`) ; scalars ; swaps/identities (a bubble-sort
  braid network) ; add/zero (`add_n`) — so that exactly one path from input `i` to
  output `j` carries the single scalar icon `M(i, j)`. The characteristic property
  `sfg_to_mat(mat_to_sfg(M)) == M` is verified in `tests/mat_to_sfg_roundtrip.rs`
  (Eq 5.57 2×2 template, Exercise 5.58's three matrices, empty-dimension edge cases,
  and a round-trip proptest over all four rigs). Every empty dimension degenerates
  naturally through the general composite — no shape is special-cased. Closes the last
  §5.3 coverage gap; FS18-AUDIT §5.3 `6/1 → 7/0` DONE/MISSING (TOTAL 27/3/3 → 28/3/2).
- **`add_n` / `zero_n` SFG helpers** ([#126](https://github.com/sustia-llc/catgraph/issues/126)):
  the additive duals of `copy_n` / `discard_n`. `add_n(m) : m → 1` sums `m` inputs
  (`add_n(0) = zero`, `add_n(1) = id`, `add_n(m) = (id ⊗ add_n(m-1)) ; add`);
  `zero_n(n) : 0 → n` emits `n` additive identities (`zero_n(n) = zero ⊗ zero_n(n-1)`).
- **Optional `serde` feature** ([#81](https://github.com/sustia-llc/catgraph/issues/81)):
  `Serialize`/`Deserialize` derives on the term-persistence surface —
  `PropExpr<G>`, `Presentation<G>`, `PresentedProp<G>`, `NormalizeEngine`,
  `NormalizeResult<G>`, and `SfgGenerator<R>` (generic over the payload's own
  serde impls). **Off by default**; the default build stays dependency-identical
  (serde is opt-in via `--features serde`). Terms are the machine-persistence
  representation of morphisms, feeding the realtime/persistence tracks
  (#72/#73). `Presentation` deserialization trusts the arity invariant (does not
  re-run `add_equation`'s check — documented on the type). CI gains a targeted
  `--features serde` test+clippy pass. (Note: the issue named a `StringDiagram<G>`
  type; the actual SMC normal form is `smc_nf::n<G>`, a *derived* form — persist
  the `PropExpr` term and re-normalize on load, so it is intentionally not
  serialized.)

### Added

- **SMC normal form: `topological_layer_order` (Step 4(c))** — sifts each
  non-identity-source generator to its earliest admissible (braid-free) layer,
  giving independent parallel work a single canonical schedule (issue #14; JS-I
  Ch 1 §4 Thm 1.2 p.71).
- **Mixed-layer braid isolation** in `collect_braid_prefix` — a `Braid`
  co-resident with an unrelated generator (`[σ, F]`) is factored by
  bifunctoriality into a braid-only layer + a generator layer, freeing the braid
  for the naturality sweep.
- **Identity-width-refined naturality sweep** — a braid can now slide past a wide
  `Identity(n>1)` or a pure-identity cover, not only two width-1 atoms.

### Fixed

- **Signed-zero `Eq`/`Hash` contract violation on the f64-wrapping rigs**
  ([#58](https://github.com/sustia-llc/catgraph/issues/58)): `UnitInterval`,
  `Tropical`, and `F64Rig` derive an IEEE `PartialEq` (under which `0.0 == -0.0`)
  but hashed via `to_bits()` (under which `0.0` and `-0.0` differ), breaking the
  `a == b ⇒ hash(a) == hash(b)` contract required of their use as
  congruence-closure `HashMap` keys — equal keys could land in different buckets
  and split a congruence class. Their `Hash` impls now normalize `-0.0` to `0.0`
  so hashing agrees with the derived `PartialEq`; all other values (incl. NaN
  payloads) keep bit-exact hashing. As a result the F64Rig depth-2 CC diagnostic
  is now deterministic and was re-baselined from the jitter band `2468..=2488` to
  an exact pin of `2229`.
- **`try_unitor_merge` source-left ordering** — the `L1 ; (X ⊗ id_k)` case with a
  zero-source `X` (e.g. `η : 0 → 1`) now PREPENDS `X` (was appended), so
  SMC-distinct morphisms like `σ;(η⊗id₂)` vs `σ;(id₂⊗η)` no longer collide.
- **Wide-braid expansion in identity-padded layers** — `hexagon_expand` now
  decomposes any `Braid(m,n)` (`m+n>2`) sitting in an identity-padded layer (as
  emitted by the naturality sweep and by isolation), restoring the "no wide
  braid" invariant.
- **Mixed-layer re-creation** — the braid+generator merge guard moved to
  `reduce_involution`'s merge site so neither `try_column_merge` nor
  `try_unitor_merge` can re-trap a braid alongside an independent generator.
- **#14 interchange proptest un-ignored and now gating.** One narrow follow-up
  remains: mid-layer zero-source (η) scheduling (ignored known-gap test).

### Changed

- **#15 resolved — functorial-terminal.** `Presentation::eq_mod_functorial`
  with `MatrixNFFunctor` is declared the terminal, complete decision procedure
  for Mat(R) (F&S Thm 5.53 / Baez-Erbele 2015); the syntactic congruence-closure
  engine is incomplete **by design**. Knuth-Bendix completion is demoted to a
  time-boxed feasibility spike (#57), relevant only for a future non-Mat(R)
  presentation lacking a semantic functor. The `cc_completeness_tracking_*`
  depth-2 diagnostics are re-baselined as **regression trackers** at the post-#14
  NF collision counts: BoolRig 1301, UnitInterval 1856, Tropical 2526 **pinned
  exactly** (two-sided `assert_eq!`, so a silent drop — KB-like progress or an
  unsound CC over-merge — is noticed too); F64 ~2777 is float-nondeterministic
  and tracked as an inclusive **jitter band** `2770..=2790` (#58). Downstream #15
  references swept accordingly; the `functor_bench` wall-time budgets were found
  unrealistic (measured, not estimated) and are tracked in #59.

> **Reconciliation note
> ([#158](https://github.com/sustia-llc/catgraph/issues/158)).** Workspace tags
> `v0.1.1`, `v0.2.0`, `v0.2.1`, and `v0.3.0` (2026-07-02 → 2026-07-11) were cut
> without per-crate sections here; this crate's changes across them are recorded
> only in git history (`git log v0.1.0..v0.3.0 -- catgraph-applied/`) and the
> workspace-level release record. Backfill was deferred out of the v0.4.0
> release (owner, 2026-07-25) and resolved as this note (#158, option 2).

## [workspace-v0.1.0] - 2026-07-01

First monorepo release: workspace-wide tag `v0.1.0` (supersedes the pre-reboot
crate-scoped version lineage below). The coalition semantic-layer handoff to
downstream koalisi.

### Added

- **`hypergraph` module — a CRUD hypergraph container** (`Hypergraph<V, HE>`,
  #23). The zero-dependency replacement for the yamafaktory `hypergraph` crate
  (v4.2.0) that the downstream **koalisi** coalition layer (sustia-llc/koalisi#4)
  re-backs its `TemporalHypergraph` on — catgraph has hypergraph *categories*
  (`Cospan`/`HypergraphCategory`), not an n-ary hyperedge data structure, so this
  supplies the operations koalisi calls, **with adapted signatures** where the
  K1 re-back improves on yamafaktory (add/get/update/remove vertex + hyperedge,
  `reverse_hyperedge`, `join_hyperedges`, `contract_hyperedge_vertices`, counts,
  **infallible** clears — koalisi's `clear_hyperedges()?` drops the `?` — plus a
  borrowing `hyperedge_vertices` and sorted iteration accessors). Plain
  `Vec`/`HashMap` + monotonic counters, **zero new dependencies**, coalition
  scale.
  - **Three load-bearing semantics:** stable, **never-reused** monotonic indices
    (`VertexIndex`/`HyperedgeIndex` — koalisi's event log replays raw indices,
    even across `clear`); hyperedges are **ordered** `Vec<VertexIndex>` with
    duplicate vertices allowed; `Copy` weights returned **by value**.
  - **Deliberate divergences from yamafaktory v4.2.0:** no-op updates (unchanged
    vertex/hyperedge weight or unchanged member list) return `Ok` instead of
    erroring — this makes `CoalitionManager::try_join_coalition`'s documented
    re-join idempotency ("idempotent if `agent` is already a member") true;
    infallible clears; generic bounds **relaxed** to `Copy + Eq + Debug` (no
    `Display`/`Into<usize>`/`Hash`); **no serde** (consumer wraps its own).
  - `add_hyperedge` is **idempotent** on an identical `(ordered vertices,
    weight)` pair, returning the **smallest** matching index (deterministic even
    after a `remove_vertex` cascade collapses two edges to the same key);
    `remove_vertex` **cascades** (sole-vertex edges removed, multi-vertex edges
    filtered); `contract_hyperedge_vertices` replaces occurrences then collapses
    **adjacent** `target` runs (empty to-contract set = true no-op);
    `join_hyperedges` keeps the **first** edge's weight and discards tail weights
    (matches yamafaktory exactly).
  - **Categorical view** `hyperedge_as_cospan(idx)` reads a hyperedge as the
    **identity cospan over its member index list** (`Cospan<VertexIndex>`,
    middle = member identities, not weights). Under the `WeightedCospan`
    implied-edge reading its edges are all `(i, j)` member pairs — the coupling
    slots the magnitude layer fills — so it is a handle for cospan-level
    composition *within applied*, **not** a shortcut into the magnitude layer.
    The real consumer path is `get_hyperedge_vertices` → koalisi maps
    capabilities→couplings → `coalition_value` (dedup members first — the
    magnitude layer rejects duplicates). Re-exported at the crate root:
    `catgraph_applied::{Hypergraph, HypergraphError, VertexIndex, HyperedgeIndex}`.
- `examples/agent_hypergraph.rs` (#23) — a worked agent-coalition registry over
  the K1 `Hypergraph`: the full coalition lifecycle (member read, join with the
  no-op re-join divergence, leave, merge, dissolve, agent-removal cascade, index
  stability) plus the `hyperedge_as_cospan` categorical view. Self-asserting;
  catgraph-applied-only (does not depend on catgraph-magnitude).

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

[Unreleased]: https://github.com/sustia-llc/catgraph/compare/v0.13.0...HEAD
[workspace-v0.13.0]: https://github.com/sustia-llc/catgraph/compare/v0.12.0...v0.13.0
[workspace-v0.12.0]: https://github.com/sustia-llc/catgraph/compare/v0.11.0...v0.12.0
[workspace-v0.11.0]: https://github.com/sustia-llc/catgraph/compare/v0.10.0...v0.11.0
[workspace-v0.10.0]: https://github.com/sustia-llc/catgraph/compare/v0.9.0...v0.10.0
[workspace-v0.9.0]: https://github.com/sustia-llc/catgraph/compare/v0.8.0...v0.9.0
[workspace-v0.8.0]: https://github.com/sustia-llc/catgraph/compare/v0.7.0...v0.8.0
[workspace-v0.6.0]: https://github.com/sustia-llc/catgraph/compare/v0.5.0...v0.6.0
[workspace-v0.5.0]: https://github.com/sustia-llc/catgraph/compare/v0.4.0...v0.5.0
[workspace-v0.4.0]: https://github.com/sustia-llc/catgraph/compare/v0.1.0...v0.4.0
[workspace-v0.1.0]: https://github.com/sustia-llc/catgraph/releases/tag/v0.1.0
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
