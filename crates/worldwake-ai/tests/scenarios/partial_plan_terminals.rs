//! Golden contract coverage for S149 typed plan terminals and partial-plan
//! segment carriers.
//!
//! Full autonomous affordance scenarios remain covered by the mechanism-owned
//! suites that create each barrier. This module locks the cross-ticket public
//! carrier contract that D11 consumes: typed terminals produce persisted
//! `PartialPlanSegment`s with the right resume/abandon conditions and survive
//! inside suspended agenda entries.

use std::collections::{BTreeMap, BTreeSet};

use crate::golden_harness::{GoldenHarness, VILLAGE_SQUARE, seed_agent, seed_owner_belief};
use worldwake_ai::htn::{BeliefPredicate, EntityTemplate, PayloadTemplate};
use worldwake_ai::{
    AgendaEntry, AgendaPhase, AgendaState, AgentDecisionRuntime, BarrierFact, FeasibilityHint,
    GoalOffer, GoalPriorityClass, PartialPlanResumeDecision, PartialPlanSegment,
    PartialPlanSegmentSeed, PlanTerminalKind, PlanTerminalKindDiscriminant, PlannedSkeletonStep,
    PlannerOpKind, SkeletonRevalidationReason, SkeletonRevalidationVerdict,
    build_partial_plan_segment, terminal_to_discrepancy, try_resume_partial_plan,
};
use worldwake_core::{
    AcquisitionQuantity, AffordanceKey, BeliefStatusTag, BlockerMemory, BlockerScope,
    CommodityKind, Discrepancy, EntityId, EventId, GoalKey, GoalKind, HomeostaticNeeds,
    IntentionAbandonCondition, IntentionResumeCondition, MetabolismProfile, OpportunityAnchor,
    PerceptionSource, Permille, Seed, Tick, UtilityProfile,
};
use worldwake_sim::PerAgentBeliefView;

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn goal_offer(kind: GoalKind, anchor: OpportunityAnchor) -> GoalOffer {
    GoalOffer {
        key: GoalKey::from(kind),
        anchor,
        evidence_entities: BTreeSet::new(),
        evidence_places: BTreeSet::new(),
        obligation_source: None,
        commitment_impact_if_ignored: Permille::ZERO,
        required_information_gaps: Vec::new(),
        invalidators: Vec::new(),
        learned_expectation_refs: Vec::new(),
        motive_sources: Vec::new(),
        acquisition_quantity: None,
    }
}

fn acquire_goal(commodity: CommodityKind) -> GoalKind {
    GoalKind::AcquireCommodity {
        commodity,
        purpose: worldwake_core::CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    }
}

fn segment(
    terminal_barrier: PlanTerminalKind,
    barrier_fact: BarrierFact,
    local_counter: u16,
) -> PartialPlanSegment {
    build_partial_plan_segment(
        PartialPlanSegmentSeed {
            goal: goal_offer(
                acquire_goal(CommodityKind::Bread),
                OpportunityAnchor::Entity(entity(100 + u32::from(local_counter))),
            ),
            completed_prefix: Vec::new(),
            remaining_skeleton: None,
            terminal_barrier,
            barrier_fact,
            created_tick: Tick(20),
            local_counter,
            causal_links: vec![EventId(u64::from(local_counter))],
        },
        &worldwake_core::CognitiveProfile {
            search_exhaustion_backoff_ticks: 9,
            ..worldwake_core::CognitiveProfile::default()
        },
    )
    .expect("barrier terminal should build a partial-plan segment")
}

fn suspended_entry(mut segment: PartialPlanSegment, anchor_slot: u32) -> AgendaEntry {
    let mut entry = AgendaEntry::pending(
        goal_offer(
            acquire_goal(CommodityKind::Bread),
            OpportunityAnchor::Entity(entity(anchor_slot)),
        ),
        Tick(20),
        GoalPriorityClass::Medium,
        50,
        Vec::new(),
        None,
        None,
        None,
        None,
        None,
        None,
        FeasibilityHint::Uncertain,
    );
    entry.phase = AgendaPhase::Suspended;
    segment.goal = entry.offer.clone();
    entry.partial_plan_segment = Some(segment);
    entry
}

