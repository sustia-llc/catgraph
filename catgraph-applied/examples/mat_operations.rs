//! Matrix prop `Mat(R)` — six demonstrations covering `MatR<R>` core algebra,
//! the in-place mutable API, the `mat_f64` nalgebra bridge (feature
//! `f64-rig`), the `graphical_linalg` faithfulness verifier closing FS18
//! Thm 5.60, and the `Z(BigInt)` integer-exact ring.
//!
//! # Paper anchors
//!
//! - **F&S 2018 Seven Sketches** (`arXiv:1803.05316`) — §5.3 Def 5.50 (`Mat(R)`
//!   as the prop of `m × n` matrices over a rig R; row-major / row-vector
//!   convention, rows = domain arity); Def 5.45 (signal flow graphs over R);
//!   Thm 5.53 (functor `S: SFG_R → Mat(R)`); Thm 5.60 (16-equation
//!   presentation of `Mat(R)` is full + faithful, after Baez-Erbele 2015).
//! - **Storjohann 2000** *Algorithms for matrix canonical forms* (`PhD` thesis,
//!   ETH Zürich) — §7 (Smith Normal Form over a PID via row + column
//!   operations: `row_swap` / `scale_row` / `add_scaled_row` and the
//!   column-axis duals). The mutable API on [`MatR`] exists precisely
//!   to give the cg-magnitude SNF stack (BV25 Prop 3.14 + Leinster 2008
//!   Cor 1.5) an in-place row-reduction substrate that does not allocate a
//!   new matrix per elementary operation.
//! - **Bourbaki *Algèbre*** Ch. I §8 — `ℤ` as the initial object of the
//!   category of unital rings. The `from_i64` constructor on `Z` exhibits
//!   this canonical homomorphism `ℤ → R`, exercising the `ZAlgebra` trait at
//!   the §6 demo below.
//!
//! # Narrative arc
//!
//! 1. Construction triad: `MatR<F64Rig>::new` / `identity` / `zero_matrix`,
//!    plus the identity-axiom check `I · M = M`.
//! 2. Multiplicative structure: `matmul` on a non-trivial 3×3 fixture,
//!    `block_diagonal` for monoidal tensor, `permutation_matrix` from a
//!    `permutations::Permutation`.
//! 3. Mutable API: Gauss-elimination-style sequence — `row_swap`,
//!    `scale_row`, `add_scaled_row`, plus column-axis duals + `entry_mut` +
//!    `entries_mut`. This is the exact API consumed by cg-magnitude's SNF
//!    benches (Storjohann §7 row + column operations, no per-op allocation).
//! 4. nalgebra bridge (`#[cfg(feature = "f64-rig")]`): `mat_to_nalgebra` →
//!    `determinant` → `try_inverse` → `mat_from_nalgebra` round-trip on a
//!    non-singular 3×3 + a singular contrast case where `try_inverse`
//!    correctly returns `None`. Under `not(feature = "f64-rig")`, prints a
//!    skip-message documenting the feature flag.
//! 5. `graphical_linalg` — bounded CC-incompleteness witness count +
//!    §5.4 Functorial-resolution diptych (Thm 5.60 closure via
//!    `MatrixNFFunctor`). The verifier
//!    [`verify_sfg_to_mat_is_full_and_faithful`] surfaces witnesses of the
//!    default CC engine's *syntactic* incompleteness against the matrix
//!    functor's ground truth — NOT Thm 5.60 faithfulness violations
//!    (Baez-Erbele 2015 already proves the theorem). We then resolve a
//!    chosen witness through `eq_mod_functorial(_, _, &MatrixNFFunctor)`,
//!    the complete-by-theorem decision procedure.
//! 6. `Z(BigInt)` arithmetic: construct via `Z::new(BigInt::from(...))` and
//!    `Z::from_i64`, exercise `+`, `*`, `-` showing arbitrary-precision
//!    behaviour past `i64::MAX` (a 64×64 factorial-style product), and prove
//!    `Z` satisfies the `ZAlgebra` trait via a compile-time witness function.
//!
//! Run: `cargo run -p catgraph-applied --example mat_operations --release`
//! With nalgebra bridge: `… --release --features f64-rig`

