//! Canonical display scored against the published differential-sweep corpora
//! ([#187](https://github.com/sustia-llc/catgraph/issues/187), PR1).
//!
//! # What this is
//!
//! Four claims about `prop::presentation::display`, all measured on the same
//! frozen corpora `tests/smc_nf_differential_sweep.rs` publishes (183
//! divergences on the default corpus, 1 153 with braids injected) and
//! `tests/content_equality_corpus.rs` scores content against. The corpus is
//! copied rather than re-derived, exactly as those files document: same seed,
//! same generator, same rewritings, so case indices line up with their pins.
//!
//! 1. **The proof obligation** — `expr_of_content` is a section of
//!    `content_of`: `content_eq(content_of(expr_of_content(C)), C)` on every
//!    content the corpus produces, both modes, both sides of every pair. This
//!    is what makes the display *the same morphism*, and so what makes gate 3
//!    below a statement about a theorem rather than about two unrelated
//!    diagrams.
//! 2. **Display convergence** — for a corpus pair `(A, B)`, which is SMC-equal
//!    by construction, `canonical_display(A) == canonical_display(B)`. Zero
//!    failures *by construction* (the readback is a function of the content's
//!    iso class, and content decides SMC-equality — Lemma 4.1); the lane pins
//!    the by-construction claim rather than discovering it.
//! 3. **The `𝔉′` agreement probe** — `nf(e) == canonical_display(e)` where
//!    Theorem 4.5 (`docs/SMC-NF-RECONCILIATION.md` §4.4) says it must hold.
//!    A disagreement inside the theorem's hypotheses is an invariant violation
//!    or a counterexample; see the classifier note below for exactly which
//!    hypotheses each tier checks and which it assumes.
//! 4. **Crossing insertion and churn** — quality, not correctness: how often
//!    the readback introduces a `Braid` the normal form does not have (`nf`
//!    cannot delete a dead braid, so such a crossing is permanent), and how
//!    often the display differs from `nf` at all.
//!
//! # The gate-3 classifier, and its scope
//!
//! "No slack" is content-level (§4.4): every source-0 hyperedge layer-pinned
//! *and* slot-pinned. Slot-pinnedness is **realization-quantified** — it ranges
//! over the braid-free invariant-satisfying diagrams with that content — so it
//! is not directly computable, and the two tiers below are deliberately
//! conservative about it.
//!
//! - **Tier 1, `η`-free.** No source-0 hyperedge at all, and both diagrams
//!   braid-free. This is §4.4's `η`-free corollary *exactly*: its hypotheses
//!   are all checked, nothing is assumed, and it predicts equality. A
//!   disagreement here is a hard find.
//! - **Tier 2, layer-pinned.** At least one source-0 hyperedge; every one of
//!   them is `0 → 1` with `ceil = 1` (so layer-pinned by §4.4's definition,
//!   which is computed here from the content); both diagrams braid-free.
//!   **Slot-pinnedness is assumed, not checked** — that is the tier's whole
//!   caveat, and a disagreement here is only a find once the case is shown to
//!   be genuinely slot-pinned.
//! - **Tier 2, narrowed to connected contents.** The same, restricted to
//!   single-component contents, where block and column freedom — themselves
//!   `ι` by Lemma 4.4 — cannot be the explanation.
//!
//! Cases outside both tiers are counted but not scored: the theorem says
//! nothing about them, and `nf` is known to separate SMC-equal writings there.
//!
//! # What the tier-2 disagreements are
//!
//! Tier 1 is exact on both corpora. Tier 2 is not, and the two remainders carry
//! **different evidential weight** — a distinction worth keeping, because the
//! multi-component bulk is unexamined and the connected few are not:
//!
//! - For the **connected sub-tier** — three cases on the default corpus, one in
//!   braid mode — the verdict is *established*: each was individually checked
//!   and each is slot-slack, hence outside `𝔉′`, hence no counterexample.
//! - For the **multi-component remainder** — the other 237 of tier 2's 240
//!   default disagreements, and 36 of its 37 in braid mode (324/54 before
//!   [#185](https://github.com/sustia-llc/catgraph/issues/185); the connected
//!   sub-tier's 3 and 1 are unmoved, so the whole reduction landed in this
//!   remainder) — the verdict is an
//!   *expectation*, not a finding: a
//!   multi-component content has block and column freedom, which is `ι` by
//!   Lemma 4.4, so the tier's slot-pinnedness assumption is the thing most
//!   likely failing. No one has checked them case by case, and this lane does
//!   not claim otherwise.
//!
//! The connected-sub-tier cases, **measured 2026-07-30** (adversarial review of
//! PR1, both 100 000-pair sweeps) — three distinct contents, not one:
//!
//! - Default 2996 and 22872 are the *same* content (`content_eq`-verified),
//!   four hyperedges: `Copy ; (id₁ ⊗ Discard ⊗ Zero) ; Add`. Pinned as a unit
//!   witness, `display::tests::a_layer_pinned_eta_can_still_take_two_layers`.
//! - Default 45412 is a **different** content — six hyperedges,
//!   `(id₁ ⊗ Scalar(false)) ; (Copy ⊗ Zero ⊗ id₁) ; (id₁ ⊗ Discard ⊗ Add) ; Add`
//!   — with `nf` at three layers against the display's four.
//! - Braid-mode 96178 is a **third** — seven hyperedges, two `Scalar`s and two
//!   `Add`s.
//!
//! All three are slot-slack by the same comparison: the `η`'s wire coordinate
//! on the boundary below its own layer differs between the two realizations
//! (45412: coordinate 2 of `W₁ = (3, 4, 5, 2)` under `nf` against coordinate 1
//! of `W₂ = (3, 5, 2)` in the display; 96178: coordinate 2 of `(2, 3, 4, 5)`
//! against coordinate 1 of `(6, 4, 5)`). One caveat rides every one of those
//! comparisons (stated with the witness in `display.rs` and repeated here
//! because the verdict rests on it): when the two realizations put the `η` at
//! *different* layers the compared boundaries have different widths, and under
//! that reading of §4.4's slot-pinnedness a λ-ambiguity yields slot-slack
//! almost automatically. Only the first content is pinned; the other two are a
//! dated measurement, and a future engine change could move which case indices
//! exhibit them.
//!
//! **Re-measured 2026-08-02 after
//! [#185](https://github.com/sustia-llc/catgraph/issues/185)** (symmetric Step
//! 6½ cuts, an engine change of exactly the kind that caveat anticipated): the
//! four indices are **unmoved** — 2996, 22872 and 45412 on the default corpus,
//! 96178 in braid mode. The dated measurement above therefore still reads on
//! this engine.
//!
//! # What #185 moved here, and in which direction
//!
//! Making Step 6½'s column cuts symmetric moved **both** sides of gates 3 and 4,
//! which is worth stating plainly because the convenient story would be that
//! only `nf` moved. `canonical_display` is `nf(expr_of_content(content_of(e)))`
//! — it runs `nf` too, on the canonical readback instead of on the user's
//! writing — so a change to `nf` reaches it as well. Measured on the default
//! corpus, A-side: `nf` alone moved on 596 cases, the display alone on 206, and
//! both on 297.
//!
//! What makes the lane's numbers move in one direction anyway is that the two
//! moved *toward each other*. The per-case transitions are **strictly
//! monotone**: 749 default and 121 braid cases turned gate-4 churn off, **zero**
//! turned it on; the same counts gained `nf == display` agreement and **zero**
//! lost it. No case changed tier, so every denominator (`eta_free`,
//! `layer_pinned`, `layer_pinned_connected`, `nf_braid_free`) and
//! `crossing_inserted` are unmoved. Tier 1 was already exact and stays exact.
//!
//! Both correctness columns stayed 0, and `convergence_failures` is the one to
//! notice: the display moved on 503 default and 143 braid cases yet
//! `canonical_display(A) == canonical_display(B)` still holds on all 100 000
//! pairs of both modes. That is claim 2 behaving as its by-construction argument
//! says it must — the readback is a function of the content's iso class, so when
//! the display moves it moves on both sides of a pair together.

