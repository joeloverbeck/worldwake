//! Golden coverage for S151 testimony reliability payload contracts.

use std::collections::{BTreeMap, BTreeSet};

use worldwake_core::{
    CauseRef, DecisionEventPayload, EntityId, EventLog, EventPayload, EventTag, EventView, GoalKey,
    GoalKind, GoalRejectionReason, GoalSuppressedPayload, PendingEvent, Permille, TellTopic,
    TestimonyReliability, TestimonyReliabilityKey, TestimonyTrustProfile, TestimonyTrustSummary,
    Tick, TopicScope, VisibilitySpec, WitnessData,
};

const SEEKER: EntityId = entity(1510);
const WITNESS: EntityId = entity(1511);
const SUBJECT: EntityId = entity(1512);
const PLACE: EntityId = entity(1513);

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn profile(minimum_observations: u8) -> TestimonyTrustProfile {
    TestimonyTrustProfile {
        confirmation_weight: Permille::new_unchecked(300),
        refutation_penalty: Permille::new_unchecked(500),
        minimum_observations,
        trust_threshold: Permille::new_unchecked(400),
        topic_weight_route_hazard: Permille::new_unchecked(1000),
        topic_weight_accusation_credibility: Permille::new_unchecked(1000),
        ..TestimonyTrustProfile::default()
    }
}

fn key(topic: TopicScope) -> TestimonyReliabilityKey {
    TestimonyReliabilityKey {
        source: WITNESS,
        topic,
    }
}

fn trust_summary(
    reliability: &TestimonyReliability,
    profile: &TestimonyTrustProfile,
    topic: TopicScope,
) -> TestimonyTrustSummary {
    let entry = reliability
        .get(&key(topic))
        .expect("fixture should have testimony reliability entry");
    TestimonyTrustSummary {
        source: WITNESS,
        topic,
        trust: entry.trust(profile, topic),
        observations: entry.observations(),
    }
}

fn emit_suppressed(summary: TestimonyTrustSummary) -> GoalSuppressedPayload {
    let mut event_log = EventLog::new();
    let event_id = event_log.emit(PendingEvent::from_payload(EventPayload {
        tick: Tick(12),
        cause: CauseRef::Bootstrap,
        actor_id: Some(SEEKER),
        action_name: None,
        target_ids: vec![WITNESS, SUBJECT],
        evidence: Vec::new(),
        place_id: Some(PLACE),
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::SamePlace,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([EventTag::GoalSuppressed]),
        contention_event_payload: None,
        decision_payload: Some(DecisionEventPayload::GoalSuppressed(
            GoalSuppressedPayload {
                agent: SEEKER,
                goal_key: GoalKey::from(GoalKind::AskWitness {
                    witness: WITNESS,
                    topic: TellTopic::EntityBelief { subject: SUBJECT },
                }),
                reason: GoalRejectionReason::SuppressedByUnreliableTestimony,
                testimony_trust_context: vec![summary],
            },
        )),
        artifact_transition_payload: None,
    }));

    let Some(DecisionEventPayload::GoalSuppressed(payload)) = event_log
        .get(event_id)
        .and_then(EventView::decision_payload)
    else {
        panic!("expected suppressed-goal decision payload");
    };
    payload.clone()
}

// Scenario 424: S151 Stale Route-Hazard Refutation Records Trust Context
// Systems: AI, EventLog
// GoalKinds: AskWitness
// ActionDomains: DecisionHistory
// Principles: P15, P16, P29
// Setup: fixture isolates a single witness and route-hazard topic; no rival
//        witness candidate is staged because this scenario proves the emitted
//        suppression payload's testimony context.
// Proves: a direct refutation lowers trust below the suppression threshold and
//         the GoalSuppressed payload preserves the source/topic trust summary.
// Cross-system chain: testimony reliability update -> suppression reason ->
//                     DecisionEventPayload context.
#[test]
fn golden_testimony_reliability_route_hazard_refutation_records_context() {
    let profile = profile(1);
    let mut reliability = TestimonyReliability::default();
    reliability.record_refutation(
        key(TopicScope::RouteHazard),
        worldwake_core::EventId(1),
        Tick(5),
    );

    let summary = trust_summary(&reliability, &profile, TopicScope::RouteHazard);
    assert!(summary.trust < profile.trust_threshold);

    let payload = emit_suppressed(summary);
    assert_eq!(
        payload.reason,
        GoalRejectionReason::SuppressedByUnreliableTestimony
    );
    assert_eq!(payload.testimony_trust_context, vec![summary]);
}

// Scenario 425: S151 Accurate Threat Confirmation Raises Source Trust
// Systems: AI, EventLog
// GoalKinds: AskWitness
// ActionDomains: DecisionHistory
// Principles: P3, P15, P29
// Setup: one direct confirmation is recorded for a route-hazard report; no
//        negative observations are staged, so the trust calculation has a
//        single concrete cause.
// Proves: confirmed testimony raises trust above neutral and the summary is
//         deterministic across repeated derivation.
// Cross-system chain: direct confirmation -> TestimonyReliabilityEntry trust
//                     -> TestimonyTrustSummary.
#[test]
fn golden_testimony_reliability_confirmation_raises_trust_above_neutral() {
    let profile = profile(1);
    let mut reliability = TestimonyReliability::default();
    reliability.record_confirmation(
        key(TopicScope::RouteHazard),
        worldwake_core::EventId(2),
        Tick(6),
    );

    let first = trust_summary(&reliability, &profile, TopicScope::RouteHazard);
    let second = trust_summary(&reliability, &profile, TopicScope::RouteHazard);

    assert!(first.trust > Permille::new_unchecked(500));
    assert_eq!(first, second);
}

// Scenario 426: S151 Repeated False Accusation Crosses Suppression Threshold
// Systems: AI, EventLog
// GoalKinds: AskWitness
// ActionDomains: DecisionHistory
// Principles: P15, P16, P31
// Setup: the same witness is refuted twice for accusation credibility, meeting
//        the minimum-observation gate; unrelated topics are absent so the
//        threshold crossing belongs to this source/topic pair.
// Proves: repeated refutations produce a below-threshold trust summary and the
//         suppression payload carries the accusation topic context.
// Cross-system chain: repeated refutations -> topic-scoped trust -> suppressed
//                     AskWitness payload.
#[test]
fn golden_testimony_reliability_repeated_false_accusation_suppresses_source() {
    let profile = profile(2);
    let mut reliability = TestimonyReliability::default();
    reliability.record_refutation(
        key(TopicScope::AccusationCredibility),
        worldwake_core::EventId(3),
        Tick(7),
    );
    reliability.record_refutation(
        key(TopicScope::AccusationCredibility),
        worldwake_core::EventId(4),
        Tick(8),
    );

    let summary = trust_summary(&reliability, &profile, TopicScope::AccusationCredibility);
    let payload = emit_suppressed(summary);

    assert_eq!(summary.observations, 2);
    assert!(summary.trust < profile.trust_threshold);
    assert_eq!(
        payload.testimony_trust_context[0].topic,
        TopicScope::AccusationCredibility
    );
}
