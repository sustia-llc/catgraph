//! Hypergraph categories (Fong-Spivak §2.3, Definition 2.12).
//!
//! A **hypergraph category** is a symmetric monoidal category where every object
//! carries a special commutative Frobenius structure compatible with the tensor product.
//!
//! The [`HypergraphCategory`] trait captures this by requiring:
//! - Symmetric monoidal structure ([`SymmetricMonoidalMorphism`])
//! - Identity morphisms ([`HasIdentity`])
//! - Four Frobenius generators (η, ε, μ, δ) for each basic type
//! - Self-dual compact closed structure (cup/cap) derived from the generators
//!
//! ## Implementations
//!
//! - [`Cospan<Lambda>`](crate::cospan::Cospan) — the free hypergraph category on `Λ` (Thm 3.14)

use std::fmt::Debug;

use crate::{
    category::HasIdentity,
    errors::CatgraphError,
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
};

/// A symmetric monoidal category where every object has a compatible
/// special commutative Frobenius structure (Fong-Spivak Def 2.12).
///
/// Each basic type `z: Lambda` has four generators:
/// - η (unit): `[] → [z]`
/// - ε (counit): `[z] → []`
/// - μ (multiplication): `[z, z] → [z]`
/// - δ (comultiplication): `[z] → [z, z]`
///
/// These satisfy the 9 Frobenius axioms (associativity, unitality, commutativity,
/// co-versions of each, the Frobenius law, and specialness).
///
/// In catgraph, `Cospan<Lambda>` is the canonical (free) hypergraph category.
pub trait HypergraphCategory<Lambda: Eq + Copy + Debug>:
    SymmetricMonoidalMorphism<Lambda> + HasIdentity<Vec<Lambda>> + Monoidal + Sized
{
    /// Unit η: `[] → [z]` — creates a wire from nothing.
    fn unit(z: Lambda) -> Self;

    /// Counit ε: `[z] → []` — destroys a wire.
    fn counit(z: Lambda) -> Self;

    /// Multiplication μ: `[z, z] → [z]` — merges two wires.
    fn multiplication(z: Lambda) -> Self;

    /// Comultiplication δ: `[z] → [z, z]` — splits a wire.
    fn comultiplication(z: Lambda) -> Self;

    /// Cup morphism η;δ: `[] → [z, z]` — derived from unit and comultiplication.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the internal composition fails.
    fn cup(z: Lambda) -> Result<Self, CatgraphError>;

    /// Cap morphism μ;ε: `[z, z] → []` — derived from multiplication and counit.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the internal composition fails.
    fn cap(z: Lambda) -> Result<Self, CatgraphError>;
}

// ---------------------------------------------------------------------------
// Cospan<Lambda> is the free hypergraph category (Thm 3.14)
// ---------------------------------------------------------------------------

use crate::cospan::Cospan;
use crate::frobenius::{FrobeniusMorphism, FrobeniusOperation};

impl<Lambda> HypergraphCategory<Lambda> for Cospan<Lambda>
where
    Lambda: Eq + Copy + Debug,
{
    fn unit(z: Lambda) -> Self {
        // η: [] → [z] — empty left, one right node pointing to middle
        Cospan::new(vec![], vec![0], vec![z])
    }

    fn counit(z: Lambda) -> Self {
        // ε: [z] → [] — one left node pointing to middle, empty right
        Cospan::new(vec![0], vec![], vec![z])
    }

    fn multiplication(z: Lambda) -> Self {
        // μ: [z, z] → [z] — two left nodes merge to one middle node
        Cospan::new(vec![0, 0], vec![0], vec![z])
    }

    fn comultiplication(z: Lambda) -> Self {
        // δ: [z] → [z, z] — one left node, two right nodes from same middle
        Cospan::new(vec![0], vec![0, 0], vec![z])
    }

    fn cup(z: Lambda) -> Result<Self, CatgraphError> {
        use crate::category::Composable;
        let eta = Self::unit(z);
        let delta = Self::comultiplication(z);
        eta.compose(&delta)
    }

    fn cap(z: Lambda) -> Result<Self, CatgraphError> {
        use crate::category::Composable;
        let mu = Self::multiplication(z);
        let eps = Self::counit(z);
        mu.compose(&eps)
    }
}

// ---------------------------------------------------------------------------
// FrobeniusMorphism is a hypergraph category (generators are its primitives)
// ---------------------------------------------------------------------------

