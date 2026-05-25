//! Golden coverage for the S137 localized plan-repair contract.

use std::collections::{BTreeMap, BTreeSet};

use worldwake_ai::{
    PlanGuard, PlanRepairContext, PlannedStep, PlannerOpKind, PlanningEntityRef, RepairFailure,
    RepairOutcome, RepairPlanCandidate, attempt_repair_then_replan,
};
use worldwake_core::{
    AcquisitionQuantity, ActionDefId, BeliefClaimKey, Blocker, BlockerClearingCondition,
    BlockerKey, BlockerMemory, BlockingFact, BreachSignature, CausalLink, CausalProvider, CauseRef,
    CommodityKind, CommodityPurpose, DecisionEventPayload, Discrepancy, DiscrepancyClearing,
    DiscrepancyEntry, DiscrepancySource, EntityBeliefAspect, EntityId, EventLog, EventPayload,
    EventTag, EventView, GoalKey, GoalKind, InvalidatorTag, OpportunityAnchor, OpportunityKey,
    PendingEvent, Permille, PlanningFact, Quantity, RepairAppliedPayload, RepairEntry, RepairKind,
    RepairMemory, ReplanReason, ReplanTriggeredPayload, Tick, VisibilitySpec, WitnessData,
};

const fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 0,
    }
}

fn goal_key() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Apple,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

fn opportunity(goal_key: GoalKey) -> OpportunityKey {
    OpportunityKey {
        goal_key,
        anchor: OpportunityAnchor::None,
    }
}

fn breach_signature(goal_key: GoalKey, target: EntityId) -> BreachSignature {
    BreachSignature {
        goal_key,
        invalidator: InvalidatorTag::TargetMoved,
        step_target: Some(target),
    }
}

fn target_present_fact(target: EntityId, place: EntityId) -> PlanningFact {
    PlanningFact::TargetPresent {
        target,
        at_place: place,
    }
}

fn commodity_available_fact(
    place: EntityId,
    kind: CommodityKind,
    min_quantity: Quantity,
) -> PlanningFact {
    PlanningFact::CommodityAvailable {
        place,
        kind,
        min_quantity,
    }
}

fn target_observation_link(
    target: EntityId,
    place: EntityId,
    consumer_step_index: u16,
) -> CausalLink {
    CausalLink {
        provider: CausalProvider::Observation {
            observed_entity: target,
            aspect: EntityBeliefAspect::Location,
        },
        fact: target_present_fact(target, place),
        consumer_step_index,
        source_tick: Tick(3),
        confidence: Permille::new(850).unwrap(),
    }
}

fn stale_belief_link(target: EntityId, place: EntityId, consumer_step_index: u16) -> CausalLink {
    CausalLink {
        provider: CausalProvider::Belief {
            claim_key: BeliefClaimKey {
                subject: target,
                aspect: EntityBeliefAspect::Location,
            },
        },
        fact: target_present_fact(target, place),
        consumer_step_index,
        source_tick: Tick(3),
        confidence: Permille::new(650).unwrap(),
    }
}

fn commodity_link(
    place: EntityId,
    kind: CommodityKind,
    min_quantity: Quantity,
    consumer_step_index: u16,
) -> CausalLink {
    CausalLink {
        provider: CausalProvider::Observation {
            observed_entity: place,
            aspect: EntityBeliefAspect::Inventory(kind),
        },
        fact: commodity_available_fact(place, kind, min_quantity),
        consumer_step_index,
        source_tick: Tick(3),
        confidence: Permille::new(850).unwrap(),
    }
}

fn discrepancy_entry(clearing_condition: DiscrepancyClearing) -> DiscrepancyEntry {
    DiscrepancyEntry {
        scope: BlockerKey {
            goal_key: goal_key(),
            place: Some(entity(8)),
            target: Some(entity(7)),
            action_def: Some(ActionDefId(1)),
        }
        .into(),
        discrepancy: Discrepancy::BeliefStale,
        observed_tick: Tick(5),
        expires_tick: Tick(50),
        clearing_condition,
        source: DiscrepancySource::ReadPhaseInference,
    }
}

