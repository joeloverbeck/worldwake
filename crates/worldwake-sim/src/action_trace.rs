//! Append-only action execution trace for debugging and golden test assertions.
//!
//! Records action lifecycle events (started, committed, aborted, start-failed)
//! during `step_tick()`. Follows the same pattern as `DecisionTraceSink` in
//! `worldwake-ai`.

use crate::{
    ActionDef, ActionError, ActionInstanceId, ActionPayload, CommitOutcome, CommitTraceData,
    ResolvedRequestTrace, TellBeliefDeltaKind, TellCommitResult, TellCommitTrace,
};
use std::collections::BTreeMap;
use worldwake_core::{
    ActionDefId, CommodityKind, EntityId, ExpectationId, InstitutionalClaim,
    PunishmentFineStartFailureTrace, PunishmentFineTraceFacts, PunishmentKind, RecordKind,
    TellTopic, Tick, ViolationId, World,
};

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
        topic: TellTopic,
    },
    Investigate {
        violation_id: ViolationId,
    },
    AskWitness {
        target: EntityId,
        topic_entity: Option<EntityId>,
        topic_commodity: Option<CommodityKind>,
    },
    AskAboutPerson {
        target: EntityId,
        subject: EntityId,
    },
    SearchPlace {
        subject: EntityId,
    },
    ReportMissing {
        expectation_id: ExpectationId,
    },
    ReportFound {
        target: EntityId,
        expectation_id: ExpectationId,
    },
    EscortToSafety {
        subject: EntityId,
        destination: EntityId,
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
        legality: Option<ActionStartLegalityTrace>,
    },
}

/// Structured legality provenance for runtime start failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionStartLegalityTrace {
    PunishmentFineStartFailure(PunishmentFineStartFailureTrace),
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

    #[must_use]
    pub fn tell_commit_trace(&self) -> Option<&TellCommitTrace> {
        match &self.kind {
            ActionTraceKind::Committed {
                outcome:
                    CommitOutcome {
                        trace: Some(CommitTraceData::Tell(trace)),
                        ..
                    },
                ..
            } => Some(trace),
            _ => None,
        }
    }

    #[must_use]
    pub fn tell_commit_result(&self) -> Option<TellCommitResult> {
        self.tell_commit_trace().map(|trace| trace.result)
    }

    #[must_use]
    pub fn tell_belief_delta(&self) -> Option<TellBeliefDeltaKind> {
        self.tell_commit_trace().map(|trace| trace.belief_delta)
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
                let commit_trace_suffix =
                    outcome.trace.as_ref().map_or_else(String::new, |trace| {
                        format!(" <{}>", format_commit_trace(trace))
                    });
                format!(
                    "tick {} seq {}: {} committed '{}' (instance {}, {} materializations){}{}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.action_name,
                    instance_id,
                    mat_count,
                    detail_suffix,
                    commit_trace_suffix,
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
            ActionTraceKind::StartFailed {
                reason,
                request,
                legality,
            } => {
                let legality_suffix = legality
                    .as_ref()
                    .map_or_else(String::new, |trace| format!(" <{}>", trace.summary()));
                format!(
                    "tick {} seq {}: {} failed to start '{}' (request#{}, {:?}, {:?}, reason: {}){}{}",
                    self.tick.0,
                    self.sequence_in_tick,
                    self.actor,
                    self.action_name,
                    request.attempt.input_sequence_no,
                    request.attempt.provenance,
                    request.binding,
                    reason,
                    detail_suffix,
                    legality_suffix,
                )
            }
        }
    }
}

impl ActionStartLegalityTrace {
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::PunishmentFineStartFailure(trace) => format!(
                "fine_start_failure office={} accusation_entry={} accused={} actor_place={:?} accused_place={:?} required={} accessible={} total={}",
                trace.facts.office,
                trace.facts.accusation_entry.0,
                trace.facts.accused,
                trace.facts.actor_place,
                trace.facts.accused_place,
                trace.facts.required_amount.0,
                trace.authoritative_accessible_quantity.0,
                trace.authoritative_total_controlled_quantity.0,
            ),
        }
    }
}

