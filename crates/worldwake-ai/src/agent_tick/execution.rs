use super::active_action::handle_current_step_failure;
use super::observation::update_runtime_observation_snapshot;
use super::{AgentTickContext, handle_recoverable_travel_step_blockage, runtime_belief_view};
use crate::{AgentDecisionRuntime, PlannedStep};
use worldwake_core::{
    ActiveGoal, BlockerMemory, CauseRef, ContentionIntents, DiscrepancyMemory, EntityId,
    LearnedOpportunityMemory, RepairMemory, Tick, VisibilitySpec, WitnessData, WorldTxn,
};
use worldwake_sim::{CommitOutcome, CommittedAction, InputKind, Scheduler, TickInputError};

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
    repair_memory: &RepairMemory,
    original_learned_opportunity_memory: &LearnedOpportunityMemory,
    learned_opportunity_memory: &LearnedOpportunityMemory,
    step: &PlannedStep,
    valid: bool,
) -> Result<(), TickInputError> {
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
            return Ok(());
        }
        return handle_current_step_failure(
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
        );
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
            );
        }
        handle_current_step_failure(
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
        )?;
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
) -> Result<(), TickInputError> {
    persist_blocked_memory(
        world,
        event_log,
        agent,
        tick,
        original_blocked,
        blocked_memory,
    )?;
    persist_discrepancy_memory(
        world,
        event_log,
        agent,
        tick,
        original_discrepancy_memory,
        discrepancy_memory,
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
) -> Result<(), TickInputError> {
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
    Ok(())
}

pub(super) fn persist_discrepancy_memory(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: &DiscrepancyMemory,
    after: &DiscrepancyMemory,
) -> Result<(), TickInputError> {
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
    Ok(())
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

/// Persist the active goal component to the world, producing a
/// `ComponentDelta` in the event log. Follows the same diff-and-commit
/// pattern as `persist_intention_frame`.
pub(super) fn persist_active_goal(
    world: &mut worldwake_core::World,
    event_log: &mut worldwake_core::EventLog,
    agent: EntityId,
    tick: Tick,
    before: Option<&ActiveGoal>,
    after: Option<&ActiveGoal>,
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
    if let Some(goal) = after {
        txn.set_component_active_goal(agent, *goal)
            .map_err(|error| TickInputError::new(error.to_string()))?;
    } else {
        txn.clear_component_active_goal(agent)
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