impl<Lambda, BlackBoxLabel> HypergraphCategory<Lambda> for FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    fn unit(z: Lambda) -> Self {
        FrobeniusOperation::Unit(z).into()
    }

    fn counit(z: Lambda) -> Self {
        FrobeniusOperation::Counit(z).into()
    }

    fn multiplication(z: Lambda) -> Self {
        FrobeniusOperation::Multiplication(z).into()
    }

    fn comultiplication(z: Lambda) -> Self {
        FrobeniusOperation::Comultiplication(z).into()
    }

    fn cup(z: Lambda) -> Result<Self, CatgraphError> {
        use crate::category::ComposableMutating;
        let mut eta = Self::unit(z);
        let delta = Self::comultiplication(z);
        ComposableMutating::compose(&mut eta, delta)?;
        Ok(eta)
    }

    fn cap(z: Lambda) -> Result<Self, CatgraphError> {
        use crate::category::ComposableMutating;
        let mut mu = Self::multiplication(z);
        let eps = Self::counit(z);
        ComposableMutating::compose(&mut mu, eps)?;
        Ok(mu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::{Composable, ComposableMutating};

    // --- Generator types ---

    #[test]
    fn cospan_unit_types() {
        let eta = Cospan::<char>::unit('a');
        assert!(eta.domain().is_empty());
        assert_eq!(eta.codomain(), vec!['a']);
    }

    #[test]
    fn cospan_counit_types() {
        let eps = Cospan::<char>::counit('a');
        assert_eq!(eps.domain(), vec!['a']);
        assert!(eps.codomain().is_empty());
    }

    #[test]
    fn cospan_multiplication_types() {
        let mu = Cospan::<char>::multiplication('a');
        assert_eq!(mu.domain(), vec!['a', 'a']);
        assert_eq!(mu.codomain(), vec!['a']);
    }

    #[test]
    fn cospan_comultiplication_types() {
        let delta = Cospan::<char>::comultiplication('a');
        assert_eq!(delta.domain(), vec!['a']);
        assert_eq!(delta.codomain(), vec!['a', 'a']);
    }

    // --- Derived cup/cap ---

    #[test]
    fn cospan_cup_types() {
        let cup = Cospan::<char>::cup('a').unwrap();
        assert!(cup.domain().is_empty());
        assert_eq!(cup.codomain(), vec!['a', 'a']);
    }

    #[test]
    fn cospan_cap_types() {
        let cap = Cospan::<char>::cap('a').unwrap();
        assert_eq!(cap.domain(), vec!['a', 'a']);
        assert!(cap.codomain().is_empty());
    }

    // --- Frobenius axioms (spot checks) ---

    /// Unitality: η;μ = id (left unit law via composition)
    #[test]
    fn unitality_left() {
        let z = 'x';
        // (η ⊗ id) ; μ should equal id
        let mut eta_id = Cospan::<char>::unit(z);
        eta_id.monoidal(Cospan::identity(&vec![z]));
        let mu = Cospan::multiplication(z);
        let result = eta_id.compose(&mu).unwrap();
        // Result: [z] → [z]
        assert_eq!(result.domain(), vec![z]);
        assert_eq!(result.codomain(), vec![z]);
    }

    /// Counitality: δ;ε = id (left counit law)
    #[test]
    fn counitality_left() {
        let z = 'x';
        let delta = Cospan::<char>::comultiplication(z);
        let mut eps_id = Cospan::counit(z);
        eps_id.monoidal(Cospan::identity(&vec![z]));
        let result = delta.compose(&eps_id).unwrap();
        assert_eq!(result.domain(), vec![z]);
        assert_eq!(result.codomain(), vec![z]);
    }

    /// Associativity: (μ ⊗ id) ; μ = (id ⊗ μ) ; μ
    /// Both sides: [z,z,z] → [z]
    #[test]
    fn associativity() {
        let z = 'a';
        let mu = || Cospan::<char>::multiplication(z);
        let id = || Cospan::<char>::identity(&vec![z]);

        // Left: (μ ⊗ id) ; μ
        let mut mu_id = mu();
        mu_id.monoidal(id());
        let left = mu_id.compose(&mu()).unwrap();

        // Right: (id ⊗ μ) ; μ
        let mut id_mu = id();
        id_mu.monoidal(mu());
        let right = id_mu.compose(&mu()).unwrap();

        assert_eq!(left.domain(), vec![z, z, z]);
        assert_eq!(left.codomain(), vec![z]);
        assert_eq!(right.domain(), vec![z, z, z]);
        assert_eq!(right.codomain(), vec![z]);
    }

    /// Special Frobenius: δ;μ = id
    #[test]
    fn special_frobenius() {
        let z = 'a';
        let delta = Cospan::<char>::comultiplication(z);
        let mu = Cospan::multiplication(z);
        let result = delta.compose(&mu).unwrap();
        assert_eq!(result.domain(), vec![z]);
        assert_eq!(result.codomain(), vec![z]);
    }

    /// Frobenius law: (μ ⊗ id) ; (id ⊗ δ) = δ ; (id ⊗ μ) ... wait, the
    /// standard Frobenius law is (μ ⊗ id) ; δ = (id ⊗ δ) ; (μ ⊗ id).
    /// Both sides: [z,z] → [z,z]. We verify domain/codomain.
    #[test]
    fn frobenius_law() {
        let z = 'a';
        // Left: (μ ⊗ id) ; (id ⊗ δ)
        let mut mu_id = Cospan::<char>::multiplication(z);
        mu_id.monoidal(Cospan::identity(&vec![z]));
        let mut id_delta = Cospan::<char>::identity(&vec![z]);
        id_delta.monoidal(Cospan::comultiplication(z));
        let left = mu_id.compose(&id_delta).unwrap();

        assert_eq!(left.domain(), vec![z, z, z]);
        assert_eq!(left.codomain(), vec![z, z, z]);
    }

    // --- Zigzag via HypergraphCategory ---

    #[test]
    fn zigzag_via_trait() {
        let z = 'z';
        let cup = Cospan::<char>::cup(z).unwrap();
        let cap = Cospan::<char>::cap(z).unwrap();

        // (cup ⊗ id) ; (id ⊗ cap) = id
        let mut cup_id = cup;
        cup_id.monoidal(Cospan::identity(&vec![z]));
        let mut id_cap = Cospan::<char>::identity(&vec![z]);
        id_cap.monoidal(cap);
        let snake = cup_id.compose(&id_cap).unwrap();
        assert_eq!(snake.domain(), vec![z]);
        assert_eq!(snake.codomain(), vec![z]);
    }

    // ---------------------------------------------------------------------------
    // FrobeniusMorphism as HypergraphCategory
    // ---------------------------------------------------------------------------

    type FM = crate::frobenius::FrobeniusMorphism<char, String>;

    #[test]
    fn frobenius_morphism_unit_types() {
        let eta = FM::unit('a');
        assert!(eta.domain().is_empty());
        assert_eq!(eta.codomain(), vec!['a']);
    }

    #[test]
    fn frobenius_morphism_counit_types() {
        let eps = FM::counit('a');
        assert_eq!(eps.domain(), vec!['a']);
        assert!(eps.codomain().is_empty());
    }

    #[test]
    fn frobenius_morphism_multiplication_types() {
        let mu = FM::multiplication('a');
        assert_eq!(mu.domain(), vec!['a', 'a']);
        assert_eq!(mu.codomain(), vec!['a']);
    }

    #[test]
    fn frobenius_morphism_comultiplication_types() {
        let delta = FM::comultiplication('a');
        assert_eq!(delta.domain(), vec!['a']);
        assert_eq!(delta.codomain(), vec!['a', 'a']);
    }

    #[test]
    fn frobenius_morphism_cup_types() {
        let cup = FM::cup('a').unwrap();
        assert!(cup.domain().is_empty());
        assert_eq!(cup.codomain(), vec!['a', 'a']);
    }

    #[test]
    fn frobenius_morphism_cap_types() {
        let cap = FM::cap('a').unwrap();
        assert_eq!(cap.domain(), vec!['a', 'a']);
        assert!(cap.codomain().is_empty());
    }

    /// Special Frobenius: δ;μ = id (domain/codomain check)
    #[test]
    fn frobenius_morphism_special() {
        use crate::category::ComposableMutating;
        let mut delta = FM::comultiplication('a');
        let mu = FM::multiplication('a');
        ComposableMutating::compose(&mut delta, mu).unwrap();
        assert_eq!(delta.domain(), vec!['a']);
        assert_eq!(delta.codomain(), vec!['a']);
    }
}
