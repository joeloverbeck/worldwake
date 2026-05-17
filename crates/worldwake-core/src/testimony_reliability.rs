use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{EntityId, EventId, Permille, TestimonyTrustProfile, Tick, TopicScope};

pub const PROVENANCE_RING_CAPACITY: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct TestimonyReliability {
    entries: BTreeMap<TestimonyReliabilityKey, TestimonyReliabilityEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TestimonyReliabilityKey {
    pub source: EntityId,
    pub topic: TopicScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyReliabilityEntry {
    pub direct_confirmations: u32,
    pub direct_refutations: u32,
    pub stale_claims: u32,
    pub contradicted_claims: u32,
    pub last_updated_tick: Tick,
    pub provenance_events: Vec<EventId>,
}

impl TestimonyReliabilityEntry {
    #[must_use]
    pub fn trust(&self, profile: &TestimonyTrustProfile, topic: TopicScope) -> Permille {
        if self.observations() < u32::from(profile.minimum_observations) {
            return Permille::new_unchecked(500);
        }

        let positive =
            i64::from(self.direct_confirmations) * i64::from(profile.confirmation_weight.value());
        let negative = i64::from(self.direct_refutations)
            * i64::from(profile.refutation_penalty.value())
            + i64::from(self.stale_claims) * i64::from(profile.stale_decay_per_tick.value())
            + i64::from(self.contradicted_claims) * i64::from(profile.contradicted_penalty.value());
        let topic_weight = i64::from(topic_weight(profile, topic).value());
        let adjusted_signal = (positive - negative) * topic_weight / 1000;

        permille_from_signed(500 + adjusted_signal)
    }

    #[must_use]
    pub fn observations(&self) -> u32 {
        self.direct_confirmations
            .saturating_add(self.direct_refutations)
            .saturating_add(self.stale_claims)
            .saturating_add(self.contradicted_claims)
    }

    fn push_provenance(&mut self, event: EventId) {
        if self.provenance_events.len() == PROVENANCE_RING_CAPACITY {
            self.provenance_events.drain(..1);
        }
        self.provenance_events.push(event);
    }
}

fn topic_weight(profile: &TestimonyTrustProfile, topic: TopicScope) -> Permille {
    match topic {
        TopicScope::RouteHazard => profile.topic_weight_route_hazard,
        TopicScope::ResourceAvailability => profile.topic_weight_resource_availability,
        TopicScope::OfficeHolder => profile.topic_weight_office_holder,
        TopicScope::AccusationCredibility => profile.topic_weight_accusation_credibility,
        TopicScope::BountyValidity => profile.topic_weight_bounty_validity,
        TopicScope::PriceLevel => profile.topic_weight_price_level,
        TopicScope::EntityWhereabouts => profile.topic_weight_entity_whereabouts,
        TopicScope::GeneralFact => profile.topic_weight_general_fact,
    }
}

fn permille_from_signed(value: i64) -> Permille {
    let clamped = u16::try_from(value.clamp(0, 1000)).expect("value clamped to permille range");
    Permille::new_unchecked(clamped)
}

impl TestimonyReliability {
    pub fn record_confirmation(
        &mut self,
        key: TestimonyReliabilityKey,
        event: EventId,
        tick: Tick,
    ) {
        self.record(key, event, tick, |entry| {
            entry.direct_confirmations = entry.direct_confirmations.saturating_add(1);
        });
    }

    pub fn record_refutation(&mut self, key: TestimonyReliabilityKey, event: EventId, tick: Tick) {
        self.record(key, event, tick, |entry| {
            entry.direct_refutations = entry.direct_refutations.saturating_add(1);
        });
    }

    pub fn record_stale(&mut self, key: TestimonyReliabilityKey, event: EventId, tick: Tick) {
        self.record(key, event, tick, |entry| {
            entry.stale_claims = entry.stale_claims.saturating_add(1);
        });
    }

    pub fn record_contradiction(
        &mut self,
        key: TestimonyReliabilityKey,
        event: EventId,
        tick: Tick,
    ) {
        self.record(key, event, tick, |entry| {
            entry.contradicted_claims = entry.contradicted_claims.saturating_add(1);
        });
    }

    #[must_use]
    pub fn get(&self, key: &TestimonyReliabilityKey) -> Option<&TestimonyReliabilityEntry> {
        self.entries.get(key)
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&TestimonyReliabilityKey, &TestimonyReliabilityEntry)> {
        self.entries.iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn record(
        &mut self,
        key: TestimonyReliabilityKey,
        event: EventId,
        tick: Tick,
        update: impl FnOnce(&mut TestimonyReliabilityEntry),
    ) {
        let entry = self
            .entries
            .entry(key)
            .or_insert(TestimonyReliabilityEntry {
                direct_confirmations: 0,
                direct_refutations: 0,
                stale_claims: 0,
                contradicted_claims: 0,
                last_updated_tick: tick,
                provenance_events: Vec::new(),
            });
        update(entry);
        entry.last_updated_tick = tick;
        entry.push_provenance(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PROVENANCE_RING_CAPACITY, TestimonyReliability, TestimonyReliabilityEntry,
        TestimonyReliabilityKey,
    };
    use crate::{EntityId, EventId, Permille, TestimonyTrustProfile, Tick, TopicScope};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn key() -> TestimonyReliabilityKey {
        TestimonyReliabilityKey {
            source: entity(7),
            topic: TopicScope::RouteHazard,
        }
    }

    #[test]
    fn record_methods_increment_counters_and_update_tick() {
        let mut reliability = TestimonyReliability::default();
        let key = key();

        reliability.record_confirmation(key, EventId(1), Tick(10));
        reliability.record_refutation(key, EventId(2), Tick(11));
        reliability.record_stale(key, EventId(3), Tick(12));
        reliability.record_contradiction(key, EventId(4), Tick(13));

        let entry = reliability.get(&key).expect("entry exists");
        assert_eq!(entry.direct_confirmations, 1);
        assert_eq!(entry.direct_refutations, 1);
        assert_eq!(entry.stale_claims, 1);
        assert_eq!(entry.contradicted_claims, 1);
        assert_eq!(entry.observations(), 4);
        assert_eq!(entry.last_updated_tick, Tick(13));
        assert_eq!(
            entry.provenance_events,
            vec![EventId(1), EventId(2), EventId(3), EventId(4)]
        );
    }

    #[test]
    fn provenance_ring_evicts_oldest_event_at_capacity() {
        let mut reliability = TestimonyReliability::default();
        let key = key();

        for event in 0..(PROVENANCE_RING_CAPACITY as u64 + 2) {
            reliability.record_confirmation(key, EventId(event), Tick(event));
        }

        let entry = reliability.get(&key).expect("entry exists");
        assert_eq!(entry.direct_confirmations, 10);
        assert_eq!(entry.provenance_events.len(), PROVENANCE_RING_CAPACITY);
        assert_eq!(entry.provenance_events.first(), Some(&EventId(2)));
        assert_eq!(entry.provenance_events.last(), Some(&EventId(9)));
    }

    #[test]
    fn testimony_reliability_bincode_round_trip_preserves_entries() {
        let mut reliability = TestimonyReliability::default();
        reliability.record_refutation(key(), EventId(3), Tick(5));

        let encoded = bincode::serialize(&reliability).expect("serialize reliability");
        let decoded: TestimonyReliability =
            bincode::deserialize(&encoded).expect("deserialize reliability");

        assert_eq!(decoded, reliability);
    }

    #[test]
    fn trust_is_neutral_below_minimum_observations() {
        let entry = TestimonyReliabilityEntry {
            direct_confirmations: 1,
            direct_refutations: 0,
            stale_claims: 0,
            contradicted_claims: 0,
            last_updated_tick: Tick(1),
            provenance_events: vec![EventId(1)],
        };

        assert_eq!(
            entry.trust(&TestimonyTrustProfile::default(), TopicScope::RouteHazard),
            Permille::new_unchecked(500)
        );
    }

    #[test]
    fn trust_increases_and_decreases_from_evidence_with_topic_weight() {
        let profile = TestimonyTrustProfile {
            topic_weight_route_hazard: Permille::new_unchecked(1000),
            ..TestimonyTrustProfile::default()
        };
        let confirmed = TestimonyReliabilityEntry {
            direct_confirmations: 2,
            direct_refutations: 0,
            stale_claims: 0,
            contradicted_claims: 0,
            last_updated_tick: Tick(2),
            provenance_events: vec![EventId(1), EventId(2)],
        };
        let refuted = TestimonyReliabilityEntry {
            direct_confirmations: 0,
            direct_refutations: 1,
            stale_claims: 0,
            contradicted_claims: 1,
            last_updated_tick: Tick(3),
            provenance_events: vec![EventId(3), EventId(4)],
        };

        assert_eq!(
            confirmed.trust(&profile, TopicScope::RouteHazard),
            Permille::new_unchecked(1000)
        );
        assert_eq!(
            refuted.trust(&profile, TopicScope::RouteHazard),
            Permille::ZERO
        );
    }
}
