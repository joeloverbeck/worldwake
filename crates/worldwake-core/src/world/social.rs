use super::World;
use crate::{EntityId, EntityKind, FactId, Permille, WorldError};

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
    pub fn office_holder(&self, office: EntityId) -> Option<EntityId> {
        self.ensure_live_kind(office, EntityKind::Office, "office query target")
            .ok()?;
        let holder = self.relations.office_holder.get(&office).copied()?;
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

    pub(crate) fn add_known_fact(
        &mut self,
        agent: EntityId,
        fact: FactId,
    ) -> Result<(), WorldError> {
        self.ensure_live_kind(agent, EntityKind::Agent, "knowledge subject")?;
        self.relations
            .knows_fact
            .entry(agent)
            .or_default()
            .insert(fact);
        Ok(())
    }

    pub(crate) fn remove_known_fact(
        &mut self,
        agent: EntityId,
        fact: FactId,
    ) -> Result<(), WorldError> {
        self.ensure_live_kind(agent, EntityKind::Agent, "knowledge subject")?;
        Self::clear_fact_relation(&mut self.relations.knows_fact, agent, fact);
        Ok(())
    }

    #[must_use]
    pub fn known_facts(&self, agent: EntityId) -> Vec<FactId> {
        if self
            .ensure_live_kind(agent, EntityKind::Agent, "knowledge subject")
            .is_err()
        {
            return Vec::new();
        }

        self.relations
            .knows_fact
            .get(&agent)
            .map(|facts| facts.iter().copied().collect())
            .unwrap_or_default()
    }

    pub(crate) fn add_believed_fact(
        &mut self,
        agent: EntityId,
        fact: FactId,
    ) -> Result<(), WorldError> {
        self.ensure_live_kind(agent, EntityKind::Agent, "belief subject")?;
        self.relations
            .believes_fact
            .entry(agent)
            .or_default()
            .insert(fact);
        Ok(())
    }

    pub(crate) fn remove_believed_fact(
        &mut self,
        agent: EntityId,
        fact: FactId,
    ) -> Result<(), WorldError> {
        self.ensure_live_kind(agent, EntityKind::Agent, "belief subject")?;
        Self::clear_fact_relation(&mut self.relations.believes_fact, agent, fact);
        Ok(())
    }

    #[must_use]
    pub fn believed_facts(&self, agent: EntityId) -> Vec<FactId> {
        if self
            .ensure_live_kind(agent, EntityKind::Agent, "belief subject")
            .is_err()
        {
            return Vec::new();
        }

        self.relations
            .believes_fact
            .get(&agent)
            .map(|facts| facts.iter().copied().collect())
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

    fn clear_fact_relation(
        relations: &mut std::collections::BTreeMap<EntityId, std::collections::BTreeSet<FactId>>,
        agent: EntityId,
        fact: FactId,
    ) {
        if let Some(facts) = relations.get_mut(&agent) {
            facts.remove(&fact);
            if facts.is_empty() {
                relations.remove(&agent);
            }
        }
    }
}
