use super::active_action::handle_current_step_failure;
use super::observation::{
    ExpectationMismatchContext, emit_expectation_mismatch, update_runtime_observation_snapshot,
};
use super::{
    AgentTickContext, AssumptionRefContext, decisive_evidence_from_blocker,
    decisive_evidence_from_discrepancy_entry, decisive_evidence_from_replan_reason,
    emit_decision_event, handle_recoverable_travel_step_blockage, runtime_belief_view,
};
use crate::RepairAttemptTrace;
use crate::failure_handling::exact_target_belief_discrepancy;
use crate::plan_repair::{
    PlanRepairContext, RepairOutcome, RepairPlanCandidate, attempt_repair_then_replan,
    repair_budget,
};
use crate::plan_step_expectations::{
    expire_plan_step_expectations, persist_expectation_store_update,
};
use crate::{
    AgentDecisionRuntime, PlannedPlan, PlannedStep, RevalidationOutcome, classify_revalidation,
};
use worldwake_core::{
    AffordanceKey, BeliefSnapshot, BeliefStatusTag, BlockerKey, BlockerMemory,
    BlockerRecordedPayload, BlockingFact, BreachSignature, CausalLink, CausalProvider, CauseRef,
    ContentionIntents, DecisionEventPayload, Discrepancy, DiscrepancyClearing, DiscrepancyEntry,
    DiscrepancyMemory, EntityId, EventId, EventLog, EventTag, EventView, InvalidatorTag,
    LearnedOpportunityMemory, MismatchDetail, PlanningFact, RepairAppliedPayload, RepairEntry,
    RepairKind, RepairMemory, ReplanTriggeredPayload, Tick, VisibilitySpec, WitnessData, WorldTxn,
};
use worldwake_sim::{
    CommitOutcome, CommittedAction, EntityBeliefView, InputKind, PerAgentBeliefView, Scheduler,
    TickInputError,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn enqueue_valid_step_or_handle_failure(
    ctx: &mut AgentTickContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    active_goal: Option<worldwake_core::GoalKey>,
    jc: &mut Option<worldwake_core::IntentionFrame>,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    facility_intents: &mut ContentionIntents,
    agent: EntityId,
    tick: Tick,
    original_blocked: &BlockerMemory,
    original_discrepancy_memory: &DiscrepancyMemory,
    original_violation_memory: &worldwake_core::ViolationMemory,
    violation_memory: &worldwake_core::ViolationMemory,
    original_repair_memory: &RepairMemory,
    repair_memory: &mut RepairMemory,
    memory_capacity: worldwake_core::MemoryCapacityProfile,
    original_learned_opportunity_memory: &LearnedOpportunityMemory,
    learned_opportunity_memory: &LearnedOpportunityMemory,
    step: &PlannedStep,
    valid: bool,
    mut repair_attempt_traces: Option<&mut Vec<RepairAttemptTrace>>,
) -> Result<(), TickInputError> {
    let active_assumptions = jc
        .as_ref()
        .map_or_else(Vec::new, |frame| frame.assumptions.clone());
    let active_plan = runtime.current_plan.clone();
    let assumption_refs = AssumptionRefContext::new(
        &active_assumptions,
        ctx.cognitive.decision_history_alternatives,
    )
    .with_plan(active_plan.as_ref());
    if !valid {
        let view = runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            ctx.action_defs,
            ctx.recipe_registry,
        );
        let (handled, updated_jc) = handle_recoverable_travel_step_blockage(
            &view,
            jc.as_ref(),
            runtime,
            active_goal,
            blocked_memory,
            facility_intents,
            agent,
            step,
            tick,
            ctx.cognitive,
        );
        *jc = updated_jc;
        if handled {
            if runtime.current_plan.is_none() {
                let _ = persist_expectation_store_update(
                    ctx.world,
                    ctx.event_log,
                    agent,
                    tick,
                    expire_plan_step_expectations,
                )?;
            }
            return Ok(());
        }
        let classification = classify_revalidation(
            &view,
            agent,
            runtime
                .current_step_index
                .try_into()
                .expect("current step index exceeds u16"),
            step,
            &runtime.materialization_bindings,
            ctx.action_defs,
            ctx.action_handlers,
        );
        let (plan_invalidation_reason, expectation_kind, mismatch_detail) = match classification {
            RevalidationOutcome::Valid => (None, None, None),
            RevalidationOutcome::Invalidated {
                reason,
                expectation_kind,
                mismatch_detail,
            } => {
                if let Some(goal_key) =
                    active_goal.or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal))
                    && let Some(repair_outcome) = attempt_local_repair_for_invalidated_step(
                        runtime,
                        repair_memory,
                        ctx.cognitive,
                        tick,
                        goal_key,
                        step,
                        reason,
                        mismatch_detail,
                    )
                {
                    match repair_outcome {
                        RepairOutcome::Repaired {
                            kind,
                            new_plan,
                            rejected,
                        } => {
                            let step_index = runtime
                                .current_step_index
                                .try_into()
                                .expect("current step index exceeds u16");
                            if let Some(traces) = &mut repair_attempt_traces {
                                traces.push(RepairAttemptTrace {
                                    breach: breach_signature_for_step(
                                        goal_key,
                                        step,
                                        mismatch_detail,
                                    ),
                                    chosen_kind: Some(kind),
                                    budget_consumed: repair_budget_consumed(&rejected, true),
                                    rejected,
                                    budget_total: repair_budget(ctx.cognitive),
                                });
                            }
                            apply_repaired_plan_and_emit(
                                ctx.event_log,
                                runtime,
                                tick,
                                agent,
                                goal_key,
                                step_index,
                                kind,
                                *new_plan,
                            );
                            return Ok(());
                        }
                        RepairOutcome::Failed { tried } => {
                            let signature =
                                breach_signature_for_step(goal_key, step, mismatch_detail);
                            if let Some(traces) = &mut repair_attempt_traces {
                                traces.push(repair_attempt_trace_from_failed(
                                    signature,
                                    &tried,
                                    ctx.cognitive,
                                ));
                            }
                            record_failed_repair_attempts(
                                repair_memory,
                                signature,
                                &tried,
                                tick,
                                ctx.cognitive.repair_memory_ticks,
                            );
                            repair_memory.enforce_capacity(&memory_capacity);
                        }
                    }
                }
                (Some(reason), expectation_kind, mismatch_detail)
            }
        };
        let belief_discrepancy = exact_target_belief_discrepancy(&view, agent, step);
        let mismatch_goal_key =
            active_goal.or_else(|| runtime.current_plan.as_ref().map(|plan| plan.goal));
        if let (
            Some(goal_key),
            Some(worldwake_core::PlanInvalidationReason::ExpectationMismatch { .. }),
        ) = (mismatch_goal_key, plan_invalidation_reason)
        {
            emit_expectation_mismatch(
                ctx.event_log,
                tick,
                agent,
                goal_key,
                runtime.current_step_index,
                step,
                ExpectationMismatchContext {
                    expectation_kind,
                    mismatch_detail,
                    assumption_refs,
                    max_decisive_evidence: ctx.cognitive.decision_history_alternatives,
                },
            );
        }
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            active_goal,
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            step,
            None,
            belief_discrepancy,
            plan_invalidation_reason,
        )?;
        if let Some(goal_key) = active_goal {
            let decisive = decisive_evidence_from_replan_reason(
                &replan_reason,
                tick,
                ctx.cognitive.decision_history_alternatives,
            );
            emit_decision_event(
                ctx.event_log,
                tick,
                agent,
                EventTag::ReplanTriggered,
                DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                    agent,
                    goal_key,
                    reason: replan_reason,
                    decisive_beliefs: decisive.beliefs,
                    decisive_records: decisive.records,
                    decisive_world_observations: decisive.world_observations,
                    assumptions: assumption_refs.to_refs(),
                }),
            );
        }
        return Ok(());
    }

    let Some(targets) = resolve_step_targets(runtime, step) else {
        let view = runtime_belief_view(
            agent,
            ctx.world,
            ctx.scheduler,
            ctx.action_defs,
            ctx.recipe_registry,
        );
        let (handled, updated_jc) = handle_recoverable_travel_step_blockage(
            &view,
            jc.as_ref(),
            runtime,
            active_goal,
            blocked_memory,
            facility_intents,
            agent,
            step,
            tick,
            ctx.cognitive,
        );
        *jc = updated_jc;
        if handled {
            if runtime.current_plan.is_none() {
                let _ = persist_expectation_store_update(
                    ctx.world,
                    ctx.event_log,
                    agent,
                    tick,
                    expire_plan_step_expectations,
                )?;
            }
            return finalize_agent_tick(
                ctx.world,
                ctx.event_log,
                ctx.scheduler,
                ctx.action_defs,
                ctx.recipe_registry,
                agent,
                tick,
                original_blocked,
                blocked_memory,
                original_discrepancy_memory,
                discrepancy_memory,
                original_violation_memory,
                violation_memory,
                original_repair_memory,
                repair_memory,
                original_learned_opportunity_memory,
                learned_opportunity_memory,
                runtime,
                assumption_refs,
            );
        }
        let replan_reason = handle_current_step_failure(
            ctx,
            runtime,
            active_goal,
            jc,
            blocked_memory,
            discrepancy_memory,
            facility_intents,
            agent,
            step,
            None,
            None,
            None,
        )?;
        if let Some(goal_key) = active_goal {
            let decisive = decisive_evidence_from_replan_reason(
                &replan_reason,
                tick,
                ctx.cognitive.decision_history_alternatives,
            );
            emit_decision_event(
                ctx.event_log,
                tick,
                agent,
                EventTag::ReplanTriggered,
                DecisionEventPayload::ReplanTriggered(ReplanTriggeredPayload {
                    agent,
                    goal_key,
                    reason: replan_reason,
                    decisive_beliefs: decisive.beliefs,
                    decisive_records: decisive.records,
                    decisive_world_observations: decisive.world_observations,
                    assumptions: assumption_refs.to_refs(),
                }),
            );
        }
        return finalize_agent_tick(
            ctx.world,
            ctx.event_log,
            ctx.scheduler,
            ctx.action_defs,
            ctx.recipe_registry,
            agent,
            tick,
            original_blocked,
            blocked_memory,
            original_discrepancy_memory,
            discrepancy_memory,
            original_violation_memory,
            violation_memory,
            original_repair_memory,
            repair_memory,
            original_learned_opportunity_memory,
            learned_opportunity_memory,
            runtime,
            assumption_refs,
        );
    };

    let _ = ctx.scheduler.input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor: agent,
            def_id: step.def_id,
            targets,
            payload_override: step.payload_override.clone(),
            mode: worldwake_sim::ActionRequestMode::BestEffort,
            provenance: worldwake_sim::RequestProvenance::AiPlan,
        },
    );
    runtime.step_in_flight = true;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn attempt_local_repair_for_invalidated_step(
    runtime: &AgentDecisionRuntime,
    repair_memory: &RepairMemory,
    cognitive: &worldwake_core::CognitiveProfile,
    tick: Tick,
    goal_key: worldwake_core::GoalKey,
    step: &PlannedStep,
    reason: worldwake_core::PlanInvalidationReason,
    mismatch_detail: Option<MismatchDetail>,
) -> Option<RepairOutcome> {
    if !matches!(
        reason,
        worldwake_core::PlanInvalidationReason::ExpectationMismatch { .. }
    ) {
        return None;
    }
    let active_plan = runtime.current_plan.as_ref()?;
    if runtime.current_step_index >= active_plan.steps.len() {
        return None;
    }
    let failed_step = runtime.current_step_index.try_into().ok()?;
    let broken_link = broken_link_for_step(step, failed_step)?;
    let signature = breach_signature_for_step(goal_key, step, mismatch_detail);
    let discrepancy_entry =
        discrepancy_entry_for_repair(goal_key, step, broken_link, signature, tick);
    let preserved_prefix = &active_plan.steps[..runtime.current_step_index];
    let reusable_suffix = active_plan
        .steps
        .get(runtime.current_step_index + 1..)
        .unwrap_or(&[]);
    let replacement_candidates = repair_candidates_from_reusable_suffix(reusable_suffix);
    let context = PlanRepairContext {
        opportunity: active_plan.opportunity,
        failed_step,
        broken_link,
        breach_signature: signature,
        preserved_prefix,
        reusable_suffix,
        replacement_candidates: &replacement_candidates,
        new_evidence: &[],
        discrepancy_entry: &discrepancy_entry,
    };
    Some(attempt_repair_then_replan(
        &context,
        cognitive,
        repair_memory,
    ))
}

