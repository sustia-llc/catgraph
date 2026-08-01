//! Shape-axis baseline for the haft `Free` / `Cofree` carriers (issue #156).
//!
//! `catgraph-dl` had no `benches/` target before this file. This is the
//! **Phase 1** baseline the issue asks for: latency *and* allocation counts for
//! the recursive carriers across a designed shape axis, so a later regression
//! (or the #74 AD path, or the #36 lazy surfaces feeding something real) has
//! something to be measured against.
//!
//! **Scope fence.** Phase 2 of #156 (re-instating the retired native carriers at
//! `65a1103^` for a side-by-side) did **not** run. Nothing here is, or may be
//! read as, a native-vs-haft performance claim. Every number below characterises
//! the *current* encoding only.
//!
//! ## Why shape is the axis
//!
//! haft places the recursion indirection *inside* the functor hole —
//! `Free::Suspend(F::Type<Box<Free<F, A>>>)`, `Cofree { tail: F::Type<Box<…>> }`.
//! So the allocation count of a carrier value is exactly **one `Box` per
//! recursive hole**, and the hole count is a function of the shape:
//!
//! | witness | `Type<X>` | holes per node |
//! |---|---|---|
//! | [`ListEndo<A>`] | `Option<(A, X)>` | 1 (`Some`), 0 (`None`) |
//! | [`TreeEndo<A>`] | `Either<A, (X, X)>` | 2 (`Right`), 0 (`Left`) |
//!
//! ## Finding: `TreeEndo`'s leaf ratio is not a free parameter
//!
//! The issue's shape axis names a "branch-dense (few `Left(A)` leaves)" case and
//! a "leaf-dense (high `Left(A)` ratio)" case as if the leaf ratio could be
//! tuned. It cannot. `TreeEndo<A>::Type<X> = Either<A, (X, X)>` is a **strict**
//! binary tree: every internal node has exactly two children, so for any shape
//! whatsoever
//!
//! ```text
//! leaves = internal + 1        and        Box allocations = 2 · (leaves − 1)
//! ```
//!
//! The `Left(A)` ratio is therefore pinned at just under ½ for every tree, and
//! two `TreeEndo` trees with the same leaf count allocate identically **no
//! matter how they are shaped**. (`Free`'s other leaf former, `Pure(z)`, is
//! unboxed too, so mixing terminators in does not move the count either.)
//!
//! What *is* tunable is leaves-per-unit-depth, which is what the two tree
//! fixtures below bracket — they are the two extremes of that ratio at a fixed
//! leaf count:
//!
//! - **`tree_balanced`** — the bushy extreme. `L = 2^d` leaves at depth `d`;
//!   maximum leaves per unit depth. This is the issue's "leaf-dense" case.
//! - **`tree_spine`** — the degenerate extreme. A left caterpillar: `L` leaves
//!   at depth `L − 1`; minimum leaves per unit depth, i.e. deep branching with
//!   as few leaves as the encoding permits. This is the issue's "branch-dense"
//!   case.
//!
//! Both allocate `2·(L − 1)` boxes at equal `L`; the measured spread between
//! them is pure locality and recursion-depth cost, not encoding cost. Reporting
//! that flatness *is* part of the baseline.
//!
//! ## Allocation accounting
//!
//! A `#[global_allocator]` wrapper over `System` counts `alloc` / `alloc_zeroed`
//! / `realloc` / `dealloc` calls and allocated bytes. Counts are collected in an
//! **explicit single-threaded pass that runs before criterion is constructed**
//! (see `main`), never inside a criterion timing loop — criterion's own
//! bookkeeping allocates, and folding that into the numbers would make them
//! meaningless. Criterion is used purely for latency.
//!
//! The counters stay armed for the whole process, so every latency figure here
//! carries one relaxed `fetch_add` per allocation. That overhead is uniform
//! across shapes, so cross-shape comparison is unaffected; absolute wall-clock
//! numbers are a hair pessimistic against an uninstrumented build.
//!
//! `bytes` sums the requested `Layout::size()` of every `alloc`/`alloc_zeroed`
//! plus the *new* size of every `realloc` — it is bytes-requested, not
//! peak-resident.
//!
//! This bench file is the only place in `catgraph-dl` containing `unsafe`; the
//! library crate keeps its `#![forbid(unsafe_code)]`. The unsafe surface is the
//! three-method `GlobalAlloc` forward to `System`, each with a `// SAFETY:`
//! note.
//!
//! ## What is measured
//!
//! | group | operation |
//! |---|---|
//! | `carrier::construct` | the bijection helpers — [`vec_to_free_mnd`], [`tree_to_free_mnd`] |
//! | `carrier::fold` | `Free::fold` — the CDL Remark 2.13 catamorphism |
//! | `carrier::unfold` | `Cofree::unfold` — the #64 anamorphism |
//! | `lazy::iter` | the #36 / PR #151 lazy surfaces + their eager sibling |
//!
//! **The lazy surfaces do not touch the carriers.** `UnfoldingRnn::unroll_iter`,
//! `MealyCell::run_iter` and `MooreCell::run_iter` are `core::iter::from_fn`
//! state machines over the coalgebra — they never build a `Free` or a `Cofree`.
//! The issue lists them under the same axis, so they are measured here, but they
//! belong to the `architectures` layer and their allocation counts are constant
//! in the step count (the eager `unroll_to_vec` sibling is benched alongside
//! precisely to show what the lazy surface avoids).
//!
//! ## Determinism
//!
//! Shapes are constructive and exact; the `f64` payloads come from
//! `catgraph_testutil::Lcg` at the per-file [`SEED`] (#33's byte-identity
//! contract), so both the latency and the allocation pass are reproducible.
//!
//! ## Measured baseline
//!
//! Recorded 2026-08-01 (`--sample-size 10 --warm-up-time 1
//! --measurement-time 3`, `bench` profile, maintainer's dev workstation). The
//! full shape × op table lives in
//! `.claude/docs/2026-08-01-156-dl-bench-baseline.md`; latencies there are
//! machine-relative, the allocation counts are exact. Headlines:
//!
//! - **Allocation count is exactly the hole count.** `1·n` boxes for a list of
//!   `n` cons cells (24 B each); `2·(L − 1)` for a tree of `L` leaves (16 B
//!   each on `Free`, 24 B on `Cofree` — the extra 8 B is the `head: A` label).
//!   Zero `realloc`s anywhere.
//! - **`Free::fold` allocates nothing** — 0 allocations in every row; its whole
//!   cost is the fmap-and-recurse walk plus tearing down the program it eats.
//!   It is also the cheapest carrier op (~51 Melem/s on lists vs ~30 for
//!   construction). `Cofree::unfold` is the most expensive (~10–11 Melem/s on
//!   trees) — it allocates *and* calls the coalgebra at every node.
//! - **Shape costs latency, not memory.** `tree_spine` and `tree_balanced`
//!   allocate identically at equal leaf count (as the identity above forces)
//!   while the spine runs 2.4–18 % slower, the gap broadly widening with size.
//! - **All three lazy surfaces are allocation-free** at every size; the eager
//!   `unroll_to_vec` sibling takes exactly one `Vec::with_capacity` (8 B/step,
//!   no regrowth), so the lazy saving is one allocation and 2–14 % of wall
//!   time — not an asymptotic difference. `unroll_iter` earns its keep through
//!   unboundedness (#36's motivation), not allocation pressure.
//!
//! [`ListEndo<A>`]: catgraph_dl::free_monad::list_endo::ListEndo
//! [`TreeEndo<A>`]: catgraph_dl::free_monad::tree_endo::TreeEndo
//! [`vec_to_free_mnd`]: catgraph_dl::free_monad::list_endo::vec_to_free_mnd
//! [`tree_to_free_mnd`]: catgraph_dl::free_monad::tree_endo::tree_to_free_mnd

