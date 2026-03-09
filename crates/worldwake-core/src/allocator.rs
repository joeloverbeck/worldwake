//! Deterministic entity allocation with generational stale-id detection.

use crate::{EntityId, EntityKind, EntityMeta, Tick, WorldError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Deterministic generational entity allocator.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityAllocator {
    slots: BTreeMap<u32, SlotRecord>,
    free_slots: BTreeSet<u32>,
    next_slot: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SlotRecord {
    generation: u32,
    meta: Option<EntityMeta>,
}

impl EntityAllocator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_entity(&mut self, kind: EntityKind, created_at: Tick) -> EntityId {
        let slot = self.allocate_slot();
        let record = self
            .slots
            .entry(slot)
            .or_insert_with(|| SlotRecord::new(0));

        debug_assert!(record.meta.is_none(), "reused slot must be empty");

        record.meta = Some(EntityMeta {
            kind,
            created_at,
            archived_at: None,
        });

        EntityId {
            slot,
            generation: record.generation,
        }
    }

    pub fn archive_entity(&mut self, id: EntityId, tick: Tick) -> Result<(), WorldError> {
        let record = self.record_mut(id)?;
        let meta = record.meta.as_mut().ok_or(WorldError::EntityNotFound(id))?;

        if meta.archived_at.is_some() {
            return Err(WorldError::ArchivedEntity(id));
        }

        meta.archived_at = Some(tick);
        Ok(())
    }

    pub fn purge_entity(&mut self, id: EntityId) -> Result<(), WorldError> {
        let record = self.record_mut(id)?;
        let meta = record.meta.as_ref().ok_or(WorldError::EntityNotFound(id))?;

        if meta.archived_at.is_none() {
            return Err(WorldError::InvalidOperation(format!(
                "cannot purge live entity: {id}"
            )));
        }

        record.meta = None;
        record.generation = record.generation.checked_add(1).ok_or_else(|| {
            WorldError::InvariantViolation(format!("entity generation overflow for slot {}", id.slot))
        })?;
        self.free_slots.insert(id.slot);
        Ok(())
    }

    #[must_use]
    pub fn is_alive(&self, id: EntityId) -> bool {
        self.get_meta(id).is_some_and(|meta| meta.archived_at.is_none())
    }

    #[must_use]
    pub fn is_archived(&self, id: EntityId) -> bool {
        self.get_meta(id).is_some_and(|meta| meta.archived_at.is_some())
    }

    #[must_use]
    pub fn get_meta(&self, id: EntityId) -> Option<&EntityMeta> {
        self.record(id)?.meta.as_ref()
    }

    pub fn entity_ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.slots.iter().filter_map(|(slot, record)| {
            record.meta.as_ref().and_then(|meta| {
                (meta.archived_at.is_none()).then_some(EntityId {
                    slot: *slot,
                    generation: record.generation,
                })
            })
        })
    }

    fn allocate_slot(&mut self) -> u32 {
        if let Some(slot) = self.free_slots.pop_first() {
            slot
        } else {
            let slot = self.next_slot;
            self.next_slot += 1;
            slot
        }
    }

    fn record(&self, id: EntityId) -> Option<&SlotRecord> {
        self.slots
            .get(&id.slot)
            .filter(|record| record.generation == id.generation)
    }

    fn record_mut(&mut self, id: EntityId) -> Result<&mut SlotRecord, WorldError> {
        match self.slots.get_mut(&id.slot) {
            Some(record) if record.generation == id.generation => Ok(record),
            _ => Err(WorldError::EntityNotFound(id)),
        }
    }
}

