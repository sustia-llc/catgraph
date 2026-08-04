use crate::errors::CatgraphError;

use std::{collections::HashMap, marker::PhantomData};

/// Trait for types that reference other labels (used for dependency tracking).
pub trait Contains<BlackBoxLabel> {
    fn contained_labels(&self) -> Vec<BlackBoxLabel>;
}

/// Trait for morphisms that can be constructed by interpreting a general (possibly black-boxed) description.
pub trait InterpretableMorphism<GeneralVersion, Lambda, BlackBoxLabel>: Sized {
    /// Interpret a general morphism description, resolving black boxes via the provided closure.
    ///
    /// # Errors
    ///
    /// Returns `CatgraphError` if black box interpretation fails.
    fn interpret<F>(
        r#gen: &GeneralVersion,
        black_box_interpreter: F,
    ) -> Result<Self, CatgraphError>
    where
        F: Fn(&BlackBoxLabel, &[Lambda], &[Lambda]) -> Result<Self, CatgraphError>;
}

/// Topologically sort the nodes `0..n` under `edges`, or return `None` if the
/// graph has a cycle.
///
/// Kahn's algorithm: repeatedly emit a node whose remaining in-degree is zero and
/// decrement its successors. For an edge `u -> v` this emits `u` before `v`; a
/// cycle is exactly the case where the queue empties before all `n` nodes are
/// emitted, since every node on a cycle keeps an in-degree of at least one.
///
/// `edges` may contain duplicates and they are counted with multiplicity, which is
/// consistent: each occurrence contributes one increment and one decrement. A
/// self-loop `u -> u` is a cycle and is reported as one.
///
/// Nodes are emitted in whatever order the ready-stack yields, which is
/// deterministic for a given `edges` but is not sorted by id — it is one valid
/// topological order, not a canonical one.
fn toposort(n: usize, edges: &[(usize, usize)]) -> Option<Vec<usize>> {
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree: Vec<usize> = vec![0; n];
    for &(u, v) in edges {
        successors[u].push(v);
        in_degree[v] += 1;
    }

    let mut ready: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut order: Vec<usize> = Vec::with_capacity(n);
    while let Some(u) = ready.pop() {
        order.push(u);
        for &v in &successors[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                ready.push(v);
            }
        }
    }

    // Short of `n` means some node never reached in-degree zero, i.e. a cycle.
    (order.len() == n).then_some(order)
}

/// DAG of named morphisms resolved via topological sort.
///
/// Composite definitions may reference other labels; `fill_black_boxes` resolves them
/// bottom-up into concrete `T` values using `InterpretableMorphism::interpret`.
///
/// The dependency graph is not stored; it is derived on demand from
/// `composite_pieces` (each `(label, def)` contributes edges `label -> child` for
/// every `child` in `def.contained_labels()`) plus the leaf labels in
/// `simple_pieces` and `main`. Acyclicity and resolution order are computed with
/// a local Kahn's-algorithm pass.
pub struct MorphismSystem<BlackBoxLabel, Lambda, GeneralBlackBoxed, T>
where
    BlackBoxLabel: std::hash::Hash + Eq,
    T: InterpretableMorphism<GeneralBlackBoxed, Lambda, BlackBoxLabel>,
    GeneralBlackBoxed: Contains<BlackBoxLabel>,
{
    composite_pieces: HashMap<BlackBoxLabel, GeneralBlackBoxed>,
    simple_pieces: HashMap<BlackBoxLabel, T>,
    pub(crate) main: BlackBoxLabel,
    dummy: PhantomData<Lambda>,
}

impl<GeneralBlackBoxed, BlackBoxLabel, Lambda, T>
    MorphismSystem<BlackBoxLabel, Lambda, GeneralBlackBoxed, T>