use core::convert::Infallible;
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group};

use catgraph_dl::Either;
use catgraph_dl::architectures::{MealyCell, MooreCell, UnfoldingRnn};
use catgraph_dl::free_monad::Cofree;
use catgraph_dl::free_monad::list_endo::{ListEndo, vec_to_free_mnd};
use catgraph_dl::free_monad::tree_endo::{BinaryTree, TreeEndo, tree_to_free_mnd};
use catgraph_testutil::Lcg;

/// Per-file seed for the LCG payload stream (#33 byte-identity contract).
const SEED: u64 = 0x0000_0000_0000_009C | 1;

/// Cons-cell counts for the `ListEndo` (single-hole) fixtures.
const LIST_SIZES: [usize; 3] = [64, 1024, 4096];

/// Leaf counts for both `TreeEndo` (two-hole) fixtures. `tree_balanced` needs
/// powers of two; `tree_spine` uses the same values so the two shapes are
/// directly comparable at equal leaf count (and therefore at equal allocation
/// count — see the module docs).
const TREE_LEAVES: [usize; 3] = [64, 1024, 4096];

/// Step counts for the lazy / eager `architectures` surfaces.
const LAZY_STEPS: [usize; 3] = [64, 1024, 4096];

// ---------------------------------------------------------------------------
// Counting global allocator
// ---------------------------------------------------------------------------

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static REALLOCS: AtomicU64 = AtomicU64::new(0);
static DEALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