#[must_use]
pub fn derive_start_failure_legality_trace(
    actor: EntityId,
    targets: &[EntityId],
    def: &ActionDef,
    payload: &ActionPayload,
    world: &World,
    error: &ActionError,
) -> Option<ActionStartLegalityTrace> {
    let punish = payload.as_punish()?;
    let PunishmentKind::Fine { commodity, amount } = punish.punishment else {
        return None;
    };
    let ActionError::AbortRequested(
        crate::ActionAbortRequestReason::HolderLacksAccessibleCommodity {
            holder,
            commodity: failed_commodity,
            quantity,
        },
    ) = error
    else {
        return None;
    };
    let accused = targets.first().copied()?;
    if def.name != "fine"
        || *holder != accused
        || *failed_commodity != commodity
        || *quantity != amount
    {
        return None;
    }
    let record = locate_crime_register(world, punish.office, world.effective_place(actor)?)?;
    let accusation = active_accusation_for_entry(&record, punish.accusation_entry, accused)?;
    let facts = PunishmentFineTraceFacts {
        office: punish.office,
        accusation_entry: punish.accusation_entry,
        accused,
        theft: accusation.theft,
        actor_place: world.effective_place(actor),
        accused_place: world.effective_place(accused),
        required_amount: amount,
    };
    Some(ActionStartLegalityTrace::PunishmentFineStartFailure(
        PunishmentFineStartFailureTrace {
            facts,
            authoritative_accessible_quantity: world.controlled_commodity_quantity_at_place(
                accused,
                record.home_place,
                commodity,
            ),
            authoritative_total_controlled_quantity: world
                .controlled_commodity_quantity(accused, commodity),
        },
    ))
}

fn locate_crime_register(
    world: &World,
    office: EntityId,
    place: EntityId,
) -> Option<worldwake_core::RecordData> {
    world.query_record_data().find_map(|(_, record)| {
        (record.record_kind == RecordKind::CrimeRegister
            && record.issuer == office
            && record.home_place == place)
            .then_some(record.clone())
    })
}

fn active_accusation_for_entry(
    record: &worldwake_core::RecordData,
    entry_id: worldwake_core::RecordEntryId,
    accused: EntityId,
) -> Option<ActiveAccusationFacts> {
    record
        .active_entries()
        .into_iter()
        .find_map(|entry| match entry.claim {
            InstitutionalClaim::Accusation {
                accused: claim_accused,
                theft,
                ..
            } if entry.entry_id == entry_id && claim_accused == accused => {
                Some(ActiveAccusationFacts { theft })
            }
            _ => None,
        })
}

struct ActiveAccusationFacts {
    theft: worldwake_core::TheftFacts,
}

fn format_commit_trace(trace: &CommitTraceData) -> String {
    match trace {
        CommitTraceData::Tell(tell) => {
            let delta = match tell.belief_delta {
                TellBeliefDeltaKind::None => "no_change".to_string(),
                other => format!("{other:?}"),
            };
            let disposition = tell
                .heard_disposition
                .map_or_else(|| "none".to_string(), |d| format!("{d:?}"));
            let result = match tell.result {
                TellCommitResult::Accepted => "Accepted",
                TellCommitResult::AlreadyHeldEqualOrNewer => "AlreadyHeldEqualOrNewer",
                TellCommitResult::NotInternalized => "NotInternalized",
                TellCommitResult::SpeakerNoLongerKnowsTopic => "SpeakerNoLongerKnowsTopic",
                TellCommitResult::RelayLimitExceeded => "RelayLimitExceeded",
            };
            format!(
                "tell result={result} disposition={disposition} changed={} delta={delta}",
                tell.artifact_changed()
            )
        }
    }
}

