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

#![cfg(feature = "internal-probes")]

use catgraph_applied::prop::PropExpr;
use catgraph_applied::prop::presentation::smc_nf::{fragment_status, nf};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;

type Sfg = SfgGenerator<BoolRig>;
type E = PropExpr<Sfg>;

/// The design round's seed. Every pin below is relative to it.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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

// ---------------------------------------------------------------- sweep

/// The three published buckets. `marked` is read off `A`'s normal form, and a
/// pair is in-`𝔉` only when both sides are — both matching the scratch driver.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    divergent: usize,
    in_fragment: usize,
    marked: usize,
}

fn sweep(pairs: usize, braid: bool) -> Counts {
    let mut counts = Counts::default();
    for i in 0..pairs {
        let (a, b) = case(i, braid);
        let (na, nb) = (nf(&a), nf(&b));
        if na == nb {
            continue;
        }
        counts.divergent += 1;
        if fragment_status(&na).in_fragment() && fragment_status(&nb).in_fragment() {
            counts.in_fragment += 1;
        }
        if fragment_status(&na).any_marked {
            counts.marked += 1;
        }
    }
    counts
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
    let counts = on_big_stack(|| sweep(FULL_PAIRS, false));
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
    let counts = on_big_stack(|| sweep(FULL_PAIRS, true));
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

/// Fast tier over the same frozen corpus — a 5 000-case prefix, so CI notices a
/// gross change without the full run and a failure names a real case index.
#[test]
fn smoke_prefix_of_the_published_corpus() {
    let counts = on_big_stack(|| sweep(SMOKE_PAIRS, false));
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
