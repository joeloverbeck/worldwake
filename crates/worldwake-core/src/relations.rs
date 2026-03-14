//! Explicit typed storage for authoritative relation state.

use crate::{EntityId, Permille, RelationRecord, ReservationId, TickRange};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReservationRecord {
    pub id: ReservationId,
    pub entity: EntityId,
    pub reserver: EntityId,
    pub range: TickRange,
}

impl RelationRecord for ReservationRecord {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArchiveDependencyKind {
    ContainsEntities,
    PossessesEntities,
    OwnsEntities,
    HasMembers,
    HasLoyalSubjects,
    HasHostileSubjects,
    HasOfficeHolder,
    HoldsOffices,
}

impl ArchiveDependencyKind {
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::ContainsEntities => "contains other entities",
            Self::PossessesEntities => "possesses other entities",
            Self::OwnsEntities => "owns other entities",
            Self::HasMembers => "has member entities",
            Self::HasLoyalSubjects => "has loyal subjects",
            Self::HasHostileSubjects => "has hostile subjects",
            Self::HasOfficeHolder => "has an office holder",
            Self::HoldsOffices => "holds offices",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveDependency {
    pub kind: ArchiveDependencyKind,
    pub dependents: Vec<EntityId>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationTables {
    pub(crate) located_in: BTreeMap<EntityId, EntityId>,
    pub(crate) entities_at: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) in_transit: BTreeSet<EntityId>,
    pub(crate) contained_by: BTreeMap<EntityId, EntityId>,
    pub(crate) contents_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) possessed_by: BTreeMap<EntityId, EntityId>,
    pub(crate) possessions_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) owned_by: BTreeMap<EntityId, EntityId>,
    pub(crate) property_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) member_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) members_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) loyal_to: BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
    pub(crate) loyalty_from: BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
    pub(crate) office_holder: BTreeMap<EntityId, EntityId>,
    pub(crate) offices_held: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) hostile_to: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) hostility_from: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) reservations: BTreeMap<ReservationId, ReservationRecord>,
    pub(crate) reservations_by_entity: BTreeMap<EntityId, BTreeSet<ReservationId>>,
    pub(crate) next_reservation_id: u64,
}

