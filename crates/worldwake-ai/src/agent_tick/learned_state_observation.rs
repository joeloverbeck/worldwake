use std::collections::BTreeMap;

use worldwake_core::{
    BeliefStoreDiff, ComponentDelta, ComponentDiff, ComponentKind, ComponentValue,
    EntityBeliefClaim, EntityId, EventId, EventLog, EventTag, EventView, PerceptionSource,
    RouteExperience, RouteSegment, StateDelta, TestimonyReliabilityKey, Tick, Topology,
    entity_aspect_to_topic_scope,
};

use crate::AgentDecisionRuntime;

pub(super) fn record_learned_state_updates(
    runtime: &mut AgentDecisionRuntime,
    agent: EntityId,
    event_log: &EventLog,
    topology: &Topology,
    tick: Tick,
) {
    let event_ids: Vec<_> = event_log.events_at_tick(tick).to_vec();
    for event_id in event_ids {
        let Some(record) = event_log.get(event_id) else {
            continue;
        };
        for delta in record.state_deltas() {
            match delta {
                StateDelta::Component(ComponentDelta::CompactSet {
                    entity,
                    component_kind: ComponentKind::AgentBeliefStore,
                    diff: ComponentDiff::BeliefStore(diff),
                }) if *entity == agent => {
                    record_testimony_updates_from_belief_diff(runtime, diff, event_id, tick);
                }
                StateDelta::Component(ComponentDelta::Set {
                    entity,
                    component_kind: ComponentKind::AgentBeliefStore,
                    before: Some(ComponentValue::AgentBeliefStore(before)),
                    after: ComponentValue::AgentBeliefStore(after),
                }) if *entity == agent => {
                    let diff = BeliefStoreDiff::compute(before, after);
                    record_testimony_updates_from_belief_diff(runtime, &diff, event_id, tick);
                }
                StateDelta::Component(ComponentDelta::Set {
                    entity,
                    component_kind: ComponentKind::RouteExperience,
                    before,
                    after: ComponentValue::RouteExperience(after),
                }) if *entity == agent => {
                    let before = before.as_ref().and_then(route_experience_value);
                    record_route_preference_updates(
                        runtime,
                        &RoutePreferenceObservation {
                            topology,
                            event_log,
                            agent,
                            before,
                            after,
                            provenance_event: event_id,
                            tick,
                        },
                    );
                }
                _ => {}
            }
        }
    }
}

struct RoutePreferenceObservation<'a> {
    topology: &'a Topology,
    event_log: &'a EventLog,
    agent: EntityId,
    before: Option<&'a RouteExperience>,
    after: &'a RouteExperience,
    provenance_event: EventId,
    tick: Tick,
}

fn route_experience_value(value: &ComponentValue) -> Option<&RouteExperience> {
    match value {
        ComponentValue::RouteExperience(route) => Some(route),
        _ => None,
    }
}

fn record_testimony_updates_from_belief_diff(
    runtime: &mut AgentDecisionRuntime,
    diff: &BeliefStoreDiff,
    event_id: EventId,
    tick: Tick,
) {
    for (_subject, after_claims) in &diff.entity_claims_set {
        let direct_claims = after_claims
            .iter()
            .filter(|claim| {
                matches!(claim.source, PerceptionSource::DirectObservation)
                    && claim.acquired_tick == tick
            })
            .collect::<Vec<_>>();

        for claim in after_claims.iter().filter_map(report_claim) {
            if claim.refuted_at_tick == Some(tick) {
                if has_direct_claim_for_aspect(&direct_claims, claim, false) {
                    runtime.testimony_reliability.record_refutation(
                        testimony_key(claim),
                        event_id,
                        tick,
                    );
                } else {
                    runtime.testimony_reliability.record_stale(
                        testimony_key(claim),
                        event_id,
                        tick,
                    );
                }
            } else if has_direct_claim_for_aspect(&direct_claims, claim, true) {
                runtime.testimony_reliability.record_confirmation(
                    testimony_key(claim),
                    event_id,
                    tick,
                );
            }
        }

        for loser in report_contradiction_losers(after_claims, tick) {
            runtime.testimony_reliability.record_contradiction(
                testimony_key(loser),
                event_id,
                tick,
            );
        }
    }
}

fn report_claim(claim: &EntityBeliefClaim) -> Option<&EntityBeliefClaim> {
    matches!(claim.source, PerceptionSource::Report { .. }).then_some(claim)
}

fn testimony_key(claim: &EntityBeliefClaim) -> TestimonyReliabilityKey {
    let PerceptionSource::Report { from, .. } = claim.source else {
        unreachable!("testimony key is built only for report claims");
    };
    TestimonyReliabilityKey {
        source: from,
        topic: entity_aspect_to_topic_scope(&claim.aspect),
    }
}

fn has_direct_claim_for_aspect(
    direct_claims: &[&EntityBeliefClaim],
    report: &EntityBeliefClaim,
    same_value: bool,
) -> bool {
    direct_claims.iter().any(|direct| {
        direct.subject == report.subject
            && direct.aspect == report.aspect
            && ((direct.value == report.value) == same_value)
    })
}

