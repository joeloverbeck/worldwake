//! Explicit typed storage for authoritative relation state.

use crate::{EntityId, RelationRecord, ReservationId, TickRange};
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationTables {
    pub(crate) located_in: BTreeMap<EntityId, EntityId>,
    pub(crate) entities_at: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) contained_by: BTreeMap<EntityId, EntityId>,
    pub(crate) contents_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) possessed_by: BTreeMap<EntityId, EntityId>,
    pub(crate) possessions_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) owned_by: BTreeMap<EntityId, EntityId>,
    pub(crate) property_of: BTreeMap<EntityId, BTreeSet<EntityId>>,
    pub(crate) reservations: BTreeMap<ReservationId, ReservationRecord>,
    pub(crate) reservations_by_entity: BTreeMap<EntityId, BTreeSet<ReservationId>>,
    pub(crate) next_reservation_id: u64,
}

impl RelationTables {
    pub fn remove_all(&mut self, entity: EntityId) {
        Self::remove_entity_relations(&mut self.located_in, &mut self.entities_at, entity);
        Self::remove_entity_relations(&mut self.contained_by, &mut self.contents_of, entity);
        Self::remove_entity_relations(&mut self.possessed_by, &mut self.possessions_of, entity);
        Self::remove_entity_relations(&mut self.owned_by, &mut self.property_of, entity);
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
}

#[cfg(test)]
mod tests {
    use super::{RelationTables, ReservationRecord};
    use crate::{EntityId, RelationRecord, ReservationId, Tick, TickRange};
    use serde::de::DeserializeOwned;
    use serde::Serialize;
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
        assert!(tables.contained_by.is_empty());
        assert!(tables.contents_of.is_empty());
        assert!(tables.possessed_by.is_empty());
        assert!(tables.possessions_of.is_empty());
        assert!(tables.owned_by.is_empty());
        assert!(tables.property_of.is_empty());
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
        let reservation_id = ReservationId(7);

        let mut tables = RelationTables::default();
        tables.located_in.insert(item, place);
        tables
            .entities_at
            .insert(place, [item].into_iter().collect());
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
        assert!(tables.contained_by.is_empty());
        assert!(tables.contents_of.is_empty());
        assert!(tables.possessed_by.is_empty());
        assert!(tables.possessions_of.is_empty());
        assert!(tables.owned_by.is_empty());
        assert!(tables.property_of.is_empty());
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
        let reservation_id = ReservationId(7);

        let mut tables = RelationTables::default();
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
        assert_eq!(tables.possessed_by.get(&item), Some(&holder));
        assert_eq!(tables.owned_by.get(&item), Some(&owner));
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
}
