use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    Blocker, BlockerKey, BlockerMemory, BlockingFact, CommodityKind, DecisionEventPayload,
    DiscrepancyMemory, EntityId, EventTag, ExpectationMismatchPayload, GoalKey,
    LearnedOpportunityMemory, PlanInvalidationReason, Quantity, RepairMemory, ReplanReason, Tick,
    UniqueItemKind,
};
use worldwake_sim::{
    ActionStartFailure, CommittedAction, RecipeRegistry, ReplanNeeded, RuntimeBeliefView,
    TickInputError,
};

use crate::candidate_generation::generate_candidates_with_memories_with_travel_horizon;
use crate::failure_handling::ExecutionFailure;
use crate::knowledge_path::KnowledgePath;
use crate::ranking::rank_candidates_with_memories;
use crate::{
    AgentDecisionRuntime, DecisionContext, GoalKindPlannerExt, PlannedStep, RankedGoal,
    authoritative_target, clear_resolved_failures,
};
use worldwake_core::{ContentionIntents, QueuedContentionIntent};

use super::{
    AgentTickContext, advance_completed_step, apply_step_materialization_bindings,
    committed_action_for_step, current_step, emit_decision_event, handle_current_step_failure,
    plan_finished, runtime_belief_view,
};

#[derive(Clone, Copy)]
pub(crate) struct ReadPhaseContext<'a> {
    pub(super) recipe_registry: &'a RecipeRegistry,
    pub(super) utility: &'a worldwake_core::UtilityProfile,
    pub(super) tick: Tick,
    pub(super) travel_horizon: u8,
    pub(super) structural_block_ticks: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct InFlightReconciliation<'a> {
    pub(super) replan_signals: &'a [&'a ReplanNeeded],
    pub(super) start_failures: &'a [ActionStartFailure],
    pub(super) committed_actions: &'a [CommittedAction],
}

#[derive(Clone, Copy)]
pub(crate) struct CompletedPlanSummary {
    pub(super) goal_key: worldwake_core::GoalKey,
    pub(super) terminal_kind: crate::PlanTerminalKind,
    pub(super) step_index: u16,
}

#[derive(Default)]
pub(crate) struct ReconciliationResult {
    pub(super) completed_plan: Option<CompletedPlanSummary>,
    pub(super) plan_invalidation: Option<(GoalKey, PlanInvalidationReason)>,
    pub(super) replan_trigger: Option<(GoalKey, ReplanReason)>,
}

/// Result of the read phase, preserving trace-relevant data alongside ranked candidates.
pub(crate) struct ReadPhaseResult {
    /// Candidate offers emitted during generation in generation order.
    pub(super) offered: Vec<crate::candidate_generation::CandidateOfferDiagnostic>,
    pub(super) ranked: Vec<RankedGoal>,
    /// Generated candidate keys (before ranking filter).
    pub(super) generated_keys: Vec<worldwake_core::OpportunityKey>,
    /// Typed candidate-evidence provenance keyed by generated goal.
    pub(super) candidate_evidence: Vec<crate::CandidateEvidenceTrace>,
    /// Desire-level diagnostics for goals whose emitted sibling opportunities
    /// were all filtered out as blocked before ranking.
    pub(super) fully_blocked_desires: Vec<crate::DesireFullyBlocked>,
    /// Aggregate reachable-place count across acquisition-place searches.
    pub(super) places_reachable: u32,
    /// Aggregate kept-place count after belief gating across acquisition-place searches.
    pub(super) places_after_belief_filter: u32,
    /// Candidate opportunities suppressed before commitment.
    pub(super) suppressed: Vec<crate::candidate_generation::CandidateSuppressionDiagnostic>,
    /// Goals with zero motive score.
    pub(super) zero_motive: Vec<worldwake_core::GoalKey>,
    /// Political goals omitted before emission due to hard gates.
    pub(super) omitted_political: Vec<crate::PoliticalCandidateOmission>,
    /// Bandit goals omitted before emission due to local candidate gates.
    pub(super) omitted_bandit: Vec<crate::BanditCandidateOmission>,
    /// Social goals omitted before emission due to resend suppression.
    pub(super) omitted_social: Vec<crate::SocialCandidateOmission>,
    /// Violation detection pass skipped due to missing prerequisites.
    pub(super) omitted_violation_detection: Vec<crate::ViolationDetectionOmission>,
    /// Shared decision context built once from beliefs for ranking + interrupts.
    pub(super) decision_context: DecisionContext,
    /// When a pursuit plan was invalidated, records the reason.
    pub(super) pursuit_invalidation: Option<crate::PursuitInvalidationReason>,
    /// Need-specific tracker resets detected during candidate generation.
    pub(super) pending_acquisition_exhaustion_resets:
        std::collections::BTreeSet<worldwake_core::HomeostaticNeedId>,
}

