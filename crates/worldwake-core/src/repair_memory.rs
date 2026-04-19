use crate::{Component, EntityId, GoalKey, MemoryCapacityProfile, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RepairKey {
    pub goal_key: GoalKey,
    pub alternate_target: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairEntry {
    pub repair_key: RepairKey,
    pub observed_tick: Tick,
    pub success_count: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemory {
    pub repairs: BTreeMap<RepairKey, RepairEntry>,
}

impl RepairMemory {
    pub fn record(&mut self, entry: RepairEntry) {
        match self.repairs.get(&entry.repair_key) {
            Some(existing) if existing.observed_tick > entry.observed_tick => {}
            _ => {
                self.repairs.insert(entry.repair_key, entry);
            }
        }
    }

    pub fn expire(&mut self, _current_tick: Tick) {
        // T002 does not yet define retention metadata for repair entries.
    }

    pub fn enforce_capacity(&mut self, profile: &MemoryCapacityProfile) {
        let capacity = usize::try_from(profile.memory_capacity).unwrap_or(usize::MAX);
        while self.repairs.len() > capacity {
            let oldest_key = self
                .repairs
                .iter()
                .min_by_key(|(key, entry)| (entry.observed_tick, *key))
                .map(|(key, _)| *key)
                .unwrap();
            self.repairs.remove(&oldest_key);
        }
    }
}

impl Component for RepairMemory {}

#[cfg(test)]
mod tests {
    use super::{RepairEntry, RepairKey, RepairMemory};
    use crate::{
        GoalKind, MemoryCapacityProfile, Tick,
        test_utils::{entity_id, sample_goal_key},
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn repair_key(slot: u32) -> RepairKey {
        RepairKey {
            goal_key: sample_goal_key(),
            alternate_target: entity_id(slot, 0),
        }
    }

    fn repair_entry(slot: u32, observed_tick: u64, success_count: u32) -> RepairEntry {
        RepairEntry {
            repair_key: repair_key(slot),
            observed_tick: Tick(observed_tick),
            success_count,
        }
    }

    #[test]
    fn repair_memory_types_satisfy_required_bounds() {
        assert_component_bounds::<RepairMemory>();
        assert_value_bounds::<RepairMemory>();
        assert_copy_value_bounds::<RepairKey>();
        assert_copy_value_bounds::<RepairEntry>();
    }

    #[test]
    fn repair_entry_roundtrips_through_bincode() {
        let entry = repair_entry(7, 12, 3);

        let bytes = bincode::serialize(&entry).unwrap();
        let roundtrip: RepairEntry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, entry);
    }

    #[test]
    fn repair_memory_roundtrips_through_bincode() {
        let mut memory = RepairMemory::default();
        memory.record(repair_entry(7, 12, 3));

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: RepairMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
    }

    #[test]
    fn record_overwrites_existing_entry_when_observation_is_fresher() {
        let key = repair_key(9);
        let stale = RepairEntry {
            repair_key: key,
            observed_tick: Tick(10),
            success_count: 1,
        };
        let fresh = RepairEntry {
            repair_key: key,
            observed_tick: Tick(15),
            success_count: 4,
        };
        let mut memory = RepairMemory::default();

        memory.record(stale);
        memory.record(fresh);

        assert_eq!(memory.repairs[&key], fresh);
    }

    #[test]
    fn record_preserves_fresher_entry_when_older_observation_arrives_late() {
        let key = repair_key(9);
        let fresh = RepairEntry {
            repair_key: key,
            observed_tick: Tick(15),
            success_count: 4,
        };
        let stale = RepairEntry {
            repair_key: key,
            observed_tick: Tick(10),
            success_count: 1,
        };
        let mut memory = RepairMemory::default();

        memory.record(fresh);
        memory.record(stale);

        assert_eq!(memory.repairs[&key], fresh);
    }

    #[test]
    fn expire_is_noop_without_retention_metadata() {
        let mut memory = RepairMemory::default();
        memory.record(repair_entry(3, 8, 2));

        memory.expire(Tick(999));

        assert_eq!(memory.repairs.len(), 1);
    }

    #[test]
    fn enforce_capacity_evicts_oldest_entries_by_observed_tick() {
        let mut memory = RepairMemory::default();
        memory.record(repair_entry(1, 5, 1));
        memory.record(repair_entry(2, 7, 1));
        memory.record(repair_entry(3, 9, 1));

        memory.enforce_capacity(&MemoryCapacityProfile { memory_capacity: 2 });

        assert_eq!(memory.repairs.len(), 2);
        assert!(!memory.repairs.contains_key(&repair_key(1)));
        assert!(memory.repairs.contains_key(&repair_key(2)));
        assert!(memory.repairs.contains_key(&repair_key(3)));
    }

    #[test]
    fn enforce_capacity_zero_evicts_all_entries() {
        let mut memory = RepairMemory::default();
        let mut other = repair_entry(5, 4, 1);
        other.repair_key.goal_key = crate::GoalKey::from(GoalKind::Sleep);
        memory.record(other);

        memory.enforce_capacity(&MemoryCapacityProfile { memory_capacity: 0 });

        assert!(memory.repairs.is_empty());
    }
}
