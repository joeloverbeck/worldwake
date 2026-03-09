use super::World;
use crate::{ArchiveDependency, ArchiveDependencyKind, EntityId, WorldError};
use std::collections::BTreeMap;

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

impl World {
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

    pub fn prepare_entity_for_archive(
        &mut self,
        entity: EntityId,
    ) -> Result<ArchivePreparationReport, WorldError> {
        self.prepare_entity_for_archive_with_policy(entity, &ArchivePreparationPolicy::all())
    }

    pub fn prepare_entity_for_archive_with_policy(
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
                self.clear_loyalty(entity, &dependency.dependents);
                Ok(())
            }
            (ArchiveDependencyKind::HasHostileSubjects, ArchiveResolution::RevokeHostility) => {
                self.clear_hostility(entity, &dependency.dependents);
                Ok(())
            }
            (ArchiveDependencyKind::HasOfficeHolder, ArchiveResolution::VacateOffice) => {
                self.vacate_office(entity);
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

    fn clear_loyalty(&mut self, entity: EntityId, dependents: &[EntityId]) {
        for subject in dependents {
            Self::clear_many_to_many_relation(
                &mut self.relations.loyal_to,
                &mut self.relations.loyalty_from,
                *subject,
                entity,
            );
        }
    }

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

    fn vacate_office(&mut self, office: EntityId) {
        Self::clear_entity_relation(
            &mut self.relations.office_holder,
            &mut self.relations.offices_held,
            office,
        );
    }

    fn relinquish_offices(&mut self, dependents: &[EntityId]) {
        for office in dependents {
            Self::clear_entity_relation(
                &mut self.relations.office_holder,
                &mut self.relations.offices_held,
                *office,
            );
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
}
