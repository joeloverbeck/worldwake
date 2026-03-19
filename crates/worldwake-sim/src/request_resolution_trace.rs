//! Append-only request-resolution trace for authority-bound request debugging.
//!
//! Records how `tick_step()` resolved an incoming `RequestAction` before the
//! authoritative action start attempt. This complements `ActionTraceSink`
//! rather than replacing it.

use crate::{ActionRequestMode, RequestProvenance};
use std::collections::BTreeMap;
use worldwake_core::{ActionDefId, EntityId, Tick};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestResolutionTraceEvent {
    pub tick: Tick,
    pub sequence_in_tick: u32,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub action_name: String,
    pub requested_targets: Vec<EntityId>,
    pub mode: ActionRequestMode,
    pub provenance: RequestProvenance,
    pub outcome: RequestResolutionOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RequestResolutionOutcome {
    Bound {
        binding: RequestBindingKind,
        resolved_targets: Vec<EntityId>,
        start_attempted: bool,
    },
    RejectedBeforeStart {
        reason: RequestResolutionRejectionReason,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RequestBindingKind {
    ReproducedAffordance,
    BestEffortFallback,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RequestResolutionRejectionReason {
    UnknownActionDef,
    MissingHandler,
    NoMatchingAffordance,
}

impl RequestResolutionTraceEvent {
    #[must_use]
    pub fn summary(&self) -> String {
        match &self.outcome {
            RequestResolutionOutcome::Bound {
                binding,
                resolved_targets,
                start_attempted,
            } => format!(
                "tick {} seq {}: {} request '{}' {:?} via {:?}, binding={binding:?}, requested_targets={:?}, resolved_targets={resolved_targets:?}, start_attempted={start_attempted}",
                self.tick.0,
                self.sequence_in_tick,
                self.actor,
                self.action_name,
                self.provenance,
                self.mode,
                self.requested_targets,
            ),
            RequestResolutionOutcome::RejectedBeforeStart { reason } => format!(
                "tick {} seq {}: {} request '{}' {:?} via {:?} rejected before start: {reason:?}, requested_targets={:?}",
                self.tick.0,
                self.sequence_in_tick,
                self.actor,
                self.action_name,
                self.provenance,
                self.mode,
                self.requested_targets,
            ),
        }
    }
}

pub struct RequestResolutionTraceSink {
    events: Vec<RequestResolutionTraceEvent>,
    next_sequence_in_tick: BTreeMap<Tick, u32>,
}

impl RequestResolutionTraceSink {
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence_in_tick: BTreeMap::new(),
        }
    }

    pub fn record(&mut self, mut event: RequestResolutionTraceEvent) {
        let sequence_in_tick = self.next_sequence_in_tick.entry(event.tick).or_insert(0);
        event.sequence_in_tick = *sequence_in_tick;
        *sequence_in_tick = sequence_in_tick
            .checked_add(1)
            .expect("request-resolution trace per-tick sequence overflowed");
        self.events.push(event);
    }

    #[must_use]
    pub fn events(&self) -> &[RequestResolutionTraceEvent] {
        &self.events
    }

    #[must_use]
    pub fn events_for(&self, actor: EntityId) -> Vec<&RequestResolutionTraceEvent> {
        self.events.iter().filter(|e| e.actor == actor).collect()
    }

    #[must_use]
    pub fn events_at(&self, tick: Tick) -> Vec<&RequestResolutionTraceEvent> {
        self.events.iter().filter(|e| e.tick == tick).collect()
    }

    #[must_use]
    pub fn events_for_at(&self, actor: EntityId, tick: Tick) -> Vec<&RequestResolutionTraceEvent> {
        self.events
            .iter()
            .filter(|e| e.actor == actor && e.tick == tick)
            .collect()
    }
}

impl Default for RequestResolutionTraceSink {
    fn default() -> Self {
        Self::new()
    }
}
