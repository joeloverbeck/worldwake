use super::World;
use crate::{
    ArchiveDependency, ArchiveDependencyKind, EntityId, EntityKind, FactId, Permille,
    ReservationRecord, WorldError,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveResolution {
    DetachContentsToGround,
    SpillContentsRecursively,
    DropPossessions,
    TransferPossessionsTo(EntityId),
    RelinquishOwnership,
    TransferOwnershipTo(EntityId),
    RevokeMemberships,
    RevokeLoyalty,
    RevokeHostility,
    VacateOffice,
    RelinquishOffices,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePreparationPolicy {
    resolutions: BTreeMap<ArchiveDependencyKind, ArchiveResolution>,
}

impl ArchivePreparationPolicy {
    #[must_use]
    pub fn all() -> Self {
        Self {
            resolutions: BTreeMap::from([
                (
                    ArchiveDependencyKind::ContainsEntities,
                    ArchiveResolution::DetachContentsToGround,
                ),
                (
                    ArchiveDependencyKind::PossessesEntities,
                    ArchiveResolution::DropPossessions,
                ),
                (
                    ArchiveDependencyKind::OwnsEntities,
                    ArchiveResolution::RelinquishOwnership,
                ),
                (
                    ArchiveDependencyKind::HasMembers,
                    ArchiveResolution::RevokeMemberships,
                ),
                (
                    ArchiveDependencyKind::HasLoyalSubjects,
                    ArchiveResolution::RevokeLoyalty,
                ),
                (
                    ArchiveDependencyKind::HasHostileSubjects,
                    ArchiveResolution::RevokeHostility,
                ),
                (
                    ArchiveDependencyKind::HasOfficeHolder,
                    ArchiveResolution::VacateOffice,
                ),
                (
                    ArchiveDependencyKind::HoldsOffices,
                    ArchiveResolution::RelinquishOffices,
                ),
            ]),
        }
    }

    #[must_use]
    pub fn none() -> Self {
        Self {
            resolutions: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_resolutions<I>(resolutions: I) -> Self
    where
        I: IntoIterator<Item = (ArchiveDependencyKind, ArchiveResolution)>,
    {
        Self {
            resolutions: resolutions.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn with_resolution(
        mut self,
        kind: ArchiveDependencyKind,
        resolution: ArchiveResolution,
    ) -> Self {
        self.resolutions.insert(kind, resolution);
        self
    }

    #[must_use]
    pub fn resolution_for(&self, kind: ArchiveDependencyKind) -> Option<ArchiveResolution> {
        self.resolutions.get(&kind).copied()
    }
}

impl Default for ArchivePreparationPolicy {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePreparationReport {
    pub applied: Vec<ArchivePreparationAction>,
    pub blocked: Vec<ArchiveDependency>,
}

impl ArchivePreparationReport {
    #[must_use]
    pub fn is_ready_for_archive(&self) -> bool {
        self.blocked.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePreparationAction {
    pub dependency: ArchiveDependency,
    pub resolution: ArchiveResolution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivePreparationPlan {
    pub actions: Vec<ArchivePreparationAction>,
    pub blocked: Vec<ArchiveDependency>,
}

impl ArchivePreparationPlan {
    #[must_use]
    pub fn is_ready_for_archive(&self) -> bool {
        self.blocked.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveMutationSnapshot {
    pub entity: EntityId,
    pub kind: EntityKind,
    pub located_in: Option<EntityId>,
    pub in_transit: bool,
    pub contained_by: Option<EntityId>,
    pub contents_of: Vec<EntityId>,
    pub possessed_by: Option<EntityId>,
    pub possessions_of: Vec<EntityId>,
    pub owned_by: Option<EntityId>,
    pub property_of: Vec<EntityId>,
    pub member_of: Vec<EntityId>,
    pub members_of: Vec<EntityId>,
    pub loyal_to: Vec<(EntityId, Permille)>,
    pub loyalty_from: Vec<(EntityId, Permille)>,
    pub office_holder: Option<EntityId>,
    pub offices_held: Vec<EntityId>,
    pub hostile_to: Vec<EntityId>,
    pub hostility_from: Vec<EntityId>,
    pub known_facts: Vec<FactId>,
    pub believed_facts: Vec<FactId>,
    pub released_reservations: Vec<ReservationRecord>,
}

impl World {
    pub(crate) fn archive_mutation_snapshot(
        &self,
        entity: EntityId,
    ) -> Result<ArchiveMutationSnapshot, WorldError> {
        let kind = self.ensure_alive(entity)?.kind;

        Ok(ArchiveMutationSnapshot {
            entity,
            kind,
            located_in: self.relations.located_in.get(&entity).copied(),
            in_transit: self.relations.in_transit.contains(&entity),
            contained_by: self.relations.contained_by.get(&entity).copied(),
            contents_of: Self::snapshot_entities(&self.relations.contents_of, entity),
            possessed_by: self.relations.possessed_by.get(&entity).copied(),
            possessions_of: Self::snapshot_entities(&self.relations.possessions_of, entity),
            owned_by: self.relations.owned_by.get(&entity).copied(),
            property_of: Self::snapshot_entities(&self.relations.property_of, entity),
            member_of: Self::snapshot_entities(&self.relations.member_of, entity),
            members_of: Self::snapshot_entities(&self.relations.members_of, entity),
            loyal_to: Self::snapshot_weighted_entities(&self.relations.loyal_to, entity),
            loyalty_from: Self::snapshot_weighted_entities(&self.relations.loyalty_from, entity),
            office_holder: self.relations.office_holder.get(&entity).copied(),
            offices_held: Self::snapshot_entities(&self.relations.offices_held, entity),
            hostile_to: Self::snapshot_entities(&self.relations.hostile_to, entity),
            hostility_from: Self::snapshot_entities(&self.relations.hostility_from, entity),
            known_facts: Self::snapshot_facts(&self.relations.knows_fact, entity),
            believed_facts: Self::snapshot_facts(&self.relations.believes_fact, entity),
            released_reservations: self.snapshot_archive_reservations(entity),
        })
    }

    pub fn plan_entity_archive_preparation(
        &self,
        entity: EntityId,
    ) -> Result<ArchivePreparationPlan, WorldError> {
        self.plan_entity_archive_preparation_with_policy(entity, &ArchivePreparationPolicy::all())
    }

    pub fn plan_entity_archive_preparation_with_policy(
        &self,
        entity: EntityId,
        policy: &ArchivePreparationPolicy,
    ) -> Result<ArchivePreparationPlan, WorldError> {
        let dependencies = self.archive_dependencies(entity)?;
        let mut actions = Vec::new();
        let mut blocked = Vec::new();

        for dependency in dependencies {
            let Some(resolution) = policy.resolution_for(dependency.kind) else {
                blocked.push(dependency);
                continue;
            };

            self.validate_archive_resolution(entity, dependency.kind, resolution)?;
            actions.push(ArchivePreparationAction {
                dependency,
                resolution,
            });
        }

        Ok(ArchivePreparationPlan { actions, blocked })
    }

    #[allow(dead_code)]
    pub(crate) fn prepare_entity_for_archive(
        &mut self,
        entity: EntityId,
    ) -> Result<ArchivePreparationReport, WorldError> {
        self.prepare_entity_for_archive_with_policy(entity, &ArchivePreparationPolicy::all())
    }

    #[allow(dead_code)]
    pub(crate) fn prepare_entity_for_archive_with_policy(
        &mut self,
        entity: EntityId,
        policy: &ArchivePreparationPolicy,
    ) -> Result<ArchivePreparationReport, WorldError> {
        let plan = self.plan_entity_archive_preparation_with_policy(entity, policy)?;
        let mut applied = Vec::with_capacity(plan.actions.len());

        for action in plan.actions {
            self.apply_archive_resolution(entity, &action.dependency, action.resolution)?;
            applied.push(action);
        }

        Ok(ArchivePreparationReport {
            applied,
            blocked: plan.blocked,
        })
    }

    #[allow(dead_code)]
    fn apply_archive_resolution(
        &mut self,
        entity: EntityId,
        dependency: &ArchiveDependency,
        resolution: ArchiveResolution,
    ) -> Result<(), WorldError> {
        self.validate_archive_resolution(entity, dependency.kind, resolution)?;

        match (dependency.kind, resolution) {
            (ArchiveDependencyKind::ContainsEntities, _) => {
                self.apply_containment_resolution(entity, resolution)
            }
            (ArchiveDependencyKind::PossessesEntities, _) => {
                self.apply_possession_resolution(&dependency.dependents, resolution)
            }
            (ArchiveDependencyKind::OwnsEntities, _) => {
                self.apply_ownership_resolution(&dependency.dependents, resolution)
            }
            (ArchiveDependencyKind::HasMembers, ArchiveResolution::RevokeMemberships) => {
                self.clear_memberships(entity, &dependency.dependents);
                Ok(())
            }
            (ArchiveDependencyKind::HasLoyalSubjects, ArchiveResolution::RevokeLoyalty) => {
                self.clear_loyalty_dependents(entity, &dependency.dependents);
                Ok(())
            }
            (ArchiveDependencyKind::HasHostileSubjects, ArchiveResolution::RevokeHostility) => {
                self.clear_hostility(entity, &dependency.dependents);
                Ok(())
            }
            (ArchiveDependencyKind::HasOfficeHolder, ArchiveResolution::VacateOffice) => {
                self.clear_office_assignment(entity);
                Ok(())
            }
            (ArchiveDependencyKind::HoldsOffices, ArchiveResolution::RelinquishOffices) => {
                self.relinquish_offices(&dependency.dependents);
                Ok(())
            }
            (kind, resolution) => Err(WorldError::InvalidOperation(format!(
                "archive resolution {resolution:?} is invalid for dependency kind {kind:?}"
            ))),
        }
    }

    #[allow(dead_code)]
    fn apply_containment_resolution(
        &mut self,
        entity: EntityId,
        resolution: ArchiveResolution,
    ) -> Result<(), WorldError> {
        match resolution {
            ArchiveResolution::DetachContentsToGround => {
                for child in self.direct_contents_of(entity) {
                    self.remove_from_container(child)?;
                }
                Ok(())
            }
            ArchiveResolution::SpillContentsRecursively => {
                for descendant in self.recursive_contents_of(entity) {
                    Self::clear_entity_relation(
                        &mut self.relations.contained_by,
                        &mut self.relations.contents_of,
                        descendant,
                    );
                }
                Ok(())
            }
            resolution => Err(WorldError::InvalidOperation(format!(
                "archive resolution {resolution:?} is invalid for dependency kind {:?}",
                ArchiveDependencyKind::ContainsEntities
            ))),
        }
    }

    #[allow(dead_code)]
    fn apply_possession_resolution(
        &mut self,
        dependents: &[EntityId],
        resolution: ArchiveResolution,
    ) -> Result<(), WorldError> {
        match resolution {
            ArchiveResolution::DropPossessions => {
                for possessed in dependents {
                    self.clear_possessor(*possessed)?;
                }
                Ok(())
            }
            ArchiveResolution::TransferPossessionsTo(target) => {
                for possessed in dependents {
                    self.set_possessor(*possessed, target)?;
                }
                Ok(())
            }
            resolution => Err(WorldError::InvalidOperation(format!(
                "archive resolution {resolution:?} is invalid for dependency kind {:?}",
                ArchiveDependencyKind::PossessesEntities
            ))),
        }
    }

    #[allow(dead_code)]
    fn apply_ownership_resolution(
        &mut self,
        dependents: &[EntityId],
        resolution: ArchiveResolution,
    ) -> Result<(), WorldError> {
        match resolution {
            ArchiveResolution::RelinquishOwnership => {
                for owned in dependents {
                    self.clear_owner(*owned)?;
                }
                Ok(())
            }
            ArchiveResolution::TransferOwnershipTo(target) => {
                for owned in dependents {
                    self.set_owner(*owned, target)?;
                }
                Ok(())
            }
            resolution => Err(WorldError::InvalidOperation(format!(
                "archive resolution {resolution:?} is invalid for dependency kind {:?}",
                ArchiveDependencyKind::OwnsEntities
            ))),
        }
    }

    #[allow(dead_code)]
    fn clear_memberships(&mut self, entity: EntityId, dependents: &[EntityId]) {
        for member in dependents {
            Self::clear_many_to_many_relation(
                &mut self.relations.member_of,
                &mut self.relations.members_of,
                *member,
                entity,
            );
        }
    }

    #[allow(dead_code)]
    fn clear_loyalty_dependents(&mut self, entity: EntityId, dependents: &[EntityId]) {
        for subject in dependents {
            Self::clear_weighted_relation(
                &mut self.relations.loyal_to,
                &mut self.relations.loyalty_from,
                *subject,
                entity,
            );
        }
    }

    #[allow(dead_code)]
    fn clear_hostility(&mut self, entity: EntityId, dependents: &[EntityId]) {
        for subject in dependents {
            Self::clear_many_to_many_relation(
                &mut self.relations.hostile_to,
                &mut self.relations.hostility_from,
                *subject,
                entity,
            );
        }
    }

    pub(super) fn clear_office_assignment(&mut self, office: EntityId) {
        Self::clear_entity_relation(
            &mut self.relations.office_holder,
            &mut self.relations.offices_held,
            office,
        );
    }

    #[allow(dead_code)]
    fn relinquish_offices(&mut self, dependents: &[EntityId]) {
        for office in dependents {
            self.clear_office_assignment(*office);
        }
    }

    fn validate_archive_resolution(
        &self,
        entity: EntityId,
        kind: ArchiveDependencyKind,
        resolution: ArchiveResolution,
    ) -> Result<(), WorldError> {
        match (kind, resolution) {
            (
                ArchiveDependencyKind::ContainsEntities,
                ArchiveResolution::DetachContentsToGround
                | ArchiveResolution::SpillContentsRecursively,
            )
            | (ArchiveDependencyKind::PossessesEntities, ArchiveResolution::DropPossessions)
            | (ArchiveDependencyKind::OwnsEntities, ArchiveResolution::RelinquishOwnership)
            | (ArchiveDependencyKind::HasMembers, ArchiveResolution::RevokeMemberships)
            | (ArchiveDependencyKind::HasLoyalSubjects, ArchiveResolution::RevokeLoyalty)
            | (ArchiveDependencyKind::HasHostileSubjects, ArchiveResolution::RevokeHostility)
            | (ArchiveDependencyKind::HasOfficeHolder, ArchiveResolution::VacateOffice)
            | (ArchiveDependencyKind::HoldsOffices, ArchiveResolution::RelinquishOffices) => Ok(()),
            (
                ArchiveDependencyKind::PossessesEntities,
                ArchiveResolution::TransferPossessionsTo(target),
            ) => {
                self.validate_archive_transfer_target(entity, target, "cannot transfer possessions")
            }
            (
                ArchiveDependencyKind::OwnsEntities,
                ArchiveResolution::TransferOwnershipTo(target),
            ) => self.validate_archive_transfer_target(entity, target, "cannot transfer ownership"),
            (kind, resolution) => Err(WorldError::InvalidOperation(format!(
                "archive resolution {resolution:?} is invalid for dependency kind {kind:?}"
            ))),
        }
    }

    fn validate_archive_transfer_target(
        &self,
        source: EntityId,
        target: EntityId,
        action: &str,
    ) -> Result<(), WorldError> {
        self.ensure_alive(target)?;
        if target == source {
            return Err(WorldError::InvalidOperation(format!(
                "{action} from {source} to itself during archive preparation"
            )));
        }
        Ok(())
    }

    fn snapshot_entities(
        relations: &BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
    ) -> Vec<EntityId> {
        relations
            .get(&entity)
            .map(|entities| entities.iter().copied().collect())
            .unwrap_or_default()
    }

    fn snapshot_weighted_entities(
        relations: &BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        entity: EntityId,
    ) -> Vec<(EntityId, Permille)> {
        relations
            .get(&entity)
            .map(|entities| {
                entities
                    .iter()
                    .map(|(&target, &strength)| (target, strength))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn snapshot_facts(
        relations: &BTreeMap<EntityId, BTreeSet<FactId>>,
        entity: EntityId,
    ) -> Vec<FactId> {
        relations
            .get(&entity)
            .map(|facts| facts.iter().copied().collect())
            .unwrap_or_default()
    }

    fn snapshot_archive_reservations(&self, entity: EntityId) -> Vec<ReservationRecord> {
        let mut released_reservations = self.reservations_for(entity);
        released_reservations.extend(
            self.relations
                .reservations
                .values()
                .filter(|reservation| reservation.reserver == entity)
                .cloned(),
        );
        released_reservations.sort_by_key(|reservation| reservation.id);
        released_reservations.dedup_by_key(|reservation| reservation.id);
        released_reservations
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CommodityKind, Container, ControlSource, EntityId, FactId, LoadUnits, Permille, Place,
        PlaceTag, Quantity, Tick, TickRange, Topology, World,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn open_container(capacity: u32) -> Container {
        Container {
            capacity: LoadUnits(capacity),
            allowed_commodities: None,
            allows_unique_items: true,
            allows_nested_containers: true,
        }
    }

    fn test_topology() -> Topology {
        let mut topology = Topology::new();
        topology
            .add_place(
                entity(5),
                Place {
                    name: "Square".to_string(),
                    capacity: None,
                    tags: [PlaceTag::Village].into_iter().collect(),
                },
            )
            .unwrap();
        topology
    }

    #[test]
    fn archive_mutation_snapshot_captures_forward_and_reverse_rows() {
        let mut world = World::new(Topology::new()).unwrap();
        let archived = world
            .create_agent("Aster", ControlSource::Ai, Tick(1))
            .unwrap();
        let owner = world.create_faction("River Pact", Tick(2)).unwrap();
        let holder = world
            .create_agent("Bram", ControlSource::Ai, Tick(3))
            .unwrap();
        let faction = world.create_faction("Granary Guild", Tick(4)).unwrap();
        let loyal_target = world.create_office("Chair", Tick(5)).unwrap();
        let hostile_target = world.create_faction("Watch", Tick(6)).unwrap();
        let hostile_subject = world
            .create_agent("Cato", ControlSource::Ai, Tick(7))
            .unwrap();
        let reserved_target = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(8))
            .unwrap();

        world.set_owner(archived, owner).unwrap();
        world.set_possessor(archived, holder).unwrap();
        world.add_member(archived, faction).unwrap();
        world
            .set_loyalty(archived, loyal_target, Permille::new(650).unwrap())
            .unwrap();
        world.add_hostility(archived, hostile_target).unwrap();
        world.add_hostility(hostile_subject, archived).unwrap();
        world.add_known_fact(archived, FactId(11)).unwrap();
        world.add_believed_fact(archived, FactId(12)).unwrap();
        world
            .try_reserve(
                archived,
                holder,
                TickRange::new(Tick(10), Tick(12)).unwrap(),
            )
            .unwrap();
        world
            .try_reserve(
                reserved_target,
                archived,
                TickRange::new(Tick(12), Tick(14)).unwrap(),
            )
            .unwrap();

        let snapshot = world.archive_mutation_snapshot(archived).unwrap();

        assert_eq!(snapshot.kind, crate::EntityKind::Agent);
        assert_eq!(snapshot.owned_by, Some(owner));
        assert_eq!(snapshot.possessed_by, Some(holder));
        assert_eq!(snapshot.member_of, vec![faction]);
        assert_eq!(
            snapshot.loyal_to,
            vec![(loyal_target, Permille::new(650).unwrap())]
        );
        assert_eq!(snapshot.hostile_to, vec![hostile_target]);
        assert_eq!(snapshot.hostility_from, vec![hostile_subject]);
        assert_eq!(snapshot.known_facts, vec![FactId(11)]);
        assert_eq!(snapshot.believed_facts, vec![FactId(12)]);
        assert!(snapshot.in_transit);
        assert_eq!(snapshot.released_reservations.len(), 2);
    }

    #[test]
    fn archive_mutation_snapshot_captures_placement_and_blocked_dependents() {
        let mut world = World::new(test_topology()).unwrap();
        let container = world.create_container(open_container(20), Tick(1)).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Coin, Quantity(1), Tick(2))
            .unwrap();
        let child = world
            .create_item_lot(CommodityKind::Bread, Quantity(1), Tick(3))
            .unwrap();

        world.set_ground_location(container, entity(5)).unwrap();
        world.put_into_container(item, container).unwrap();
        world.put_into_container(child, container).unwrap();

        let snapshot = world.archive_mutation_snapshot(container).unwrap();

        assert_eq!(snapshot.located_in, Some(entity(5)));
        assert!(!snapshot.in_transit);
        assert_eq!(snapshot.contents_of, vec![item, child]);
    }
}