fn emit_expectation_mismatch(
    event_log: &mut worldwake_core::EventLog,
    tick: Tick,
    agent: EntityId,
    goal_key: worldwake_core::GoalKey,
    step_index: usize,
    step: &PlannedStep,
) {
    emit_decision_event(
        event_log,
        tick,
        agent,
        EventTag::ExpectationMismatch,
        DecisionEventPayload::ExpectationMismatch(ExpectationMismatchPayload {
            agent,
            goal_key,
            step_index: step_index.try_into().expect("step index exceeds u16"),
            expected_materializations: step
                .expected_materializations
                .iter()
                .map(|expected| expected.tag)
                .collect(),
            expectation_kind: None,
            mismatch_detail: None,
        }),
    );
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn refresh_runtime_for_read_phase(
    world: &worldwake_core::World,
    scheduler: &worldwake_sim::Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    violation_memory: &mut worldwake_core::ViolationMemory,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
    tracing: bool,
) -> ReadPhaseResult {
    let mut discrepancy_memory = DiscrepancyMemory::default();
    refresh_runtime_for_read_phase_with_memories(
        world,
        scheduler,
        action_defs,
        runtime,
        active_goal,
        facility_intents,
        blocked_memory,
        &mut discrepancy_memory,
        violation_memory,
        &RepairMemory::default(),
        &LearnedOpportunityMemory::default(),
        agent,
        replan_signals,
        phase,
        tracing,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refresh_runtime_for_read_phase_with_memories(
    world: &worldwake_core::World,
    scheduler: &worldwake_sim::Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    violation_memory: &mut worldwake_core::ViolationMemory,
    repair_memory: &RepairMemory,
    learned_opportunity_memory: &LearnedOpportunityMemory,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
    tracing: bool,
) -> ReadPhaseResult {
    // One authoritative read view covers blocker cleanup, snapshot dirtiness, and ranking.
    let view = runtime_belief_view(agent, world, scheduler, action_defs, phase.recipe_registry);
    let before = blocked_memory.clone();
    let queue_transition_changed = handle_facility_queue_transitions(
        &view,
        runtime,
        facility_intents,
        blocked_memory,
        agent,
        phase.tick,
        phase,
    );
    clear_resolved_failures(&view, agent, blocked_memory, discrepancy_memory, phase.tick);
    let blocked_changed_from_cleanup = *blocked_memory != before;
    let snapshot_domains =
        observation_snapshot_changed(&view, agent, active_goal, runtime, phase.recipe_registry);
    let queue_patience_exhausted = facility_queue_patience_exhausted(&view, agent, phase.tick);

    // Accumulate all dirty bits directly on runtime.dirty — no dual-tracking.
    if runtime.current_plan.is_none() {
        runtime.dirty.insert(crate::DirtySet::NO_PLAN);
    }
    if plan_finished(runtime) {
        runtime.dirty.insert(crate::DirtySet::PLAN_FINISHED);
    }
    if !replan_signals.is_empty() {
        runtime.dirty.insert(crate::DirtySet::REPLAN_SIGNAL);
    }
    if queue_transition_changed {
        runtime.dirty.insert(crate::DirtySet::QUEUE_TRANSITION);
    }
    if blocked_changed_from_cleanup {
        runtime.dirty.insert(crate::DirtySet::BLOCKER_CLEANUP);
    }
    runtime.dirty.insert(snapshot_domains);
    if queue_patience_exhausted {
        runtime.dirty.insert(crate::DirtySet::QUEUE_PATIENCE);
    }

    // Pursuit plan invalidation: if the active plan is a remote pursuit
    // (Travel + Attack for RaidTarget/EngageHostile), check whether the
    // underlying belief assumptions still hold. If not, clear the plan
    // and force replanning.
    let pursuit_invalidation = runtime
        .current_plan
        .as_ref()
        .and_then(|plan| crate::is_pursuit_plan_invalid(&view, agent, plan, phase.tick));
    if pursuit_invalidation.is_some() {
        runtime.current_plan = None;
        runtime.current_step_index = 0;
        runtime.materialization_bindings.clear();
        facility_intents.intents.clear();
        runtime.dirty.insert(crate::DirtySet::REPLAN_SIGNAL);
    }

    let mut candidates = generate_candidates_with_memories_with_travel_horizon(
        &view,
        agent,
        blocked_memory,
        discrepancy_memory,
        violation_memory,
        phase.recipe_registry,
        phase.tick,
        phase.travel_horizon,
        tracing,
    );
    reinstate_current_plan_candidate(&mut candidates, runtime, active_goal);

    // Apply deferred violation records from candidate generation.
    for pending in &candidates.pending_violations {
        let recorded_id =
            violation_memory.record(pending.kind.clone(), pending.observed_tick, pending.ttl);
        debug_assert_eq!(recorded_id, pending.id);
    }

    let generated_keys = candidates
        .candidates
        .iter()
        .map(|c| worldwake_core::OpportunityKey {
            goal_key: c.key,
            anchor: c.anchor,
        })
        .collect();
    let candidate_evidence = candidates.diagnostics.evidence.values().cloned().collect();
    let dc = crate::build_decision_context(&view, agent);
    let outcome = rank_candidates_with_memories(
        &candidates.candidates,
        &view,
        agent,
        phase.tick,
        phase.utility,
        dc,
        repair_memory,
        learned_opportunity_memory,
    );

    ReadPhaseResult {
        offered: candidates.diagnostics.offers,
        ranked: outcome.ranked,
        generated_keys,
        candidate_evidence,
        fully_blocked_desires: candidates.diagnostics.fully_blocked_desires,
        places_reachable: candidates.diagnostics.places_reachable,
        places_after_belief_filter: candidates.diagnostics.places_after_belief_filter,
        suppressed: {
            let mut suppressed = candidates.diagnostics.suppressed;
            suppressed.extend(outcome.suppressed);
            suppressed
        },
        zero_motive: outcome.zero_motive,
        omitted_political: candidates.diagnostics.omitted_political,
        omitted_bandit: candidates.diagnostics.omitted_bandit,
        omitted_social: candidates.diagnostics.omitted_social,
        omitted_violation_detection: candidates.diagnostics.omitted_violation_detection,
        decision_context: dc,
        pursuit_invalidation,
        pending_acquisition_exhaustion_resets: candidates.pending_acquisition_exhaustion_resets,
    }
}

fn reinstate_current_plan_candidate(
    candidates: &mut crate::candidate_generation::CandidateGenerationResult,
    runtime: &AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
) {
    let Some(plan) = runtime.current_plan.as_ref() else {
        return;
    };
    let opportunity = plan.opportunity;
    if Some(opportunity.goal_key) != active_goal {
        return;
    }

    if candidates.candidates.iter().any(|candidate| {
        candidate.key == opportunity.goal_key && candidate.anchor == opportunity.anchor
    }) {
        return;
    }
    if !candidates
        .candidates
        .iter()
        .any(|candidate| candidate.key == opportunity.goal_key)
    {
        return;
    }

    let mut evidence_entities = BTreeSet::new();
    let mut evidence_places = BTreeSet::new();
    match opportunity.anchor {
        worldwake_core::OpportunityAnchor::Place(place) => {
            evidence_places.insert(place);
        }
        worldwake_core::OpportunityAnchor::Entity(entity) => {
            evidence_entities.insert(entity);
        }
        worldwake_core::OpportunityAnchor::None => {
            if let Some(place) = opportunity.goal_key.place {
                evidence_places.insert(place);
            }
            if let Some(entity) = opportunity.goal_key.entity {
                evidence_entities.insert(entity);
            }
        }
    }

    candidates.candidates.push(crate::GroundedGoal {
        key: opportunity.goal_key,
        anchor: opportunity.anchor,
        evidence_entities,
        evidence_places,
    });
    candidates
        .diagnostics
        .evidence
        .entry(opportunity)
        .or_insert(crate::CandidateEvidenceTrace {
            opportunity,
            contributors: Vec::new(),
            exclusions: Vec::new(),
            knowledge_path: KnowledgePath::default(),
            legality: None,
            pursuit: None,
        });
}

pub(super) fn handle_facility_queue_transitions(
    view: &dyn RuntimeBeliefView,
    runtime: &AgentDecisionRuntime,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    agent: EntityId,
    tick: Tick,
    phase: ReadPhaseContext<'_>,
) -> bool {
    let previous_place = runtime.last_effective_place;
    let current_place = view.effective_place(agent);
    let current_signature = facility_access_signature(view, agent);
    let current_by_facility = current_signature
        .iter()
        .copied()
        .map(|(facility, queued, grant)| (facility, (queued, grant)))
        .collect::<BTreeMap<_, _>>();
    let mut changed = false;

    for (facility, was_queued, previous_grant) in runtime.last_facility_access_signature.clone() {
        let current = current_by_facility.get(&facility).copied();
        let now_queued = current.is_some_and(|(queued, _)| queued);
        let now_granted = current.and_then(|(_, grant)| grant);

        if was_queued && !now_queued && now_granted.is_none() {
            if previous_place == current_place {
                let fallback_intent = runtime.current_plan.as_ref().and_then(|plan| {
                    let intended_action = view
                        .facility_grant(facility)
                        .map(|grant| grant.intended_action)
                        .or_else(|| {
                            plan.steps.iter().find_map(|step| {
                                let matches_facility = step
                                    .targets
                                    .first()
                                    .copied()
                                    .and_then(crate::authoritative_target)
                                    == Some(facility);
                                if !matches_facility {
                                    return None;
                                }
                                match step.op_kind {
                                    crate::PlannerOpKind::QueueForFacilityUse => step
                                        .payload_override
                                        .as_ref()
                                        .and_then(
                                            worldwake_sim::ActionPayload::as_queue_for_facility_use,
                                        )
                                        .map(|payload| payload.intended_action),
                                    crate::PlannerOpKind::Harvest | crate::PlannerOpKind::Craft => {
                                        Some(step.def_id)
                                    }
                                    _ => None,
                                }
                            })
                        })?;
                    Some(QueuedContentionIntent {
                        goal_key: plan.goal,
                        intended_action,
                    })
                });
                if let Some(intent) = facility_intents
                    .intents
                    .remove(&facility)
                    .or(fallback_intent)
                {
                    blocked_memory.record(Blocker {
                        blocker_key: BlockerKey {
                            goal_key: intent.goal_key,
                            place: current_place,
                            target: Some(facility),
                            action_def: Some(intent.intended_action),
                        },
                        blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
                        diagnostic_context: None,
                        observed_tick: tick,
                        expires_tick: tick + u64::from(phase.structural_block_ticks),
                        clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
                        baseline_snapshot: None,
                    });
                    changed = true;
                }
            } else if facility_intents.intents.remove(&facility).is_some() {
                changed = true;
            }
        }

        if previous_grant.is_some() && now_granted.is_none() {
            changed |= facility_intents.intents.remove(&facility).is_some();
        }
    }

    changed
}

#[allow(clippy::too_many_arguments)]
pub(super) fn reconcile_in_flight_state(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: &mut Option<worldwake_core::ActiveGoal>,
    jc: &mut Option<worldwake_core::IntentionFrame>,
    facility_intents: &mut ContentionIntents,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    active_action: Option<&worldwake_sim::ActionInstance>,
    agent: EntityId,
    reconciliation: InFlightReconciliation<'_>,
) -> Result<ReconciliationResult, TickInputError> {
    if !runtime.step_in_flight {
        return Ok(ReconciliationResult::default());
    }
    if active_action.is_some() {
        return Ok(ReconciliationResult::default());
    }

    let failed_signal = reconciliation.replan_signals.first().copied();
    let Some(step) = current_step(runtime).cloned() else {
        runtime.step_in_flight = false;
        return Ok(ReconciliationResult::default());
    };
    let goal_key = active_goal
        .as_ref()
        .map(|ag| ag.goal_key)
        .or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
        .expect("in-flight step must have a current goal");

    if let Some(signal) = failed_signal {
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            Some(ExecutionFailure::Replan(signal)),
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: None,
            replan_trigger: Some((goal_key, replan_reason)),
        });
    }

    if let Some(start_failure) = matching_start_failure(&step, reconciliation.start_failures) {
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            Some(ExecutionFailure::Start(start_failure)),
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: None,
            replan_trigger: Some((goal_key, replan_reason)),
        });
    }

    let Some(committed_action) = committed_action_for_step(&step, reconciliation.committed_actions)
    else {
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            None,
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: None,
            replan_trigger: Some((goal_key, replan_reason)),
        });
    };
    let completed_plan = runtime.current_plan.as_ref().and_then(|plan| {
        let next_index = runtime.current_step_index.checked_add(1)?;
        (next_index >= plan.steps.len()
            && matches!(
                plan.terminal_kind,
                crate::PlanTerminalKind::GoalSatisfied | crate::PlanTerminalKind::CombatCommitment
            ))
        .then_some(CompletedPlanSummary {
            goal_key: plan.goal,
            terminal_kind: plan.terminal_kind,
            step_index: runtime
                .current_step_index
                .try_into()
                .expect("step index exceeds u16"),
        })
    });
    reconcile_committed_facility_queue_intents(runtime, facility_intents, Some(goal_key), &step);
    if apply_step_materialization_bindings(runtime, &step, &committed_action.outcome).is_err() {
        emit_expectation_mismatch(
            ctx.event_log,
            ctx.tick,
            agent,
            goal_key,
            runtime.current_step_index,
            &step,
        );
        let invalidation_reason = PlanInvalidationReason::ExpectationMismatch {
            step_index: runtime
                .current_step_index
                .try_into()
                .expect("step index exceeds u16"),
        };
        let _ = handle_current_step_failure(
            ctx,
            runtime,
            Some(goal_key),
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            &step,
            None,
            None,
        )?;
        return Ok(ReconciliationResult {
            completed_plan: None,
            plan_invalidation: Some((goal_key, invalidation_reason)),
            replan_trigger: Some((
                goal_key,
                ReplanReason::PlanInvalidated {
                    reason: invalidation_reason,
                },
            )),
        });
    }

    runtime.step_in_flight = false;
    *jc = advance_completed_step(
        runtime,
        active_goal,
        facility_intents,
        jc.as_ref(),
        step.op_kind,
        ctx.tick,
    );
    Ok(ReconciliationResult {
        completed_plan,
        plan_invalidation: None,
        replan_trigger: None,
    })
}

