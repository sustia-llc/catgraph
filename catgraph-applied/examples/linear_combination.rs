//! LinearCombination API demonstration.
//!
//! Shows construction (singleton, FromIterator, zero), addition, subtraction,
//! negation, scalar multiplication, ring multiplication (target * target),
//! linear_combine with a custom combiner, simplify, change_coeffs,
//! linearly_extend, inj_linearly_extend, and all_terms_satisfy.

use catgraph_applied::linear_combination::LinearCombination;

// ============================================================================
// Construction
// ============================================================================

fn construction() {
    println!("=== Construction ===\n");

    // singleton: a single term with coefficient 1
    let one_x: LinearCombination<i64, String> = LinearCombination::singleton("x".into());
    println!("singleton(\"x\") = {:?}", one_x);

    // scalar multiplication to set coefficient
    let three_x = LinearCombination::<i64, String>::singleton("x".into()) * 3;
    println!("singleton(\"x\") * 3 = {:?}", three_x);

    // build from iterator
    let from_iter: LinearCombination<i64, String> = vec![
        ("a".to_string(), 2),
        ("b".to_string(), 5),
        ("c".to_string(), -1),
    ]
    .into_iter()
    .collect();
    println!("from_iter = {:?}", from_iter);

    // zero (empty combination)
    let zero: LinearCombination<i64, String> = vec![].into_iter().collect();
    println!("zero = {:?}", zero);
    println!();
}

// ============================================================================
// Addition and Subtraction
// ============================================================================

fn addition_subtraction() {
    println!("=== Addition & Subtraction ===\n");

    type LC = LinearCombination<i64, String>;

    let a: LC =
        LinearCombination::singleton("x".into()) * 3 + LinearCombination::singleton("y".into()) * 2;
    let b: LC =
        LinearCombination::singleton("y".into()) + LinearCombination::singleton("z".into()) * 5;

    // Addition merges terms, summing shared coefficients.
    let sum = a.clone() + b.clone();
    println!("a = 3x + 2y");
    println!("b = 1y + 5z");
    println!("a + b = {:?}", sum);

    // Commutativity: a + b == b + a.
    let ba = b.clone() + a.clone();
    println!("a + b == b + a: {}", sum == ba);

    // Subtraction.
    let diff = a.clone() - b.clone();
    println!("a - b = {:?}", diff);

    // Negation.
    let neg_a = -a.clone();
    println!("-a = {:?}", neg_a);

    // a + (-a) should produce all-zero coefficients.
    let mut should_be_zero = a.clone() + neg_a;
    should_be_zero.simplify();
    let zero: LC = vec![].into_iter().collect();
    println!("a + (-a) == 0 (after simplify): {}", should_be_zero == zero);
    println!();
}

// ============================================================================
// Scalar Multiplication
// ============================================================================

fn scalar_multiplication() {
    println!("=== Scalar Multiplication ===\n");

    type LC = LinearCombination<i64, String>;

    let a: LC =
        LinearCombination::singleton("x".into()) * 3 + LinearCombination::singleton("y".into()) * 5;
    println!("a = 3x + 5y");

    // Mul<Coeffs>: multiply all coefficients by a scalar.
    let scaled = a.clone() * 4;
    println!("a * 4 = {:?}", scaled);

    // MulAssign<Coeffs>: in-place scaling.
    let mut a_mut = a;
    a_mut *= 4;
    println!("a *= 4: {:?}", a_mut);

    // Distributivity: scalar * (a + b) == scalar * a + scalar * b.
    let b: LC =
        LinearCombination::singleton("y".into()) * 2 + LinearCombination::singleton("z".into());
    let lhs = (a_mut.clone() + b.clone()) * 3;
    let rhs = a_mut * 3 + b * 3;
    println!("scalar distributes over addition: {}", lhs == rhs);
    println!();
}

// ============================================================================
// Ring Multiplication (Target * Target)
// ============================================================================

fn ring_multiplication() {
    println!("=== Ring Multiplication (Target * Target) ===\n");

    // When Target implements Mul (here i64 * i64 = i64), we can multiply
    // two LinearCombinations via convolution.
    type LcInt = LinearCombination<i64, i64>;

    // lhs = 2*target(1) + 3*target(2)
    let lhs: LcInt = LinearCombination::singleton(1) * 2 + LinearCombination::singleton(2) * 3;
    // rhs = 1*target(1) + 1*target(2)
    let rhs: LcInt = LinearCombination::singleton(1) + LinearCombination::singleton(2);

    println!("lhs = 2*[1] + 3*[2]");
    println!("rhs = 1*[1] + 1*[2]");

    // Product via convolution:
    //   2*1*1 + 2*1*2 + 3*2*1 + 3*2*2
    // = 2*[1] + 2*[2] + 3*[2] + 3*[4]
    // = 2*[1] + 5*[2] + 3*[4]
    let product = lhs * rhs;
    println!("lhs * rhs = {:?}", product);

    // Distributivity: a * (b + c) == a*b + a*c.
    let a: LcInt = LinearCombination::singleton(2) * 3 + LinearCombination::singleton(5);
    let b: LcInt = LinearCombination::singleton(1) * 4 + LinearCombination::singleton(3) * 2;
    let c: LcInt = LinearCombination::singleton(1) + LinearCombination::singleton(7) * 3;
    let lhs_dist = a.clone() * (b.clone() + c.clone());
    let rhs_dist = a.clone() * b + a * c;
    println!("a*(b+c) == a*b + a*c: {}", lhs_dist == rhs_dist);
    println!();
}