fn broken_link_for_step(step: &PlannedStep, failed_step: u16) -> Option<CausalLink> {
    step.guard
        .as_ref()?
        .causal_links
        .iter()
        .copied()
        .find(|link| link.consumer_step_index == failed_step)
        .or_else(|| step.guard.as_ref()?.causal_links.first().copied())
}

fn breach_signature_for_step(
    goal_key: worldwake_core::GoalKey,
    step: &PlannedStep,
    mismatch_detail: Option<MismatchDetail>,
) -> BreachSignature {
    let invalidator = match mismatch_detail {
        Some(MismatchDetail::GuardInvalidator(tag)) => tag,
        _ => InvalidatorTag::TargetMoved,
    };
    BreachSignature {
        goal_key,
        invalidator,
        step_target: step.primary_target(),
    }
}

fn discrepancy_entry_for_repair(
    goal_key: worldwake_core::GoalKey,
    step: &PlannedStep,
    broken_link: CausalLink,
    signature: BreachSignature,
    tick: Tick,
) -> DiscrepancyEntry {
    let (discrepancy, clearing_condition) =
        discrepancy_for_repair_signature(signature, broken_link);
    DiscrepancyEntry {
        blocker_key: BlockerKey {
            goal_key,
            place: step.target_place,
            target: signature.step_target.or_else(|| match broken_link.fact {
                PlanningFact::TargetPresent { target, .. } => Some(target),
                PlanningFact::CommodityAvailable { place, .. } => Some(place),
                PlanningFact::RouteKnown { to, .. } => Some(to),
                PlanningFact::ResourceAccess { resource, .. } => Some(resource),
            }),
            action_def: Some(step.def_id),
        },
        discrepancy,
        observed_tick: tick,
        expires_tick: Tick(tick.0 + 1),
        clearing_condition,
    }
}