fn skeleton_step(op: PlannerOpKind, expected_pre: Vec<BeliefPredicate>) -> PlannedSkeletonStep {
    PlannedSkeletonStep {
        op,
        target_template: PayloadTemplate::FromContext,
        expected_pre,
    }
}

fn information_barrier_segment_with_skeleton(
    subject: EntityId,
    remaining_skeleton: Vec<PlannedSkeletonStep>,
    local_counter: u16,
) -> PartialPlanSegment {
    let mut segment = segment(
        PlanTerminalKind::InformationBarrier {
            topic: worldwake_core::TellTopic::EntityBelief { subject },
        },
        BarrierFact::MissingBelief(BeliefPredicate::TargetLastSeenKnown {
            target: EntityTemplate::Fixed(subject),
        }),
        local_counter,
    );
    segment.remaining_skeleton = Some(remaining_skeleton);
    segment.resume_conditions = vec![IntentionResumeCondition::BeliefStatusChanged {
        subject,
        target_status: BeliefStatusTag::Certain,
    }];
    segment
}

fn inject_suspended_runtime(harness: &mut GoldenHarness, agent: EntityId, entry: AgendaEntry) {
    harness.driver.set_runtime(
        agent,
        AgentDecisionRuntime {
            agenda_state: AgendaState {
                suspended: BTreeMap::from([(entry.key, entry)]),
                ..AgendaState::default()
            },
            ..AgentDecisionRuntime::default()
        },
    );
}

fn first_partial_plan_resume_trace(
    harness: &GoldenHarness,
    agent: EntityId,
) -> &worldwake_ai::PartialPlanResumeTrace {
    harness
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .traces_for(agent)
        .into_iter()
        .flat_map(|trace| trace.partial_plan_resumes.iter())
        .next()
        .expect("resuming a suspended partial plan should emit a resume trace")
}

fn terminal_cases() -> Vec<(
    PlanTerminalKind,
    BarrierFact,
    PlanTerminalKindDiscriminant,
    Option<Discrepancy>,
    IntentionResumeCondition,
)> {
    let subject = entity(10);
    let resource = entity(20);
    let market = entity(30);
    let authority = entity(40);
    vec![
        (
            PlanTerminalKind::InformationBarrier {
                topic: worldwake_core::TellTopic::EntityBelief { subject },
            },
            BarrierFact::MissingBelief(BeliefPredicate::TargetLastSeenKnown {
                target: EntityTemplate::Fixed(subject),
            }),
            PlanTerminalKindDiscriminant::InformationBarrier,
            Some(Discrepancy::MissingObservation),
            IntentionResumeCondition::BeliefStatusChanged {
                subject,
                target_status: BeliefStatusTag::Certain,
            },
        ),
        (
            PlanTerminalKind::CoordinationBarrier {
                contested_resource: resource,
            },
            BarrierFact::ContestedReservation(resource),
            PlanTerminalKindDiscriminant::CoordinationBarrier,
            None,
            IntentionResumeCondition::ArtifactLegalEffectActive(resource),
        ),
        (
            PlanTerminalKind::ResourceBarrier {
                commodity: CommodityKind::Bread,
                place: market,
            },
            BarrierFact::DepletedResource {
                commodity: CommodityKind::Bread,
                place: market,
            },
            PlanTerminalKindDiscriminant::ResourceBarrier,
            Some(Discrepancy::BeliefStale),
            IntentionResumeCondition::BeliefStatusChanged {
                subject: market,
                target_status: BeliefStatusTag::Certain,
            },
        ),
        (
            PlanTerminalKind::JurisdictionBarrier {
                authority,
                jurisdiction: entity(41),
            },
            BarrierFact::NoAuthorityForAction(authority),
            PlanTerminalKindDiscriminant::JurisdictionBarrier,
            Some(Discrepancy::NoLegalBinding),
            IntentionResumeCondition::ArtifactLegalEffectActive(authority),
        ),
        (
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 7,
                budget_total: 7,
            },
            BarrierFact::BudgetExhausted {
                remaining_stages: 1,
            },
            PlanTerminalKindDiscriminant::SearchBudgetExhausted,
            Some(Discrepancy::SearchBudgetExhausted),
            IntentionResumeCondition::TickElapsed(9),
        ),
    ]
}

