//! `MatKron(R)` — `FdVect` with the **Kronecker** tensor: a genuine hypergraph
//! category (Fong & Spivak 2019, *Hypergraph Categories* arXiv:1806.08304v3,
//! Ex 2.16, §2.3), expressed on catgraph's native [`Monoidal`] /
//! [`Composable`] / [`SymmetricMonoidalMorphism`] traits.
//!
//! A sibling carrier to [`MatR`]: both wrap the same row-major matrix data, but
//!
//! | | [`MatR`] (`Mat(R)`) | [`MatKron`] (`MatKron(R)`) |
//! |---|---|---|
//! | Tensor `a ⊗ b` | `a + b` (block-diagonal ⊕) | `a · b` (Kronecker) |
//! | Monoidal unit | object `0` | object `1` |
//! | SCFM | none | **Hadamard** (special) |
//! | Hypergraph category? | no | **yes** |
//!
//! **Row-vector convention**, inherited from [`MatR`]: a morphism `a → b` is an
//! `a × b` matrix (rows = domain arity, cols = codomain arity); composition
//! `self ; other` is row-major [`matmul`](crate::mat::MatR::matmul). Objects are
//! dimensions `usize`, encoded as `Vec<()>`, so `domain()` returns `vec![(); rows]`.
//!
//! Every object `n` carries a special commutative Frobenius monoid, realized as
//! the inherent generators `eta`/`epsilon`/`mu`/`delta` rather than a separate
//! trait; speciality `δ ; μ = id_n` and the other SCFM laws below are
//! property-tested over `n ∈ {0, 1, 2, 3}` (`n = 0` collapsing every
//! generator to a `0`-dimensioned matrix), for both `F64Rig` and `BoolRig`,
//! in the `tests` module below.

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

    /// The braiding `σ : a⊗b → b⊗a` — the perfect-shuffle permutation matrix of
    /// shape `(a·b) × (a·b)` with `result[i*b + j][j*a + i] = 1` for
    /// `i in 0..a, j in 0..b`, else `0`. Maps `e_i ⊗ e_j` to `e_j ⊗ e_i`.
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

// Composition is matmul for both carriers and both have `Vec<()>` objects, so
// these delegate to `MatR` rather than duplicating the permutation machinery.
impl<R: Rig> SymmetricMonoidalMorphism<()> for MatKron<R> {
    fn from_permutation_on_domain(p: Permutation, types: &[()]) -> Result<Self, CatgraphError> {
        MatR::from_permutation_on_domain(p, types).map(Self)
    }

    fn from_permutation_on_codomain(p: Permutation, types: &[()]) -> Result<Self, CatgraphError> {
        MatR::from_permutation_on_codomain(p, types).map(Self)
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

    /// Object-arity sweep shared by every law test below. `n = 0` collapses
    /// every generator to a `0`-dimensioned matrix (`MatR::new` has no
    /// rows>0/cols>0 requirement — mat.rs — so these shapes build and
    /// compose without special-casing); `n = 1` is the smallest nontrivial
    /// case; `n ∈ {2, 3}` exercise genuine cross terms.
    const N_SWEEP: [usize; 4] = [0, 1, 2, 3];

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

    // 2. Speciality (marquee gate): delta ; mu = id_n. n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_speciality<R: Rig + std::fmt::Debug>(n: usize) {
        let prod = MatKron::<R>::delta(n)
            .compose(&MatKron::<R>::mu(n))
            .unwrap();
        assert_eq!(
            prod,
            MatKron::<R>::identity(n),
            "speciality failed for n={n}"
        );
    }

    #[test]
    fn speciality_delta_then_mu_is_identity() {
        for n in N_SWEEP {
            check_speciality::<F64Rig>(n);
            check_speciality::<BoolRig>(n);
        }
    }