/// A pass-through `GlobalAlloc` that tallies every allocator call before
/// forwarding to [`System`]. Counters are `Relaxed` — the allocation pass is
/// single-threaded, and the tallies are read only between measured windows.
struct CountingAllocator;

// SAFETY (whole impl): every method forwards its arguments unchanged to the
// corresponding `System` method, which satisfies the `GlobalAlloc` contract.
// The wrapper adds only `Relaxed` atomic arithmetic on statics, which cannot
// allocate, cannot unwind, and does not touch the returned pointers — so the
// contract `System` upholds is upheld verbatim.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: `layout` is forwarded unchanged from the caller, who already
        // owes `System::alloc` its non-zero-size guarantee.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: as `alloc` — `layout` is forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        REALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        // SAFETY: `ptr`, `layout` and `new_size` are forwarded unchanged; the
        // caller already owes `System::realloc` the "ptr came from this
        // allocator with this layout" guarantee.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: `ptr` and `layout` are forwarded unchanged; the caller already
        // owes `System::dealloc` the matching-allocation guarantee.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Allocator activity attributable to one measured closure.
#[derive(Clone, Copy)]
struct AllocDelta {
    allocs: u64,
    reallocs: u64,
    deallocs: u64,
    bytes: u64,
}

fn snapshot() -> AllocDelta {
    AllocDelta {
        allocs: ALLOCS.load(Ordering::Relaxed),
        reallocs: REALLOCS.load(Ordering::Relaxed),
        deallocs: DEALLOCS.load(Ordering::Relaxed),
        bytes: BYTES.load(Ordering::Relaxed),
    }
}

/// Run `f` and return the allocator activity it caused, alongside its result.
///
/// The result is returned rather than dropped so the caller decides whether the
/// teardown belongs inside the measured window. (For `construct`/`unfold` it
/// does not; for `fold` the teardown *is* the operation, and shows up as
/// `dealloc`s with zero `alloc`s.)
fn measure<T>(f: impl FnOnce() -> T) -> (AllocDelta, T) {
    let before = snapshot();
    let value = f();
    let after = snapshot();
    (
        AllocDelta {
            allocs: after.allocs - before.allocs,
            reallocs: after.reallocs - before.reallocs,
            deallocs: after.deallocs - before.deallocs,
            bytes: after.bytes - before.bytes,
        },
        value,
    )
}

