//! Adversarial probes for the §4.4 **pass-disjointness obligation**
//! (`docs/SMC-NF-RECONCILIATION.md`).
//!
//! The obligation: every move is either a free zero-width permutation
//! (content-decided, Step 6) or a pinned wire-carrying transposition
//! (geometry-decided, Steps 7 / 6½), and **no adjacency admits both readings**.
//!
//! These probes construct adjacencies that admit both readings with the two
//! families prescribing *opposite* layouts, and pin the resulting behaviour:
//! the `nf` loop cycles (Step 7/6½ swap, Step 6 swaps back), exits via the
//! `sd == prev` exact-cancellation accident, and the fixpoint **violates the
//! pre-restatement §1 transposition clauses** for Step 7 / Step 6½ order.
//! (The 2026-07-28 restatement ratifies these fixpoints — the both-readings
//! carve excepts exactly these adjacencies, and the class order wins. The
//! violation checkers below therefore replicate the *unrestated* clauses:
//! they pin the behaviour the carve exists to ratify.)
//!
//! Requires `--features internal-probes` (for `fragment_status` /
//! `nf_without_column_pass`).
#![cfg(feature = "internal-probes")]

use catgraph_applied::prop::presentation::smc_nf::{
    Atom, Layer, StringDiagram, fragment_status, from_string_diagram, nf, nf_without_column_pass,
};
use catgraph_applied::prop::{PropExpr, PropSignature, mono_word};
use catgraph_applied::rig::BoolRig;
use catgraph_applied::sfg::SfgGenerator;
use std::borrow::Cow;
use std::sync::mpsc;
use std::time::Duration;

type Sfg = SfgGenerator<BoolRig>;

// ============================================================================
// Test signatures
// ============================================================================

/// Shape-1 signature: derived `Ord` is `A < B < C`, so the closed block
/// `A;B` has in-situ reading `[Gen(A), Gen(B)]` which sorts *before* the
/// scalar's reading `[Gen(C)]`. Step 7 therefore wants the block LEFT of the
/// scalar; Step 6's class order (`scalar < η`) wants the scalar LEFT of the
/// block's η. Direct conflict on the same adjacency.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum EtaFirst {
    A, // 0 → 1 (η)
    B, // 1 → 0 (ε)
    C, // 0 → 0 (scalar)
}

impl PropSignature for EtaFirst {
    type Color = ();
    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
    fn source(&self) -> usize {
        match self {
            EtaFirst::A | EtaFirst::C => 0,
            EtaFirst::B => 1,
        }
    }
    fn target(&self) -> usize {
        match self {
            EtaFirst::A => 1,
            EtaFirst::B | EtaFirst::C => 0,
        }
    }
}

/// Control signature: same arities, but `Ord` is `C < A < B`, so the scalar's
/// reading `[Gen(C)]` sorts before the block's `[Gen(A), Gen(B)]` — Step 7 and
/// Step 6 *agree* (scalar left). No conflict; used to show the shape-1 fight
/// is exactly the `Ord`-dependent reading inversion.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum ScalarFirst {
    C, // 0 → 0 (scalar)
    A, // 0 → 1 (η)
    B, // 1 → 0 (ε)
}

impl PropSignature for ScalarFirst {
    type Color = ();
    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
    fn source(&self) -> usize {
        match self {
            ScalarFirst::A | ScalarFirst::C => 0,
            ScalarFirst::B => 1,
        }
    }
    fn target(&self) -> usize {
        match self {
            ScalarFirst::A => 1,
            ScalarFirst::B | ScalarFirst::C => 0,
        }
    }
}

/// Hunt signature for the enumerated divergence / livelock search. Derived
/// `Ord` = declaration order, so:
/// - closed blocks headed by `Ea`/`Eb` read *below* the scalars (`Ea < Sa`) —
///   the Step-7-vs-Step-6 conflict class;
/// - the `Ec`-headed block reads *above* the scalars (`Sa < Ec`) — the
///   agreement class.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Hunt {
    Ea, // 0 → 1
    Eb, // 0 → 1
    Ka, // 1 → 0
    Kb, // 1 → 0
    Sa, // 0 → 0
    Sb, // 0 → 0
    Ff, // 1 → 1
    Ec, // 0 → 1
    Kc, // 1 → 0
}