    // 3. mu associativity: (mu ⊗ id) ; mu == (id ⊗ mu) ; mu. n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_mu_associativity<R: Rig + std::fmt::Debug>(n: usize) {
        let mu = MatKron::<R>::mu(n);
        let id = MatKron::<R>::identity(n);
        let left = mu.kron(&id).compose(&mu).unwrap();
        let right = id.kron(&mu).compose(&mu).unwrap();
        assert_eq!(left, right, "mu associativity failed for n={n}");
    }

    #[test]
    fn mu_associativity() {
        for n in N_SWEEP {
            check_mu_associativity::<F64Rig>(n);
            check_mu_associativity::<BoolRig>(n);
        }
    }

    // 4. delta coassociativity: delta ; (delta ⊗ id) == delta ; (id ⊗ delta).
    // n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_delta_coassociativity<R: Rig + std::fmt::Debug>(n: usize) {
        let delta = MatKron::<R>::delta(n);
        let id = MatKron::<R>::identity(n);
        let left = delta.compose(&delta.kron(&id)).unwrap();
        let right = delta.compose(&id.kron(&delta)).unwrap();
        assert_eq!(left, right, "delta coassociativity failed for n={n}");
    }

    #[test]
    fn delta_coassociativity() {
        for n in N_SWEEP {
            check_delta_coassociativity::<F64Rig>(n);
            check_delta_coassociativity::<BoolRig>(n);
        }
    }

    // 5. Frobenius law: (delta ⊗ id);(id ⊗ mu) == mu;delta == (id ⊗ delta);(mu ⊗ id).
    // n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_frobenius_law<R: Rig + std::fmt::Debug>(n: usize) {
        let mu = MatKron::<R>::mu(n);
        let delta = MatKron::<R>::delta(n);
        let id = MatKron::<R>::identity(n);

        let left = delta.kron(&id).compose(&id.kron(&mu)).unwrap();
        let middle = mu.compose(&delta).unwrap();
        let right = id.kron(&delta).compose(&mu.kron(&id)).unwrap();

        assert_eq!(left, middle, "Frobenius left = middle failed for n={n}");
        assert_eq!(middle, right, "Frobenius middle = right failed for n={n}");
    }

    #[test]
    fn frobenius_law() {
        for n in N_SWEEP {
            check_frobenius_law::<F64Rig>(n);
            check_frobenius_law::<BoolRig>(n);
        }
    }

    // 6. Commutativity: braiding(n,n) ; mu == mu. n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_mu_commutativity<R: Rig + std::fmt::Debug>(n: usize) {
        let mu = MatKron::<R>::mu(n);
        let braided = MatKron::<R>::braiding(n, n).compose(&mu).unwrap();
        assert_eq!(braided, mu, "mu commutativity failed for n={n}");
    }

    #[test]
    fn mu_commutativity() {
        for n in N_SWEEP {
            check_mu_commutativity::<F64Rig>(n);
            check_mu_commutativity::<BoolRig>(n);
        }
    }

    // 7. Unit laws: (eta ⊗ id) ; mu == id and (id ⊗ eta) ; mu == id.
    // n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_unit_laws<R: Rig + std::fmt::Debug>(n: usize) {
        let mu = MatKron::<R>::mu(n);
        let eta = MatKron::<R>::eta(n);
        let id = MatKron::<R>::identity(n);

        let left = eta.kron(&id).compose(&mu).unwrap();
        assert_eq!(left, id, "(eta ⊗ id) ; mu = id failed for n={n}");

        let right = id.kron(&eta).compose(&mu).unwrap();
        assert_eq!(right, id, "(id ⊗ eta) ; mu = id failed for n={n}");
    }

    #[test]
    fn unit_laws() {
        for n in N_SWEEP {
            check_unit_laws::<F64Rig>(n);
            check_unit_laws::<BoolRig>(n);
        }
    }

    // 8. cup/cap dims. n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_cup_cap_dims<R: Rig + std::fmt::Debug>(n: usize) {
        let cup = MatKron::<R>::cup(n);
        assert_eq!(cup.rows(), 1, "cup rows wrong for n={n}");
        assert_eq!(cup.cols(), n * n, "cup cols wrong for n={n}");

        let cap = MatKron::<R>::cap(n);
        assert_eq!(cap.rows(), n * n, "cap rows wrong for n={n}");
        assert_eq!(cap.cols(), 1, "cap cols wrong for n={n}");
    }

    #[test]
    fn cup_cap_dims() {
        for n in N_SWEEP {
            check_cup_cap_dims::<F64Rig>(n);
            check_cup_cap_dims::<BoolRig>(n);
        }
    }

    // 9. Snake (zigzag) equations. n ∈ {0,1,2,3}, F64Rig + BoolRig.
    fn check_snake_equations<R: Rig + std::fmt::Debug>(n: usize) {
        let id = MatKron::<R>::identity(n);
        let cup = MatKron::<R>::cup(n);
        let cap = MatKron::<R>::cap(n);

        // Right snake: (id ⊗ cup) ; (cap ⊗ id) = id.
        let right = id.kron(&cup).compose(&cap.kron(&id)).unwrap();
        assert_eq!(right, id, "right snake failed for n={n}");

        // Left snake (dual): (cup ⊗ id) ; (id ⊗ cap) = id.
        let left = cup.kron(&id).compose(&id.kron(&cap)).unwrap();
        assert_eq!(left, id, "left snake failed for n={n}");
    }

    #[test]
    fn snake_equations() {
        for n in N_SWEEP {
            check_snake_equations::<F64Rig>(n);
            check_snake_equations::<BoolRig>(n);
        }
    }

    // 10. braiding involution: braiding(a,b) ; braiding(b,a) = id_{a*b}.
    // (a,b) ranges over the full N_SWEEP × N_SWEEP cross product (16 pairs,
    // including asymmetric a≠b and either side 0), F64Rig + BoolRig — the
    // shape convention (distinct a,b rather than a single n) is unchanged.
    fn check_braiding_involution<R: Rig + std::fmt::Debug>(a: usize, b: usize) {
        let composed = MatKron::<R>::braiding(a, b)
            .compose(&MatKron::<R>::braiding(b, a))
            .unwrap();
        assert_eq!(
            composed,
            MatKron::<R>::identity(a * b),
            "braiding involution failed for (a,b)=({a},{b})"
        );
    }

    #[test]
    fn braiding_involution() {
        for a in N_SWEEP {
            for b in N_SWEEP {
                check_braiding_involution::<F64Rig>(a, b);
                check_braiding_involution::<BoolRig>(a, b);
            }
        }
    }
}
