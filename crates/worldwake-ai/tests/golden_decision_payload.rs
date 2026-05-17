use std::collections::{BTreeMap, BTreeSet};

use worldwake_core::{
    AcquisitionQuantity, BeliefClaimKey, BeliefRef, BeliefStatusTag, CauseRef, CommodityKind,
    CommodityPurpose, DecisionEventPayload, EntityBeliefAspect, EntityId, EventLog, EventPayload,
    EventTag, EventView, ExpectationFailureCauseTag, ExpectationFailurePhaseTag,
    ExpectationKindTag, ExpectationMismatchPayload, FrameAssumption, GoalCommittedPayload, GoalKey,
    GoalKind, GoalRejectionReason, HomeostaticNeedId, MismatchDetail, MotiveSource,
    MotiveSourceRef, ObservationRef, OpportunityAnchor, OpportunityExpectationKindTag,
    OpportunityKey, PendingEvent, PlanAssumptionRef, Quantity, RankedGoalComparisonDimensionTag,
    RejectedAlternativeSummary, ReplanReason, ReplanTriggeredPayload, SourceAttributionOutcomeTag,
    SourceExpectationFailurePayload, SourceKeyPayload, StatePredicate, Tick, VisibilitySpec,
    WitnessData,
};

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn consume_goal(commodity: CommodityKind) -> GoalKey {
    GoalKey::from(GoalKind::ConsumeOwnedCommodity { commodity })
}

fn acquire_goal(commodity: CommodityKind) -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

fn emit_decision_payload(tag: EventTag, payload: DecisionEventPayload) -> DecisionEventPayload {
    let mut event_log = EventLog::new();
    let event_id = event_log.emit(PendingEvent::from_payload(EventPayload {
        tick: Tick(42),
        cause: CauseRef::Bootstrap,
        actor_id: Some(entity(1)),
        action_name: None,
        target_ids: Vec::new(),
        evidence: Vec::new(),
        place_id: Some(entity(100)),
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::SamePlace,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([tag]),
        contention_event_payload: None,
        decision_payload: Some(payload),
        artifact_transition_payload: None,
    }));

    event_log
        .get(event_id)
        .and_then(EventView::decision_payload)
        .cloned()
        .expect("golden payload event should roundtrip through EventLog")
}

fn claim(subject: EntityId, aspect: EntityBeliefAspect) -> BeliefClaimKey {
    BeliefClaimKey { subject, aspect }
}

fn need_assumption() -> PlanAssumptionRef {
    PlanAssumptionRef {
        assumption: FrameAssumption::NeedSafeUntilTick {
            need: HomeostaticNeedId::Hunger,
            until_tick: Tick(60),
        },
        introduced_at_step: 0,
    }
}

// Scenario 384: S136 Decision Payload Eat Commitment Records Drink Rejection
// Systems: AI, EventLog
// GoalKinds: ConsumeOwnedCommodity
// ActionDomains: DecisionHistory
// Principles: P3, P20, P21
// Setup: fixture isolates the contested self-care decision payload: no travel,
//        acquisition, or source-failure branch is included because this
//        scenario proves the already-emitted commitment event shape.
// Proves: GoalCommitted payloads preserve the rejected Drink alternative with
//         MotiveScore provenance and the active frame's assumptions.
// Cross-system chain: planner ranking outcome -> DecisionEventPayload ->
//                     append-only event log.
#[test]
fn golden_decision_payload_goal_committed_records_rejected_drink_and_assumptions() {
    let eat = consume_goal(CommodityKind::Bread);
    let drink = consume_goal(CommodityKind::Water);
    let motive_source = MotiveSourceRef {
        source: MotiveSource::NeedPressure {
            need: HomeostaticNeedId::Hunger,
        },
        introduced_tick: Tick(412),
    };
    let payload = emit_decision_payload(
        EventTag::GoalCommitted,
        DecisionEventPayload::GoalCommitted(GoalCommittedPayload {
            agent: entity(1),
            goal_key: eat,
            motive_score: 18420,
            decisive_motive_sources: vec![motive_source.clone()],
            rejected_alternatives: vec![RejectedAlternativeSummary {
                goal_key: drink,
                rejection_reason: GoalRejectionReason::LowerMotive,
                score_gap: 270,
                rejection_dimension: Some(RankedGoalComparisonDimensionTag::MotiveScore),
            }],
            assumptions: vec![need_assumption()],
            testimony_trust_context: Vec::new(),
            route_preference_context: Vec::new(),
        }),
    );

    let DecisionEventPayload::GoalCommitted(payload) = payload else {
        panic!("expected GoalCommitted payload");
    };
    assert_eq!(payload.goal_key, eat);
    let drink_rejection = payload
        .rejected_alternatives
        .iter()
        .find(|alternative| alternative.goal_key == drink)
        .expect("Drink should be recorded as the rejected alternative");
    assert_eq!(drink_rejection.score_gap, 270);
    assert_eq!(
        drink_rejection.rejection_dimension,
        Some(RankedGoalComparisonDimensionTag::MotiveScore)
    );
    assert!(!payload.assumptions.is_empty());
    assert_eq!(payload.decisive_motive_sources, vec![motive_source]);
}

