//! Integration tests for higher-order `LinearCombination` methods.
//!
//! Focuses on `linear_combine`, `change_coeffs`, `all_terms_satisfy`,
//! `inj_linearly_extend`, and `linearly_extend` — functions that transform
//! linear combinations via closures and produce new algebraic structures.

use catgraph_applied::linear_combination::LinearCombination;

/// Helper: build a `LinearCombination<i64, String>` from `(coeff, basis)` pairs.
/// Uses `FromIterator`, swapping to `(basis, coeff)` order.
fn lc(pairs: &[(i64, &str)]) -> LinearCombination<i64, String> {
    pairs.iter().map(|(c, b)| (b.to_string(), *c)).collect()
}

/// Helper: build a `LinearCombination<i64, i64>` from `(coeff, basis)` pairs.
fn lc_int(pairs: &[(i64, i64)]) -> LinearCombination<i64, i64> {
    pairs.iter().map(|(c, b)| (*b, *c)).collect()
}

// ---------------------------------------------------------------------------
// 1. linear_combine — basic bilinear multiplication via combiner
// ---------------------------------------------------------------------------

#[test]
fn linear_combine_pairs_all_terms() {
    // lhs = 2"x" + 3"y", rhs = 1"a" + 4"b"
    // combiner: concatenate strings => "xa","xb","ya","yb"
    let lhs = lc(&[(2, "x"), (3, "y")]);
    let rhs = lc(&[(1, "a"), (4, "b")]);

    let result: LinearCombination<i64, String> =
        lhs.linear_combine(rhs, |s1, s2| format!("{s1}{s2}"));

    // Expected: "xa"=>2*1=2, "xb"=>2*4=8, "ya"=>3*1=3, "yb"=>3*4=12
    let expected = lc(&[(2, "xa"), (8, "xb"), (3, "ya"), (12, "yb")]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// 2. linear_combine — with coefficient collisions (non-injective combiner)
// ---------------------------------------------------------------------------

#[test]
fn linear_combine_with_collisions() {
    // lhs = 1*10 + 2*20, rhs = 1*3 + 1*7
    // combiner: addition (10+3=13, 10+7=17, 20+3=23, 20+7=27) — all distinct
    let lhs = lc_int(&[(1, 10), (2, 20)]);
    let rhs = lc_int(&[(1, 3), (1, 7)]);

    let result: LinearCombination<i64, i64> = lhs.linear_combine(rhs, |a, b| a + b);

    // 10+3=13 => 1*1=1, 10+7=17 => 1*1=1, 20+3=23 => 2*1=2, 20+7=27 => 2*1=2
    let expected = lc_int(&[(1, 13), (1, 17), (2, 23), (2, 27)]);
    assert_eq!(result, expected);
}

#[test]
fn linear_combine_collision_merges_coefficients() {
    // lhs = 3*1 + 5*2, rhs = 1*10 + 1*11
    // combiner: |a, b| b (project to rhs basis, so 1->10, 1->11, 2->10, 2->11)
    // Two pairs map to 10: (1,10)=>3*1=3 and (2,10)=>5*1=5 => merged 8
    // Two pairs map to 11: (1,11)=>3*1=3 and (2,11)=>5*1=5 => merged 8
    let lhs = lc_int(&[(3, 1), (5, 2)]);
    let rhs = lc_int(&[(1, 10), (1, 11)]);

    let result: LinearCombination<i64, i64> = lhs.linear_combine(rhs, |_a, b| b);

    let expected = lc_int(&[(8, 10), (8, 11)]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// 3. linear_combine — one side empty yields empty result
// ---------------------------------------------------------------------------

#[test]
fn linear_combine_with_empty_yields_empty() {
    let lhs = lc(&[(2, "x"), (3, "y")]);
    let empty: LinearCombination<i64, String> = LinearCombination::default();

    let result: LinearCombination<i64, String> =
        lhs.linear_combine(empty, |s1, s2| format!("{s1}{s2}"));

    assert_eq!(result, LinearCombination::default());
}

// ---------------------------------------------------------------------------
// 4. change_coeffs — double all coefficients
// ---------------------------------------------------------------------------

#[test]
fn change_coeffs_doubles_values() {
    let mut combo = lc(&[(3, "a"), (7, "b"), (-2, "c")]);
    combo.change_coeffs(|c| c * 2);

    let expected = lc(&[(6, "a"), (14, "b"), (-4, "c")]);
    assert_eq!(combo, expected);
}

// ---------------------------------------------------------------------------
// 5. change_coeffs — absolute value (non-linear map)
// ---------------------------------------------------------------------------

#[test]
fn change_coeffs_absolute_value() {
    let mut combo = lc(&[(-5, "neg"), (3, "pos"), (0, "zero")]);
    combo.change_coeffs(i64::abs);

    let expected = lc(&[(5, "neg"), (3, "pos"), (0, "zero")]);
    assert_eq!(combo, expected);
}

// ---------------------------------------------------------------------------
// 6. all_terms_satisfy — true when all match, false otherwise
// ---------------------------------------------------------------------------

#[test]
fn all_terms_satisfy_predicate() {
    let short_words = lc(&[(1, "ab"), (2, "cd"), (3, "ef")]);
    assert!(short_words.all_terms_satisfy(|s| s.len() == 2));
    assert!(!short_words.all_terms_satisfy(|s| s.starts_with('a')));

    // Empty combination: vacuously true for any predicate.
    let empty: LinearCombination<i64, String> = LinearCombination::default();
    assert!(empty.all_terms_satisfy(|_| false));
}

// ---------------------------------------------------------------------------
// 7. inj_linearly_extend — injective map preserves coefficients
// ---------------------------------------------------------------------------

#[test]
fn inj_linearly_extend_preserves_coefficients() {
    let combo = lc(&[(4, "alpha"), (9, "beta")]);

    // Injective map: wrap each basis element in angle brackets.
    let result: LinearCombination<i64, String> = combo.inj_linearly_extend(|s| format!("<{s}>"));

    let expected = lc(&[(4, "<alpha>"), (9, "<beta>")]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// 8. inj_linearly_extend panics on non-injective map
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "injective")]
fn inj_linearly_extend_panics_on_non_injective() {
    let combo = lc(&[(1, "foo"), (2, "bar")]);
    // Constant map: both "foo" and "bar" map to "same".
    let _ = combo.inj_linearly_extend(|_| "same".to_string());
}

// ---------------------------------------------------------------------------
// 9. linearly_extend — non-injective map merges coefficients
// ---------------------------------------------------------------------------

#[test]
fn linearly_extend_merges_on_collision() {
    // combo = 2"apple" + 5"avocado" + 3"banana"
    let combo = lc(&[(2, "apple"), (5, "avocado"), (3, "banana")]);

    // Map each fruit to its first character (non-injective: apple and avocado both -> "a").
    let result: LinearCombination<i64, String> =
        combo.linearly_extend(|s| s.chars().next().unwrap().to_string());

    // "a" => 2 + 5 = 7, "b" => 3
    let expected = lc(&[(7, "a"), (3, "b")]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// 10. linearly_extend — injective case matches inj_linearly_extend
// ---------------------------------------------------------------------------

#[test]
fn linearly_extend_injective_case_matches_inj() {
    let combo = lc(&[(6, "one"), (11, "two")]);

    // An actually-injective map: uppercase.
    let via_inj: LinearCombination<i64, String> = combo.inj_linearly_extend(|s| s.to_uppercase());
    let via_gen: LinearCombination<i64, String> = combo.linearly_extend(|s| s.to_uppercase());

    assert_eq!(via_inj, via_gen);
}
