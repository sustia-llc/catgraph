//! Free monad `FreeMnd(F)(Z) = Fix(X ↦ F(X) + Z)` and cofree comonad
//! `CofreeCmnd(F)(Z) = Fix(X ↦ F(X) × Z)` (CDL Prop B.18) over an [`HKT`]
//! witness `F`.
//!
//! - [`Free<F, A>`] — `Pure(A) | Suspend(F::Type<Box<Free>>)`, read through
//!   [`FreeView`]; [`Free::fold`] is the catamorphism (CDL Remark 2.13).
//! - [`Cofree<F, A>`] — `head :< F::Type<Box<Cofree>>`; [`Cofree::unfold`] is
//!   the anamorphism (CDL Remark H.6 / App I).
//!
//! One `Box` per recursive hole. Specialisations (CDL Ex B.19, B.20):
//! `FreeMnd(1 + A × −) ≅ List(A)` in [`list_endo`], `FreeMnd(A + (−)²) ≅ Tree(A)`
//! in [`tree_endo`]; the bijection helpers are not re-exported at the crate root.
//!
//! Every walk — [`Free::fold`], [`Cofree::unfold`], the tree bijections,
//! `Drop`, `PartialEq`, `Debug`, [`BinaryTree`](tree_endo::BinaryTree)'s `Clone`
//! — is an explicit heap worklist; no spine depth overflows the stack.
//!
//! `PartialEq` / `Debug` are opt-in through
//! [`EqFunctor::eq_shape`](crate::endofunctor::EqFunctor::eq_shape) /
//! [`DebugFunctor::fmt_shape`] on the witness; no `Eq`. A carrier's `Debug`
//! honours `{:#?}`, precision and width; fill/alignment, sign, zero-pad and hex
//! flags render as if absent.
//!
//! The carriers hand-write [`Drop`], so a borrowed payload must outlive the
//! carrier:
//!
//! ```compile_fail,E0597
//! use catgraph_dl::free_monad::tree_endo::BinaryTree;
//! let tree;
//! let payload = String::from("x");
//! tree = BinaryTree::leaf(payload.as_str());
//! # let _ = &tree;
//! ```
//!
//! ```
//! use catgraph_dl::free_monad::tree_endo::BinaryTree;
//! let payload = String::from("x");
//! let tree = BinaryTree::leaf(payload.as_str());
//! # let _ = &tree;
//! ```
//!
//! Surface: [`Free`] has `pure`/`suspend`/`into_view`/`as_view`/`fold`;
//! [`Cofree`] has `new`/`head`/`tail`/`into_parts`/`unfold`. No `bind`/`map`
//! on `Free`, no `extract`/`extend` on `Cofree`, no [`Functor`] impl on
//! [`CofreeWitness`], no carrier `Clone`.

mod cofree;
mod free;

pub mod list_endo;
pub mod tree_endo;

pub use cofree::{Cofree, CofreeWitness};
pub use free::{Free, FreeView, FreeWitness};

// The endofunctor witnesses live in `crate::endofunctor`, the substrate seam
// (issue #12); surfaced here so this module's consumers can name the bound the
// bijection helpers below are written against.
pub use crate::endofunctor::{EndoWitness, Functor, HKT};

use core::cell::RefCell;
use core::fmt::{self, Write as _};

use crate::endofunctor::DebugFunctor;

// ---------------------------------------------------------------------------
// Shared `Debug` scaffolding for the iterative carrier renderings (#200)
// ---------------------------------------------------------------------------
//
// A carrier renders **top-down and streaming**: each cell's own shape is laid
// out once, into a small scratch buffer, with its recursion slots stood in for
// by probes; the segments between the probes are then written straight to the
// caller's `Formatter` while the children are visited on an explicit stack. No
// child's text is ever copied into a parent's, so the cost is the size of the
// output and nothing more, and no stack frame is spent per level of spine.
//
// The first version of this (#200 as merged) rendered **bottom-up** instead —
// `format!` per cell, each parent copying its children's finished text — which
// is Θ(Σ_v |subtree text|), quadratic in the depth of a spine even in compact
// mode: a 32 768-deep `BinaryTree<u8>` took ~2 s to print (now 0.02 s). The
// rewrite keeps the output byte-identical — pinned by `debug_reproduces_*` and,
// more sharply, by `every_carrier_debug_is_byte_identical_to_a_derived_twin`,
// which diffs each carrier against a plain `#[derive(Debug)]` type of the same
// shape, at every format spec a scratch pass can carry — and drops the cost to
// the linear one the docs always claimed.
//
// Byte-identity comes from never re-implementing `core::fmt`'s layout: a
// cell's shape is still rendered by `debug_tuple`/`debug_struct`, alternate
// flag and all. The one thing this module *does* reproduce is `PadAdapter`'s
// per-line indentation, because a child's text is written to the real
// formatter rather than through the parent's builder — see [`Indenter`]. Under
// `{:#?}` the indentation is itself Θ(depth) per line, so the pretty output of
// a caterpillar is inherently quadratic *in characters*; writing it is linear
// in that output.
//
// The format spec is the one place the reconstruction is *partial*, because a
// scratch pass cannot inherit the caller's `Formatter` wholesale — see
// [`Spec`], which is the exact statement of what survives and what does not.

