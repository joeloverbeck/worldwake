//! Tick and status command handlers.

use worldwake_ai::AgentTickDriver;
use worldwake_core::{drives::ThresholdBand, numerics::Permille};
use worldwake_sim::{
    step_tick, ActionDuration, AutonomousControllerRuntime, SimulationState, SystemDispatchTable,
    TickStepServices,
};
use worldwake_systems::ActionRegistries;

use crate::commands::{CommandError, CommandOutcome, CommandResult};
use crate::display::{
    entity_display_name, format_control_source, format_location, format_needs_bar,
};

/// Display-only threshold band for needs bars.
///
/// This is a derived read-model for the CLI, not authoritative state.
/// When per-agent threshold bands exist (E14+), this should be replaced.
fn default_display_band() -> ThresholdBand {
    ThresholdBand::new(
        Permille::new(250).unwrap(),
        Permille::new(500).unwrap(),
        Permille::new(750).unwrap(),
        Permille::new(900).unwrap(),
    )
    .unwrap()
}

/// Advance the simulation by `n` ticks, running AI each tick.
///
/// Wraps `driver` in `AutonomousControllerRuntime` as the `TickInputProducer`
/// and calls `step_tick()` for each tick.
pub fn handle_tick(
    n: u32,
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    registries: &ActionRegistries,
    dispatch_table: &SystemDispatchTable,
) -> CommandResult {
    if n == 0 {
        println!("nothing to do");
        return Ok(CommandOutcome::Continue);
    }

    for _ in 0..n {
        let mut controllers = AutonomousControllerRuntime::new(vec![driver]);

        let (world, event_log, scheduler, controller, rng, recipe_registry) = sim.tick_parts_mut();

        let result = step_tick(
            world,
            event_log,
            scheduler,
            controller,
            rng,
            TickStepServices {
                action_defs: &registries.defs,
                action_handlers: &registries.handlers,
                recipe_registry,
                systems: dispatch_table,
                input_producer: Some(&mut controllers),
                action_trace: None,
                request_resolution_trace: None,
                politics_trace: None,
            },
        )
        .map_err(|e| CommandError::new(format!("tick error: {e:?}")))?;

        println!(
            "--- Tick {} --- ({} events)",
            result.tick.0, result.events_emitted_count
        );
    }

    Ok(CommandOutcome::Continue)
}

