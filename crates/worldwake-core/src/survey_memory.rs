//! Per-agent survey records for remembered exploration outcomes.

use crate::{CognitiveProfile, EntityId, HypothesisKind, Permille, Tick, traits::Component};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyRecord {
    pub place: EntityId,
    pub hypothesis: HypothesisKind,
    pub found: bool,
    pub confidence: Permille,
    pub recorded_tick: Tick,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurveyMemory {
    pub entries: Vec<SurveyRecord>,
}

impl Component for SurveyMemory {}

impl SurveyMemory {
    #[must_use]
    pub fn find(&self, place: EntityId, hypothesis: HypothesisKind) -> Option<&SurveyRecord> {
        self.entries
            .iter()
            .filter(|record| record.place == place && record.hypothesis == hypothesis)
            .max_by_key(|record| record.recorded_tick)
    }

    pub fn record(&mut self, record: SurveyRecord, capacity: usize) {
        if let Some(existing) = self.entries.iter_mut().find(|existing| {
            existing.place == record.place && existing.hypothesis == record.hypothesis
        }) {
            *existing = record;
            return;
        }

        self.entries.push(record);
        if self.entries.len() > capacity {
            self.entries.sort_by_key(|record| record.recorded_tick);
            self.entries.remove(0);
        }
    }

    pub fn enforce_limits(&mut self, current_tick: Tick, profile: &CognitiveProfile) {
        let retention = profile.survey_memory_retention_ticks;
        self.entries
            .retain(|record| current_tick.0.saturating_sub(record.recorded_tick.0) <= retention);
    }
}

#[cfg(test)]
mod tests {
    use super::{SurveyMemory, SurveyRecord};
    use crate::{
        CognitiveProfile, CommodityKind, HypothesisKind, Permille, Tick, test_utils::entity_id,
        traits::Component,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn record(
        place_index: u32,
        hypothesis: HypothesisKind,
        found: bool,
        confidence: u16,
        recorded_tick: u64,
    ) -> SurveyRecord {
        SurveyRecord {
            place: entity_id(place_index, 0),
            hypothesis,
            found,
            confidence: Permille::new(confidence).unwrap(),
            recorded_tick: Tick(recorded_tick),
        }
    }

    fn apple_hypothesis() -> HypothesisKind {
        HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Apple,
        }
    }

    #[test]
    fn survey_memory_component_bounds() {
        assert_component_bounds::<SurveyMemory>();
        assert_value_bounds::<SurveyMemory>();
        assert_value_bounds::<SurveyRecord>();
    }

    #[test]
    fn survey_memory_record_replaces_same_place_hypothesis_entry() {
        let mut memory = SurveyMemory::default();
        let original = record(1, apple_hypothesis(), false, 600, 10);
        let replacement = record(1, apple_hypothesis(), true, 900, 20);

        memory.record(original, 4);
        memory.record(replacement, 4);

        assert_eq!(memory.entries, vec![replacement]);
    }

    #[test]
    fn survey_memory_record_appends_distinct_place_or_hypothesis_entries() {
        let mut memory = SurveyMemory::default();
        let first = record(1, apple_hypothesis(), false, 600, 10);
        let distinct_place = record(2, apple_hypothesis(), false, 600, 11);
        let distinct_hypothesis = record(1, HypothesisKind::MayContainLatrine, true, 700, 12);

        memory.record(first, 4);
        memory.record(distinct_place, 4);
        memory.record(distinct_hypothesis, 4);

        assert_eq!(
            memory.entries,
            vec![first, distinct_place, distinct_hypothesis]
        );
    }

    #[test]
    fn survey_memory_record_evicts_oldest_on_capacity_overflow() {
        let mut memory = SurveyMemory::default();
        let oldest = record(1, apple_hypothesis(), false, 600, 10);
        let retained = record(2, apple_hypothesis(), true, 700, 20);
        let newest = record(3, apple_hypothesis(), false, 800, 30);

        memory.record(oldest, 2);
        memory.record(retained, 2);
        memory.record(newest, 2);

        assert_eq!(memory.entries, vec![retained, newest]);
    }

    #[test]
    fn survey_memory_find_returns_freshest_matching_record() {
        let stale = record(1, apple_hypothesis(), false, 600, 10);
        let unrelated = record(2, apple_hypothesis(), true, 900, 30);
        let fresh = record(1, apple_hypothesis(), true, 800, 20);
        let memory = SurveyMemory {
            entries: vec![stale, unrelated, fresh],
        };

        assert_eq!(
            memory.find(entity_id(1, 0), apple_hypothesis()),
            Some(&fresh)
        );
    }

    #[test]
    fn survey_memory_enforce_limits_drops_entries_older_than_retention() {
        let mut memory = SurveyMemory {
            entries: vec![
                record(1, apple_hypothesis(), false, 600, 199),
                record(2, apple_hypothesis(), true, 700, 200),
            ],
        };
        let profile = CognitiveProfile {
            survey_memory_retention_ticks: 100,
            ..CognitiveProfile::default()
        };

        memory.enforce_limits(Tick(300), &profile);

        assert_eq!(
            memory.entries,
            vec![record(2, apple_hypothesis(), true, 700, 200)]
        );
    }

    #[test]
    fn survey_memory_enforce_limits_keeps_entries_within_retention() {
        let retained_boundary = record(1, apple_hypothesis(), false, 600, 200);
        let retained_fresh = record(2, HypothesisKind::MayContainWashBasin, true, 900, 250);
        let mut memory = SurveyMemory {
            entries: vec![retained_boundary, retained_fresh],
        };
        let profile = CognitiveProfile {
            survey_memory_retention_ticks: 100,
            ..CognitiveProfile::default()
        };

        memory.enforce_limits(Tick(300), &profile);

        assert_eq!(memory.entries, vec![retained_boundary, retained_fresh]);
    }

    #[test]
    fn survey_memory_roundtrips_through_bincode() {
        let memory = SurveyMemory {
            entries: vec![
                record(1, apple_hypothesis(), false, 600, 10),
                record(2, HypothesisKind::MayContainSleepSite, true, 900, 20),
            ],
        };

        let bytes = bincode::serialize(&memory).unwrap();
        let roundtrip: SurveyMemory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, memory);
    }
}
