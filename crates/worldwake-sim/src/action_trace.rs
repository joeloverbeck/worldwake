//! Append-only action execution trace for debugging and golden test assertions.
//!
//! Records action lifecycle events (started, committed, aborted, start-failed)
//! during `step_tick()`. Follows the same pattern as `DecisionTraceSink` in
//! `worldwake-ai`.

use crate::{ActionInstanceId, ActionPayload, CommitOutcome, ResolvedRequestTrace};
use std::collections::BTreeMap;
use worldwake_core::{ActionDefId, EntityId, Tick};

/// A single action lifecycle event recorded during `step_tick()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTraceEvent {
    pub tick: Tick,
    pub sequence_in_tick: u32,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub action_name: String,
    pub detail: Option<ActionTraceDetail>,
    pub kind: ActionTraceKind,
}

/// Optional typed detail extracted directly from the action payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionTraceDetail {
    Tell {
        listener: EntityId,
        subject: EntityId,
    },
}

/// The lifecycle transition that this trace event represents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionTraceKind {
    /// Action was successfully started and is now active.
    Started { targets: Vec<EntityId> },
    /// Action completed successfully via handler commit.
    Committed {
        instance_id: ActionInstanceId,
        outcome: CommitOutcome,
    },
    /// Action was aborted, interrupted, or cancelled.
    Aborted {
        instance_id: ActionInstanceId,
        reason: String,
    },
    /// Action start was requested but failed (`BestEffort` mode).
    StartFailed {
        reason: String,
        request: ResolvedRequestTrace,
    },
}

impl ActionTraceEvent {
    #[must_use]
    pub fn new(
        tick: Tick,
        actor: EntityId,
        def_id: ActionDefId,
        action_name: String,
        kind: ActionTraceKind,
    ) -> Self {
        Self {
            tick,
            sequence_in_tick: 0,
            actor,
            def_id,
            action_name,
            detail: None,
            kind,
        }
    }

    #[must_use]
    pub fn with_detail(mut self, detail: Option<ActionTraceDetail>) -> Self {
        self.detail = detail;
        self
    }

    /// One-line human-readable summary (no registry lookups required).
    #[must_use]
    pub fn summary(&self) -> String {
        let detail_suffix = self
            .detail
            .as_ref()
            .map_or_else(String::new, |detail| format!(" [{}]", detail.summary()));
        match &self.kind {
            ActionTraceKind::Started { targets } => {
                format!(
                    "tick {} seq {}: {} started '{}' targeting {:?}{}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.action_name,
                    targets,
                    detail_suffix
                )
            }
            ActionTraceKind::Committed {
                instance_id,
                outcome,
            } => {
                let mat_count = outcome.materializations.len();
                format!(
                    "tick {} seq {}: {} committed '{}' (instance {}, {} materializations){}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.action_name,
                    instance_id,
                    mat_count,
                    detail_suffix
                )
            }
            ActionTraceKind::Aborted {
                instance_id,
                reason,
            } => {
                format!(
                    "tick {} seq {}: {} aborted '{}' (instance {}, reason: {}){}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.action_name,
                    instance_id,
                    reason,
                    detail_suffix
                )
            }
            ActionTraceKind::StartFailed { reason, request } => {
                format!(
                    "tick {} seq {}: {} failed to start '{}' (request#{}, {:?}, {:?}, reason: {}){}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.action_name,
                    request.attempt.input_sequence_no,
                    request.attempt.provenance,
                    request.binding,
                    reason,
                    detail_suffix
                )
            }
        }
    }
}

impl ActionTraceDetail {
    #[must_use]
    pub const fn from_payload(payload: &ActionPayload) -> Option<Self> {
        match payload {
            ActionPayload::Tell(payload) => Some(Self::Tell {
                listener: payload.listener,
                subject: payload.subject_entity,
            }),
            ActionPayload::None
            | ActionPayload::ConsultRecord(_)
            | ActionPayload::Bribe(_)
            | ActionPayload::Threaten(_)
            | ActionPayload::DeclareSupport(_)
            | ActionPayload::Transport(_)
            | ActionPayload::Harvest(_)
            | ActionPayload::Craft(_)
            | ActionPayload::Trade(_)
            | ActionPayload::Combat(_)
            | ActionPayload::Loot(_)
            | ActionPayload::QueueForFacilityUse(_) => None,
        }
    }

    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Tell { listener, subject } => {
                format!("tell listener {listener} subject {subject}")
            }
        }
    }
}