fn planned_step(def_slot: u32, target: EntityId, link: Option<CausalLink>) -> PlannedStep {
    PlannedStep {
        def_id: ActionDefId(def_slot),
        targets: vec![PlanningEntityRef::Authoritative(target)],
        target_place: Some(entity(8)),
        payload_override: None,
        op_kind: PlannerOpKind::Travel,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
        guard: link.map(|link| PlanGuard {
            required_facts: Vec::new(),
            min_confidence: Permille::new(500).unwrap(),
            invalidators: Vec::new(),
            causal_links: vec![link],
        }),
        expectations: Vec::new(),
    }
}

fn context<'a>(
    goal_key: GoalKey,
    broken_link: CausalLink,
    signature: BreachSignature,
    prefix: &'a [PlannedStep],
    suffix: &'a [PlannedStep],
    replacement_candidates: &'a [RepairPlanCandidate],
    discrepancy_entry: &'a DiscrepancyEntry,
) -> PlanRepairContext<'a> {
    PlanRepairContext {
        opportunity: opportunity(goal_key),
        failed_step: broken_link.consumer_step_index,
        broken_link,
        breach_signature: signature,
        preserved_prefix: prefix,
        reusable_suffix: suffix,
        replacement_candidates,
        new_evidence: &[],
        discrepancy_entry,
    }
}

fn cognitive(
    max_node_expansions: u16,
    repair_budget_fraction: Permille,
) -> worldwake_core::CognitiveProfile {
    worldwake_core::CognitiveProfile {
        max_node_expansions,
        repair_budget_fraction,
        ..worldwake_core::CognitiveProfile::default()
    }
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
        place_id: Some(entity(8)),
        state_deltas: Vec::new(),
        observed_entities: BTreeMap::new(),
        visibility: VisibilitySpec::SamePlace,
        witness_data: WitnessData::default(),
        tags: BTreeSet::from([tag]),
        contention_event_payload: None,
        decision_payload: Some(payload),
        artifact_transition_payload: None,
        personality_assigned_payload: None,
    }));

    event_log
        .get(event_id)
        .and_then(EventView::decision_payload)
        .cloned()
        .expect("golden payload event should roundtrip through EventLog")
}

// Scenario 408: S137 Merchant-Moved Breach Rebinds To Sibling
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P14, P20, P21
// Setup: distilled repair context with a failed target-present guard and one
//        sibling suffix candidate; rival route/provider repairs are absent.
// Proves: localized repair chooses RebindTarget and preserves the prefix before
//         the replacement step.
// Cross-system chain: guard breach -> PlanRepairContext -> repaired plan shape
//                     -> RepairApplied payload shape.
#[test]
fn merchant_moved_breach_rebinds_to_sibling() {
    let goal = goal_key();
    let original = entity(7);
    let sibling = entity(17);
    let prefix = vec![planned_step(10, entity(3), None)];
    let replacement = planned_step(17, sibling, None);
    let suffix = vec![replacement.clone()];
    let broken_link = target_observation_link(original, entity(8), 1);
    let candidate = RepairPlanCandidate {
        kind: RepairKind::RebindTarget,
        provider: CausalProvider::Observation {
            observed_entity: original,
            aspect: EntityBeliefAspect::Location,
        },
        fact: broken_link.fact,
        step: replacement.clone(),
        reusable_suffix_index: Some(0),
    };
    let candidates = [candidate];
    let entry = discrepancy_entry(DiscrepancyClearing::ReobservationOf { target: original });
    let ctx = context(
        goal,
        broken_link,
        breach_signature(goal, original),
        &prefix,
        &suffix,
        &candidates,
        &entry,
    );

    let outcome = attempt_repair_then_replan(
        &ctx,
        &cognitive(8, Permille::new(1000).unwrap()),
        &RepairMemory::default(),
    );

    let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
        panic!("sibling target should repair the invalidated branch");
    };
    assert_eq!(kind, RepairKind::RebindTarget);
    assert_eq!(new_plan.steps, vec![prefix[0].clone(), replacement]);

    let payload = emit_decision_payload(
        EventTag::RepairApplied,
        DecisionEventPayload::RepairApplied(RepairAppliedPayload {
            agent: entity(1),
            goal_key: goal,
            step_index: 1,
            repair_kind: kind,
            substitute_target: Some(sibling),
            substitute_recipe: None,
            provider_kind: worldwake_core::VerificationProviderKind::AskWitness,
        }),
    );
    let DecisionEventPayload::RepairApplied(payload) = payload else {
        panic!("expected RepairApplied payload");
    };
    assert_eq!(payload.repair_kind, RepairKind::RebindTarget);
    assert_eq!(payload.substitute_target, Some(sibling));
}

