//! The #174 design round's differential sweep, ported in-tree.
//!
//! # What this is
//!
//! `docs/SMC-NF-RECONCILIATION.md` §4.6 publishes divergence counts from a
//! 100 000-pair sweep run during the #174 comparator design round. The driver
//! that produced them was a scratch harness outside the repository: real,
//! seed-addressable, and reproducible by hand, but invisible to CI — so nothing
//! failed when the engine drifted away from the published numbers.
//!
//! This is that driver, ported. The corpus generator, the rewritings, the seed
//! and the classification are the round's, unchanged, so the pins below are
//! **the published figures themselves** rather than a re-based substitute.
//!
//! # The experiment
//!
//! Case `i` is generated purely from `splitmix64(SEED ^ i)`, so the corpus is
//! identical across builds and any case can be re-run in isolation. Each case is
//! a random layered SFG expression `A` over `BoolRig` paired with `B`, one sound
//! rewriting of `A` — either an interchange split of a `⊗` node
//! (`a⊗b = (a⊗id);(id⊗b)`, both associations) or an identity-padding slide on a
//! `;` node. All three are instances of bifunctoriality with the unit (JS-I Ch 1
//! §4 Thm 1.2 p.71), so `A` and `B` are SMC-equal by construction and any
//! difference in normal form is a canonicality gap, never a soundness one.
//!
//! Divergences are bucketed by `smc_nf::fragment_status`, which reproduces the
//! round's `probe::marking` exactly — including its reading of the **stored**
//! layers rather than the identity-split refinement. A pair counts as in-`𝔉`
//! only when *both* normal forms are, matching the driver.
//!
//! # Two deviations from the scratch driver, both deliberate
//!
//! * **The intransitivity counters are not ported.** They read
//!   `analyze_components_refined` and `column_key_order`, neither of which
//!   survives in the shipped engine — the Cmin rework deleted the first and
//!   never had the second. What they measured is `0` on this engine by
//!   construction anyway: `component_key_order` is a plain `CompKey` comparison
//!   and `Ord` is transitive. The figure worth guarding was the *rejected*
//!   variant's ≈37%, and that variant is deliberately not in the tree.
//! * **The hang watchdog is dropped.** The driver restarted its sweep past cases
//!   that stalled for 20 s, because some candidate comparators did not terminate.
//!   The shipped engine terminates (§2.4's measure), and a sweep that silently
//!   skipped cases could not pin an exact count. If this test hangs, that is a
//!   termination regression and hanging is the correct signal.
//!
//! # These are `nf`-display trackers (re-labelled 2026-07-30, [#187](https://github.com/sustia-llc/catgraph/issues/187))
//!
//! The 253 and 1 162 figures below count SMC-equal pairs that **the normal form
//! separates**. They no longer measure anything unresolved, and it is worth
//! being exact about what closed them:
//!
//! * **Equality** — closed by content ([#57](https://github.com/sustia-llc/catgraph/issues/57)
//!   a1). `content_eq` decides all 253 and all 1 162 (`tests/content_equality_corpus.rs`),
//!   and `Presentation::eq_mod` has taken the content path since a1 PR2.
//! * **Display** — closed by canonical display (#187 PR1).
//!   `canonical_display` converges on all 100 000 pairs of both modes
//!   (`tests/canonical_display_corpus.rs`, expected 0 and measured 0). That lane
//!   is the "expected 0" counterpart to these counts' "expected exactly N".
//!
//! What is left is a fact about `nf` itself, which #187 deliberately did not
//! change: these are the pairs whose *normal forms* differ. The pins therefore
//! stay, at exactly these numbers, as **drift detectors on the engine** — a
//! movement means `nf` changed, which is news whichever direction it moves.
//! §4.6's ledger entries keep their meaning for the same reason.
//!
//! # The interleave tier ([#183](https://github.com/sustia-llc/catgraph/issues/183))
//!
//! Residual (a) — marked (interleaved) components, the one lettered residual of
//! §4.6 still open, tracked on
//! [#174](https://github.com/sustia-llc/catgraph/issues/174) — is the weakest
//! axis of both corpora above. The default corpus produces **23** marked
//! divergences per 100 000, and it produces them incidentally: nothing in
//! `gen_layer` aims at interleaving. The braid tier reaches 237, but by a
//! mechanism §4.6 itself flags as conservative — a `Braid` *merges* the
//! components it spans, so braid-mode marking is an upper bound on content-level
//! marking (§4.4, `braid_coarsening_marks_content_clear_diagram`).
//!
//! The third tier fixes both. Its generator emits the interleaving **by
//! construction**, braid-free, so every case is a genuine content-level marked
//! case rather than a coarsening artifact.
//!
//! ## What is being forced
//!
//! `mark_interleaved` (`smc_nf.rs`) reads the owner word of a boundary — the
//! component owning each wire, left to right — collapses it to runs, and marks
//! every component in the span whenever one owner appears in **two
//! non-adjacent runs**. That is exactly an `A…B…A` owner word, and it is what
//! the generator builds.
//!
//! ## The gadget
//!
//! Two layers, over the input boundary (`pre` and `post` are random flanks,
//! `mid ∈ {1, 2}`):
//!
//! ```text
//!  layer 0:  id^pre ⊗ armL ⊗ (!)^mid ⊗ armR ⊗ id^post      arm ∈ {id, scalar}
//!  layer 1:  id^pre ⊗    μ (Add)     ⊗ id^post
//! ```
//!
//! Each `!` (`Discard`, `1 → 0`) has an **empty target**, so
//! `analyze_components`' emptiness guard joins it to nothing below: every `!` is
//! its own component, terminal, and can never be absorbed by a later layer. The
//! `μ` spans exactly `armL`'s and `armR`'s targets — which are adjacent, the
//! `!`s having consumed their wires — so union-find joins the two arms into one
//! component `A` across the splitters. The input owner word is therefore
//! `[…, A, B₁, …, B_mid, A, …]`: an `A…B…A` run pattern, forced, on 100% of
//! cases. Because the `B`s are terminal and `A` is boundary-attached, no tail
//! layer can undo it — the tail (0–2 layers of the round's own `gen_layer`) is
//! free to supply the `η`/`ε` shapes that make pairs actually diverge.
//!
//! The `marked_cases` pin below is the assertion that the construction holds:
//! it counts cases whose `nf(A)` carries a marked component at all, divergent or
//! not, and it is the full corpus size. `in_fragment` is 0 by the same
//! construction — `𝔉` excludes marked diagrams — so this tier trades the
//! in-fragment axis for the marked one deliberately.

