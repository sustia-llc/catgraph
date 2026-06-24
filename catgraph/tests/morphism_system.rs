//! Integration tests for `MorphismSystem` DAG-based resolution.
//!
//! These tests exercise `add_definition_composite` and `fill_black_boxes`,
//! which require the `rustworkx` feature (petgraph-backed `DiGraph`).
#![cfg(feature = "rustworkx")]

use catgraph::errors::CatgraphError;
use catgraph::frobenius::{Contains, InterpretableMorphism, MorphismSystem};

// ── Test fixtures ──

#[derive(Clone, Debug)]
struct TestContainer(Vec<String>);

impl Contains<String> for TestContainer {
    fn contained_labels(&self) -> Vec<String> {
        self.0.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestMorphism(i32);

impl InterpretableMorphism<TestContainer, (), String> for TestMorphism {
    fn interpret<F>(r#gen: &TestContainer, black_box_interpreter: F) -> Result<Self, CatgraphError>
    where
        F: Fn(&String, &[()], &[()]) -> Result<Self, CatgraphError>,
    {
        let mut sum = 0i32;
        for label in r#gen.contained_labels() {
            let resolved = black_box_interpreter(&label, &[], &[])?;
            sum += resolved.0;
        }
        Ok(TestMorphism(sum))
    }
}

type TestSystem = MorphismSystem<String, (), TestContainer, TestMorphism>;

// ── Tests ──

#[test]
fn multi_level_dag() {
    let mut sys: TestSystem = MorphismSystem::new("top".into());

    // Leaves
    sys.add_definition_simple("leaf1".into(), TestMorphism(10))
        .unwrap();
    sys.add_definition_simple("leaf2".into(), TestMorphism(20))
        .unwrap();
    sys.add_definition_simple("leaf3".into(), TestMorphism(30))
        .unwrap();

    // Mid composites
    sys.add_definition_composite(
        "mid1".into(),
        TestContainer(vec!["leaf1".into(), "leaf2".into()]),
    )
    .unwrap();
    sys.add_definition_composite("mid2".into(), TestContainer(vec!["leaf3".into()]))
        .unwrap();

    // Top composite
    sys.add_definition_composite(
        "top".into(),
        TestContainer(vec!["mid1".into(), "mid2".into()]),
    )
    .unwrap();

    let result = sys.fill_black_boxes(None).unwrap();
    // top = mid1 + mid2 = (leaf1 + leaf2) + leaf3 = (10 + 20) + 30 = 60
    assert_eq!(result, TestMorphism(60));
}

#[test]
fn diamond_dependency() {
    let mut sys: TestSystem = MorphismSystem::new("a".into());

    // Shared leaf
    sys.add_definition_simple("d".into(), TestMorphism(7))
        .unwrap();

    // B and C both depend on D
    sys.add_definition_composite("b".into(), TestContainer(vec!["d".into()]))
        .unwrap();
    sys.add_definition_composite("c".into(), TestContainer(vec!["d".into()]))
        .unwrap();

    // A depends on B and C
    sys.add_definition_composite("a".into(), TestContainer(vec!["b".into(), "c".into()]))
        .unwrap();

    let result = sys.fill_black_boxes(None).unwrap();
    // a = b + c = d + d = 7 + 7 = 14
    assert_eq!(result, TestMorphism(14));
}

#[test]
fn cycle_detection() {
    let mut sys: TestSystem = MorphismSystem::new("a".into());

    sys.add_definition_composite("a".into(), TestContainer(vec!["b".into()]))
        .unwrap();
    let err = sys
        .add_definition_composite("b".into(), TestContainer(vec!["a".into()]))
        .unwrap_err();

    assert!(matches!(err, CatgraphError::Interpret { .. }));
}

#[test]
fn self_cycle() {
    let mut sys: TestSystem = MorphismSystem::new("a".into());

    let err = sys
        .add_definition_composite("a".into(), TestContainer(vec!["a".into()]))
        .unwrap_err();

    assert!(matches!(err, CatgraphError::Interpret { .. }));
}

#[test]
fn resolve_specific_target() {
    let mut sys: TestSystem = MorphismSystem::new("top".into());

    sys.add_definition_simple("leaf".into(), TestMorphism(42))
        .unwrap();
    sys.add_definition_composite("mid".into(), TestContainer(vec!["leaf".into()]))
        .unwrap();
    sys.add_definition_composite("top".into(), TestContainer(vec!["mid".into()]))
        .unwrap();

    // Resolve just "mid" instead of the main "top"
    let result = sys.fill_black_boxes(Some("mid".into())).unwrap();
    assert_eq!(result, TestMorphism(42));
}

#[test]
fn missing_definition() {
    let mut sys: TestSystem = MorphismSystem::new("top".into());

    sys.add_definition_composite("top".into(), TestContainer(vec!["ghost".into()]))
        .unwrap();

    let err = sys.fill_black_boxes(None).unwrap_err();
    assert!(matches!(err, CatgraphError::Interpret { .. }));
}

#[test]
fn set_main_then_fill() {
    let mut sys: TestSystem = MorphismSystem::new("a".into());

    sys.add_definition_simple("leaf".into(), TestMorphism(5))
        .unwrap();
    sys.add_definition_composite("a".into(), TestContainer(vec!["leaf".into()]))
        .unwrap();
    sys.add_definition_composite(
        "b".into(),
        TestContainer(vec!["leaf".into(), "leaf".into()]),
    )
    .unwrap();

    // Switch main from "a" to "b"
    sys.set_main("b".into());
    let result = sys.fill_black_boxes(None).unwrap();
    // b = leaf + leaf = 5 + 5 = 10
    assert_eq!(result, TestMorphism(10));
}

#[test]
fn empty_composite() {
    let mut sys: TestSystem = MorphismSystem::new("empty".into());

    sys.add_definition_composite("empty".into(), TestContainer(vec![]))
        .unwrap();

    let result = sys.fill_black_boxes(None).unwrap();
    // Sum of no children = 0
    assert_eq!(result, TestMorphism(0));
}