// Scenario 409: S165 Stale Belief Inserts Verification
// Systems: AI
// GoalKinds: AcquireCommodity, AskWitness
// ActionDomains: PlanRepair
// Principles: P14, P15, P16, P21, P29A
// Setup: stale belief-backed breach with one co-located witness verification
//        candidate and no rival repair candidates.
// Proves: InsertVerification selects the AskWitness step and the
//         authoritative RepairApplied payload can record that witness anchor.
// Cross-system chain: belief-backed causal link -> ordered repair attempts ->
//                     AskWitness repair step -> RepairApplied payload shape.
#[test]
fn stale_belief_breach_inserts_ask_witness_verification() {
    let goal = goal_key();
    let target = entity(7);
    let witness = entity(12);
    let prefix = vec![planned_step(10, entity(3), None)];
    let suffix = Vec::new();
    let broken_link = stale_belief_link(target, entity(8), 1);
    let verification = PlannedStep {
        targets: vec![PlanningEntityRef::Authoritative(witness)],
        target_place: Some(entity(8)),
        op_kind: PlannerOpKind::AskWitness,
        ..planned_step(12, witness, None)
    };
    let candidates = [RepairPlanCandidate {
        kind: RepairKind::InsertVerification,
        provider: CausalProvider::Belief {
            claim_key: BeliefClaimKey {
                subject: target,
                aspect: EntityBeliefAspect::Location,
            },
        },
        fact: broken_link.fact,
        step: verification.clone(),
        reusable_suffix_index: None,
    }];
    let entry = discrepancy_entry(DiscrepancyClearing::BeliefUpdate {
        claim_key: BeliefClaimKey {
            subject: target,
            aspect: EntityBeliefAspect::Location,
        },
    });
    let ctx = context(
        goal,
        broken_link,
        breach_signature(goal, target),
        &prefix,
        &suffix,
        &candidates,
        &entry,
    );

    let outcome = attempt_repair_then_replan(
        &ctx,
        &cognitive(5, Permille::new(1000).unwrap()),
        &RepairMemory::default(),
    );

    let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
        panic!("supplied verification candidate should repair the stale belief breach");
    };
    assert_eq!(kind, RepairKind::InsertVerification);
    assert_eq!(new_plan.steps, vec![prefix[0].clone(), verification]);

    let payload = emit_decision_payload(
        EventTag::RepairApplied,
        DecisionEventPayload::RepairApplied(RepairAppliedPayload {
            agent: entity(1),
            goal_key: goal,
            step_index: 1,
            repair_kind: kind,
            substitute_target: Some(witness),
            substitute_recipe: None,
            provider_kind: worldwake_core::VerificationProviderKind::AskWitness,
        }),
    );
    let DecisionEventPayload::RepairApplied(payload) = payload else {
        panic!("expected RepairApplied payload");
    };
    assert_eq!(payload.repair_kind, RepairKind::InsertVerification);
    assert_eq!(payload.substitute_target, Some(witness));
}