// Scenario 385: S136 Decision Payload Stale-Belief Replan References Claim
// Systems: AI, EventLog
// GoalKinds: ConsumeOwnedCommodity
// ActionDomains: DecisionHistory
// Principles: P15, P16, P21
// Setup: fixture isolates the replan payload after stale-belief detection; no
//        rival goal branches are staged because the contract is the emitted
//        ReplanTriggered causal reference set.
// Proves: ReplanTriggered payloads carry the contradicted stale belief ref and
//         the active frame assumptions.
// Cross-system chain: belief-status invalidation -> DecisionEventPayload ->
//                     append-only event log.
#[test]
fn golden_decision_payload_replan_triggered_records_stale_belief_and_assumptions() {
    let stale_claim = claim(entity(9), EntityBeliefAspect::Location);
    let payload = emit_decision_payload(
        EventTag::ReplanTriggered,
        DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
            agent: entity(1),
            goal_key: consume_goal(CommodityKind::Bread),
            reason: ReplanReason::PlanInvalidated {
                reason: worldwake_core::PlanInvalidationReason::BeliefUpdate {
                    claim_key: stale_claim,
                },
            },
            decisive_beliefs: vec![BeliefRef {
                claim_key: stale_claim,
                claim_held_at_tick: Tick(42),
                status: BeliefStatusTag::Stale,
            }],
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
            assumptions: vec![need_assumption()],
        }),
    );

    let DecisionEventPayload::ReplanTriggered(payload) = payload else {
        panic!("expected ReplanTriggered payload");
    };
    assert_eq!(
        payload.decisive_beliefs,
        vec![BeliefRef {
            claim_key: stale_claim,
            claim_held_at_tick: Tick(42),
            status: BeliefStatusTag::Stale,
        }]
    );
    assert!(!payload.assumptions.is_empty());
    assert!(payload.decisive_records.is_empty());
}

// Scenario 386: S136 Decision Payload Commodity Assumption Breach Records Observation
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: DecisionHistory
// Principles: P7, P17, P21
// Setup: fixture isolates the post-arrival commodity-absence breach; purchase,
//        harvest, and substitute-source branches are excluded so the payload is
//        about the breached CommodityAvailableAt assumption only.
// Proves: ExpectationMismatch payloads carry the breached CommodityAvailableAt
//         assumption and the local observation that contradicted it.
// Cross-system chain: frame assumption evaluation -> DecisionEventPayload ->
//                     append-only event log.
#[test]
fn golden_decision_payload_expectation_mismatch_records_assumption_and_observation() {
    let market = entity(100);
    let goal = acquire_goal(CommodityKind::Apple);
    let assumption = PlanAssumptionRef {
        assumption: FrameAssumption::CommodityAvailableAt {
            commodity: CommodityKind::Apple,
            place: market,
        },
        introduced_at_step: 1,
    };
    let observation = ObservationRef {
        observed_entity: market,
        aspect: EntityBeliefAspect::Inventory(CommodityKind::Apple),
        observed_tick: Tick(42),
    };
    let payload = emit_decision_payload(
        EventTag::ExpectationMismatch,
        DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
            agent: entity(1),
            goal_key: goal,
            step_index: 1,
            expected_materializations: Vec::new(),
            expectation_kind: Some(ExpectationKindTag::State),
            mismatch_detail: Some(MismatchDetail::StateUnmet {
                predicate: StatePredicate::CommodityAtPlaceAtLeast {
                    place: market,
                    kind: CommodityKind::Apple,
                    quantity: Quantity(1),
                },
            }),
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: vec![observation],
            assumptions: vec![assumption],
        }),
    );

    let DecisionEventPayload::ExpectationMismatch(payload) = payload else {
        panic!("expected ExpectationMismatch payload");
    };
    assert_eq!(payload.assumptions, vec![assumption]);
    assert_eq!(payload.decisive_world_observations, vec![observation]);
    assert!(payload.decisive_beliefs.is_empty());
    assert!(payload.decisive_records.is_empty());
}

// Scenario 387: S136 Decision Payload Source Failure Records Source Observation
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: DecisionHistory
// Principles: P7, P15, P29A
// Setup: fixture isolates a concrete-source depletion incident; no belief or
//        record carrier is staged because the live source-expectation seam only
//        carries the source observation address.
// Proves: SourceExpectationFailure payloads carry source-attribution
//         observation refs while belief and record refs remain empty.
// Cross-system chain: source reliability failure -> DecisionEventPayload ->
//                     append-only event log.
#[test]
fn golden_decision_payload_source_expectation_failure_records_source_observation_only() {
    let source = entity(77);
    let observation = ObservationRef {
        observed_entity: source,
        aspect: EntityBeliefAspect::ResourceAvailable(CommodityKind::Apple),
        observed_tick: Tick(42),
    };
    let payload = emit_decision_payload(
        EventTag::SourceExpectationFailure,
        DecisionEventPayload::SourceExpectationFailure(SourceExpectationFailurePayload {
            agent: entity(1),
            opportunity: OpportunityKey {
                goal_key: acquire_goal(CommodityKind::Apple),
                anchor: OpportunityAnchor::Place(entity(100)),
            },
            source: SourceKeyPayload {
                entity: source,
                commodity: CommodityKind::Apple,
            },
            expectation_kind: OpportunityExpectationKindTag::AcquireCommodityFromConcreteSource,
            phase: ExpectationFailurePhaseTag::Observation,
            cause: ExpectationFailureCauseTag::SourceDepletedLocally,
            detected_at_tick: Tick(42),
            attribution_outcome: SourceAttributionOutcomeTag::SourceReliabilityDecremented,
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: vec![observation],
        }),
    );

    let DecisionEventPayload::SourceExpectationFailure(payload) = payload else {
        panic!("expected SourceExpectationFailure payload");
    };
    assert_eq!(payload.decisive_world_observations, vec![observation]);
    assert!(payload.decisive_beliefs.is_empty());
    assert!(payload.decisive_records.is_empty());
}
