use super::World;
use crate::{
    CommodityKind, ControlDeniedReason, EffectiveRight, EntityId, Quantity, RightKind,
    UniqueItemKind, WorldError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum ControlOutcome {
    Allowed(Vec<EffectiveRight>),
    BlockedByPossessor(EntityId),
    NoRights,
}

impl World {
    fn controlled_item_lots_for(
        &self,
        holder: EntityId,
    ) -> impl Iterator<Item = (EntityId, &crate::ItemLot)> + '_ {
        self.query_item_lot()
            .filter(move |(entity, _)| self.can_exercise_control(holder, *entity).is_ok())
    }

    fn controlled_unique_items_for(
        &self,
        holder: EntityId,
    ) -> impl Iterator<Item = (EntityId, &crate::UniqueItem)> + '_ {
        self.query_unique_item()
            .filter(move |(entity, _)| self.can_exercise_control(holder, *entity).is_ok())
    }

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
        self.controlled_item_lots_for(holder)
            .filter(|(_, lot)| lot.commodity == kind)
            .fold(Quantity(0), |total, (_, lot)| {
                Quantity(
                    total
                        .0
                        .checked_add(lot.quantity.0)
                        .expect("controlled commodity quantity overflowed"),
                )
            })
    }

    #[must_use]
    pub fn controlled_commodity_quantity_at_place(
        &self,
        holder: EntityId,
        place: EntityId,
        kind: CommodityKind,
    ) -> Quantity {
        self.controlled_item_lots_for(holder)
            .filter(|(entity, lot)| {
                lot.commodity == kind && self.effective_place(*entity) == Some(place)
            })
            .fold(Quantity(0), |total, (_, lot)| {
                Quantity(
                    total
                        .0
                        .checked_add(lot.quantity.0)
                        .expect("controlled commodity quantity at place overflowed"),
                )
            })
    }

    #[must_use]
    pub fn controlled_unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        self.controlled_unique_items_for(holder)
            .filter(|(_, item)| item.kind == kind)
            .fold(0u32, |total, _| {
                total
                    .checked_add(1)
                    .expect("controlled unique item count overflowed")
            })
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

        match self.collect_control_rights(actor, entity) {
            ControlOutcome::Allowed(_) => Ok(()),
            ControlOutcome::BlockedByPossessor(holder) => {
                Err(WorldError::ControlDenied {
                    actor,
                    entity,
                    reason: ControlDeniedReason::BlockedByPossessor(holder),
                })
            }
            ControlOutcome::NoRights => Err(WorldError::ControlDenied {
                actor,
                entity,
                reason: ControlDeniedReason::NoRights,
            }),
        }
    }

    #[must_use]
    pub fn effective_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
        if self.ensure_alive(actor).is_err() || self.ensure_alive(entity).is_err() {
            return Vec::new();
        }

        let mut rights = match self.collect_control_rights(actor, entity) {
            ControlOutcome::Allowed(rights) => rights,
            ControlOutcome::BlockedByPossessor(_) | ControlOutcome::NoRights => Vec::new(),
        };
        if let Some(jurisdictional_right) = self.jurisdictional_right(actor, entity) {
            rights.push(jurisdictional_right);
        }
        rights
    }

    #[must_use]
    pub fn has_right(&self, actor: EntityId, entity: EntityId, kind: RightKind) -> bool {
        self.effective_rights(actor, entity)
            .iter()
            .any(|right| right.kind == kind)
    }

    fn collect_control_rights(&self, actor: EntityId, entity: EntityId) -> ControlOutcome {
        if let Some(container) = self.direct_container(entity) {
            return match self.collect_control_rights(actor, container) {
                ControlOutcome::Allowed(_) => ControlOutcome::Allowed(vec![EffectiveRight {
                    kind: RightKind::ContainerAccess,
                    via: Some(container),
                }]),
                blocked => blocked,
            };
        }

        let mut rights = Vec::new();

        if self.relations.possessed_by.get(&entity) == Some(&actor) {
            rights.push(EffectiveRight {
                kind: RightKind::PhysicalPossession,
                via: None,
            });
        }

        let unpossessed = !self.relations.possessed_by.contains_key(&entity);
        if self.relations.owned_by.get(&entity) == Some(&actor) && unpossessed {
            rights.push(EffectiveRight {
                kind: RightKind::Ownership,
                via: None,
            });
        }

        if let Some(owner) = self.relations.owned_by.get(&entity).copied()
            && unpossessed
        {
            if self.factions_of(actor).contains(&owner) {
                rights.push(EffectiveRight {
                    kind: RightKind::FactionAuthority,
                    via: Some(owner),
                });
            }
            if self.offices_held_by(actor).contains(&owner) {
                rights.push(EffectiveRight {
                    kind: RightKind::OfficeAuthority,
                    via: Some(owner),
                });
            }
        }

        if !rights.is_empty() {
            return ControlOutcome::Allowed(rights);
        }

        if let Some(holder) = self.relations.possessed_by.get(&entity).copied() {
            return ControlOutcome::BlockedByPossessor(holder);
        }

        ControlOutcome::NoRights
    }

    fn jurisdictional_right(&self, actor: EntityId, entity: EntityId) -> Option<EffectiveRight> {
        let entity_place = self.effective_place(entity)?;
        self.offices_held_by(actor)
            .into_iter()
            .find(|office| {
                self.get_component_office_data(*office)
                    .is_some_and(|office_data| office_data.jurisdiction.contains(&entity_place))
            })
            .map(|office| EffectiveRight {
                kind: RightKind::JurisdictionalAuthority,
                via: Some(office),
            })
    }
}