fn matching_start_failure<'a>(
    step: &PlannedStep,
    start_failures: &'a [ActionStartFailure],
) -> Option<&'a ActionStartFailure> {
    start_failures
        .iter()
        .find(|failure| failure.def_id == step.def_id)
}

fn reconcile_committed_facility_queue_intents(
    runtime: &AgentDecisionRuntime,
    facility_intents: &mut ContentionIntents,
    active_goal: Option<worldwake_core::GoalKey>,
    step: &PlannedStep,
) {
    let Some(facility) = step.targets.first().copied().and_then(authoritative_target) else {
        return;
    };

    match step.op_kind {
        crate::PlannerOpKind::QueueForFacilityUse => {
            let Some(goal_key) =
                active_goal.or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
            else {
                return;
            };
            let Some(payload) = step
                .payload_override
                .as_ref()
                .and_then(worldwake_sim::ActionPayload::as_queue_for_facility_use)
            else {
                return;
            };
            facility_intents.intents.insert(
                facility,
                QueuedContentionIntent {
                    goal_key,
                    intended_action: payload.intended_action,
                },
            );
        }
        crate::PlannerOpKind::Harvest | crate::PlannerOpKind::Craft => {
            facility_intents.intents.remove(&facility);
        }
        crate::PlannerOpKind::Travel
        | crate::PlannerOpKind::Patrol
        | crate::PlannerOpKind::Sleep
        | crate::PlannerOpKind::Relieve
        | crate::PlannerOpKind::EstablishCamp
        | crate::PlannerOpKind::Trade
        | crate::PlannerOpKind::Consume
        | crate::PlannerOpKind::Wash
        | crate::PlannerOpKind::Heal
        | crate::PlannerOpKind::MoveCargo
        | crate::PlannerOpKind::DropItem
        | crate::PlannerOpKind::Loot
        | crate::PlannerOpKind::Bury
        | crate::PlannerOpKind::Tell
        | crate::PlannerOpKind::ConsultRecord
        | crate::PlannerOpKind::Attack
        | crate::PlannerOpKind::Defend
        | crate::PlannerOpKind::Bribe
        | crate::PlannerOpKind::Threaten
        | crate::PlannerOpKind::Accuse
        | crate::PlannerOpKind::Fine
        | crate::PlannerOpKind::Exile
        | crate::PlannerOpKind::DeclareSupport
        | crate::PlannerOpKind::PressForceClaim
        | crate::PlannerOpKind::YieldForceClaim
        | crate::PlannerOpKind::Investigate
        | crate::PlannerOpKind::AskWitness
        | crate::PlannerOpKind::SearchPlace
        | crate::PlannerOpKind::AskAboutPerson
        | crate::PlannerOpKind::ReportMissing
        | crate::PlannerOpKind::EscortToSafety
        | crate::PlannerOpKind::ReportFound
        | crate::PlannerOpKind::ClaimBounty
        | crate::PlannerOpKind::PostBounty
        | crate::PlannerOpKind::PostNotice
        | crate::PlannerOpKind::StaffMarket
        | crate::PlannerOpKind::StockManagement => {}
    }
}