// ============================================================================
// linear_combine with Custom Combiner
// ============================================================================

fn linear_combine_demo() {
    println!("=== linear_combine (Custom Combiner) ===\n");

    // linear_combine generalizes multiplication by accepting an arbitrary
    // combiner function instead of requiring Mul on the target type.
    type LC = LinearCombination<i64, String>;

    let lhs: LC =
        LinearCombination::singleton("x".into()) * 2 + LinearCombination::singleton("y".into()) * 3;
    let rhs: LC =
        LinearCombination::singleton("a".into()) + LinearCombination::singleton("b".into()) * 4;

    println!("lhs = 2\"x\" + 3\"y\"");
    println!("rhs = 1\"a\" + 4\"b\"");

    // Combine via string concatenation.
    let result = lhs.linear_combine(rhs, |s1, s2| format!("{s1}{s2}"));
    println!("linear_combine(concat) = {:?}", result);
    // Expected: "xa"=>2, "xb"=>8, "ya"=>3, "yb"=>12
    println!();
}

// ============================================================================
// Simplify (Remove Zero Coefficients)
// ============================================================================

fn simplify_demo() {
    println!("=== Simplify ===\n");

    type LC = LinearCombination<i64, String>;

    // Build a combination with a zero coefficient term.
    let zero_coeff = 0;
    let mut lc: LC = LinearCombination::singleton("a".into()) * 5
        + LinearCombination::singleton("b".into()) * zero_coeff
        + LinearCombination::singleton("c".into()) * 3;
    println!("before simplify: {:?}", lc);

    lc.simplify();
    println!("after simplify:  {:?}", lc);

    // Subtraction producing zeros, then simplify.
    let x: LC =
        LinearCombination::singleton("p".into()) * 7 + LinearCombination::singleton("q".into()) * 3;
    let y: LC =
        LinearCombination::singleton("p".into()) * 7 + LinearCombination::singleton("q".into()) * 1;
    let mut diff = x - y;
    println!("\n7p + 3q - (7p + 1q) before simplify: {:?}", diff);
    diff.simplify();
    println!("after simplify: {:?}", diff);
    println!();
}

// ============================================================================
// change_coeffs
// ============================================================================

fn change_coeffs_demo() {
    println!("=== change_coeffs ===\n");

    type LC = LinearCombination<i64, String>;

    let mut lc: LC =
        LinearCombination::singleton("a".into()) * 3 + LinearCombination::singleton("b".into()) * 5;
    println!("before: {:?}", lc);

    // Square every coefficient.
    lc.change_coeffs(|c| c * c);
    println!("after squaring coeffs: {:?}", lc);
    println!();
}

// ============================================================================
// linearly_extend and inj_linearly_extend
// ============================================================================

fn extend_demos() {
    println!("=== linearly_extend & inj_linearly_extend ===\n");

    type LC = LinearCombination<i64, i64>;

    let lc: LC = LinearCombination::singleton(1) * 2
        + LinearCombination::singleton(2) * 3
        + LinearCombination::singleton(3) * 5;
    println!("lc = 2*[1] + 3*[2] + 5*[3]");

    // linearly_extend: possibly non-injective map T -> T2.
    // Map each target to its parity. 1->1, 2->0, 3->1.
    // Target 1 gets 2+5=7, target 0 gets 3.
    let parity = lc.linearly_extend(|x| x % 2);
    println!("linearly_extend(mod 2) = {:?}", parity);

    // inj_linearly_extend: injective map T -> T2.
    type LcStr = LinearCombination<i64, String>;
    let lc_str: LcStr =
        LinearCombination::singleton("a".into()) * 2 + LinearCombination::singleton("b".into()) * 5;
    let prefixed = lc_str.inj_linearly_extend(|s| format!("prefix_{s}"));
    println!("inj_linearly_extend(prefix) = {:?}", prefixed);
    println!();
}

// ============================================================================
// all_terms_satisfy
// ============================================================================

fn predicate_demo() {
    println!("=== all_terms_satisfy ===\n");

    type LC = LinearCombination<i64, i64>;

    let lc: LC = LinearCombination::singleton(2)
        + LinearCombination::singleton(4)
        + LinearCombination::singleton(6);
    println!("lc = [2] + [4] + [6]");
    println!("all even: {}", lc.all_terms_satisfy(|t| t % 2 == 0));
    println!("all > 3:  {}", lc.all_terms_satisfy(|t| *t > 3));
    println!();
}

fn main() {
    construction();
    addition_subtraction();
    scalar_multiplication();
    ring_multiplication();
    linear_combine_demo();
    simplify_demo();
    change_coeffs_demo();
    extend_demos();
    predicate_demo();
}