fn discrepancy_for_repair_signature(
    signature: BreachSignature,
    broken_link: CausalLink,
) -> (Discrepancy, DiscrepancyClearing) {
    match signature.invalidator {
        InvalidatorTag::BeliefStatusChange => {
            let clearing = match broken_link.provider {
                CausalProvider::Belief { claim_key } => {
                    DiscrepancyClearing::BeliefUpdate { claim_key }
                }
                _ => DiscrepancyClearing::TtlExpiry,
            };
            (Discrepancy::BeliefStale, clearing)
        }
        InvalidatorTag::TargetMoved => {
            let target = signature.step_target.or_else(|| match broken_link.fact {
                PlanningFact::TargetPresent { target, .. } => Some(target),
                PlanningFact::ResourceAccess { resource, .. } => Some(resource),
                PlanningFact::CommodityAvailable { place, .. }
                | PlanningFact::RouteKnown { to: place, .. } => Some(place),
            });
            (
                Discrepancy::BeliefContradicted,
                target.map_or(DiscrepancyClearing::TtlExpiry, |target| {
                    DiscrepancyClearing::ReobservationOf { target }
                }),
            )
        }
        InvalidatorTag::CommodityDepleted => {
            let clearing = match broken_link.fact {
                PlanningFact::CommodityAvailable { place, kind, .. } => {
                    DiscrepancyClearing::CommodityAvailabilityChanged {
                        commodity: kind,
                        place,
                    }
                }
                _ => DiscrepancyClearing::TtlExpiry,
            };
            (Discrepancy::SourceInvalidated, clearing)
        }
        InvalidatorTag::NewBlockerRecorded => (
            Discrepancy::PartialExecutionDrift,
            DiscrepancyClearing::TtlExpiry,
        ),
    }
}

fn repair_candidates_from_reusable_suffix(
    reusable_suffix: &[PlannedStep],
) -> Vec<RepairPlanCandidate> {
    reusable_suffix
        .iter()
        .enumerate()
        .filter_map(|(index, step)| {
            let link = step.guard.as_ref()?.causal_links.first().copied()?;
            Some(RepairPlanCandidate {
                kind: repair_kind_for_fact(link.fact),
                provider: link.provider,
                fact: link.fact,
                step: step.clone(),
                reusable_suffix_index: Some(index.try_into().ok()?),
            })
        })
        .collect()
}

const fn repair_kind_for_fact(fact: PlanningFact) -> RepairKind {
    match fact {
        PlanningFact::RouteKnown { .. } => RepairKind::ReplaceProvider,
        PlanningFact::TargetPresent { .. }
        | PlanningFact::CommodityAvailable { .. }
        | PlanningFact::ResourceAccess { .. } => RepairKind::RebindTarget,
    }
}

fn record_failed_repair_attempts(
    repair_memory: &mut RepairMemory,
    signature: BreachSignature,
    tried: &[(RepairKind, crate::RepairFailure)],
    tick: Tick,
    ttl_ticks: u32,
) {
    for (kind, _) in tried {
        repair_memory.record(RepairEntry {
            signature,
            kind: *kind,
            succeeded: false,
            observed_tick: tick,
            expires_tick: Tick(tick.0 + u64::from(ttl_ticks)),
            success_count: 0,
        });
    }
}

fn repair_attempt_trace_from_failed(
    breach: BreachSignature,
    tried: &[(RepairKind, crate::RepairFailure)],
    cognitive: &worldwake_core::CognitiveProfile,
) -> RepairAttemptTrace {
    RepairAttemptTrace {
        breach,
        chosen_kind: None,
        rejected: tried.to_vec(),
        budget_consumed: repair_budget_consumed(tried, false),
        budget_total: repair_budget(cognitive),
    }
}