pub(super) fn observation_snapshot_changed(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    active_goal: Option<worldwake_core::GoalKey>,
    runtime: &AgentDecisionRuntime,
    recipe_registry: &RecipeRegistry,
) -> crate::DirtySet {
    let mut result = crate::DirtySet::default();
    let current_commodity_signature = commodity_signature(view, agent);
    let commodity_filter = active_goal
        .map(|goal| goal.kind.relevant_observed_commodities(recipe_registry))
        .or_else(|| {
            runtime.current_plan.as_ref().map(|plan| {
                plan.goal
                    .kind
                    .relevant_observed_commodities(recipe_registry)
            })
        });
    if runtime.last_effective_place != view.effective_place(agent) {
        result.insert(crate::DirtySet::POSITION);
    }
    if runtime.last_needs != view.homeostatic_needs(agent) {
        result.insert(crate::DirtySet::NEEDS);
    }
    if runtime.last_wounds != view.wounds(agent) {
        result.insert(crate::DirtySet::WOUNDS);
    }
    if filtered_commodity_signature(&runtime.last_commodity_signature, commodity_filter.as_ref())
        != filtered_commodity_signature(&current_commodity_signature, commodity_filter.as_ref())
    {
        result.insert(crate::DirtySet::COMMODITY);
    }
    if runtime.last_unique_item_signature != unique_item_signature(view, agent) {
        result.insert(crate::DirtySet::UNIQUE_ITEMS);
    }
    if runtime.last_facility_access_signature != facility_access_signature(view, agent) {
        result.insert(crate::DirtySet::FACILITIES);
    }
    if runtime.last_patrol_route != view.patrol_route(agent) {
        result.insert(crate::DirtySet::PATROL_ROUTE);
    }
    result
}

