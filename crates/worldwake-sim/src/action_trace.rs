//! Append-only action execution trace for debugging and golden test assertions.
//!
//! Records action lifecycle events (started, committed, aborted, start-failed)
//! during `step_tick()`. Follows the same pattern as `DecisionTraceSink` in
//! `worldwake-ai`.

use crate::{ActionInstanceId, CommitOutcome};
use worldwake_core::{ActionDefId, EntityId, Tick};

/// A single action lifecycle event recorded during `step_tick()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTraceEvent {
    pub tick: Tick,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub action_name: String,
    pub kind: ActionTraceKind,
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
    StartFailed { reason: String },
}

impl ActionTraceEvent {
    /// One-line human-readable summary (no registry lookups required).
    #[must_use]
    pub fn summary(&self) -> String {
        match &self.kind {
            ActionTraceKind::Started { targets } => {
                format!(
                    "tick {}: {} started '{}' targeting {:?}",
                    self.tick.0, self.actor, self.action_name, targets
                )
            }
            ActionTraceKind::Committed {
                instance_id,
                outcome,
            } => {
                let mat_count = outcome.materializations.len();
                format!(
                    "tick {}: {} committed '{}' (instance {}, {} materializations)",
                    self.tick.0, self.actor, self.action_name, instance_id, mat_count
                )
            }
            ActionTraceKind::Aborted {
                instance_id,
                reason,
            } => {
                format!(
                    "tick {}: {} aborted '{}' (instance {}, reason: {})",
                    self.tick.0, self.actor, self.action_name, instance_id, reason
                )
            }
            ActionTraceKind::StartFailed { reason } => {
                format!(
                    "tick {}: {} failed to start '{}' (reason: {})",
                    self.tick.0, self.actor, self.action_name, reason
                )
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
}

impl ActionTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: ActionTraceEvent) {
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

    fn sample_event(tick: u64, kind: ActionTraceKind) -> ActionTraceEvent {
        ActionTraceEvent {
            tick: Tick(tick),
            actor: EntityId {
                slot: 1,
                generation: 0,
            },
            def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            kind,
        }
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

        sink.record(ActionTraceEvent {
            tick: Tick(1),
            actor: actor_a,
            def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            kind: ActionTraceKind::Started { targets: vec![] },
        });
        sink.record(ActionTraceEvent {
            tick: Tick(1),
            actor: actor_b,
            def_id: ActionDefId(1),
            action_name: "loot".to_string(),
            kind: ActionTraceKind::Started {
                targets: vec![actor_a],
            },
        });

        assert_eq!(sink.events_for(actor_a).len(), 1);
        assert_eq!(sink.events_for(actor_b).len(), 1);
        assert_eq!(sink.events().len(), 2);
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
        sink.record(ActionTraceEvent {
            tick: Tick(1),
            actor,
            def_id: ActionDefId(0),
            action_name: "eat".to_string(),
            kind: ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        });
        sink.record(ActionTraceEvent {
            tick: Tick(3),
            actor,
            def_id: ActionDefId(1),
            action_name: "loot".to_string(),
            kind: ActionTraceKind::Committed {
                instance_id: ActionInstanceId(2),
                outcome: CommitOutcome::empty(),
            },
        });

        let last = sink.last_committed(actor).unwrap();
        assert_eq!(last.action_name, "loot");
        assert_eq!(last.tick, Tick(3));
    }

    #[test]
    fn summary_format_covers_all_variants() {
        let started = sample_event(
            1,
            ActionTraceKind::Started { targets: vec![] },
        );
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
            },
        );
        assert!(failed.summary().contains("failed to start"));
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
}
