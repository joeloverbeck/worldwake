use super::World;
use crate::{CommodityKind, EntityId, Quantity, UniqueItemKind, WorldError};
use std::collections::BTreeSet;

impl World {
    #[must_use]
    pub fn owner_of(&self, entity: EntityId) -> Option<EntityId> {
        let owner = self
            .is_alive(entity)
            .then(|| self.relations.owned_by.get(&entity).copied())
            .flatten()?;
        self.is_alive(owner).then_some(owner)
    }

    #[must_use]
    pub fn possessor_of(&self, entity: EntityId) -> Option<EntityId> {
        let holder = self
            .is_alive(entity)
            .then(|| self.relations.possessed_by.get(&entity).copied())
            .flatten()?;
        self.is_alive(holder).then_some(holder)
    }

    #[must_use]
    pub fn possessions_of(&self, holder: EntityId) -> Vec<EntityId> {
        if !self.is_alive(holder) {
            return Vec::new();
        }

        self.relations
            .possessions_of
            .get(&holder)
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
    pub fn controlled_commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
        let mut total = 0u32;
        let mut frontier = vec![holder];
        let mut visited = BTreeSet::new();

        while let Some(current) = frontier.pop() {
            if !visited.insert(current) || !self.is_alive(current) {
                continue;
            }

            if let Some(lot) = self.get_component_item_lot(current) {
                if lot.commodity == kind {
                    total = total
                        .checked_add(lot.quantity.0)
                        .expect("controlled commodity quantity overflowed");
                }
            }

            frontier.extend(self.direct_contents_of(current).into_iter().rev());
            frontier.extend(self.possessions_of(current).into_iter().rev());
        }

        Quantity(total)
    }

    #[must_use]
    pub fn controlled_unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        let mut total = 0u32;
        let mut frontier = vec![holder];
        let mut visited = BTreeSet::new();

        while let Some(current) = frontier.pop() {
            if !visited.insert(current) || !self.is_alive(current) {
                continue;
            }

            if self
                .get_component_unique_item(current)
                .is_some_and(|item| item.kind == kind)
            {
                total = total
                    .checked_add(1)
                    .expect("controlled unique item count overflowed");
            }

            frontier.extend(self.direct_contents_of(current).into_iter().rev());
            frontier.extend(self.possessions_of(current).into_iter().rev());
        }

        total
    }

    pub(crate) fn set_owner(
        &mut self,
        entity: EntityId,
        owner: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        self.ensure_alive(owner)?;
        Self::set_entity_relation(
            &mut self.relations.owned_by,
            &mut self.relations.property_of,
            entity,
            owner,
        );
        Ok(())
    }

    pub(crate) fn clear_owner(&mut self, entity: EntityId) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        Self::clear_entity_relation(
            &mut self.relations.owned_by,
            &mut self.relations.property_of,
            entity,
        );
        Ok(())
    }

    pub(crate) fn set_possessor(
        &mut self,
        entity: EntityId,
        holder: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        self.ensure_alive(holder)?;
        Self::set_entity_relation(
            &mut self.relations.possessed_by,
            &mut self.relations.possessions_of,
            entity,
            holder,
        );
        Ok(())
    }

    pub(crate) fn clear_possessor(&mut self, entity: EntityId) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        Self::clear_entity_relation(
            &mut self.relations.possessed_by,
            &mut self.relations.possessions_of,
            entity,
        );
        Ok(())
    }

    pub fn can_exercise_control(
        &self,
        actor: EntityId,
        entity: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(actor)?;
        self.ensure_alive(entity)?;

        if let Some(container) = self.direct_container(entity) {
            return self.can_exercise_control(actor, container);
        }

        if self.relations.possessed_by.get(&entity) == Some(&actor) {
            return Ok(());
        }

        if self.relations.owned_by.get(&entity) == Some(&actor)
            && !self.relations.possessed_by.contains_key(&entity)
        {
            return Ok(());
        }

        // Institutional delegation: faction membership or office holding
        if let Some(owner) = self.relations.owned_by.get(&entity).copied() {
            if !self.relations.possessed_by.contains_key(&entity) {
                if self.factions_of(actor).contains(&owner) {
                    return Ok(());
                }
                if self.offices_held_by(actor).contains(&owner) {
                    return Ok(());
                }
            }
        }

        if let Some(holder) = self.relations.possessed_by.get(&entity) {
            return Err(WorldError::PreconditionFailed(format!(
                "entity {entity} is possessed by {holder}, so {actor} cannot exercise control"
            )));
        }

        Err(WorldError::PreconditionFailed(format!(
            "entity {actor} neither possesses nor owns {entity}"
        )))
    }
}
