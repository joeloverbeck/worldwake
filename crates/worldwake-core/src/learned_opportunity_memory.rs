use crate::{Component, MemoryCapacityProfile, OpportunityKey, Tick};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpportunityEntry {
    pub opportunity: OpportunityKey,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub observed_at: crate::EntityId,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LearnedOpportunityMemory {
    pub opportunities: BTreeMap<OpportunityKey, OpportunityEntry>,
}

impl LearnedOpportunityMemory {
    pub fn record(&mut self, entry: OpportunityEntry) {
        match self.opportunities.get(&entry.opportunity) {
            Some(existing) if existing.observed_tick > entry.observed_tick => {}
            _ => {
                self.opportunities.insert(entry.opportunity, entry);
            }
        }
    }

    pub fn expire(&mut self, current_tick: Tick) {
        self.opportunities
            .retain(|_, entry| entry.expires_tick > current_tick);
    }

    pub fn enforce_capacity(&mut self, profile: &MemoryCapacityProfile) {
        let capacity = usize::try_from(profile.memory_capacity).unwrap_or(usize::MAX);
        while self.opportunities.len() > capacity {
            let oldest_key = self
                .opportunities
                .iter()
                .min_by_key(|(key, entry)| (entry.observed_tick, *key))
                .map(|(key, _)| *key)
                .unwrap();
            self.opportunities.remove(&oldest_key);
        }
    }
}

impl Component for LearnedOpportunityMemory {}

#[cfg(test)]
mod tests {
    use super::{LearnedOpportunityMemory, OpportunityEntry};
    use crate::{
        AcquisitionQuantity, CommodityKind, CommodityPurpose, GoalKey, GoalKind,
        MemoryCapacityProfile, OpportunityAnchor, OpportunityKey, Tick, test_utils::entity_id,
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_copy_value_bounds<T: Copy + Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn opportunity_key(slot: u32) -> OpportunityKey {
        OpportunityKey {
            goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            anchor: OpportunityAnchor::Place(entity_id(slot, 0)),
        }
    }

    fn opportunity_entry(slot: u32, observed_tick: u64) -> OpportunityEntry {
        OpportunityEntry {
            opportunity: opportunity_key(slot),
            observed_tick: Tick(observed_tick),
            expires_tick: Tick(observed_tick + 20),
            observed_at: entity_id(slot + 10, 0),
        }
    }

    #[test]
    fn learned_opportunity_memory_types_satisfy_required_bounds() {
        assert_component_bounds::<LearnedOpportunityMemory>();
        assert_value_bounds::<LearnedOpportunityMemory>();
        assert_copy_value_bounds::<OpportunityEntry>();
    }

    #[test]
    fn opportunity_entry_roundtrips_through_bincode() {
        let entry = opportunity_entry(4, 12);

        let bytes = bincode::serialize(&entry).unwrap();
        let roundtrip: OpportunityEntry = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, entry);
    }

    #[test]
    fn learned_opportunity_memory_roundtrips_through_bincode() {
        let mut memory = LearnedOpportunityMemory::default();
        memory.record(opportunity_entry(4, 12));

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: LearnedOpportunityMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
    }

    #[test]
    fn record_overwrites_existing_entry_when_revisit_is_fresher() {
        let key = opportunity_key(4);
        let stale = OpportunityEntry {
            opportunity: key,
            observed_tick: Tick(10),
            expires_tick: Tick(20),
            observed_at: entity_id(20, 0),
        };
        let fresh = OpportunityEntry {
            opportunity: key,
            observed_tick: Tick(14),
            expires_tick: Tick(30),
            observed_at: entity_id(21, 0),
        };
        let mut memory = LearnedOpportunityMemory::default();

        memory.record(stale);
        memory.record(fresh);

        assert_eq!(memory.opportunities[&key], fresh);
    }

    #[test]
    fn expire_prunes_stale_entries() {
        let mut memory = LearnedOpportunityMemory::default();
        memory.record(opportunity_entry(4, 12));
        memory.record(opportunity_entry(5, 18));

        memory.expire(Tick(35));

        assert_eq!(memory.opportunities.len(), 1);
        assert!(!memory.opportunities.contains_key(&opportunity_key(4)));
        assert!(memory.opportunities.contains_key(&opportunity_key(5)));
    }

    #[test]
    fn enforce_capacity_evicts_oldest_entries() {
        let mut memory = LearnedOpportunityMemory::default();
        memory.record(opportunity_entry(1, 5));
        memory.record(opportunity_entry(2, 7));
        memory.record(opportunity_entry(3, 9));

        memory.enforce_capacity(&MemoryCapacityProfile { memory_capacity: 2 });

        assert_eq!(memory.opportunities.len(), 2);
        assert!(!memory.opportunities.contains_key(&opportunity_key(1)));
        assert!(memory.opportunities.contains_key(&opportunity_key(2)));
        assert!(memory.opportunities.contains_key(&opportunity_key(3)));
    }
}
