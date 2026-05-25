use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{EventId, Permille, RoutePreferenceProfile, RouteSegment, Tick};

const TICKS_PER_DAY: u64 = 1440;

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

impl RoutePreferenceEntry {
    #[must_use]
    pub fn preference(&self, profile: &RoutePreferenceProfile, current_tick: Tick) -> Permille {
        let traversals = self
            .safe_traversals
            .saturating_add(self.dangerous_traversals);
        if traversals < u32::from(profile.minimum_traversals) {
            return Permille::new_unchecked(500);
        }

        let positive =
            i64::from(self.safe_traversals) * i64::from(profile.safe_traversal_weight.value());
        let negative = i64::from(self.dangerous_traversals)
            * i64::from(profile.dangerous_traversal_penalty.value());
        let signal = positive - negative;
        let decayed_signal = apply_route_preference_decay(signal, profile, self, current_tick);

        permille_from_signed(500 + decayed_signal)
    }
}

fn apply_route_preference_decay(
    signal: i64,
    profile: &RoutePreferenceProfile,
    entry: &RoutePreferenceEntry,
    current_tick: Tick,
) -> i64 {
    let Some(last_observation_tick) = latest_observation_tick(entry) else {
        return signal;
    };
    let decay_ticks = u64::from(profile.days_to_decay_observations).saturating_mul(TICKS_PER_DAY);
    if decay_ticks == 0 {
        return 0;
    }
    let age = current_tick.0.saturating_sub(last_observation_tick.0);
    if age >= decay_ticks {
        return 0;
    }
    let remaining = decay_ticks - age;
    signal * i64::try_from(remaining).unwrap_or(i64::MAX)
        / i64::try_from(decay_ticks).unwrap_or(i64::MAX)
}

fn latest_observation_tick(entry: &RoutePreferenceEntry) -> Option<Tick> {
    match (entry.last_safe_tick, entry.last_dangerous_tick) {
        (Some(safe), Some(dangerous)) => Some(if safe.0 >= dangerous.0 {
            safe
        } else {
            dangerous
        }),
        (Some(safe), None) => Some(safe),
        (None, Some(dangerous)) => Some(dangerous),
        (None, None) => None,
    }
}

fn permille_from_signed(value: i64) -> Permille {
    let clamped = u16::try_from(value.clamp(0, 1000)).expect("value clamped to permille range");
    Permille::new_unchecked(clamped)
}

impl RoutePreference {
    pub fn record_safe(&mut self, segment: RouteSegment, event: EventId, tick: Tick) {
        let entry = self.entry(segment);
        entry.safe_traversals = entry.safe_traversals.saturating_add(1);
        entry.last_safe_tick = Some(tick);
        entry.last_traversal_event = Some(event);
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
    use super::{RoutePreference, RoutePreferenceEntry};
    use crate::{EntityId, EventId, Permille, RoutePreferenceProfile, RouteSegment, Tick};

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

        preference.record_safe(segment, EventId(7), Tick(5));
        preference.record_dangerous(segment, EventId(8), Tick(9));

        let entry = preference.get(&segment).expect("entry exists");
        assert_eq!(entry.safe_traversals, 1);
        assert_eq!(entry.dangerous_traversals, 1);
        assert_eq!(entry.last_safe_tick, Some(Tick(5)));
        assert_eq!(entry.last_dangerous_tick, Some(Tick(9)));
        assert_eq!(entry.last_traversal_event, Some(EventId(8)));
    }

    #[test]
    fn record_safe_populates_last_traversal_event() {
        let segment = RouteSegment::new(entity(1), entity(2));
        let mut preference = RoutePreference::default();

        preference.record_safe(segment, EventId(42), Tick(5));

        let entry = preference.get(&segment).expect("entry exists");
        assert_eq!(entry.last_traversal_event, Some(EventId(42)));
    }

    #[test]
    fn route_preference_uses_canonical_route_segment_key() {
        let forward = RouteSegment::new(entity(1), entity(2));
        let reverse = RouteSegment::new(entity(2), entity(1));
        let mut preference = RoutePreference::default();

        preference.record_safe(forward, EventId(7), Tick(5));
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
        preference.record_safe(segment, EventId(11), Tick(11));

        let encoded = bincode::serialize(&preference).expect("serialize preference");
        let decoded: RoutePreference =
            bincode::deserialize(&encoded).expect("deserialize preference");

        assert_eq!(decoded, preference);
    }

    #[test]
    fn preference_is_neutral_below_minimum_traversals() {
        let entry = RoutePreferenceEntry {
            safe_traversals: 1,
            dangerous_traversals: 0,
            last_safe_tick: Some(Tick(1)),
            last_dangerous_tick: None,
            last_traversal_event: None,
        };

        assert_eq!(
            entry.preference(&RoutePreferenceProfile::default(), Tick(1)),
            Permille::new_unchecked(500)
        );
    }

    #[test]
    fn preference_increases_decreases_and_decays_toward_neutral() {
        let profile = RoutePreferenceProfile {
            days_to_decay_observations: 1,
            ..RoutePreferenceProfile::default()
        };
        let preferred = RoutePreferenceEntry {
            safe_traversals: 3,
            dangerous_traversals: 0,
            last_safe_tick: Some(Tick(10)),
            last_dangerous_tick: None,
            last_traversal_event: None,
        };
        let avoided = RoutePreferenceEntry {
            safe_traversals: 0,
            dangerous_traversals: 2,
            last_safe_tick: None,
            last_dangerous_tick: Some(Tick(10)),
            last_traversal_event: Some(EventId(1)),
        };

        assert_eq!(
            preferred.preference(&profile, Tick(10)),
            Permille::new_unchecked(1000)
        );
        assert_eq!(avoided.preference(&profile, Tick(10)), Permille::ZERO);
        assert_eq!(
            preferred.preference(&profile, Tick(10 + 1440)),
            Permille::new_unchecked(500)
        );
    }
}
