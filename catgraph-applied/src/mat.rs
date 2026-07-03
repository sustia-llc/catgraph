//! Matrix prop `Mat(R)` over a rig R (F&S 2018 Def 5.50).
//!
//! Objects are ℕ. A morphism `m → n` is an `m × n` matrix, i.e. **rows =
//! domain arity, cols = codomain arity** (per Def 5.50 + Remark 5.49 row-vector
//! convention). Composition is `v ; A` row-major matmul:
//! `(M ; N)(i, c) = Σ_b M(i, b) · N(b, c)`. Monoidal tensor is block-diagonal
//! sum.
//!
//! This module does NOT use nalgebra — `Mat(R)` over an arbitrary rig may fail
//! nalgebra's field-like trait bounds (`Tropical`, `BoolRig`, any semiring
//! without subtraction). An nalgebra bridge specialized to `F64Rig` may be
//! added in a later release behind a feature flag.

use catgraph::{
    category::{Composable, HasIdentity},
    errors::CatgraphError,
    monoidal::{Monoidal, MonoidalMorphism, SymmetricMonoidalMorphism},
};
use permutations::Permutation;

use crate::rig::Rig;

/// An `m × n` matrix over a rig `R`, representing a morphism `m → n` of
/// `Mat(R)` (F&S Def 5.50).
///
/// Row-major layout: `entries[i][j]` is the entry `M(i, j)`.
#[derive(Clone, Debug, PartialEq)]
pub struct MatR<R: Rig> {
    rows: usize,
    cols: usize,
    entries: Vec<Vec<R>>,
}

impl<R: Rig> MatR<R> {
    /// Construct a matrix, validating that `entries.len() == rows` and each
    /// inner `entries[i].len() == cols`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::Composition`] on shape mismatch.
    pub fn new(rows: usize, cols: usize, entries: Vec<Vec<R>>) -> Result<Self, CatgraphError> {
        if entries.len() != rows {
            return Err(CatgraphError::Composition {
                message: format!("expected {rows} rows, got {}", entries.len()),
            });
        }
        for (i, row) in entries.iter().enumerate() {
            if row.len() != cols {
                return Err(CatgraphError::Composition {
                    message: format!("row {i} has {} cols, expected {cols}", row.len()),
                });
            }
        }
        Ok(Self {
            rows,
            cols,
            entries,
        })
    }

    /// The `n × n` identity matrix.
    #[must_use]
    pub fn identity(n: usize) -> Self {
        let mut entries = vec![vec![R::zero(); n]; n];
        for (i, row) in entries.iter_mut().enumerate().take(n) {
            row[i] = R::one();
        }
        Self {
            rows: n,
            cols: n,
            entries,
        }
    }

