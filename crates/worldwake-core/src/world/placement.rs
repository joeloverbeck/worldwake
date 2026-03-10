use super::World;
use crate::{load_of_entity, remaining_container_capacity, Container, EntityId, WorldError};
use std::collections::BTreeSet;

impl World {
    #[must_use]
    pub fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        self.is_alive(entity)
            .then(|| self.relations.located_in.get(&entity).copied())
            .flatten()
    }

    #[must_use]
    pub fn is_in_transit(&self, entity: EntityId) -> bool {
        self.is_alive(entity) && self.relations.in_transit.contains(&entity)
    }

    #[must_use]
    pub fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
        let container = self
            .is_alive(entity)
            .then(|| self.relations.contained_by.get(&entity).copied())
            .flatten()?;
        self.is_alive(container).then_some(container)
    }

    #[must_use]
    pub fn direct_contents_of(&self, container: EntityId) -> Vec<EntityId> {
        if !self.is_alive(container) {
            return Vec::new();
        }

        self.relations
            .contents_of
            .get(&container)
            .map(|contents| {
                contents
                    .iter()
                    .copied()
                    .filter(|entity| self.is_alive(*entity))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn recursive_contents_of(&self, container: EntityId) -> Vec<EntityId> {
        if !self.is_alive(container) {
            return Vec::new();
        }

        let mut descendants = Vec::new();
        let mut frontier = self
            .relations
            .contents_of
            .get(&container)
            .map(|contents| contents.iter().rev().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut visited = BTreeSet::new();

        while let Some(current) = frontier.pop() {
            if !visited.insert(current) {
                continue;
            }

            if self.is_alive(current) {
                descendants.push(current);
            }

            if let Some(children) = self.relations.contents_of.get(&current) {
                frontier.extend(children.iter().rev().copied());
            }
        }

        descendants
    }

    #[must_use]
    pub fn entities_effectively_at(&self, place: EntityId) -> Vec<EntityId> {
        self.relations
            .entities_at
            .get(&place)
            .map(|entities| {
                entities
                    .iter()
                    .copied()
                    .filter(|entity| self.is_alive(*entity))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn ground_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.entities_effectively_at(place)
            .into_iter()
            .filter(|entity| !self.relations.contained_by.contains_key(entity))
            .collect()
    }

    pub(crate) fn set_ground_location(
        &mut self,
        entity: EntityId,
        place: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        self.require_place(place)?;
        self.clear_contained_by(entity);
        self.set_effective_place(entity, place)
    }

    pub(crate) fn put_into_container(
        &mut self,
        entity: EntityId,
        container: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        self.require_live_container(container)?;
        self.ensure_no_containment_cycle(entity, container)?;
        self.validate_container_admission(entity, container)?;

        let effective_place = self.effective_place_from_container(container)?;
        self.clear_contained_by(entity);
        self.set_contained_by(entity, container);
        self.set_effective_place(entity, effective_place)
    }

    pub(crate) fn remove_from_container(&mut self, entity: EntityId) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        if !self.relations.contained_by.contains_key(&entity) {
            return Err(WorldError::PreconditionFailed(format!(
                "entity {entity} is not currently contained"
            )));
        }

        self.clear_contained_by(entity);
        Ok(())
    }

    pub(crate) fn move_container_subtree(
        &mut self,
        container: EntityId,
        new_place: EntityId,
    ) -> Result<(), WorldError> {
        self.require_live_container(container)?;
        self.set_ground_location(container, new_place)
    }

    pub(crate) fn set_in_transit(&mut self, entity: EntityId) -> Result<(), WorldError> {
        let meta = self.ensure_alive(entity)?;
        if !Self::requires_physical_placement(meta.kind) {
            return Err(WorldError::InvalidOperation(format!(
                "entity kind {:?} does not support physical transit placement: {}",
                meta.kind, entity
            )));
        }

        self.clear_located_in(entity);
        self.relations.in_transit.insert(entity);
        if self.get_component_container(entity).is_some() {
            let descendants = self.collect_container_descendants(entity)?;
            for descendant in descendants {
                self.clear_located_in(descendant);
                self.relations.in_transit.insert(descendant);
            }
        }

        Ok(())
    }

    fn require_place(&self, place: EntityId) -> Result<(), WorldError> {
        if self.topology.place(place).is_some() {
            return Ok(());
        }

        Err(WorldError::InvalidOperation(format!(
            "entity {place} is not a topology place"
        )))
    }

    fn require_live_container(&self, entity: EntityId) -> Result<&Container, WorldError> {
        self.ensure_alive(entity)?;
        self.get_component_container(entity)
            .ok_or(WorldError::ComponentNotFound {
                entity,
                component_type: "Container",
            })
    }

    fn validate_container_admission(
        &self,
        entity: EntityId,
        container: EntityId,
    ) -> Result<(), WorldError> {
        let container_component = self.require_live_container(container)?;

        if let Some(lot) = self.get_component_item_lot(entity) {
            if let Some(allowed) = &container_component.allowed_commodities {
                if !allowed.contains(&lot.commodity) {
                    return Err(WorldError::InvalidOperation(format!(
                        "container {container} does not allow commodity {:?} for {entity}",
                        lot.commodity
                    )));
                }
            }
        }

        if self.get_component_unique_item(entity).is_some()
            && !container_component.allows_unique_items
        {
            return Err(WorldError::InvalidOperation(format!(
                "container {container} does not allow unique items like {entity}"
            )));
        }

        if self.get_component_container(entity).is_some()
            && !container_component.allows_nested_containers
        {
            return Err(WorldError::InvalidOperation(format!(
                "container {container} does not allow nested containers like {entity}"
            )));
        }

        let requested = load_of_entity(self, entity)?;
        let existing_contents = self
            .relations
            .contents_of
            .get(&container)
            .map(|contents| {
                contents
                    .iter()
                    .copied()
                    .filter(|existing| *existing != entity)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let remaining = remaining_container_capacity(self, container, existing_contents)?;
        if requested.0 > remaining.0 {
            return Err(WorldError::CapacityExceeded {
                container,
                requested: requested.0,
                remaining: remaining.0,
            });
        }

        Ok(())
    }

    fn ensure_no_containment_cycle(
        &self,
        entity: EntityId,
        container: EntityId,
    ) -> Result<(), WorldError> {
        let mut current = container;
        let mut visited = BTreeSet::new();

        loop {
            if current == entity {
                return Err(WorldError::ContainmentCycle { entity, container });
            }
            if !visited.insert(current) {
                return Err(WorldError::InvariantViolation(format!(
                    "containment chain for {container} is cyclic"
                )));
            }

            let Some(parent) = self.relations.contained_by.get(&current).copied() else {
                return Ok(());
            };
            current = parent;
        }
    }

    fn effective_place_from_container(&self, container: EntityId) -> Result<EntityId, WorldError> {
        let mut current = container;
        let mut visited = BTreeSet::new();

        loop {
            if !visited.insert(current) {
                return Err(WorldError::InvariantViolation(format!(
                    "effective place lookup for {container} encountered a containment cycle"
                )));
            }

            if let Some(place) = self.relations.located_in.get(&current).copied() {
                return Ok(place);
            }

            let Some(parent) = self.relations.contained_by.get(&current).copied() else {
                return Err(WorldError::PreconditionFailed(format!(
                    "container {container} has no effective place"
                )));
            };
            current = parent;
        }
    }

    fn set_effective_place(&mut self, entity: EntityId, place: EntityId) -> Result<(), WorldError> {
        self.relations.in_transit.remove(&entity);
        self.set_located_in(entity, place);
        if self.get_component_container(entity).is_some() {
            let descendants = self.collect_container_descendants(entity)?;
            for descendant in descendants {
                self.relations.in_transit.remove(&descendant);
                self.set_located_in(descendant, place);
            }
        }

        Ok(())
    }

    fn collect_container_descendants(
        &self,
        container: EntityId,
    ) -> Result<Vec<EntityId>, WorldError> {
        let mut descendants = Vec::new();
        let mut frontier = self
            .relations
            .contents_of
            .get(&container)
            .map(|contents| contents.iter().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut visited = BTreeSet::new();

        while let Some(current) = frontier.pop() {
            if !visited.insert(current) {
                return Err(WorldError::InvariantViolation(format!(
                    "container subtree rooted at {container} is cyclic"
                )));
            }

            descendants.push(current);
            if let Some(children) = self.relations.contents_of.get(&current) {
                frontier.extend(children.iter().rev().copied());
            }
        }

        Ok(descendants)
    }

    fn set_located_in(&mut self, entity: EntityId, place: EntityId) {
        Self::set_entity_relation(
            &mut self.relations.located_in,
            &mut self.relations.entities_at,
            entity,
            place,
        );
    }

    fn set_contained_by(&mut self, entity: EntityId, container: EntityId) {
        Self::set_entity_relation(
            &mut self.relations.contained_by,
            &mut self.relations.contents_of,
            entity,
            container,
        );
    }

    fn clear_contained_by(&mut self, entity: EntityId) {
        Self::clear_entity_relation(
            &mut self.relations.contained_by,
            &mut self.relations.contents_of,
            entity,
        );
    }

    fn clear_located_in(&mut self, entity: EntityId) {
        Self::clear_entity_relation(
            &mut self.relations.located_in,
            &mut self.relations.entities_at,
            entity,
        );
    }
}