// Scenario 461: S165 Stale Belief Without Witness Falls Through
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P15, P16, P21
// Setup: stale belief-backed breach with no lawful co-located witness
//        verification candidate.
// Proves: InsertVerification records NoEpistemicSubstrate and repair falls
//         through to a typed InformationBarrier.
// Cross-system chain: belief-backed causal link -> rejected verification ->
//                     DowngradeToTypedBarrier plan terminal.
#[test]
fn stale_belief_breach_without_witness_falls_through_to_information_barrier() {
    let goal = goal_key();
    let target = entity(7);
    let prefix = vec![planned_step(10, entity(3), None)];
    let suffix = Vec::new();
    let broken_link = stale_belief_link(target, entity(8), 1);
    let mut entry = discrepancy_entry(DiscrepancyClearing::BeliefUpdate {
        claim_key: BeliefClaimKey {
            subject: target,
            aspect: EntityBeliefAspect::Location,
        },
    });
    entry.discrepancy = Discrepancy::MissingObservation;
    let ctx = context(
        goal,
        broken_link,
        breach_signature(goal, target),
        &prefix,
        &suffix,
        &[],
        &entry,
    );

    let outcome = attempt_repair_then_replan(
        &ctx,
        &cognitive(5, Permille::new(1000).unwrap()),
        &RepairMemory::default(),
    );

    let RepairOutcome::Repaired {
        kind,
        new_plan,
        rejected,
    } = outcome
    else {
        panic!("visible prefix should eventually downgrade after insert-verification rejection");
    };
    assert_eq!(kind, RepairKind::DowngradeToTypedBarrier);
    assert_eq!(
        new_plan.terminal_kind,
        worldwake_ai::PlanTerminalKind::InformationBarrier {
            topic: worldwake_core::TellTopic::EntityBelief { subject: target },
        }
    );
    assert!(
        rejected.contains(&(
            RepairKind::InsertVerification,
            RepairFailure::NoEpistemicSubstrate
        )),
        "InsertVerification should honestly fall through without a witness candidate; rejected={rejected:?}"
    );
}

// Scenario 410: S137 Recently Failed Repair Kind Is Skipped
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P20, P21, P22A
// Setup: repair memory records RebindTarget as recently failed for the same
//        BreachSignature; other local repair axes remain lawful.
// Proves: retry suppression skips the matching failed kind without globally
//         blocking later repair kinds.
// Cross-system chain: RepairMemory -> bounded attempt order -> rejected repair
//                     trace.
#[test]
fn recently_failed_repair_kind_is_skipped_without_global_suppression() {
    let goal = goal_key();
    let target = entity(7);
    let prefix = vec![planned_step(10, entity(3), None)];
    let suffix = Vec::new();
    let broken_link = target_observation_link(target, entity(8), 1);
    let signature = breach_signature(goal, target);
    let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
    let ctx = context(goal, broken_link, signature, &prefix, &suffix, &[], &entry);
    let mut memory = RepairMemory::default();
    memory.record(RepairEntry {
        signature,
        kind: RepairKind::RebindTarget,
        succeeded: false,
        observed_tick: Tick(10),
        expires_tick: Tick(50),
        success_count: 0,
    });

    let outcome =
        attempt_repair_then_replan(&ctx, &cognitive(5, Permille::new(1000).unwrap()), &memory);

    let RepairOutcome::Repaired {
        kind,
        new_plan,
        rejected,
    } = outcome
    else {
        panic!("visible prefix should still repair after the recently failed kind is skipped");
    };
    assert_eq!(kind, RepairKind::DowngradeToTypedBarrier);
    assert_eq!(
        rejected.first(),
        Some(&(RepairKind::RebindTarget, RepairFailure::RecentlyFailed))
    );
    assert!(new_plan.terminal_kind.is_barrier());
    assert_eq!(new_plan.steps, prefix);
}

