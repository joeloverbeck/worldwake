use super::World;
use crate::{EntityId, EntityKind, Permille, WorldError};

impl World {
    pub(crate) fn add_member(
        &mut self,
        member: EntityId,
        faction: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(member)?;
        self.ensure_live_kind(faction, EntityKind::Faction, "member target")?;
        Self::set_many_to_many_relation(
            &mut self.relations.member_of,
            &mut self.relations.members_of,
            member,
            faction,
        );
        Ok(())
    }

    pub(crate) fn remove_member(
        &mut self,
        member: EntityId,
        faction: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(member)?;
        self.ensure_live_kind(faction, EntityKind::Faction, "member target")?;
        Self::clear_many_to_many_relation(
            &mut self.relations.member_of,
            &mut self.relations.members_of,
            member,
            faction,
        );
        Ok(())
    }

    #[must_use]
    pub fn members_of(&self, faction: EntityId) -> Vec<EntityId> {
        if self
            .ensure_live_kind(faction, EntityKind::Faction, "member target")
            .is_err()
        {
            return Vec::new();
        }

        self.relations
            .members_of
            .get(&faction)
            .map(|members| {
                members
                    .iter()
                    .copied()
                    .filter(|member| self.is_alive(*member))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn factions_of(&self, member: EntityId) -> Vec<EntityId> {
        if !self.is_alive(member) {
            return Vec::new();
        }

        self.relations
            .member_of
            .get(&member)
            .map(|factions| {
                factions
                    .iter()
                    .copied()
                    .filter(|faction| self.entity_kind(*faction) == Some(EntityKind::Faction))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Sets the internal loyalty relation row from `subject` to `target`.
    ///
    /// This is a low-level world mutation helper over the canonical weighted relation tables.
    /// Event-sourced simulation mutations should go through `WorldTxn::set_loyalty`, and loyalty
    /// strength must remain input state for agent reasoning rather than a scripted threshold.
    pub(crate) fn set_loyalty(
        &mut self,
        subject: EntityId,
        target: EntityId,
        strength: Permille,
    ) -> Result<(), WorldError> {
        self.ensure_alive(subject)?;
        self.ensure_alive(target)?;
        Self::set_weighted_relation(
            &mut self.relations.loyal_to,
            &mut self.relations.loyalty_from,
            subject,
            target,
            strength,
        );
        Ok(())
    }

    /// Clears the internal loyalty relation row from `subject` to `target`.
    ///
    /// This preserves the same low-level weighted relation semantics as `set_loyalty`.
    /// Event-sourced simulation mutations should go through `WorldTxn::clear_loyalty` so the
    /// append-only event log records the canonical removal delta.
    pub(crate) fn clear_loyalty(
        &mut self,
        subject: EntityId,
        target: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(subject)?;
        self.ensure_alive(target)?;
        Self::clear_weighted_relation(
            &mut self.relations.loyal_to,
            &mut self.relations.loyalty_from,
            subject,
            target,
        );
        Ok(())
    }

    #[must_use]
    pub fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<Permille> {
        if !self.is_alive(subject) || !self.is_alive(target) {
            return None;
        }

        self.relations
            .loyal_to
            .get(&subject)
            .and_then(|targets| targets.get(&target).copied())
    }

    #[must_use]
    pub fn loyal_targets_of(&self, subject: EntityId) -> Vec<(EntityId, Permille)> {
        if !self.is_alive(subject) {
            return Vec::new();
        }

        self.relations
            .loyal_to
            .get(&subject)
            .map(|targets| {
                targets
                    .iter()
                    .filter_map(|(target, strength)| {
                        self.is_alive(*target).then_some((*target, *strength))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn loyal_subjects_of(&self, target: EntityId) -> Vec<(EntityId, Permille)> {
        if !self.is_alive(target) {
            return Vec::new();
        }

        self.relations
            .loyalty_from
            .get(&target)
            .map(|subjects| {
                subjects
                    .iter()
                    .filter_map(|(subject, strength)| {
                        self.is_alive(*subject).then_some((*subject, *strength))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn assign_office(
        &mut self,
        office: EntityId,
        holder: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_live_kind(office, EntityKind::Office, "office assignment target")?;
        self.ensure_alive(holder)?;
        Self::set_entity_relation(
            &mut self.relations.office_holder,
            &mut self.relations.offices_held,
            office,
            holder,
        );
        Ok(())
    }

    pub(crate) fn vacate_office(&mut self, office: EntityId) -> Result<(), WorldError> {
        self.ensure_live_kind(office, EntityKind::Office, "office vacancy target")?;
        self.clear_office_assignment(office);
        Ok(())
    }

    #[must_use]
    pub(crate) fn authoritative_office_holder(&self, office: EntityId) -> Option<EntityId> {
        self.ensure_live_kind(office, EntityKind::Office, "office query target")
            .ok()?;
        self.relations.office_holder.get(&office).copied()
    }

    #[must_use]
    pub fn office_holder(&self, office: EntityId) -> Option<EntityId> {
        let holder = self.authoritative_office_holder(office)?;
        self.is_alive(holder).then_some(holder)
    }

    #[must_use]
    pub fn offices_held_by(&self, holder: EntityId) -> Vec<EntityId> {
        if !self.is_alive(holder) {
            return Vec::new();
        }

        self.relations
            .offices_held
            .get(&holder)
            .map(|offices| {
                offices
                    .iter()
                    .copied()
                    .filter(|office| self.entity_kind(*office) == Some(EntityKind::Office))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn declare_support(
        &mut self,
        supporter: EntityId,
        office: EntityId,
        candidate: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(supporter)?;
        self.ensure_live_kind(office, EntityKind::Office, "support declaration office")?;
        self.ensure_alive(candidate)?;
        self.relations
            .support_declarations
            .insert((supporter, office), candidate);
        Ok(())
    }

    pub(crate) fn clear_support_declaration(
        &mut self,
        supporter: EntityId,
        office: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(supporter)?;
        self.ensure_live_kind(office, EntityKind::Office, "support declaration office")?;
        self.relations
            .support_declarations
            .remove(&(supporter, office));
        Ok(())
    }

    pub(crate) fn clear_support_declarations_for_office(
        &mut self,
        office: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_live_kind(office, EntityKind::Office, "support declaration office")?;
        self.relations
            .support_declarations
            .retain(|(_, declared_office), _| *declared_office != office);
        Ok(())
    }

    #[must_use]
    pub fn support_declaration(&self, supporter: EntityId, office: EntityId) -> Option<EntityId> {
        if !self.is_alive(supporter)
            || self
                .ensure_live_kind(office, EntityKind::Office, "support declaration office")
                .is_err()
        {
            return None;
        }

        let candidate = self
            .relations
            .support_declarations
            .get(&(supporter, office))
            .copied()?;
        self.is_alive(candidate).then_some(candidate)
    }

    #[must_use]
    pub fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
        if self
            .ensure_live_kind(office, EntityKind::Office, "support declaration office")
            .is_err()
        {
            return Vec::new();
        }

        self.relations
            .support_declarations
            .iter()
            .filter_map(|((supporter, declared_office), candidate)| {
                (*declared_office == office
                    && self.is_alive(*supporter)
                    && self.is_alive(*candidate))
                .then_some((*supporter, *candidate))
            })
            .collect()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn support_declarations_made_by(
        &self,
        supporter: EntityId,
    ) -> Vec<(EntityId, EntityId)> {
        if !self.is_alive(supporter) {
            return Vec::new();
        }

        self.relations
            .support_declarations
            .iter()
            .filter_map(|((declared_supporter, office), candidate)| {
                (*declared_supporter == supporter
                    && self.entity_kind(*office) == Some(EntityKind::Office)
                    && self.is_alive(*candidate))
                .then_some((*office, *candidate))
            })
            .collect()
    }

    #[must_use]
    pub fn count_support_declarations_for_candidate(
        &self,
        office: EntityId,
        candidate: EntityId,
    ) -> usize {
        if !self.is_alive(candidate) {
            return 0;
        }

        self.support_declarations_for_office(office)
            .into_iter()
            .filter(|(_, declared_candidate)| *declared_candidate == candidate)
            .count()
    }

    pub(crate) fn add_hostility(
        &mut self,
        subject: EntityId,
        target: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(subject)?;
        self.ensure_alive(target)?;
        Self::set_many_to_many_relation(
            &mut self.relations.hostile_to,
            &mut self.relations.hostility_from,
            subject,
            target,
        );
        Ok(())
    }

    pub(crate) fn remove_hostility(
        &mut self,
        subject: EntityId,
        target: EntityId,
    ) -> Result<(), WorldError> {
        self.ensure_alive(subject)?;
        self.ensure_alive(target)?;
        Self::clear_many_to_many_relation(
            &mut self.relations.hostile_to,
            &mut self.relations.hostility_from,
            subject,
            target,
        );
        Ok(())
    }

    #[must_use]
    pub fn hostile_targets_of(&self, subject: EntityId) -> Vec<EntityId> {
        if !self.is_alive(subject) {
            return Vec::new();
        }

        self.relations
            .hostile_to
            .get(&subject)
            .map(|targets| {
                targets
                    .iter()
                    .copied()
                    .filter(|target| self.is_alive(*target))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn hostile_towards(&self, target: EntityId) -> Vec<EntityId> {
        if !self.is_alive(target) {
            return Vec::new();
        }

        self.relations
            .hostility_from
            .get(&target)
            .map(|subjects| {
                subjects
                    .iter()
                    .copied()
                    .filter(|subject| self.is_alive(*subject))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn ensure_live_kind(
        &self,
        entity: EntityId,
        expected: EntityKind,
        context: &str,
    ) -> Result<(), WorldError> {
        let meta = self.ensure_alive(entity)?;
        if meta.kind == expected {
            return Ok(());
        }

        Err(WorldError::InvalidOperation(format!(
            "{context} must be a {:?}, but {entity} is a {:?}",
            expected, meta.kind
        )))
    }
}
