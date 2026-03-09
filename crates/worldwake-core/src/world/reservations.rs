use super::World;
use crate::{EntityId, ReservationId, ReservationRecord, TickRange, WorldError};

impl World {
    #[must_use]
    pub fn reservation(&self, reservation_id: ReservationId) -> Option<&ReservationRecord> {
        self.relations.reservations.get(&reservation_id)
    }

    pub(crate) fn try_reserve(
        &mut self,
        entity: EntityId,
        reserver: EntityId,
        range: TickRange,
    ) -> Result<ReservationId, WorldError> {
        self.ensure_alive(entity)?;
        self.ensure_alive(reserver)?;

        for reservation in self.indexed_reservations_for(entity)? {
            if reservation.range.overlaps(&range) {
                return Err(WorldError::ConflictingReservation { entity });
            }
        }

        let next_id = self.relations.next_reservation_id;
        self.relations.next_reservation_id =
            next_id
                .checked_add(1)
                .ok_or(WorldError::InvariantViolation(
                    "reservation id counter overflowed".to_string(),
                ))?;

        let reservation_id = ReservationId(next_id);
        self.relations.reservations.insert(
            reservation_id,
            ReservationRecord {
                id: reservation_id,
                entity,
                reserver,
                range,
            },
        );
        self.relations
            .reservations_by_entity
            .entry(entity)
            .or_default()
            .insert(reservation_id);

        Ok(reservation_id)
    }

    pub(crate) fn release_reservation(
        &mut self,
        reservation_id: ReservationId,
    ) -> Result<(), WorldError> {
        let reservation = self.relations.reservations.remove(&reservation_id).ok_or(
            WorldError::InvalidOperation(format!("reservation {reservation_id} does not exist")),
        )?;

        self.remove_reservation_index(reservation.entity, reservation_id)
    }

    #[must_use]
    pub fn reservations_for(&self, entity: EntityId) -> Vec<ReservationRecord> {
        if !self.is_alive(entity) {
            return Vec::new();
        }

        self.relations
            .reservations
            .values()
            .filter(|reservation| reservation.entity == entity)
            .cloned()
            .collect()
    }

    fn indexed_reservations_for(
        &self,
        entity: EntityId,
    ) -> Result<Vec<ReservationRecord>, WorldError> {
        let Some(reservation_ids) = self.relations.reservations_by_entity.get(&entity) else {
            return Ok(Vec::new());
        };

        let mut reservations = Vec::with_capacity(reservation_ids.len());
        for reservation_id in reservation_ids {
            let reservation = self.relations.reservations.get(reservation_id).ok_or(
                WorldError::InvariantViolation(format!(
                    "reservation index for {entity} points to missing reservation {reservation_id}"
                )),
            )?;
            if reservation.entity != entity {
                return Err(WorldError::InvariantViolation(format!(
                    "reservation {reservation_id} is indexed under {entity} but belongs to {}",
                    reservation.entity
                )));
            }
            reservations.push(reservation.clone());
        }

        Ok(reservations)
    }

    fn remove_reservation_index(
        &mut self,
        entity: EntityId,
        reservation_id: ReservationId,
    ) -> Result<(), WorldError> {
        let Some(reservation_ids) = self.relations.reservations_by_entity.get_mut(&entity) else {
            return Err(WorldError::InvariantViolation(format!(
                "reservation {reservation_id} existed without an entity index for {entity}"
            )));
        };

        if !reservation_ids.remove(&reservation_id) {
            return Err(WorldError::InvariantViolation(format!(
                "reservation {reservation_id} existed but was missing from the entity index for {entity}"
            )));
        }

        if reservation_ids.is_empty() {
            self.relations.reservations_by_entity.remove(&entity);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{CommodityKind, ControlSource, Quantity, Tick, TickRange, Topology, World};

    #[test]
    fn reservation_accessor_returns_created_record_and_none_after_release() {
        let mut world = World::new(Topology::new()).unwrap();
        let item = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let reserver = world
            .create_agent("Aster", ControlSource::Ai, Tick(2))
            .unwrap();
        let range = TickRange::new(Tick(4), Tick(7)).unwrap();

        let reservation_id = world.try_reserve(item, reserver, range).unwrap();
        let reservation = world.reservation(reservation_id).unwrap();
        assert_eq!(reservation.id, reservation_id);
        assert_eq!(reservation.entity, item);
        assert_eq!(reservation.reserver, reserver);
        assert_eq!(reservation.range, range);

        world.release_reservation(reservation_id).unwrap();
        assert_eq!(world.reservation(reservation_id), None);
    }
}