pub(super) fn update_runtime_observation_snapshot(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    runtime: &mut AgentDecisionRuntime,
) {
    runtime.last_effective_place = view.effective_place(agent);
    runtime.last_needs = view.homeostatic_needs(agent);
    runtime.last_wounds = view.wounds(agent);
    runtime.last_commodity_signature = commodity_signature(view, agent);
    runtime.last_unique_item_signature = unique_item_signature(view, agent);
    runtime.last_facility_access_signature = facility_access_signature(view, agent);
    runtime.last_patrol_route = view.patrol_route(agent);
    runtime.last_in_transit = view.in_transit_state(agent).is_some();
}

pub(super) fn facility_access_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(EntityId, bool, Option<worldwake_core::ActionDefId>)> {
    let Some(place) = view.effective_place(agent) else {
        return Vec::new();
    };

    view.entities_at(place)
        .into_iter()
        .filter(|entity| view.has_contention_policy(*entity))
        .filter_map(|facility| {
            let queued = view.facility_queue_position(facility, agent).is_some();
            let matching_grant = view
                .facility_grant(facility)
                .and_then(|grant| (grant.actor == agent).then_some(grant.intended_action));
            (queued || matching_grant.is_some()).then_some((facility, queued, matching_grant))
        })
        .collect()
}

