//! Core category-theoretic composition traits.
//!
//! Provides identity morphisms and two composition interfaces (immutable and
//! mutating) that form the backbone of all categorical structures in catgraph.

use crate::errors::CatgraphError;

/// A type that can produce identity morphisms on objects of type `T`.
pub trait HasIdentity<T>: Sized {
    /// Returns the identity morphism on the given object.
    fn identity(on_this: &T) -> Self;
}

/// Immutable composition of morphisms with domain/codomain of type `T`.
pub trait Composable<T: Eq>: Sized {
    /// Composes `self` with `other` (self ; other), returning a new morphism.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if `self.codomain()` does not match `other.domain()`.
    fn compose(&self, other: &Self) -> Result<Self, CatgraphError>;
    fn domain(&self) -> T;
    fn codomain(&self) -> T;
    /// Checks that `self.codomain() == other.domain()`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the codomain/domain interface is incompatible.
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        if self.codomain() == other.domain() {
            Ok(())
        } else {
            Err(CatgraphError::Composition {
                message: "Not composable. No details on how domain and codomain mismatched"
                    .to_string(),
            })
        }
    }
}

/// In-place composition of morphisms, consuming `other` and mutating `self`.
pub trait ComposableMutating<T: Eq>: Sized {
    /// Composes `self` with `other` in place (self := self ; other).
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if `self.codomain()` does not match `other.domain()`.
    fn compose(&mut self, other: Self) -> Result<(), CatgraphError>;
    fn domain(&self) -> T;
    fn codomain(&self) -> T;
    /// Checks that `self.codomain() == other.domain()`.
    ///
    /// # Errors
    ///
    /// Returns [`CatgraphError`] if the codomain/domain interface is incompatible.
    fn composable(&self, other: &Self) -> Result<(), CatgraphError> {
        if self.codomain() == other.domain() {
            Ok(())
        } else {
            Err(CatgraphError::Composition {
                message: "Not composable. No details on how domain and codomain mismatched"
                    .to_string(),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn catgraph_error_composition_from_string() {
        let error = CatgraphError::Composition {
            message: "test error".to_string(),
        };
        match error {
            CatgraphError::Composition { message } => assert_eq!(message, "test error"),
            _ => panic!("Expected Composition variant"),
        }
    }

    #[test]
    fn catgraph_error_composition_display() {
        let error = CatgraphError::Composition {
            message: "display test".to_string(),
        };
        let display_str = format!("{error}");
        assert!(display_str.contains("display test"));
    }

    #[test]
    fn catgraph_error_composition_debug() {
        let error = CatgraphError::Composition {
            message: "debug test".to_string(),
        };
        let debug_str = format!("{error:?}");
        assert!(debug_str.contains("debug test"));
    }

    // Test implementation for HasIdentity
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestMorphism {
        source: i32,
        target: i32,
    }

    impl HasIdentity<i32> for TestMorphism {
        fn identity(on_this: &i32) -> Self {
            TestMorphism {
                source: *on_this,
                target: *on_this,
            }
        }
    }

    impl Composable<i32> for TestMorphism {
        fn compose(&self, other: &Self) -> Result<Self, CatgraphError> {
            if self.target == other.source {
                Ok(TestMorphism {
                    source: self.source,
                    target: other.target,
                })
            } else {
                Err(CatgraphError::Composition {
                    message: "Cannot compose".to_string(),
                })
            }
        }

        fn domain(&self) -> i32 {
            self.source
        }

        fn codomain(&self) -> i32 {
            self.target
        }
    }

    #[test]
    fn has_identity_trait() {
        let id = TestMorphism::identity(&5);
        assert_eq!(id.source, 5);
        assert_eq!(id.target, 5);
    }

    #[test]
    fn composable_trait_compose() {
        let f = TestMorphism {
            source: 1,
            target: 2,
        };
        let g = TestMorphism {
            source: 2,
            target: 3,
        };
        let result = f.compose(&g);
        assert!(result.is_ok());
        let composed = result.unwrap();
        assert_eq!(composed.source, 1);
        assert_eq!(composed.target, 3);
    }

    #[test]
    fn composable_trait_compose_fail() {
        let f = TestMorphism {
            source: 1,
            target: 2,
        };
        let g = TestMorphism {
            source: 3,
            target: 4,
        };
        let result = f.compose(&g);
        assert!(result.is_err());
    }

    #[test]
    fn composable_trait_domain_codomain() {
        let f = TestMorphism {
            source: 10,
            target: 20,
        };
        assert_eq!(f.domain(), 10);
        assert_eq!(f.codomain(), 20);
    }

    #[test]
    fn composable_default_composable() {
        let f = TestMorphism {
            source: 1,
            target: 2,
        };
        let g = TestMorphism {
            source: 2,
            target: 3,
        };
        assert!(f.composable(&g).is_ok());

        let h = TestMorphism {
            source: 5,
            target: 6,
        };
        assert!(f.composable(&h).is_err());
    }

    // Test ComposableMutating with a simple implementation
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct MutableMorphism {
        source: i32,
        target: i32,
    }

    impl ComposableMutating<i32> for MutableMorphism {
        fn compose(&mut self, other: Self) -> Result<(), CatgraphError> {
            if self.target == other.source {
                self.target = other.target;
                Ok(())
            } else {
                Err(CatgraphError::Composition {
                    message: "Cannot compose".to_string(),
                })
            }
        }

        fn domain(&self) -> i32 {
            self.source
        }

        fn codomain(&self) -> i32 {
            self.target
        }
    }

    #[test]
    fn composable_mutating_compose() {
        let mut f = MutableMorphism {
            source: 1,
            target: 2,
        };
        let g = MutableMorphism {
            source: 2,
            target: 3,
        };
        let result = f.compose(g);
        assert!(result.is_ok());
        assert_eq!(f.source, 1);
        assert_eq!(f.target, 3);
    }

    #[test]
    fn composable_mutating_compose_fail() {
        let mut f = MutableMorphism {
            source: 1,
            target: 2,
        };
        let g = MutableMorphism {
            source: 5,
            target: 6,
        };
        let result = f.compose(g);
        assert!(result.is_err());
    }

    #[test]
    fn composable_mutating_default_composable() {
        let f = MutableMorphism {
            source: 1,
            target: 2,
        };
        let g = MutableMorphism {
            source: 2,
            target: 3,
        };
        assert!(f.composable(&g).is_ok());

        let h = MutableMorphism {
            source: 5,
            target: 6,
        };
        assert!(f.composable(&h).is_err());
    }
}