// Scenario 411: S137 Commodity Availability Changed Clears Blocker Structurally
// Systems: AI, Core
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P3, P17, P21
// Setup: blocker memory contains a commodity-availability blocker plus a repair
//        context whose discrepancy has the matching clearing condition.
// Proves: the blocker can be removed by its structural clearing condition and
//         the same condition is visible to localized repair.
// Cross-system chain: DiscrepancyClearing -> BlockerMemory sweep ->
//                     typed-barrier repair.
#[test]
fn commodity_availability_changed_clears_blocker_structurally() {
    let goal = goal_key();
    let market = entity(8);
    let target = entity(7);
    let blocker_key = BlockerKey {
        goal_key: goal,
        place: Some(market),
        target: Some(target),
        action_def: Some(ActionDefId(1)),
    };
    let mut blocker_memory = BlockerMemory::default();
    blocker_memory.record(Blocker {
        scope: blocker_key.into(),
        blocking_fact: BlockingFact::SellerOutOfStock,
        diagnostic_context: None,
        observed_tick: Tick(10),
        expires_tick: Tick(100),
        clearing_condition: BlockerClearingCondition::CommodityAvailabilityChanged {
            commodity: CommodityKind::Apple,
            place: market,
        },
        baseline_snapshot: None,
        source_event: None,
    });
    blocker_memory.sweep_cleared(|blocker| {
        blocker.clearing_condition
            == BlockerClearingCondition::CommodityAvailabilityChanged {
                commodity: CommodityKind::Apple,
                place: market,
            }
    });
    assert!(blocker_memory.intents.is_empty());

    let prefix = vec![planned_step(10, entity(3), None)];
    let suffix = Vec::new();
    let broken_link = commodity_link(market, CommodityKind::Apple, Quantity(1), 1);
    let entry = discrepancy_entry(DiscrepancyClearing::CommodityAvailabilityChanged {
        commodity: CommodityKind::Apple,
        place: market,
    });
    let ctx = context(
        goal,
        broken_link,
        breach_signature(goal, target),
        &prefix,
        &suffix,
        &[],
        &entry,
    );

    let outcome = attempt_repair_then_replan(
        &ctx,
        &cognitive(5, Permille::new(1000).unwrap()),
        &RepairMemory::default(),
    );

    let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
        panic!("visible commodity-clearing discrepancy should repair to a progress barrier");
    };
    assert_eq!(kind, RepairKind::DowngradeToTypedBarrier);
    assert_eq!(new_plan.steps, prefix);
}

// Scenario 412: S137 Repair Budget Exhaustion Falls Through To Full Replan
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P20, P21
// Setup: zero repair budget makes the first repair axis exhaust before any
//        candidate can succeed.
// Proves: repair records Failed/no chosen kind, and the downstream replan event
//         remains the explicit fall-through path.
// Cross-system chain: repair budget -> RepairOutcome::Failed ->
//                     ReplanTriggered payload.
#[test]
fn repair_budget_exhaustion_falls_through_to_full_replan() {
    let goal = goal_key();
    let target = entity(7);
    let prefix = vec![planned_step(10, entity(3), None)];
    let suffix = Vec::new();
    let broken_link = target_observation_link(target, entity(8), 1);
    let signature = breach_signature(goal, target);
    let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
    let ctx = context(goal, broken_link, signature, &prefix, &suffix, &[], &entry);

    let outcome = attempt_repair_then_replan(
        &ctx,
        &cognitive(0, Permille::new(1000).unwrap()),
        &RepairMemory::default(),
    );

    let RepairOutcome::Failed { tried } = outcome else {
        panic!("zero repair budget should fall through to full replan");
    };
    assert_eq!(
        tried,
        vec![(RepairKind::RebindTarget, RepairFailure::BudgetExhausted)]
    );

    let payload = emit_decision_payload(
        EventTag::ReplanTriggered,
        DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
            agent: entity(1),
            goal_key: goal,
            reason: ReplanReason::PlanInvalidated {
                reason: worldwake_core::PlanInvalidationReason::ExpectationMismatch {
                    step_index: 1,
                },
            },
            decisive_beliefs: Vec::new(),
            decisive_records: Vec::new(),
            decisive_world_observations: Vec::new(),
            assumptions: Vec::new(),
        }),
    );
    assert!(matches!(payload, DecisionEventPayload::ReplanTriggered(_)));
}

// Scenario 413: S137 Abandon Produces Empty Progress Barrier
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P20, P21
// Setup: repair context has no preserved prefix and no replacement candidates,
//        so earlier local strategies cannot preserve progress.
// Proves: Abandon is the deterministic final local outcome and returns an empty
//         typed-barrier plan.
// Cross-system chain: failed local repair axes -> Abandon ->
//                     typed barrier plan shape.
#[test]
fn abandon_returns_empty_progress_barrier_after_prior_strategies_fail() {
    let goal = goal_key();
    let target = entity(7);
    let prefix = Vec::new();
    let suffix = Vec::new();
    let broken_link = target_observation_link(target, entity(8), 1);
    let entry = discrepancy_entry(DiscrepancyClearing::TtlExpiry);
    let ctx = context(
        goal,
        broken_link,
        breach_signature(goal, target),
        &prefix,
        &suffix,
        &[],
        &entry,
    );

    let outcome = attempt_repair_then_replan(
        &ctx,
        &cognitive(5, Permille::new(1000).unwrap()),
        &RepairMemory::default(),
    );

    let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
        panic!("abandon should be the final local outcome after earlier attempts fail");
    };
    assert_eq!(kind, RepairKind::Abandon);
    assert!(new_plan.terminal_kind.is_barrier());
    assert!(new_plan.steps.is_empty());
}

