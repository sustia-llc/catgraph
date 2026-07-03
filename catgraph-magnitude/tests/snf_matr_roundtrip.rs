//! Round-trip API: `smith_normal_form_matr<R: IntegerLikeRig>` wraps
//! the `Vec<Vec<i64>>` backend for `MatR<R>` consumers. Tests verify
//! round-trip identity over both `F64Rig` and `Z(BigInt)` inputs.

use catgraph_applied::mat::MatR;
use catgraph_applied::rig::F64Rig;
use catgraph_applied::z::Z;
use catgraph_magnitude::snf::{smith_normal_form, smith_normal_form_matr};

#[test]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn smith_normal_form_matr_wikipedia_3x3_f64rig() {
    // Wikipedia SNF example: A = [[2,4,4],[-6,6,12],[10,4,16]]; n = 1872.
    let entries_int: Vec<Vec<i64>> = vec![vec![2, 4, 4], vec![-6, 6, 12], vec![10, 4, 16]];
    let entries_float: Vec<Vec<F64Rig>> = entries_int
        .iter()
        .map(|row| row.iter().map(|&x| F64Rig(x as f64)).collect())
        .collect();
    let m = MatR::<F64Rig>::new(3, 3, entries_float).unwrap();

    let (_u, _v, s) = smith_normal_form_matr(&m, &F64Rig(1872.0)).unwrap();
    let (_u_ref, _v_ref, s_ref) = smith_normal_form(&entries_int, 1872).unwrap();

    for (i, ref_row) in s_ref.iter().enumerate() {
        for (j, &ref_val) in ref_row.iter().enumerate() {
            let lhs = s.entries()[i][j].0.round() as i64;
            assert_eq!(lhs, ref_val, "S[{i}][{j}] mismatch");
        }
    }
}

#[test]
fn smith_normal_form_matr_wikipedia_3x3_z() {
    let entries_int: Vec<Vec<i64>> = vec![vec![2, 4, 4], vec![-6, 6, 12], vec![10, 4, 16]];
    let entries_z: Vec<Vec<Z>> = entries_int
        .iter()
        .map(|row| row.iter().map(|&x| Z::from(x)).collect())
        .collect();
    let m = MatR::<Z>::new(3, 3, entries_z).unwrap();

    let (_u, _v, s) = smith_normal_form_matr(&m, &Z::from(1872_i64)).unwrap();
    let (_u_ref, _v_ref, s_ref) = smith_normal_form(&entries_int, 1872).unwrap();

    for (i, ref_row) in s_ref.iter().enumerate() {
        for (j, &ref_val) in ref_row.iter().enumerate() {
            assert_eq!(s.entries()[i][j], Z::from(ref_val), "S[{i}][{j}] mismatch");
        }
    }
}
