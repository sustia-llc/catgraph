//! Self-dual compact closed structure on hypergraph categories (Fong-Spivak §3.1).
//!
//! Every hypergraph category is self-dual compact closed: each object `X` has
//! cup and cap morphisms satisfying the zigzag identities.
//!
//! - **cup** (η;δ): `I → X ⊗ X` — unit followed by comultiplication
//! - **cap** (μ;ε): `X ⊗ X → I` — multiplication followed by counit
//!
//! The zigzag (snake) identities assert:
//! ```text
//! (id_X ⊗ cap_X) ; (cup_X ⊗ id_X) = id_X
//! (cap_X ⊗ id_X) ; (id_X ⊗ cup_X) = id_X
//! ```
//!
//! ## Name bijection (Prop 3.2)
//!
//! The cup/cap morphisms give a bijection `H(X, Y) ≅ H(I, X ⊗ Y)`:
//! - [`name`]: `f: X → Y` ↦ `cup_X ; (id_X ⊗ f) : I → X ⊗ Y`
//! - [`unname`]: `g: I → X ⊗ Y` ↦ `(id_X ⊗ g) ; (cap_X ⊗ id_Y) : X → Y`

use std::fmt::Debug;

use permutations::Permutation;

use crate::{
    category::{ComposableMutating, HasIdentity},
    errors::CatgraphError,
    frobenius::{FrobeniusMorphism, FrobeniusOperation},
    monoidal::{Monoidal, SymmetricMonoidalMorphism},
};

// ---------------------------------------------------------------------------
// Per-type cup/cap (paired ordering)
// ---------------------------------------------------------------------------

/// Construct the cup morphism for a single type: `[] → [z, z]`.
///
/// This is η;δ — the unit creating one wire, then comultiplication splitting it.
///
/// # Panics
///
/// Cannot panic — the internal composition has matching interfaces by construction.
#[must_use]
pub fn cup_single<Lambda, BlackBoxLabel>(z: Lambda) -> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    let unit: FrobeniusMorphism<Lambda, BlackBoxLabel> = FrobeniusOperation::Unit(z).into();
    let comult: FrobeniusMorphism<Lambda, BlackBoxLabel> =
        FrobeniusOperation::Comultiplication(z).into();
    let mut result = unit;
    result
        .compose(comult)
        .expect("unit codomain [z] matches comult domain [z]");
    result
}

/// Construct the cap morphism for a single type: `[z, z] → []`.
///
/// This is μ;ε — multiplication merging two wires, then counit destroying the result.
///
/// # Panics
///
/// Cannot panic — the internal composition has matching interfaces by construction.
#[must_use]
pub fn cap_single<Lambda, BlackBoxLabel>(z: Lambda) -> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    let mult: FrobeniusMorphism<Lambda, BlackBoxLabel> =
        FrobeniusOperation::Multiplication(z).into();
    let counit: FrobeniusMorphism<Lambda, BlackBoxLabel> = FrobeniusOperation::Counit(z).into();
    let mut result = mult;
    result
        .compose(counit)
        .expect("mult codomain [z] matches counit domain [z]");
    result
}

/// Construct the cup morphism for a list of types: `[] → [z₁, z₁, z₂, z₂, …]`.
///
/// The monoidal product of `cup_single(zᵢ)` for each type in the slice.
/// Returns the identity on `[]` (empty morphism) when types is empty.
///
/// # Panics
///
/// Cannot panic — all internal compositions are on matching interfaces.
#[must_use]
pub fn cup<Lambda, BlackBoxLabel>(types: &[Lambda]) -> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    if types.is_empty() {
        return FrobeniusMorphism::identity(&vec![]);
    }
    let mut result: FrobeniusMorphism<Lambda, BlackBoxLabel> = cup_single(types[0]);
    for &z in &types[1..] {
        result.monoidal(cup_single(z));
    }
    result
}

