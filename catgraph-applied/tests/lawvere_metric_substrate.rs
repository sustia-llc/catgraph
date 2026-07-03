//! Tests for the `LawvereMetricSpace` substrate (`size`, `objects`, `from_distance_fn`).

#![allow(clippy::float_cmp)]

use catgraph_applied::lawvere_metric::LawvereMetricSpace;

#[test]
fn size_returns_object_count() {
    let space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![0, 1, 2, 3]);
    assert_eq!(space.size(), 4);
}

#[test]
fn objects_exposes_object_slice() {
    let space: LawvereMetricSpace<usize> = LawvereMetricSpace::new(vec![10, 20, 30]);
    assert_eq!(space.objects(), &[10, 20, 30][..]);
}

#[test]
fn from_distance_fn_uniform_metric() {
    let space =
        LawvereMetricSpace::<usize>::from_distance_fn(3, |a, b| if a == b { 0.0 } else { 2.0 });
    assert_eq!(space.size(), 3);
    assert_eq!(space.distance(&0, &0).0, 0.0);
    assert_eq!(space.distance(&0, &1).0, 2.0);
    assert_eq!(space.distance(&2, &1).0, 2.0);
}

#[test]
fn from_distance_fn_table_lookup() {
    let table = [[0.0, 1.0, 2.0], [1.0, 0.0, 1.0], [2.0, 1.0, 0.0]];
    let space = LawvereMetricSpace::<usize>::from_distance_fn(3, |a, b| table[a][b]);
    assert!(space.triangle_inequality_holds());
}