impl PropSignature for Hunt {
    type Color = ();
    fn source_word(&self) -> Cow<'_, [()]> {
        mono_word(self.source())
    }
    fn target_word(&self) -> Cow<'_, [()]> {
        mono_word(self.target())
    }
    fn source(&self) -> usize {
        match self {
            Hunt::Ea | Hunt::Eb | Hunt::Ec | Hunt::Sa | Hunt::Sb => 0,
            Hunt::Ka | Hunt::Kb | Hunt::Kc | Hunt::Ff => 1,
        }
    }
    fn target(&self) -> usize {
        match self {
            Hunt::Ea | Hunt::Eb | Hunt::Ec | Hunt::Ff => 1,
            Hunt::Ka | Hunt::Kb | Hunt::Kc | Hunt::Sa | Hunt::Sb => 0,
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn prim<G: PropSignature>(g: G) -> PropExpr<G> {
    PropExpr::Generator(g)
}
fn seq<G: PropSignature>(a: PropExpr<G>, b: PropExpr<G>) -> PropExpr<G> {
    PropExpr::Compose(Box::new(a), Box::new(b))
}
fn par<G: PropSignature>(a: PropExpr<G>, b: PropExpr<G>) -> PropExpr<G> {
    PropExpr::Tensor(Box::new(a), Box::new(b))
}

/// Same worker-stack size the differential sweep uses.
const STACK_BYTES: usize = 64 * 1024 * 1024;
/// Generous per-call budget; the witnesses are tiny, so a hit means the `nf`
/// fixpoint loop is livelocking, not that the machine is slow.
const NF_TIMEOUT: Duration = Duration::from_secs(20);

/// Run `nf` on a watchdog thread. A timeout is treated as a **nontermination
/// witness** and fails the test with the reproducer.
fn nf_timed<G>(e: &PropExpr<G>, tag: &str) -> StringDiagram<G>
where
    G: PropSignature + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let e2 = e.clone();
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(move || {
            let _ = tx.send(nf(&e2));
        })
        .expect("spawn nf worker");
    match rx.recv_timeout(NF_TIMEOUT) {
        Ok(sd) => sd,
        Err(_) => panic!(
            "NONTERMINATION witness ({tag}): nf did not return within {NF_TIMEOUT:?} on\n{e:?}"
        ),
    }
}

/// Assert every writing in an SMC-equal family reaches the same NF; return it.
fn family_nf<G>(tag: &str, writings: &[PropExpr<G>]) -> StringDiagram<G>
where
    G: PropSignature + Send + 'static,
{
    assert!(!writings.is_empty());
    let first = nf_timed(&writings[0], tag);
    for (i, w) in writings.iter().enumerate().skip(1) {
        let ni = nf_timed(w, tag);
        assert_eq!(
            first, ni,
            "DIVERGENCE ({tag}): writing 0 and writing {i} are SMC-equal but reach different NFs.\n\
             writing 0 = {:?}\nwriting {i} = {:?}",
            writings[0], w
        );
    }
    first
}

// ============================================================================
// §1-invariant checkers (replicated component analysis — the engine's
// versions are private, so the logic of `analyze_components`,
// `block_pair_is_free`, `blocks_are_adjacent`, `block_sorts_before` and the
// single-layer core of `find_column_transposition` is re-implemented here;
// smc_nf.rs:1542/2132/2152/1413/2583 are the references). `blocks_adjacent`
// below is the two-`contiguous_run` shape `blocks_are_adjacent` carried before
// #303; `blocks_are_adjacent` now routes through `adjacent_runs`, computing
// the same predicate.
// ============================================================================

mod check {
    use super::*;
    use std::cmp::Ordering;

    fn atom_src<G: PropSignature>(a: &Atom<G>) -> usize {
        match a {
            Atom::Identity(n) => *n,
            Atom::Braid(m, n) => m + n,
            Atom::Generator(g) => g.source(),
        }
    }
    fn atom_tgt<G: PropSignature>(a: &Atom<G>) -> usize {
        match a {
            Atom::Identity(n) => *n,
            Atom::Braid(m, n) => m + n,
            Atom::Generator(g) => g.target(),
        }
    }

    /// The identity-split refinement Step 7 / Step 6½ analyse on.
    pub fn explode<G: PropSignature + Clone>(sd: &StringDiagram<G>) -> StringDiagram<G> {
        StringDiagram {
            layers: sd
                .layers
                .iter()
                .map(|l| Layer {
                    atoms: l
                        .atoms
                        .iter()
                        .flat_map(|a| match a {
                            Atom::Identity(n) => vec![Atom::Identity(1); *n],
                            other => vec![other.clone()],
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub struct An {
        pub ids: Vec<Vec<usize>>,
        pub key: Vec<(u8, usize)>,
        pub size: Vec<usize>,
        pub inter: Vec<bool>,
        pub tin: Vec<bool>,
        pub tout: Vec<bool>,
        pub braid: Vec<bool>,
    }

    fn uf_find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    fn uf_union(parent: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (uf_find(parent, a), uf_find(parent, b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
        }
    }

    fn intervals<G: PropSignature>(atoms: &[Atom<G>], use_target: bool) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(atoms.len());
        let mut p = 0;
        for a in atoms {
            let w = if use_target { atom_tgt(a) } else { atom_src(a) };
            out.push((p, p + w));
            p += w;
        }
        out
    }

    fn owners(iv: &[(usize, usize)], comp: &[usize]) -> Vec<usize> {
        let mut out = Vec::new();
        for (i, (s, e)) in iv.iter().enumerate() {
            for _ in *s..*e {
                out.push(comp[i]);
            }
        }
        out
    }

    fn mark_interleaved(owner_word: &[usize], inter: &mut [bool]) {
        let mut runs: Vec<usize> = Vec::new();
        for &c in owner_word {
            if runs.last() != Some(&c) {
                runs.push(c);
            }
        }
        for (i, first) in runs.iter().enumerate() {
            for (k, second) in runs.iter().enumerate().skip(i + 1) {
                if first == second {
                    for &m in &runs[i..=k] {
                        inter[m] = true;
                    }
                }
            }
        }
    }

    /// Replica of `analyze_components` (incl. the zero-width emptiness guard)
    /// plus the braid-bearing flags.
    pub fn analyze<G: PropSignature>(sd: &StringDiagram<G>) -> An {
        let mut offsets = Vec::with_capacity(sd.layers.len() + 1);
        let mut total = 0;
        for l in &sd.layers {
            offsets.push(total);
            total += l.atoms.len();
        }
        offsets.push(total);
        let mut parent: Vec<usize> = (0..total).collect();
        for j in 1..sd.layers.len() {
            let ups = intervals(&sd.layers[j - 1].atoms, true);
            let downs = intervals(&sd.layers[j].atoms, false);
            let (mut a, mut b) = (0, 0);
            while a < ups.len() && b < downs.len() {
                let (a0, a1) = ups[a];
                let (b0, b1) = downs[b];
                if a0 < a1 && b0 < b1 && a0 < b1 && b0 < a1 {
                    uf_union(&mut parent, offsets[j - 1] + a, offsets[j] + b);
                }
                if a1 <= b1 {
                    a += 1;
                } else {
                    b += 1;
                }
            }
        }
        let roots: Vec<usize> = (0..total).map(|x| uf_find(&mut parent, x)).collect();
        let mut label = vec![usize::MAX; total];
        let mut ncomp = 0;
        for &r in &roots {
            if label[r] == usize::MAX {
                label[r] = ncomp;
                ncomp += 1;
            }
        }
        let comp_of: Vec<usize> = roots.iter().map(|&r| label[r]).collect();
        let mut size = vec![0usize; ncomp];
        for &c in &comp_of {
            size[c] += 1;
        }
        let mut in_min = vec![usize::MAX; ncomp];
        let mut out_min = vec![usize::MAX; ncomp];
        let mut inter = vec![false; ncomp];
        if let Some(first) = sd.layers.first() {
            let word = owners(&intervals(&first.atoms, false), &comp_of[offsets[0]..]);
            for (w, &c) in word.iter().enumerate() {
                in_min[c] = in_min[c].min(w);
            }
            mark_interleaved(&word, &mut inter);
        }
        if let Some(last) = sd.layers.last() {
            let off = offsets[sd.layers.len() - 1];
            let word = owners(&intervals(&last.atoms, true), &comp_of[off..]);
            for (w, &c) in word.iter().enumerate() {
                out_min[c] = out_min[c].min(w);
            }
            mark_interleaved(&word, &mut inter);
        }
        let key: Vec<(u8, usize)> = (0..ncomp)
            .map(|c| {
                if in_min[c] != usize::MAX {
                    (1, in_min[c])
                } else if out_min[c] != usize::MAX {
                    (2, out_min[c])
                } else {
                    (0, 0)
                }
            })
            .collect();
        let ids: Vec<Vec<usize>> = sd
            .layers
            .iter()
            .enumerate()
            .map(|(j, l)| comp_of[offsets[j]..offsets[j] + l.atoms.len()].to_vec())
            .collect();
        let mut braid = vec![false; ncomp];
        for (row, layer) in ids.iter().zip(sd.layers.iter()) {
            for (&c, atom) in row.iter().zip(layer.atoms.iter()) {
                if matches!(atom, Atom::Braid(_, _)) {
                    braid[c] = true;
                }
            }
        }
        An {
            ids,
            key,
            size,
            inter,
            tin: in_min.iter().map(|&v| v != usize::MAX).collect(),
            tout: out_min.iter().map(|&v| v != usize::MAX).collect(),
            braid,
        }
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum RA<'a, G: PropSignature> {
        Id(usize),
        Braid(usize, usize),
        Gen(&'a G),
    }

    fn reading<'a, G: PropSignature>(
        sd: &'a StringDiagram<G>,
        an: &An,
        c: usize,
    ) -> Vec<RA<'a, G>> {
        let mut out = Vec::new();
        for (row, layer) in an.ids.iter().zip(sd.layers.iter()) {
            for (&owner, atom) in row.iter().zip(layer.atoms.iter()) {
                if owner == c {
                    out.push(match atom {
                        Atom::Identity(n) => RA::Id(*n),
                        Atom::Braid(m, n) => RA::Braid(*m, *n),
                        Atom::Generator(g) => RA::Gen(g),
                    });
                }
            }
        }
        out
    }

    fn contiguous_run(row: &[usize], c: usize) -> Option<(usize, usize)> {
        let first = row.iter().position(|&x| x == c)?;
        let last = row
            .iter()
            .rposition(|&x| x == c)
            .expect("first implies last");
        row[first..=last]
            .iter()
            .all(|&x| x == c)
            .then_some((first, last + 1))
    }

    fn blocks_adjacent(ids: &[Vec<usize>], c1: usize, c2: usize) -> bool {
        for row in ids {
            if !row.contains(&c1) || !row.contains(&c2) {
                continue;
            }
            let (Some((_, b1)), Some((a2, _))) = (contiguous_run(row, c1), contiguous_run(row, c2))
            else {
                return false;
            };
            if b1 != a2 {
                return false;
            }
        }
        true
    }

    fn free_pair(an: &An, c1: usize, c2: usize) -> bool {
        c1 != c2
            && (an.size[c1] > 1 || an.size[c2] > 1)
            && !an.inter[c1]
            && !an.inter[c2]
            && !(an.tin[c1] && an.tin[c2])
            && !(an.tout[c1] && an.tout[c2])
    }

    fn block_sorts_before<G: PropSignature>(
        sd: &StringDiagram<G>,
        an: &An,
        b: usize,
        a: usize,
    ) -> bool {
        match an.key[b].cmp(&an.key[a]) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => reading(sd, an, b) < reading(sd, an, a),
        }
    }

    /// §1 Step-7 clause: adjacent *free* pairs ordered against the block order
    /// (rule-(i) key, then in-situ reading). Returns `(layer, c1, c2)` for each
    /// violation. Run on the exploded refinement.
    pub fn step7_violations<G: PropSignature>(sd: &StringDiagram<G>) -> Vec<(usize, usize, usize)> {
        let an = analyze(sd);
        let mut out = Vec::new();
        for (j, row) in an.ids.iter().enumerate() {
            for w in row.windows(2) {
                let (c1, c2) = (w[0], w[1]);
                if c1 != c2
                    && !an.braid[c1]
                    && !an.braid[c2]
                    && free_pair(&an, c1, c2)
                    && blocks_adjacent(&an.ids, c1, c2)
                    && block_sorts_before(sd, &an, c2, c1)
                    && !out.contains(&(j, c1, c2))
                {
                    out.push((j, c1, c2));
                }
            }
        }
        out
    }

    /// §1 Step-6½ clause, restricted to single-layer intervals (a sound
    /// under-approximation: `find_column_transposition` always tries the
    /// length-1 sub-interval containing the seed, so anything flagged here is
    /// a pair the shipped pass would transpose). Returns `(layer, c1, c2)`.
    ///
    /// **Column reading (symmetric since 2026-08-02, #185).** The two columns
    /// are the two maximal **local** runs meeting at the adjacency — `c1`'s run
    /// ending at `p`, `c2`'s run starting at `p + 1` — which is
    /// `adjacent_column_cuts_at`'s predicate verbatim. It used to read `c2`'s
    /// *whole* layer presence via `contiguous_run`, mirroring the pre-#185 seed
    /// test, and so was blind to exactly the split-presence violations #185
    /// fixed (`[L, B, L]` rows): a checker that could not see the bug it is
    /// meant to guard against. Extending it moved no existing probe's verdict
    /// (whole suite re-run 2026-08-02, 7/7 green); the newly visible case is
    /// `split_presence_both_readings_pair_is_newly_seeded`, which the
    /// whole-presence form could not flag.
    pub fn column_violations_len1<G: PropSignature>(
        sd: &StringDiagram<G>,
    ) -> Vec<(usize, usize, usize)> {
        let an = analyze(sd);
        let mut out = Vec::new();
        for (j, row) in an.ids.iter().enumerate() {
            let layer = &sd.layers[j];
            for p in 0..row.len().saturating_sub(1) {
                let (c1, c2) = (row[p], row[p + 1]);
                let admissible = c1 != c2
                    && (an.size[c1] > 1 || an.size[c2] > 1)
                    && !an.inter[c1]
                    && !an.inter[c2]
                    && !an.braid[c1]
                    && !an.braid[c2];
                // Strictly `Less` only — the closed↔closed equal-key tie is
                // declined by the pass, so it is not a violation.
                if !admissible || an.key[c2].cmp(&an.key[c1]) != Ordering::Less {
                    continue;
                }
                // The two maximal *local* runs meeting at (p, p+1): neither
                // component's whole presence in the row is consulted (#185).
                let mid = p + 1;
                let mut left = p;
                while left > 0 && row[left - 1] == c1 {
                    left -= 1;
                }
                let mut right = mid;
                while right < row.len() && row[right] == c2 {
                    right += 1;
                }
                let width = |r: std::ops::Range<usize>, tgt: bool| -> usize {
                    layer.atoms[r]
                        .iter()
                        .map(|a| if tgt { atom_tgt(a) } else { atom_src(a) })
                        .sum()
                };
                let (sx, sb) = (width(left..mid, false), width(mid..right, false));
                let (tx, tb) = (width(left..mid, true), width(mid..right, true));
                if (sx == 0 || sb == 0) && (tx == 0 || tb == 0) && !out.contains(&(j, c1, c2)) {
                    out.push((j, c1, c2));
                }
            }
        }
        out
    }
}

// ============================================================================
// Shape 1 — scalar ↔ closed multi-atom block, adversarial `G::Ord`
// (test signature; no shipped signature has a 0→0 generator)
// ============================================================================

/// The conflicted adjacency: Step 6 (class order) wants the scalar `C` left of
/// the block's η `A`; Step 7 (equal closed keys → reading key, `[A,B] < [C]`)
/// wants the block left of the scalar. Verified behaviour: `nf` terminates,
/// every writing converges, and the fixpoint keeps the scalar LEFT — an
/// adjacent free pair ordered **against** the Step-7 order, violating the §1
/// post-`nf` invariant clause.
#[test]
fn shape1_conflict_terminates_converges_but_violates_step7_invariant() {
    use EtaFirst::{A, B, C};
    let block = || seq(prim(A), prim(B));
    let writings = vec![
        par(block(), prim(C)),
        par(prim(C), block()),
        seq(par(prim(A), prim(C)), prim(B)), // interchange slicing
        par(block(), seq(PropExpr::Identity(0), prim(C))), // scalar layer-shifted
    ];
    let out = family_nf("shape1", &writings);

    // Fixpoint: scalar left (Step 6 wins the standing layout).
    let expected = StringDiagram {
        layers: vec![
            Layer {
                atoms: vec![Atom::Generator(C), Atom::Generator(A)],
            },
            Layer {
                atoms: vec![Atom::Generator(B)],
            },
        ],
    };
    assert_eq!(out, expected, "unexpected shape-1 fixpoint layout");

    // Idempotent, and the Step-7-canonical writing of the same morphism is
    // actively rewritten INTO the violating layout.
    assert_eq!(out, nf_timed(&from_string_diagram(&out), "shape1-idem"));
    let step7_canonical = StringDiagram {
        layers: vec![
            Layer {
                atoms: vec![Atom::Generator(A), Atom::Generator(C)],
            },
            Layer {
                atoms: vec![Atom::Generator(B)],
            },
        ],
    };
    assert_eq!(
        out,
        nf_timed(
            &from_string_diagram(&step7_canonical),
            "shape1-from-canonical"
        ),
        "nf should map the rule-(i)+reading-canonical layout to the Step-6 layout"
    );

    // The §1 Step-7 invariant clause is violated at the fixpoint.
    let refined = check::explode(&out);
    let v = check::step7_violations(&refined);
    assert!(
        !v.is_empty(),
        "expected a Step-7 order violation at the shape-1 fixpoint; found none"
    );
}

/// Control: flip the derived `Ord` so the scalar's reading sorts first. Step 6
/// and Step 7 then agree, no cycling, and the (same) scalar-left fixpoint
/// satisfies the invariant.
#[test]
fn shape1_control_agreeing_ord_has_no_violation() {
    use ScalarFirst::{A, B, C};
    let block = || seq(prim(A), prim(B));
    let writings = vec![par(block(), prim(C)), par(prim(C), block())];
    let out = family_nf("shape1-control", &writings);
    let refined = check::explode(&out);
    assert!(check::step7_violations(&refined).is_empty());
    assert!(check::column_violations_len1(&refined).is_empty());
}

// ============================================================================
// Shape 2 — η ↔ ε across components, SHIPPED generators only
// ============================================================================

/// `Discard ⊗ (Zero ; Scalar)` over `BoolRig`. The tied adjacency is
/// `Discard` (ε, input-anchored `{D}`, key (1,0)) against `Zero` (η, of the
/// output-only multi-atom `{Zero, Scalar}`, key (2,0)). Step 6: η < ε → Zero
/// left. Step 7 (free pair: input-only ∥ output-only, one multi-atom): key
/// (1,0) < (2,0) → Discard left. The η sits in the TOP layer, so the
/// point-span sift can never dissolve the tie; `nf` cycles (Step 7 swaps,
/// Step 6 swaps back), exits by exact cancellation, and the fixpoint violates
/// both the Step-7 and (keys distinct) the Step-6½ §1 clauses — with shipped
/// generators, inside the fragment `𝔉`.
#[test]
fn shape2_shipped_eta_eps_conflict_fixpoint_violates_invariants() {
    let d = || prim::<Sfg>(SfgGenerator::Discard);
    let z = || prim::<Sfg>(SfgGenerator::Zero);
    let s = || prim::<Sfg>(SfgGenerator::Scalar(BoolRig(true)));
    let writings = vec![
        par(d(), seq(z(), s())),
        par(seq(z(), s()), d()), // σ_{0,1} = id: same morphism on the nose
        seq(par(PropExpr::Identity(1), z()), par(d(), s())), // interchange
        seq(par(d(), z()), s()),
    ];
    let out = family_nf("shape2", &writings);

    let expected = StringDiagram {
        layers: vec![
            Layer {
                atoms: vec![
                    Atom::Generator(SfgGenerator::Zero),
                    Atom::Generator(SfgGenerator::Discard),
                ],
            },
            Layer {
                atoms: vec![Atom::Generator(SfgGenerator::Scalar(BoolRig(true)))],
            },
        ],
    };
    assert_eq!(out, expected, "unexpected shape-2 fixpoint layout");
    assert!(
        fragment_status(&out).in_fragment(),
        "shape-2 witness should live inside 𝔉"
    );

    let refined = check::explode(&out);
    let v7 = check::step7_violations(&refined);
    let v6h = check::column_violations_len1(&refined);
    assert!(
        !v7.is_empty(),
        "expected a Step-7 order violation at the shape-2 fixpoint"
    );
    assert!(
        !v6h.is_empty(),
        "expected a Step-6½ order violation at the shape-2 fixpoint (keys (1,0) vs (2,0) are distinct — the closed↔closed carve does not apply)"
    );
    assert_eq!(out, nf_timed(&from_string_diagram(&out), "shape2-idem"));
}

/// Step 6 vs Step 6½ **with Step 7 structurally blocked** (both components
/// touch the input boundary, so no free pair exists): `(Zero ⊗ Discard ⊗
/// Scalar) ; Add`. Components: `{Discard}` key (1,0) vs `{Zero, Scalar, Add}`
/// key (1,1) — distinct keys, so the §4.4 closed↔closed carve is silent, and
/// Step 6½ transposes the `Zero | Discard` columns toward key order while
/// Step 6 orders the same two atoms η-before-ε. Exact cancellation again;
/// the fixpoint violates the §1 Step-6½ clause. This directly refutes §4.4's
/// "that carve is what keeps Step 6's class fallback from fighting Step 6½
/// over the same adjacency" — the carve only covers equal keys.
#[test]
fn shipped_step6_vs_step6half_distinct_keys_conflict() {
    let d = || prim::<Sfg>(SfgGenerator::Discard);
    let z = || prim::<Sfg>(SfgGenerator::Zero);
    let s = || prim::<Sfg>(SfgGenerator::Scalar(BoolRig(true)));
    let add = || prim::<Sfg>(SfgGenerator::Add);
    let writings = vec![
        seq(par(z(), par(d(), s())), add()),
        seq(par(par(z(), d()), s()), add()),
        seq(par(d(), par(z(), s())), add()), // slide Zero across Discard (σ_{0,1} = id)
    ];
    let out = family_nf("step6-vs-6half", &writings);

    let expected = StringDiagram {
        layers: vec![
            Layer {
                atoms: vec![
                    Atom::Generator(SfgGenerator::Zero),
                    Atom::Generator(SfgGenerator::Discard),
                    Atom::Generator(SfgGenerator::Scalar(BoolRig(true))),
                ],
            },
            Layer {
                atoms: vec![Atom::Generator(SfgGenerator::Add)],
            },
        ],
    };
    assert_eq!(out, expected, "unexpected 6-vs-6½ fixpoint layout");
    assert!(fragment_status(&out).in_fragment());

    let refined = check::explode(&out);
    // Step 7 clause is NOT violated (no free pair — both components touch the
    // input boundary)…
    assert!(
        check::step7_violations(&refined).is_empty(),
        "step 7 should be structurally blocked here"
    );
    // …but the Step-6½ clause IS.
    assert!(
        !check::column_violations_len1(&refined).is_empty(),
        "expected a Step-6½ order violation (distinct keys (1,0) vs (1,1))"
    );

    // The fight is externally cancellation-exact: ablating the column pass
    // gives the identical NF (Step 6 alone lands on the same layout, without
    // the counter-swap). The conflict is invisible in output — it exists as a
    // 6½-swap + 6-swap-back cycle inside every pass until `sd == prev`.
    for w in &writings {
        assert_eq!(out, nf_without_column_pass(w), "ablation should agree");
    }
}

/// Mechanism variety: a *transient* both-readings adjacency that dissolves
/// instead of deadlocking. `(Sa ⊗ Sb) ; (Discard ⊗ Zero ⊗ Sb2) ; Add`-shaped
/// writings put `Zero` (η, of the input+output component through `Add`) tied
/// against `Discard` (ε, of `{Sa, Discard}`, key (1,0) < (1,1)) mid-layer.
/// Here the point-span sift lifts the η out of the tied layer before the
/// fight can stand, and the 2-layer Step-6½ column move then carries the
/// `{Sa, Discard}` column left past the `Zero` column — geometry overriding
/// the class order on an adjacency that briefly admitted both readings, with
/// a clean (violation-free) fixpoint. Shows the deadlock-vs-dissolve
/// distinction is incidental geometry (top-layer η vs siftable η), not a
/// disjointness property.
#[test]
fn shipped_transient_tie_dissolves_via_sift_and_column_move() {
    let d = || prim::<Sfg>(SfgGenerator::Discard);
    let z = || prim::<Sfg>(SfgGenerator::Zero);
    let st = || prim::<Sfg>(SfgGenerator::Scalar(BoolRig(true)));
    let sf = || prim::<Sfg>(SfgGenerator::Scalar(BoolRig(false)));
    let add = || prim::<Sfg>(SfgGenerator::Add);
    let id1 = || PropExpr::Identity(1);
    let writings = vec![
        // Zero in the top layer, left of everything.
        seq(
            seq(par(par(z(), st()), sf()), par(par(id1(), d()), sf())),
            add(),
        ),
        // Zero mid-layer, left of the tied Discard.
        seq(seq(par(st(), sf()), par(par(z(), d()), sf())), add()),
        // Zero mid-layer, right of the tied Discard.
        seq(seq(par(st(), sf()), par(par(d(), z()), sf())), add()),
    ];
    let out = family_nf("transient-tie", &writings);
    assert!(fragment_status(&out).in_fragment());
    let refined = check::explode(&out);
    assert!(
        check::step7_violations(&refined).is_empty()
            && check::column_violations_len1(&refined).is_empty(),
        "the dissolved-tie fixpoint should satisfy both §1 order clauses, got {out:?}"
    );
}

/// **P1 — the #185 newly-seeded both-readings witness** (added 2026-08-02).
///
/// `Copy ; (id₁ ⊗ Zero ⊗ Discard) ; (id₁ ⊗ Scalar)` over `BoolRig`. The middle
/// layer's exploded row is `[Id_L, Zero_B, Discard_L]`: the input-anchored
/// component `L = {Copy, Id, Discard, Id}` (key `(1, 0)`) has **split** layer
/// presence around the output-only `B = {Zero, Scalar}` (key `(2, 1)`). The
/// contested adjacency is the `(B, L)` one at position 1 — `Zero | Discard`,
/// which strictly commutes (`src Zero = 0`, `tgt Discard = 0`) — so it admits
/// **both readings**: Step 6's class order wants η before ε (`Zero` left),
/// Step 6½'s key order wants `L`'s column left of `B`'s.
///
/// Why this probe exists: the pre-#185 seed test demanded the *right*
/// component's whole layer presence be one contiguous run, and `L`'s is
/// `{0, 2}`, so it declined this adjacency outright. Symmetric local runs seed
/// it. #185's seeding is therefore strictly **wider**, and the both-readings
/// carve and the `sd == prev` exact-cancellation exit
/// ([#186](https://github.com/sustia-llc/catgraph/issues/186)) are exercised at
/// sites the old engine never reached — this is one. What the carve promises is
/// that this costs nothing at the fixpoint, and that is what is pinned here:
/// `nf` terminates, the two SMC-equal writings (the second slides `Zero` across
/// `Discard`, `σ_{0,1} = id`) converge, the fixpoint is idempotent, and it is
/// the **Step-6-sorted** layout — `Zero` left of `Discard`, the class order
/// winning exactly as §4.4 ratifies.
///
/// The layout below is pinned as measured on this tree. The #185 adversarial
/// review (2026-08-02) additionally measured it **equal to the pre-#185
/// fixpoint** — recorded here as that review's measurement, not as a live
/// assertion: the old engine is not in-tree, so nothing here can re-check it.
///
/// *Scope (measured 2026-08-02, deliberately not asserted).* The `σ_{0,1}`-slid
/// writing `Copy ; (id₁ ⊗ Discard ⊗ Zero) ; (id₁ ⊗ Scalar)` is the same morphism
/// but **diverges** — it reaches the 2-layer `[Copy, Zero] ; [Id, Discard,
/// Scalar]`, having sifted the `η` up a layer. That is `η`-placement slack
/// (§4.6's residual `ι`, the hypothesis Theorem 4.5 excludes), not this
/// adjacency's doing: `nf_without_column_pass` reproduces **both** normal forms
/// verbatim, so the column pass is not a party to the split, and P1 claims
/// nothing about it.
#[test]
fn split_presence_both_readings_pair_is_newly_seeded() {
    let c = || prim::<Sfg>(SfgGenerator::Copy);
    let d = || prim::<Sfg>(SfgGenerator::Discard);
    let z = || prim::<Sfg>(SfgGenerator::Zero);
    let s = || prim::<Sfg>(SfgGenerator::Scalar(BoolRig(true)));
    let id1 = || PropExpr::Identity(1);
    let writing = seq(seq(c(), par(id1(), par(z(), d()))), par(id1(), s()));
    let out = nf_timed(&writing, "p1-split-presence");

    // Step-6-sorted: `Zero` (η) left of `Discard` (ε) in the contested layer,
    // the class order winning over Step 6½'s key order (1, 0) < (2, 1).
    let expected = StringDiagram {
        layers: vec![
            Layer {
                atoms: vec![Atom::Generator(SfgGenerator::Copy)],
            },
            Layer {
                atoms: vec![
                    Atom::Identity(1),
                    Atom::Generator(SfgGenerator::Zero),
                    Atom::Generator(SfgGenerator::Discard),
                ],
            },
            Layer {
                atoms: vec![
                    Atom::Identity(1),
                    Atom::Generator(SfgGenerator::Scalar(BoolRig(true))),
                ],
            },
        ],
    };
    assert_eq!(out, expected, "unexpected P1 fixpoint layout");
    assert!(
        fragment_status(&out).in_fragment(),
        "P1 should live inside 𝔉"
    );
    assert_eq!(out, nf_timed(&from_string_diagram(&out), "p1-idem"));

    // The fight is externally cancellation-exact: ablating the column pass
    // reaches the identical NF, so #185's wider seeding costs nothing here.
    assert_eq!(
        out,
        nf_without_column_pass(&writing),
        "the newly-seeded both-readings pair should cancel exactly"
    );

    let refined = check::explode(&out);
    // Step 7 is structurally blocked (both components touch the output
    // boundary, so no free pair)…
    assert!(
        check::step7_violations(&refined).is_empty(),
        "step 7 should be structurally blocked here"
    );
    // …and the fixpoint violates the *unrestated* §1 Step-6½ clause, which the
    // both-readings carve excepts. Seeing it at all needs the symmetric column
    // reading: the pre-#185 whole-presence checker declined this row, because
    // `L`'s presence `{0, 2}` is not one contiguous run.
    assert_eq!(
        check::column_violations_len1(&refined),
        vec![(1, 1, 0)],
        "expected the split-presence Step-6½ order violation in the middle layer"
    );
}

// ============================================================================
// Enumerated divergence / livelock hunt over scalar-and-block families
// ============================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Strand {
    C1,   // closed block Ea;Ka         (reading below scalars → conflict)
    C2,   // closed block Eb;Kb         (conflict)
    C3,   // closed block Ea;Ff;Ka      (3-layer, conflict)
    C4,   // closed block Ec;Kc         (reading above scalars → agreement)
    S1,   // scalar Sa
    S2,   // scalar Sb
    AEta, // anchored η strand Eb (output wire)
    AEps, // anchored ε strand Ka (input wire)
    Wire, // solid Ff (input+output)
}

impl Strand {
    fn closed(self) -> bool {
        matches!(
            self,
            Strand::C1 | Strand::C2 | Strand::C3 | Strand::C4 | Strand::S1 | Strand::S2
        )
    }
    fn expr(self, shift: bool) -> PropExpr<Hunt> {
        let base = match self {
            Strand::C1 => seq(prim(Hunt::Ea), prim(Hunt::Ka)),
            Strand::C2 => seq(prim(Hunt::Eb), prim(Hunt::Kb)),
            Strand::C3 => seq(prim(Hunt::Ea), seq(prim(Hunt::Ff), prim(Hunt::Ka))),
            Strand::C4 => seq(prim(Hunt::Ec), prim(Hunt::Kc)),
            Strand::S1 => prim(Hunt::Sa),
            Strand::S2 => prim(Hunt::Sb),
            Strand::AEta => prim(Hunt::Eb),
            Strand::AEps => prim(Hunt::Ka),
            Strand::Wire => prim(Hunt::Ff),
        };
        if shift && self.closed() {
            seq(PropExpr::Identity(0), base) // one-layer downward shift: Id₀ ; s
        } else {
            base
        }
    }
}

fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut out = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let head = rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, head.clone());
            out.push(tail);
        }
    }
    out
}

