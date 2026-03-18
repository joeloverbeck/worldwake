use worldwake_ai::DecisionTraceSink;
use worldwake_core::{
    ComponentDelta, ComponentKind, EntityId, EventId, EventLog, EventRecord, EventTag, EventView,
    RelationDelta, StateDelta, Tick,
};
use worldwake_sim::{ActionTraceSink, PoliticalTraceSink};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineLayer {
    Decision,
    Action,
    EventLog,
    Politics,
}

impl TimelineLayer {
    fn label(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Action => "action",
            Self::EventLog => "event",
            Self::Politics => "politics",
        }
    }

    fn sort_order(self) -> u8 {
        match self {
            Self::Decision => 0,
            Self::Action => 1,
            Self::EventLog => 2,
            Self::Politics => 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineEntry {
    pub tick: Tick,
    pub layer: TimelineLayer,
    pub summary: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CrossLayerTimeline {
    entries: Vec<TimelineEntry>,
}

impl CrossLayerTimeline {
    #[must_use]
    pub fn entries(&self) -> &[TimelineEntry] {
        &self.entries
    }

    #[must_use]
    pub fn render(&self) -> String {
        self.entries
            .iter()
            .map(|entry| {
                format!(
                    "[tick {}] {}: {}",
                    entry.tick.0,
                    entry.layer.label(),
                    entry.summary
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub struct CrossLayerTimelineBuilder<'a> {
    event_log: &'a EventLog,
    decision_trace: Option<&'a DecisionTraceSink>,
    action_trace: Option<&'a ActionTraceSink>,
    politics_trace: Option<&'a PoliticalTraceSink>,
    agent: Option<EntityId>,
    office: Option<EntityId>,
    tick_window: Option<(Tick, Tick)>,
}

impl<'a> CrossLayerTimelineBuilder<'a> {
    #[must_use]
    pub fn new(event_log: &'a EventLog) -> Self {
        Self {
            event_log,
            decision_trace: None,
            action_trace: None,
            politics_trace: None,
            agent: None,
            office: None,
            tick_window: None,
        }
    }

    #[must_use]
    pub fn decision_trace(mut self, sink: &'a DecisionTraceSink) -> Self {
        self.decision_trace = Some(sink);
        self
    }

    #[must_use]
    pub fn action_trace(mut self, sink: &'a ActionTraceSink) -> Self {
        self.action_trace = Some(sink);
        self
    }

    #[must_use]
    pub fn politics_trace(mut self, sink: &'a PoliticalTraceSink) -> Self {
        self.politics_trace = Some(sink);
        self
    }

    #[must_use]
    pub fn for_agent(mut self, agent: EntityId) -> Self {
        self.agent = Some(agent);
        self
    }

    #[must_use]
    pub fn for_office(mut self, office: EntityId) -> Self {
        self.office = Some(office);
        self
    }

    #[must_use]
    pub fn tick_window(mut self, start: Tick, end: Tick) -> Self {
        self.tick_window = Some((start, end));
        self
    }

    #[must_use]
    pub fn build(self) -> CrossLayerTimeline {
        self.build_with_event_filter(|_, _| false)
    }

    #[must_use]
    pub fn build_with_event_filter(
        self,
        mut include_event: impl FnMut(EventId, &EventRecord) -> bool,
    ) -> CrossLayerTimeline {
        let mut entries = Vec::new();

        if let Some(sink) = self.decision_trace {
            let traces = if let Some(agent) = self.agent {
                sink.traces_for(agent)
            } else {
                sink.traces().iter().collect()
            };
            for trace in traces {
                if self.contains_tick(trace.tick) {
                    entries.push(TimelineEntry {
                        tick: trace.tick,
                        layer: TimelineLayer::Decision,
                        summary: trace.outcome.summary(),
                    });
                }
            }
        }

        if let Some(sink) = self.action_trace {
            let events = if let Some(agent) = self.agent {
                sink.events_for(agent)
            } else {
                sink.events().iter().collect()
            };
            for event in events {
                if self.contains_tick(event.tick) {
                    entries.push(TimelineEntry {
                        tick: event.tick,
                        layer: TimelineLayer::Action,
                        summary: event.summary(),
                    });
                }
            }
        }

        if let Some(sink) = self.politics_trace {
            let events = if let Some(office) = self.office {
                sink.events_for_office(office)
            } else {
                sink.events().iter().collect()
            };
            for event in events {
                if self.contains_tick(event.tick) {
                    entries.push(TimelineEntry {
                        tick: event.tick,
                        layer: TimelineLayer::Politics,
                        summary: event.summary(),
                    });
                }
            }
        }

        for raw_id in 0..self.event_log.len() {
            let event_id = EventId(raw_id as u64);
            let Some(record) = self.event_log.get(event_id) else {
                continue;
            };
            if !self.contains_tick(record.tick()) || !include_event(event_id, record) {
                continue;
            }
            entries.push(TimelineEntry {
                tick: record.tick(),
                layer: TimelineLayer::EventLog,
                summary: summarize_event_record(event_id, record),
            });
        }

        entries.sort_by_key(|entry| (entry.tick, entry.layer.sort_order(), entry.summary.clone()));

        CrossLayerTimeline { entries }
    }

    fn contains_tick(&self, tick: Tick) -> bool {
        self.tick_window
            .is_none_or(|(start, end)| start <= tick && tick <= end)
    }
}

fn summarize_event_record(event_id: EventId, record: &EventRecord) -> String {
    let tags = summarize_tags(record.tags());
    let deltas = summarize_state_deltas(record.state_deltas());
    if deltas.is_empty() {
        format!("{event_id:?} tags={tags}")
    } else {
        format!("{event_id:?} tags={tags} {}", deltas.join("; "))
    }
}

fn summarize_tags(tags: &std::collections::BTreeSet<EventTag>) -> String {
    if tags.is_empty() {
        return "[]".to_string();
    }
    let joined = tags
        .iter()
        .map(|tag| format!("{tag:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{joined}]")
}

fn summarize_state_deltas(deltas: &[StateDelta]) -> Vec<String> {
    deltas
        .iter()
        .map(|delta| match delta {
            StateDelta::Component(ComponentDelta::Set {
                entity,
                component_kind,
                ..
            }) => {
                format!("set {component_kind:?} on {entity}")
            }
            StateDelta::Component(ComponentDelta::Removed {
                entity,
                component_kind,
                ..
            }) => {
                format!("remove {component_kind:?} from {entity}")
            }
            StateDelta::Relation(RelationDelta::Added { relation, .. }) => {
                format!("add relation {relation:?}")
            }
            StateDelta::Relation(RelationDelta::Removed { relation, .. }) => {
                format!("remove relation {relation:?}")
            }
            other => format!("{other:?}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_ai::{AgentDecisionTrace, DecisionOutcome, DecisionTraceSink};
    use worldwake_core::{
        CauseRef, EventPayload, PendingEvent, SuccessionLaw, VisibilitySpec, WitnessData,
    };
    use worldwake_core::ComponentValue;
    use worldwake_sim::{
        ActionTraceEvent, ActionTraceKind, ActionTraceSink, OfficeSuccessionOutcome,
        OfficeSuccessionTrace, PoliticalTraceEvent, PoliticalTraceSink, SupportDeclarationTrace,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn event_with_tag_and_delta(
        tick: Tick,
        tag: EventTag,
        delta: StateDelta,
    ) -> PendingEvent {
        let mut tags = BTreeSet::new();
        tags.insert(tag);
        PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::Bootstrap,
            actor_id: Some(entity(1)),
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: vec![delta],
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags,
        })
    }

    #[test]
    fn merges_entries_by_tick_and_layer() {
        let agent = entity(1);
        let office = entity(2);

        let mut decision_sink = DecisionTraceSink::new();
        decision_sink.record(AgentDecisionTrace {
            agent,
            tick: Tick(2),
            outcome: DecisionOutcome::Dead,
        });

        let mut action_sink = ActionTraceSink::new();
        action_sink.record(ActionTraceEvent {
            tick: Tick(3),
            actor: agent,
            def_id: worldwake_core::ActionDefId(7),
            action_name: "attack".to_string(),
            kind: ActionTraceKind::Started { targets: vec![office] },
        });

        let mut politics_sink = PoliticalTraceSink::new();
        politics_sink.record(PoliticalTraceEvent {
            tick: Tick(4),
            office,
            trace: OfficeSuccessionTrace {
                jurisdiction: entity(3),
                succession_law: SuccessionLaw::Force,
                holder_before: None,
                vacancy_since_before: Some(Tick(3)),
                outcome: OfficeSuccessionOutcome::VacancyActivated,
                support_declarations: vec![SupportDeclarationTrace {
                    supporter: entity(4),
                    candidate: entity(5),
                    candidate_eligible: false,
                }],
                force_candidates: Vec::new(),
            },
        });

        let mut event_log = EventLog::new();
        let event_id = event_log.emit(event_with_tag_and_delta(
            Tick(3),
            EventTag::Combat,
            StateDelta::Component(ComponentDelta::Set {
                entity: office,
                component_kind: ComponentKind::DeadAt,
                before: None,
                after: ComponentValue::DeadAt(worldwake_core::DeadAt(Tick(3))),
            }),
        ));

        let timeline = CrossLayerTimelineBuilder::new(&event_log)
            .decision_trace(&decision_sink)
            .action_trace(&action_sink)
            .politics_trace(&politics_sink)
            .for_agent(agent)
            .for_office(office)
            .tick_window(Tick(2), Tick(4))
            .build_with_event_filter(|candidate_id, _| candidate_id == event_id);

        let rendered = timeline.render();
        assert_eq!(timeline.entries().len(), 4);
        assert_eq!(timeline.entries()[0].layer, TimelineLayer::Decision);
        assert_eq!(timeline.entries()[1].layer, TimelineLayer::Action);
        assert_eq!(timeline.entries()[2].layer, TimelineLayer::EventLog);
        assert_eq!(timeline.entries()[3].layer, TimelineLayer::Politics);
        assert!(rendered.contains("decision: DEAD"));
        assert!(rendered.contains("action: tick 3"));
        assert!(rendered.contains("event: EventId(0) tags=[Combat] set DeadAt"));
        assert!(rendered.contains("politics: tick 4: office"));
    }

    #[test]
    fn build_requires_explicit_event_filter_for_authoritative_entries() {
        let mut event_log = EventLog::new();
        event_log.emit(event_with_tag_and_delta(
            Tick(1),
            EventTag::Political,
            StateDelta::Component(ComponentDelta::Set {
                entity: entity(9),
                component_kind: ComponentKind::DeadAt,
                before: None,
                after: ComponentValue::DeadAt(worldwake_core::DeadAt(Tick(1))),
            }),
        ));

        let without_events = CrossLayerTimelineBuilder::new(&event_log).build();
        let with_events = CrossLayerTimelineBuilder::new(&event_log)
            .build_with_event_filter(|_, _| true);

        assert!(without_events.entries().is_empty());
        assert_eq!(with_events.entries().len(), 1);
        assert_eq!(with_events.entries()[0].layer, TimelineLayer::EventLog);
    }
}