/// Show the controlled agent's status.
pub fn handle_status(sim: &SimulationState, registries: &ActionRegistries) -> CommandResult {
    let entity = sim
        .controller_state()
        .controlled_entity()
        .ok_or_else(|| CommandError::new("no controlled agent (observer mode)"))?;

    let world = sim.world();

    // Agent name and location.
    let name = entity_display_name(world, entity);
    let location = format_location(world, entity);
    println!("{name} {location}");

    // Current action (if any).
    let mut has_action = false;
    for action in sim.scheduler().active_actions().values() {
        if action.actor == entity {
            let action_name = registries
                .defs
                .get(action.def_id)
                .map_or("unknown", |def| def.name.as_str());
            let remaining = match action.remaining_duration {
                ActionDuration::Finite(t) => format!("{t} ticks remaining"),
                ActionDuration::Indefinite => "indefinite".to_string(),
            };
            println!("action: {action_name} ({remaining})");
            has_action = true;
            break;
        }
    }
    if !has_action {
        println!("action: idle");
    }

    // Homeostatic needs.
    let band = default_display_band();
    if let Some(needs) = world.get_component_homeostatic_needs(entity) {
        println!("{}", format_needs_bar("hunger", needs.hunger, &band));
        println!("{}", format_needs_bar("thirst", needs.thirst, &band));
        println!("{}", format_needs_bar("fatigue", needs.fatigue, &band));
        println!("{}", format_needs_bar("bladder", needs.bladder, &band));
        println!("{}", format_needs_bar("dirtiness", needs.dirtiness, &band));
    } else {
        println!("(no needs data)");
    }

    // Wounds.
    if let Some(wound_list) = world.get_component_wound_list(entity) {
        let count = wound_list.wounds.len();
        if count > 0 {
            println!("wounds: {count}");
        }
    }

    // Control source.
    if let Some(agent_data) = world.get_component_agent_data(entity) {
        println!("{}", format_control_source(agent_data.control_source));
    }

    Ok(CommandOutcome::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{spawn_scenario, types::*, SpawnedSimulation};
    use worldwake_ai::PlanningBudget;
    use worldwake_core::{
        control::ControlSource, ids::EntityId, needs::HomeostaticNeeds, numerics::Permille,
        topology::PlaceTag,
    };

    fn pm(v: u16) -> Permille {
        Permille::new(v).unwrap()
    }

    /// Build a scenario with one AI agent that has needs, spawned at a named place.
    fn ai_agent_scenario() -> (SpawnedSimulation, EntityId) {
        let def = ScenarioDef {
            seed: 42,
            places: vec![PlaceDef {
                name: "Market Square".into(),
                tags: vec![PlaceTag::Village],
            }],
            edges: vec![],
            agents: vec![AgentDef {
                name: "Kael".into(),
                location: "Market Square".into(),
                control: ControlSource::Ai,
                needs: Some(HomeostaticNeeds::new(
                    pm(100),
                    pm(200),
                    pm(50),
                    pm(30),
                    pm(10),
                )),
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![],
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

    /// Build a scenario with a human-controlled agent.
    fn human_agent_scenario() -> (SpawnedSimulation, EntityId) {
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
                    pm(300),
                    pm(400),
                    pm(100),
                    pm(50),
                    pm(20),
                )),
                combat_profile: None,
                utility_profile: None,
                merchandise_profile: None,
                trade_disposition: None,
            }],
            items: vec![],
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

    #[test]
    fn test_tick_advances_simulation() {
        let (spawned, _) = ai_agent_scenario();
        let mut sim = spawned.state;
        let mut driver = AgentTickDriver::new(PlanningBudget::default());
        let before = sim.scheduler().current_tick();

        let result = handle_tick(
            1,
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
        );

        assert!(result.is_ok());
        let after = sim.scheduler().current_tick();
        assert_eq!(after.0, before.0 + 1);
    }

    #[test]
    fn test_tick_n_advances_n() {
        let (spawned, _) = ai_agent_scenario();
        let mut sim = spawned.state;
        let mut driver = AgentTickDriver::new(PlanningBudget::default());
        let before = sim.scheduler().current_tick();

        let result = handle_tick(
            5,
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
        );

        assert!(result.is_ok());
        let after = sim.scheduler().current_tick();
        assert_eq!(after.0, before.0 + 5);
    }

    #[test]
    fn test_tick_runs_ai() {
        let (spawned, _) = ai_agent_scenario();
        let mut sim = spawned.state;
        let mut driver = AgentTickDriver::new(PlanningBudget::default());
        let events_before = sim.event_log().len();

        let result = handle_tick(
            3,
            &mut sim,
            &mut driver,
            &spawned.action_registries,
            &spawned.dispatch_table,
        );

        assert!(result.is_ok());
        // After 3 ticks with an AI agent, the event log should have grown
        // (at minimum, system tick events are emitted).
        assert!(
            sim.event_log().len() > events_before,
            "event log should grow after ticking with AI agent"
        );
    }

    #[test]
    fn test_status_shows_needs() {
        let (spawned, _agent_id) = human_agent_scenario();
        let sim = spawned.state;

        // handle_status prints needs; we verify it succeeds with needs data present.
        let result = handle_status(&sim, &spawned.action_registries);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);

        // Verify the agent actually has needs data (precondition for display).
        let entity = sim.controller_state().controlled_entity().unwrap();
        let needs = sim
            .world()
            .get_component_homeostatic_needs(entity)
            .expect("agent should have HomeostaticNeeds");
        // All 5 need fields are non-default (set in scenario).
        assert!(needs.hunger.value() > 0);
        assert!(needs.thirst.value() > 0);
        assert!(needs.fatigue.value() > 0);
        assert!(needs.bladder.value() > 0);
        assert!(needs.dirtiness.value() > 0);
    }

    #[test]
    fn test_status_shows_location() {
        let (spawned, _agent_id) = human_agent_scenario();
        let sim = spawned.state;

        let result = handle_status(&sim, &spawned.action_registries);
        assert_eq!(result.unwrap(), CommandOutcome::Continue);

        // Verify the agent is placed at the expected location.
        let entity = sim.controller_state().controlled_entity().unwrap();
        let place_id = sim
            .world()
            .effective_place(entity)
            .expect("agent should be placed");
        let place = sim
            .world()
            .topology()
            .place(place_id)
            .expect("place should exist");
        assert_eq!(place.name, "Village");
    }

    #[test]
    fn test_status_no_controlled_agent() {
        let (spawned, _) = ai_agent_scenario();
        // No human agent → observer mode (no controlled entity).
        let sim = spawned.state;

        let result = handle_status(&sim, &spawned.action_registries);
        let err = result.unwrap_err();
        assert!(
            err.message.contains("no controlled agent"),
            "expected observer mode error, got: {}",
            err.message
        );
    }
}
