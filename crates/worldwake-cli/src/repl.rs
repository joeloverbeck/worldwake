use clap::Parser;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::error::Error;
use worldwake_ai::AgentTickDriver;
use worldwake_core::Name;
use worldwake_sim::{
    Affordance, PerAgentBeliefRuntime, PerAgentBeliefView, SimulationState, SystemDispatchTable,
    get_affordances,
};
use worldwake_systems::ActionRegistries;

use crate::commands::{CommandOutcome, CommandParser};
use crate::handlers::dispatch_command;

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

/// Run a single command in non-interactive mode (`--exec`).
///
/// Parses the command string, dispatches it, and returns. If the command is
/// `do N` and no prior `actions` call populated affordances, automatically
/// runs `actions` first to populate the affordance list.
pub fn run_single_command(
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    registries: &ActionRegistries,
    dispatch_table: &SystemDispatchTable,
    command_str: &str,
) -> Result<(), Box<dyn Error>> {
    let mut repl_state = ReplState::new();

    let trimmed = command_str.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();

    // Auto-populate affordances if command is `do N` (affordances are empty
    // in a fresh ReplState). Call get_affordances directly to avoid printing
    // the action list — the user only wants the `do` output.
    // Apply the same filters as handle_actions() to keep indices aligned.
    if words.first().is_some_and(|w| w.eq_ignore_ascii_case("do"))
        && let Some(entity) = sim.controller_state().controlled_entity()
    {
        let runtime =
            PerAgentBeliefRuntime::new(sim.scheduler().active_actions(), &registries.defs);
        let view = PerAgentBeliefView::with_runtime_from_world(entity, sim.world(), runtime);
        let mut affordances =
            get_affordances(&view, entity, &registries.defs, &registries.handlers);
        affordances.retain(|a| !a.bound_targets.contains(&entity));
        affordances.retain(|a| {
            registries.defs.get(a.def_id).is_none_or(|def| {
                !crate::handlers::actions::HIDDEN_ACTIONS.contains(&def.name.as_str())
            })
        });
        affordances.dedup_by(|a, b| a.def_id == b.def_id && a.bound_targets == b.bound_targets);
        repl_state.last_affordances = affordances;
    }

    let parsed = match CommandParser::try_parse_from(words) {
        Ok(p) => p,
        Err(e) => {
            // Clap returns DisplayHelp / DisplayVersion as "errors". Print
            // them to stdout and treat as success rather than propagating.
            println!("{e}");
            return Ok(());
        }
    };

    match dispatch_command(
        parsed.command,
        sim,
        driver,
        registries,
        dispatch_table,
        &mut repl_state,
    ) {
        Ok(CommandOutcome::Continue | CommandOutcome::Quit) => Ok(()),
        Err(e) => {
            println!("error: {e}");
            Ok(())
        }
    }
}

/// Run the interactive REPL loop.
///
/// Reads commands from stdin, dispatches them, and loops until quit/EOF/interrupt.
pub fn run_repl(
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    registries: &ActionRegistries,
    dispatch_table: &SystemDispatchTable,
) -> Result<(), Box<dyn Error>> {
    let mut editor = DefaultEditor::new()?;
    let mut repl_state = ReplState::new();

    loop {
        let prompt = format_prompt(sim);

        match editor.readline(&prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let _ = editor.add_history_entry(trimmed);

                let words: Vec<&str> = trimmed.split_whitespace().collect();
                let parsed = CommandParser::try_parse_from(words);

                match parsed {
                    Ok(parser) => {
                        match dispatch_command(
                            parser.command,
                            sim,
                            driver,
                            registries,
                            dispatch_table,
                            &mut repl_state,
                        ) {
                            Ok(CommandOutcome::Quit) => break,
                            Ok(CommandOutcome::Continue) => {}
                            Err(e) => println!("error: {e}"),
                        }
                    }
                    Err(e) => {
                        // Clap parse errors include help text and unknown command messages.
                        println!("{e}");
                    }
                }
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
    use std::collections::BTreeSet;
    use worldwake_core::{
        CauseRef, ControlSource, EntityId, EventLog, Place, Seed, Tick, Topology, VisibilitySpec,
        WitnessData, WorldTxn, hash_world,
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
                    tags: BTreeSet::default(),
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

        let controller_state = if controlled {
            ControllerState::with_entity(agent_id)
        } else {
            ControllerState::new()
        };

        let scheduler = Scheduler::new_with_tick(Tick(5), SystemManifest::canonical());
        let recipe_registry = RecipeRegistry::new();
        let seed = Seed([0u8; 32]);
        let state_hash = hash_world(&world).unwrap();
        let replay = ReplayState::new(state_hash, seed, Tick(5), ReplayRecordingConfig::disabled());
        let rng = DeterministicRng::new(seed);

        SimulationState::new(
            world,
            event_log,
            scheduler,
            recipe_registry,
            replay,
            controller_state,
            rng,
        )
    }

    #[test]
    fn test_prompt_with_agent() {
        let sim = build_sim_with_agent("Aster", "Market Square", true);
        let prompt = format_prompt(&sim);
        assert!(
            prompt.contains("tick 5"),
            "prompt should contain tick: {prompt}"
        );
        assert!(
            prompt.contains("Aster"),
            "prompt should contain agent name: {prompt}"
        );
        assert!(
            prompt.contains("Market Square"),
            "prompt should contain place name: {prompt}"
        );
        assert!(
            prompt.ends_with("> "),
            "prompt should end with '> ': {prompt}"
        );
    }

    #[test]
    fn test_prompt_observer_mode() {
        let sim = build_sim_with_agent("Aster", "Market Square", false);
        let prompt = format_prompt(&sim);
        assert!(
            prompt.contains("tick 5"),
            "prompt should contain tick: {prompt}"
        );
        assert!(
            prompt.contains("observer"),
            "prompt should contain 'observer': {prompt}"
        );
        assert!(
            !prompt.contains("Aster"),
            "prompt should not contain agent name in observer mode: {prompt}"
        );
    }
}
