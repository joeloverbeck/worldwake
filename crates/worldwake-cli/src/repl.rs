use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::error::Error;
use worldwake_ai::AgentTickDriver;
use worldwake_core::Name;
use worldwake_sim::{Affordance, SimulationState, SystemDispatchTable};
use worldwake_systems::ActionRegistries;

/// Ephemeral UI state for the REPL session (not serialized, not part of simulation).
pub struct ReplState {
    pub last_affordances: Vec<Affordance>,
}

impl ReplState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_affordances: Vec::new(),
        }
    }
}

impl Default for ReplState {
    fn default() -> Self {
        Self::new()
    }
}

/// Format the REPL prompt based on current simulation state.
///
/// - With a controlled agent: `"[tick {t}] {agent_name} @ {place_name} > "`
/// - Without (observer mode): `"[tick {t}] observer > "`
pub fn format_prompt(sim: &SimulationState) -> String {
    let tick = sim.scheduler().current_tick().0;

    let Some(entity) = sim.controller_state().controlled_entity() else {
        return format!("[tick {tick}] observer > ");
    };

    let agent_name = sim
        .world()
        .get_component_name(entity)
        .map_or("???", |Name(n)| n.as_str());

    let place_name = sim
        .world()
        .effective_place(entity)
        .and_then(|place_id| sim.world().topology().place(place_id))
        .map_or("???", |place| place.name.as_str());

    format!("[tick {tick}] {agent_name} @ {place_name} > ")
}

/// Run the interactive REPL loop.
///
/// Reads commands from stdin, dispatches them, and loops until quit/EOF/interrupt.
/// Actual command parsing and dispatch are deferred to later tickets (004+).
pub fn run_repl(
    sim: &mut SimulationState,
    _driver: &mut AgentTickDriver,
    _registries: &ActionRegistries,
    _dispatch_table: &SystemDispatchTable,
) -> Result<(), Box<dyn Error>> {
    let mut editor = DefaultEditor::new()?;
    let mut _repl_state = ReplState::new();

    loop {
        let prompt = format_prompt(sim);

        match editor.readline(&prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let _ = editor.add_history_entry(trimmed);

                if trimmed == "quit" || trimmed == "exit" {
                    break;
                }

                // Command parsing deferred to E21CLIHUMCON-004.
                // For now, print an unrecognized-command message.
                println!("Unknown command: {trimmed}. Command parsing is not yet implemented.");
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldwake_core::{
        CauseRef, ControlSource, EntityId, EventLog, Place, Seed, Tick, Topology,
        VisibilitySpec, WitnessData, WorldTxn, hash_world,
    };
    use worldwake_sim::{
        ControllerState, DeterministicRng, RecipeRegistry, ReplayRecordingConfig, ReplayState,
        Scheduler, SimulationState, SystemManifest,
    };

    fn build_sim_with_agent(
        agent_name: &str,
        place_name: &str,
        controlled: bool,
    ) -> SimulationState {
        let mut topology = Topology::new();
        let place_id = EntityId {
            slot: 0,
            generation: 0,
        };
        topology
            .add_place(
                place_id,
                Place {
                    name: place_name.to_string(),
                    capacity: None,
                    tags: Default::default(),
                },
            )
            .unwrap();

        let mut world = worldwake_core::World::new(topology).unwrap();
        let mut event_log = EventLog::new();

        let agent_id = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(0),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::Hidden,
                WitnessData::default(),
            );
            let id = txn.create_agent(agent_name, ControlSource::Human).unwrap();
            txn.set_ground_location(id, place_id).unwrap();
            txn.commit(&mut event_log);
            id
        };

        let controller = if controlled {
            ControllerState::with_entity(agent_id)
        } else {
            ControllerState::new()
        };

        let scheduler = Scheduler::new_with_tick(Tick(5), SystemManifest::canonical());
        let recipe_registry = RecipeRegistry::new();
        let seed = Seed([0u8; 32]);
        let state_hash = hash_world(&world).unwrap();
        let replay = ReplayState::new(
            state_hash,
            seed,
            Tick(5),
            ReplayRecordingConfig::disabled(),
        );
        let rng = DeterministicRng::new(seed);

        SimulationState::new(
            world,
            event_log,
            scheduler,
            recipe_registry,
            replay,
            controller,
            rng,
        )
    }

    #[test]
    fn test_prompt_with_agent() {
        let sim = build_sim_with_agent("Aster", "Market Square", true);
        let prompt = format_prompt(&sim);
        assert!(prompt.contains("tick 5"), "prompt should contain tick: {prompt}");
        assert!(prompt.contains("Aster"), "prompt should contain agent name: {prompt}");
        assert!(
            prompt.contains("Market Square"),
            "prompt should contain place name: {prompt}"
        );
        assert!(prompt.ends_with("> "), "prompt should end with '> ': {prompt}");
    }

    #[test]
    fn test_prompt_observer_mode() {
        let sim = build_sim_with_agent("Aster", "Market Square", false);
        let prompt = format_prompt(&sim);
        assert!(prompt.contains("tick 5"), "prompt should contain tick: {prompt}");
        assert!(
            prompt.contains("observer"),
            "prompt should contain 'observer': {prompt}"
        );
        assert!(!prompt.contains("Aster"), "prompt should not contain agent name in observer mode: {prompt}");
    }
}