/// The caller's format spec as far as a scratch `write!` can carry it:
/// `alternate`, `precision`, `width`. Fill, alignment, sign, zero-pad and
/// debug-hex flags are dropped.
#[derive(Clone, Copy)]
struct Spec {
    alternate: bool,
    width: Option<usize>,
    precision: Option<usize>,
}

impl Spec {
    /// Read the carryable part of the spec off the caller's formatter.
    fn of(f: &fmt::Formatter<'_>) -> Self {
        Self {
            alternate: f.alternate(),
            width: f.width(),
            precision: f.precision(),
        }
    }
}

/// Formats an `F::Type<T>` through the witness's `fmt_shape`, with the
/// recursion slots supplied as probes — the seam that keeps the carrier's
/// `Debug` off the stack, and that avoids the projection bound whose E0275
/// hazard [`crate::endofunctor::EqFunctor`] documents.
pub(crate) struct FmtShape<'a, F: HKT, T>(
    pub(crate) &'a F::Type<T>,
    pub(crate) &'a [&'a dyn fmt::Debug],
);

impl<F, T> fmt::Debug for FmtShape<'_, F, T>
where
    F: DebugFunctor,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        F::fmt_shape(self.0, f, self.1)
    }
}

/// One node of a carrier, as the shared renderer sees it: a shape that can be
/// laid out on its own, plus the recursion slots that shape leaves holes for.
///
/// Implemented by [`Free`], [`Cofree`] and
/// [`BinaryTree`](tree_endo::BinaryTree); [`write_debug`] is the whole reason
/// it exists.
pub(crate) trait DebugNode {
    /// This node's recursion slots, in position order — the same order
    /// [`fmt_cell`](Self::fmt_cell) consumes its `holes`.
    fn slots(&self) -> Vec<&Self>;

    /// Lay out this node's own shape, writing `holes[i]` wherever slot `i`'s
    /// rendering belongs. `holes` has one entry per [`slots`](Self::slots)
    /// entry.
    fn fmt_cell(&self, f: &mut fmt::Formatter<'_>, holes: &[&dyn fmt::Debug]) -> fmt::Result;
}

/// [`DebugNode::fmt_cell`] as a `Debug` value, so it can be laid out by
/// `write!` and land in [`Capture`]'s buffer.
struct Cell<'a, N: ?Sized>(&'a N, &'a [&'a dyn fmt::Debug]);

impl<N: DebugNode + ?Sized> fmt::Debug for Cell<'_, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_cell(f, self.1)
    }
}

/// Where one cell's shape is laid out, and where its probes record themselves.
///
/// Interior mutability is forced by `Debug::fmt(&self, …)`: a [`Probe`] has to
/// read the buffer it is being written into. The crate is
/// `#![forbid(unsafe_code)]`, so that is a [`RefCell`], never a raw pointer.
#[derive(Default)]
struct Capture {
    text: RefCell<String>,
    holes: RefCell<Vec<Hole>>,
    /// Set by a [`Probe`] whose own writes did not land in `text` — the
    /// out-of-contract witness [`frame`] turns into `Err(fmt::Error)`. See
    /// [`Probe::fmt`] for why the probe records the fact instead of returning
    /// the error itself.
    lost_hole: RefCell<bool>,
}

/// Where one recursion slot's rendering goes, in the enclosing cell's text.
///
/// The text up to `cut` is written before the child and the text from `resume`
/// on is written after it; the bytes between are the probe's own tracer and are
/// dropped. `indent` is the per-line padding `core::fmt`'s pretty builders would
/// have added to the child's text, recovered by measurement rather than
/// re-derived.
#[derive(Clone, Copy)]
struct Hole {
    slot: usize,
    cut: usize,
    resume: usize,
    indent: usize,
}

/// The sink a cell's shape is laid out into.
struct Sink<'a>(&'a Capture);

impl fmt::Write for Sink<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.text.borrow_mut().push_str(s);
        Ok(())
    }
}

/// The single byte a [`Probe`] leaves behind to make the enclosing builder's
/// indentation observable. It is always cut back out; it never reaches output.
const TRACER: &str = "\0";

/// Stands in for one recursion slot while its parent's shape is laid out.
///
/// Writing `"\n" + TRACER` — rather than nothing — is what makes the
/// surrounding pad adapters *show* their indentation: whatever they insert
/// between the newline and the tracer is exactly what they would insert before
/// every line of the real child. Measuring it beats re-deriving it, since the
/// nesting depth is the witness's business, not the carrier's.
struct Probe<'a> {
    slot: usize,
    capture: &'a Capture,
}