    /// The all-zeros `rows × cols` matrix.
    #[must_use]
    pub fn zero_matrix(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            entries: vec![vec![R::zero(); cols]; rows],
        }
    }

    /// Matrix multiplication: `self ; other` where `self: m × n` and
    /// `other: n × p`, producing an `m × p` matrix.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError::CompositionSizeMismatch`] if
    /// `self.cols != other.rows`.
    pub fn matmul(&self, other: &Self) -> Result<Self, CatgraphError> {
        if self.cols != other.rows {
            return Err(CatgraphError::CompositionSizeMismatch {
                expected: self.cols,
                actual: other.rows,
            });
        }
        let mut result = vec![vec![R::zero(); other.cols]; self.rows];
        for (i, out_row) in result.iter_mut().enumerate().take(self.rows) {
            for (c, out_cell) in out_row.iter_mut().enumerate().take(other.cols) {
                let mut sum = R::zero();
                for b in 0..self.cols {
                    sum = sum + (self.entries[i][b].clone() * other.entries[b][c].clone());
                }
                *out_cell = sum;
            }
        }
        Ok(Self {
            rows: self.rows,
            cols: other.cols,
            entries: result,
        })
    }

    /// Block-diagonal sum: `[[self, 0], [0, other]]`.
    #[must_use]
    pub fn block_diagonal(&self, other: &Self) -> Self {
        let new_rows = self.rows + other.rows;
        let new_cols = self.cols + other.cols;
        let mut entries = vec![vec![R::zero(); new_cols]; new_rows];
        for (i, src_row) in self.entries.iter().enumerate() {
            for (j, src_cell) in src_row.iter().enumerate() {
                entries[i][j] = src_cell.clone();
            }
        }
        for (i, src_row) in other.entries.iter().enumerate() {
            for (j, src_cell) in src_row.iter().enumerate() {
                entries[self.rows + i][self.cols + j] = src_cell.clone();
            }
        }
        Self {
            rows: new_rows,
            cols: new_cols,
            entries,
        }
    }

    /// Matrix from a permutation: `n × n` matrix with `entries[i][p(i)] = 1`.
    #[must_use]
    pub fn permutation_matrix(p: &Permutation) -> Self {
        let n = p.len();
        let mut entries = vec![vec![R::zero(); n]; n];
        for (i, row) in entries.iter_mut().enumerate().take(n) {
            row[p.apply(i)] = R::one();
        }
        Self {
            rows: n,
            cols: n,
            entries,
        }
    }

    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    #[must_use]
    pub fn entries(&self) -> &[Vec<R>] {
        &self.entries
    }

    /// Mutable access to a single entry. Returns `None` if `(row, col)` is out of bounds.
    ///
    /// Substrate for in-place row/column operations in
    /// `catgraph-magnitude`'s SNF Phase 1 + Phase 2 (Storjohann 2000).
    pub fn entry_mut(&mut self, row: usize, col: usize) -> Option<&mut R> {
        self.entries.get_mut(row)?.get_mut(col)
    }

    /// Mutable access to the underlying `Vec<Vec<R>>` storage.
    ///
    /// Caller is responsible for preserving rectangularity — if any row is resized,
    /// `rows()`/`cols()` will return stale values.
    pub fn entries_mut(&mut self) -> &mut [Vec<R>] {
        &mut self.entries
    }

    /// Swap two rows in place. Returns `Err(CatgraphError::Composition)` if
    /// either index is out of bounds.
    ///
    /// Substrate for Storjohann SNF row operations.
    ///
    /// # Errors
    ///
    /// Out-of-bounds row index.
    pub fn row_swap(&mut self, i: usize, j: usize) -> Result<(), CatgraphError> {
        if i >= self.rows || j >= self.rows {
            return Err(CatgraphError::Composition {
                message: format!(
                    "MatR::row_swap: index out of bounds (rows={}, i={}, j={})",
                    self.rows, i, j
                ),
            });
        }
        self.entries.swap(i, j);
        Ok(())
    }

    /// Scale a single row in place: `row_i ← factor * row_i`.
    ///
    /// Substrate for Storjohann SNF row operations.
    ///
    /// # Errors
    ///
    /// Out-of-bounds row index.
    pub fn scale_row(&mut self, row: usize, factor: &R) -> Result<(), CatgraphError> {
        if row >= self.rows {
            return Err(CatgraphError::Composition {
                message: format!(
                    "MatR::scale_row: index out of bounds (rows={}, row={})",
                    self.rows, row
                ),
            });
        }
        for col in 0..self.cols {
            let v = self.entries[row][col].clone();
            self.entries[row][col] = factor.clone() * v;
        }
        Ok(())
    }

    /// Add a scaled copy of one row into another: `row_dst ← row_dst + factor * row_src`.
    ///
    /// `dst == src` is rejected; the Storjohann SNF call sites always operate on
    /// distinct rows. (Self-scaling is `scale_row(row, 1+factor)` if needed.)
    ///
    /// Substrate for Storjohann SNF row operations.
    ///
    /// # Errors
    ///
    /// - `dst == src` (use `scale_row` instead).
    /// - `dst` or `src` out of bounds.
    pub fn add_scaled_row(
        &mut self,
        dst: usize,
        src: usize,
        factor: &R,
    ) -> Result<(), CatgraphError> {
        if dst == src {
            return Err(CatgraphError::Composition {
                message: "MatR::add_scaled_row: dst == src not supported (use scale_row)".into(),
            });
        }
        if dst >= self.rows || src >= self.rows {
            return Err(CatgraphError::Composition {
                message: format!(
                    "MatR::add_scaled_row: index out of bounds (rows={}, dst={}, src={})",
                    self.rows, dst, src
                ),
            });
        }
        // TODO(perf, #37): in the SNF inner loop, this `dst.clone() + factor*src`
        // pattern allocates a new R per cell. Consider `mem::replace` to avoid the
        // dst-side clone if benches show it as a hotspot.
        for col in 0..self.cols {
            let s = self.entries[src][col].clone();
            let scaled = factor.clone() * s;
            let d = self.entries[dst][col].clone();
            self.entries[dst][col] = d + scaled;
        }
        Ok(())
    }

    /// Swap two columns in place.
    ///
    /// Substrate for Storjohann SNF column operations
    /// (column-axis dual of [`Self::row_swap`]).
    ///
    /// # Errors
    ///
    /// Out-of-bounds column index.
    pub fn col_swap(&mut self, i: usize, j: usize) -> Result<(), CatgraphError> {
        if i >= self.cols || j >= self.cols {
            return Err(CatgraphError::Composition {
                message: format!(
                    "MatR::col_swap: index out of bounds (cols={}, i={}, j={})",
                    self.cols, i, j
                ),
            });
        }
        for row in 0..self.rows {
            self.entries[row].swap(i, j);
        }
        Ok(())
    }

    /// Scale a single column in place: `col_c ← factor * col_c`.
    ///
    /// Substrate for Storjohann SNF column operations
    /// (column-axis dual of [`Self::scale_row`]).
    ///
    /// # Errors
    ///
    /// Out-of-bounds column index.
    pub fn scale_col(&mut self, col: usize, factor: &R) -> Result<(), CatgraphError> {
        if col >= self.cols {
            return Err(CatgraphError::Composition {
                message: format!(
                    "MatR::scale_col: index out of bounds (cols={}, col={})",
                    self.cols, col
                ),
            });
        }
        for row in 0..self.rows {
            let v = self.entries[row][col].clone();
            self.entries[row][col] = factor.clone() * v;
        }
        Ok(())
    }

    /// Add a scaled copy of one column into another: `col_dst ← col_dst + factor * col_src`.
    ///
    /// Substrate for Storjohann SNF column operations
    /// (column-axis dual of [`Self::add_scaled_row`]); `dst == src` rejected.
    ///
    /// # Errors
    ///
    /// - `dst == src` (use `scale_col` instead).
    /// - `dst` or `src` out of bounds.
    pub fn add_scaled_col(
        &mut self,
        dst: usize,
        src: usize,
        factor: &R,
    ) -> Result<(), CatgraphError> {
        if dst == src {
            return Err(CatgraphError::Composition {
                message: "MatR::add_scaled_col: dst == src not supported (use scale_col)".into(),
            });
        }
        if dst >= self.cols || src >= self.cols {
            return Err(CatgraphError::Composition {
                message: format!(
                    "MatR::add_scaled_col: index out of bounds (cols={}, dst={}, src={})",
                    self.cols, dst, src
                ),
            });
        }
        for row in 0..self.rows {
            let s = self.entries[row][src].clone();
            let scaled = factor.clone() * s;
            let d = self.entries[row][dst].clone();
            self.entries[row][dst] = d + scaled;
        }
        Ok(())
    }
}