#![cfg(feature = "internal-probes")]

use std::time::Instant;

use catgraph_applied::prop::PropExpr;
use catgraph_applied::prop::presentation::content::{Content, content_eq, content_of};
use catgraph_applied::prop::presentation::display::{canonical_display, expr_of_content};
use catgraph_applied::prop::presentation::smc_nf::{Atom, StringDiagram, nf};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;

type Sfg = SfgGenerator<BoolRig>;
type E = PropExpr<Sfg>;

/// The design round's seed. Every pin below is relative to it.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The published corpus size.
const FULL_PAIRS: usize = 100_000;

/// Fast tier — a prefix of the same corpus, not a different one.
const SMOKE_PAIRS: usize = 5_000;

/// `nf`, `content_of` and the readback all recurse over the expression tree
/// deep enough to overflow the default 2 MiB test-thread stack.
const STACK_BYTES: usize = 64 * 1024 * 1024;

// ---------------------------------------------------------------- rng

struct Rng(u64);

fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    fn new(index: usize) -> Self {
        let mut s = SEED ^ (index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
        splitmix64(&mut s);
        Rng(s)
    }
    fn next(&mut self) -> u64 {
        splitmix64(&mut self.0)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

// ---------------------------------------------------------------- corpus

const MAX_LAYERS: u64 = 4;
const MAX_INIT_WIDTH: u64 = 4;
const MAX_ATOMS: usize = 7;
const WIDTH_CAP: usize = 7;

fn g(x: Sfg) -> E {
    PropExpr::Generator(x)
}

fn gen_layer(rng: &mut Rng, w: usize, braid: bool) -> (E, usize) {
    let mut atoms: Vec<E> = Vec::new();
    let mut rem = w;
    while rem > 0 {
        if atoms.len() < MAX_ATOMS && rng.below(5) == 0 {
            atoms.push(g(SfgGenerator::Zero));
        }
        if braid && rem >= 2 && rng.below(3) == 0 {
            atoms.push(PropExpr::Braid(1, 1));
            rem -= 2;
            continue;
        }
        let wide = rem >= WIDTH_CAP;
        let a = match rng.below(6) {
            0 => PropExpr::Identity(1),
            1 if !wide => g(SfgGenerator::Copy),
            2 => g(SfgGenerator::Discard),
            3 if !wide => g(SfgGenerator::Scalar(BoolRig(true))),
            4 if !wide => g(SfgGenerator::Scalar(BoolRig(false))),
            _ if rem >= 2 => g(SfgGenerator::Add),
            _ => g(SfgGenerator::Discard),
        };
        rem -= a.source();
        atoms.push(a);
    }
    if atoms.len() < MAX_ATOMS && rng.below(4) == 0 {
        atoms.push(g(SfgGenerator::Zero));
    }
    if atoms.is_empty() {
        atoms.push(g(SfgGenerator::Zero));
    }
    let out = atoms.iter().map(PropExpr::target).sum();
    let layer = atoms
        .into_iter()
        .reduce(|x, y| PropExpr::Tensor(Box::new(x), Box::new(y)))
        .expect("nonempty");
    (layer, out)
}

fn gen_expr(rng: &mut Rng, braid: bool) -> E {
    let mut w = rng.below(MAX_INIT_WIDTH) as usize;
    let n = 1 + rng.below(MAX_LAYERS) as usize;
    let mut layers = Vec::new();
    for _ in 0..n {
        let (l, out) = gen_layer(rng, w, braid);
        w = out;
        layers.push(l);
    }
    layers
        .into_iter()
        .reduce(|x, y| PropExpr::Compose(Box::new(x), Box::new(y)))
        .expect("nonempty")
}

fn count_nodes(e: &E, tensor: bool) -> usize {
    match e {
        PropExpr::Compose(a, b) => {
            usize::from(!tensor) + count_nodes(a, tensor) + count_nodes(b, tensor)
        }
        PropExpr::Tensor(a, b) => {
            usize::from(tensor) + count_nodes(a, tensor) + count_nodes(b, tensor)
        }
        _ => 0,
    }
}

fn rewrite_nth(e: &E, n: &mut isize, kind: u8) -> E {
    match e {
        PropExpr::Compose(a, b) => {
            if kind == 2 {
                if *n == 0 {
                    *n -= 1;
                    let k = a.target();
                    return PropExpr::Compose(
                        a.clone(),
                        Box::new(PropExpr::Compose(
                            Box::new(PropExpr::Identity(k)),
                            b.clone(),
                        )),
                    );
                }
                *n -= 1;
            }
            let na = rewrite_nth(a, n, kind);
            let nb = rewrite_nth(b, n, kind);
            PropExpr::Compose(Box::new(na), Box::new(nb))
        }
        PropExpr::Tensor(a, b) => {
            if kind != 2 {
                if *n == 0 {
                    *n -= 1;
                    let (m, nn) = (a.source(), a.target());
                    let (p, q) = (b.source(), b.target());
                    return if kind == 0 {
                        PropExpr::Compose(
                            Box::new(PropExpr::Tensor(a.clone(), Box::new(PropExpr::Identity(p)))),
                            Box::new(PropExpr::Tensor(
                                Box::new(PropExpr::Identity(nn)),
                                b.clone(),
                            )),
                        )
                    } else {
                        PropExpr::Compose(
                            Box::new(PropExpr::Tensor(Box::new(PropExpr::Identity(m)), b.clone())),
                            Box::new(PropExpr::Tensor(a.clone(), Box::new(PropExpr::Identity(q)))),
                        )
                    };
                }
                *n -= 1;
            }
            let na = rewrite_nth(a, n, kind);
            let nb = rewrite_nth(b, n, kind);
            PropExpr::Tensor(Box::new(na), Box::new(nb))
        }
        other => other.clone(),
    }
}

/// Case `i`: the pair `(A, B)` with `B` one sound rewriting of `A`.
fn case(i: usize, braid: bool) -> (E, E) {
    let mut rng = Rng::new(i);
    let a = gen_expr(&mut rng, braid);
    let kind = match rng.below(10) {
        0 | 1 => 2u8,
        x if x % 2 == 0 => 0,
        _ => 1,
    };
    let total = count_nodes(&a, kind != 2);
    if total == 0 {
        return (a.clone(), a);
    }
    let mut n = (rng.below(total as u64)) as isize;
    let b = rewrite_nth(&a, &mut n, kind);
    (a, b)
}

// ------------------------------------------------- content-level classifier

/// Whether the diagram carries a crossing.
fn braid_free(sd: &StringDiagram<Sfg>) -> bool {
    !sd.layers
        .iter()
        .any(|l| l.atoms.iter().any(|a| matches!(a, Atom::Braid(_, _))))
}

/// §4.4's `ldepth`, per hyperedge: 0 when every source node is on the input
/// foot (in particular when there are no source tentacles), else one more than
/// the deepest producer of a source node. A function of the content alone.
fn ldepth(c: &Content<Sfg>) -> Vec<usize> {
    let mut producer = vec![usize::MAX; c.node_count()];
    for (index, edge) in c.edges().iter().enumerate() {
        for &x in &edge.targets {
            producer[x] = index;
        }
    }
    let mut on_input = vec![false; c.node_count()];
    for &x in c.input() {
        on_input[x] = true;
    }

    let mut depth = vec![0usize; c.edges().len()];
    // The content is acyclic (BGKSZ Thm 3.12), so relaxation reaches its
    // fixpoint in at most one pass per hyperedge.
    for _ in 0..=c.edges().len() {
        let mut changed = false;
        for (index, edge) in c.edges().iter().enumerate() {
            let d = if edge.sources.iter().all(|&x| on_input[x]) {
                0
            } else {
                1 + edge
                    .sources
                    .iter()
                    .filter(|&&x| producer[x] != usize::MAX)
                    .map(|&x| depth[producer[x]])
                    .max()
                    .unwrap_or(0)
            };
            if d != depth[index] {
                depth[index] = d;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    depth
}

/// Whether the content is one connected component — no closed block, no second
/// boundary-attached component.
///
/// This is the sub-tier of the layer-pinned tier where slot-pinnedness is not
/// merely assumed but plausible. A content with two components has *block* and
/// *column* freedom on top of the slot freedom (§4.4's Lemma 4.4 note: block
/// order, column order and closed-block placement are consequences of `(λ, ι)`,
/// so a free block pair is exactly an `ι` the content leaves open), and no
/// computable classifier can rule that out case by case.
fn connected(c: &Content<Sfg>) -> bool {
    let mut parent: Vec<usize> = (0..c.node_count()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    for edge in c.edges() {
        let mut nodes = edge.sources.iter().chain(edge.targets.iter()).copied();
        let Some(head) = nodes.next() else {
            // A `0 → 0` hyperedge is a component with no nodes at all, so a
            // content carrying one is never single-component.
            return false;
        };
        for x in nodes {
            let (a, b) = (find(&mut parent, head), find(&mut parent, x));
            parent[a] = b;
        }
    }
    (0..c.node_count())
        .map(|x| find(&mut parent, x))
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        <= 1
}

/// The gate-3 tier of a content: see the module docs for what each tier checks
/// and what it assumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    /// No source-0 hyperedge — §4.4's `η`-free corollary, hypotheses all met.
    EtaFree,
    /// Every source-0 hyperedge is `0 → 1` with `ceil = 1`; slot-pinnedness
    /// assumed.
    LayerPinned,
    /// Outside both — not scored.
    Unscored,
}

fn tier(c: &Content<Sfg>) -> Tier {
    let sources0: Vec<usize> = (0..c.edges().len())
        .filter(|&e| c.edges()[e].sources.is_empty())
        .collect();
    if sources0.is_empty() {
        return Tier::EtaFree;
    }
    let depth = ldepth(c);
    let overall = 1 + depth.iter().copied().max().unwrap_or(0);
    let mut consumer = vec![usize::MAX; c.node_count()];
    for (index, edge) in c.edges().iter().enumerate() {
        for &x in &edge.sources {
            consumer[x] = index;
        }
    }
    for e in sources0 {
        let targets = &c.edges()[e].targets;
        // §4.4's definedness scope: `0 → 0` and `0 → n` (n ≥ 2) are deemed
        // slack-bearing, so they leave the tier.
        let [z] = targets[..] else {
            return Tier::Unscored;
        };
        let ceiling = if consumer[z] == usize::MAX {
            overall
        } else {
            depth[consumer[z]]
        };
        if ceiling != 1 {
            return Tier::Unscored;
        }
    }
    Tier::LayerPinned
}

// ---------------------------------------------------------------- scoring

/// Everything one pass over the corpus measures.
#[derive(Debug, Default, PartialEq, Eq)]
struct Gate {
    /// Contents checked by the proof obligation — both sides of every pair.
    readback_checked: usize,
    /// Of those, the ones where the readback lost the content. Must be 0.
    readback_failures: usize,
    /// Pairs whose displays differ. Must be 0.
    convergence_failures: usize,
    /// A-side cases whose normal form is braid-free.
    nf_braid_free: usize,
    /// Of those, the ones whose display carries a crossing — gate 1.
    crossing_inserted: usize,
    /// A-side cases where the display differs from the normal form — gate 4.
    churn: usize,
    /// Gate 3, tier 1: `η`-free with both diagrams braid-free.
    eta_free: usize,
    /// Of those, the ones where display and normal form agree.
    eta_free_agree: usize,
    /// Gate 3, tier 2: every source-0 hyperedge `0 → 1` and layer-pinned, both
    /// diagrams braid-free.
    layer_pinned: usize,
    /// Of those, the ones where display and normal form agree.
    layer_pinned_agree: usize,
    /// Gate 3, tier 2 narrowed to single-component contents — the sub-tier
    /// where slot-pinnedness is plausible rather than merely assumed.
    layer_pinned_connected: usize,
    /// Of those, the ones where display and normal form agree.
    layer_pinned_connected_agree: usize,
}

fn sweep(pairs: usize, braid: bool) -> Gate {
    let mut gate = Gate::default();
    for i in 0..pairs {
        let (a, b) = case(i, braid);

        for e in [&a, &b] {
            let c = content_of(e);
            gate.readback_checked += 1;
            if !content_eq(&content_of(&expr_of_content(&c)), &c) {
                gate.readback_failures += 1;
            }
        }

        let (da, db) = (canonical_display(&a), canonical_display(&b));
        if da != db {
            gate.convergence_failures += 1;
        }

        let na = nf(&a);
        if da != na {
            gate.churn += 1;
        }
        if braid_free(&na) {
            gate.nf_braid_free += 1;
            if !braid_free(&da) {
                gate.crossing_inserted += 1;
            }
        }

        if braid_free(&na) && braid_free(&da) {
            let ca = content_of(&a);
            match tier(&ca) {
                Tier::EtaFree => {
                    gate.eta_free += 1;
                    gate.eta_free_agree += usize::from(da == na);
                }
                Tier::LayerPinned => {
                    gate.layer_pinned += 1;
                    gate.layer_pinned_agree += usize::from(da == na);
                    if connected(&ca) {
                        gate.layer_pinned_connected += 1;
                        gate.layer_pinned_connected_agree += usize::from(da == na);
                    }
                }
                Tier::Unscored => {}
            }
        }
    }
    gate
}

/// Run `f` on a thread with a stack deep enough for the recursion.
fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(f)
        .expect("spawn worker")
        .join()
        .expect("worker panicked")
}

// ---------------------------------------------------------------- pins

/// Fast tier, default corpus: the 5 000-case prefix of the published corpus.
///
/// Lineage: **churn 264 → 219, `layer_pinned_agree` 1 975 → 1 982**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02); every other field verified unmoved. 45 prefix cases turned
/// churn off and none turned it on, 7 of them inside tier 2.
#[test]
fn smoke_prefix_default_corpus() {
    let gate = on_big_stack(|| sweep(SMOKE_PAIRS, false));
    assert_eq!(
        gate,
        Gate {
            readback_checked: 10_000,
            readback_failures: 0,
            convergence_failures: 0,
            nf_braid_free: 5_000,
            crossing_inserted: 87,
            churn: 219,
            eta_free: 785,
            eta_free_agree: 785,
            layer_pinned: 1_995,
            layer_pinned_agree: 1_982,
            layer_pinned_connected: 632,
            layer_pinned_connected_agree: 631,
        },
        "the 5k prefix moved. `readback_failures` and `convergence_failures` \
         are correctness; the rest are measurements, and a change in them is a \
         behavior change in the readback."
    );
}

/// Fast tier, braid-injecting corpus.
///
/// Lineage: **churn 1 030 → 1 027**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02); every other field verified unmoved, `layer_pinned_agree`
/// included — all three prefix cases that turned churn off sit outside tier 2.
#[test]
fn smoke_prefix_braid_corpus() {
    let gate = on_big_stack(|| sweep(SMOKE_PAIRS, true));
    assert_eq!(
        gate,
        Gate {
            readback_checked: 10_000,
            readback_failures: 0,
            convergence_failures: 0,
            nf_braid_free: 3_439,
            crossing_inserted: 22,
            churn: 1_027,
            eta_free: 562,
            eta_free_agree: 562,
            layer_pinned: 1_552,
            layer_pinned_agree: 1_551,
            layer_pinned_connected: 607,
            layer_pinned_connected_agree: 607,
        },
        "the braid-mode 5k prefix moved"
    );
}

/// **The PR1 proof obligation, at full corpus size, default mode.**
///
/// Lineage: **churn 5 334 → 4 585, `layer_pinned_agree` 39 991 → 40 075**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02); every other field verified unmoved. 749 cases turned churn
/// off and none turned it on — 84 of them inside tier 2, the rest unscored (tier
/// 1 gained none, being exact already). Both `nf` and the display moved here
/// (the display runs `nf` on the canonical readback — see the module docs); what
/// the two figures record is that they moved *toward each other*, never apart.
#[test]
#[ignore = "100k-pair sweep. Run with --ignored alongside the NF sweep."]
fn published_corpus_gates() {
    let gate = on_big_stack(|| sweep(FULL_PAIRS, false));
    assert_eq!(
        gate,
        Gate {
            readback_checked: 200_000,
            readback_failures: 0,
            convergence_failures: 0,
            nf_braid_free: 100_000,
            crossing_inserted: 1_628,
            churn: 4_585,
            eta_free: 16_103,
            eta_free_agree: 16_103,
            layer_pinned: 40_315,
            layer_pinned_agree: 40_075,
            layer_pinned_connected: 12_616,
            layer_pinned_connected_agree: 12_613,
        },
        "the published-corpus gate numbers moved"
    );
}

/// The same, with braids injected — the mode whose divergences no NF-level fix
/// reaches (dead braid prefixes, marked residual-(a)).
///
/// Lineage: **churn 19 098 → 18 977, `layer_pinned_agree` 32 270 → 32 287**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02); every other field verified unmoved. 121 cases turned churn
/// off and none turned it on, 17 of them inside tier 2.
#[test]
#[ignore = "100k-pair braid-mode sweep. Run with --ignored alongside the NF sweep."]
fn braid_mode_corpus_gates() {
    let gate = on_big_stack(|| sweep(FULL_PAIRS, true));
    assert_eq!(
        gate,
        Gate {
            readback_checked: 200_000,
            readback_failures: 0,
            convergence_failures: 0,
            nf_braid_free: 70_449,
            crossing_inserted: 351,
            churn: 18_977,
            eta_free: 11_756,
            eta_free_agree: 11_756,
            layer_pinned: 32_324,
            layer_pinned_agree: 32_287,
            layer_pinned_connected: 12_257,
            layer_pinned_connected_agree: 12_256,
        },
        "the braid-mode gate numbers moved"
    );
}

/// Gate 4's cost half: roughly what one `canonical_display` call costs against
/// one `nf` call, on the corpus's shape distribution. Reported, not asserted —
/// a wall-clock ratio is not a pin.
#[test]
#[ignore = "timing measurement; run with --ignored --nocapture in release"]
fn display_cost_against_nf() {
    const CASES: usize = 20_000;
    let (nf_nanos, display_nanos) = on_big_stack(|| {
        let corpus: Vec<E> = (0..CASES).map(|i| case(i, false).0).collect();

        let start = Instant::now();
        let mut sink = 0usize;
        for e in &corpus {
            sink += nf(e).layers.len();
        }
        let nf_elapsed = start.elapsed();

        let start = Instant::now();
        for e in &corpus {
            sink += canonical_display(e).layers.len();
        }
        let display_elapsed = start.elapsed();

        assert!(sink > 0);
        (
            nf_elapsed.as_nanos() / CASES as u128,
            display_elapsed.as_nanos() / CASES as u128,
        )
    });
    println!("nf: {nf_nanos} ns/call, canonical_display: {display_nanos} ns/call");
}