fn report_contradiction_losers(
    claims: &[EntityBeliefClaim],
    tick: Tick,
) -> Vec<&EntityBeliefClaim> {
    let mut grouped = BTreeMap::new();
    for claim in claims
        .iter()
        .filter_map(report_claim)
        .filter(|claim| claim.acquired_tick == tick && claim.refuted_at_tick.is_none())
    {
        grouped
            .entry((claim.subject, claim.aspect))
            .or_insert_with(Vec::new)
            .push(claim);
    }

    grouped
        .into_values()
        .filter(|group| has_conflicting_values(group))
        .flat_map(|mut group| {
            group.sort_by_key(|claim| {
                (
                    claim.confidence,
                    claim.claimed_event_tick.unwrap_or(claim.acquired_tick),
                    claim.acquired_tick,
                    claim.claim_id,
                )
            });
            group.pop();
            group
        })
        .collect()
}

fn has_conflicting_values(claims: &[&EntityBeliefClaim]) -> bool {
    claims
        .first()
        .is_some_and(|first| claims.iter().any(|claim| claim.value != first.value))
}

fn record_route_preference_updates(
    runtime: &mut AgentDecisionRuntime,
    observation: &RoutePreferenceObservation<'_>,
) {
    for (edge_id, after_entry) in &observation.after.edges {
        let Some(edge) = observation.topology.edge(*edge_id) else {
            continue;
        };
        let before_entry = observation
            .before
            .and_then(|route| route.edges.get(edge_id));
        let before_safe = before_entry.map_or(0, |entry| entry.safe_trips);
        let before_hostile = before_entry.map_or(0, |entry| entry.hostile_encounters);
        let segment = RouteSegment::new(edge.from(), edge.to());

        for _ in before_safe..after_entry.safe_trips {
            runtime
                .route_preference
                .record_safe(segment, observation.tick);
        }
        for _ in before_hostile..after_entry.hostile_encounters {
            let threat_event = latest_threat_event_for_agent(
                observation.event_log,
                observation.agent,
                observation.tick,
            )
            .unwrap_or(observation.provenance_event);
            runtime
                .route_preference
                .record_dangerous(segment, threat_event, observation.tick);
        }
    }
}