impl SlotRecord {
    const fn new(generation: u32) -> Self {
        Self {
            generation,
            meta: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EntityAllocator;
    use crate::{EntityId, EntityKind, WorldError};
    use crate::Tick;

    #[test]
    fn create_produces_unique_ids() {
        let mut allocator = EntityAllocator::new();
        let ids = [
            allocator.create_entity(EntityKind::Agent, Tick(1)),
            allocator.create_entity(EntityKind::Place, Tick(2)),
            allocator.create_entity(EntityKind::Facility, Tick(3)),
        ];

        assert_eq!(ids[0].slot, 0);
        assert_eq!(ids[1].slot, 1);
        assert_eq!(ids[2].slot, 2);
        assert!(ids.windows(2).all(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn archive_marks_non_live() {
        let mut allocator = EntityAllocator::new();
        let id = allocator.create_entity(EntityKind::Agent, Tick(5));

        allocator.archive_entity(id, Tick(8)).unwrap();

        assert!(!allocator.is_alive(id));
        assert!(allocator.is_archived(id));
    }

    #[test]
    fn archived_entity_still_has_meta() {
        let mut allocator = EntityAllocator::new();
        let id = allocator.create_entity(EntityKind::Facility, Tick(10));

        allocator.archive_entity(id, Tick(11)).unwrap();

        let meta = allocator.get_meta(id).unwrap();
        assert_eq!(meta.kind, EntityKind::Facility);
        assert_eq!(meta.created_at, Tick(10));
        assert_eq!(meta.archived_at, Some(Tick(11)));
    }

    #[test]
    fn purge_frees_slot() {
        let mut allocator = EntityAllocator::new();
        let id = allocator.create_entity(EntityKind::Agent, Tick(1));

        allocator.archive_entity(id, Tick(2)).unwrap();
        allocator.purge_entity(id).unwrap();

        let reused = allocator.create_entity(EntityKind::Place, Tick(3));
        assert_eq!(reused.slot, id.slot);
    }

    #[test]
    fn slot_reuse_increments_generation() {
        let mut allocator = EntityAllocator::new();
        let id = allocator.create_entity(EntityKind::Agent, Tick(1));

        allocator.archive_entity(id, Tick(2)).unwrap();
        allocator.purge_entity(id).unwrap();

        let reused = allocator.create_entity(EntityKind::Place, Tick(3));
        assert_eq!(reused.slot, id.slot);
        assert_eq!(reused.generation, id.generation + 1);
    }

    #[test]
    fn stale_id_not_found_after_reuse() {
        let mut allocator = EntityAllocator::new();
        let stale = allocator.create_entity(EntityKind::Agent, Tick(1));

        allocator.archive_entity(stale, Tick(2)).unwrap();
        allocator.purge_entity(stale).unwrap();
        let fresh = allocator.create_entity(EntityKind::Place, Tick(3));

        assert_ne!(stale, fresh);
        assert!(allocator.get_meta(stale).is_none());
        assert!(!allocator.is_alive(stale));
        assert_eq!(allocator.get_meta(fresh).unwrap().kind, EntityKind::Place);
    }

    #[test]
    fn purge_live_entity_errors() {
        let mut allocator = EntityAllocator::new();
        let id = allocator.create_entity(EntityKind::Agent, Tick(1));

        let err = allocator.purge_entity(id).unwrap_err();
        assert!(matches!(err, WorldError::InvalidOperation(_)));
    }

    #[test]
    fn double_archive_errors() {
        let mut allocator = EntityAllocator::new();
        let id = allocator.create_entity(EntityKind::Agent, Tick(1));

        allocator.archive_entity(id, Tick(2)).unwrap();
        let err = allocator.archive_entity(id, Tick(3)).unwrap_err();
        assert!(matches!(err, WorldError::ArchivedEntity(actual) if actual == id));
    }

    #[test]
    fn entity_ids_sorted_order() {
        let mut allocator = EntityAllocator::new();
        let id0 = allocator.create_entity(EntityKind::Agent, Tick(1));
        let id1 = allocator.create_entity(EntityKind::Place, Tick(2));
        let id2 = allocator.create_entity(EntityKind::Facility, Tick(3));

        allocator.archive_entity(id1, Tick(4)).unwrap();

        let live: Vec<EntityId> = allocator.entity_ids().collect();
        assert_eq!(live, vec![id0, id2]);
    }

    #[test]
    fn allocator_bincode_roundtrip() {
        let mut allocator = EntityAllocator::new();
        let archived = allocator.create_entity(EntityKind::Agent, Tick(1));
        let live = allocator.create_entity(EntityKind::Place, Tick(2));

        allocator.archive_entity(archived, Tick(3)).unwrap();
        allocator.purge_entity(archived).unwrap();
        let reused = allocator.create_entity(EntityKind::Facility, Tick(4));

        let bytes = bincode::serialize(&allocator).unwrap();
        let roundtrip: EntityAllocator = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, allocator);
        assert!(!roundtrip.is_alive(archived));
        assert!(roundtrip.is_alive(live));
        assert!(roundtrip.is_alive(reused));
    }
}