#![cfg(feature = "internal-probes")]

use catgraph_applied::prop::PropExpr;
use catgraph_applied::prop::presentation::smc_nf::{fragment_status, nf};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;

type Sfg = SfgGenerator<BoolRig>;
type E = PropExpr<Sfg>;

/// The design round's seed. The default- and braid-tier pins are relative to it.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The interleave tier's seed (#183). A distinct odd constant — the second
/// splitmix64 finalizer multiplier — so this corpus is independent of the two
/// above rather than a re-reading of the same stream.
const INTERLEAVE_SEED: u64 = 0x94D0_49BB_1331_11EB;

/// Which corpus a case is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The design round's corpus, exactly.
    Default,
    /// The design round's corpus with `Braid(1, 1)` atoms injected.
    Braid,
    /// The #183 corpus: a forced `A…B…A` owner word on every case.
    Interleave,
}

impl Mode {
    fn seed(self) -> u64 {
        match self {
            Mode::Default | Mode::Braid => SEED,
            Mode::Interleave => INTERLEAVE_SEED,
        }
    }
}

/// The published corpus size.
const FULL_PAIRS: usize = 100_000;

/// Fast tier — the same generator and seed, truncated. A prefix of the corpus,
/// not a different one, so a smoke failure localizes to a real case index.
const SMOKE_PAIRS: usize = 5_000;

