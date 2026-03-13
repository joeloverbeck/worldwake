//! Action command handlers: actions, do, cancel.

use worldwake_sim::{
    get_affordances, ActionRequestMode, InputKind, OmniscientBeliefRuntime, OmniscientBeliefView,
    SimulationState,
};
use worldwake_systems::ActionRegistries;

use crate::commands::{CommandError, CommandOutcome, CommandResult};
use crate::display::entity_display_name;
use crate::repl::ReplState;

/// List available actions for the controlled agent.
///
/// Queries affordances via `get_affordances()` (same query as AI agents),
/// stores them in `repl_state.last_affordances`, and prints a numbered menu.
pub fn handle_actions(
    sim: &SimulationState,
    registries: &ActionRegistries,
    repl_state: &mut ReplState,
) -> CommandResult {
    let entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent"))?;

    let runtime = OmniscientBeliefRuntime::new(
        sim.scheduler().active_actions(),
        &registries.defs,
    );
    let view = OmniscientBeliefView::with_runtime(sim.world(), runtime);

    let affordances = get_affordances(&view, entity, &registries.defs, &registries.handlers);

    if affordances.is_empty() {
        println!("no actions available");
        repl_state.last_affordances.clear();
        return Ok(CommandOutcome::Continue);
    }

    println!("Available actions:");
    for (i, affordance) in affordances.iter().enumerate() {
        let action_name = registries
            .defs
            .get(affordance.def_id)
            .map_or("unknown", |def| def.name.as_str());

        let targets_str = if affordance.bound_targets.is_empty() {
            String::new()
        } else {
            let names: Vec<String> = affordance
                .bound_targets
                .iter()
                .map(|t| entity_display_name(sim.world(), *t))
                .collect();
            format!(" ({})", names.join(", "))
        };

        let duration_str = if let Some(def) = registries.defs.get(affordance.def_id) {
            format_duration_estimate(&def.duration)
        } else {
            String::new()
        };

        println!("  {}. {action_name}{targets_str}{duration_str}", i + 1);
    }

    repl_state.last_affordances = affordances;
    Ok(CommandOutcome::Continue)
}

/// Execute an action by menu number from the last `actions` output.
///
/// Creates an `InputKind::RequestAction` and enqueues it in the input queue.
/// The action won't execute until the next `tick`.
pub fn handle_do(
    n: usize,
    sim: &mut SimulationState,
    registries: &ActionRegistries,
    repl_state: &ReplState,
) -> CommandResult {
    let _entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent"))?;

    if repl_state.last_affordances.is_empty() {
        return Err(CommandError::new(
            "run 'actions' first to see available actions",
        ));
    }

    if n == 0 || n > repl_state.last_affordances.len() {
        return Err(CommandError::new(
            "invalid action number, run 'actions' first",
        ));
    }

    let affordance = &repl_state.last_affordances[n - 1];

    let action_name = registries
        .defs
        .get(affordance.def_id)
        .map_or("unknown", |def| def.name.as_str());

    let tick = sim.scheduler().current_tick();
    sim.scheduler_mut().input_queue_mut().enqueue(
        tick,
        InputKind::RequestAction {
            actor: affordance.actor,
            def_id: affordance.def_id,
            targets: affordance.bound_targets.clone(),
            payload_override: affordance.payload_override.clone(),
            mode: ActionRequestMode::Strict,
        },
    );

    println!("Requested: {action_name}");
    Ok(CommandOutcome::Continue)
}

/// Cancel the current action for the controlled agent.
///
/// Creates an `InputKind::CancelAction` and enqueues it in the input queue.
pub fn handle_cancel(sim: &mut SimulationState) -> CommandResult {
    let entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent"))?;

    // Find the active action for this agent.
    let active = sim
        .scheduler()
        .active_actions()
        .iter()
        .find(|(_, instance)| instance.actor == entity)
        .map(|(id, _)| *id);

    let Some(action_instance_id) = active else {
        println!("no action to cancel");
        return Ok(CommandOutcome::Continue);
    };

    let tick = sim.scheduler().current_tick();
    sim.scheduler_mut().input_queue_mut().enqueue(
        tick,
        InputKind::CancelAction {
            actor: entity,
            action_instance_id,
        },
    );

    println!("Cancel requested");
    Ok(CommandOutcome::Continue)
}

