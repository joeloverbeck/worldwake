use worldwake_ai::AgentTickDriver;
use worldwake_sim::SimulationState;

use crate::commands::{CliCommand, CommandOutcome, CommandResult};
use crate::repl::ReplState;

/// Dispatch a parsed CLI command to its handler.
///
/// Currently all handlers are stubs that print "not implemented".
/// Individual handlers will be filled in by tickets 006–012.
#[allow(clippy::needless_pass_by_value)]
pub fn dispatch_command(
    cmd: CliCommand,
    _sim: &mut SimulationState,
    _driver: &mut AgentTickDriver,
    _repl_state: &mut ReplState,
) -> CommandResult {
    match cmd {
        CliCommand::Tick { .. } => {
            println!("tick: not implemented");
        }
        CliCommand::Status => {
            println!("status: not implemented");
        }
        CliCommand::Look => {
            println!("look: not implemented");
        }
        CliCommand::Actions => {
            println!("actions: not implemented");
        }
        CliCommand::Do { .. } => {
            println!("do: not implemented");
        }
        CliCommand::Cancel => {
            println!("cancel: not implemented");
        }
        CliCommand::Inventory { .. } => {
            println!("inventory: not implemented");
        }
        CliCommand::Needs { .. } => {
            println!("needs: not implemented");
        }
        CliCommand::Inspect { .. } => {
            println!("inspect: not implemented");
        }
        CliCommand::Relations { .. } => {
            println!("relations: not implemented");
        }
        CliCommand::Events { .. } => {
            println!("events: not implemented");
        }
        CliCommand::Event { .. } => {
            println!("event: not implemented");
        }
        CliCommand::Trace { .. } => {
            println!("trace: not implemented");
        }
        CliCommand::Switch { .. } => {
            println!("switch: not implemented");
        }
        CliCommand::Observe => {
            println!("observe: not implemented");
        }
        CliCommand::World => {
            println!("world: not implemented");
        }
        CliCommand::Places => {
            println!("places: not implemented");
        }
        CliCommand::Agents => {
            println!("agents: not implemented");
        }
        CliCommand::Goods => {
            println!("goods: not implemented");
        }
        CliCommand::Save { .. } => {
            println!("save: not implemented");
        }
        CliCommand::Load { .. } => {
            println!("load: not implemented");
        }
        CliCommand::Quit => {
            return Ok(CommandOutcome::Quit);
        }
    }
    Ok(CommandOutcome::Continue)
}