where
    BlackBoxLabel: std::hash::Hash + Eq + Clone + std::fmt::Debug,
    Lambda: Eq + std::fmt::Debug + Copy,
    T: InterpretableMorphism<GeneralBlackBoxed, Lambda, BlackBoxLabel> + Clone,
    GeneralBlackBoxed: Contains<BlackBoxLabel>,
{
    /// Create an empty system with the given main entry point label.
    pub fn new(main_name: BlackBoxLabel) -> Self {
        Self {
            composite_pieces: HashMap::new(),
            simple_pieces: HashMap::new(),
            main: main_name,
            dummy: PhantomData,
        }
    }

    /// Build the dependency graph over the current state — assigning each distinct
    /// `BlackBoxLabel` a `usize` id, adding an edge `parent -> child` for every
    /// composite `(parent, def)` and each `child` in `def.contained_labels()`, plus
    /// isolated nodes for the `simple_pieces` leaves and `main` — and run
    /// [`toposort`] over it.
    ///
    /// An optional `prospective` `(parent, children)` group adds a not-yet-committed
    /// composite's edges (used by `add_definition_composite` to test a candidate
    /// before insertion).
    ///
    /// Returns `None` when the graph is cyclic, otherwise `Some(order)` with the
    /// labels in topological order. By the `parent -> child` convention, the sort
    /// yields each parent before its children (u-before-v for edge u->v), matching
    /// the resolution order `fill_black_boxes` expects.
    ///
    /// Note that the label ids depend on `HashMap` iteration order, so the specific
    /// topological order this returns may differ between runs. Every order it can
    /// return is a valid one, which is all either caller needs.
    fn resolve_order(
        &self,
        prospective: Option<(&BlackBoxLabel, &[BlackBoxLabel])>,
    ) -> Option<Vec<BlackBoxLabel>> {
        let mut id_of: HashMap<BlackBoxLabel, usize> = HashMap::new();
        let mut labels: Vec<BlackBoxLabel> = Vec::new();
        let mut edges: Vec<(usize, usize)> = Vec::new();

        {
            let mut intern = |label: &BlackBoxLabel| -> usize {
                if let Some(&id) = id_of.get(label) {
                    id
                } else {
                    let id = labels.len();
                    id_of.insert(label.clone(), id);
                    labels.push(label.clone());
                    id
                }
            };

            // Committed composite edges: parent -> child.
            for (parent, def) in &self.composite_pieces {
                let p = intern(parent);
                for child in def.contained_labels() {
                    let c = intern(&child);
                    edges.push((p, c));
                }
            }
            // The candidate composite under test, if any.
            if let Some((parent, children)) = prospective {
                let p = intern(parent);
                for child in children {
                    let c = intern(child);
                    edges.push((p, c));
                }
            }
            // Leaf simples and `main` appear as nodes even when unreferenced.
            for leaf in self.simple_pieces.keys() {
                intern(leaf);
            }
            intern(&self.main);
        }

        let order = toposort(labels.len(), &edges)?;
        Some(order.into_iter().map(|id| labels[id].clone()).collect())
    }

    /// Register a composite definition that depends on other labels.
    ///
    /// Derives the dependency graph including the prospective edges
    /// `new_name -> child` for each label returned by `new_def.contained_labels()`,
    /// then verifies acyclicity via `topological_sort`. Returns
    /// `CatgraphError::Interpret` if the addition would create a cycle.
    ///
    /// # Errors
    ///
    /// - Adding the definition would create a cycle in the dependency DAG.
    pub fn add_definition_composite(
        &mut self,
        new_name: BlackBoxLabel,
        new_def: GeneralBlackBoxed,
    ) -> Result<(), CatgraphError> {
        let contained = new_def.contained_labels();
        if self.resolve_order(Some((&new_name, &contained))).is_none() {
            return Err(CatgraphError::Interpret {
                context: format!(
                    "Adding composite {new_name:?} would create a cycle in the dependency DAG"
                ),
            });
        }
        self.composite_pieces.insert(new_name, new_def);
        Ok(())
    }

    /// Register a simple (leaf) definition with no dependencies.
    ///
    /// Stores the definition in `simple_pieces`; the dependency graph derives a
    /// leaf node for `new_name` on demand.
    ///
    /// # Errors
    ///
    /// - Name already exists in the system.
    #[allow(clippy::unnecessary_wraps)] // consistent API with add_definition_composite
    pub fn add_definition_simple(
        &mut self,
        new_name: BlackBoxLabel,
        new_def: T,
    ) -> Result<(), CatgraphError> {
        self.simple_pieces.insert(new_name, new_def);
        Ok(())
    }

    /// Change which label is treated as the top-level entry point for resolution.
    pub fn set_main(&mut self, main_name: BlackBoxLabel) {
        self.main = main_name;
    }

    fn interpret_nomut(&self, interpret_target: Option<BlackBoxLabel>) -> Result<T, CatgraphError> {
        let which_interpreting = interpret_target.unwrap_or(self.main.clone());
        if let Some(simple_answer) = self.simple_pieces.get(&which_interpreting) {
            return Ok(simple_answer.clone());
        }
        let complicated_answer = self.composite_pieces.get(&which_interpreting);
        if let Some(complicated_answer_2) = complicated_answer {
            let black_box_interpreter = |bb: &BlackBoxLabel, _src: &[Lambda], _tgt: &[Lambda]| {
                let simple_answer = self
                    .simple_pieces
                    .get(bb)
                    .ok_or(CatgraphError::Interpret {
                        context: format!("No filling for {:?}", bb.clone()),
                    })
                    .cloned();
                if simple_answer.is_err() {
                    self.interpret_nomut(Some(bb.clone()))
                } else {
                    simple_answer
                }
            };
            T::interpret(complicated_answer_2, black_box_interpreter).map_err(
                |internal_explanation| CatgraphError::Interpret {
                    context: format!("When doing {which_interpreting:?}\n{internal_explanation:?}"),
                },
            )
        } else {
            Err(CatgraphError::Interpret {
                context: format!("No {which_interpreting:?} found"),
            })
        }
    }

    /// Resolve all definitions in topological order, returning the concrete morphism for
    /// `interpret_target` (or `main` if `None`). Resolved composites are cached as simple pieces.
    ///
    /// # Errors
    ///
    /// - Topological resolution fails (cyclic dependencies).
    /// - Black box interpretation fails for any definition.
    pub fn fill_black_boxes(
        &mut self,
        interpret_target: Option<BlackBoxLabel>,
    ) -> Result<T, CatgraphError> {
        let which_interpreting = interpret_target.unwrap_or(self.main.clone());
        if let Some(simple_answer) = self.simple_pieces.get(&which_interpreting) {
            return Ok(simple_answer.clone());
        }

        let Some(ordered) = self.resolve_order(None) else {
            return Err(CatgraphError::Interpret {
                context: "Not acyclic dependencies".to_string(),
            });
        };

        for my_bb in ordered {
            let cur_answer = self.interpret_nomut(Some(my_bb.clone()));
            if let Ok(real_cur_answer) = cur_answer.clone() {
                self.simple_pieces.insert(my_bb.clone(), real_cur_answer);
                let _ = self.composite_pieces.remove(&my_bb);
            }
            if my_bb == which_interpreting {
                return cur_answer;
            }
        }
        Err(CatgraphError::Interpret {
            context: format!("Through all but never found {which_interpreting:?}"),
        })
    }
}

