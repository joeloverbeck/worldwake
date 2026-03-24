use std::collections::BTreeMap;
use worldwake_core::{
    BlockedIntent, BlockedIntentMemory, BlockerKey, BlockingFact, CommodityKind, EntityId,
    Quantity, Tick, UniqueItemKind,
};
use worldwake_sim::{
    ActionStartFailure, CommittedAction, RecipeRegistry, ReplanNeeded, RuntimeBeliefView,
    TickInputError,
};

use crate::candidate_generation::generate_candidates_with_travel_horizon;
use crate::failure_handling::ExecutionFailure;
use crate::{
    authoritative_target, clear_resolved_blockers, rank_candidates, AgentDecisionRuntime,
    DecisionContext, GoalKindPlannerExt, PlannedStep, RankedGoal,
};
use worldwake_core::{FacilityQueueIntents, QueuedFacilityIntent};

use super::{
    advance_completed_step, apply_step_materialization_bindings, committed_action_for_step,
    current_step, handle_current_step_failure, plan_finished, runtime_belief_view,
    AgentTickContext,
};

use crate::decision_trace::DirtyReason;

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

/// Result of the read phase, preserving trace-relevant data alongside ranked candidates.
pub(crate) struct ReadPhaseResult {
    pub(super) ranked: Vec<RankedGoal>,
    pub(super) dirty_reasons: Vec<DirtyReason>,
    /// Generated candidate keys (before ranking filter).
    pub(super) generated_keys: Vec<worldwake_core::GoalKey>,
    /// Typed candidate-evidence provenance keyed by generated goal.
    pub(super) candidate_evidence: Vec<crate::CandidateEvidenceTrace>,
    /// Goals suppressed by situational conditions.
    pub(super) suppressed: Vec<worldwake_core::GoalKey>,
    /// Goals with zero motive score.
    pub(super) zero_motive: Vec<worldwake_core::GoalKey>,
    /// Political goals omitted before emission due to hard gates.
    pub(super) omitted_political: Vec<crate::PoliticalCandidateOmission>,
    /// Social goals omitted before emission due to resend suppression.
    pub(super) omitted_social: Vec<crate::SocialCandidateOmission>,
    /// Shared decision context built once from beliefs for ranking + interrupts.
    pub(super) decision_context: DecisionContext,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refresh_runtime_for_read_phase(
    world: &worldwake_core::World,
    scheduler: &worldwake_sim::Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    facility_intents: &mut FacilityQueueIntents,
    blocked_memory: &mut BlockedIntentMemory,
    agent: EntityId,
    replan_signals: &[&ReplanNeeded],
    phase: ReadPhaseContext<'_>,
    _tracing: bool,
) -> ReadPhaseResult {
    // One authoritative read view covers blocker cleanup, snapshot dirtiness, and ranking.
    let view = runtime_belief_view(agent, world, scheduler, action_defs);
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
    clear_resolved_blockers(&view, agent, blocked_memory, phase.tick);
    let blocked_changed_from_cleanup = *blocked_memory != before;
    let snapshot_changed =
        observation_snapshot_changed(&view, agent, active_goal, runtime, phase.recipe_registry);
    let queue_patience_exhausted = facility_queue_patience_exhausted(&view, agent, phase.tick);

    // Decompose dirty-flag into individual reasons for tracing.
    let mut dirty_reasons = Vec::new();
    if runtime.current_plan.is_none() {
        dirty_reasons.push(DirtyReason::NoPlan);
    }
    if plan_finished(runtime) {
        dirty_reasons.push(DirtyReason::PlanFinished);
    }
    if !replan_signals.is_empty() {
        dirty_reasons.push(DirtyReason::ReplanSignal);
    }
    if queue_transition_changed {
        dirty_reasons.push(DirtyReason::QueueTransition);
    }
    if blocked_changed_from_cleanup {
        dirty_reasons.push(DirtyReason::BlockerCleanup);
    }
    if snapshot_changed {
        dirty_reasons.push(DirtyReason::SnapshotChanged);
    }
    if queue_patience_exhausted {
        dirty_reasons.push(DirtyReason::QueuePatienceExhausted);
    }

    // Preserve original boolean behavior exactly.
    runtime.dirty = runtime.dirty || !dirty_reasons.is_empty();

    let candidates = generate_candidates_with_travel_horizon(
        &view,
        agent,
        blocked_memory,
        phase.recipe_registry,
        phase.tick,
        phase.travel_horizon,
    );
    let generated_keys = candidates.candidates.iter().map(|c| c.key).collect();
    let candidate_evidence = candidates.diagnostics.evidence.values().cloned().collect();
    let dc = crate::build_decision_context(&view, agent);
    let outcome = rank_candidates(
        &candidates.candidates,
        &view,
        agent,
        phase.tick,
        phase.utility,
        phase.recipe_registry,
        &dc,
    );

    ReadPhaseResult {
        ranked: outcome.ranked,
        dirty_reasons,
        generated_keys,
        candidate_evidence,
        suppressed: outcome.suppressed,
        zero_motive: outcome.zero_motive,
        omitted_political: candidates.diagnostics.omitted_political,
        omitted_social: candidates.diagnostics.omitted_social,
        decision_context: dc,
    }
}

pub(super) fn handle_facility_queue_transitions(
    view: &dyn RuntimeBeliefView,
    runtime: &AgentDecisionRuntime,
    facility_intents: &mut FacilityQueueIntents,
    blocked_memory: &mut BlockedIntentMemory,
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
                if let Some(intent) = facility_intents.intents.remove(&facility) {
                    blocked_memory.record(BlockedIntent {
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
    jc: &mut Option<worldwake_core::JourneyCommitment>,
    facility_intents: &mut FacilityQueueIntents,
    blocked_memory: &mut BlockedIntentMemory,
    active_action: Option<&worldwake_sim::ActionInstance>,
    agent: EntityId,
    reconciliation: InFlightReconciliation<'_>,
) -> Result<(), TickInputError> {
    if !runtime.step_in_flight {
        return Ok(());
    }
    if active_action.is_some() {
        return Ok(());
    }

    let failed_signal = reconciliation.replan_signals.first().copied();
    let Some(step) = current_step(runtime).cloned() else {
        runtime.step_in_flight = false;
        return Ok(());
    };

    if let Some(signal) = failed_signal {
        handle_current_step_failure(
            ctx,
            runtime,
            active_goal.as_ref().map(|ag| ag.goal_key),
            jc,
            blocked_memory,
            agent,
            &step,
            Some(ExecutionFailure::Replan(signal)),
        )?;
        return Ok(());
    }

    if let Some(start_failure) = matching_start_failure(&step, reconciliation.start_failures) {
        handle_current_step_failure(
            ctx,
            runtime,
            active_goal.as_ref().map(|ag| ag.goal_key),
            jc,
            blocked_memory,
            agent,
            &step,
            Some(ExecutionFailure::Start(start_failure)),
        )?;
        return Ok(());
    }

    let Some(committed_action) = committed_action_for_step(&step, reconciliation.committed_actions)
    else {
        handle_current_step_failure(ctx, runtime, active_goal.as_ref().map(|ag| ag.goal_key), jc, blocked_memory, agent, &step, None)?;
        return Ok(());
    };
    let goal_key = active_goal.as_ref().map(|ag| ag.goal_key);
    reconcile_committed_facility_queue_intents(runtime, facility_intents, goal_key, &step);
    if apply_step_materialization_bindings(runtime, &step, &committed_action.outcome).is_err() {
        handle_current_step_failure(ctx, runtime, active_goal.as_ref().map(|ag| ag.goal_key), jc, blocked_memory, agent, &step, None)?;
        return Ok(());
    }

    runtime.step_in_flight = false;
    *jc = advance_completed_step(runtime, active_goal, jc.as_ref(), step.op_kind, ctx.tick);
    Ok(())
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
    facility_intents: &mut FacilityQueueIntents,
    active_goal: Option<worldwake_core::GoalKey>,
    step: &PlannedStep,
) {
    let Some(facility) = step.targets.first().copied().and_then(authoritative_target) else {
        return;
    };

    match step.op_kind {
        crate::PlannerOpKind::QueueForFacilityUse => {
            let Some(goal_key) = active_goal
                .or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
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
                QueuedFacilityIntent {
                    goal_key,
                    intended_action: payload.intended_action,
                },
            );
        }
        crate::PlannerOpKind::Harvest | crate::PlannerOpKind::Craft => {
            facility_intents.intents.remove(&facility);
        }
        crate::PlannerOpKind::Travel
        | crate::PlannerOpKind::Sleep
        | crate::PlannerOpKind::Relieve
        | crate::PlannerOpKind::Trade
        | crate::PlannerOpKind::Consume
        | crate::PlannerOpKind::Wash
        | crate::PlannerOpKind::Heal
        | crate::PlannerOpKind::MoveCargo
        | crate::PlannerOpKind::Loot
        | crate::PlannerOpKind::Bury
        | crate::PlannerOpKind::Tell
        | crate::PlannerOpKind::ConsultRecord
        | crate::PlannerOpKind::Attack
        | crate::PlannerOpKind::Defend
        | crate::PlannerOpKind::Bribe
        | crate::PlannerOpKind::Threaten
        | crate::PlannerOpKind::DeclareSupport
        | crate::PlannerOpKind::PressForceClaim
        | crate::PlannerOpKind::YieldForceClaim => {}
    }
}

pub(super) fn observation_snapshot_changed(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    active_goal: Option<worldwake_core::GoalKey>,
    runtime: &AgentDecisionRuntime,
    recipe_registry: &RecipeRegistry,
) -> bool {
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
    runtime.last_effective_place != view.effective_place(agent)
        || runtime.last_needs != view.homeostatic_needs(agent)
        || runtime.last_wounds != view.wounds(agent)
        || filtered_commodity_signature(
            &runtime.last_commodity_signature,
            commodity_filter.as_ref(),
        ) != filtered_commodity_signature(
            &current_commodity_signature,
            commodity_filter.as_ref(),
        )
        || runtime.last_unique_item_signature != unique_item_signature(view, agent)
        || runtime.last_facility_access_signature != facility_access_signature(view, agent)
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
        .filter(|entity| view.has_exclusive_facility_policy(*entity))
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
        if !view.has_exclusive_facility_policy(facility) {
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
