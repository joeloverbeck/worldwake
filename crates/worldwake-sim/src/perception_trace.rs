//! Append-only perception trace for debugging institutional belief projection.
//!
//! Records per-observer, per-event outcomes during `perception_system()`:
//! which witnesses passed observation checks, which entity snapshots were
//! recorded, and which institutional claims were projected into belief stores.
//!
//! Follows the same pattern as `ActionTraceSink` and `PoliticalTraceSink` —
//! zero-cost when not created, opt-in per-test.

use std::collections::BTreeMap;
use worldwake_core::{EntityId, EventId, InstitutionalBeliefKey, InstitutionalClaim, Tick};

/// A single perception outcome recorded during `perception_system()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerceptionTraceEvent {
    pub tick: Tick,
    pub sequence_in_tick: u32,
    pub observer: EntityId,
    pub event_id: EventId,
    pub effective_fidelity: u16,
    pub observation_passed: bool,
    /// Entities whose snapshots were recorded by this observer from this event.
    pub entity_observations: Vec<EntityId>,
    /// Institutional claims projected into the observer's belief store.
    pub institutional_claims: Vec<(InstitutionalBeliefKey, InstitutionalClaim)>,
}

impl PerceptionTraceEvent {
    /// One-line human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        let status = if self.observation_passed {
            "passed"
        } else {
            "FAILED"
        };
        format!(
            "tick {} seq {}: {} observed {} ({} @ {}‰), {} entities, {} institutional claims",
            self.tick.0,
            self.sequence_in_tick,
            self.observer,
            self.event_id,
            status,
            self.effective_fidelity,
            self.entity_observations.len(),
            self.institutional_claims.len(),
        )
    }
}

/// Append-only collector for perception trace events.
///
/// Zero-cost when not created. When present, `perception_system()` records
/// observation outcomes here. Query methods enable structured introspection
/// for debugging and golden test assertions.
pub struct PerceptionTraceSink {
    events: Vec<PerceptionTraceEvent>,
    next_sequence_in_tick: BTreeMap<Tick, u32>,
}

impl PerceptionTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence_in_tick: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, mut event: PerceptionTraceEvent) {
        let seq = self.next_sequence_in_tick.entry(event.tick).or_insert(0);
        event.sequence_in_tick = *seq;
        *seq = seq
            .checked_add(1)
            .expect("perception trace per-tick sequence overflowed");
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[PerceptionTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn events_for(&self, observer: EntityId) -> Vec<&PerceptionTraceEvent> {
        self.events
            .iter()
            .filter(|e| e.observer == observer)
            .collect()
    }

    #[must_use]
    pub fn events_at(&self, tick: Tick) -> Vec<&PerceptionTraceEvent> {
        self.events.iter().filter(|e| e.tick == tick).collect()
    }

    #[must_use]
    pub fn events_for_at(&self, observer: EntityId, tick: Tick) -> Vec<&PerceptionTraceEvent> {
        self.events
            .iter()
            .filter(|e| e.observer == observer && e.tick == tick)
            .collect()
    }

    /// All institutional claims recorded for an observer at a given tick.
    #[must_use]
    pub fn claims_at(
        &self,
        observer: EntityId,
        tick: Tick,
    ) -> Vec<&(InstitutionalBeliefKey, InstitutionalClaim)> {
        self.events
            .iter()
            .filter(|e| e.observer == observer && e.tick == tick && e.observation_passed)
            .flat_map(|e| e.institutional_claims.iter())
            .collect()
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.next_sequence_in_tick.clear();
    }

    /// Dump all events for an observer to stderr (for interactive debugging).
    pub fn dump_agent(&self, observer: EntityId) {
        let agent_events = self.events_for(observer);
        if agent_events.is_empty() {
            eprintln!("[PerceptionTrace] No events for {observer}");
            return;
        }
        eprintln!(
            "[PerceptionTrace] {} events for {observer}:",
            agent_events.len()
        );
        for event in agent_events {
            eprintln!("  {}", event.summary());
        }
    }
}

impl Default for PerceptionTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sample_event(tick: u64, observer: EntityId, passed: bool) -> PerceptionTraceEvent {
        PerceptionTraceEvent {
            tick: Tick(tick),
            sequence_in_tick: 0,
            observer,
            event_id: EventId(tick),
            effective_fidelity: if passed { 1000 } else { 0 },
            observation_passed: passed,
            entity_observations: vec![],
            institutional_claims: vec![],
        }
    }