fn latest_threat_event_for_agent(
    event_log: &EventLog,
    agent: EntityId,
    tick: Tick,
) -> Option<EventId> {
    [
        EventTag::Combat,
        EventTag::Escalation,
        EventTag::WildernessRelief,
    ]
    .into_iter()
    .flat_map(|tag| event_log.events_by_tag(tag))
    .copied()
    .filter(|event_id| {
        let Some(record) = event_log.get(*event_id) else {
            return false;
        };
        record.tick() == tick
            && (record.actor_id() == Some(agent) || record.target_ids().contains(&agent))
    })
    .max()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::record_learned_state_updates;
    use crate::AgentDecisionRuntime;
    use worldwake_core::{
        AgentBeliefStore, BeliefConfidencePolicy, CauseRef, ClaimId, ClaimValue, ComponentDelta,
        ComponentKind, ComponentValue, EdgeExperience, EntityBeliefAspect, EntityBeliefClaim,
        EntityId, EventId, EventLog, EventPayload, EventTag, PendingEvent, PerceptionSource,
        Permille, Place, PlaceTag, RouteExperience, RouteSegment, StateDelta, Tick, Topology,
        TravelEdge, TravelEdgeId, VisibilitySpec, WitnessData,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn claim(
        claim_id: u64,
        witness: Option<EntityId>,
        subject: EntityId,
        value: ClaimValue,
        tick: Tick,
    ) -> EntityBeliefClaim {
        EntityBeliefClaim {
            claim_id: ClaimId(claim_id),
            subject,
            aspect: EntityBeliefAspect::Alive,
            value,
            source: witness.map_or(PerceptionSource::DirectObservation, |from| {
                PerceptionSource::Report { from, chain_len: 1 }
            }),
            acquired_tick: tick,
            claimed_event_tick: Some(tick),
            confidence: Permille::new_unchecked(800),
            refuted_at_tick: None,
        }
    }

    fn event_payload(tick: Tick, tags: BTreeSet<EventTag>) -> EventPayload {
        EventPayload {
            tick,
            cause: CauseRef::SystemTick(tick),
            actor_id: None,
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags,
            contention_event_payload: None,
            decision_payload: None,
            artifact_transition_payload: None,
        }
    }

    fn emit_belief_store_set(
        event_log: &mut EventLog,
        agent: EntityId,
        before: AgentBeliefStore,
        after: AgentBeliefStore,
        tick: Tick,
    ) -> EventId {
        let mut payload = event_payload(tick, BTreeSet::from([EventTag::WorldMutation]));
        payload
            .state_deltas
            .push(StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::AgentBeliefStore,
                before: Some(ComponentValue::AgentBeliefStore(before)),
                after: ComponentValue::AgentBeliefStore(after),
            }));
        event_log.emit(PendingEvent::from_payload(payload))
    }

    #[test]
    fn records_testimony_confirmation_refutation_stale_and_contradiction() {
        let agent = entity(1);
        let witness = entity(2);
        let other = entity(3);
        let confirmation_subject = entity(4);
        let refutation_subject = entity(5);
        let stale_subject = entity(6);
        let contradiction_subject = entity(7);
        let tick = Tick(10);
        let mut runtime = AgentDecisionRuntime::default();
        let topology = Topology::new();
        let mut event_log = EventLog::new();

        let mut before = AgentBeliefStore::new();
        before.record_entity_claim(claim(
            1,
            Some(witness),
            confirmation_subject,
            ClaimValue::Bool(true),
            Tick(1),
        ));
        before.record_entity_claim(claim(
            2,
            Some(witness),
            refutation_subject,
            ClaimValue::Bool(false),
            Tick(1),
        ));
        before.record_entity_claim(claim(
            3,
            Some(witness),
            stale_subject,
            ClaimValue::Bool(true),
            Tick(1),
        ));

        let mut after = before.clone();
        after.record_entity_claim(claim(
            4,
            None,
            confirmation_subject,
            ClaimValue::Bool(true),
            tick,
        ));
        after.record_entity_claim(claim(
            5,
            None,
            refutation_subject,
            ClaimValue::Bool(true),
            tick,
        ));
        after.refute_entity_claims(
            worldwake_core::BeliefClaimKey {
                subject: stale_subject,
                aspect: EntityBeliefAspect::Alive,
            },
            tick,
            tick,
            &BeliefConfidencePolicy::default(),
        );
        after.record_entity_claim(claim(
            6,
            Some(witness),
            contradiction_subject,
            ClaimValue::Bool(true),
            tick,
        ));
        after.record_entity_claim(claim(
            7,
            Some(other),
            contradiction_subject,
            ClaimValue::Bool(false),
            tick,
        ));

        let event_id = emit_belief_store_set(&mut event_log, agent, before, after, tick);

        record_learned_state_updates(&mut runtime, agent, &event_log, &topology, tick);

        let witness_entry = runtime
            .testimony_reliability
            .iter()
            .find(|(key, _)| key.source == witness)
            .map(|(_, entry)| entry)
            .expect("witness reliability entry");
        assert_eq!(witness_entry.direct_confirmations, 1);
        assert_eq!(witness_entry.direct_refutations, 1);
        assert_eq!(witness_entry.stale_claims, 1);
        assert_eq!(witness_entry.contradicted_claims, 1);
        assert_eq!(witness_entry.provenance_events.last(), Some(&event_id));
    }

    #[test]
    fn records_safe_and_dangerous_route_preference_from_route_experience_delta() {
        let agent = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let tick = Tick(20);
        let edge_id = TravelEdgeId(7);
        let segment = RouteSegment::new(origin, destination);
        let mut runtime = AgentDecisionRuntime::default();
        let mut topology = Topology::new();
        topology
            .add_place(
                origin,
                Place {
                    name: "Origin".to_string(),
                    capacity: None,
                    tags: BTreeSet::from([PlaceTag::Village]),
                },
            )
            .unwrap();
        topology
            .add_place(
                destination,
                Place {
                    name: "Destination".to_string(),
                    capacity: None,
                    tags: BTreeSet::from([PlaceTag::Forest]),
                },
            )
            .unwrap();
        topology
            .add_edge(TravelEdge::new(edge_id, origin, destination, 3, None).unwrap())
            .unwrap();

        let before = RouteExperience::default();
        let after = RouteExperience {
            edges: BTreeMap::from([(
                edge_id,
                EdgeExperience {
                    safe_trips: 1,
                    hostile_encounters: 1,
                    last_travel_tick: tick,
                },
            )]),
        };
        let mut event_log = EventLog::new();
        let mut threat_payload = event_payload(tick, BTreeSet::from([EventTag::Combat]));
        threat_payload.actor_id = Some(agent);
        let threat_id = event_log.emit(PendingEvent::from_payload(threat_payload));
        let mut route_payload = event_payload(tick, BTreeSet::from([EventTag::WorldMutation]));
        route_payload
            .state_deltas
            .push(StateDelta::Component(ComponentDelta::Set {
                entity: agent,
                component_kind: ComponentKind::RouteExperience,
                before: Some(ComponentValue::RouteExperience(before)),
                after: ComponentValue::RouteExperience(after),
            }));
        event_log.emit(PendingEvent::from_payload(route_payload));

        record_learned_state_updates(&mut runtime, agent, &event_log, &topology, tick);

        let entry = runtime
            .route_preference
            .get(&segment)
            .expect("route preference entry");
        assert_eq!(entry.safe_traversals, 1);
        assert_eq!(entry.dangerous_traversals, 1);
        assert_eq!(entry.last_dangerous_tick, Some(tick));
        assert_eq!(entry.last_traversal_event, Some(threat_id));
    }
}
