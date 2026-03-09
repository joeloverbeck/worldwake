use super::World;
use crate::{EntityId, Permille};
use std::collections::{BTreeMap, BTreeSet};

impl World {
    pub(super) fn set_many_to_many_relation(
        forward: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
        target: EntityId,
    ) {
        forward.entry(entity).or_default().insert(target);
        reverse.entry(target).or_default().insert(entity);
    }

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

    pub(super) fn set_weighted_relation(
        forward: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        reverse: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        entity: EntityId,
        target: EntityId,
        strength: Permille,
    ) {
        forward.entry(entity).or_default().insert(target, strength);
        reverse.entry(target).or_default().insert(entity, strength);
    }

    pub(super) fn clear_weighted_relation(
        forward: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        reverse: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        entity: EntityId,
        target: EntityId,
    ) {
        if let Some(targets) = forward.get_mut(&entity) {
            targets.remove(&target);
            if targets.is_empty() {
                forward.remove(&entity);
            }
        }

        if let Some(entities) = reverse.get_mut(&target) {
            entities.remove(&entity);
            if entities.is_empty() {
                reverse.remove(&target);
            }
        }
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