#[cfg(test)]
mod test {
    use super::{CatgraphError, Contains, InterpretableMorphism, MorphismSystem};

    #[test]
    fn catgraph_error_interpret() {
        let error = CatgraphError::Interpret {
            context: "test error".to_string(),
        };
        match error {
            CatgraphError::Interpret { context } => assert_eq!(context, "test error"),
            _ => panic!("Expected Interpret variant"),
        }
    }

    #[test]
    fn catgraph_error_interpret_display() {
        let error = CatgraphError::Interpret {
            context: "test error".to_string(),
        };
        let display_str = format!("{error}");
        assert!(display_str.contains("test error"));
    }

    #[test]
    fn contains_trait() {
        #[derive(Clone)]
        struct SimpleContainer {
            labels: Vec<String>,
        }

        impl Contains<String> for SimpleContainer {
            fn contained_labels(&self) -> Vec<String> {
                self.labels.clone()
            }
        }

        let container = SimpleContainer {
            labels: vec!["a".to_string(), "b".to_string()],
        };
        let labels = container.contained_labels();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0], "a");
        assert_eq!(labels[1], "b");
    }

    #[test]
    fn morphism_system_new() {
        #[derive(Clone)]
        struct SimpleContainer {
            labels: Vec<String>,
        }
        impl Contains<String> for SimpleContainer {
            fn contained_labels(&self) -> Vec<String> {
                self.labels.clone()
            }
        }
        #[derive(Clone, Debug)]
        struct SimpleMorphism;
        impl InterpretableMorphism<SimpleContainer, char, String> for SimpleMorphism {
            fn interpret<F>(
                _container: &SimpleContainer,
                _black_box_interpreter: F,
            ) -> Result<Self, CatgraphError>
            where
                F: Fn(&String, &[char], &[char]) -> Result<Self, CatgraphError>,
            {
                Ok(SimpleMorphism)
            }
        }

        let system: MorphismSystem<String, char, SimpleContainer, SimpleMorphism> =
            MorphismSystem::new("main".to_string());
        assert_eq!(system.main, "main".to_string());
    }

    #[test]
    fn morphism_system_set_main() {
        #[derive(Clone)]
        struct SimpleContainer {
            labels: Vec<String>,
        }
        impl Contains<String> for SimpleContainer {
            fn contained_labels(&self) -> Vec<String> {
                self.labels.clone()
            }
        }
        #[derive(Clone, Debug)]
        struct SimpleMorphism {
            name: String,
        }
        impl InterpretableMorphism<SimpleContainer, char, String> for SimpleMorphism {
            fn interpret<F>(
                container: &SimpleContainer,
                _black_box_interpreter: F,
            ) -> Result<Self, CatgraphError>
            where
                F: Fn(&String, &[char], &[char]) -> Result<Self, CatgraphError>,
            {
                Ok(SimpleMorphism {
                    name: container.labels.join(","),
                })
            }
        }

        let mut system: MorphismSystem<String, char, SimpleContainer, SimpleMorphism> =
            MorphismSystem::new("initial".to_string());
        system.set_main("new_main".to_string());
        assert_eq!(system.main, "new_main".to_string());

        // read the name field so it is not considered dead code
        let container = SimpleContainer {
            labels: vec!["a".to_string(), "b".to_string()],
        };
        let interpreted = SimpleMorphism::interpret(&container, |_bb, _src, _tgt| {
            panic!("No black-boxs expected")
        })
        .unwrap();
        assert_eq!(interpreted.name, "a,b".to_string());
    }

    // ── `toposort` unit tests ──
    // Direct coverage of the Kahn pass underneath `resolve_order`, including the
    // cycle detection both `add_definition_composite` and `fill_black_boxes`
    // depend on for their error paths.

    /// Every edge `u -> v` puts `u` strictly before `v` in the returned order,
    /// and every node appears exactly once.
    fn assert_valid_topological_order(n: usize, edges: &[(usize, usize)], order: &[usize]) {
        assert_eq!(order.len(), n, "order covers every node");
        let mut position = vec![usize::MAX; n];
        for (pos, &node) in order.iter().enumerate() {
            assert_eq!(position[node], usize::MAX, "node {node} emitted twice");
            position[node] = pos;
        }
        for &(u, v) in edges {
            assert!(
                position[u] < position[v],
                "edge {u} -> {v} is inverted in the order"
            );
        }
    }

    #[test]
    fn toposort_empty_graph_is_the_empty_order() {
        assert_eq!(super::toposort(0, &[]), Some(Vec::new()));
    }

    #[test]
    fn toposort_isolated_nodes_all_appear() {
        let order = super::toposort(3, &[]).expect("no edges, so acyclic");
        assert_valid_topological_order(3, &[], &order);
    }

    #[test]
    fn toposort_chain_orders_parent_before_child() {
        // 0 -> 1 -> 2, the shape `fill_black_boxes` resolves along.
        let edges = [(0, 1), (1, 2)];
        let order = super::toposort(3, &edges).expect("a chain is acyclic");
        assert_eq!(order, vec![0, 1, 2], "a chain has exactly one valid order");
    }

    #[test]
    fn toposort_diamond_respects_every_edge() {
        // 0 -> {1, 2} -> 3: two valid orders, so check the property not a literal.
        let edges = [(0, 1), (0, 2), (1, 3), (2, 3)];
        let order = super::toposort(4, &edges).expect("a diamond is acyclic");
        assert_valid_topological_order(4, &edges, &order);
    }

    #[test]
    fn toposort_duplicate_edges_are_counted_with_multiplicity() {
        // The same `parent -> child` twice must not leave a residual in-degree.
        let edges = [(0, 1), (0, 1), (1, 2)];
        let order = super::toposort(3, &edges).expect("duplicated edges are still acyclic");
        assert_valid_topological_order(3, &edges, &order);
    }

    #[test]
    fn toposort_detects_a_two_node_cycle() {
        assert_eq!(super::toposort(2, &[(0, 1), (1, 0)]), None);
    }

    #[test]
    fn toposort_detects_a_self_loop() {
        assert_eq!(super::toposort(1, &[(0, 0)]), None);
    }

    #[test]
    fn toposort_detects_a_cycle_disjoint_from_an_acyclic_part() {
        // 0 -> 1 is fine; 2 <-> 3 is not. The whole sort must fail.
        assert_eq!(super::toposort(4, &[(0, 1), (2, 3), (3, 2)]), None);
    }

    // ── MorphismSystem DAG registration tests ──
    // These tests exercise composite definitions and topological resolution
    // over the derived dependency graph.

    /// Shared test scaffolding: a container whose `contained_labels` returns
    /// an arbitrary set of `String` labels, and a morphism that records
    /// the label it was resolved from via `interpret`.
    mod dag_fixtures {
        use super::*;

        #[derive(Clone, Debug)]
        pub struct TestContainer {
            pub labels: Vec<String>,
        }

        impl Contains<String> for TestContainer {
            fn contained_labels(&self) -> Vec<String> {
                self.labels.clone()
            }
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct TestMorphism {
            pub resolved_from: String,
        }

        impl InterpretableMorphism<TestContainer, char, String> for TestMorphism {
            fn interpret<F>(
                container: &TestContainer,
                black_box_interpreter: F,
            ) -> Result<Self, CatgraphError>
            where
                F: Fn(&String, &[char], &[char]) -> Result<Self, CatgraphError>,
            {
                // Resolve the first contained label via the interpreter,
                // falling back to the joined label list as the name.
                if let Some(first) = container.labels.first() {
                    black_box_interpreter(first, &[], &[])
                } else {
                    Ok(TestMorphism {
                        resolved_from: container.labels.join("+"),
                    })
                }
            }
        }

        pub type TestSystem = MorphismSystem<String, char, TestContainer, TestMorphism>;

        pub fn new_system(main: &str) -> TestSystem {
            MorphismSystem::new(main.to_string())
        }
    }

    use dag_fixtures::{TestContainer, TestMorphism, new_system};

    #[test]
    fn add_simple_definitions_then_fill() {
        let mut sys = new_system("A");
        sys.add_definition_simple(
            "A".to_string(),
            TestMorphism {
                resolved_from: "leaf-A".into(),
            },
        )
        .unwrap();
        sys.add_definition_simple(
            "B".to_string(),
            TestMorphism {
                resolved_from: "leaf-B".into(),
            },
        )
        .unwrap();

        // fill_black_boxes should resolve immediately for a simple definition
        let result = sys.fill_black_boxes(None).unwrap();
        assert_eq!(result.resolved_from, "leaf-A");

        let result_b = sys.fill_black_boxes(Some("B".to_string())).unwrap();
        assert_eq!(result_b.resolved_from, "leaf-B");
    }

    #[test]
    fn composite_referencing_simples_resolves() {
        let mut sys = new_system("top");

        // Register two leaf definitions
        sys.add_definition_simple(
            "leaf1".to_string(),
            TestMorphism {
                resolved_from: "leaf1".into(),
            },
        )
        .unwrap();
        sys.add_definition_simple(
            "leaf2".to_string(),
            TestMorphism {
                resolved_from: "leaf2".into(),
            },
        )
        .unwrap();

        // Register a composite that depends on leaf1 and leaf2
        sys.add_definition_composite(
            "top".to_string(),
            TestContainer {
                labels: vec!["leaf1".to_string(), "leaf2".to_string()],
            },
        )
        .unwrap();

        // fill_black_boxes resolves the composite through topological order
        let result = sys.fill_black_boxes(None).unwrap();
        // interpret delegates to black_box_interpreter for the first label ("leaf1")
        assert_eq!(result.resolved_from, "leaf1");
    }

    #[test]
    fn cycle_detection_returns_error() {
        let mut sys = new_system("A");

        // A depends on B
        sys.add_definition_composite(
            "A".to_string(),
            TestContainer {
                labels: vec!["B".to_string()],
            },
        )
        .unwrap();

        // B depends on A — this would create a cycle
        let result = sys.add_definition_composite(
            "B".to_string(),
            TestContainer {
                labels: vec!["A".to_string()],
            },
        );

        assert!(result.is_err());
        match result {
            Err(CatgraphError::Interpret { context }) => {
                assert!(
                    context.contains("cycle"),
                    "Error message should mention 'cycle', got: {context}"
                );
            }
            other => panic!("Expected Interpret error, got: {other:?}"),
        }
    }

    #[test]
    fn deep_chain_resolves_in_topological_order() {
        // Chain: top → mid → base (each composite except the leaf)
        let mut sys = new_system("top");

        sys.add_definition_simple(
            "base".to_string(),
            TestMorphism {
                resolved_from: "base".into(),
            },
        )
        .unwrap();

        sys.add_definition_composite(
            "mid".to_string(),
            TestContainer {
                labels: vec!["base".to_string()],
            },
        )
        .unwrap();

        sys.add_definition_composite(
            "top".to_string(),
            TestContainer {
                labels: vec!["mid".to_string()],
            },
        )
        .unwrap();

        let result = sys.fill_black_boxes(None).unwrap();
        // "top" interpret calls black_box_interpreter("mid"), which was
        // already resolved to the "base" leaf by topological processing
        assert_eq!(result.resolved_from, "base");
    }

    #[test]
    fn diamond_dependency_resolves() {
        //    top
        //   /   \
        //  left  right
        //   \   /
        //    base
        let mut sys = new_system("top");

        sys.add_definition_simple(
            "base".to_string(),
            TestMorphism {
                resolved_from: "base".into(),
            },
        )
        .unwrap();

        sys.add_definition_composite(
            "left".to_string(),
            TestContainer {
                labels: vec!["base".to_string()],
            },
        )
        .unwrap();

        sys.add_definition_composite(
            "right".to_string(),
            TestContainer {
                labels: vec!["base".to_string()],
            },
        )
        .unwrap();

        sys.add_definition_composite(
            "top".to_string(),
            TestContainer {
                labels: vec!["left".to_string(), "right".to_string()],
            },
        )
        .unwrap();

        let result = sys.fill_black_boxes(None).unwrap();
        // "top" interpret resolves first contained label "left", which
        // was already resolved to "base"
        assert_eq!(result.resolved_from, "base");
    }

    #[test]
    fn self_cycle_detected() {
        let mut sys = new_system("A");

        // A depends on itself
        let result = sys.add_definition_composite(
            "A".to_string(),
            TestContainer {
                labels: vec!["A".to_string()],
            },
        );

        assert!(result.is_err());
        match result {
            Err(CatgraphError::Interpret { context }) => {
                assert!(
                    context.contains("cycle"),
                    "Expected cycle error, got: {context}"
                );
            }
            other => panic!("Expected Interpret error, got: {other:?}"),
        }
    }

    #[test]
    fn adding_composite_with_no_deps_succeeds() {
        let mut sys = new_system("solo");

        // A composite with an empty contained_labels list is valid
        sys.add_definition_composite("solo".to_string(), TestContainer { labels: vec![] })
            .unwrap();

        let result = sys.fill_black_boxes(None).unwrap();
        // interpret with empty labels returns joined label list ""
        assert_eq!(result.resolved_from, "");
    }
}