    #[test]
    fn sink_starts_empty() {
        let sink = PerceptionTraceSink::new();
        assert!(sink.events().is_empty());
    }

    #[test]
    fn record_and_query_by_observer() {
        let mut sink = PerceptionTraceSink::new();
        let observer_a = entity(1);
        let observer_b = entity(2);

        sink.record(sample_event(1, observer_a, true));
        sink.record(sample_event(1, observer_b, true));

        assert_eq!(sink.events_for(observer_a).len(), 1);
        assert_eq!(sink.events_for(observer_b).len(), 1);
        assert_eq!(sink.events().len(), 2);
        assert_eq!(sink.events()[0].sequence_in_tick, 0);
        assert_eq!(sink.events()[1].sequence_in_tick, 1);
    }

    #[test]
    fn query_by_tick() {
        let mut sink = PerceptionTraceSink::new();
        sink.record(sample_event(1, entity(1), true));
        sink.record(sample_event(2, entity(1), false));

        assert_eq!(sink.events_at(Tick(1)).len(), 1);
        assert_eq!(sink.events_at(Tick(2)).len(), 1);
        assert_eq!(sink.events_at(Tick(3)).len(), 0);
    }

    #[test]
    fn query_by_observer_and_tick() {
        let mut sink = PerceptionTraceSink::new();
        let a = entity(1);
        let b = entity(2);
        sink.record(sample_event(1, a, true));
        sink.record(sample_event(1, b, true));
        sink.record(sample_event(2, a, false));

        assert_eq!(sink.events_for_at(a, Tick(1)).len(), 1);
        assert_eq!(sink.events_for_at(b, Tick(1)).len(), 1);
        assert_eq!(sink.events_for_at(a, Tick(2)).len(), 1);
        assert_eq!(sink.events_for_at(b, Tick(2)).len(), 0);
    }

    #[test]
    fn claims_at_returns_only_passed_observations() {
        let mut sink = PerceptionTraceSink::new();
        let observer = entity(1);
        let office = entity(10);
        let claim = InstitutionalClaim::OfficeHolder {
            office,
            holder: Some(entity(5)),
            effective_tick: Tick(0),
        };
        let key = InstitutionalBeliefKey::OfficeHolderOf { office };

        // Passed observation with a claim.
        let mut passed = sample_event(1, observer, true);
        passed.institutional_claims.push((key, claim));
        sink.record(passed);

        // Failed observation with a claim — should be excluded.
        let mut failed = sample_event(1, observer, false);
        failed.institutional_claims.push((key, claim));
        sink.record(failed);

        let claims = sink.claims_at(observer, Tick(1));
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].0, key);
    }

    #[test]
    fn clear_removes_all_events() {
        let mut sink = PerceptionTraceSink::new();
        sink.record(sample_event(1, entity(1), true));
        assert_eq!(sink.events().len(), 1);
        sink.clear();
        assert!(sink.events().is_empty());
    }

    #[test]
    fn sequence_numbers_assigned_per_tick() {
        let mut sink = PerceptionTraceSink::new();
        sink.record(sample_event(1, entity(1), true));
        sink.record(sample_event(1, entity(2), true));
        sink.record(sample_event(2, entity(1), false));
        sink.record(sample_event(1, entity(3), true));

        let tick_one = sink.events_at(Tick(1));
        assert_eq!(tick_one.len(), 3);
        assert_eq!(tick_one[0].sequence_in_tick, 0);
        assert_eq!(tick_one[1].sequence_in_tick, 1);
        assert_eq!(tick_one[2].sequence_in_tick, 2);
        assert_eq!(sink.events_at(Tick(2))[0].sequence_in_tick, 0);
    }

    #[test]
    fn summary_format() {
        let mut event = sample_event(3, entity(7), true);
        event.entity_observations = vec![entity(10), entity(11)];
        let summary = event.summary();
        assert!(summary.contains("tick 3"));
        assert!(summary.contains("passed"));
        assert!(summary.contains("@ 1000‰"));
        assert!(summary.contains("2 entities"));
        assert!(summary.contains("0 institutional claims"));

        let failed = sample_event(5, entity(8), false);
        assert!(failed.summary().contains("FAILED"));
    }
}