/// Append-only collector for action execution traces.
///
/// Zero-cost when not created. When present, `step_tick()` records action
/// lifecycle events here. Query methods enable structured introspection
/// for debugging and golden test assertions.
pub struct ActionTraceSink {
    events: Vec<ActionTraceEvent>,
    next_sequence_in_tick: BTreeMap<Tick, u32>,
}

impl ActionTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence_in_tick: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, mut event: ActionTraceEvent) {
        let sequence_in_tick = self.next_sequence_in_tick.entry(event.tick).or_insert(0);
        event.sequence_in_tick = *sequence_in_tick;
        *sequence_in_tick = sequence_in_tick
            .checked_add(1)
            .expect("action trace per-tick sequence overflowed");
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[ActionTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn events_for(&self, actor: EntityId) -> Vec<&ActionTraceEvent> {
        self.events.iter().filter(|e| e.actor == actor).collect()
    }

    #[must_use]
    pub fn events_at(&self, tick: Tick) -> Vec<&ActionTraceEvent> {
        self.events.iter().filter(|e| e.tick == tick).collect()
    }

    #[must_use]
    pub fn events_for_at(&self, actor: EntityId, tick: Tick) -> Vec<&ActionTraceEvent> {
        self.events
            .iter()
            .filter(|e| e.actor == actor && e.tick == tick)
            .collect()
    }

    /// Most recent `Committed` event for an actor, if any.
    #[must_use]
    pub fn last_committed(&self, actor: EntityId) -> Option<&ActionTraceEvent> {
        self.events
            .iter()
            .rev()
            .find(|e| e.actor == actor && matches!(e.kind, ActionTraceKind::Committed { .. }))
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.next_sequence_in_tick.clear();
    }

    /// Dump all events for an agent to stderr (for interactive debugging).
    pub fn dump_agent(&self, actor: EntityId) {
        let agent_events = self.events_for(actor);
        if agent_events.is_empty() {
            eprintln!("[ActionTrace] No events for {actor}");
            return;
        }
        eprintln!("[ActionTrace] {} events for {actor}:", agent_events.len());
        for event in agent_events {
            eprintln!("  {}", event.summary());
        }
    }
}

impl Default for ActionTraceSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RequestAttemptTrace, RequestBindingKind, RequestProvenance, ResolvedRequestTrace,
        TellActionPayload,
    };

    const fn sample_request(input_sequence_no: u64) -> ResolvedRequestTrace {
        ResolvedRequestTrace {
            attempt: RequestAttemptTrace {
                input_sequence_no,
                provenance: RequestProvenance::AiPlan,
            },
            binding: RequestBindingKind::ReproducedAffordance,
        }
    }

    fn sample_event(tick: u64, kind: ActionTraceKind) -> ActionTraceEvent {
        ActionTraceEvent::new(
            Tick(tick),
            EntityId {
                slot: 1,
                generation: 0,
            },
            ActionDefId(0),
            "eat".to_string(),
            kind,
        )
    }

    #[test]
    fn sink_starts_empty() {
        let sink = ActionTraceSink::new();
        assert!(sink.events().is_empty());
    }

    #[test]
    fn record_and_query_by_actor() {
        let mut sink = ActionTraceSink::new();
        let actor_a = EntityId {
            slot: 1,
            generation: 0,
        };
        let actor_b = EntityId {
            slot: 2,
            generation: 0,
        };

        sink.record(ActionTraceEvent::new(
            Tick(1),
            actor_a,
            ActionDefId(0),
            "eat".to_string(),
            ActionTraceKind::Started { targets: vec![] },
        ));
        sink.record(ActionTraceEvent::new(
            Tick(1),
            actor_b,
            ActionDefId(1),
            "loot".to_string(),
            ActionTraceKind::Started {
                targets: vec![actor_a],
            },
        ));

        assert_eq!(sink.events_for(actor_a).len(), 1);
        assert_eq!(sink.events_for(actor_b).len(), 1);
        assert_eq!(sink.events().len(), 2);
        assert_eq!(sink.events()[0].sequence_in_tick, 0);
        assert_eq!(sink.events()[1].sequence_in_tick, 1);
    }

    #[test]
    fn query_by_tick() {
        let mut sink = ActionTraceSink::new();
        sink.record(sample_event(
            1,
            ActionTraceKind::Started { targets: vec![] },
        ));
        sink.record(sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        ));

        assert_eq!(sink.events_at(Tick(1)).len(), 1);
        assert_eq!(sink.events_at(Tick(2)).len(), 1);
        assert_eq!(sink.events_at(Tick(3)).len(), 0);
    }

    #[test]
    fn last_committed_returns_most_recent() {
        let mut sink = ActionTraceSink::new();
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        sink.record(ActionTraceEvent::new(
            Tick(1),
            actor,
            ActionDefId(0),
            "eat".to_string(),
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        ));
        sink.record(ActionTraceEvent::new(
            Tick(3),
            actor,
            ActionDefId(1),
            "loot".to_string(),
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(2),
                outcome: CommitOutcome::empty(),
            },
        ));

        let last = sink.last_committed(actor).unwrap();
        assert_eq!(last.action_name, "loot");
        assert_eq!(last.tick, Tick(3));
    }

    #[test]
    fn summary_format_covers_all_variants() {
        let started = sample_event(1, ActionTraceKind::Started { targets: vec![] });
        assert!(started.summary().contains("seq 0"));
        assert!(started.summary().contains("started"));

        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        );
        assert!(committed.summary().contains("committed"));

        let aborted = sample_event(
            3,
            ActionTraceKind::Aborted {
                instance_id: ActionInstanceId(1),
                reason: "test".to_string(),
            },
        );
        assert!(aborted.summary().contains("aborted"));

        let failed = sample_event(
            4,
            ActionTraceKind::StartFailed {
                reason: "precondition".to_string(),
                request: sample_request(9),
            },
        );
        assert!(failed.summary().contains("failed to start"));
        assert!(failed.summary().contains("request#9"));
    }

    #[test]
    fn detail_from_payload_extracts_tell_identity() {
        let listener = EntityId {
            slot: 7,
            generation: 0,
        };
        let subject = EntityId {
            slot: 8,
            generation: 0,
        };

        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            })),
            Some(ActionTraceDetail::Tell { listener, subject })
        );
        assert_eq!(ActionTraceDetail::from_payload(&ActionPayload::None), None);
    }

    #[test]
    fn summary_includes_typed_detail_when_present() {
        let listener = EntityId {
            slot: 7,
            generation: 0,
        };
        let subject = EntityId {
            slot: 8,
            generation: 0,
        };
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        )
        .with_detail(Some(ActionTraceDetail::Tell { listener, subject }));

        let summary = committed.summary();
        assert!(summary.contains("committed"));
        assert!(summary.contains("tell listener"));
        assert!(summary.contains(&listener.to_string()));
        assert!(summary.contains(&subject.to_string()));
    }

    #[test]
    fn clear_removes_all_events() {
        let mut sink = ActionTraceSink::new();
        sink.record(sample_event(
            1,
            ActionTraceKind::Started { targets: vec![] },
        ));
        assert_eq!(sink.events().len(), 1);
        sink.clear();
        assert!(sink.events().is_empty());
    }

    #[test]
    fn record_assigns_explicit_sequence_per_tick_even_when_ticks_interleave() {
        let mut sink = ActionTraceSink::new();
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let other = EntityId {
            slot: 2,
            generation: 0,
        };

        sink.record(ActionTraceEvent::new(
            Tick(1),
            actor,
            ActionDefId(0),
            "eat".to_string(),
            ActionTraceKind::Started { targets: vec![] },
        ));
        sink.record(ActionTraceEvent::new(
            Tick(1),
            other,
            ActionDefId(1),
            "loot".to_string(),
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        ));
        sink.record(ActionTraceEvent::new(
            Tick(2),
            actor,
            ActionDefId(2),
            "rest".to_string(),
            ActionTraceKind::Aborted {
                instance_id: ActionInstanceId(2),
                reason: "test".to_string(),
            },
        ));
        sink.record(ActionTraceEvent::new(
            Tick(1),
            actor,
            ActionDefId(3),
            "craft".to_string(),
            ActionTraceKind::StartFailed {
                reason: "missing tool".to_string(),
                request: sample_request(11),
            },
        ));

        let tick_one = sink.events_at(Tick(1));
        assert_eq!(tick_one.len(), 3);
        assert_eq!(tick_one[0].sequence_in_tick, 0);
        assert_eq!(tick_one[1].sequence_in_tick, 1);
        assert_eq!(tick_one[2].sequence_in_tick, 2);
        assert_eq!(sink.events_at(Tick(2))[0].sequence_in_tick, 0);
    }
}