#![allow(clippy::too_many_lines)] // 6 narrative sections in one main; idiomatic for examples.
#![allow(clippy::float_cmp)] // f64 ops here are exact-by-construction on small integers.

use catgraph_applied::{
    graphical_linalg::{matr_presentation, verify_sfg_to_mat_is_full_and_faithful},
    integer::ZAlgebra,
    mat::MatR,
    prop::presentation::functorial::MatrixNFFunctor,
    rig::{BoolRig, F64Rig},
    z::Z,
};
use num::BigInt;
use permutations::Permutation;

fn main() {
    println!("=== Mat(R) — matrix prop algebra + mat_f64 bridge + Z(BigInt) ===\n");

    // -------- §1. Construction triad ------------------------------------
    //
    // Three canonical entry points to `MatR<F64Rig>`:
    //   - `MatR::new(rows, cols, entries)`: validated literal construction.
    //   - `MatR::identity(n)`: the n × n multiplicative identity.
    //   - `MatR::zero_matrix(r, c)`: the additive identity of shape r × c.
    // Identity axiom `I · M = M` verified pointwise below.
    println!("§1. Construction triad — `new` / `identity` / `zero_matrix`");

    let m = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0), F64Rig(3.0)],
            vec![F64Rig(4.0), F64Rig(5.0), F64Rig(6.0)],
            vec![F64Rig(7.0), F64Rig(8.0), F64Rig(10.0)], // [7,8,10] makes det nonzero.
        ],
    )
    .expect("3×3 with 9 entries");

    let id3 = MatR::<F64Rig>::identity(3);
    let zero23 = MatR::<F64Rig>::zero_matrix(2, 3);

    println!(
        "  m            : {}×{}  (entries[0] = {:?})",
        m.rows(),
        m.cols(),
        m.entries()[0]
    );
    println!("  I_3          : {}×{}", id3.rows(), id3.cols());
    println!(
        "  0_{{2×3}}        : {}×{}  (entries[0] = {:?})",
        zero23.rows(),
        zero23.cols(),
        zero23.entries()[0]
    );

    // Identity axiom: I · M = M.
    let id_m = id3.matmul(&m).expect("I_3 · m: shape-compatible");
    assert_eq!(id_m, m, "MatR::identity is left-identity for matmul");
    println!("  I_3 · m == m : verified pointwise.\n");

    // -------- §2. Multiplicative structure ------------------------------
    //
    // - `matmul`: row-major triple loop per F&S Def 5.50.
    // - `block_diagonal`: the monoidal tensor (block-diag sum) of `Mat(R)`
    //   per the prop's symmetric monoidal structure (Def 5.45 + §5.3).
    // - `permutation_matrix`: lifts a `permutations::Permutation` into
    //   `MatR<R>` for any rig R via `entries[i][p(i)] = R::one()`. This is
    //   the `SymmetricMonoidalMorphism::from_permutation` realisation.
    println!("§2. Multiplicative structure — `matmul`, `block_diagonal`, `permutation_matrix`");

    // 3×3 × 3×2 → 3×2 to exercise non-square matmul.
    let a = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(1.0), F64Rig(0.0), F64Rig(2.0)],
            vec![F64Rig(0.0), F64Rig(3.0), F64Rig(0.0)],
            vec![F64Rig(4.0), F64Rig(0.0), F64Rig(5.0)],
        ],
    )
    .expect("3×3");
    let b = MatR::<F64Rig>::new(
        3,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(0.0)],
            vec![F64Rig(0.0), F64Rig(1.0)],
            vec![F64Rig(1.0), F64Rig(1.0)],
        ],
    )
    .expect("3×2");
    let ab = a.matmul(&b).expect("3×3 · 3×2 → 3×2");
    println!("  matmul(3×3, 3×2) : shape = {}×{}", ab.rows(), ab.cols());
    println!("  (a · b)[0]      = {:?}", ab.entries()[0]);
    println!("  (a · b)[2]      = {:?}", ab.entries()[2]);
    // (a·b)[0][0] = 1·1 + 0·0 + 2·1 = 3.0; (a·b)[2][1] = 4·0 + 0·1 + 5·1 = 5.0.
    assert_eq!(ab.entries()[0][0], F64Rig(3.0));
    assert_eq!(ab.entries()[2][1], F64Rig(5.0));

    // block_diagonal: build a 5×5 block-diag = diag(2×2, 3×3) — the monoidal
    // tensor of two matrices on object-side `2 ⊗ 3 = 5`.
    let small = MatR::<F64Rig>::new(
        2,
        2,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(3.0), F64Rig(4.0)],
        ],
    )
    .expect("2×2");
    let blk = small.block_diagonal(&a);
    println!(
        "  block_diag(2×2, 3×3) : {}×{}  (top-left block = small, bottom-right = a)",
        blk.rows(),
        blk.cols()
    );
    assert_eq!(blk.rows(), 5);
    assert_eq!(blk.cols(), 5);
    // Off-block zeros: blk[0][2] = 0 (top-row of small, into a-side cols).
    assert_eq!(blk.entries()[0][2], F64Rig(0.0));
    assert_eq!(blk.entries()[2][0], F64Rig(0.0));
    // Block contents preserved.
    assert_eq!(blk.entries()[0][1], F64Rig(2.0)); // small[0][1] = 2.0
    assert_eq!(blk.entries()[2][2], F64Rig(1.0)); // a[0][0] = 1.0 (top-left of bottom-right block)

    // permutation_matrix: swap rows 0 and 1 in a 3-vector — exercises the
    // `entries[i][p(i)] = R::one()` recipe.
    let swap_01 = Permutation::transposition(3, 0, 1);
    let p_mat = MatR::<F64Rig>::permutation_matrix(&swap_01);
    println!("  perm(swap(0,1)) entries:");
    for (i, row) in p_mat.entries().iter().enumerate() {
        println!("    row {i} = {row:?}");
    }
    // P · I should swap rows 0 and 1 of the identity.
    let pi = p_mat.matmul(&id3).expect("3×3 matmul");
    // pi[0][1] = 1.0 because (P·I)[0][1] = sum_k P[0][k]·I[k][1] = P[0][1]·I[1][1] = 1·1.
    // The permutation P swaps rows 0↔1 of any matrix it multiplies on the left.
    assert_eq!(pi.entries()[0][1], F64Rig(1.0));
    assert_eq!(pi.entries()[1][0], F64Rig(1.0));
    println!("  P · I_3 swaps rows 0 ↔ 1 of identity.\n");

    // -------- §3. Mutable API — Gauss-elimination sequence ----------
    //
    // The Storjohann SNF stack in cg-magnitude operates row-by-row on a
    // working matrix. Allocating a fresh `MatR` per elementary operation is
    // prohibitive; the mutable API gives the SNF benches an in-place
    // row + column operation surface that does not re-allocate. We exercise
    // all 8 mutating methods on a 3×3 fixture, printing state after each step.
    println!("§3. Mutable API — in-place row + column operations (Storjohann §7)");

    let mut g = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(0.0), F64Rig(1.0), F64Rig(2.0)],
            vec![F64Rig(1.0), F64Rig(0.0), F64Rig(3.0)],
            vec![F64Rig(2.0), F64Rig(4.0), F64Rig(8.0)],
        ],
    )
    .expect("3×3");

    println!("  initial g:");
    print_mat(&g);

    // Step 1: swap rows 0 ↔ 1 so g[0][0] = 1 (a "good" pivot).
    g.row_swap(0, 1).expect("rows 0,1 in range");
    println!("  after row_swap(0, 1):");
    print_mat(&g);
    assert_eq!(g.entries()[0][0], F64Rig(1.0));

    // Step 2: scale_row(0, 2) — double row 0.
    g.scale_row(0, &F64Rig(2.0)).expect("row 0 in range");
    println!("  after scale_row(0, 2):");
    print_mat(&g);
    assert_eq!(g.entries()[0][2], F64Rig(6.0)); // was 3.0 → 6.0

    // Step 3: add_scaled_row(2, 0, -1) — row 2 ← row 2 + (-1) · row 0.
    g.add_scaled_row(2, 0, &F64Rig(-1.0))
        .expect("rows 0,2 distinct, in range");
    println!("  after add_scaled_row(2, 0, -1):");
    print_mat(&g);
    // g[2] was [2,4,8] minus 1·[2,0,6] = [0,4,2].
    assert_eq!(g.entries()[2], vec![F64Rig(0.0), F64Rig(4.0), F64Rig(2.0)]);

    // Step 4: column-side duals — col_swap, scale_col, add_scaled_col.
    g.col_swap(0, 2).expect("cols 0,2 in range");
    println!("  after col_swap(0, 2):");
    print_mat(&g);

    // scale_col(1, F64Rig(0.5)): field-only operation. SNF over Z(BigInt) is restricted
    // to ±1 row/col scaling (PID-only); the 0.5 here is to demonstrate the F64Rig field path.
    g.scale_col(1, &F64Rig(0.5)).expect("col 1 in range");
    println!("  after scale_col(1, 0.5):");
    print_mat(&g);

    g.add_scaled_col(0, 2, &F64Rig(3.0))
        .expect("cols 0,2 distinct, in range");
    println!("  after add_scaled_col(0, 2, 3):");
    print_mat(&g);

    // Step 5: entry_mut + entries_mut — direct cell access.
    if let Some(cell) = g.entry_mut(0, 0) {
        *cell = F64Rig(42.0);
    }
    assert_eq!(g.entries()[0][0], F64Rig(42.0));
    println!(
        "  entry_mut(0, 0) ← 42.0  : g[0][0] = {:?}",
        g.entries()[0][0]
    );

    // Bulk mutation via entries_mut(): negate every cell of row 0.
    {
        let rows = g.entries_mut();
        for cell in &mut rows[0] {
            *cell = F64Rig(-cell.0);
        }
    }
    println!("  entries_mut() negated row 0: g[0] = {:?}", g.entries()[0]);
    assert_eq!(g.entries()[0][0], F64Rig(-42.0));
    println!();

    // -------- §4. nalgebra bridge (feature-gated) ------------------------
    println!("§4. nalgebra bridge — `mat_to_nalgebra` / `determinant` / `try_inverse`");
    run_mat_f64_demo();
    println!();

    // -------- §5. graphical_linalg — bounded CC-incompleteness witness + the §5.4 Functorial resolution ---
    //
    // F&S Thm 5.60 (Baez-Erbele 2015) proves `Free(Σ_SFG)/⟨E_{17}⟩ ≅ Mat(R)`
    // abstractly — we do NOT empirically re-verify the theorem. What the
    // bounded-enumeration verifier `verify_sfg_to_mat_is_full_and_faithful`
    // actually surfaces is the *incompleteness* of the syntactic
    // `NormalizeEngine::CongruenceClosure` default engine against the
    // semantic ground truth carried by the matrix functor `S = sfg_to_mat`.
    // Plain CC (with or without `smc_refine`) cannot synthesize fresh
    // composite intermediates, so it leaves witness pairs that are
    // presentation-equivalent under E_{17} but appear CC-distinct.
    //
    // The Functorial engine closes §5.4 semantically:
    // `Presentation::eq_mod_functorial(&a, &b, &MatrixNFFunctor::new())`
    // is complete-by-theorem for Mat(R) — it just checks `S(a) == S(b)`.
    // We demonstrate both engines side-by-side here.
    println!("§5. graphical_linalg — bounded CC tracking + §5.4 Functorial resolution");

    // Build the F&S Thm 5.60 presentation parameterised by BoolRig samples.
    // Two samples (false, true) suffice for the D-group scalar equations
    // since BoolRig has only those two elements.
    let bool_samples = [BoolRig(false), BoolRig(true)];
    let presentation =
        matr_presentation::<BoolRig>(&bool_samples).expect("matr_presentation builds for BoolRig");
    println!(
        "  matr_presentation::<BoolRig> built with {} equations",
        presentation.equations().len()
    );

    // size_bound = 2 keeps the bounded SFG enumeration tractable for BoolRig
    // while still surfacing the CC-incompleteness witnesses the atom-canonical
    // `smc_refine` partially closes: BoolRig d=2 collisions
    // 2574 → 1433 (smc_refine) → 1301 (post-#14 NF), ~49% total.
    let report = verify_sfg_to_mat_is_full_and_faithful::<BoolRig>(2, &bool_samples)
        .expect("verifier runs on BoolRig size_bound=2");
    println!("  FaithfulnessReport (BoolRig, size_bound=2):");
    println!("    size_bound          = {}", report.size_bound);
    println!("    expressions_checked = {}", report.expressions_checked);
    println!(
        "    collisions_under_S  = {}  (CC-incompleteness witnesses, NOT Thm 5.60 violations)",
        report.collisions_under_s
    );
    println!("    witnesses.len()     = {}", report.witnesses.len());
    // The report's `collisions_under_s` counts CC-incompleteness witnesses,
    // not Thm 5.60 faithfulness violations. The matrix functor S is faithful
    // by Baez-Erbele 2015; what's missing from plain CC is syntactic
    // completeness. The `atom_canonical` pass in `kb.rs` gives ~44%
    // reduction; the residual gap is closed by the terminal Functorial engine
    // demonstrated below (issue #15 resolved functorial-terminal; syntactic
    // Knuth-Bendix completion is the #57 feasibility spike).
    assert!(
        report.expressions_checked > 0,
        "verifier must enumerate at least one expression"
    );
    println!(
        "  Note: nonzero `collisions_under_S` is *expected* under the default CC engine.\n  \
        See `tests/graphical_linalg.rs` rustdoc for the CC-completeness rationale."
    );

    // Functorial resolution: pick any CC-incompleteness witness from the
    // report and verify the Functorial engine resolves it correctly to
    // `Some(true)`. This is the §5.4 Thm 5.60 semantic closure path —
    // `eq_mod_functorial` with `MatrixNFFunctor<R>` is
    // complete by theorem (it just checks `S(a) == S(b)` in `Mat(R)`).
    if let Some((a, b)) = report.witnesses.first() {
        let cc_verdict = presentation
            .eq_mod(a.as_prop_expr(), b.as_prop_expr())
            .expect("CC engine runs");
        let nf_functor = MatrixNFFunctor::<BoolRig>::new();
        let func_verdict = presentation
            .eq_mod_functorial(a.as_prop_expr(), b.as_prop_expr(), &nf_functor)
            .expect("Functorial engine runs");
        println!("  Picking witness pair from report:");
        println!("    eq_mod (default CC engine)     = {cc_verdict:?}");
        println!(
            "    eq_mod_functorial (Functorial) = {func_verdict:?}  (always Some(true) on a CC witness — §5.4 closure)"
        );
        // The witness was binned by matrix image, so S(a) == S(b), so the
        // Functorial verdict MUST be Some(true). This is what closes Thm 5.60
        // semantically.
        assert_eq!(
            func_verdict,
            Some(true),
            "Functorial engine is complete for Mat(R) per Baez-Erbele 2015"
        );
    }
    println!("  §5.4 Thm 5.60 semantic closure verified via the Functorial engine.\n");

    // -------- §6. Z(BigInt) — integer-exact ring + ZAlgebra witness --------
    //
    // `Z(BigInt)` is the substrate for cg-magnitude §1.17 Leinster 2008
    // Cor 1.5 integer-exact Möbius inversion. Bourbaki *Algèbre* Ch. I §8
    // identifies ℤ as the initial object of the category of unital rings;
    // `ZAlgebra::from_i64` exhibits the canonical homomorphism `ℤ → R`.
    println!("§6. Z(BigInt) — arbitrary-precision integer ring + ZAlgebra witness");

    let a_z = Z::new(BigInt::from(42));
    let b_z = Z::from_i64(13);
    let sum = a_z.clone() + b_z.clone();
    let prod = a_z.clone() * b_z.clone();
    let diff = a_z.clone() - b_z.clone();
    println!("  a = Z::new(BigInt::from(42)) = {a_z:?}");
    println!("  b = Z::from_i64(13)          = {b_z:?}");
    println!("  a + b = {sum:?}");
    println!("  a * b = {prod:?}");
    println!("  a - b = {diff:?}");
    assert_eq!(sum, Z::from_i64(55));
    assert_eq!(prod, Z::from_i64(546));
    assert_eq!(diff, Z::from_i64(29));

    // Demonstrate arbitrary-precision: 25! overflows i64 (max ≈ 9.2e18; 25!
    // ≈ 1.55e25). BigInt handles it exactly.
    let mut fact = Z::from_i64(1);
    for k in 1_i64..=25 {
        fact = fact * Z::from_i64(k);
    }
    println!("  25! (i64 would overflow at 21!): {fact:?}");
    // Check that 25! > i64::MAX symbolically. `Z` derives `Ord` (z.rs:59), so we
    // compare `Z` values directly rather than reaching into the inner `BigInt`.
    assert!(fact > Z::new(BigInt::from(i64::MAX)));
    println!("  25! > i64::MAX confirmed; BigInt arithmetic is genuinely unbounded.");

    // Compile-time witness: `Z` satisfies `ZAlgebra` (sealed trait). If the
    // crate-private `Sealed` impl on `Z` were removed, this call site would
    // fail to compile with `the trait bound Z: Sealed is not satisfied` as
    // the proximate compile failure — the seal documented in `src/z.rs:138`.
    require_zalgebra(&fact);
    println!("  `Z` satisfies `ZAlgebra` (sealed trait — compile-time witness).");

    println!("\nAll demonstrations green.");
}