// Scenario 439: S149 Typed Terminal Segments Carry Resume And Failure Shape
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity, AskWitness
// ActionDomains: Planning, Agenda
// Principles: P15, P16, P20, P21, P29
// Setup: fixture constructs one segment per live S149 barrier terminal; no
//        autonomous trade, arrest, witness, or facility branches are staged
//        because the mechanism-owned unit and golden suites already exercise
//        those producers.
// Proves: every non-safety S149 barrier terminal maps to the expected
//         discriminant, failure-attribution surface, resume condition, and
//         PatienceExhausted abandon condition.
// Cross-system chain: typed search terminal -> partial-plan segment carrier ->
//                     agenda-runtime resume/abandon contract.
#[test]
fn golden_s149_typed_terminal_segments_carry_resume_and_failure_shape() {
    for (idx, (terminal, fact, discriminant, discrepancy, resume_condition)) in
        terminal_cases().into_iter().enumerate()
    {
        let segment = segment(terminal, fact, idx as u16);

        assert_eq!(
            PlanTerminalKindDiscriminant::from(&segment.terminal_barrier),
            discriminant
        );
        assert_eq!(
            terminal_to_discrepancy(&segment.terminal_barrier),
            discrepancy
        );
        assert_eq!(segment.resume_conditions, vec![resume_condition]);
        assert_eq!(
            segment.abandon_conditions,
            vec![IntentionAbandonCondition::PatienceExhausted]
        );
        assert_eq!(segment.resume_attempt_count, 0);
        assert_eq!(segment.last_resume_attempt_tick, None);

        let decoded: PartialPlanSegment =
            bincode::deserialize(&bincode::serialize(&segment).unwrap()).unwrap();
        assert_eq!(decoded, segment);
    }
}

