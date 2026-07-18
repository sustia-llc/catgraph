//! `MatKron(R)` — `FdVect` with the **Kronecker** tensor: a genuine hypergraph
//! category (Fong & Spivak 2019, *Hypergraph Categories* arXiv:1806.08304v3,
//! Ex 2.16, §2.3).
//!
//! This is a concrete (semantic) re-expression onto catgraph's **native**
//! [`Monoidal`] / [`Composable`] / [`SymmetricMonoidalMorphism`] traits. It is
//! a sibling carrier to [`MatR`]: both wrap the same
//! row-major matrix data, but
//!
//! | | [`MatR`] (`Mat(R)`) | [`MatKron`] (`MatKron(R)`) |
//! |---|---|---|
//! | Tensor `a ⊗ b` | `a + b` (block-diagonal ⊕) | `a · b` (Kronecker) |
//! | Monoidal unit | object `0` | object `1` |
//! | SCFM | addition (fails speciality) | **Hadamard** (special) |
//! | Hypergraph category? | no | **yes** |
//!
//! Because [`MatR`] already carries the block-diagonal [`Monoidal`] impl,
//! `MatKron` is a distinct newtype so that the Kronecker [`Monoidal`] impl does
//! not collide.
//!
//! # Conventions (inherited from [`MatR`])
//!
//! **Row-vector convention.** A morphism `a → b` is an `a × b` matrix (rows =
//! domain arity, cols = codomain arity); composition `self ; other` is
//! row-major [`matmul`](crate::mat::MatR::matmul). Objects are dimensions
//! `usize`, encoded as `Vec<()>` (`domain()` returns `vec![(); rows]`) to
//! mirror [`MatR`].
//!
//! # Hadamard SCFM
//!
//! Every object `n` carries a genuine special commutative Frobenius monoid,
//! realized here as **inherent generators** (`eta`/`epsilon`/`mu`/`delta`)
//! rather than a separate trait. Speciality `δ ; μ = id_n` holds (the marquee
//! property that makes this a hypergraph category, not merely compact closed).

use catgraph::{
    category::{Composable, HasIdentity},
    errors::CatgraphError,
    monoidal::{Monoidal, MonoidalMorphism, SymmetricMonoidalMorphism},
};
use permutations::Permutation;

use crate::mat::MatR;
use crate::rig::Rig;

/// A matrix carrier over a rig `R` whose monoidal product is the **Kronecker
/// product** and which carries the Hadamard SCFM on every object — a genuine
/// hypergraph category (F&S 2019 Ex 2.16).
///
/// Row-vector convention: a morphism `a → b` is an `a × b` matrix. Wraps a
/// [`MatR`] for the underlying storage and matmul/identity machinery.
#[derive(Clone, Debug, PartialEq)]
pub struct MatKron<R: Rig>(MatR<R>);

impl<R: Rig> MatKron<R> {
    /// Wrap an existing [`MatR`] as a `MatKron` morphism.
    #[must_use]
    pub fn from_mat(inner: MatR<R>) -> Self {
        Self(inner)
    }

    /// The underlying [`MatR`].
    #[must_use]
    pub fn inner(&self) -> &MatR<R> {
        &self.0
    }

    /// Number of rows (domain arity).
    #[must_use]
    pub fn rows(&self) -> usize {
        self.0.rows()
    }

    /// Number of columns (codomain arity).
    #[must_use]
    pub fn cols(&self) -> usize {
        self.0.cols()
    }

    /// Row-major entries `entries[i][j]`.
    #[must_use]
    pub fn entries(&self) -> &[Vec<R>] {
        self.0.entries()
    }

    /// The `n × n` identity morphism `n → n`.
    #[must_use]
    pub fn identity(n: usize) -> Self {
        Self(MatR::identity(n))
    }

    /// The all-zeros `rows × cols` morphism.
    #[must_use]
    pub fn zero_matrix(rows: usize, cols: usize) -> Self {
        Self(MatR::zero_matrix(rows, cols))
    }

