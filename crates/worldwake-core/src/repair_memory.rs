use crate::{
    Component, EntityId, GoalKey, InvalidatorTag, MemoryCapacityProfile, RepairKind, Tick,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BreachSignature {
    pub goal_key: GoalKey,
    pub invalidator: InvalidatorTag,
    pub step_target: Option<EntityId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairEntry {
    pub signature: BreachSignature,
    pub kind: RepairKind,
    pub succeeded: bool,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub success_count: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairMemory {
    pub repairs: BTreeMap<BreachSignature, RepairEntry>,
}

impl RepairMemory {
    pub fn record(&mut self, entry: RepairEntry) {
        match self.repairs.get(&entry.signature) {
            Some(existing) if existing.observed_tick > entry.observed_tick => {}
            _ => {
                self.repairs.insert(entry.signature, entry);
            }
        }
    }

    pub fn expire(&mut self, current_tick: Tick) {
        self.repairs
            .retain(|_, entry| entry.expires_tick > current_tick);
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
    use super::{BreachSignature, RepairEntry, RepairMemory};
    use crate::{
        GoalKind, InvalidatorTag, MemoryCapacityProfile, RepairKind, Tick,
        test_utils::{entity_id, sample_goal_key},
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_ord_hash_value_bounds<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + Debug + Serialize + DeserializeOwned,
    >() {
    }

    fn breach_signature(slot: u32) -> BreachSignature {
        BreachSignature {
            goal_key: sample_goal_key(),
            invalidator: InvalidatorTag::TargetMoved,
            step_target: Some(entity_id(slot, 0)),
        }
    }

    fn repair_entry(slot: u32, observed_tick: u64, success_count: u32) -> RepairEntry {
        RepairEntry {
            signature: breach_signature(slot),
            kind: RepairKind::RebindTarget,
            succeeded: true,
            observed_tick: Tick(observed_tick),
            expires_tick: Tick(observed_tick + 20),
            success_count,
        }
    }

    #[test]
    fn repair_memory_types_satisfy_required_bounds() {
        assert_component_bounds::<RepairMemory>();
        assert_value_bounds::<RepairMemory>();
        assert_copy_ord_hash_value_bounds::<BreachSignature>();
        assert_copy_value_bounds::<RepairEntry>();
    }

    #[test]
    fn breach_signature_roundtrips_through_bincode() {
        let signature = BreachSignature {
            goal_key: sample_goal_key(),
            invalidator: crate::InvalidatorTag::TargetMoved,
            step_target: Some(entity_id(7, 0)),
        };

        let bytes = bincode::serialize(&signature).unwrap();
        let roundtrip: BreachSignature = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, signature);
    }

    #[test]
    fn repair_entry_roundtrips_through_bincode() {
        let entry = repair_entry(7, 12, 3);

        let bytes = bincode::serialize(&entry).unwrap();
        let roundtrip: RepairEntry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, entry);
    }

    #[test]
    fn repair_entry_carries_kind_and_succeeded() {
        let mut entry = repair_entry(7, 12, 3);
        entry.kind = RepairKind::ReplaceProvider;
        entry.succeeded = false;

        let bytes = bincode::serialize(&entry).unwrap();
        let roundtrip: RepairEntry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip.kind, RepairKind::ReplaceProvider);
        assert!(!roundtrip.succeeded);
        assert_eq!(roundtrip.success_count, 3);
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
        let signature = breach_signature(9);
        let stale = RepairEntry {
            signature,
            kind: RepairKind::RebindTarget,
            succeeded: true,
            observed_tick: Tick(10),
            expires_tick: Tick(20),
            success_count: 1,
        };
        let fresh = RepairEntry {
            signature,
            kind: RepairKind::RebindTarget,
            succeeded: true,
            observed_tick: Tick(15),
            expires_tick: Tick(35),
            success_count: 4,
        };
        let mut memory = RepairMemory::default();

        memory.record(stale);
        memory.record(fresh);

        assert_eq!(memory.repairs[&signature], fresh);
    }

    #[test]
    fn record_preserves_fresher_entry_when_older_observation_arrives_late() {
        let signature = breach_signature(9);
        let fresh = RepairEntry {
            signature,
            kind: RepairKind::RebindTarget,
            succeeded: true,
            observed_tick: Tick(15),
            expires_tick: Tick(35),
            success_count: 4,
        };
        let stale = RepairEntry {
            signature,
            kind: RepairKind::RebindTarget,
            succeeded: true,
            observed_tick: Tick(10),
            expires_tick: Tick(20),
            success_count: 1,
        };
        let mut memory = RepairMemory::default();

        memory.record(fresh);
        memory.record(stale);

        assert_eq!(memory.repairs[&signature], fresh);
    }

    #[test]
    fn expire_prunes_stale_entries() {
        let mut memory = RepairMemory::default();
        memory.record(repair_entry(3, 8, 2));
        memory.record(repair_entry(4, 15, 1));

        memory.expire(Tick(30));

        assert_eq!(memory.repairs.len(), 1);
        assert!(!memory.repairs.contains_key(&breach_signature(3)));
        assert!(memory.repairs.contains_key(&breach_signature(4)));
    }

    #[test]
    fn enforce_capacity_evicts_oldest_entries_by_observed_tick() {
        let mut memory = RepairMemory::default();
        memory.record(repair_entry(1, 5, 1));
        memory.record(repair_entry(2, 7, 1));
        memory.record(repair_entry(3, 9, 1));

        memory.enforce_capacity(&MemoryCapacityProfile { memory_capacity: 2 });

        assert_eq!(memory.repairs.len(), 2);
        assert!(!memory.repairs.contains_key(&breach_signature(1)));
        assert!(memory.repairs.contains_key(&breach_signature(2)));
        assert!(memory.repairs.contains_key(&breach_signature(3)));
    }

    #[test]
    fn enforce_capacity_zero_evicts_all_entries() {
        let mut memory = RepairMemory::default();
        let mut other = repair_entry(5, 4, 1);
        other.signature.goal_key = crate::GoalKey::from(GoalKind::Sleep);
        memory.record(other);

        memory.enforce_capacity(&MemoryCapacityProfile { memory_capacity: 0 });

        assert!(memory.repairs.is_empty());
    }
}
