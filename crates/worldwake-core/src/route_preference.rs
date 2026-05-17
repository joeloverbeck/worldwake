use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{EventId, RouteSegment, Tick};

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RoutePreference {
    entries: BTreeMap<RouteSegment, RoutePreferenceEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceEntry {
    pub safe_traversals: u32,
    pub dangerous_traversals: u32,
    pub last_safe_tick: Option<Tick>,
    pub last_dangerous_tick: Option<Tick>,
    pub last_traversal_event: Option<EventId>,
}

impl RoutePreference {
    pub fn record_safe(&mut self, segment: RouteSegment, tick: Tick) {
        let entry = self.entry(segment);
        entry.safe_traversals = entry.safe_traversals.saturating_add(1);
        entry.last_safe_tick = Some(tick);
    }

    pub fn record_dangerous(&mut self, segment: RouteSegment, event: EventId, tick: Tick) {
        let entry = self.entry(segment);
        entry.dangerous_traversals = entry.dangerous_traversals.saturating_add(1);
        entry.last_dangerous_tick = Some(tick);
        entry.last_traversal_event = Some(event);
    }

    #[must_use]
    pub fn get(&self, segment: &RouteSegment) -> Option<&RoutePreferenceEntry> {
        self.entries.get(segment)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&RouteSegment, &RoutePreferenceEntry)> {
        self.entries.iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn entry(&mut self, segment: RouteSegment) -> &mut RoutePreferenceEntry {
        self.entries.entry(segment).or_insert(RoutePreferenceEntry {
            safe_traversals: 0,
            dangerous_traversals: 0,
            last_safe_tick: None,
            last_dangerous_tick: None,
            last_traversal_event: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RoutePreference;
    use crate::{EntityId, EventId, RouteSegment, Tick};

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    #[test]
    fn record_safe_and_dangerous_update_counts_and_timestamps() {
        let segment = RouteSegment::new(entity(1), entity(2));
        let mut preference = RoutePreference::default();

        preference.record_safe(segment, Tick(5));
        preference.record_dangerous(segment, EventId(8), Tick(9));

        let entry = preference.get(&segment).expect("entry exists");
        assert_eq!(entry.safe_traversals, 1);
        assert_eq!(entry.dangerous_traversals, 1);
        assert_eq!(entry.last_safe_tick, Some(Tick(5)));
        assert_eq!(entry.last_dangerous_tick, Some(Tick(9)));
        assert_eq!(entry.last_traversal_event, Some(EventId(8)));
    }

    #[test]
    fn route_preference_uses_canonical_route_segment_key() {
        let forward = RouteSegment::new(entity(1), entity(2));
        let reverse = RouteSegment::new(entity(2), entity(1));
        let mut preference = RoutePreference::default();

        preference.record_safe(forward, Tick(5));
        preference.record_dangerous(reverse, EventId(8), Tick(9));

        assert_eq!(preference.iter().count(), 1);
        let entry = preference.get(&forward).expect("entry exists");
        assert_eq!(entry.safe_traversals, 1);
        assert_eq!(entry.dangerous_traversals, 1);
    }

    #[test]
    fn route_preference_bincode_round_trip_preserves_entries() {
        let segment = RouteSegment::new(entity(3), entity(4));
        let mut preference = RoutePreference::default();
        preference.record_safe(segment, Tick(11));

        let encoded = bincode::serialize(&preference).expect("serialize preference");
        let decoded: RoutePreference =
            bincode::deserialize(&encoded).expect("deserialize preference");

        assert_eq!(decoded, preference);
    }
}