fn repair_budget_consumed(
    rejected: &[(RepairKind, crate::RepairFailure)],
    includes_successful_attempt: bool,
) -> u16 {
    let failed_expansions = rejected
        .iter()
        .filter(|(_, failure)| {
            !matches!(
                failure,
                crate::RepairFailure::RecentlyFailed | crate::RepairFailure::BudgetExhausted
            )
        })
        .count();
    u16::try_from(failed_expansions)
        .unwrap_or(u16::MAX)
        .saturating_add(u16::from(includes_successful_attempt))
}

#[allow(clippy::too_many_arguments)]
fn apply_repaired_plan_and_emit(
    event_log: &mut EventLog,
    runtime: &mut AgentDecisionRuntime,
    tick: Tick,
    agent: EntityId,
    goal_key: worldwake_core::GoalKey,
    step_index: u16,
    kind: RepairKind,
    new_plan: PlannedPlan,
) {
    let substitute_target = substitute_target_from_repaired_plan(kind, step_index, &new_plan);
    let substitute_recipe = substitute_recipe_from_repaired_plan(&new_plan);
    install_repaired_plan(runtime, new_plan);
    emit_decision_event(
        event_log,
        tick,
        agent,
        EventTag::RepairApplied,
        DecisionEventPayload::RepairApplied(RepairAppliedPayload {
            agent,
            goal_key,
            step_index,
            repair_kind: kind,
            substitute_target,
            substitute_recipe,
        }),
    );
}

fn install_repaired_plan(runtime: &mut AgentDecisionRuntime, mut new_plan: PlannedPlan) {
    let next_step_index = runtime.current_step_index.min(new_plan.steps.len());
    if let Some(current_plan) = runtime.current_plan.as_ref() {
        new_plan.committed_source = current_plan.committed_source;
        new_plan.expectation_kind = current_plan.expectation_kind;
    }
    runtime.current_step_index = next_step_index;
    runtime.current_plan = Some(new_plan);
    runtime.step_in_flight = false;
    runtime.accepted_repair = None;
}

fn substitute_target_from_repaired_plan(
    kind: RepairKind,
    step_index: u16,
    new_plan: &PlannedPlan,
) -> Option<EntityId> {
    matches!(kind, RepairKind::RebindTarget)
        .then(|| {
            new_plan
                .steps
                .get(usize::from(step_index))?
                .primary_target()
        })
        .flatten()
}

fn substitute_recipe_from_repaired_plan(
    new_plan: &PlannedPlan,
) -> Option<worldwake_core::RecipeId> {
    new_plan.steps.iter().find_map(|step| {
        step.payload_override
            .as_ref()
            .and_then(worldwake_sim::ActionPayload::as_craft)
            .map(|payload| payload.recipe_id)
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_agent_tick(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    scheduler: &Scheduler,
    action_defs: &worldwake_sim::ActionDefRegistry,
    recipe_registry: &worldwake_sim::RecipeRegistry,
    agent: EntityId,
    tick: Tick,
    original_blocked: &BlockerMemory,
    blocked_memory: &BlockerMemory,
    original_discrepancy_memory: &DiscrepancyMemory,
    discrepancy_memory: &DiscrepancyMemory,
    original_violation_memory: &worldwake_core::ViolationMemory,
    violation_memory: &worldwake_core::ViolationMemory,
    original_repair_memory: &RepairMemory,
    repair_memory: &RepairMemory,
    original_learned_opportunity_memory: &LearnedOpportunityMemory,
    learned_opportunity_memory: &LearnedOpportunityMemory,
    runtime: &mut AgentDecisionRuntime,
    assumption_refs: AssumptionRefContext<'_>,
) -> Result<(), TickInputError> {
    let populated_blocked_memory = populate_contention_event_refs(blocked_memory, event_log, tick);
    let blocked_memory = populated_blocked_memory.as_ref().unwrap_or(blocked_memory);

    persist_blocked_memory(
        world,
        event_log,
        agent,
        tick,
        original_blocked,
        blocked_memory,
        assumption_refs,
    )?;
    persist_discrepancy_memory(
        world,
        event_log,
        agent,
        tick,
        original_discrepancy_memory,
        discrepancy_memory,
        assumption_refs,
    )?;
    persist_violation_memory(
        world,
        event_log,
        agent,
        tick,
        original_violation_memory,
        violation_memory,
    )?;
    persist_repair_memory(
        world,
        event_log,
        agent,
        tick,
        original_repair_memory,
        repair_memory,
    )?;
    persist_learned_opportunity_memory(
        world,
        event_log,
        agent,
        tick,
        original_learned_opportunity_memory,
        learned_opportunity_memory,
    )?;
    {
        // Snapshot the post-mutation world state before ending the tick.
        let view = runtime_belief_view(agent, world, scheduler, action_defs, recipe_registry);
        update_runtime_observation_snapshot(&view, agent, runtime);
    }
    Ok(())
}

fn populate_contention_event_refs(
    blocked_memory: &BlockerMemory,
    event_log: &EventLog,
    tick: Tick,
) -> Option<BlockerMemory> {
    let mut updated = None;
    for (key, blocker) in &blocked_memory.intents {
        let BlockingFact::ReservationConflict {
            affordance,
            contention_event: None,
        } = blocker.blocking_fact
        else {
            continue;
        };
        let Some(event_id) = contention_event_for_affordance_at_tick(event_log, affordance, tick)
        else {
            continue;
        };

        let memory = updated.get_or_insert_with(|| blocked_memory.clone());
        let Some(updated_blocker) = memory.intents.get_mut(key) else {
            continue;
        };
        if let BlockingFact::ReservationConflict {
            contention_event, ..
        } = &mut updated_blocker.blocking_fact
        {
            *contention_event = Some(event_id);
        }
    }
    updated
}

fn contention_event_for_affordance_at_tick(
    event_log: &EventLog,
    affordance: AffordanceKey,
    tick: Tick,
) -> Option<EventId> {
    let mut matched = None;
    for event_id in event_log.events_by_tag(EventTag::ContentionResolved) {
        let Some(payload) = event_log
            .get(*event_id)
            .and_then(EventView::contention_event_payload)
        else {
            continue;
        };
        if payload.contested_affordance != affordance || payload.at_tick != tick {
            continue;
        }

        debug_assert!(
            matched.is_none(),
            "multiple ContentionResolved events for affordance {affordance:?} at tick {tick:?}"
        );
        matched.get_or_insert(*event_id);
    }
    matched
}

pub(super) fn resolve_step_targets(
    runtime: &AgentDecisionRuntime,
    step: &PlannedStep,
) -> Option<Vec<EntityId>> {
    crate::resolve_planning_targets_with(&step.targets, |id| {
        runtime.materialization_bindings.resolve(id)
    })
}

pub(super) fn committed_action_for_step<'a>(
    step: &PlannedStep,
    committed_actions: &'a [CommittedAction],
) -> Option<&'a CommittedAction> {
    if committed_actions.len() != 1 {
        return None;
    }
    let committed = &committed_actions[0];
    (committed.def_id == step.def_id).then_some(committed)
}

pub(super) fn apply_step_materialization_bindings(
    runtime: &mut AgentDecisionRuntime,
    step: &PlannedStep,
    outcome: &CommitOutcome,
) -> Result<(), ()> {
    use std::collections::BTreeSet;

    let tags = step
        .expected_materializations
        .iter()
        .map(|expected| expected.tag)
        .chain(outcome.materializations.iter().map(|actual| actual.tag))
        .collect::<BTreeSet<_>>();
    let mut newly_bound_entities = BTreeSet::new();

    for tag in tags {
        let expected = step
            .expected_materializations
            .iter()
            .filter(|expected| expected.tag == tag)
            .collect::<Vec<_>>();
        let actual = outcome
            .materializations
            .iter()
            .filter(|materialization| materialization.tag == tag)
            .collect::<Vec<_>>();
        if expected.len() != actual.len() {
            return Err(());
        }

        for (expected, actual) in expected.into_iter().zip(actual.into_iter()) {
            if !newly_bound_entities.insert(actual.entity) {
                return Err(());
            }
            if let Some(existing) = runtime
                .materialization_bindings
                .resolve(expected.hypothetical_id)
            {
                if existing != actual.entity {
                    return Err(());
                }
                continue;
            }
            runtime
                .materialization_bindings
                .bind(expected.hypothetical_id, actual.entity);
        }
    }

    Ok(())
}

pub(super) fn persist_blocked_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &BlockerMemory,
    after: &BlockerMemory,
    assumption_refs: AssumptionRefContext<'_>,
) -> Result<(), TickInputError> {
    let changed_entries = after
        .intents
        .iter()
        .filter_map(|(key, blocker)| match before.intents.get(key) {
            Some(existing) if existing == blocker => None,
            _ => Some(*blocker),
        })
        .collect::<Vec<_>>();
    let existing = world.get_component_blocker_memory(agent);
    if existing == Some(after)
        || (existing.is_none() && before == after && after.intents.is_empty())
    {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_blocker_memory(agent, after.clone())
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    for blocker in changed_entries {
        let decisive = decisive_evidence_from_blocker(&blocker, assumption_refs.max_assumptions);
        emit_decision_event(
            event_log,
            tick,
            agent,
            EventTag::BlockerRecorded,
            DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                agent,
                blocker_key: blocker.blocker_key,
                discrepancy: None,
                blocking_fact: Some(blocker.blocking_fact),
                expires_tick: blocker.expires_tick,
                belief_snapshot: None,
                decisive_beliefs: decisive.beliefs,
                decisive_records: decisive.records,
                decisive_world_observations: decisive.world_observations,
                assumptions: assumption_refs.to_refs(),
            }),
        );
    }
    Ok(())
}