pub(super) fn facility_queue_patience_exhausted(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    tick: Tick,
) -> bool {
    let Some(limit) = view.facility_queue_patience_ticks(agent) else {
        return false;
    };
    let Some(place) = view.effective_place(agent) else {
        return false;
    };

    view.entities_at(place).into_iter().any(|facility| {
        if !view.has_contention_policy(facility) {
            return false;
        }
        if view
            .facility_grant(facility)
            .is_some_and(|grant| grant.actor == agent)
        {
            return false;
        }
        view.facility_queue_join_tick(facility, agent)
            .is_some_and(|queued_at| tick >= queued_at + u64::from(limit.get()))
    })
}

pub(super) fn commodity_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(CommodityKind, Quantity)> {
    CommodityKind::ALL
        .into_iter()
        .filter_map(|commodity| {
            let quantity = view.commodity_quantity(agent, commodity);
            (quantity > Quantity(0)).then_some((commodity, quantity))
        })
        .collect()
}

pub(super) fn filtered_commodity_signature(
    signature: &[(CommodityKind, Quantity)],
    relevant: Option<&Option<std::collections::BTreeSet<CommodityKind>>>,
) -> Vec<(CommodityKind, Quantity)> {
    match relevant {
        Some(Some(relevant)) => signature
            .iter()
            .copied()
            .filter(|(commodity, _)| relevant.contains(commodity))
            .collect(),
        Some(None) | None => signature.to_vec(),
    }
}

