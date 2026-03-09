use super::World;
use crate::EntityId;
use std::collections::{BTreeMap, BTreeSet};

impl World {
    pub(super) fn set_entity_relation(
        forward: &mut BTreeMap<EntityId, EntityId>,
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
        target: EntityId,
    ) {
        if let Some(previous) = forward.insert(entity, target) {
            if previous != target {
                Self::remove_reverse_link(reverse, previous, entity);
            }
        }
        reverse.entry(target).or_default().insert(entity);
    }

    pub(super) fn clear_entity_relation(
        forward: &mut BTreeMap<EntityId, EntityId>,
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
    ) {
        if let Some(target) = forward.remove(&entity) {
            Self::remove_reverse_link(reverse, target, entity);
        }
    }

    pub(super) fn clear_many_to_many_relation(
        forward: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
        target: EntityId,
    ) {
        if let Some(targets) = forward.get_mut(&entity) {
            targets.remove(&target);
            if targets.is_empty() {
                forward.remove(&entity);
            }
        }
        Self::remove_reverse_link(reverse, target, entity);
    }

    fn remove_reverse_link(
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        target: EntityId,
        entity: EntityId,
    ) {
        if let Some(entities) = reverse.get_mut(&target) {
            entities.remove(&entity);
            if entities.is_empty() {
                reverse.remove(&target);
            }
        }
    }
}
