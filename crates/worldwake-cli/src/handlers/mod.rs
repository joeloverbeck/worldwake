pub(crate) mod actions;
mod control;
mod events;
mod inspect;
mod persistence;
pub(crate) mod tick;
mod world_overview;

use worldwake_ai::AgentTickDriver;
use worldwake_sim::{SimulationState, SystemDispatchTable};
use worldwake_systems::ActionRegistries;

use crate::commands::{CliCommand, CommandOutcome, CommandResult};
use crate::repl::ReplState;

/// Dispatch a parsed CLI command to its handler.
///
/// Tick and Status are implemented in `tick.rs`.
/// Look, Inspect, Inventory, Needs, Relations are implemented in `inspect.rs`.
/// Actions, Do, Cancel are implemented in `actions.rs`.
/// Other handlers are stubs that will be filled in by tickets 009–012.
#[allow(clippy::needless_pass_by_value)]
pub fn dispatch_command(
    cmd: CliCommand,
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    registries: &ActionRegistries,
    dispatch_table: &SystemDispatchTable,
    repl_state: &mut ReplState,
) -> CommandResult {
    match cmd {
        CliCommand::Tick { n } => {
            tick::handle_tick(n.unwrap_or(1), sim, driver, registries, dispatch_table)
        }
        CliCommand::Status => tick::handle_status(sim, registries),
        CliCommand::Look => inspect::handle_look(sim),
        CliCommand::Inspect { entity } => inspect::handle_inspect(sim, &entity.join(" ")),
        CliCommand::Inventory { entity } => {
            let joined = entity.join(" ");
            let arg = if joined.is_empty() {
                None
            } else {
                Some(joined.as_str())
            };
            inspect::handle_inventory(sim, arg)
        }
        CliCommand::Needs { entity } => {
            let joined = entity.join(" ");
            let arg = if joined.is_empty() {
                None
            } else {
                Some(joined.as_str())
            };
            inspect::handle_needs(sim, arg)
        }
        CliCommand::Relations { entity } => inspect::handle_relations(sim, &entity.join(" ")),
        CliCommand::Actions => actions::handle_actions(sim, registries, repl_state),
        CliCommand::Do { n } => actions::handle_do(n, sim, registries, repl_state),
        CliCommand::Cancel => actions::handle_cancel(sim),
        CliCommand::Events { n } => events::handle_events(sim, n, registries),
        CliCommand::Event { id } => events::handle_event(sim, id, registries),
        CliCommand::Trace { id } => events::handle_trace(sim, id, registries),
        CliCommand::Switch { name } => control::handle_switch(sim, &name.join(" ")),
        CliCommand::Observe => control::handle_observe(sim),
        CliCommand::World => world_overview::handle_world(sim),
        CliCommand::Places => world_overview::handle_places(sim),
        CliCommand::Agents => world_overview::handle_agents(sim, registries),
        CliCommand::Goods => world_overview::handle_goods(sim),
        CliCommand::Save { path } => persistence::handle_save(sim, driver, &path),
        CliCommand::Load { path } => persistence::handle_load(sim, driver, repl_state, &path),
        CliCommand::Quit => Ok(CommandOutcome::Quit),
    }
}