impl fmt::Debug for Probe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let start = self.capture.text.borrow().len();
        f.write_str("\n")?;
        let after_break = self.capture.text.borrow().len();
        f.write_str(TRACER)?;
        let resume = self.capture.text.borrow().len();

        // Both writes have to have landed in *this* buffer for the offsets to
        // mean anything. They do whenever `fmt_shape` writes the content
        // position into the `f` it was handed, which is its contract; a witness
        // that renders one into a buffer of its own instead leaves the buffer
        // untouched here, so there is no offset to splice at and the child would
        // simply vanish from the output.
        //
        // That is out of contract, and every other such path in this crate
        // returns `fmt::Error` for it (`OptionWitness` / `ListEndo` / `TreeEndo`
        // on an arity mismatch, `BinaryTree::fmt_cell` likewise). So does this
        // one — but by *recording* the fact rather than returning it, for two
        // reasons. A witness that renders a hole with `format!` would **panic**
        // on an `Err` returned from here, which is exactly the failure mode the
        // renderer exists to avoid; and a witness that swallows the error would
        // turn the whole thing back into a silent omission. Poisoning the
        // capture is immune to both: `frame` reads it after the layout pass and
        // fails there, whatever the shape did with this call's return value.
        if after_break <= start || resume <= after_break {
            *self.capture.lost_hole.borrow_mut() = true;
            return Ok(());
        }

        self.capture.holes.borrow_mut().push(Hole {
            slot: self.slot,
            // The newline just written is the last byte before `after_break`,
            // and the child's text starts exactly where it sits.
            cut: after_break - 1,
            resume,
            // …with the padding that followed it repeated on every later line.
            indent: resume - after_break - TRACER.len(),
        });
        Ok(())
    }
}

/// One node's laid-out shape plus the slots still to be spliced into it.
struct Frame<'a, N: ?Sized> {
    text: String,
    holes: Vec<Hole>,
    slots: Vec<&'a N>,
    next: usize,
    cursor: usize,
    indent: usize,
}

/// Lay out one node's shape and hand back the frame that streams it.
///
/// The scratch pass re-expresses as much of the caller's format spec as a
/// format string can carry — see [`Spec`] for what that is and what it is not.
fn frame<'a, N>(node: &'a N, indent: usize, spec: Spec) -> Result<Frame<'a, N>, fmt::Error>
where
    N: DebugNode + ?Sized,
{
    let slots = node.slots();
    let capture = Capture::default();
    let probes: Vec<Probe<'_>> = (0..slots.len())
        .map(|slot| Probe {
            slot,
            capture: &capture,
        })
        .collect();
    let holes: Vec<&dyn fmt::Debug> = probes.iter().map(|p| p as &dyn fmt::Debug).collect();

    let cell = Cell(node, &holes);
    let mut sink = Sink(&capture);
    // Eight literal format strings, because `#`, `.p$` and `w$` can only be
    // spelled in the string itself — `p` and `w` are ordinary named arguments,
    // so only the *values* are dynamic.
    match (spec.alternate, spec.width, spec.precision) {
        (false, None, None) => write!(sink, "{cell:?}"),
        (false, None, Some(p)) => write!(sink, "{cell:.p$?}"),
        (false, Some(w), None) => write!(sink, "{cell:w$?}"),
        (false, Some(w), Some(p)) => write!(sink, "{cell:w$.p$?}"),
        (true, None, None) => write!(sink, "{cell:#?}"),
        (true, None, Some(p)) => write!(sink, "{cell:#.p$?}"),
        (true, Some(w), None) => write!(sink, "{cell:#w$?}"),
        (true, Some(w), Some(p)) => write!(sink, "{cell:#w$.p$?}"),
    }?;

    // A hole the shape declined to write into the `Formatter` it was handed
    // cannot be spliced anywhere, so the child would silently vanish. Fail
    // instead — see `Probe::fmt`.
    if capture.lost_hole.take() {
        return Err(fmt::Error);
    }

    Ok(Frame {
        text: capture.text.take(),
        holes: capture.holes.take(),
        slots,
        next: 0,
        cursor: 0,
        indent,
    })
}

/// Reproduces `core::fmt`'s `PadAdapter`: `indent` spaces before the content of
/// every line after a newline, and nothing before the first.
///
/// A child's text goes straight to the caller's formatter instead of through
/// its parent's `debug_tuple` builder, so the padding those nested builders
/// would have applied is re-applied here — additively, exactly as nested pad
/// adapters compose.
struct Indenter<'a, 'b> {
    out: &'a mut fmt::Formatter<'b>,
    on_newline: bool,
}

impl Indenter<'_, '_> {
    fn write(&mut self, indent: usize, s: &str) -> fmt::Result {
        for line in s.split_inclusive('\n') {
            if self.on_newline {
                self.pad(indent)?;
            }
            self.on_newline = line.ends_with('\n');
            self.out.write_str(line)?;
        }
        Ok(())
    }

    fn pad(&mut self, mut spaces: usize) -> fmt::Result {
        const CHUNK: &str = "                                                                ";
        while spaces > 0 {
            let take = spaces.min(CHUNK.len());
            self.out.write_str(&CHUNK[..take])?;
            spaces -= take;
        }
        Ok(())
    }
}

/// What to do after writing one segment of a frame.
enum Step<'a, N: ?Sized> {
    /// Descend into a slot, at the given inherited indentation.
    Descend(&'a N, usize),
    /// The shape declined to write this slot; stay on the frame.
    Stay,
    /// The frame is fully written.
    Done,
}