/// `nf` recurses over layered diagrams deep enough to overflow the default 2 MiB
/// test-thread stack; the scratch driver spawned its worker with 64 MiB for the
/// same reason.
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
    fn new(seed: u64, index: usize) -> Self {
        let mut s = seed ^ (index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
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

/// One layer over `w` incoming wires: a tensor of atoms whose sources sum to
/// `w`. Returns the layer and its outgoing width.
///
/// `braid` injects `Braid(1, 1)` atoms so components can own non-contiguous
/// boundary intervals and guard 3's marking actually fires. When off it consumes
/// no randomness, so the default corpus is bit-identical to the round's.
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

fn stack_layers(layers: Vec<E>) -> E {
    layers
        .into_iter()
        .reduce(|x, y| PropExpr::Compose(Box::new(x), Box::new(y)))
        .expect("nonempty")
}

fn tensor_atoms(atoms: Vec<E>) -> E {
    atoms
        .into_iter()
        .reduce(|x, y| PropExpr::Tensor(Box::new(x), Box::new(y)))
        .expect("nonempty")
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
    stack_layers(layers)
}

// ------------------------------------------------- the interleave gadget (#183)

/// Wires flanking the gadget on either side, exclusive bound.
const MAX_FLANK: u64 = 3;
/// Splitter (`Discard`) wires between the shared component's two arms, so
/// `mid ∈ {1, 2}`.
const MAX_SPLITTERS: u64 = 2;
/// Random layers appended below the gadget, exclusive bound — so a case is 2–4
/// layers deep, the same budget as [`MAX_LAYERS`] rather than a tuned one.
const MAX_TAIL: u64 = 3;

fn push_ids(atoms: &mut Vec<E>, n: usize) {
    for _ in 0..n {
        atoms.push(PropExpr::Identity(1));
    }
}

/// One arm of the shared component: a `1 → 1` atom, so the joining `μ` below
/// still spans exactly two wires.
fn arm(rng: &mut Rng) -> E {
    match rng.below(4) {
        0 => g(SfgGenerator::Scalar(BoolRig(true))),
        1 => g(SfgGenerator::Scalar(BoolRig(false))),
        _ => PropExpr::Identity(1),
    }
}

/// An expression whose input owner word is `A…B…A` **by construction** — see the
/// module docs for why the two layers force it and why no tail can undo it.
fn gen_interleaved_expr(rng: &mut Rng) -> E {
    let pre = rng.below(MAX_FLANK) as usize;
    let post = rng.below(MAX_FLANK) as usize;
    let mid = 1 + rng.below(MAX_SPLITTERS) as usize;

    let mut top: Vec<E> = Vec::new();
    push_ids(&mut top, pre);
    top.push(arm(rng));
    for _ in 0..mid {
        top.push(g(SfgGenerator::Discard));
    }
    top.push(arm(rng));
    push_ids(&mut top, post);

    let mut join: Vec<E> = Vec::new();
    push_ids(&mut join, pre);
    join.push(g(SfgGenerator::Add));
    push_ids(&mut join, post);

    let mut layers = vec![tensor_atoms(top), tensor_atoms(join)];
    let mut w = pre + 1 + post;
    for _ in 0..rng.below(MAX_TAIL) {
        let (l, out) = gen_layer(rng, w, false);
        w = out;
        layers.push(l);
    }
    stack_layers(layers)
}

// ------------------------------------------------------- sound rewritings

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

/// Rewrite the `n`-th eligible node (pre-order). `kind`:
/// 0 = `a⊗b → (a⊗id_p);(id_n⊗b)`, 1 = `a⊗b → (id_m⊗b);(a⊗id_q)`,
/// 2 = `x;y → x;(id;y)` (identity-padding slide on a composition).
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
fn case(i: usize, mode: Mode) -> (E, E) {
    let mut rng = Rng::new(mode.seed(), i);
    let a = match mode {
        Mode::Default => gen_expr(&mut rng, false),
        Mode::Braid => gen_expr(&mut rng, true),
        Mode::Interleave => gen_interleaved_expr(&mut rng),
    };
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

// ---------------------------------------------------------------- sweep

/// The three published buckets. `marked` is read off `A`'s normal form, and a
/// pair is in-`𝔉` only when both sides are — both matching the scratch driver.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    divergent: usize,
    in_fragment: usize,
    marked: usize,
}

/// [`Counts`] plus the coverage figure the interleave tier needs (#183).
///
/// The extra field is a property of the *corpus*, not of a divergence, so it is
/// kept off [`Counts`]: the two published tiers pin exactly the scratch driver's
/// three buckets and nothing else.
#[derive(Debug, Default, PartialEq, Eq)]
struct Tally {
    divergent: usize,
    in_fragment: usize,
    marked: usize,
    /// Cases whose `nf(A)` carries a marked component, **divergent or not**.
    marked_cases: usize,
}

impl Tally {
    fn counts(&self) -> Counts {
        Counts {
            divergent: self.divergent,
            in_fragment: self.in_fragment,
            marked: self.marked,
        }
    }
}

fn tally(pairs: usize, mode: Mode) -> Tally {
    let mut t = Tally::default();
    for i in 0..pairs {
        let (a, b) = case(i, mode);
        let (na, nb) = (nf(&a), nf(&b));
        let sa = fragment_status(&na);
        if sa.any_marked {
            t.marked_cases += 1;
        }
        if na == nb {
            continue;
        }
        t.divergent += 1;
        if sa.in_fragment() && fragment_status(&nb).in_fragment() {
            t.in_fragment += 1;
        }
        if sa.any_marked {
            t.marked += 1;
        }
    }
    t
}

fn sweep(pairs: usize, mode: Mode) -> Counts {
    tally(pairs, mode).counts()
}

/// Run `f` on a thread with a stack deep enough for `nf`'s recursion.
fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(f)
        .expect("spawn sweep worker")
        .join()
        .expect("sweep worker panicked")
}

/// **The published §4.6 figures, reproduced.** Exact two-sided pins on the
/// design round's own corpus, seed and classification.
///
/// A move here is a canonicality change, not a corpus artifact — the corpus is
/// frozen. Diagnose through `smc_canonicality_probes` (still the gate of record;
/// this is a tracker), then re-pin here and in §4.6's table together, because
/// the table quotes this test.
#[test]
#[ignore = "100k-pair sweep. Run with --ignored when the NF changes."]
fn published_divergence_figures_reproduce() {
    let counts = on_big_stack(|| sweep(FULL_PAIRS, Mode::Default));
    assert_eq!(
        counts,
        Counts {
            divergent: 253,
            in_fragment: 128,
            marked: 23,
        },
        "the sweep no longer reproduces SMC-NF-RECONCILIATION.md §4.6's published \
         figures (253 / 128 / 23). Either the normal form changed, or the corpus \
         or the fragment classification drifted from the design round's."
    );
}

/// **Residual-(a) tracker.** The braid-injecting corpus makes components own
/// non-contiguous boundary intervals, so guard 3's marking actually fires — the
/// default corpus produces too few marked cases to track residual (a) at all.
#[test]
#[ignore = "100k-pair braid-mode sweep. Run with --ignored when the NF changes."]
fn published_braid_mode_figures_reproduce() {
    let counts = on_big_stack(|| sweep(FULL_PAIRS, Mode::Braid));
    assert_eq!(
        counts,
        Counts {
            divergent: 1_162,
            in_fragment: 634,
            marked: 237,
        },
        "braid-mode sweep moved; this is the residual-(a) tracker (§4.6(a))."
    );
}

/// **Residual-(a) tracker, interleave-biased corpus
/// ([#183](https://github.com/sustia-llc/catgraph/issues/183); residual (a) is
/// tracked on [#174](https://github.com/sustia-llc/catgraph/issues/174)).**
///
/// Where the braid tier reaches marked cases incidentally and conservatively —
/// a `Braid` merges the components it spans, so its marking is an upper bound on
/// content-level marking (§4.4) — this corpus forces an `A…B…A` input owner word
/// on every case, braid-free, with terminal `Discard` splitters between the two
/// arms of one component. See the module docs for the gadget.
///
/// `marked_cases` is the construction's own assertion: every one of the 100 000
/// cases normalizes to a diagram guard 3 marks. `in_fragment` is 0 for the same
/// reason — `𝔉` excludes marked diagrams — which is the axis this tier trades
/// away.
///
/// Marked-case yield against §4.6's motivating deficit: the default corpus gives
/// **23** marked divergences per 100 000 and the braid corpus **237**; this one
/// gives **745** — 32× the default and 3.1× the braid tier, and unlike either it
/// draws them from a corpus that is 100% marked rather than incidentally so.
#[test]
#[ignore = "100k-pair interleave-mode sweep. Run with --ignored when the NF changes."]
fn published_interleave_mode_figures_reproduce() {
    let t = on_big_stack(|| tally(FULL_PAIRS, Mode::Interleave));
    assert_eq!(
        t,
        Tally {
            divergent: 745,
            in_fragment: 0,
            marked: 745,
            marked_cases: 100_000,
        },
        "interleave-mode sweep moved; this is the #183 residual-(a) tracker \
         (§4.6(a)). A `marked_cases` below {FULL_PAIRS} means the A…B…A \
         construction stopped forcing the marking, which is a generator bug \
         rather than an engine one."
    );
}

/// Fast tier over the same frozen corpus — a 5 000-case prefix, so CI notices a
/// gross change without the full run and a failure names a real case index.
#[test]
fn smoke_prefix_of_the_published_corpus() {
    let counts = on_big_stack(|| sweep(SMOKE_PAIRS, Mode::Default));
    assert_eq!(
        counts,
        Counts {
            divergent: 16,
            in_fragment: 6,
            marked: 2,
        },
        "the 5k prefix of the published corpus moved; run the full \
         `published_divergence_figures_reproduce` with --ignored to see whether \
         the published totals moved with it."
    );
}
