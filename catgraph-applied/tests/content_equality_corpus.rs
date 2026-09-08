//! Content equality scored against the published differential-sweep corpora
//! ([#57](https://github.com/sustia-llc/catgraph/issues/57), a1).
//!
//! # What this is
//!
//! `tests/smc_nf_differential_sweep.rs` pins how many SMC-equal pairs the normal
//! form *separates*: 183 on the default corpus, 1 153 with braids injected. By
//! Lemma 4.1 (`docs/SMC-NF-RECONCILIATION.md` §4.2) content decides SMC-equality
//! on every one of them, so the pins here are the complementary claim —
//! `content_eq` closes **all** of them, including the marked residual-(a) cases
//! and the dead-braid-prefix shapes no NF-level fix reaches.
//!
//! # The #185 re-pin (2026-08-02)
//!
//! [#185](https://github.com/sustia-llc/catgraph/issues/185)'s symmetric Step 6½
//! cuts converged 70 default and 9 braid pairs (and newly diverged none), so the
//! `divergent` and `in_fragment` figures below move in lockstep with the sweep's
//! — this file's corpus is a copy of that one's, and its pins are that file's
//! pins plus the content verdict. **Nothing about content moved**: `content_of`
//! and `content_eq` never consult `nf`, every one of the 79 converged pairs was
//! already `content_eq`- *and* `canonical_key`-equal before the change, and the
//! remaining divergences are closed exactly as before. The claim this file
//! exists to make — content closes *all* of them — is therefore re-verified at
//! the new totals rather than weakened by them.
//!
//! The corpus is the sweep's, copied rather than re-derived, exactly as that file
//! documents: same seed, same generator, same rewritings, so case indices line up
//! with its pins and with §4.6's table. A divergence count that no longer matches
//! the sweep means the copy has drifted, and the sweep's own pins are the
//! authority.
//!
//! # The negative controls
//!
//! Scoring only divergent pairs would be passed by a function that returns
//! `true`. The cross-corpus controls pair case `i` against case `i + 50 000` —
//! unrelated expressions — and pin the exact set of indices where the two are
//! genuinely the same morphism. Each hit is cross-checked against `nf`, and every
//! non-hit must come out unequal under both `content_eq` and `canonical_key`.

#![cfg(feature = "internal-probes")]

use catgraph_applied::prop::PropExpr;
use catgraph_applied::prop::presentation::content::{
    Content, canonical_key, content_eq, content_of,
};
use catgraph_applied::prop::presentation::smc_nf::{fragment_status, from_string_diagram, nf};
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
    /// The #183 corpus: the `A…B…A` owner-word gadget.
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

/// Fast tier — a prefix of the same corpus, not a different one.
const SMOKE_PAIRS: usize = 5_000;

/// Cross-corpus negative controls: case `i` against case `i + CROSS_OFFSET`.
const CROSS_PAIRS: usize = 2_000;
const CROSS_OFFSET: usize = 50_000;

/// `nf` and `content_of` both recurse over the expression tree deep enough to
/// overflow the default 2 MiB test-thread stack.
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
    (tensor_atoms(atoms), out)
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

/// An expression whose input owner word is `A…B…A` **by construction** — the
/// gadget and why no tail undoes it are described in
/// `tests/smc_nf_differential_sweep.rs`'s module docs.
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

// ---------------------------------------------------------------- scoring

/// What the sweep counts, plus what content does with it.
#[derive(Debug, Default, PartialEq, Eq)]
struct Score {
    /// Pairs whose normal forms differ — the sweep's own count.
    divergent: usize,
    /// Of those, the ones with both normal forms in `𝔉`.
    in_fragment: usize,
    /// Of the divergent pairs, the ones `content_eq` closes.
    content_equal: usize,
    /// Of the divergent pairs, the ones `canonical_key` closes.
    key_equal: usize,
}