/// Compile-time witness function — only types implementing the sealed
/// [`ZAlgebra`] trait may be passed in. Mirrors the upstream
/// `causality:numeric-algebra` `require_<Trait>()` pattern used to expose
/// trait membership as a compile-time fact at the call site.
fn require_zalgebra<T: ZAlgebra>(_value: &T) {}

/// Pretty-printer for `MatR<F64Rig>` — used by §3 to surface step-by-step
/// row + column operations.
fn print_mat(m: &MatR<F64Rig>) {
    for row in m.entries() {
        let cells: Vec<String> = row.iter().map(|c| format!("{:>5.1}", c.0)).collect();
        println!("    [{}]", cells.join(", "));
    }
}

/// `mat_f64` bridge demonstration — split into a separate function so the
/// `#[cfg(feature = "f64-rig")]` and `#[cfg(not(...))]` arms stay tight.
///
/// **Math:nalgebra-applied review surface.** This subsection is the primary
/// nalgebra-bridge consumer in the example file. Decisions made:
///
/// - Round-trip via `mat_to_nalgebra` → `determinant` → `try_inverse` →
///   `mat_from_nalgebra`. Determinant uses nalgebra's LU decomposition under
///   the hood (`DMatrix::determinant()` dispatches through `LU`); inverse
///   uses the same. For dense `f64` matrices the LU path is the standard
///   non-LAPACK route — LAPACK crossover is empirically ~64×64 (see the
///   `nalgebra-lapack` skill's xgesdd/xgetrf crossover knowledge), well
///   above the 3×3 fixture here.
/// - Non-singular fixture: `[[1,2,0], [0,3,4], [5,0,6]]` has determinant
///   `1·(3·6 − 0·4) − 2·(0·6 − 4·5) + 0 = 18 + 40 = 58`. The hand-computed
///   value gives the reader a non-trivial determinant target to verify.
/// - Singular contrast: a matrix with two identical rows has determinant 0
///   and `try_inverse` MUST return `None`. The `Option` return type is the
///   nalgebra-bridge correctness gate — `None` on singular, `Some` on
///   invertible.
/// - Round-trip: `inv · m_original = I_3` is the inverse-axiom check, verified
///   to tolerance `1e-12` per IEEE-754 LU accumulation. The 3×3 integer-valued
///   fixture's exact rational inverse (denominator 58, max entry ≈ 0.31) yields
///   worst-case f64 round-trip error on the order of `~n²·u ≈ 1e-15`; `1e-12`
///   stays 3 orders above that realistic worst case while tightening the
///   correctness gate enough to catch a hypothetical row/col layout bug.
#[cfg(feature = "f64-rig")]
fn run_mat_f64_demo() {
    use catgraph_applied::mat_f64::{determinant, mat_from_nalgebra, mat_to_nalgebra, try_inverse};

    // Non-singular 3×3 with a hand-verifiable determinant.
    let m = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0), F64Rig(0.0)],
            vec![F64Rig(0.0), F64Rig(3.0), F64Rig(4.0)],
            vec![F64Rig(5.0), F64Rig(0.0), F64Rig(6.0)],
        ],
    )
    .expect("3×3");

    // mat_to_nalgebra: hand off to nalgebra's DMatrix for field-specific ops.
    let dm = mat_to_nalgebra(&m);
    println!("  m → DMatrix<f64>     : {}×{}", dm.nrows(), dm.ncols());
    println!(
        "    dm[(0, 1)] = {}    (should equal m.entries()[0][1] = 2.0)",
        dm[(0, 1)]
    );

    // determinant: hand-computed 1·(3·6 − 0·4) − 2·(0·6 − 4·5) + 0 = 18 + 40 = 58.
    let det = determinant(&m).expect("3×3 is square");
    println!("  determinant(m)       = {det}");
    assert!((det - 58.0).abs() < 1e-9, "expected det = 58.0");

    // try_inverse: returns Some(inv) for non-singular m. Verify inv · m = I_3
    // to a tight tolerance. The 3×3 integer-valued fixture has exact rational
    // inverse with denominator 58 and max entry ≈ 0.31; LU round-trip error is
    // on the order of ~n²·u ≈ 1e-15. 1e-12 keeps 3 orders of magnitude of
    // slack above that without leaving room for a hypothetical layout bug.
    let inv = try_inverse(&m).expect("m is non-singular");
    println!(
        "  try_inverse(m).rows  = {}  (Some(inv) — non-singular path)",
        inv.rows()
    );
    let prod = inv.matmul(&m).expect("3×3 matmul");
    for i in 0..3 {
        for j in 0..3 {
            let target = if i == j { 1.0 } else { 0.0 };
            assert!(
                (prod.entries()[i][j].0 - target).abs() < 1e-12,
                "inv · m at ({i}, {j}) = {} ≠ {target}",
                prod.entries()[i][j].0
            );
        }
    }
    println!("  inv · m ≈ I_3        : verified to tolerance 1e-12.");

    // Round-trip: mat_from_nalgebra(mat_to_nalgebra(m)) must equal m
    // entry-wise (the conversion is lossless on finite f64).
    // 3×3 chosen for hand-verifiable det/inverse; non-square round-trip (e.g. 3×2)
    // is covered by mat_f64.rs unit tests — the bridge `DMatrix::from_fn(rows, cols, ...)`
    // in mat_f64.rs:27 correctly handles row≠col cases via explicit shape.
    let m_back = mat_from_nalgebra(&dm);
    assert_eq!(m_back.rows(), m.rows());
    assert_eq!(m_back.cols(), m.cols());
    for i in 0..m.rows() {
        for j in 0..m.cols() {
            assert_eq!(
                m_back.entries()[i][j].0,
                m.entries()[i][j].0,
                "round-trip diverged at ({i}, {j})"
            );
        }
    }
    println!("  mat_from_nalgebra(mat_to_nalgebra(m)) == m : round-trip lossless.");

    // Singular contrast: non-trivial linear dependence (row 1 = 2·row 0; rank-2;
    // det=0 by row-linearity). More pedagogically informative than a duplicate-row
    // fixture — the example communicates that try_inverse handles the general
    // rank-deficient case, not just trivially-degenerate input.
    let singular = MatR::<F64Rig>::new(
        3,
        3,
        vec![
            vec![F64Rig(1.0), F64Rig(2.0), F64Rig(3.0)],
            vec![F64Rig(2.0), F64Rig(4.0), F64Rig(6.0)], // row 1 = 2·row 0
            vec![F64Rig(4.0), F64Rig(5.0), F64Rig(6.0)],
        ],
    )
    .expect("3×3");
    let det_sing = determinant(&singular).expect("3×3 is square");
    let inv_sing = try_inverse(&singular);
    println!("  determinant(singular) = {det_sing}  (≈ 0 — row 1 = 2·row 0; rank-2)");
    println!(
        "  try_inverse(singular) = {}  (None — singular path)",
        if inv_sing.is_some() {
            "Some(_)"
        } else {
            "None"
        }
    );
    assert!(det_sing.abs() < 1e-9);
    assert!(inv_sing.is_none());
}

#[cfg(not(feature = "f64-rig"))]
fn run_mat_f64_demo() {
    println!(
        "  (Skipping nalgebra bridge subsection — run with `--features f64-rig` \n   \
         to exercise `mat_to_nalgebra` / `determinant` / `try_inverse` / `mat_from_nalgebra`.)"
    );
}