/// One printed row of the allocation table.
struct Row {
    shape: &'static str,
    op: &'static str,
    size: usize,
    delta: AllocDelta,
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// `n` deterministic `f64` payloads from the per-file LCG stream.
fn payloads(n: usize) -> Vec<f64> {
    let mut rng = Lcg::new(SEED);
    (0..n).map(|_| rng.next_f64()).collect()
}

/// A perfectly balanced `BinaryTree` with `2^depth` leaves, labelled from
/// `labels` in left-to-right order (cycling if `labels` is shorter).
fn balanced_tree(depth: u32, labels: &[f64], next: &mut usize) -> BinaryTree<f64> {
    if depth == 0 {
        let a = labels[*next % labels.len()];
        *next += 1;
        BinaryTree::leaf(a)
    } else {
        let l = balanced_tree(depth - 1, labels, next);
        let r = balanced_tree(depth - 1, labels, next);
        BinaryTree::node(l, r)
    }
}

/// A left caterpillar with `leaves` leaves and depth `leaves − 1`: every
/// internal node has a leaf as its right child.
fn spine_tree(leaves: usize, labels: &[f64]) -> BinaryTree<f64> {
    assert!(leaves >= 1, "spine_tree needs at least one leaf");
    let mut acc = BinaryTree::leaf(labels[0]);
    for i in 1..leaves {
        acc = BinaryTree::node(acc, BinaryTree::leaf(labels[i % labels.len()]));
    }
    acc
}

/// `log2` of a power-of-two leaf count, for [`balanced_tree`].
fn leaves_to_depth(leaves: usize) -> u32 {
    assert!(
        leaves.is_power_of_two(),
        "balanced fixtures need a power-of-two leaf count"
    );
    leaves.trailing_zeros()
}

fn build_balanced(leaves: usize, labels: &[f64]) -> BinaryTree<f64> {
    let mut next = 0usize;
    balanced_tree(leaves_to_depth(leaves), labels, &mut next)
}

/// Sum-of-labels algebra for `Free<ListEndo<f64>, ()>`.
fn list_algebra(cell: Option<(f64, f64)>) -> f64 {
    match cell {
        None => 0.0,
        Some((a, rest)) => a + rest,
    }
}

/// Sum-of-leaves algebra for `Free<TreeEndo<f64>, Infallible>`.
fn tree_algebra(cell: Either<f64, (f64, f64)>) -> f64 {
    match cell {
        Either::Left(a) => a,
        Either::Right((l, r)) => l + r,
    }
}

/// `Pure` handler for the `Infallible`-terminated tree carrier — statically
/// unreachable, discharged by `Infallible` exhaustion.
fn tree_pure_case(z: Infallible) -> f64 {
    match z {}
}

/// Coalgebra growing a `Cofree<ListEndo<f64>, f64>` spine of `n` cons cells.
fn list_coalgebra(n: usize) -> (f64, Option<(f64, usize)>) {
    #[allow(clippy::cast_precision_loss)]
    let label = n as f64;
    if n == 0 {
        (label, None)
    } else {
        (label, Some((label, n - 1)))
    }
}

/// Coalgebra growing a balanced `Cofree<TreeEndo<f64>, f64>` of depth `d`.
fn balanced_coalgebra(d: usize) -> (f64, Either<f64, (usize, usize)>) {
    #[allow(clippy::cast_precision_loss)]
    let label = d as f64;
    if d == 0 {
        (label, Either::Left(label))
    } else {
        (label, Either::Right((d - 1, d - 1)))
    }
}

/// Coalgebra growing a left-caterpillar `Cofree<TreeEndo<f64>, f64>` whose
/// seed counts down the spine; the right child of every node is a leaf.
fn spine_coalgebra(n: usize) -> (f64, Either<f64, (usize, usize)>) {
    #[allow(clippy::cast_precision_loss)]
    let label = n as f64;
    if n == 0 {
        (label, Either::Left(label))
    } else {
        (label, Either::Right((n - 1, 0)))
    }
}

/// The `Para(O × −)` counter cell type used by the lazy/eager unroller benches.
/// Spelled with `fn` pointers so the fixture has a nameable, non-inferred type.
type CounterCell = UnfoldingRnn<f64, f64, fn((f64, f64)) -> f64, fn((f64, f64)) -> f64, f64>;

/// The `Para(O × −)` counter cell used by the lazy/eager unroller benches.
fn unfolding_cell() -> CounterCell {
    UnfoldingRnn::new(0.5, |(_p, s)| s, |(p, s)| s + p)
}

// ---------------------------------------------------------------------------
// Allocation pass (explicit, outside criterion)
// ---------------------------------------------------------------------------

/// Collect and print the allocation table.
///
/// Runs before criterion exists, single-threaded, one un-measured warm-up per
/// operation so first-touch lazy initialisation never lands inside a window.
fn allocation_pass() {
    let mut rows: Vec<Row> = Vec::new();

    // --- construct -------------------------------------------------------
    for n in LIST_SIZES {
        let items = payloads(n);
        drop(black_box(vec_to_free_mnd(items.clone(), ())));
        let input = items.clone();
        let (delta, built) = measure(|| vec_to_free_mnd(input, ()));
        drop(built);
        rows.push(Row {
            shape: "list",
            op: "construct",
            size: n,
            delta,
        });
    }
    for leaves in TREE_LEAVES {
        let labels = payloads(leaves);
        for (shape, tree) in [
            ("tree_balanced", build_balanced(leaves, &labels)),
            ("tree_spine", spine_tree(leaves, &labels)),
        ] {
            drop(black_box(tree_to_free_mnd(tree.clone())));
            let input = tree.clone();
            let (delta, built) = measure(|| tree_to_free_mnd(input));
            drop(built);
            rows.push(Row {
                shape,
                op: "construct",
                size: leaves,
                delta,
            });
        }
    }

    // --- fold ------------------------------------------------------------
    for n in LIST_SIZES {
        let items = payloads(n);
        let _ = black_box(vec_to_free_mnd(items.clone(), ()).fold(&|()| 0.0, &list_algebra));
        let program = vec_to_free_mnd(items.clone(), ());
        let (delta, total) = measure(|| program.fold(&|()| 0.0, &list_algebra));
        let _ = black_box(total);
        rows.push(Row {
            shape: "list",
            op: "fold",
            size: n,
            delta,
        });
    }
    for leaves in TREE_LEAVES {
        let labels = payloads(leaves);
        for (shape, tree) in [
            ("tree_balanced", build_balanced(leaves, &labels)),
            ("tree_spine", spine_tree(leaves, &labels)),
        ] {
            let _ = black_box(tree_to_free_mnd(tree.clone()).fold(&tree_pure_case, &tree_algebra));
            let program = tree_to_free_mnd(tree.clone());
            let (delta, total) = measure(|| program.fold(&tree_pure_case, &tree_algebra));
            let _ = black_box(total);
            rows.push(Row {
                shape,
                op: "fold",
                size: leaves,
                delta,
            });
        }
    }

    // --- unfold ----------------------------------------------------------
    for n in LIST_SIZES {
        drop(black_box(Cofree::<ListEndo<f64>, f64>::unfold(
            n,
            &list_coalgebra,
        )));
        let (delta, grown) = measure(|| Cofree::<ListEndo<f64>, f64>::unfold(n, &list_coalgebra));
        drop(grown);
        rows.push(Row {
            shape: "list",
            op: "unfold",
            size: n,
            delta,
        });
    }
    for leaves in TREE_LEAVES {
        let depth = leaves_to_depth(leaves) as usize;
        drop(black_box(Cofree::<TreeEndo<f64>, f64>::unfold(
            depth,
            &balanced_coalgebra,
        )));
        let (delta, grown) =
            measure(|| Cofree::<TreeEndo<f64>, f64>::unfold(depth, &balanced_coalgebra));
        drop(grown);
        rows.push(Row {
            shape: "tree_balanced",
            op: "unfold",
            size: leaves,
            delta,
        });

        let spine = leaves - 1;
        drop(black_box(Cofree::<TreeEndo<f64>, f64>::unfold(
            spine,
            &spine_coalgebra,
        )));
        let (delta, grown) =
            measure(|| Cofree::<TreeEndo<f64>, f64>::unfold(spine, &spine_coalgebra));
        drop(grown);
        rows.push(Row {
            shape: "tree_spine",
            op: "unfold",
            size: leaves,
            delta,
        });
    }

    // --- lazy / eager architecture surfaces -------------------------------
    let cell = unfolding_cell();
    for n in LAZY_STEPS {
        let _ = black_box(UnfoldingRnn::unroll_iter(&cell, 0.0).take(n).sum::<f64>());
        let (delta, total) = measure(|| UnfoldingRnn::unroll_iter(&cell, 0.0).take(n).sum::<f64>());
        let _ = black_box(total);
        rows.push(Row {
            shape: "unroll_iter",
            op: "lazy",
            size: n,
            delta,
        });

        drop(black_box(UnfoldingRnn::unroll_to_vec(&cell, 0.0, n)));
        let (delta, out) = measure(|| UnfoldingRnn::unroll_to_vec(&cell, 0.0, n));
        drop(out);
        rows.push(Row {
            shape: "unroll_to_vec",
            op: "eager",
            size: n,
            delta,
        });
    }

    let mealy: MealyCell<f64, f64, _, f64, f64> =
        MealyCell::new(0.5, |(p, s): (f64, f64)| move |i: f64| (s + i, s * p + i));
    let moore: MooreCell<f64, f64, _, _, f64, f64> = MooreCell::new(
        0.5,
        |(p, s): (f64, f64)| s * p,
        |(_p, s, i): (f64, f64, f64)| s + i,
    );
    for n in LAZY_STEPS {
        let inputs = payloads(n);
        let _ = black_box(MealyCell::run_iter(&mealy, 0.0, inputs.iter().copied()).sum::<f64>());
        let (delta, total) =
            measure(|| MealyCell::run_iter(&mealy, 0.0, inputs.iter().copied()).sum::<f64>());
        let _ = black_box(total);
        rows.push(Row {
            shape: "mealy_run_iter",
            op: "lazy",
            size: n,
            delta,
        });

        let _ = black_box(MooreCell::run_iter(&moore, 0.0, inputs.iter().copied()).sum::<f64>());
        let (delta, total) =
            measure(|| MooreCell::run_iter(&moore, 0.0, inputs.iter().copied()).sum::<f64>());
        let _ = black_box(total);
        rows.push(Row {
            shape: "moore_run_iter",
            op: "lazy",
            size: n,
            delta,
        });
    }

    print_rows(&rows);
}

fn print_rows(rows: &[Row]) {
    println!();
    println!("catgraph-dl #156 — shape-axis allocation baseline (Phase 1; haft carriers only)");
    println!(
        "size = cons cells (list) / leaves (tree) / steps (lazy). bytes = requested, not resident."
    );
    println!(
        "{:<15} {:<10} {:>6} {:>9} {:>9} {:>9} {:>11}",
        "shape", "op", "size", "alloc", "realloc", "dealloc", "bytes"
    );
    for row in rows {
        println!(
            "{:<15} {:<10} {:>6} {:>9} {:>9} {:>9} {:>11}",
            row.shape,
            row.op,
            row.size,
            row.delta.allocs,
            row.delta.reallocs,
            row.delta.deallocs,
            row.delta.bytes
        );
    }
    println!();
}

// ---------------------------------------------------------------------------
// Criterion groups (latency only)
// ---------------------------------------------------------------------------

fn bench_construct(c: &mut Criterion) {
    let mut group = c.benchmark_group("carrier::construct");
    for n in LIST_SIZES {
        let items = payloads(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("list", n), &items, |b, items| {
            b.iter_batched(
                || items.clone(),
                |items| black_box(vec_to_free_mnd(items, ())),
                BatchSize::LargeInput,
            );
        });
    }
    for leaves in TREE_LEAVES {
        let labels = payloads(leaves);
        group.throughput(Throughput::Elements(leaves as u64));
        for (shape, tree) in [
            ("tree_balanced", build_balanced(leaves, &labels)),
            ("tree_spine", spine_tree(leaves, &labels)),
        ] {
            group.bench_with_input(BenchmarkId::new(shape, leaves), &tree, |b, tree| {
                b.iter_batched(
                    || tree.clone(),
                    |tree| black_box(tree_to_free_mnd(tree)),
                    BatchSize::LargeInput,
                );
            });
        }
    }
    group.finish();
}

fn bench_fold(c: &mut Criterion) {
    let mut group = c.benchmark_group("carrier::fold");
    for n in LIST_SIZES {
        let items = payloads(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("list", n), &items, |b, items| {
            b.iter_batched(
                || vec_to_free_mnd(items.clone(), ()),
                |program| black_box(program.fold(&|()| 0.0, &list_algebra)),
                BatchSize::LargeInput,
            );
        });
    }
    for leaves in TREE_LEAVES {
        let labels = payloads(leaves);
        group.throughput(Throughput::Elements(leaves as u64));
        for (shape, tree) in [
            ("tree_balanced", build_balanced(leaves, &labels)),
            ("tree_spine", spine_tree(leaves, &labels)),
        ] {
            group.bench_with_input(BenchmarkId::new(shape, leaves), &tree, |b, tree| {
                b.iter_batched(
                    || tree_to_free_mnd(tree.clone()),
                    |program| black_box(program.fold(&tree_pure_case, &tree_algebra)),
                    BatchSize::LargeInput,
                );
            });
        }
    }
    group.finish();
}

fn bench_unfold(c: &mut Criterion) {
    let mut group = c.benchmark_group("carrier::unfold");
    for n in LIST_SIZES {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("list", n), &n, |b, &n| {
            b.iter(|| black_box(Cofree::<ListEndo<f64>, f64>::unfold(n, &list_coalgebra)));
        });
    }
    for leaves in TREE_LEAVES {
        group.throughput(Throughput::Elements(leaves as u64));
        let depth = leaves_to_depth(leaves) as usize;
        group.bench_with_input(
            BenchmarkId::new("tree_balanced", leaves),
            &depth,
            |b, &d| {
                b.iter(|| black_box(Cofree::<TreeEndo<f64>, f64>::unfold(d, &balanced_coalgebra)));
            },
        );
        let spine = leaves - 1;
        group.bench_with_input(BenchmarkId::new("tree_spine", leaves), &spine, |b, &n| {
            b.iter(|| black_box(Cofree::<TreeEndo<f64>, f64>::unfold(n, &spine_coalgebra)));
        });
    }
    group.finish();
}

