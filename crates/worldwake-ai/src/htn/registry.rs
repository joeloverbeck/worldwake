use crate::htn::{MethodSchema, methods};
use std::collections::BTreeMap;
use worldwake_core::{GoalKindDiscriminant, MethodSchemaId};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MethodRegistry {
    methods: BTreeMap<MethodSchemaId, MethodSchema>,
    by_goal_kind: BTreeMap<GoalKindDiscriminant, Vec<MethodSchemaId>>,
}

impl MethodRegistry {
    pub fn insert(&mut self, schema: MethodSchema) {
        let id = schema.id;
        let goal_kind = schema.goal_kind;
        assert!(
            self.methods.insert(id, schema).is_none(),
            "duplicate MethodSchemaId {id:?}"
        );
        self.by_goal_kind.entry(goal_kind).or_default().push(id);
    }

    #[must_use]
    pub fn get(&self, id: MethodSchemaId) -> Option<&MethodSchema> {
        self.methods.get(&id)
    }

    #[must_use]
    pub fn methods_for(&self, goal_kind: GoalKindDiscriminant) -> &[MethodSchemaId] {
        self.by_goal_kind.get(&goal_kind).map_or(&[], Vec::as_slice)
    }

    pub fn all_method_ids(&self) -> impl Iterator<Item = MethodSchemaId> + '_ {
        self.methods.keys().copied()
    }

    pub fn all_methods(&self) -> impl Iterator<Item = &MethodSchema> + '_ {
        self.methods.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.methods.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }
}

#[must_use]
pub fn build_method_registry() -> MethodRegistry {
    let mut registry = MethodRegistry::default();
    registry.insert(methods::fulfill_bounty_direct());
    registry.insert(methods::fulfill_bounty_investigation());
    registry.insert(methods::fulfill_bounty_group_hunt());
    registry.insert(methods::produce_from_owned_stock());
    registry.insert(methods::produce_with_gather());
    registry.insert(methods::produce_with_purchase());
    registry.insert(methods::restock_from_harvest());
    registry.insert(methods::restock_from_market());
    registry.insert(methods::investigate_on_scene());
    registry.insert(methods::investigate_by_witness());
    registry.insert(methods::investigate_by_ledger());
    registry.insert(methods::escort_to_home());
    registry.insert(methods::escort_to_office());
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_builds_with_13_methods() {
        let registry = build_method_registry();

        assert_eq!(registry.len(), 13);
        assert!(!registry.is_empty());
    }

    #[test]
    fn methods_for_returns_correct_ids() {
        let registry = build_method_registry();

        assert_eq!(
            registry.methods_for(GoalKindDiscriminant::FulfillBounty),
            &[MethodSchemaId(1), MethodSchemaId(2), MethodSchemaId(3)]
        );
    }

    #[test]
    fn empty_registry_returns_empty_methods_for_slice() {
        let registry = MethodRegistry::default();

        assert!(
            registry
                .methods_for(GoalKindDiscriminant::FulfillBounty)
                .is_empty()
        );
    }
}
