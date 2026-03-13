pub(crate) mod actions;
mod inspect;
pub(crate) mod tick;

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
        CliCommand::Inspect { entity } => inspect::handle_inspect(sim, &entity),
        CliCommand::Inventory { entity } => {
            inspect::handle_inventory(sim, entity.as_deref())
        }
        CliCommand::Needs { entity } => {
            inspect::handle_needs(sim, entity.as_deref())
        }
        CliCommand::Relations { entity } => inspect::handle_relations(sim, &entity),
        CliCommand::Actions => actions::handle_actions(sim, registries, repl_state),
        CliCommand::Do { n } => actions::handle_do(n, sim, registries, repl_state),
        CliCommand::Cancel => actions::handle_cancel(sim),
        CliCommand::Events { .. } => {
            println!("events: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Event { .. } => {
            println!("event: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Trace { .. } => {
            println!("trace: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Switch { .. } => {
            println!("switch: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Observe => {
            println!("observe: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::World => {
            println!("world: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Places => {
            println!("places: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Agents => {
            println!("agents: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Goods => {
            println!("goods: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Save { .. } => {
            println!("save: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Load { .. } => {
            println!("load: not implemented");
            Ok(CommandOutcome::Continue)
        }
        CliCommand::Quit => Ok(CommandOutcome::Quit),
    }
}