/// Construct the cap morphism for a list of types: `[z₁, z₁, z₂, z₂, …] → []`.
///
/// The monoidal product of `cap_single(zᵢ)` for each type in the slice.
/// Returns the identity on `[]` (empty morphism) when types is empty.
///
/// # Panics
///
/// Cannot panic — all internal compositions are on matching interfaces.
#[must_use]
pub fn cap<Lambda, BlackBoxLabel>(types: &[Lambda]) -> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug,
    BlackBoxLabel: Eq + Clone,
{
    if types.is_empty() {
        return FrobeniusMorphism::identity(&vec![]);
    }
    let mut result: FrobeniusMorphism<Lambda, BlackBoxLabel> = cap_single(types[0]);
    for &z in &types[1..] {
        result.monoidal(cap_single(z));
    }
    result
}

// ---------------------------------------------------------------------------
// Tensor-ordered cup/cap (X⊗X ordering)
// ---------------------------------------------------------------------------

/// Build the deinterleave permutation for `n` types.
///
/// Maps paired ordering `[z₀,z₀,z₁,z₁,…]` to tensor ordering `[z₀,z₁,…,z₀,z₁,…]`.
///
/// The permutation `p` satisfies `p(2k) = k` and `p(2k+1) = n + k`.
fn deinterleave_permutation(n: usize) -> Permutation {
    let mut perm = vec![0usize; 2 * n];
    for k in 0..n {
        perm[2 * k] = k;
        perm[2 * k + 1] = n + k;
    }
    Permutation::try_from(perm).expect("deinterleave is a valid permutation")
}

/// Construct the cup morphism with tensor ordering: `[] → X ⊗ X` where `X = types`.
///
/// Produces `[] → [z₁, z₂, …, zₙ, z₁, z₂, …, zₙ]` by composing the paired cup
/// with a deinterleave permutation.
///
/// For a single type, this is identical to [`cup_single`]. For empty types, returns
/// the identity on `[]`.
///
/// # Panics
///
/// Cannot panic — internal compositions have matching interfaces by construction.
#[must_use]
pub fn cup_tensor<Lambda, BlackBoxLabel>(
    types: &[Lambda],
) -> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    let n = types.len();
    if n <= 1 {
        return cup(types);
    }
    let mut result = cup(types);
    // result: [] → [z₁,z₁,...,zₙ,zₙ] (paired)
    // Apply deinterleave to get [] → [z₁,...,zₙ,z₁,...,zₙ] (tensor)
    let paired_types: Vec<Lambda> = types.iter().flat_map(|&z| [z, z]).collect();
    let perm = deinterleave_permutation(n);
    let shuffle = FrobeniusMorphism::from_permutation(perm, &paired_types, true)
        .expect("deinterleave permutation is valid for paired types");
    result
        .compose(shuffle)
        .expect("paired cup codomain matches shuffle domain");
    result
}

/// Construct the cap morphism with tensor ordering: `X ⊗ X → []` where `X = types`.
///
/// Accepts `[z₁, z₂, …, zₙ, z₁, z₂, …, zₙ]` and produces `[]` by first applying
/// an interleave permutation (inverse of deinterleave), then the paired cap.
///
/// For a single type, this is identical to [`cap_single`]. For empty types, returns
/// the identity on `[]`.
///
/// # Panics
///
/// Cannot panic — internal compositions have matching interfaces by construction.
#[must_use]
pub fn cap_tensor<Lambda, BlackBoxLabel>(
    types: &[Lambda],
) -> FrobeniusMorphism<Lambda, BlackBoxLabel>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    let n = types.len();
    if n <= 1 {
        return cap(types);
    }
    // tensor_types = [z₁,...,zₙ,z₁,...,zₙ]
    let tensor_types: Vec<Lambda> = types.iter().chain(types.iter()).copied().collect();
    // Interleave: inverse of deinterleave, maps tensor → paired
    let perm = deinterleave_permutation(n).inv();
    let unshuffle = FrobeniusMorphism::from_permutation(perm, &tensor_types, true)
        .expect("interleave permutation is valid for tensor types");
    let mut result = unshuffle;
    result
        .compose(cap(types))
        .expect("unshuffle codomain matches paired cap domain");
    result
}

