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
//! Unrollers (CDL Ex J.1–J.5; Remark 2.13 for algebras, Remark H.6 for
//! coalgebras): [`FoldingRnn::unroll`] (right fold over `Vec<A>`),
//! [`RecursiveNn::unroll`] (post-order over
//! [`crate::free_monad::tree_endo::BinaryTree`]),
//! [`UnfoldingRnn::unroll_to_vec`] / `unroll_iter`, [`MealyCell::run`],
//! [`MooreCell::run`]. All infallible.

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