fn bench_lazy(c: &mut Criterion) {
    let mut group = c.benchmark_group("lazy::iter");
    let cell = unfolding_cell();
    let mealy: MealyCell<f64, f64, _, f64, f64> =
        MealyCell::new(0.5, |(p, s): (f64, f64)| move |i: f64| (s + i, s * p + i));
    let moore: MooreCell<f64, f64, _, _, f64, f64> = MooreCell::new(
        0.5,
        |(p, s): (f64, f64)| s * p,
        |(_p, s, i): (f64, f64, f64)| s + i,
    );

    for n in LAZY_STEPS {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("unroll_iter", n), &n, |b, &n| {
            b.iter(|| black_box(UnfoldingRnn::unroll_iter(&cell, 0.0).take(n).sum::<f64>()));
        });
        group.bench_with_input(BenchmarkId::new("unroll_to_vec", n), &n, |b, &n| {
            b.iter(|| black_box(UnfoldingRnn::unroll_to_vec(&cell, 0.0, n)));
        });

        let inputs = payloads(n);
        group.bench_with_input(
            BenchmarkId::new("mealy_run_iter", n),
            &inputs,
            |b, inputs| {
                b.iter(|| {
                    black_box(MealyCell::run_iter(&mealy, 0.0, inputs.iter().copied()).sum::<f64>())
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("moore_run_iter", n),
            &inputs,
            |b, inputs| {
                b.iter(|| {
                    black_box(MooreCell::run_iter(&moore, 0.0, inputs.iter().copied()).sum::<f64>())
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    carrier_shapes,
    bench_construct,
    bench_fold,
    bench_unfold,
    bench_lazy
);

/// Custom harness: the allocation pass runs **first**, single-threaded and with
/// criterion not yet constructed, so the counters see only carrier work. The
/// remainder is the standard `criterion_main!` body.
fn main() {
    allocation_pass();
    carrier_shapes();
    Criterion::default().configure_from_args().final_summary();
}