// ---------------------------------------------------------------------------
// Name bijection (Prop 3.2)
// ---------------------------------------------------------------------------

/// Compute the *name* of a morphism: `f: X → Y` ↦ `cup_X ; (id_X ⊗ f) : I → X ⊗ Y`.
///
/// This is the bijection direction `H(X, Y) → H(I, X ⊗ Y)` from Fong-Spivak Prop 3.2.
///
/// # Errors
///
/// Returns `Err` if internal composition fails (should not happen for well-formed morphisms).
pub fn name<Lambda, BlackBoxLabel>(
    f: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    let x = f.domain();
    // cup_X : I → X ⊗ X
    let mut result = cup_tensor(&x);
    // id_X ⊗ f : X ⊗ X → X ⊗ Y
    let mut id_tensor_f = FrobeniusMorphism::identity(&x);
    id_tensor_f.monoidal(f.clone());
    result.compose(id_tensor_f)?;
    Ok(result)
}

/// Recover a morphism from its name: `g: I → X ⊗ Y` ↦ `(id_X ⊗ g) ; (cap_X ⊗ id_Y) : X → Y`.
///
/// This is the inverse direction `H(I, X ⊗ Y) → H(X, Y)` from Fong-Spivak Prop 3.2.
///
/// `x_len` specifies how many leading types in `g.codomain()` belong to `X`.
/// The remaining types form `Y`.
///
/// # Errors
///
/// - `x_len > g.codomain().len()`: split point exceeds codomain length.
/// - `g.domain()` is not empty.
pub fn unname<Lambda, BlackBoxLabel>(
    g: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    x_len: usize,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    if !g.domain().is_empty() {
        return Err(CatgraphError::Composition {
            message: format!(
                "unname requires domain = I (empty), got length {}",
                g.domain().len()
            ),
        });
    }
    let cod = g.codomain();
    if x_len > cod.len() {
        return Err(CatgraphError::Composition {
            message: format!("x_len ({x_len}) exceeds codomain length ({})", cod.len()),
        });
    }
    let x: Vec<Lambda> = cod[..x_len].to_vec();
    let y: Vec<Lambda> = cod[x_len..].to_vec();

    // id_X ⊗ g : X ⊗ I → X ⊗ X ⊗ Y  (since X ⊗ I = X)
    let mut result = FrobeniusMorphism::identity(&x);
    result.monoidal(g.clone());
    // cap_X ⊗ id_Y : X ⊗ X ⊗ Y → Y  (since cap_X: X⊗X → I)
    let mut cap_id = cap_tensor(&x);
    cap_id.monoidal(FrobeniusMorphism::identity(&y));
    result.compose(cap_id)?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Composition via names (Props 3.3-3.4)
// ---------------------------------------------------------------------------

/// Compose two named morphisms: given `f̂: I → X ⊗ Y` and `ĝ: I → Y ⊗ Z`,
/// compute the name of `f;g`, i.e., `(f;g)^: I → X ⊗ Z`.
///
/// Implements Fong-Spivak Prop 3.3: `(f̂ ⊗ ĝ) ; comp^Y_{X,Z} = (f;g)^`.
///
/// This is the **canonical** implementation: it materialises the comp cospan
/// `comp^Y_{X,Z} = id_X ⊗ cap_Y ⊗ id_Z: X ⊗ Y ⊗ Y ⊗ Z → X ⊗ Z` and composes
/// it directly with `f̂ ⊗ ĝ`, exhibiting the paper's formula structurally.
///
/// `x_len` and `y_len` specify the boundary between `X` and `Y` in
/// `f_hat.codomain()` and between `Y` and `Z` in `g_hat.codomain()`.
///
/// # Errors
///
/// - Domain of either name is not empty.
/// - `x_len` exceeds `f_hat.codomain().len()` or `y_len` exceeds `g_hat.codomain().len()`.
/// - The `Y` portions of `f_hat.codomain()` and `g_hat.codomain()` don't match.
pub fn compose_names_direct<Lambda, BlackBoxLabel>(
    f_hat: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    g_hat: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    x_len: usize,
    y_len: usize,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    if !f_hat.domain().is_empty() || !g_hat.domain().is_empty() {
        return Err(CatgraphError::Composition {
            message: "compose_names_direct requires both names to have domain I (empty)"
                .to_string(),
        });
    }
    let f_cod = f_hat.codomain();
    let g_cod = g_hat.codomain();
    if x_len > f_cod.len() {
        return Err(CatgraphError::Composition {
            message: format!(
                "x_len ({x_len}) exceeds f_hat.codomain().len() ({})",
                f_cod.len()
            ),
        });
    }
    if y_len > g_cod.len() {
        return Err(CatgraphError::Composition {
            message: format!(
                "y_len ({y_len}) exceeds g_hat.codomain().len() ({})",
                g_cod.len()
            ),
        });
    }
    let x: Vec<Lambda> = f_cod[..x_len].to_vec();
    let y_from_f: Vec<Lambda> = f_cod[x_len..].to_vec();
    let y_from_g: Vec<Lambda> = g_cod[..y_len].to_vec();
    if y_from_f != y_from_g {
        return Err(CatgraphError::Composition {
            message: format!(
                "Y portions don't match: f_hat.Y = {y_from_f:?} vs g_hat.Y = {y_from_g:?}"
            ),
        });
    }
    let y = y_from_f;
    let z: Vec<Lambda> = g_cod[y_len..].to_vec();

    // f̂ ⊗ ĝ: I → X ⊗ Y ⊗ Y ⊗ Z
    let mut result = f_hat.clone();
    result.monoidal(g_hat.clone());

    // comp^Y_{X,Z} = id_X ⊗ cap_Y ⊗ id_Z: X ⊗ Y ⊗ Y ⊗ Z → X ⊗ Z
    let mut comp: FrobeniusMorphism<Lambda, BlackBoxLabel> = FrobeniusMorphism::identity(&x);
    comp.monoidal(cap_tensor(&y));
    comp.monoidal(FrobeniusMorphism::identity(&z));

    result.compose(comp)?;
    Ok(result)
}

/// Reference implementation of `compose_names` that goes through `unname`
/// and `name`: `name(unname(f̂); unname(ĝ))`.
///
/// Mathematically equivalent to [`compose_names_direct`] — the latter
/// exhibits the paper's Prop 3.3 formula structurally, while this version
/// uses the name bijection as a black box. Kept as a reference for
/// equivalence testing (see `tests/compact_closed.rs`).
///
/// # Errors
///
/// - Domain of either name is not empty.
/// - `x_len` exceeds `f_hat.codomain().len()` or `y_len` exceeds `g_hat.codomain().len()`.
pub fn compose_names_via_unname<Lambda, BlackBoxLabel>(
    f_hat: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    g_hat: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    x_len: usize,
    y_len: usize,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    if !f_hat.domain().is_empty() || !g_hat.domain().is_empty() {
        return Err(CatgraphError::Composition {
            message: "compose_names_via_unname requires both names to have domain I (empty)"
                .to_string(),
        });
    }
    let f = unname(f_hat, x_len)?;
    let g = unname(g_hat, y_len)?;
    let mut fg = f;
    fg.compose(g)?;
    name(&fg)
}

/// Compose two named morphisms via the canonical direct formula.
///
/// Alias for [`compose_names_direct`], matching Fong-Spivak Prop 3.3 literally.
///
/// # Errors
///
/// See [`compose_names_direct`].
pub fn compose_names<Lambda, BlackBoxLabel>(
    f_hat: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    g_hat: &FrobeniusMorphism<Lambda, BlackBoxLabel>,
    x_len: usize,
    y_len: usize,
) -> Result<FrobeniusMorphism<Lambda, BlackBoxLabel>, CatgraphError>
where
    Lambda: Eq + Copy + Debug + Send + Sync,
    BlackBoxLabel: Eq + Clone + Send + Sync,
{
    compose_names_direct(f_hat, g_hat, x_len, y_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::ComposableMutating;

    type FM = FrobeniusMorphism<char, String>;

    // --- Per-type cup/cap (existing tests) ---

    #[test]
    fn cup_single_domain_codomain() {
        let c: FM = cup_single('a');
        assert!(c.domain().is_empty(), "cup domain should be empty (I)");
        assert_eq!(
            c.codomain(),
            vec!['a', 'a'],
            "cup codomain should be [a, a]"
        );
    }

    #[test]
    fn cap_single_domain_codomain() {
        let c: FM = cap_single('a');
        assert_eq!(c.domain(), vec!['a', 'a'], "cap domain should be [a, a]");
        assert!(c.codomain().is_empty(), "cap codomain should be empty (I)");
    }

    #[test]
    fn cup_multi_type_domain_codomain() {
        let c: FM = cup(&['a', 'b']);
        assert!(c.domain().is_empty());
        assert_eq!(c.codomain(), vec!['a', 'a', 'b', 'b']);
    }

    #[test]
    fn cap_multi_type_domain_codomain() {
        let c: FM = cap(&['a', 'b']);
        assert_eq!(c.domain(), vec!['a', 'a', 'b', 'b']);
        assert!(c.codomain().is_empty());
    }

    #[test]
    fn cup_empty_is_identity() {
        let c: FM = cup(&[]);
        assert!(c.domain().is_empty());
        assert!(c.codomain().is_empty());
    }

    #[test]
    fn cap_empty_is_identity() {
        let c: FM = cap(&[]);
        assert!(c.domain().is_empty());
        assert!(c.codomain().is_empty());
    }

    // --- Zigzag identities ---

    #[test]
    fn zigzag_right_snake_single_type() {
        let z = 'x';
        let mut left_half: FM = FrobeniusMorphism::identity(&vec![z]);
        left_half.monoidal(cap_single(z));
        let mut right_half: FM = cup_single(z);
        right_half.monoidal(FrobeniusMorphism::identity(&vec![z]));
        let mut snake = right_half;
        snake.compose(left_half).expect("interfaces match");
        assert_eq!(snake.domain(), vec![z]);
        assert_eq!(snake.codomain(), vec![z]);
    }

    #[test]
    fn zigzag_left_snake_single_type() {
        let z = 'x';
        let mut left_half: FM = cap_single(z);
        left_half.monoidal(FrobeniusMorphism::identity(&vec![z]));
        let mut right_half: FM = FrobeniusMorphism::identity(&vec![z]);
        right_half.monoidal(cup_single(z));
        let mut snake = right_half;
        snake.compose(left_half).expect("interfaces match");
        assert_eq!(snake.domain(), vec![z]);
        assert_eq!(snake.codomain(), vec![z]);
    }

    #[test]
    fn zigzag_right_snake_multi_type() {
        for z in ['a', 'b'] {
            let mut right_half: FM = cup_single(z);
            right_half.monoidal(FrobeniusMorphism::identity(&vec![z]));
            let mut left_half: FM = FrobeniusMorphism::identity(&vec![z]);
            left_half.monoidal(cap_single(z));
            let mut snake = right_half;
            snake.compose(left_half).expect("interfaces match");
            assert_eq!(snake.domain(), vec![z]);
            assert_eq!(snake.codomain(), vec![z]);
        }
    }

    #[test]
    #[allow(clippy::similar_names)] // cap/cup naming is intrinsic to compact-closed structure
    fn cap_cup_composition_types() {
        let z = 'q';
        let cap_z: FM = cap_single(z);
        let cup_z: FM = cup_single(z);
        let mut composed = cap_z;
        composed.compose(cup_z).expect("[] interface matches");
        assert_eq!(composed.domain(), vec![z, z]);
        assert_eq!(composed.codomain(), vec![z, z]);
    }

    // --- Tensor-ordered cup/cap ---

    #[test]
    fn cup_tensor_single_type() {
        let c: FM = cup_tensor(&['a']);
        assert!(c.domain().is_empty());
        assert_eq!(c.codomain(), vec!['a', 'a']);
    }

    #[test]
    fn cup_tensor_multi_type() {
        let c: FM = cup_tensor(&['a', 'b']);
        assert!(c.domain().is_empty());
        assert_eq!(c.codomain(), vec!['a', 'b', 'a', 'b']);
    }

    #[test]
    fn cup_tensor_three_types() {
        let c: FM = cup_tensor(&['x', 'y', 'z']);
        assert!(c.domain().is_empty());
        assert_eq!(c.codomain(), vec!['x', 'y', 'z', 'x', 'y', 'z']);
    }

    #[test]
    fn cap_tensor_multi_type() {
        let c: FM = cap_tensor(&['a', 'b']);
        assert_eq!(c.domain(), vec!['a', 'b', 'a', 'b']);
        assert!(c.codomain().is_empty());
    }

    #[test]
    fn cup_tensor_cap_tensor_roundtrip() {
        // cup_tensor ; cap_tensor should compose through X⊗X
        let types = &['a', 'b'];
        let mut dim: FM = cup_tensor(types);
        dim.compose(cap_tensor(types)).expect("X⊗X interface");
        assert!(dim.domain().is_empty());
        assert!(dim.codomain().is_empty());
    }

    // --- Name bijection ---

    #[test]
    fn name_identity_single_type() {
        let id: FM = FrobeniusMorphism::identity(&vec!['a']);
        let named = name(&id).unwrap();
        assert!(named.domain().is_empty());
        assert_eq!(named.codomain(), vec!['a', 'a']);
    }

    #[test]
    fn name_identity_multi_type() {
        let id: FM = FrobeniusMorphism::identity(&vec!['a', 'b']);
        let named = name(&id).unwrap();
        assert!(named.domain().is_empty());
        assert_eq!(named.codomain(), vec!['a', 'b', 'a', 'b']);
    }

    #[test]
    fn name_unit_morphism() {
        // η: [] → [z] has name cup_[] ; (id_[] ⊗ η) = η : [] → [z]
        let unit: FM = FrobeniusOperation::Unit('a').into();
        let named = name(&unit).unwrap();
        assert!(named.domain().is_empty());
        assert_eq!(named.codomain(), vec!['a']);
    }

    #[test]
    fn unname_roundtrip_single() {
        let z = 'x';
        let id: FM = FrobeniusMorphism::identity(&vec![z]);
        let named = name(&id).unwrap();
        let recovered = unname(&named, 1).unwrap();
        assert_eq!(recovered.domain(), vec![z]);
        assert_eq!(recovered.codomain(), vec![z]);
    }

    #[test]
    fn unname_roundtrip_multi_type() {
        let types = vec!['a', 'b'];
        let id: FM = FrobeniusMorphism::identity(&types);
        let named = name(&id).unwrap();
        let recovered = unname(&named, 2).unwrap();
        assert_eq!(recovered.domain(), types);
        assert_eq!(recovered.codomain(), types);
    }

    #[test]
    fn unname_error_nonempty_domain() {
        let f: FM = FrobeniusMorphism::identity(&vec!['a']);
        assert!(unname(&f, 1).is_err());
    }

    #[test]
    fn unname_error_x_len_too_large() {
        let g: FM = cup_single('a');
        assert!(unname(&g, 5).is_err());
    }

    // --- Compose names ---

    #[test]
    fn compose_names_identity_identity() {
        let id: FM = FrobeniusMorphism::identity(&vec!['a']);
        let f_hat = name(&id).unwrap();
        let g_hat = name(&id).unwrap();
        let result = compose_names(&f_hat, &g_hat, 1, 1).unwrap();
        assert!(result.domain().is_empty());
        assert_eq!(result.codomain(), vec!['a', 'a']);
    }
}