    /// Kronecker product. For `self` of shape `a × b` and `other` of shape
    /// `c × d`, the result is `(a·c) × (b·d)` with
    /// `result[i*c + k][j*d + l] = self[i][j] * other[k][l]`.
    ///
    /// This is the monoidal tensor of `MatKron(R)`: `a ⊗ b = a · b` on objects.
    #[must_use]
    pub fn kron(&self, other: &Self) -> Self {
        let a = self.rows();
        let b = self.cols();
        let c = other.rows();
        let d = other.cols();
        let s = self.entries();
        let o = other.entries();
        let mut entries = vec![vec![R::zero(); b * d]; a * c];
        for i in 0..a {
            for j in 0..b {
                let aij = s[i][j].clone();
                for k in 0..c {
                    for l in 0..d {
                        entries[i * c + k][j * d + l] = aij.clone() * o[k][l].clone();
                    }
                }
            }
        }
        // Dimensions are exact by construction; `new` cannot fail here.
        Self(
            MatR::new(a * c, b * d, entries)
                .expect("invariant: kron builds an (a*c)x(b*d) rectangular matrix"),
        )
    }

    /// Hadamard SCFM unit `η : 1 → n` (shape `1 × n`, all entries `1`).
    #[must_use]
    pub fn eta(n: usize) -> Self {
        Self(
            MatR::new(1, n, vec![vec![R::one(); n]])
                .expect("invariant: eta builds a 1xn rectangular matrix"),
        )
    }

    /// Hadamard SCFM counit `ε : n → 1` (shape `n × 1`, all entries `1`).
    #[must_use]
    pub fn epsilon(n: usize) -> Self {
        Self(
            MatR::new(n, 1, vec![vec![R::one(); 1]; n])
                .expect("invariant: epsilon builds an nx1 rectangular matrix"),
        )
    }

    /// Hadamard SCFM multiplication `μ : n⊗n = n² → n` (shape `n² × n`).
    ///
    /// Row index encodes `(i, j)` as `i*n + j`; `mu[i*n + j][k] = 1` iff
    /// `i == j && j == k`, else `0`. (Pointwise product:
    /// `μ(e_i ⊗ e_j) = δ_ij · e_i`.)
    #[must_use]
    pub fn mu(n: usize) -> Self {
        let mut entries = vec![vec![R::zero(); n]; n * n];
        for i in 0..n {
            entries[i * n + i][i] = R::one();
        }
        Self(
            MatR::new(n * n, n, entries)
                .expect("invariant: mu builds an (n*n)xn rectangular matrix"),
        )
    }

    /// Hadamard SCFM comultiplication `δ : n → n⊗n = n²` (shape `n × n²`).
    ///
    /// Col index encodes `(j, k)` as `j*n + k`; `delta[i][j*n + k] = 1` iff
    /// `i == j && j == k`, else `0`. (Duplication: `δ(e_i) = e_i ⊗ e_i`.)
    #[must_use]
    pub fn delta(n: usize) -> Self {
        let mut entries = vec![vec![R::zero(); n * n]; n];
        for i in 0..n {
            entries[i][i * n + i] = R::one();
        }
        Self(
            MatR::new(n, n * n, entries)
                .expect("invariant: delta builds an nx(n*n) rectangular matrix"),
        )
    }

    /// Compact-closed cup `I = 1 → n²`, i.e. `η ; δ` (shape `1 × n²`).
    ///
    /// Equivalently a `1` in column `j*n + k` exactly where `j == k`.
    ///
    /// # Panics
    ///
    /// Panics only if the internal `eta ; delta` composition shape invariant is
    /// violated, which is unreachable by construction.
    #[must_use]
    pub fn cup(n: usize) -> Self {
        Self::eta(n)
            .compose(&Self::delta(n))
            .expect("invariant: eta(1xn) ; delta(nxn^2) composes to 1xn^2")
    }