// Scenario 440: S149 Suspended Agenda Entries Preserve Partial Plan Segments
// Systems: AI, SaveLoad
// GoalKinds: AcquireCommodity
// ActionDomains: Agenda
// Principles: P4, P20, P21, P29
// Setup: fixture suspends one agenda entry for each live S149 barrier terminal;
//        no competing goals are inserted because this scenario proves the
//        per-intention storage boundary, not ranking or branch selection.
// Proves: suspended agenda entries retain their typed partial-plan segments
//         across the serialized agenda-state payload and no shared segment pool
//         is introduced.
// Cross-system chain: partial-plan segment -> suspended AgendaEntry ->
//                     AgentDecisionRuntime save payload shape.
#[test]
fn golden_s149_suspended_agenda_entries_preserve_partial_plan_segments() {
    let mut state = AgendaState::default();
    for (idx, (terminal, fact, _, _, _)) in terminal_cases().into_iter().enumerate() {
        let entry = suspended_entry(segment(terminal, fact, idx as u16), 200 + idx as u32);
        state.suspended.insert(entry.key, entry);
    }

    let decoded: AgendaState = bincode::deserialize(&bincode::serialize(&state).unwrap()).unwrap();

    assert_eq!(decoded, state);
    assert_eq!(decoded.suspended.len(), 5);
    let discriminants = decoded
        .suspended
        .values()
        .map(|entry| {
            PlanTerminalKindDiscriminant::from(
                &entry
                    .partial_plan_segment
                    .as_ref()
                    .expect("suspended entry should carry a segment")
                    .terminal_barrier,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        discriminants,
        BTreeSet::from([
            PlanTerminalKindDiscriminant::InformationBarrier,
            PlanTerminalKindDiscriminant::CoordinationBarrier,
            PlanTerminalKindDiscriminant::ResourceBarrier,
            PlanTerminalKindDiscriminant::JurisdictionBarrier,
            PlanTerminalKindDiscriminant::SearchBudgetExhausted,
        ])
    );
}

// Scenario 441: S149 Partial Plan Resume And Patience Abandon Lifecycle
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: Agenda
// Principles: P20, P21, P29
// Setup: fixture uses a concrete PerAgentBeliefView over a prototype world and
//        a suspended SearchBudgetExhausted segment with a TickElapsed resume
//        condition; no autonomous candidates are staged because the lifecycle
//        boundary under test is agenda resume/abandon.
// Proves: eligible partial-plan segments resume back to Pending with an
//         incremented retry counter, while PatienceExhausted removes an
//         over-limit segment before replaying the stale tail.
// Cross-system chain: suspended AgendaEntry -> RuntimeBeliefView resume check
//                     -> retry/abandon agenda transition.
#[test]
fn golden_s149_partial_plan_resume_and_patience_abandon_lifecycle() {
    let mut harness = GoldenHarness::new(Seed([149; 32]));
    let agent = seed_agent(
        &mut harness.world,
        &mut harness.event_log,
        "S149 partial planner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let belief_view = PerAgentBeliefView::from_world(agent, &harness.world);

    let mut state = AgendaState::default();
    let resumable = suspended_entry(
        segment(
            PlanTerminalKind::SearchBudgetExhausted {
                budget_consumed: 7,
                budget_total: 7,
            },
            BarrierFact::BudgetExhausted {
                remaining_stages: 1,
            },
            1,
        ),
        500,
    );
    let resumable_key = resumable.key;
    state.suspended.insert(resumable_key, resumable);

    let resumed = try_resume_partial_plan(&mut state, agent, &belief_view, Tick(29), 3)
        .expect("elapsed backoff should resume the suspended segment");

    assert_eq!(resumed.key, resumable_key);
    assert_eq!(resumed.entry.phase, AgendaPhase::Pending);
    assert_eq!(resumed.segment.resume_attempt_count, 1);
    assert_eq!(resumed.segment.last_resume_attempt_tick, Some(Tick(29)));
    assert!(state.suspended.is_empty());

    let mut exhausted_segment = segment(
        PlanTerminalKind::SearchBudgetExhausted {
            budget_consumed: 7,
            budget_total: 7,
        },
        BarrierFact::BudgetExhausted {
            remaining_stages: 1,
        },
        2,
    );
    exhausted_segment.resume_attempt_count = 3;
    let exhausted = suspended_entry(exhausted_segment, 501);
    state.suspended.insert(exhausted.key, exhausted);

    let abandoned = try_resume_partial_plan(&mut state, agent, &belief_view, Tick(29), 3);

    assert_eq!(abandoned, None);
    assert!(state.suspended.is_empty());
}

// Scenario 465: S168 Information Barrier Resume Reuses Skeleton
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: Planning, Agenda
// Principles: P12, P20, P21, P29
// Setup: fixture seeds a suspended information-barrier agenda entry with a
//        populated, preservable skeleton and a lawful belief-status update for
//        the barrier subject; no rival agenda entries are inserted.
// Proves: on resume, revalidation marks the skeleton reusable, the pending
//         entry keeps the skeleton for seeded tactical search, and the decision
//         trace records ReusedSeededSearch with the seeded ops.
// Cross-system chain: information-barrier segment -> belief-backed resume gate
//                     -> skeleton revalidation -> decision trace.
#[test]
fn golden_s168_information_barrier_resume_reuses_skeleton() {
    let mut harness = GoldenHarness::new(Seed([168; 32]));
    let agent = seed_agent(
        &mut harness.world,
        &mut harness.event_log,
        "S168 reuse planner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject = entity(900);
    seed_owner_belief(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        subject,
        Some(agent),
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    let skeleton = vec![skeleton_step(PlannerOpKind::Sleep, Vec::new())];
    let entry = suspended_entry(
        information_barrier_segment_with_skeleton(subject, skeleton.clone(), 10),
        910,
    );

    inject_suspended_runtime(&mut harness, agent, entry);
    harness.driver.enable_tracing();
    harness.step_once();

    let trace = first_partial_plan_resume_trace(&harness, agent);
    assert_eq!(
        trace.decision,
        PartialPlanResumeDecision::ReusedSeededSearch
    );
    assert_eq!(
        trace.per_step_verdicts,
        vec![SkeletonRevalidationVerdict::Reusable]
    );
    assert_eq!(trace.seeded_ops, Some(vec![PlannerOpKind::Sleep]));
    let runtime = harness
        .driver
        .runtime(agent)
        .expect("agent runtime should survive the tick");
    let retained_skeleton = runtime
        .agenda_state
        .pending
        .values()
        .chain(runtime.agenda_state.committed.iter())
        .filter_map(|entry| entry.partial_plan_segment.as_ref())
        .find_map(|segment| segment.remaining_skeleton.as_ref());
    assert_eq!(retained_skeleton, Some(&skeleton));
}

// Scenario 466: S168 Information Barrier Resume Falls Back On Invalid Skeleton
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: Planning, Agenda
// Principles: P14, P20, P21, P29
// Setup: fixture satisfies the information-barrier resume gate while leaving a
//        load-bearing seller-known skeleton predicate unknown; no seller or
//        sale-lot belief is present.
// Proves: the resume trace records FallbackToReplanInvalid with the concrete
//         invalidation reason and clears the unusable skeleton before the
//         ordinary pending replan path continues.
// Cross-system chain: information-barrier segment -> belief-backed resume gate
//                     -> skeleton revalidation failure -> full-replan fallback.
#[test]
fn golden_s168_information_barrier_resume_falls_back_when_skeleton_invalid() {
    let mut harness = GoldenHarness::new(Seed([169; 32]));
    let agent = seed_agent(
        &mut harness.world,
        &mut harness.event_log,
        "S168 fallback planner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject = entity(901);
    seed_owner_belief(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        subject,
        Some(agent),
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    let entry = suspended_entry(
        information_barrier_segment_with_skeleton(
            subject,
            vec![skeleton_step(
                PlannerOpKind::Trade,
                vec![BeliefPredicate::SellerKnown {
                    commodity: worldwake_ai::htn::CommodityTemplate::Fixed(CommodityKind::Bread),
                }],
            )],
            11,
        ),
        911,
    );

    inject_suspended_runtime(&mut harness, agent, entry);
    harness.driver.enable_tracing();
    harness.step_once();

    let trace = first_partial_plan_resume_trace(&harness, agent);
    assert_eq!(
        trace.decision,
        PartialPlanResumeDecision::FallbackToReplanInvalid(
            SkeletonRevalidationReason::BeliefUnknown
        )
    );
    assert_eq!(
        trace.per_step_verdicts,
        vec![SkeletonRevalidationVerdict::Invalid(
            SkeletonRevalidationReason::BeliefUnknown
        )]
    );
    assert_eq!(trace.seeded_ops, None);
    let runtime = harness
        .driver
        .runtime(agent)
        .expect("agent runtime should survive the tick");
    assert!(
        runtime
            .agenda_state
            .pending
            .values()
            .chain(runtime.agenda_state.committed.iter())
            .filter_map(|entry| entry.partial_plan_segment.as_ref())
            .all(|segment| segment.remaining_skeleton.is_none()),
        "invalid skeletons must be cleared before ordinary replan"
    );
}

// Scenario 467: S168 Populated Skeleton Survives Save Load Before Resume
// Systems: AI, SaveLoad
// GoalKinds: AcquireCommodity
// ActionDomains: Planning, Agenda
// Principles: P4, P12, P20, P21
// Setup: fixture saves and reloads a harness whose driver runtime contains a
//        suspended information-barrier segment with a populated skeleton before
//        any resume tick runs.
// Proves: the populated skeleton survives the enclosing simulation+runtime
//         save/load boundary and the reloaded runtime emits the same reuse
//         decision on the next tick as the original.
// Cross-system chain: partial-plan segment -> AgentDecisionRuntime save payload
//                     -> SimulationState save/load -> resume trace.
#[test]
fn golden_s168_populated_skeleton_survives_save_load_before_resume() {
    let mut harness = GoldenHarness::new(Seed([170; 32]));
    let agent = seed_agent(
        &mut harness.world,
        &mut harness.event_log,
        "S168 save-load planner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::default(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let subject = entity(902);
    seed_owner_belief(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        subject,
        Some(agent),
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    let skeleton = vec![skeleton_step(PlannerOpKind::Sleep, Vec::new())];
    let entry = suspended_entry(
        information_barrier_segment_with_skeleton(subject, skeleton.clone(), 12),
        912,
    );
    inject_suspended_runtime(&mut harness, agent, entry);

    let mut reloaded = harness.save_load_roundtrip();
    let reloaded_skeleton = reloaded
        .driver
        .runtime(agent)
        .and_then(|runtime| runtime.agenda_state.suspended.values().next())
        .and_then(|entry| entry.partial_plan_segment.as_ref())
        .and_then(|segment| segment.remaining_skeleton.as_ref());
    assert_eq!(reloaded_skeleton, Some(&skeleton));

    harness.driver.enable_tracing();
    reloaded.driver.enable_tracing();
    harness.step_once();
    reloaded.step_once();

    let original_trace = first_partial_plan_resume_trace(&harness, agent);
    let reloaded_trace = first_partial_plan_resume_trace(&reloaded, agent);
    assert_eq!(reloaded_trace.decision, original_trace.decision);
    assert_eq!(
        reloaded_trace.per_step_verdicts,
        original_trace.per_step_verdicts
    );
    assert_eq!(reloaded_trace.seeded_ops, original_trace.seeded_ops);
}

// Scenario 442: S149 Coordination Barrier Uses Blocker Memory
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: Planning, Contention
// Principles: P8, P12, P20, P29
// Setup: fixture isolates the coordination terminal from other barriers and
//        records it through the public blocker-memory helper with a concrete
//        contested affordance.
// Proves: CoordinationBarrier remains a BlockingFact::ReservationConflict
//         path rather than a Discrepancy, preserving the contention-owned
//         failure-attribution surface.
// Cross-system chain: coordination terminal -> blocker-memory record ->
//                     contention clearing condition.
#[test]
fn golden_s149_coordination_barrier_records_blocker_memory_not_discrepancy() {
    let facility = entity(300);
    let terminal = PlanTerminalKind::CoordinationBarrier {
        contested_resource: facility,
    };
    let affordance = AffordanceKey {
        facility,
        action: worldwake_core::ActionDefId(44),
    };
    let mut memory = BlockerMemory::default();
    let recorded = worldwake_ai::record_coordination_barrier_blocker(
        &mut memory,
        &terminal,
        worldwake_ai::CoordinationBarrierBlockerRecord {
            scope: BlockerScope::Exact(worldwake_core::BlockerKey {
                goal_key: GoalKey::from(acquire_goal(CommodityKind::Bread)),
                place: Some(entity(1)),
                target: Some(facility),
                action_def: Some(affordance.action),
            }),
            affordance,
            contention_event: Some(EventId(88)),
            observed_tick: Tick(40),
            source_event: EventId(89),
        },
        &worldwake_core::CognitiveProfile::default(),
    );

    assert!(recorded);
    assert_eq!(terminal_to_discrepancy(&terminal), None);
    let blocker = memory
        .intents
        .values()
        .next()
        .expect("coordination terminal should record one blocker");
    assert!(matches!(
        blocker.blocking_fact,
        worldwake_core::BlockingFact::ReservationConflict { .. }
    ));
    assert!(matches!(
        blocker.clearing_condition,
        worldwake_core::BlockerClearingCondition::ContentionChanged { facility: actual }
            if actual == facility
    ));
    assert_eq!(
        blocker.source,
        worldwake_core::BlockerSource::Event(EventId(89))
    );
    assert!(blocker.expires_tick > Tick(40));
}