fn key_agrees(a: &Content<Sfg>, b: &Content<Sfg>) -> bool {
    canonical_key(a) == canonical_key(b)
}

fn sweep(pairs: usize, mode: Mode) -> Score {
    let mut score = Score::default();
    for i in 0..pairs {
        let (a, b) = case(i, mode);
        let (na, nb) = (nf(&a), nf(&b));
        if na == nb {
            continue;
        }
        score.divergent += 1;
        let in_fragment = fragment_status(&na).in_fragment() && fragment_status(&nb).in_fragment();
        if in_fragment {
            score.in_fragment += 1;
        }
        let (ca, cb) = (content_of(&a), content_of(&b));
        if content_eq(&ca, &cb) {
            score.content_equal += 1;
        }
        if key_agrees(&ca, &cb) {
            score.key_equal += 1;
        }
    }
    score
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

/// Fast tier: the 5 000-case prefix of the published corpus. Its divergence
/// count is `smoke_prefix_of_the_published_corpus`'s, and content closes all of
/// them.
///
/// Lineage: **16 / 6 / 16 / 16 → 14 / 6 / 14 / 14**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02) — cases 3061 and 4722 converged, both already
/// content-closed, so `divergent` and both content columns drop by the same 2
/// and the claim is unweakened.
#[test]
fn smoke_prefix_is_closed_by_content() {
    let score = on_big_stack(|| sweep(SMOKE_PAIRS, Mode::Default));
    assert_eq!(
        score,
        Score {
            divergent: 14,
            in_fragment: 6,
            content_equal: 14,
            key_equal: 14,
        },
        "the 5k prefix moved. If `divergent` is what changed, compare against \
         `smc_nf_differential_sweep::smoke_prefix_of_the_published_corpus` — the \
         corpus copy here is meant to be bit-identical to that file's."
    );
}

/// **The a1 claim, at full corpus size.** All 183 divergent pairs of §4.6's
/// published table are content-equal.
///
/// Lineage: **253 / 128 / 253 / 253 → 183 / 93 / 183 / 183**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02) — 70 pairs converged, all 70 already content-closed, none
/// newly divergent.
#[test]
#[ignore = "100k-pair sweep. Run with --ignored alongside the NF sweep."]
fn published_corpus_is_closed_by_content() {
    let score = on_big_stack(|| sweep(FULL_PAIRS, Mode::Default));
    assert_eq!(
        score,
        Score {
            divergent: 183,
            in_fragment: 93,
            content_equal: 183,
            key_equal: 183,
        },
        "content no longer closes every published divergence"
    );
}

/// The braid-injecting corpus: 1 153 divergences, including the marked
/// residual-(a) cases and the dead-braid-prefix shapes. Content closes those
/// too, which is the part no NF-level fix reaches.
///
/// Lineage: **1162 / 634 / 1162 / 1162 → 1153 / 630 / 1153 / 1153**
/// ([#185](https://github.com/sustia-llc/catgraph/issues/185) symmetric Step 6½
/// cuts, 2026-08-02) — 9 pairs converged, all 9 already content-closed, none
/// newly divergent. The marked and dead-braid-prefix shapes this tier is here
/// for are untouched: none of the 9 was marked.
#[test]
#[ignore = "100k-pair braid-mode sweep. Run with --ignored alongside the NF sweep."]
fn braid_mode_corpus_is_closed_by_content() {
    let score = on_big_stack(|| sweep(FULL_PAIRS, Mode::Braid));
    assert_eq!(
        score,
        Score {
            divergent: 1_153,
            in_fragment: 630,
            content_equal: 1_153,
            key_equal: 1_153,
        },
        "content no longer closes every braid-mode divergence"
    );
}

/// The #183 interleave corpus: the braid-free `A…B…A` owner-word gadget, whose
/// marked-case coverage is pinned by
/// `smc_nf_differential_sweep::published_interleave_mode_figures_reproduce`.
/// 745 divergences, none in `𝔉`, all closed by content and by `canonical_key`.
///
/// Lineage: **745 / 0 / 745 / 745**, at introduction (2026-09-08). The
/// `divergent` and `in_fragment` columns are
/// `smc_nf_differential_sweep::published_interleave_mode_figures_reproduce`'s;
/// the two content columns are measured here.
#[test]
#[ignore = "100k-pair interleave-mode sweep. Run with --ignored alongside the NF sweep."]
fn interleave_mode_corpus_is_closed_by_content() {
    let score = on_big_stack(|| sweep(FULL_PAIRS, Mode::Interleave));
    assert_eq!(
        score,
        Score {
            divergent: 745,
            in_fragment: 0,
            content_equal: 745,
            key_equal: 745,
        },
        "content no longer closes every interleave-mode divergence. If \
         `divergent` or `in_fragment` is what moved, compare against \
         `smc_nf_differential_sweep::published_interleave_mode_figures_reproduce` \
         — the corpus copy here is meant to be bit-identical to that file's."
    );
}

/// **Negative controls.** Unrelated corpus cases must come out unequal, with the
/// exception of the pairs that happen to be the same morphism — pinned by index,
/// and each one cross-checked against `nf`.
#[test]
fn cross_corpus_pairs_are_separated() {
    /// The pairs `(i, i + 50 000)` that really are the same morphism.
    const KNOWN_EQUAL: &[usize] = &[44, 354, 646, 795, 818, 822, 1098, 1116, 1914, 1997];

    let hits = on_big_stack(|| {
        let mut hits = Vec::new();
        for i in 0..CROSS_PAIRS {
            let (a, _) = case(i, Mode::Default);
            let (c, _) = case(i + CROSS_OFFSET, Mode::Default);
            let (ca, cc) = (content_of(&a), content_of(&c));
            let equal = content_eq(&ca, &cc);
            assert_eq!(
                key_agrees(&ca, &cc),
                equal,
                "case {i}: canonical_key disagrees with content_eq"
            );
            // **Containment**, on unrelated pairs — the direction that matters
            // for `Presentation::eq_mod`. `nf` preserves content (§4.3 Lemma
            // 4.2), so equal normal forms force equal content, and the content
            // relation therefore *contains* the NF relation. That is what makes
            // swapping the one for the other in `eq_mod` monotone: every pair the
            // NF short-circuit decided equal, content decides equal too, so the
            // change can only add `Some(true)` verdicts and never remove one.
            if !equal {
                assert_ne!(
                    nf(&a),
                    nf(&c),
                    "case {i}: normal forms agree but content does not — `nf` \
                     would have decided a pair content declines, so the content \
                     relation does not contain the NF relation"
                );
            }
            if equal {
                // A content-equal hit is a claim of SMC-equality; `nf` must
                // agree, or one of the two is unsound.
                assert_eq!(
                    nf(&a),
                    nf(&c),
                    "case {i}: content says equal but the normal forms differ"
                );
                hits.push(i);
            }
        }
        hits
    });

    assert_eq!(
        hits, KNOWN_EQUAL,
        "the set of genuinely-equal cross pairs moved; a *new* index is a false \
         equality unless `nf` also agrees on it (which is asserted above)"
    );
}

/// §4.3 Lemma 4.2 over the corpus: `nf` preserves content, so `C(e)` and
/// `C(readback(nf(e)))` agree. Sampled, since the point is coverage of shapes
/// rather than of every case.
#[test]
fn nf_preserves_content_across_the_corpus() {
    let checked = on_big_stack(|| {
        let mut checked = 0usize;
        for i in (0..SMOKE_PAIRS).step_by(25) {
            let (a, _) = case(i, Mode::Default);
            let readback = from_string_diagram(&nf(&a));
            assert!(
                content_eq(&content_of(&a), &content_of(&readback)),
                "case {i}: nf did not preserve content"
            );
            checked += 1;
        }
        checked
    });
    assert_eq!(checked, 200);
}