/// Format a duration estimate from a `DurationExpr` for display.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn format_duration_estimate(duration: &worldwake_sim::DurationExpr) -> String {
    match duration {
        worldwake_sim::DurationExpr::Fixed(n) => format!(" — {} ticks", n.get()),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repl::ReplState;
    use crate::scenario::{spawn_scenario, types::*, SpawnedSimulation};
    use worldwake_ai::{AgentTickDriver, PlanningBudget};
    use worldwake_core::{
        control::ControlSource,
        ids::EntityId,
        items::CommodityKind,
        needs::HomeostaticNeeds,
        numerics::{Permille, Quantity},
        topology::PlaceTag,
    };
    use worldwake_sim::InputKind;

    fn pm(v: u16) -> Permille {
        Permille::new(v).unwrap()
    }

    /// Scenario with a human agent at a village with food available (enables eat affordance).
    fn human_with_food_scenario() -> (SpawnedSimulation, EntityId) {
        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Aster".into(),
                location: "Village".into(),
                control: ControlSource::Human,
                needs: Some(HomeostaticNeeds::new(
                    pm(600),
                    pm(600),
                    pm(600),
                    pm(600),
                    pm(600),
                )),
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![ItemDef {
                commodity: CommodityKind::Apple,
                quantity: Quantity(5),
                location: "Aster".into(),
                container: false,
            }],
            facilities: vec![],
            resource_sources: vec![],
        };
        let spawned = spawn_scenario(&def).unwrap();
        let agent_id = spawned
            .state
            .world()
            .entities_with_name_and_agent_data()
            .next()
            .unwrap();
        (spawned, agent_id)
    }

    /// Scenario with an observer (no controlled agent).
    fn observer_scenario() -> SpawnedSimulation {
        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Village".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Kael".into(),
                location: "Village".into(),
                control: ControlSource::Ai,
                needs: None,
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![],
            facilities: vec![],
            resource_sources: vec![],
        };
        spawn_scenario(&def).unwrap()
    }

    #[test]
    fn test_actions_lists_affordances() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let sim = spawned.state;
        let mut repl_state = ReplState::new();

        let result = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);
        // Agent with food should have at least an eat affordance.
        assert!(
            !repl_state.last_affordances.is_empty(),
            "should have at least one affordance"
        );
    }

    #[test]
    fn test_actions_stores_in_repl_state() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let sim = spawned.state;
        let mut repl_state = ReplState::new();

        assert!(repl_state.last_affordances.is_empty());
        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        assert!(!repl_state.last_affordances.is_empty());
    }

    #[test]
    fn test_do_enqueues_input() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        // First populate affordances.
        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        assert!(!repl_state.last_affordances.is_empty());

        let queue_before = sim.scheduler().input_queue().len();
        let result = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);

        // Input queue should have one more event.
        assert_eq!(sim.scheduler().input_queue().len(), queue_before + 1);

        // Verify it's a RequestAction.
        let tick = sim.scheduler().current_tick();
        let events = sim.scheduler().input_queue().peek_tick(tick);
        let last = events.last().unwrap();
        assert!(
            matches!(last.kind, InputKind::RequestAction { .. }),
            "expected RequestAction, got {:?}",
            last.kind
        );
    }

    #[test]
    fn test_do_out_of_range() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        let n = repl_state.last_affordances.len() + 1;

        let result = handle_do(n, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid action number"));
    }

    #[test]
    fn test_do_zero_out_of_range() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);

        let result = handle_do(0, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid action number"));
    }

    #[test]
    fn test_do_before_actions() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let repl_state = ReplState::new();

        let result = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("run 'actions' first"));
    }

    #[test]
    fn test_cancel_enqueues_input() {
        let (spawned, _agent_id) = human_with_food_scenario();
        let mut sim = spawned.state;
        let mut repl_state = ReplState::new();

        // Start an action first: get affordances, do one, then tick to start it.
        let _ = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        if repl_state.last_affordances.is_empty() {
            // No affordances — cancel should say "no action to cancel".
            let result = handle_cancel(&mut sim);
            assert_eq!(result.unwrap(), CommandOutcome::Continue);
            return;
        }

        let _ = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);

        // Tick to process the request and start the action.
        let mut driver = AgentTickDriver::new(PlanningBudget::default());
        let _ = crate::handlers::tick::handle_tick(
            1,
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
        );

        // Now check if there's an active action for the agent.
        let entity = sim.controller_state().controlled_entity().unwrap();
        let has_active = sim
            .scheduler()
            .active_actions()
            .values()
            .any(|a| a.actor == entity);

        if has_active {
            let queue_before = sim.scheduler().input_queue().len();
            let result = handle_cancel(&mut sim);
            assert_eq!(result.unwrap(), CommandOutcome::Continue);
            assert_eq!(sim.scheduler().input_queue().len(), queue_before + 1);

            let tick = sim.scheduler().current_tick();
            let events = sim.scheduler().input_queue().peek_tick(tick);
            let last = events.last().unwrap();
            assert!(
                matches!(last.kind, InputKind::CancelAction { .. }),
                "expected CancelAction, got {:?}",
                last.kind
            );
        }
    }

    #[test]
    fn test_actions_no_controlled_agent() {
        let spawned = observer_scenario();
        let sim = spawned.state;
        let mut repl_state = ReplState::new();

        let result = handle_actions(&sim, &spawned.action_registries, &mut repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("no controlled agent"));
    }

    #[test]
    fn test_do_no_controlled_agent() {
        let spawned = observer_scenario();
        let mut sim = spawned.state;
        let repl_state = ReplState::new();

        let result = handle_do(1, &mut sim, &spawned.action_registries, &repl_state);
        let err = result.unwrap_err();
        assert!(err.message.contains("no controlled agent"));
    }

    #[test]
    fn test_cancel_no_controlled_agent() {
        let spawned = observer_scenario();
        let mut sim = spawned.state;

        let result = handle_cancel(&mut sim);
        let err = result.unwrap_err();
        assert!(err.message.contains("no controlled agent"));
    }
}