/// Write `root`'s `Debug` rendering to `f`, streaming and iteratively.
///
/// Every byte of the output is written exactly once, so the cost is Θ(output)
/// — and no stack frame is spent per level, so no spine is too deep. The
/// explicit stack holds one small scratch buffer per *ancestor*, not one per
/// node.
///
/// # Format spec
///
/// `alternate`, `precision` and `width` are carried down to every cell;
/// fill, alignment, the sign/zero-pad flags and `{:x?}`/`{:X?}` are **not**.
/// [`Spec`] states why, and what the visible difference from a
/// `#[derive(Debug)]` type of the same shape is.
///
/// # Errors
///
/// Propagates any `fmt::Error` from the sink, from a payload's own `Debug`, or
/// from a witness's [`DebugFunctor::fmt_shape`] — including the out-of-contract
/// case where `fmt_shape` never writes a recursion slot into the `Formatter` it
/// was handed, which would otherwise drop that whole subtree from the output.
pub(crate) fn write_debug<N>(root: &N, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    N: DebugNode + ?Sized,
{
    let spec = Spec::of(f);
    let mut out = Indenter {
        out: f,
        on_newline: false,
    };
    let mut stack: Vec<Frame<'_, N>> = vec![frame(root, 0, spec)?];

    while let Some(top) = stack.last_mut() {
        let step = match top.holes.get(top.next).copied() {
            Some(hole) => {
                out.write(top.indent, &top.text[top.cursor..hole.cut])?;
                top.cursor = hole.resume;
                top.next += 1;
                match top.slots.get(hole.slot) {
                    Some(&child) => Step::Descend(child, top.indent + hole.indent),
                    None => Step::Stay,
                }
            }
            None => {
                out.write(top.indent, &top.text[top.cursor..])?;
                Step::Done
            }
        };
        match step {
            Step::Descend(child, indent) => stack.push(frame(child, indent, spec)?),
            Step::Stay => (),
            Step::Done => {
                stack.pop();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Cofree, Free, FreeView};
    use crate::container::Container;
    use crate::endofunctor::{DebugFunctor, Either, Functor, HKT, OptionWitness};
    use crate::free_monad::tree_endo::{BinaryTree, TreeEndo, TreeView};
    use core::fmt;
    use core::fmt::Write as _;

    /// Plain `#[derive(Debug)]` twins of the three carriers, used as the
    /// **oracle** for their hand-written renderings.
    ///
    /// The carriers cannot derive `Debug` (the GAT projection overflows the
    /// trait solver — see `EqFunctor`), and since #200 they do not even walk
    /// recursively, so "byte-identical to the derive" had been pinned only
    /// against hand-typed strings at two or three levels. These types put the
    /// real derive back within reach: same variant/field names, same nesting,
    /// so `format!` on a carrier and on its twin must agree character for
    /// character at any depth, under every format spec the renderer carries.
    ///
    /// Each twin is named for the carrier it mirrors, because a derived
    /// `Debug` prints the type's own name. Each is **generic in its payload**,
    /// so the same shapes can be built over a `f64` — the payload whose own
    /// `Debug` honours `precision` and `width`, and so the only one that can
    /// tell whether the renderer carried the caller's format spec down.
    mod mirror {
        // Every field here is read by the derived `Debug` and nothing else,
        // which dead-code analysis deliberately does not count.
        #![allow(dead_code)]

        use crate::endofunctor::Either;

        #[derive(Debug)]
        pub enum BinaryTree<A> {
            Leaf(A),
            Node(Box<BinaryTree<A>>, Box<BinaryTree<A>>),
        }

        /// `TreeEndo<A>`'s functor hole at the `Free` twin — the alias exists
        /// only to keep `clippy::type_complexity` quiet; a `#[derive(Debug)]`
        /// renders the value, so it does not touch the output.
        type FreeHole<A> = Either<A, (Box<Free<A>>, Box<Free<A>>)>;

        #[derive(Debug)]
        pub enum Free<A> {
            Pure(A),
            Suspend(FreeHole<A>),
        }

        pub mod stream {
            #[derive(Debug)]
            pub struct Cofree<A> {
                pub head: A,
                pub tail: Option<Box<Cofree<A>>>,
            }
        }

        pub mod branching {
            use crate::endofunctor::Either;

            /// As `super::FreeHole`, and for the same reason.
            type CofreeHole<L, H> = Either<L, (Box<Cofree<L, H>>, Box<Cofree<L, H>>)>;

            #[derive(Debug)]
            pub struct Cofree<L, H> {
                pub head: H,
                pub tail: CofreeHole<L, H>,
            }
        }
    }

    /// The shape every twin pair below is built to: a left caterpillar that
    /// branches on every third level, so the layout is exercised with nesting
    /// on both sides and at several depths at once. Payloads are the level
    /// number, so a mis-ordered slot shows up as a wrong number rather than a
    /// coincidence.
    const SHAPE: u8 = 12;

    fn tree_pair<A: From<u8>>(depth: u8) -> (BinaryTree<A>, mirror::BinaryTree<A>) {
        if depth == 0 {
            return (
                BinaryTree::leaf(A::from(0)),
                mirror::BinaryTree::Leaf(A::from(0)),
            );
        }
        let (left, left_m) = tree_pair(depth - 1);
        let (right, right_m) = if depth.is_multiple_of(3) {
            tree_pair(depth - 1)
        } else {
            (
                BinaryTree::leaf(A::from(depth)),
                mirror::BinaryTree::Leaf(A::from(depth)),
            )
        };
        (
            BinaryTree::node(left, right),
            mirror::BinaryTree::Node(Box::new(left_m), Box::new(right_m)),
        )
    }

    fn free_pair<A: From<u8>>(depth: u8) -> (Free<TreeEndo<A>, A>, mirror::Free<A>) {
        if depth == 0 {
            // Both leaf arms: `Pure` (the `Z` slot) and `Suspend(Left(_))`.
            return (Free::pure(A::from(0)), mirror::Free::Pure(A::from(0)));
        }
        if depth == 1 {
            return (
                Free::suspend(Either::Left(A::from(1))),
                mirror::Free::Suspend(Either::Left(A::from(1))),
            );
        }
        let (left, left_m) = free_pair(depth - 1);
        let (right, right_m) = if depth.is_multiple_of(3) {
            free_pair(depth - 2)
        } else {
            (
                Free::suspend(Either::Left(A::from(depth))),
                mirror::Free::Suspend(Either::Left(A::from(depth))),
            )
        };
        (
            Free::suspend(Either::Right((Box::new(left), Box::new(right)))),
            mirror::Free::Suspend(Either::Right((Box::new(left_m), Box::new(right_m)))),
        )
    }

    fn stream_pair<A: From<u8>>(len: u8) -> (Cofree<OptionWitness, A>, mirror::stream::Cofree<A>) {
        let mut carrier = Cofree::new(A::from(0), None);
        let mut twin = mirror::stream::Cofree {
            head: A::from(0),
            tail: None,
        };
        for step in 1..=len {
            carrier = Cofree::new(A::from(step), Some(Box::new(carrier)));
            twin = mirror::stream::Cofree {
                head: A::from(step),
                tail: Some(Box::new(twin)),
            };
        }
        (carrier, twin)
    }

    fn branching_pair<L: From<u8>, H: From<u8>>(
        depth: u8,
    ) -> (Cofree<TreeEndo<L>, H>, mirror::branching::Cofree<L, H>) {
        if depth == 0 {
            return (
                Cofree::new(H::from(0), Either::Left(L::from(0))),
                mirror::branching::Cofree {
                    head: H::from(0),
                    tail: Either::Left(L::from(0)),
                },
            );
        }
        let (left, left_m) = branching_pair(depth - 1);
        let (right, right_m) = if depth.is_multiple_of(3) {
            branching_pair(depth - 1)
        } else {
            (
                Cofree::new(H::from(depth), Either::Left(L::from(depth))),
                mirror::branching::Cofree {
                    head: H::from(depth),
                    tail: Either::Left(L::from(depth)),
                },
            )
        };
        (
            Cofree::new(
                H::from(depth),
                Either::Right((Box::new(left), Box::new(right))),
            ),
            mirror::branching::Cofree {
                head: H::from(depth),
                tail: Either::Right((Box::new(left_m), Box::new(right_m))),
            },
        )
    }

    /// Assert a carrier and its derived twin agree **character for character**
    /// across every format spec the renderer claims to carry.
    ///
    /// The default `{:?}` / `{:#?}` pair is the shape guard; the four
    /// spec-bearing forms are the [`Spec`](super::Spec) guard, and only bite on
    /// a payload whose own `Debug` honours `precision` / `width` — hence the
    /// `f64` instantiations at the call sites. `width` bites on integers too.
    macro_rules! assert_agrees_at_every_carried_spec {
        ($carrier:expr, $twin:expr, $what:expr) => {{
            let (live, twin, what) = (&$carrier, &$twin, $what);
            assert_eq!(format!("{live:?}"), format!("{twin:?}"), "{what} {{:?}}");
            assert_eq!(format!("{live:#?}"), format!("{twin:#?}"), "{what} {{:#?}}");
            assert_eq!(
                format!("{live:.2?}"),
                format!("{twin:.2?}"),
                "{what} {{:.2?}} — precision must reach every payload"
            );
            assert_eq!(
                format!("{live:#.2?}"),
                format!("{twin:#.2?}"),
                "{what} {{:#.2?}} — precision must survive the pretty form too"
            );
            assert_eq!(
                format!("{live:12?}"),
                format!("{twin:12?}"),
                "{what} {{:12?}} — width must reach every payload"
            );
            assert_eq!(
                format!("{live:#12?}"),
                format!("{twin:#12?}"),
                "{what} {{:#12?}} — width must survive the pretty form too"
            );
        }};
    }

    /// Each carrier vs a `#[derive(Debug)]` twin of the same shape, at `{:?}`,
    /// `{:#?}`, `{:.2?}`, `{:#.2?}`, `{:12?}`, `{:#12?}`, for `u8`/`u32` and
    /// `f64` payloads (precision is visible only on floats).
    #[test]
    fn every_carrier_debug_is_byte_identical_to_a_derived_twin() {
        // Integer payloads: the shape guard, plus `width`.
        let (tree, tree_m) = tree_pair::<u8>(SHAPE);
        assert_agrees_at_every_carried_spec!(tree, tree_m, "BinaryTree<u8>");

        let (free, free_m) = free_pair::<u8>(SHAPE);
        assert_agrees_at_every_carried_spec!(free, free_m, "Free<TreeEndo<u8>, u8>");

        let (stream, stream_m) = stream_pair::<u32>(SHAPE * 4);
        assert_agrees_at_every_carried_spec!(stream, stream_m, "Cofree<OptionWitness, u32>");

        let (branching, branching_m) = branching_pair::<u8, u32>(SHAPE);
        assert_agrees_at_every_carried_spec!(branching, branching_m, "Cofree<TreeEndo<u8>, u32>");

        // Float payloads: the same four shapes, at the only payload that can
        // see `precision` go missing.
        let (tree_f, tree_fm) = tree_pair::<f64>(SHAPE);
        assert_agrees_at_every_carried_spec!(tree_f, tree_fm, "BinaryTree<f64>");

        let (free_f, free_fm) = free_pair::<f64>(SHAPE);
        assert_agrees_at_every_carried_spec!(free_f, free_fm, "Free<TreeEndo<f64>, f64>");

        let (stream_f, stream_fm) = stream_pair::<f64>(SHAPE * 4);
        assert_agrees_at_every_carried_spec!(stream_f, stream_fm, "Cofree<OptionWitness, f64>");

        let (branching_f, branching_fm) = branching_pair::<f64, f64>(SHAPE);
        assert_agrees_at_every_carried_spec!(
            branching_f,
            branching_fm,
            "Cofree<TreeEndo<f64>, f64>"
        );

        // The pretty forms really are the compounding case, not a rerun of the
        // compact one: at this depth they are many times longer and multi-line.
        assert!(format!("{tree:#?}").len() > 8 * format!("{tree:?}").len());
        assert!(format!("{branching:#?}").lines().count() > 100);

        // …and the spec-bearing forms really are a different rendering, not a
        // rerun of the default one. Without this, every `{:.2?}` assertion
        // above would hold vacuously the moment precision stopped propagating
        // on *both* sides — which is exactly how the default-spec-only version
        // of this oracle passed through a regression.
        assert_ne!(
            format!("{tree_f:.2?}"),
            format!("{tree_f:?}"),
            "the f64 payload must actually render differently under {{:.2?}}"
        );
        assert_ne!(
            format!("{tree:12?}"),
            format!("{tree:?}"),
            "the u8 payload must actually render differently under {{:12?}}"
        );
    }

    /// The specs the renderer does **not** carry render as if the flag were
    /// absent — a deliberate, documented divergence from the derive, not an
    /// accident.
    ///
    /// A cell's shape is laid out by a `write!` into a scratch buffer, so the
    /// spec has to be re-expressed in that `write!`'s own format string.
    /// `alternate`, `precision` and `width` can be (the last two through
    /// `{:.p$?}` / `{:w$?}`); fill, alignment and the sign/zero-pad flags can
    /// only be spelled literally, and `{:x?}` / `{:X?}` cannot even be *read*
    /// off a `Formatter` on stable. See [`Spec`](super::Spec).
    ///
    /// Each assertion is the exact positive claim — "renders as the same value
    /// with the flag absent" — rather than a bare `assert_ne!` against the
    /// twin, which would also pass if the carrier started producing garbage.
    /// The twin comparisons at the end are the anti-vacuity guard: they show
    /// the flag is one a `#[derive(Debug)]` of the same shape really does
    /// honour, so this is divergence and not a payload that ignores it anyway.
    #[test]
    fn dropped_format_specs_render_as_if_the_flag_were_absent() {
        let (tree, tree_m) = tree_pair::<f64>(4);
        // At this depth the leaf labels run past 9, so the hex rendering really
        // is a different string (`Leaf(c)` for 12, not `Leaf(12)`).
        let (hex, hex_m) = tree_pair::<u8>(SHAPE);

        // Fill and alignment: dropped. Width is not, so `{:*>12?}` renders at
        // width 12 with the default fill and the payload's default alignment.
        assert_eq!(
            format!("{tree:*>12?}"),
            format!("{tree:12?}"),
            "fill is dropped; width is carried"
        );
        // Alignment separately, because `f64` is right-aligned by default —
        // `{:*>12?}` alone would still pass if only the fill were dropped.
        assert_eq!(
            format!("{tree:<12?}"),
            format!("{tree:12?}"),
            "alignment is dropped; width is carried"
        );
        // The sign flags: dropped.
        assert_eq!(format!("{tree:+?}"), format!("{tree:?}"), "`+` is dropped");
        // Zero-padding: dropped (again, the width itself is carried).
        assert_eq!(
            format!("{tree:012?}"),
            format!("{tree:12?}"),
            "`0` is dropped; width is carried"
        );
        // The debug-hex flags: dropped, and unreadable on stable.
        assert_eq!(
            format!("{hex:x?}"),
            format!("{hex:?}"),
            "`x?` is dropped — `Formatter` has no stable accessor for it"
        );

        // Anti-vacuity: the derive honours every one of them.
        assert_ne!(format!("{tree_m:*>12?}"), format!("{tree_m:12?}"));
        assert_ne!(format!("{tree_m:<12?}"), format!("{tree_m:12?}"));
        assert_ne!(format!("{tree_m:+?}"), format!("{tree_m:?}"));
        assert_ne!(format!("{tree_m:012?}"), format!("{tree_m:12?}"));
        assert_ne!(format!("{hex_m:x?}"), format!("{hex_m:?}"));
    }

    /// A payload whose `Debug` legitimately fails.
    struct Grumpy;

    impl fmt::Debug for Grumpy {
        fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    /// A failing inner `Debug` must **propagate** out of the carrier, not
    /// panic.
    ///
    /// [`DebugFunctor::fmt_shape`](crate::endofunctor::DebugFunctor::fmt_shape)
    /// documents returning "a formatting error rather than a panic", and a
    /// payload `A` whose own `Debug` returns `Err` used to propagate out of the
    /// derive. Rendering a cell with `format!` broke both — `format!` panics
    /// when a formatting impl returns `Err`. The streaming renderer lays cells
    /// out with `write!` into a `String` sink and propagates instead.
    #[test]
    fn a_failing_inner_debug_propagates_rather_than_panicking() {
        let mut sink = String::new();

        let tree = BinaryTree::node(BinaryTree::leaf(Grumpy), BinaryTree::leaf(Grumpy));
        assert!(
            write!(sink, "{tree:?}").is_err(),
            "BinaryTree must surface the payload's fmt::Error"
        );

        let free: Free<TreeEndo<Grumpy>, u8> = Free::suspend(Either::Right((
            Box::new(Free::suspend(Either::Left(Grumpy))),
            Box::new(Free::pure(1)),
        )));
        assert!(
            write!(sink, "{free:?}").is_err(),
            "Free must surface the witness label's fmt::Error"
        );

        let cofree: Cofree<OptionWitness, Grumpy> =
            Cofree::new(Grumpy, Some(Box::new(Cofree::new(Grumpy, None))));
        assert!(
            write!(sink, "{cofree:?}").is_err(),
            "Cofree must surface the head's fmt::Error"
        );
    }

    /// A witness that renders a recursion slot into a buffer of **its own**
    /// instead of into the `Formatter` it was handed — the out-of-contract
    /// shape [`DebugFunctor::fmt_shape`](crate::endofunctor::DebugFunctor::fmt_shape)
    /// now names explicitly.
    ///
    /// Its object map is `Option`, so it is `OptionWitness` in every respect
    /// but `fmt_shape`. Two deliberate details make the pin sharp:
    ///
    /// - it writes the hole with `write!` into a `String`, **not** `format!` —
    ///   `format!` panics on an `Err` from a formatting impl, and a panic is
    ///   not what is being pinned;
    /// - it **swallows** that write's result. A witness that propagated would
    ///   surface an error the moment the probe returned one, so the test would
    ///   pass even if the renderer itself stayed silent. Swallowing leaves the
    ///   renderer as the only thing that can fail — which is the claim.
    struct StrayWitness;

    impl HKT for StrayWitness {
        type Type<T> = Option<T>;
    }

    impl Functor<Self> for StrayWitness {
        fn fmap<A, B, Func>(m_a: Option<A>, f: Func) -> Option<B>
        where
            Func: FnMut(A) -> B,
        {
            m_a.map(f)
        }
    }

    impl Container for StrayWitness {
        type Shape = bool;

        fn arity(shape: &bool) -> usize {
            usize::from(*shape)
        }

        fn decompose<X>(fx: Option<X>) -> (bool, Vec<X>) {
            match fx {
                None => (false, Vec::new()),
                Some(x) => (true, vec![x]),
            }
        }

        fn recompose<X>(shape: bool, contents: Vec<X>) -> Option<Option<X>> {
            if shape {
                let [x] = <[X; 1]>::try_from(contents).ok()?;
                Some(Some(x))
            } else {
                contents.is_empty().then_some(None)
            }
        }

        fn contents<X>(fx: &Option<X>) -> Vec<&X> {
            fx.as_ref().into_iter().collect()
        }
    }

    impl DebugFunctor for StrayWitness {
        fn fmt_shape<T>(
            fa: &Option<T>,
            f: &mut fmt::Formatter<'_>,
            contents: &[&dyn fmt::Debug],
        ) -> fmt::Result {
            match (fa, contents) {
                (None, _) => f.write_str("None"),
                (Some(_), [inner]) => {
                    let mut stray = String::new();
                    // Both halves of the misbehaviour: a private sink, and the
                    // result thrown away.
                    let _ = write!(stray, "Some({inner:?})");
                    f.write_str(&stray)
                }
                (Some(_), _) => Err(fmt::Error),
            }
        }
    }

    /// A witness that never writes a recursion slot into the `Formatter` it was
    /// handed must produce an **error**, not a silently truncated rendering.
    ///
    /// The renderer stands each slot in for by a probe whose own writes are
    /// measured to find the splice offsets. If those writes land somewhere
    /// else, there is no offset to splice at — so the child, and its whole
    /// subtree, simply do not appear. That used to return `Ok(())`, which made
    /// it the one out-of-contract path in this crate that failed *silently*:
    /// `OptionWitness` / `ListEndo` / `TreeEndo` on an arity mismatch, and
    /// `BinaryTree::fmt_cell` likewise, all return `fmt::Error`.
    ///
    /// Measured with the guard reverted, this printed
    /// `Cofree { head: 1, tail: Some(\n\0) }` and returned `Ok`: the child
    /// `Cofree { head: 2, tail: None }` is gone, and the probe's own tracer
    /// bytes have leaked into public output in its place.
    #[test]
    fn a_witness_that_never_writes_its_hole_errors_rather_than_dropping_the_child() {
        let mut sink = String::new();

        let stream: Cofree<StrayWitness, u8> = Cofree::new(1, Some(Box::new(Cofree::new(2, None))));
        let result = write!(sink, "{stream:?}");
        assert!(
            result.is_err(),
            "Cofree must reject a witness that renders its hole elsewhere; \
             got Ok with {sink:?}"
        );

        let free: Free<StrayWitness, u8> = Free::suspend(Some(Box::new(Free::pure(2))));
        let mut free_sink = String::new();
        let free_result = write!(free_sink, "{free:?}");
        assert!(
            free_result.is_err(),
            "Free must reject it too; got Ok with {free_sink:?}"
        );

        // The pretty form is the same renderer and the same failure.
        let mut pretty = String::new();
        assert!(write!(pretty, "{stream:#?}").is_err(), "…in {{:#?}} too");

        // Only the holes are affected: a cell with no recursion slot never
        // builds a probe, so this witness renders a leaf exactly as
        // `OptionWitness` does. Without this the test would also pass if the
        // renderer simply errored on every value.
        let leaf: Cofree<StrayWitness, u8> = Cofree::new(3, None);
        let mut leaf_sink = String::new();
        write!(leaf_sink, "{leaf:?}").expect("a slot-free cell has no hole to lose");
        assert_eq!(leaf_sink, "Cofree { head: 3, tail: None }");
    }

    /// `size_of` carrier vs cell: `Free` and `BinaryTree` equal at every
    /// instantiation checked; `Cofree` exactly one word larger.
    #[test]
    fn the_private_cell_costs_at_most_one_word() {
        use super::cofree::CofreeCell;
        use core::mem::size_of;

        // `Free`: the wrapper is free at every instantiation — the property the
        // original test checked, kept and widened.
        assert_eq!(
            size_of::<Free<OptionWitness, u64>>(),
            size_of::<FreeView<OptionWitness, u64>>(),
            "Free's Option cell must niche into FreeView's tag"
        );
        assert_eq!(
            size_of::<Free<TreeEndo<u8>, u8>>(),
            size_of::<FreeView<TreeEndo<u8>, u8>>(),
            "…at the branching witness too"
        );

        // `BinaryTree`: free as well, since `TreeView::Node` became one boxed
        // pair. A regression to the two-`Box` shape re-spends the view's niche
        // and puts the word back.
        assert_eq!(
            size_of::<BinaryTree<u8>>(),
            size_of::<TreeView<u8>>(),
            "BinaryTree's Option cell must niche into TreeView's tag"
        );
        assert_eq!(
            size_of::<BinaryTree<f64>>(),
            size_of::<TreeView<f64>>(),
            "…for a wider payload too"
        );

        // `Cofree`: exactly one word over its cell — no more, and no less to be
        // had without a mechanism the carrier does not have (see `Cofree`).
        assert_eq!(
            size_of::<Cofree<OptionWitness, u32>>(),
            size_of::<CofreeCell<OptionWitness, u32>>() + size_of::<usize>(),
            "Cofree costs exactly one word over its cell"
        );
        assert_eq!(
            size_of::<Cofree<OptionWitness, f64>>(),
            size_of::<CofreeCell<OptionWitness, f64>>() + size_of::<usize>(),
            "…for a wider label too"
        );
        assert_eq!(
            size_of::<Cofree<TreeEndo<u8>, f64>>(),
            size_of::<CofreeCell<TreeEndo<u8>, f64>>() + size_of::<usize>(),
            "…and at the branching witness"
        );
        assert_eq!(
            size_of::<Cofree<TreeEndo<u8>, usize>>(),
            size_of::<CofreeCell<TreeEndo<u8>, usize>>() + size_of::<usize>(),
            "…there too, for a wider label"
        );

        // The exact 64-bit layout of every carrier the reshape touched. A
        // 32-bit host has a different (still at-most-one-word) answer, so the
        // byte counts are gated while the relations above are not.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<Free<OptionWitness, u64>>(), 16);
            assert_eq!(size_of::<FreeView<OptionWitness, u64>>(), 16);
            assert_eq!(size_of::<Free<TreeEndo<u8>, u8>>(), 24);
            assert_eq!(size_of::<FreeView<TreeEndo<u8>, u8>>(), 24);
            // 16, not the 24 the two-`Box` `TreeView::Node` measured.
            assert_eq!(size_of::<BinaryTree<u8>>(), 16);
            assert_eq!(size_of::<TreeView<u8>>(), 16);
            assert_eq!(size_of::<BinaryTree<f64>>(), 16);
            assert_eq!(size_of::<TreeView<f64>>(), 16);
            assert_eq!(size_of::<Cofree<OptionWitness, u32>>(), 24);
            assert_eq!(size_of::<CofreeCell<OptionWitness, u32>>(), 16);
            assert_eq!(size_of::<Cofree<OptionWitness, f64>>(), 24);
            assert_eq!(size_of::<CofreeCell<OptionWitness, f64>>(), 16);
            assert_eq!(size_of::<Cofree<TreeEndo<u8>, f64>>(), 32);
            assert_eq!(size_of::<CofreeCell<TreeEndo<u8>, f64>>(), 24);
            assert_eq!(size_of::<Cofree<TreeEndo<u8>, usize>>(), 32);
            assert_eq!(size_of::<CofreeCell<TreeEndo<u8>, usize>>(), 24);
        }
    }
}
