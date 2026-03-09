use super::World;
use crate::{EntityId, WorldError};

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

        if self.relations.possessed_by.get(&entity) == Some(&actor) {
            return Ok(());
        }

        if self.relations.owned_by.get(&entity) == Some(&actor)
            && !self.relations.possessed_by.contains_key(&entity)
        {
            return Ok(());
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