pub(super) fn persist_discrepancy_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &DiscrepancyMemory,
    after: &DiscrepancyMemory,
    assumption_refs: AssumptionRefContext<'_>,
) -> Result<(), TickInputError> {
    let changed_entries = after
        .entries
        .iter()
        .filter_map(|(key, entry)| match before.entries.get(key) {
            Some(existing) if existing == entry => None,
            _ => Some(*entry),
        })
        .collect::<Vec<_>>();
    let existing = world.get_component_discrepancy_memory(agent);
    if existing == Some(after)
        || (existing.is_none() && before == after && after.entries.is_empty())
    {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_discrepancy_memory(agent, after.clone())
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    for entry in changed_entries {
        let belief_snapshot = belief_snapshot_for_discrepancy_entry(world, agent, tick, &entry);
        let decisive =
            decisive_evidence_from_discrepancy_entry(&entry, assumption_refs.max_assumptions);
        emit_decision_event(
            event_log,
            tick,
            agent,
            EventTag::BlockerRecorded,
            DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload {
                agent,
                blocker_key: entry.blocker_key,
                discrepancy: Some(entry.discrepancy),
                blocking_fact: None,
                expires_tick: entry.expires_tick,
                belief_snapshot,
                decisive_beliefs: decisive.beliefs,
                decisive_records: decisive.records,
                decisive_world_observations: decisive.world_observations,
                assumptions: assumption_refs.to_refs(),
            }),
        );
    }
    Ok(())
}

fn belief_snapshot_for_discrepancy_entry(
    world: &worldwake_core::World,
    agent: EntityId,
    tick: Tick,
    entry: &DiscrepancyEntry,
) -> Option<BeliefSnapshot> {
    if !matches!(
        entry.discrepancy,
        Discrepancy::BeliefStale | Discrepancy::BeliefContradicted
    ) {
        return None;
    }

    let _ = world.get_component_agent_belief_store(agent)?;
    let target = entry.blocker_key.target?;
    let view = PerAgentBeliefView::from_world_at_tick(agent, tick, world);
    let envelope = view.believed_target_location(agent, target);
    let expected = match envelope.status {
        worldwake_sim::belief_view::BeliefStatus::Stale => Discrepancy::BeliefStale,
        worldwake_sim::belief_view::BeliefStatus::Contradicted => Discrepancy::BeliefContradicted,
        worldwake_sim::belief_view::BeliefStatus::Certain
        | worldwake_sim::belief_view::BeliefStatus::Probable
        | worldwake_sim::belief_view::BeliefStatus::Disputed => return None,
    };
    (expected == entry.discrepancy).then_some(BeliefSnapshot {
        confidence: envelope.confidence,
        status: belief_status_tag(envelope.status),
        acquired_tick: envelope.acquired_tick,
    })
}