pub(super) fn unique_item_signature(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
) -> Vec<(UniqueItemKind, u32)> {
    UniqueItemKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = view.unique_item_count(agent, kind);
            (count > 0).then_some((kind, count))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{emit_expectation_mismatch, reinstate_current_plan_candidate};
    use crate::{
        AgentDecisionRuntime, CommodityPurpose, ExpectedMaterialization, GroundedGoal,
        HypotheticalEntityId, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind,
        PlanningEntityRef,
        candidate_generation::{CandidateGenerationDiagnostics, CandidateGenerationResult},
    };
    use std::collections::BTreeSet;
    use worldwake_core::{
        ActionDefId, CommodityKind, DecisionEventPayload, EntityId, EventLog, EventTag, EventView,
        GoalKey, GoalKind, MaterializationTag, OpportunityAnchor,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[test]
    fn reinstate_current_plan_candidate_restores_missing_committed_opportunity() {
        let market = entity(10);
        let inn = entity(11);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let committed_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: OpportunityAnchor::Place(market),
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(1),
                targets: vec![PlanningEntityRef::Authoritative(market)],
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: 3,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
                guard: None,
                expectations: Vec::new(),
            }],
            PlanTerminalKind::ProgressBarrier,
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(committed_plan),
            ..AgentDecisionRuntime::default()
        };

        let mut candidates = CandidateGenerationResult {
            candidates: vec![GroundedGoal {
                key: goal,
                anchor: OpportunityAnchor::Place(inn),
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::from([inn]),
            }],
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
            pending_acquisition_exhaustion_resets: BTreeSet::new(),
        };

        reinstate_current_plan_candidate(&mut candidates, &runtime, Some(goal));

        assert!(candidates.candidates.iter().any(|candidate| {
            candidate.key == goal && candidate.anchor == OpportunityAnchor::Place(market)
        }));
        assert!(
            candidates
                .diagnostics
                .evidence
                .contains_key(&worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: OpportunityAnchor::Place(market),
                })
        );
    }

    #[test]
    fn reinstate_current_plan_candidate_skips_when_active_goal_differs() {
        let market = entity(10);
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let other_goal = GoalKey::from(GoalKind::Sleep);
        let committed_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: committed_goal,
                anchor: OpportunityAnchor::Place(market),
            },
            committed_goal,
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(committed_plan),
            ..AgentDecisionRuntime::default()
        };
        let mut candidates = CandidateGenerationResult {
            candidates: Vec::new(),
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
            pending_acquisition_exhaustion_resets: BTreeSet::new(),
        };

        reinstate_current_plan_candidate(&mut candidates, &runtime, Some(other_goal));

        assert!(candidates.candidates.is_empty());
        assert!(candidates.diagnostics.evidence.is_empty());
    }

    #[test]
    fn reinstate_current_plan_candidate_skips_when_goal_has_no_live_siblings() {
        let market = entity(10);
        let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let committed_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: committed_goal,
                anchor: OpportunityAnchor::Place(market),
            },
            committed_goal,
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        );
        let runtime = AgentDecisionRuntime {
            current_plan: Some(committed_plan),
            ..AgentDecisionRuntime::default()
        };
        let mut candidates = CandidateGenerationResult {
            candidates: vec![GroundedGoal {
                key: GoalKey::from(GoalKind::Sleep),
                anchor: OpportunityAnchor::None,
                evidence_entities: BTreeSet::new(),
                evidence_places: BTreeSet::new(),
            }],
            diagnostics: CandidateGenerationDiagnostics::default(),
            pending_violations: Vec::new(),
            pending_acquisition_exhaustion_resets: BTreeSet::new(),
        };

        reinstate_current_plan_candidate(&mut candidates, &runtime, Some(committed_goal));

        assert_eq!(candidates.candidates.len(), 1);
        assert_eq!(candidates.candidates[0].key, GoalKey::from(GoalKind::Sleep));
        assert!(candidates.diagnostics.evidence.is_empty());
    }

    #[test]
    fn emit_expectation_mismatch_records_expected_tags_and_step_index() {
        let mut event_log = EventLog::new();
        let agent = entity(4);
        let goal_key = GoalKey::from(GoalKind::Sleep);
        let step = PlannedStep {
            def_id: ActionDefId(7),
            targets: Vec::new(),
            payload_override: None,
            op_kind: PlannerOpKind::Craft,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: vec![ExpectedMaterialization {
                hypothetical_id: HypotheticalEntityId(3),
                tag: MaterializationTag::SplitOffLot,
            }],
            guard: None,
            expectations: Vec::new(),
        };

        emit_expectation_mismatch(
            &mut event_log,
            worldwake_core::Tick(12),
            agent,
            goal_key,
            5,
            &step,
        );

        let events = event_log.events_by_tag(EventTag::ExpectationMismatch);
        assert_eq!(events.len(), 1);
        let payload = event_log
            .get(events[0])
            .and_then(|record| record.decision_payload())
            .expect("expectation mismatch event should carry payload");
        assert_eq!(
            payload,
            &DecisionEventPayload::ExpectationMismatch(
                worldwake_core::ExpectationMismatchPayload {
                    agent,
                    goal_key,
                    step_index: 5,
                    expected_materializations: vec![MaterializationTag::SplitOffLot],
                    expectation_kind: None,
                    mismatch_detail: None,
                }
            )
        );
    }
}