// ---- Category / monoidal trait impls ----

impl<R: Rig> HasIdentity<Vec<()>> for MatR<R> {
    fn identity(on_this: &Vec<()>) -> Self {
        Self::identity(on_this.len())
    }
}

impl<R: Rig> Composable<Vec<()>> for MatR<R> {
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        self.matmul(other)
    }

    fn domain(&self) -> Vec<()> {
        vec![(); self.rows]
    }

    fn codomain(&self) -> Vec<()> {
        vec![(); self.cols]
    }
}

impl<R: Rig> Monoidal for MatR<R> {
    fn monoidal(&mut self, other: Self) {
        *self = self.block_diagonal(&other);
    }
}

impl<R: Rig> MonoidalMorphism<Vec<()>> for MatR<R> {}

impl<R: Rig> SymmetricMonoidalMorphism<()> for MatR<R> {
    fn from_permutation(
        p: Permutation,
        _types: &[()],
        _types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError> {
        Ok(Self::permutation_matrix(&p))
    }

    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        // Right-mul by P permutes columns (codomain side); left-mul by P^T
        // permutes rows (domain side). Length-mismatch is defensive no-op
        // to match the trait's non-fallible signature.
        let expected = if of_codomain { self.cols } else { self.rows };
        if p.len() != expected {
            return;
        }
        let perm_mat = Self::permutation_matrix(p);
        if of_codomain {
            if let Ok(result) = self.matmul(&perm_mat) {
                *self = result;
            }
        } else {
            // P^T has entries[p(i)][i] = 1; equivalently, the transpose of P.
            let n = p.len();
            let mut entries = vec![vec![R::zero(); n]; n];
            for i in 0..n {
                entries[p.apply(i)][i] = R::one();
            }
            let p_transpose = Self {
                rows: n,
                cols: n,
                entries,
            };
            if let Ok(result) = p_transpose.matmul(self) {
                *self = result;
            }
        }
    }
}
