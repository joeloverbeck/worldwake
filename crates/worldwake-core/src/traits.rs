//! Core marker traits for authoritative component and relation types.
//!
//! These traits enforce the serialization and determinism bounds that all
//! authoritative state must satisfy. No runtime type registration or
//! type-erased storage is introduced here.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Marker trait for authoritative ECS component types.
///
/// Required bounds: `'static + Send + Sync + Clone + Debug + Serialize +
/// DeserializeOwned`.
pub trait Component: 'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned {}

/// Marker trait for authoritative relation rows.
///
/// Same bounds as [`Component`].
pub trait RelationRecord:
    'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned
{
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test struct that implements `Component`.
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    struct TestComponent {
        value: u32,
    }
    impl Component for TestComponent {}

    /// A test struct that implements `RelationRecord`.
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    struct TestRelation {
        source: u32,
        target: u32,
    }
    impl RelationRecord for TestRelation {}

    fn assert_component_bounds<T: Component>() {}
    fn assert_relation_bounds<T: RelationRecord>() {}

    #[test]
    fn test_component_implementor() {
        assert_component_bounds::<TestComponent>();
    }

    #[test]
    fn test_relation_implementor() {
        assert_relation_bounds::<TestRelation>();
    }

    /// Verify Component implementor satisfies all required bounds individually.
    fn assert_all_bounds<
        T: 'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned,
    >() {
    }

    #[test]
    fn component_satisfies_all_required_bounds() {
        assert_all_bounds::<TestComponent>();
    }

    #[test]
    fn relation_satisfies_all_required_bounds() {
        assert_all_bounds::<TestRelation>();
    }
}