impl RelationTables {
    pub(crate) fn archive_dependencies(&self, entity: EntityId) -> Vec<ArchiveDependency> {
        let mut dependencies = Vec::new();

        Self::push_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::ContainsEntities,
            self.contents_of.get(&entity),
        );
        Self::push_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::PossessesEntities,
            self.possessions_of.get(&entity),
        );
        Self::push_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::OwnsEntities,
            self.property_of.get(&entity),
        );
        Self::push_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::HasMembers,
            self.members_of.get(&entity),
        );
        Self::push_weighted_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::HasLoyalSubjects,
            self.loyalty_from.get(&entity),
        );
        Self::push_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::HasHostileSubjects,
            self.hostility_from.get(&entity),
        );
        if let Some(holder) = self.office_holder.get(&entity).copied() {
            dependencies.push(ArchiveDependency {
                kind: ArchiveDependencyKind::HasOfficeHolder,
                dependents: vec![holder],
            });
        }
        Self::push_archive_dependency(
            &mut dependencies,
            ArchiveDependencyKind::HoldsOffices,
            self.offices_held.get(&entity),
        );

        dependencies
    }

    pub fn remove_all(&mut self, entity: EntityId) {
        Self::remove_entity_relations(&mut self.located_in, &mut self.entities_at, entity);
        self.in_transit.remove(&entity);
        Self::remove_entity_relations(&mut self.contained_by, &mut self.contents_of, entity);
        Self::remove_entity_relations(&mut self.possessed_by, &mut self.possessions_of, entity);
        Self::remove_entity_relations(&mut self.owned_by, &mut self.property_of, entity);
        Self::remove_many_to_many_relations(&mut self.member_of, &mut self.members_of, entity);
        Self::remove_weighted_many_to_many_relations(
            &mut self.loyal_to,
            &mut self.loyalty_from,
            entity,
        );
        Self::remove_entity_relations(&mut self.office_holder, &mut self.offices_held, entity);
        Self::remove_many_to_many_relations(&mut self.hostile_to, &mut self.hostility_from, entity);
        self.remove_entity_reservations(entity);
    }

    fn remove_entity_relations(
        forward: &mut BTreeMap<EntityId, EntityId>,
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
    ) {
        if let Some(target) = forward.remove(&entity) {
            Self::remove_reverse_link(reverse, target, entity);
        }

        if let Some(sources) = reverse.remove(&entity) {
            for source in sources {
                forward.remove(&source);
            }
        }
    }

    fn remove_entity_reservations(&mut self, entity: EntityId) {
        let mut reservation_ids = self
            .reservations_by_entity
            .remove(&entity)
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        reservation_ids.extend(
            self.reservations
                .iter()
                .filter_map(|(id, reservation)| (reservation.reserver == entity).then_some(*id)),
        );

        reservation_ids.sort();
        reservation_ids.dedup();

        for reservation_id in reservation_ids {
            if let Some(reservation) = self.reservations.remove(&reservation_id) {
                Self::remove_reservation_index(
                    &mut self.reservations_by_entity,
                    reservation.entity,
                    reservation_id,
                );
            }
        }
    }

    fn remove_many_to_many_relations(
        forward: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        reverse: &mut BTreeMap<EntityId, BTreeSet<EntityId>>,
        entity: EntityId,
    ) {
        if let Some(targets) = forward.remove(&entity) {
            for target in targets {
                Self::remove_reverse_link(reverse, target, entity);
            }
        }

        if let Some(sources) = reverse.remove(&entity) {
            for source in sources {
                if let Some(targets) = forward.get_mut(&source) {
                    targets.remove(&entity);
                    if targets.is_empty() {
                        forward.remove(&source);
                    }
                }
            }
        }
    }

    fn remove_weighted_many_to_many_relations(
        forward: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        reverse: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
        entity: EntityId,
    ) {
        if let Some(targets) = forward.remove(&entity) {
            for target in targets.into_keys() {
                Self::remove_weighted_reverse_link(reverse, target, entity);
            }
        }

        if let Some(sources) = reverse.remove(&entity) {
            for source in sources.into_keys() {
                if let Some(targets) = forward.get_mut(&source) {
                    targets.remove(&entity);
                    if targets.is_empty() {
                        forward.remove(&source);
                    }
                }
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

    fn remove_reservation_index(
        reverse: &mut BTreeMap<EntityId, BTreeSet<ReservationId>>,
        entity: EntityId,
        reservation_id: ReservationId,
    ) {
        if let Some(reservations) = reverse.get_mut(&entity) {
            reservations.remove(&reservation_id);
            if reservations.is_empty() {
                reverse.remove(&entity);
            }
        }
    }

    fn push_archive_dependency(
        dependencies: &mut Vec<ArchiveDependency>,
        kind: ArchiveDependencyKind,
        rows: Option<&BTreeSet<EntityId>>,
    ) {
        if let Some(rows) = rows.filter(|rows| !rows.is_empty()) {
            dependencies.push(ArchiveDependency {
                kind,
                dependents: rows.iter().copied().collect(),
            });
        }
    }

    fn push_weighted_archive_dependency(
        dependencies: &mut Vec<ArchiveDependency>,
        kind: ArchiveDependencyKind,
        rows: Option<&BTreeMap<EntityId, Permille>>,
    ) {
        if let Some(rows) = rows.filter(|rows| !rows.is_empty()) {
            dependencies.push(ArchiveDependency {
                kind,
                dependents: rows.keys().copied().collect(),
            });
        }
    }

    fn remove_weighted_reverse_link(
        reverse: &mut BTreeMap<EntityId, BTreeMap<EntityId, Permille>>,
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

#[cfg(test)]
mod tests {
    use super::{RelationTables, ReservationRecord};
    use crate::{EntityId, Permille, RelationRecord, ReservationId, Tick, TickRange};
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_relation_record_bounds<T: RelationRecord + Eq + PartialEq>() {}
    fn assert_serde_bounds<T: Clone + Debug + Eq + Serialize + DeserializeOwned>() {}

    #[test]
    fn default_tables_are_empty() {
        let tables = RelationTables::default();

        assert!(tables.located_in.is_empty());
        assert!(tables.entities_at.is_empty());
        assert!(tables.in_transit.is_empty());
        assert!(tables.contained_by.is_empty());
        assert!(tables.contents_of.is_empty());
        assert!(tables.possessed_by.is_empty());
        assert!(tables.possessions_of.is_empty());
        assert!(tables.owned_by.is_empty());
        assert!(tables.property_of.is_empty());
        assert!(tables.member_of.is_empty());
        assert!(tables.members_of.is_empty());
        assert!(tables.loyal_to.is_empty());
        assert!(tables.loyalty_from.is_empty());
        assert!(tables.office_holder.is_empty());
        assert!(tables.offices_held.is_empty());
        assert!(tables.hostile_to.is_empty());
        assert!(tables.hostility_from.is_empty());
        assert!(tables.reservations.is_empty());
        assert!(tables.reservations_by_entity.is_empty());
        assert_eq!(tables.next_reservation_id, 0);
    }

    #[test]
    fn reservation_record_satisfies_required_bounds() {
        assert_relation_record_bounds::<ReservationRecord>();
        assert_serde_bounds::<ReservationRecord>();
    }

    #[test]
    fn relation_tables_bincode_roundtrip() {
        let item = entity(10);
        let place = entity(2);
        let container = entity(11);
        let holder = entity(12);
        let owner = entity(13);
        let reserver = entity(14);
        let faction = entity(15);
        let loyal_target = entity(16);
        let office = entity(17);
        let enemy = entity(18);
        let reservation_id = ReservationId(7);
        let reservation = ReservationRecord {
            id: reservation_id,
            entity: item,
            reserver,
            range: TickRange::new(Tick(5), Tick(9)).unwrap(),
        };

        let mut tables = RelationTables::default();
        tables.located_in.insert(item, place);
        tables
            .entities_at
            .insert(place, [item].into_iter().collect());
        tables.in_transit.insert(container);
        tables.contained_by.insert(item, container);
        tables
            .contents_of
            .insert(container, [item].into_iter().collect());
        tables.possessed_by.insert(item, holder);
        tables
            .possessions_of
            .insert(holder, [item].into_iter().collect());
        tables.owned_by.insert(item, owner);
        tables
            .property_of
            .insert(owner, [item].into_iter().collect());
        tables
            .member_of
            .insert(item, [faction].into_iter().collect());
        tables
            .members_of
            .insert(faction, [item].into_iter().collect());
        tables.loyal_to.insert(
            item,
            BTreeMap::from([(loyal_target, Permille::new(650).unwrap())]),
        );
        tables.loyalty_from.insert(
            loyal_target,
            BTreeMap::from([(item, Permille::new(650).unwrap())]),
        );
        tables.office_holder.insert(office, item);
        tables
            .offices_held
            .insert(item, [office].into_iter().collect());
        tables
            .hostile_to
            .insert(item, [enemy].into_iter().collect());
        tables
            .hostility_from
            .insert(enemy, [item].into_iter().collect());
        tables.reservations.insert(reservation_id, reservation);
        tables
            .reservations_by_entity
            .insert(item, [reservation_id].into_iter().collect());
        tables.next_reservation_id = 8;

        let bytes = bincode::serialize(&tables).unwrap();
        let roundtrip: RelationTables = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, tables);
    }

    #[test]
    fn remove_all_cleans_source_target_and_reserver_rows() {
        let item = entity(10);
        let place = entity(2);
        let container = entity(11);
        let holder = entity(12);
        let owner = entity(13);
        let reserver = entity(14);
        let faction = entity(15);
        let loyal_target = entity(16);
        let office = entity(17);
        let enemy = entity(18);
        let reservation_id = ReservationId(7);

        let mut tables = RelationTables::default();
        tables.located_in.insert(item, place);
        tables
            .entities_at
            .insert(place, [item].into_iter().collect());
        tables.in_transit.insert(item);
        tables.contained_by.insert(item, container);
        tables
            .contents_of
            .insert(container, [item].into_iter().collect());
        tables.possessed_by.insert(item, holder);
        tables
            .possessions_of
            .insert(holder, [item].into_iter().collect());
        tables.owned_by.insert(item, owner);
        tables
            .property_of
            .insert(owner, [item].into_iter().collect());
        tables
            .member_of
            .insert(item, [faction].into_iter().collect());
        tables
            .members_of
            .insert(faction, [item].into_iter().collect());
        tables.loyal_to.insert(
            item,
            BTreeMap::from([(loyal_target, Permille::new(650).unwrap())]),
        );
        tables.loyalty_from.insert(
            loyal_target,
            BTreeMap::from([(item, Permille::new(650).unwrap())]),
        );
        tables.office_holder.insert(office, item);
        tables
            .offices_held
            .insert(item, [office].into_iter().collect());
        tables
            .hostile_to
            .insert(item, [enemy].into_iter().collect());
        tables
            .hostility_from
            .insert(enemy, [item].into_iter().collect());
        tables.reservations.insert(
            reservation_id,
            ReservationRecord {
                id: reservation_id,
                entity: item,
                reserver,
                range: TickRange::new(Tick(5), Tick(9)).unwrap(),
            },
        );
        tables
            .reservations_by_entity
            .insert(item, [reservation_id].into_iter().collect());

        tables.remove_all(item);

        assert!(tables.located_in.is_empty());
        assert!(tables.entities_at.is_empty());
        assert!(tables.in_transit.is_empty());
        assert!(tables.contained_by.is_empty());
        assert!(tables.contents_of.is_empty());
        assert!(tables.possessed_by.is_empty());
        assert!(tables.possessions_of.is_empty());
        assert!(tables.owned_by.is_empty());
        assert!(tables.property_of.is_empty());
        assert!(tables.member_of.is_empty());
        assert!(tables.members_of.is_empty());
        assert!(tables.loyal_to.is_empty());
        assert!(tables.loyalty_from.is_empty());
        assert!(tables.office_holder.is_empty());
        assert!(tables.offices_held.is_empty());
        assert!(tables.hostile_to.is_empty());
        assert!(tables.hostility_from.is_empty());
        assert!(tables.reservations.is_empty());
        assert!(tables.reservations_by_entity.is_empty());
    }

    #[test]
    fn remove_all_cleans_rows_when_entity_is_relation_target_or_reserver() {
        let item = entity(10);
        let container = entity(11);
        let holder = entity(12);
        let owner = entity(13);
        let reserver = entity(14);
        let faction = entity(15);
        let office = entity(16);
        let enemy = entity(17);
        let reservation_id = ReservationId(7);

        let mut tables = RelationTables::default();
        tables.in_transit.insert(container);
        tables.contained_by.insert(item, container);
        tables
            .contents_of
            .insert(container, [item].into_iter().collect());
        tables.possessed_by.insert(item, holder);
        tables
            .possessions_of
            .insert(holder, [item].into_iter().collect());
        tables.owned_by.insert(item, owner);
        tables
            .property_of
            .insert(owner, [item].into_iter().collect());
        tables
            .member_of
            .insert(item, [faction].into_iter().collect());
        tables
            .members_of
            .insert(faction, [item].into_iter().collect());
        tables.office_holder.insert(office, item);
        tables
            .offices_held
            .insert(item, [office].into_iter().collect());
        tables
            .hostile_to
            .insert(item, [enemy].into_iter().collect());
        tables
            .hostility_from
            .insert(enemy, [item].into_iter().collect());
        tables.reservations.insert(
            reservation_id,
            ReservationRecord {
                id: reservation_id,
                entity: item,
                reserver,
                range: TickRange::new(Tick(5), Tick(9)).unwrap(),
            },
        );
        tables
            .reservations_by_entity
            .insert(item, [reservation_id].into_iter().collect());

        tables.remove_all(container);
        assert!(tables.contained_by.is_empty());
        assert!(tables.contents_of.is_empty());
        assert!(tables.in_transit.is_empty());
        assert_eq!(tables.possessed_by.get(&item), Some(&holder));
        assert_eq!(tables.owned_by.get(&item), Some(&owner));
        assert_eq!(
            tables.member_of.get(&item),
            Some(&BTreeSet::from([faction]))
        );
        assert_eq!(tables.office_holder.get(&office), Some(&item));
        assert_eq!(tables.hostile_to.get(&item), Some(&BTreeSet::from([enemy])));
        assert_eq!(
            tables.reservations.get(&reservation_id).unwrap().reserver,
            reserver
        );

        tables.remove_all(reserver);
        assert!(tables.reservations.is_empty());
        assert!(tables.reservations_by_entity.is_empty());
        assert_eq!(tables.possessed_by.get(&item), Some(&holder));
        assert_eq!(tables.owned_by.get(&item), Some(&owner));
    }

    #[test]
    fn remove_all_cleans_social_rows_when_entity_is_target_holder_or_office() {
        let member = entity(10);
        let faction = entity(11);
        let loyal_subject = entity(12);
        let target = entity(13);
        let office = entity(14);
        let holder = entity(15);
        let hostile_subject = entity(16);

        let mut tables = RelationTables::default();
        tables
            .member_of
            .insert(member, [faction].into_iter().collect());
        tables
            .members_of
            .insert(faction, [member].into_iter().collect());
        tables.loyal_to.insert(
            loyal_subject,
            BTreeMap::from([(target, Permille::new(500).unwrap())]),
        );
        tables.loyalty_from.insert(
            target,
            BTreeMap::from([(loyal_subject, Permille::new(500).unwrap())]),
        );
        tables.office_holder.insert(office, holder);
        tables
            .offices_held
            .insert(holder, [office].into_iter().collect());
        tables
            .hostile_to
            .insert(hostile_subject, [target].into_iter().collect());
        tables
            .hostility_from
            .insert(target, [hostile_subject].into_iter().collect());

        tables.remove_all(target);
        assert!(tables.loyal_to.is_empty());
        assert!(tables.loyalty_from.is_empty());
        assert!(tables.hostile_to.is_empty());
        assert!(tables.hostility_from.is_empty());
        assert_eq!(
            tables.member_of.get(&member),
            Some(&BTreeSet::from([faction]))
        );
        assert_eq!(tables.office_holder.get(&office), Some(&holder));

        tables.remove_all(holder);
        assert!(tables.office_holder.is_empty());
        assert!(tables.offices_held.is_empty());
        assert_eq!(
            tables.member_of.get(&member),
            Some(&BTreeSet::from([faction]))
        );

        tables.remove_all(faction);
        assert!(tables.member_of.is_empty());
        assert!(tables.members_of.is_empty());

        tables.office_holder.insert(office, member);
        tables
            .offices_held
            .insert(member, [office].into_iter().collect());

        tables.remove_all(office);
        assert!(tables.office_holder.is_empty());
        assert!(tables.offices_held.is_empty());
    }
}