    /// Compact-closed cap `n² → I = 1`, i.e. `μ ; ε` (shape `n² × 1`).
    ///
    /// Equivalently a `1` in row `i*n + j` exactly where `i == j`.
    ///
    /// # Panics
    ///
    /// Panics only if the internal `mu ; epsilon` composition shape invariant is
    /// violated, which is unreachable by construction.
    #[must_use]
    pub fn cap(n: usize) -> Self {
        Self::mu(n)
            .compose(&Self::epsilon(n))
            .expect("invariant: mu(n^2xn) ; epsilon(nx1) composes to n^2x1")
    }

    /// The braiding `σ : a⊗b = a·b → b·a = b·a` — the perfect-shuffle
    /// permutation matrix of shape `(a·b) × (a·b)` with
    /// `result[i*b + j][j*a + i] = 1` for `i in 0..a, j in 0..b`, else `0`.
    /// Maps the basis vector `e_i ⊗ e_j` to `e_j ⊗ e_i`.
    #[must_use]
    pub fn braiding(a: usize, b: usize) -> Self {
        let n = a * b;
        let mut entries = vec![vec![R::zero(); n]; n];
        for i in 0..a {
            for j in 0..b {
                entries[i * b + j][j * a + i] = R::one();
            }
        }
        Self(
            MatR::new(n, n, entries)
                .expect("invariant: braiding builds a (a*b)x(a*b) rectangular matrix"),
        )
    }
}

// ---- Category / monoidal trait impls ----

impl<R: Rig> HasIdentity<Vec<()>> for MatKron<R> {
    fn identity(on_this: &Vec<()>) -> Self {
        Self::identity(on_this.len())
    }
}

impl<R: Rig> Composable<Vec<()>> for MatKron<R> {
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
        Ok(Self(self.0.matmul(&other.0)?))
    }

    fn domain(&self) -> Vec<()> {
        vec![(); self.rows()]
    }

    fn codomain(&self) -> Vec<()> {
        vec![(); self.cols()]
    }
}

impl<R: Rig> Monoidal for MatKron<R> {
    fn monoidal(&mut self, other: Self) {
        *self = self.kron(&other);
    }
}

impl<R: Rig> MonoidalMorphism<Vec<()>> for MatKron<R> {}

// Both the permutation-matrix construction and the pre/post-multiply logic are
// identical to `MatR`'s (composition is matmul for both carriers), so these
// delegate to the inner `MatR` rather than duplicating that machinery.
impl<R: Rig> SymmetricMonoidalMorphism<()> for MatKron<R> {
    fn from_permutation(
        p: Permutation,
        _types: &[()],
        _types_as_on_domain: bool,
    ) -> Result<Self, CatgraphError> {
        Ok(Self(MatR::permutation_matrix(&p)))
    }