impl ActionTraceDetail {
    #[must_use]
    pub const fn from_payload(payload: &ActionPayload) -> Option<Self> {
        match payload {
            ActionPayload::Tell(payload) => Some(Self::Tell {
                listener: payload.listener,
                topic: payload.topic,
            }),
            ActionPayload::Investigate(payload) => Some(Self::Investigate {
                violation_id: payload.violation_id,
            }),
            ActionPayload::AskWitness(payload) => Some(Self::AskWitness {
                target: payload.target,
                topic_entity: payload.topic_entity,
                topic_commodity: payload.topic_commodity,
            }),
            ActionPayload::AskAboutPerson(payload) => Some(Self::AskAboutPerson {
                target: payload.target,
                subject: payload.subject,
            }),
            ActionPayload::SearchPlace(payload) => Some(Self::SearchPlace {
                subject: payload.subject,
            }),
            ActionPayload::ReportMissing(payload) => Some(Self::ReportMissing {
                expectation_id: payload.expectation_id,
            }),
            ActionPayload::ReportFound(payload) => Some(Self::ReportFound {
                target: payload.target,
                expectation_id: payload.expectation_id,
            }),
            ActionPayload::EscortToSafety(payload) => Some(Self::EscortToSafety {
                subject: payload.subject,
                destination: payload.destination,
            }),
            ActionPayload::None
            | ActionPayload::ConsultRecord(_)
            | ActionPayload::Bribe(_)
            | ActionPayload::Threaten(_)
            | ActionPayload::Accuse(_)
            | ActionPayload::Punish(_)
            | ActionPayload::EstablishCamp(_)
            | ActionPayload::DeclareSupport(_)
            | ActionPayload::PressForceClaim(_)
            | ActionPayload::YieldForceClaim(_)
            | ActionPayload::Transport(_)
            | ActionPayload::Harvest(_)
            | ActionPayload::Craft(_)
            | ActionPayload::Trade(_)
            | ActionPayload::Combat(_)
            | ActionPayload::Loot(_)
            | ActionPayload::QueueForFacilityUse(_)
            | ActionPayload::StaffMarket(_)
            | ActionPayload::PostBounty(_)
            | ActionPayload::PostNotice(_) => None,
        }
    }

    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Tell { listener, topic } => {
                format!("tell listener {listener} topic {topic:?}")
            }
            Self::Investigate { violation_id } => {
                format!("investigate violation {}", violation_id.0)
            }
            Self::AskWitness {
                target,
                topic_entity,
                topic_commodity,
            } => {
                format!(
                    "ask_witness target {target} entity {topic_entity:?} commodity {topic_commodity:?}"
                )
            }
            Self::AskAboutPerson { target, subject } => {
                format!("ask_about_person target {target} subject {subject}")
            }
            Self::SearchPlace { subject } => {
                format!("search_place subject {subject}")
            }
            Self::ReportMissing { expectation_id } => {
                format!("report_missing expectation {expectation_id}")
            }
            Self::ReportFound {
                target,
                expectation_id,
            } => {
                format!("report_found target {target} expectation {expectation_id}")
            }
            Self::EscortToSafety {
                subject,
                destination,
            } => {
                format!("escort_to_safety subject {subject} destination {destination}")
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
        ActionAbortRequestReason, AskAboutPersonActionPayload, AskWitnessPayload,
        PunishActionPayload, RequestAttemptTrace, RequestBindingKind, RequestProvenance,
        ResolvedRequestTrace, SearchPlaceActionPayload, TellActionPayload,
    };
    use worldwake_core::{
        CauseRef, CommodityKind, ControlSource, EventLog, InstitutionalClaim,
        InstitutionalRecordEntry, PrototypePlace, PunishmentKind, Quantity, RecordData,
        RecordEntryId, Tick, VisibilitySpec, WitnessData, WorldTxn, build_prototype_world,
        prototype_place_entity,
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
                legality: None,
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
        let topic = TellTopic::EntityBelief {
            subject: EntityId {
                slot: 8,
                generation: 0,
            },
        };

        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::Tell(TellActionPayload {
                listener,
                topic,
            })),
            Some(ActionTraceDetail::Tell { listener, topic })
        );
        assert_eq!(ActionTraceDetail::from_payload(&ActionPayload::None), None);
    }

    #[test]
    fn detail_from_payload_extracts_investigate_identity() {
        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::Investigate(
                crate::InvestigateActionPayload {
                    violation_id: ViolationId(9),
                }
            )),
            Some(ActionTraceDetail::Investigate {
                violation_id: ViolationId(9),
            })
        );
    }

    #[test]
    fn detail_from_payload_extracts_ask_witness_identity() {
        let target = EntityId {
            slot: 7,
            generation: 0,
        };
        let topic_entity = Some(EntityId {
            slot: 8,
            generation: 0,
        });
        let topic_commodity = Some(CommodityKind::Apple);

        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::AskWitness(AskWitnessPayload {
                target,
                topic_entity,
                topic_commodity,
            })),
            Some(ActionTraceDetail::AskWitness {
                target,
                topic_entity,
                topic_commodity,
            })
        );
    }

    #[test]
    fn detail_from_payload_extracts_ask_about_person_identity() {
        let target = EntityId {
            slot: 9,
            generation: 0,
        };
        let subject = EntityId {
            slot: 10,
            generation: 1,
        };

        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::AskAboutPerson(
                AskAboutPersonActionPayload { target, subject }
            )),
            Some(ActionTraceDetail::AskAboutPerson { target, subject })
        );
    }

    #[test]
    fn detail_from_payload_extracts_search_place_identity() {
        let subject = EntityId {
            slot: 11,
            generation: 0,
        };

        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::SearchPlace(
                SearchPlaceActionPayload { subject }
            )),
            Some(ActionTraceDetail::SearchPlace { subject })
        );
    }

    #[test]
    fn detail_from_payload_extracts_report_missing_identity() {
        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::ReportMissing(
                crate::ReportMissingActionPayload {
                    expectation_id: worldwake_core::ExpectationId(9),
                }
            )),
            Some(ActionTraceDetail::ReportMissing {
                expectation_id: worldwake_core::ExpectationId(9),
            })
        );
    }

    #[test]
    fn detail_from_payload_extracts_report_found_identity() {
        let target = EntityId {
            slot: 12,
            generation: 0,
        };
        assert_eq!(
            ActionTraceDetail::from_payload(&ActionPayload::ReportFound(
                crate::ReportFoundActionPayload {
                    target,
                    expectation_id: worldwake_core::ExpectationId(10),
                }
            )),
            Some(ActionTraceDetail::ReportFound {
                target,
                expectation_id: worldwake_core::ExpectationId(10),
            })
        );
    }

    #[test]
    fn summary_includes_typed_detail_when_present() {
        let listener = EntityId {
            slot: 7,
            generation: 0,
        };
        let topic = TellTopic::EntityBelief {
            subject: EntityId {
                slot: 8,
                generation: 0,
            },
        };
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        )
        .with_detail(Some(ActionTraceDetail::Tell { listener, topic }));

        let summary = committed.summary();
        assert!(summary.contains("committed"));
        assert!(summary.contains("tell listener"));
        assert!(summary.contains(&listener.to_string()));
        assert!(summary.contains("EntityBelief"));
    }

    #[test]
    fn summary_includes_ask_about_person_detail_when_present() {
        let target = EntityId {
            slot: 11,
            generation: 0,
        };
        let subject = EntityId {
            slot: 12,
            generation: 0,
        };
        let event = sample_event(
            6,
            ActionTraceKind::Started {
                targets: vec![target],
            },
        )
        .with_detail(Some(ActionTraceDetail::AskAboutPerson { target, subject }));

        assert!(event.summary().contains("ask_about_person"));
        assert!(event.summary().contains("subject"));
    }

    #[test]
    fn summary_includes_search_place_detail_when_present() {
        let subject = EntityId {
            slot: 13,
            generation: 0,
        };
        let event = sample_event(
            7,
            ActionTraceKind::Started {
                targets: vec![EntityId {
                    slot: 14,
                    generation: 0,
                }],
            },
        )
        .with_detail(Some(ActionTraceDetail::SearchPlace { subject }));

        assert!(event.summary().contains("search_place"));
        assert!(event.summary().contains(&subject.to_string()));
    }

    #[test]
    fn summary_includes_tell_commit_trace_when_present() {
        let listener = EntityId {
            slot: 7,
            generation: 0,
        };
        let topic = TellTopic::EntityBelief {
            subject: EntityId {
                slot: 8,
                generation: 0,
            },
        };
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty().with_trace(CommitTraceData::Tell(
                    crate::TellCommitTrace {
                        listener,
                        topic,
                        result: crate::TellCommitResult::AlreadyHeldEqualOrNewer,
                        heard_disposition: Some(
                            worldwake_core::HeardBeliefDisposition::AlreadyHeldEqualOrNewer,
                        ),
                        belief_delta: crate::TellBeliefDeltaKind::None,
                    },
                )),
            },
        );

        let summary = committed.summary();
        assert!(summary.contains("AlreadyHeldEqualOrNewer"));
        assert!(summary.contains("changed=false"));
        assert!(summary.contains("delta=no_change"));
        assert_eq!(
            committed.tell_commit_result(),
            Some(crate::TellCommitResult::AlreadyHeldEqualOrNewer)
        );
        assert_eq!(
            committed.tell_belief_delta(),
            Some(crate::TellBeliefDeltaKind::None)
        );
        assert_eq!(
            committed
                .tell_commit_trace()
                .expect("tell commit trace should be queryable")
                .listener,
            listener
        );
    }

    #[test]
    fn tell_commit_query_helpers_return_none_for_non_tell_commit() {
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        );

        assert!(committed.tell_commit_trace().is_none());
        assert!(committed.tell_commit_result().is_none());
        assert!(committed.tell_belief_delta().is_none());
    }

    #[test]
    fn summary_includes_investigate_detail_when_present() {
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        )
        .with_detail(Some(ActionTraceDetail::Investigate {
            violation_id: ViolationId(11),
        }));

        let summary = committed.summary();
        assert!(summary.contains("committed"));
        assert!(summary.contains("investigate violation 11"));
    }

    #[test]
    fn summary_includes_ask_witness_detail_when_present() {
        let target = EntityId {
            slot: 7,
            generation: 0,
        };
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        )
        .with_detail(Some(ActionTraceDetail::AskWitness {
            target,
            topic_entity: Some(EntityId {
                slot: 8,
                generation: 0,
            }),
            topic_commodity: Some(CommodityKind::Apple),
        }));

        let summary = committed.summary();
        assert!(summary.contains("committed"));
        assert!(summary.contains("ask_witness target"));
        assert!(summary.contains(&target.to_string()));
        assert!(summary.contains("Apple"));
    }

    #[test]
    fn summary_includes_report_missing_detail_when_present() {
        let committed = sample_event(
            2,
            ActionTraceKind::Committed {
                instance_id: ActionInstanceId(1),
                outcome: CommitOutcome::empty(),
            },
        )
        .with_detail(Some(ActionTraceDetail::ReportMissing {
            expectation_id: worldwake_core::ExpectationId(12),
        }));

        let summary = committed.summary();
        assert!(summary.contains("committed"));
        assert!(summary.contains("report_missing expectation exp12"));
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
                legality: None,
            },
        ));

        let tick_one = sink.events_at(Tick(1));
        assert_eq!(tick_one.len(), 3);
        assert_eq!(tick_one[0].sequence_in_tick, 0);
        assert_eq!(tick_one[1].sequence_in_tick, 1);
        assert_eq!(tick_one[2].sequence_in_tick, 2);
        assert_eq!(sink.events_at(Tick(2))[0].sequence_in_tick, 0);
    }

    #[test]
    fn derive_start_failure_legality_trace_records_punishment_quantity_contradiction() {
        let place = prototype_place_entity(PrototypePlace::RulersHall);
        let remote_place = prototype_place_entity(PrototypePlace::GeneralStore);
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let accusation_entry = RecordEntryId(7);
        let (actor, accused, office) = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::ExternalInput(0),
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            let actor = txn.create_agent("Judge", ControlSource::Ai).unwrap();
            let accused = txn.create_agent("Accused", ControlSource::Ai).unwrap();
            let office = txn.create_office("Magistrate").unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(accused, place).unwrap();
            txn.set_component_office_data(
                office,
                worldwake_core::OfficeData {
                    title: "Magistrate".to_string(),
                    seat: place,
                    jurisdiction: std::collections::BTreeSet::from([place]),
                    succession_law: worldwake_core::SuccessionLaw::Support,
                    eligibility_rules: Vec::new(),
                    succession_period_ticks: 8,
                    vacancy_since: None,
                },
            )
            .unwrap();
            let record = txn
                .create_record(RecordData {
                    record_kind: worldwake_core::RecordKind::CrimeRegister,
                    home_place: place,
                    issuer: office,
                    consultation_ticks: 1,
                    max_entries_per_consult: 8,
                    entries: vec![InstitutionalRecordEntry {
                        entry_id: accusation_entry,
                        claim: InstitutionalClaim::Accusation {
                            accuser: actor,
                            accused,
                            violation_id: worldwake_core::ViolationId(2),
                            theft: worldwake_core::TheftFacts {
                                missing_entity: office,
                                expected_place: place,
                                commodity: CommodityKind::Bread,
                                quantity: Quantity(4),
                            },
                            effective_tick: Tick(1),
                        },
                        recorded_tick: Tick(1),
                        supersedes: None,
                    }],
                    next_entry_id: accusation_entry.0 + 1,
                })
                .unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(4))
                .unwrap();
            txn.set_ground_location(lot, remote_place).unwrap();
            txn.set_owner(lot, accused).unwrap();
            let _ = txn.commit(&mut event_log);
            let _ = record;
            (actor, accused, office)
        };

        let def = ActionDef {
            id: ActionDefId(9),
            name: "fine".to_string(),
            domain: worldwake_core::ActionDomain::Generic,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: crate::DurationExpr::Fixed(std::num::NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: crate::Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: std::collections::BTreeSet::new(),
            payload: ActionPayload::None,
            handler: crate::ActionHandlerId(0),
        };
        let payload = ActionPayload::Punish(PunishActionPayload {
            office,
            accusation_entry,
            punishment: PunishmentKind::Fine {
                commodity: CommodityKind::Bread,
                amount: Quantity(2),
            },
        });
        let error =
            ActionError::AbortRequested(ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                holder: accused,
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
            });

        let legality =
            derive_start_failure_legality_trace(actor, &[accused], &def, &payload, &world, &error);

        assert_eq!(
            legality,
            Some(ActionStartLegalityTrace::PunishmentFineStartFailure(
                PunishmentFineStartFailureTrace {
                    facts: PunishmentFineTraceFacts {
                        office,
                        accusation_entry,
                        accused,
                        theft: worldwake_core::TheftFacts {
                            missing_entity: office,
                            expected_place: place,
                            commodity: CommodityKind::Bread,
                            quantity: Quantity(4),
                        },
                        actor_place: Some(place),
                        accused_place: Some(place),
                        required_amount: Quantity(2),
                    },
                    authoritative_accessible_quantity: Quantity(0),
                    authoritative_total_controlled_quantity: Quantity(4),
                }
            ))
        );
    }
}