/// All SMC-equal writings of a strand multiset: permutations that keep the
/// anchored strands' relative order (closed strands — 0→0 blocks and scalars —
/// commute with everything on the nose, σ_{0,n} = id), each further varied by
/// per-closed-strand one-layer shifts (interchange).
fn family_writings(strands: &[Strand]) -> Vec<PropExpr<Hunt>> {
    let anchored: Vec<Strand> = strands.iter().copied().filter(|s| !s.closed()).collect();
    let mut out = Vec::new();
    for perm in permutations(strands) {
        let pa: Vec<Strand> = perm.iter().copied().filter(|s| !s.closed()).collect();
        if pa != anchored {
            continue;
        }
        let closed_idx: Vec<usize> = (0..perm.len()).filter(|&i| perm[i].closed()).collect();
        for mask in 0u32..(1 << closed_idx.len()) {
            let exprs: Vec<PropExpr<Hunt>> = perm
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let shift = closed_idx
                        .iter()
                        .position(|&k| k == i)
                        .is_some_and(|bit| mask & (1 << bit) != 0);
                    s.expr(shift)
                })
                .collect();
            out.push(
                exprs
                    .into_iter()
                    .reduce(par)
                    .expect("non-empty strand list"),
            );
        }
    }
    out
}

/// Exhaustive convergence + termination check over conflict-rich strand
/// families. Any panic here is either a nontermination reproducer (watchdog)
/// or an SMC-equal divergence reproducer (family_nf's assert). The invariant
/// checkers run on every family NF; violations are *expected* (the obligation
/// is false) and counted, not failed.
#[test]
fn hunt_no_divergence_no_livelock_across_conflict_families() {
    use Strand::*;
    let catalog: Vec<(&str, Vec<Strand>)> = vec![
        ("c1+s1", vec![C1, S1]),
        ("c1+s1+s2", vec![C1, S1, S2]),
        ("c1+c2+s1", vec![C1, C2, S1]),
        ("c1+s1+wire", vec![C1, S1, Wire]),
        ("c1+s1+aeps", vec![C1, S1, AEps]),
        ("c1+s1+aeta", vec![C1, S1, AEta]),
        ("c3+s1", vec![C3, S1]),
        ("c1+c2+s1+wire", vec![C1, C2, S1, Wire]),
        ("aeta+aeps+s1", vec![AEta, AEps, S1]),
        ("c3+c1+s1", vec![C3, C1, S1]),
        ("c1+s1+aeta+aeps", vec![C1, S1, AEta, AEps]),
        ("c2+c1", vec![C2, C1]),
        ("c4+s1", vec![C4, S1]),
        ("c1+c4+s1", vec![C1, C4, S1]),
        ("c3+c2+s1", vec![C3, C2, S1]),
        ("c1+c2+s1+s2", vec![C1, C2, S1, S2]),
        ("wire+c1+s1+wire", vec![Wire, C1, S1, Wire]),
        ("c1+c2+c4+s1", vec![C1, C2, C4, S1]),
        ("aeps+c1+s1+aeta", vec![AEps, C1, S1, AEta]),
    ];
    let (mut families, mut writings_total) = (0usize, 0usize);
    let (mut v7_families, mut v6h_families) = (Vec::new(), Vec::new());
    for (tag, strands) in &catalog {
        let ws = family_writings(strands);
        writings_total += ws.len();
        families += 1;
        let out = family_nf(tag, &ws);
        let refined = check::explode(&out);
        if !check::step7_violations(&refined).is_empty() {
            v7_families.push(*tag);
        }
        if !check::column_violations_len1(&refined).is_empty() {
            v6h_families.push(*tag);
        }
        // Idempotence on the family NF.
        assert_eq!(
            out,
            nf_timed(&from_string_diagram(&out), tag),
            "not idempotent on {tag}"
        );
    }
    println!(
        "hunt: {families} families / {writings_total} writings — all converged, none timed out;\n\
         §1 Step-7 clause violated at {} family fixpoints: {v7_families:?}\n\
         Step-6½ clause violated at {}: {v6h_families:?}",
        v7_families.len(),
        v6h_families.len()
    );
    // The hunt's *finding* is the violation counts above; convergence and
    // termination are the assertions. Sanity: the conflict families must
    // actually exhibit the violation (otherwise the probes test nothing).
    assert!(
        !v7_families.is_empty(),
        "expected at least one Step-7-invariant-violating fixpoint in the catalog"
    );
}