    fn permute_side(&mut self, p: &Permutation, of_codomain: bool) {
        self.0.permute_side(p, of_codomain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::{BoolRig, F64Rig};

    type M = MatKron<F64Rig>;

    // 1. Kronecker dims + id ⊗ id = id.
    #[test]
    fn kron_dims_and_identity_tensor() {
        let a = M::zero_matrix(2, 3);
        let b = M::zero_matrix(4, 5);
        let k = a.kron(&b);
        assert_eq!(k.rows(), 8);
        assert_eq!(k.cols(), 15);

        let id_kron = M::identity(2).kron(&M::identity(3));
        assert_eq!(id_kron, M::identity(6));
    }

    // 2. Speciality (marquee gate): delta ; mu = id_n.
    #[test]
    fn speciality_delta_then_mu_is_identity() {
        for n in [2usize, 3, 5] {
            let prod = M::delta(n).compose(&M::mu(n)).unwrap();
            assert_eq!(
                prod,
                M::identity(n),
                "speciality failed for n={n} over F64Rig"
            );
        }
        // Also over BoolRig for n=2.
        let prod = MatKron::<BoolRig>::delta(2)
            .compose(&MatKron::<BoolRig>::mu(2))
            .unwrap();
        assert_eq!(
            prod,
            MatKron::<BoolRig>::identity(2),
            "speciality failed for n=2 over BoolRig"
        );
    }

    // 3. mu associativity: (mu ⊗ id) ; mu == (id ⊗ mu) ; mu.
    #[test]
    fn mu_associativity() {
        let n = 2usize;
        let mu = M::mu(n);
        let id = M::identity(n);
        let left = mu.kron(&id).compose(&mu).unwrap();
        let right = id.kron(&mu).compose(&mu).unwrap();
        assert_eq!(left, right);
    }

    // 4. delta coassociativity: delta ; (delta ⊗ id) == delta ; (id ⊗ delta).
    #[test]
    fn delta_coassociativity() {
        let n = 2usize;
        let delta = M::delta(n);
        let id = M::identity(n);
        let left = delta.compose(&delta.kron(&id)).unwrap();
        let right = delta.compose(&id.kron(&delta)).unwrap();
        assert_eq!(left, right);
    }

    // 5. Frobenius law: (delta ⊗ id);(id ⊗ mu) == mu;delta == (id ⊗ delta);(mu ⊗ id).
    #[test]
    fn frobenius_law() {
        let n = 2usize;
        let mu = M::mu(n);
        let delta = M::delta(n);
        let id = M::identity(n);

        let left = delta.kron(&id).compose(&id.kron(&mu)).unwrap();
        let middle = mu.compose(&delta).unwrap();
        let right = id.kron(&delta).compose(&mu.kron(&id)).unwrap();

        assert_eq!(left, middle, "Frobenius left = middle failed");
        assert_eq!(middle, right, "Frobenius middle = right failed");
    }

    // 6. Commutativity: braiding(n,n) ; mu == mu.
    #[test]
    fn mu_commutativity() {
        let n = 2usize;
        let mu = M::mu(n);
        let braided = M::braiding(n, n).compose(&mu).unwrap();
        assert_eq!(braided, mu);
    }

    // 7. Unit laws: (eta ⊗ id) ; mu == id and (id ⊗ eta) ; mu == id.
    #[test]
    fn unit_laws() {
        let n = 2usize;
        let mu = M::mu(n);
        let eta = M::eta(n);
        let id = M::identity(n);

        let left = eta.kron(&id).compose(&mu).unwrap();
        assert_eq!(left, id, "(eta ⊗ id) ; mu = id failed");

        let right = id.kron(&eta).compose(&mu).unwrap();
        assert_eq!(right, id, "(id ⊗ eta) ; mu = id failed");
    }

    // 8. cup/cap dims.
    #[test]
    fn cup_cap_dims() {
        let cup = M::cup(2);
        assert_eq!(cup.rows(), 1);
        assert_eq!(cup.cols(), 4);

        let cap = M::cap(2);
        assert_eq!(cap.rows(), 4);
        assert_eq!(cap.cols(), 1);
    }

    // 9. Snake (zigzag) equations.
    #[test]
    fn snake_equations() {
        for n in [2usize, 3] {
            let id = M::identity(n);
            let cup = M::cup(n);
            let cap = M::cap(n);

            // Right snake: (id ⊗ cup) ; (cap ⊗ id) = id.
            let right = id.kron(&cup).compose(&cap.kron(&id)).unwrap();
            assert_eq!(right, id, "right snake failed for n={n}");

            // Left snake (dual): (cup ⊗ id) ; (id ⊗ cap) = id.
            let left = cup.kron(&id).compose(&id.kron(&cap)).unwrap();
            assert_eq!(left, id, "left snake failed for n={n}");
        }
    }

    // 10. braiding involution: braiding(a,b) ; braiding(b,a) = id_{a*b}.
    #[test]
    fn braiding_involution() {
        let (a, b) = (2usize, 3usize);
        let composed = M::braiding(a, b).compose(&M::braiding(b, a)).unwrap();
        assert_eq!(composed, M::identity(a * b));
    }
}