fn belief_status_tag(status: worldwake_sim::belief_view::BeliefStatus) -> BeliefStatusTag {
    match status {
        worldwake_sim::belief_view::BeliefStatus::Certain => BeliefStatusTag::Certain,
        worldwake_sim::belief_view::BeliefStatus::Probable => BeliefStatusTag::Probable,
        worldwake_sim::belief_view::BeliefStatus::Stale => BeliefStatusTag::Stale,
        worldwake_sim::belief_view::BeliefStatus::Disputed => BeliefStatusTag::Disputed,
        worldwake_sim::belief_view::BeliefStatus::Contradicted => BeliefStatusTag::Contradicted,
    }
}

/// Persist the violation memory component to the world, producing a
/// `ComponentDelta` in the event log. Follows the same diff-and-commit
/// pattern as `persist_blocked_memory`.
pub(super) fn persist_violation_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &worldwake_core::ViolationMemory,
    after: &worldwake_core::ViolationMemory,
) -> Result<(), TickInputError> {
    let existing = world.get_component_violation_memory(agent);
    if existing == Some(after)
        || (existing.is_none() && before == after && after.violations.is_empty())
    {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_violation_memory(agent, after.clone())
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

pub(super) fn persist_repair_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &RepairMemory,
    after: &RepairMemory,
) -> Result<(), TickInputError> {
    let existing = world.get_component_repair_memory(agent);
    if existing == Some(after)
        || (existing.is_none() && before == after && after.repairs.is_empty())
    {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_repair_memory(agent, after.clone())
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

pub(super) fn persist_learned_opportunity_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &LearnedOpportunityMemory,
    after: &LearnedOpportunityMemory,
) -> Result<(), TickInputError> {
    let existing = world.get_component_learned_opportunity_memory(agent);
    if existing == Some(after)
        || (existing.is_none() && before == after && after.opportunities.is_empty())
    {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    txn.set_component_learned_opportunity_memory(agent, after.clone())
        .map_err(|error| TickInputError::new(error.to_string()))?;
    let _ = txn.commit(event_log);
    Ok(())
}

/// Persist the intention frame component to the world, producing a
/// `ComponentDelta` in the event log. Follows the same diff-and-commit
/// pattern as `persist_blocked_memory`.
pub(super) fn persist_intention_frame(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: Option<&worldwake_core::IntentionFrame>,
    after: Option<&worldwake_core::IntentionFrame>,
) -> Result<(), TickInputError> {
    if before == after {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    if let Some(frame) = after {
        txn.set_component_intention_frame(agent, frame.clone())
            .map_err(|error| TickInputError::new(error.to_string()))?;
    } else {
        txn.clear_component_intention_frame(agent)
            .map_err(|error| TickInputError::new(error.to_string()))?;
    }
    let _ = txn.commit(event_log);
    Ok(())
}

/// Persist the facility queue intents component to the world, producing a
/// `ComponentDelta` in the event log. Follows the same diff-and-commit
/// pattern as `persist_intention_frame`.
pub(super) fn persist_facility_queue_intents(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &ContentionIntents,
    after: &ContentionIntents,
) -> Result<(), TickInputError> {
    if before == after {
        return Ok(());
    }

    let mut txn = WorldTxn::new(
        world,
        tick,
        CauseRef::SystemTick(tick),
        Some(agent),
        None,
        VisibilitySpec::Hidden,
        WitnessData::default(),
    );
    if after.intents.is_empty() {
        txn.clear_component_contention_intents(agent)
            .map_err(|error| TickInputError::new(error.to_string()))?;
    } else {
        txn.set_component_contention_intents(agent, after.clone())
            .map_err(|error| TickInputError::new(error.to_string()))?;
    }
    let _ = txn.commit(event_log);
    Ok(())
}

pub(super) fn current_step(runtime: &AgentDecisionRuntime) -> Option<&PlannedStep> {
    runtime
        .current_plan
        .as_ref()
        .and_then(|plan| plan.steps.get(runtime.current_step_index))
}

pub(super) fn plan_finished(runtime: &AgentDecisionRuntime) -> bool {
    runtime.current_plan.as_ref().is_some_and(|plan| {
        runtime.current_step_index >= plan.steps.len() && !runtime.step_in_flight
    })
}

#[cfg(test)]
mod tests {
    use super::{
        apply_repaired_plan_and_emit, attempt_local_repair_for_invalidated_step,
        breach_signature_for_step, populate_contention_event_refs, record_failed_repair_attempts,
        repair_attempt_trace_from_failed, repair_budget_consumed,
    };
    use crate::RepairOutcome;
    use crate::{PlanGuard, PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind};
    use std::collections::{BTreeMap, BTreeSet};
    use worldwake_core::{
        ActionDefId, AffordanceKey, Blocker, BlockerClearingCondition, BlockerKey, BlockerMemory,
        BlockingFact, BreachSignature, CausalLink, CausalProvider, CauseRef, ClaimantOutcome,
        ContentionClaimant, ContentionEventPayload, ContentionResolutionRule, DecisionEventPayload,
        EntityId, EventLog, EventPayload, EventTag, EventView, GoalKey, GoalKind, InvalidatorTag,
        MismatchDetail, OpportunityKey, PendingEvent, Permille, PlanningFact, RepairAppliedPayload,
        RepairKind, RepairMemory, Tick, VisibilitySpec, WitnessData, test_utils::sample_goal_key,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn affordance(facility_slot: u32, action: u32) -> AffordanceKey {
        AffordanceKey {
            facility: entity(facility_slot),
            action: ActionDefId(action),
        }
    }

    fn opportunity(goal_key: GoalKey) -> OpportunityKey {
        OpportunityKey {
            goal_key,
            anchor: worldwake_core::OpportunityAnchor::None,
        }
    }

    fn causal_link(consumer_step_index: u16) -> CausalLink {
        CausalLink {
            provider: CausalProvider::PriorStep { step_index: 0 },
            fact: PlanningFact::TargetPresent {
                target: entity(7),
                at_place: entity(8),
            },
            consumer_step_index,
            source_tick: Tick(3),
            confidence: Permille::new(800).unwrap(),
        }
    }

    fn observed_target_link(consumer_step_index: u16) -> CausalLink {
        CausalLink {
            provider: CausalProvider::Observation {
                observed_entity: entity(7),
                aspect: worldwake_core::EntityBeliefAspect::Location,
            },
            fact: PlanningFact::TargetPresent {
                target: entity(7),
                at_place: entity(8),
            },
            consumer_step_index,
            source_tick: Tick(3),
            confidence: Permille::new(800).unwrap(),
        }
    }

    fn planned_step(slot: u32, link: Option<CausalLink>) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(slot),
            targets: vec![crate::PlanningEntityRef::Authoritative(entity(slot))],
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

    fn repair_mismatch() -> MismatchDetail {
        MismatchDetail::GuardInvalidator(InvalidatorTag::TargetMoved)
    }

    fn blocker_memory_for(affordance: AffordanceKey) -> BlockerMemory {
        let blocker_key = BlockerKey {
            goal_key: GoalKey {
                kind: GoalKind::Sleep,
                commodity: None,
                entity: Some(affordance.facility),
                place: Some(entity(99)),
            },
            place: Some(entity(99)),
            target: Some(affordance.facility),
            action_def: Some(affordance.action),
        };
        let mut memory = BlockerMemory::default();
        memory.record(Blocker {
            blocker_key,
            blocking_fact: BlockingFact::ReservationConflict {
                affordance,
                contention_event: None,
            },
            diagnostic_context: None,
            observed_tick: Tick(40),
            expires_tick: Tick(50),
            clearing_condition: BlockerClearingCondition::ContentionChanged {
                facility: affordance.facility,
            },
            baseline_snapshot: None,
        });
        memory
    }

    fn emit_contention_event(
        event_log: &mut EventLog,
        affordance: AffordanceKey,
        tick: Tick,
    ) -> worldwake_core::EventId {
        event_log.emit(PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::SystemTick(tick),
            actor_id: None,
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: Some(entity(99)),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: BTreeSet::from([EventTag::ContentionResolved]),
            contention_event_payload: Some(ContentionEventPayload {
                contested_affordance: affordance,
                place: entity(99),
                resolution_rule: ContentionResolutionRule::ArrivalTime,
                claimants: vec![ContentionClaimant {
                    agent: entity(7),
                    arrived_tick: Tick(39),
                    queue_position: 1,
                    outcome: ClaimantOutcome::Granted,
                }],
                total_claimants: 1,
                winner: Some(entity(7)),
                at_tick: tick,
            }),
            decision_payload: None,
            artifact_transition_payload: None,
        }))
    }

    fn recorded_contention_event(memory: &BlockerMemory) -> Option<worldwake_core::EventId> {
        let blocker = memory.intents.values().next().expect("missing blocker");
        let BlockingFact::ReservationConflict {
            contention_event, ..
        } = blocker.blocking_fact
        else {
            panic!("expected reservation conflict blocker");
        };
        contention_event
    }

    #[test]
    fn local_repair_success_emits_repair_applied_and_replaces_plan() {
        let agent = entity(1);
        let goal_key = sample_goal_key();
        let prefix = planned_step(10, None);
        let failed = planned_step(11, Some(causal_link(1)));
        let plan = PlannedPlan::new(
            opportunity(goal_key),
            goal_key,
            vec![prefix.clone(), failed.clone()],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = crate::AgentDecisionRuntime {
            current_plan: Some(plan),
            current_step_index: 1,
            ..crate::AgentDecisionRuntime::default()
        };
        let cognitive = cognitive(4, Permille::new(1000).unwrap());
        let outcome = attempt_local_repair_for_invalidated_step(
            &runtime,
            &RepairMemory::default(),
            &cognitive,
            Tick(9),
            goal_key,
            &failed,
            worldwake_core::PlanInvalidationReason::ExpectationMismatch { step_index: 1 },
            Some(repair_mismatch()),
        )
        .expect("causal-link invalidator should attempt localized repair");
        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!(
                "visible discrepancy with a committed prefix should repair to a progress barrier"
            );
        };
        assert_eq!(kind, RepairKind::DowngradeToProgressBarrier);

        let mut event_log = EventLog::new();
        apply_repaired_plan_and_emit(
            &mut event_log,
            &mut runtime,
            Tick(9),
            agent,
            goal_key,
            1,
            kind,
            *new_plan,
        );

        let repaired_plan = runtime
            .current_plan
            .as_ref()
            .expect("repaired plan should stay active");
        assert_eq!(
            repaired_plan.terminal_kind,
            PlanTerminalKind::ProgressBarrier
        );
        assert_eq!(repaired_plan.steps, vec![prefix]);
        assert_eq!(runtime.current_step_index, 1);
        let events = event_log.events_by_tag(EventTag::RepairApplied);
        assert_eq!(events.len(), 1);
        let payload = event_log
            .get(events[0])
            .and_then(|event| event.decision_payload())
            .expect("repair event should carry a payload");
        assert_eq!(
            payload,
            &DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                agent,
                goal_key,
                step_index: 1,
                repair_kind: RepairKind::DowngradeToProgressBarrier,
                substitute_target: None,
                substitute_recipe: None,
            })
        );
    }

    #[test]
    fn suffix_sourced_local_repair_promotes_candidate_without_duplication() {
        let agent = entity(1);
        let goal_key = sample_goal_key();
        let prefix = planned_step(10, None);
        let failed = planned_step(11, Some(observed_target_link(1)));
        let promoted = planned_step(7, Some(observed_target_link(2)));
        let plan = PlannedPlan::new(
            opportunity(goal_key),
            goal_key,
            vec![prefix.clone(), failed.clone(), promoted.clone()],
            PlanTerminalKind::GoalSatisfied,
        );
        let mut runtime = crate::AgentDecisionRuntime {
            current_plan: Some(plan),
            current_step_index: 1,
            ..crate::AgentDecisionRuntime::default()
        };
        let cognitive = cognitive(4, Permille::new(1000).unwrap());
        let outcome = attempt_local_repair_for_invalidated_step(
            &runtime,
            &RepairMemory::default(),
            &cognitive,
            Tick(9),
            goal_key,
            &failed,
            worldwake_core::PlanInvalidationReason::ExpectationMismatch { step_index: 1 },
            Some(repair_mismatch()),
        )
        .expect("causal-link invalidator should attempt localized repair");
        let RepairOutcome::Repaired { kind, new_plan, .. } = outcome else {
            panic!("suffix candidate should repair the invalidated step");
        };
        assert_eq!(kind, RepairKind::RebindTarget);

        let mut event_log = EventLog::new();
        apply_repaired_plan_and_emit(
            &mut event_log,
            &mut runtime,
            Tick(9),
            agent,
            goal_key,
            1,
            kind,
            *new_plan,
        );

        let repaired_plan = runtime
            .current_plan
            .as_ref()
            .expect("repaired plan should stay active");
        assert_eq!(repaired_plan.steps, vec![prefix, promoted]);
        assert_eq!(runtime.current_step_index, 1);
        let events = event_log.events_by_tag(EventTag::RepairApplied);
        assert_eq!(events.len(), 1);
        let payload = event_log
            .get(events[0])
            .and_then(|event| event.decision_payload())
            .expect("repair event should carry a payload");
        assert_eq!(
            payload,
            &DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                agent,
                goal_key,
                step_index: 1,
                repair_kind: RepairKind::RebindTarget,
                substitute_target: Some(entity(7)),
                substitute_recipe: None,
            })
        );
    }

    #[test]
    fn local_repair_failure_records_failed_attempts_in_repair_memory() {
        let goal_key = sample_goal_key();
        let failed = planned_step(11, Some(causal_link(1)));
        let plan = PlannedPlan::new(
            opportunity(goal_key),
            goal_key,
            vec![planned_step(10, None), failed.clone()],
            PlanTerminalKind::GoalSatisfied,
        );
        let runtime = crate::AgentDecisionRuntime {
            current_plan: Some(plan),
            current_step_index: 1,
            ..crate::AgentDecisionRuntime::default()
        };
        let cognitive = cognitive(0, Permille::new(1000).unwrap());
        let outcome = attempt_local_repair_for_invalidated_step(
            &runtime,
            &RepairMemory::default(),
            &cognitive,
            Tick(9),
            goal_key,
            &failed,
            worldwake_core::PlanInvalidationReason::ExpectationMismatch { step_index: 1 },
            Some(repair_mismatch()),
        )
        .expect("causal-link invalidator should attempt localized repair");
        let RepairOutcome::Failed { tried } = outcome else {
            panic!("zero repair budget should fail before a local repair succeeds");
        };
        let signature = breach_signature_for_step(goal_key, &failed, Some(repair_mismatch()));
        let mut repair_memory = RepairMemory::default();
        record_failed_repair_attempts(&mut repair_memory, signature, &tried, Tick(9), 20);

        assert_eq!(tried.len(), 1);
        let entry = repair_memory
            .repairs
            .get(&BreachSignature {
                goal_key,
                invalidator: InvalidatorTag::TargetMoved,
                step_target: Some(entity(11)),
            })
            .expect("failed repair attempt should be recorded");
        assert_eq!(entry.kind, RepairKind::RebindTarget);
        assert!(!entry.succeeded);
        assert_eq!(entry.observed_tick, Tick(9));
        assert_eq!(entry.expires_tick, Tick(29));
        assert_eq!(entry.success_count, 0);
    }

    #[test]
    fn failed_local_repair_attempt_trace_records_budget_and_rejections() {
        let goal_key = sample_goal_key();
        let signature = BreachSignature {
            goal_key,
            invalidator: InvalidatorTag::TargetMoved,
            step_target: Some(entity(11)),
        };
        let tried = vec![
            (
                RepairKind::RebindTarget,
                crate::RepairFailure::NoSiblingTargetFound,
            ),
            (
                RepairKind::InsertVerification,
                crate::RepairFailure::NoEpistemicSubstrate,
            ),
            (RepairKind::Abandon, crate::RepairFailure::BudgetExhausted),
        ];
        let cognitive = cognitive(12, Permille::new(500).unwrap());

        let trace = repair_attempt_trace_from_failed(signature, &tried, &cognitive);

        assert_eq!(trace.breach, signature);
        assert_eq!(trace.chosen_kind, None);
        assert_eq!(trace.rejected, tried);
        assert_eq!(trace.budget_consumed, 2);
        assert_eq!(trace.budget_total, 6);
    }

    #[test]
    fn successful_local_repair_budget_consumed_includes_chosen_attempt() {
        let rejected = vec![(
            RepairKind::RebindTarget,
            crate::RepairFailure::NoSiblingTargetFound,
        )];

        assert_eq!(repair_budget_consumed(&rejected, true), 2);
    }

    #[test]
    fn populate_contention_event_ref_sets_matching_resolution_event() {
        let affordance = affordance(10, 3);
        let mut event_log = EventLog::new();
        let event_id = emit_contention_event(&mut event_log, affordance, Tick(40));
        let memory = blocker_memory_for(affordance);

        let populated = populate_contention_event_refs(&memory, &event_log, Tick(40))
            .expect("expected contention event population");

        assert_eq!(recorded_contention_event(&populated), Some(event_id));
    }

    #[test]
    fn populate_contention_event_ref_ignores_affordance_mismatch() {
        let blocker_affordance = affordance(10, 3);
        let mut event_log = EventLog::new();
        emit_contention_event(&mut event_log, affordance(11, 3), Tick(40));
        let memory = blocker_memory_for(blocker_affordance);

        assert!(populate_contention_event_refs(&memory, &event_log, Tick(40)).is_none());
    }

    #[test]
    fn populate_contention_event_ref_ignores_tick_mismatch() {
        let affordance = affordance(10, 3);
        let mut event_log = EventLog::new();
        emit_contention_event(&mut event_log, affordance, Tick(39));
        let memory = blocker_memory_for(affordance);

        assert!(populate_contention_event_refs(&memory, &event_log, Tick(40)).is_none());
    }
}