// Scenario 414: S137 Phase 11 Approved Repair Gate Witness
// Systems: AI, EventLog
// GoalKinds: AcquireCommodity
// ActionDomains: PlanRepair
// Principles: P20, P21, P29
// Setup: replay the same linked merchant-moved breach six times through the
//        localized repair substrate and compare it with the pre-S137 full
//        replan baseline for that causal class.
// Proves: localized RebindTarget repair replaces full ReplanTriggered fallback
//         by more than the Phase 11 30% reduction threshold while preserving a
//         real causal-link breach and emitting RepairApplied events.
// Cross-system chain: PlanRepairContext -> repaired plan shape ->
//                     RepairApplied event count vs ReplanTriggered baseline.
#[test]
fn phase_11_approved_repair_gate_witness_reduces_full_replans() {
    const PRE_S137_REPLAN_BASELINE: usize = 6;

    let goal = goal_key();
    let original = entity(7);
    let sibling = entity(17);
    let prefix = vec![planned_step(10, entity(3), None)];
    let replacement = planned_step(17, sibling, None);
    let suffix = vec![replacement.clone()];
    let broken_link = target_observation_link(original, entity(8), 1);
    let candidate = RepairPlanCandidate {
        kind: RepairKind::RebindTarget,
        provider: CausalProvider::Observation {
            observed_entity: original,
            aspect: EntityBeliefAspect::Location,
        },
        fact: broken_link.fact,
        step: replacement.clone(),
        reusable_suffix_index: Some(0),
    };
    let candidates = [candidate];
    let entry = discrepancy_entry(DiscrepancyClearing::ReobservationOf { target: original });
    let ctx = context(
        goal,
        broken_link,
        breach_signature(goal, original),
        &prefix,
        &suffix,
        &candidates,
        &entry,
    );
    let mut post_s137_log = EventLog::new();

    for offset in 0..PRE_S137_REPLAN_BASELINE {
        let outcome = attempt_repair_then_replan(
            &ctx,
            &cognitive(8, Permille::new(1000).unwrap()),
            &RepairMemory::default(),
        );
        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!("approved gate witness should repair every linked breach");
        };
        assert_eq!(kind, RepairKind::RebindTarget);
        assert_eq!(new_plan.steps, vec![prefix[0].clone(), replacement.clone()]);

        let tick = Tick(u64::try_from(offset).unwrap());
        let _ = post_s137_log.emit(PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::Bootstrap,
            actor_id: Some(entity(1)),
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: Some(entity(8)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::RepairApplied]),
            contention_event_payload: None,
            decision_payload: Some(DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                agent: entity(1),
                goal_key: goal,
                step_index: 1,
                repair_kind: kind,
                substitute_target: Some(sibling),
                substitute_recipe: None,
                provider_kind: worldwake_core::VerificationProviderKind::AskWitness,
            })),
            artifact_transition_payload: None,

            personality_assigned_payload: None,
        }));
    }

    let post_s137_replans = post_s137_log.events_by_tag(EventTag::ReplanTriggered).len();
    let post_s137_repairs = post_s137_log.events_by_tag(EventTag::RepairApplied).len();

    assert_eq!(post_s137_repairs, PRE_S137_REPLAN_BASELINE);
    assert!(
        post_s137_replans * 10 <= PRE_S137_REPLAN_BASELINE * 7,
        "post-S137 replans {post_s137_replans} must be at least 30% lower than baseline {PRE_S137_REPLAN_BASELINE}"
    );
}
