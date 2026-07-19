//! Five neural-network architectures as parametric (co)algebras.
//!
//! CDL Appendix I + Appendix J — the central catalogue of CDL: five
//! standard NN architectures arise as `Para(F)` (co)algebras for specific
//! endofunctors.
//!
//! | Architecture       | Endofunctor          | Type         |
//! |--------------------|----------------------|--------------|
//! | Folding RNN        | `1 + A × −`          | Algebra      |
//! | Unfolding RNN      | `O × −`              | Coalgebra    |
//! | Recursive NN       | `A + (−)²`           | Algebra      |
//! | Full RNN (Mealy)   | `I → O × −`          | Coalgebra    |
//! | Moore Machine NN   | `O × (I → −)`        | Coalgebra    |
//!
//! ## Unrollers
//!
//! Beyond the typed wrappers, each type has an **unrolling**
//! method that turns a `cell` into a function over the corresponding
//! inductive/coinductive carrier (CDL Examples J.1–J.5; uniqueness by
//! Remark 2.13 for the algebra unrollers and Remark H.6 for the
//! coalgebra unrollers):
//!
//! - [`FoldingRnn::unroll`] — right-fold over `Vec<A>`, the initial
//!   algebra of `1 + A × −`.
//! - [`RecursiveNn::unroll`] — post-order walk over
//!   [`crate::free_monad::tree_endo::BinaryTree`], the initial algebra
//!   of `A + (−)²`.
//! - [`UnfoldingRnn::unroll_to_vec`] — bounded-depth coalgebra unfolding
//!   into `Vec<O>`.
//! - [`MealyCell::run`] — left-to-right stream-process of inputs into
//!   per-step outputs.
//! - [`MooreCell::run`] — output-then-step Moore stream-process.
//!
//! Each unroller is the unique algebra (resp. coalgebra) homomorphism
//! between the relevant initial / final carrier and the cell's
//! (co)algebra. The `tests/architecture_unrollers.rs` harness includes a
//! direct *FreeMnd-equivalence* test (`unroll(cell, vec) ==
//! unroll_via_free_mnd(cell, vec_to_free_mnd(vec, ()))`) demonstrating
//! the central CDL claim that **the unroller IS the algebra
//! homomorphism from the initial algebra of the free monad**.

mod folding_rnn;
mod mealy_cell;
mod moore_cell;
mod recursive_nn;
mod unfolding_rnn;

pub use folding_rnn::FoldingRnn;
pub use mealy_cell::MealyCell;
pub use moore_cell::MooreCell;
pub use recursive_nn::RecursiveNn;
pub use unfolding_rnn::UnfoldingRnn;
