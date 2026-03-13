use clap::{Parser, Subcommand};
use std::fmt;

/// Clap wrapper for REPL command parsing.
///
/// `multicall = true` so the parser works with REPL input (no binary name prefix).
#[derive(Parser, Debug)]
#[command(multicall = true, disable_help_subcommand = false)]
pub struct CommandParser {
    #[command(subcommand)]
    pub command: CliCommand,
}

/// All CLI commands available in the REPL.
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Advance simulation n ticks (default 1)
    Tick { n: Option<u32> },
    /// Show controlled agent status
    Status,
    /// Describe current location
    Look,
    /// List available actions
    Actions,
    /// Execute action by menu number
    Do { n: usize },
    /// Cancel current action
    Cancel,
    /// Show inventory
    Inventory { entity: Option<String> },
    /// Show homeostatic needs
    Needs { entity: Option<String> },
    /// Show all components on entity
    Inspect { entity: String },
    /// Show relations for entity
    Relations { entity: String },
    /// Show recent events
    Events { n: Option<usize> },
    /// Show event details
    Event { id: u64 },
    /// Trace causal chain
    Trace { id: u64 },
    /// Switch control to agent
    Switch { name: String },
    /// Enter observer mode
    Observe,
    /// World summary
    World,
    /// List places
    Places,
    /// List agents
    Agents,
    /// Global goods summary
    Goods,
    /// Save state to file
    Save { path: String },
    /// Load state from file
    Load { path: String },
    /// Exit
    Quit,
}

/// Outcome of a successfully dispatched command.
#[derive(Debug, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Continue the REPL loop.
    Continue,
    /// Exit the REPL loop.
    Quit,
}

/// Display-friendly error from command dispatch.
#[derive(Debug)]
pub struct CommandError {
    pub message: String,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CommandError {}

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Result type for command dispatch.
pub type CommandResult = Result<CommandOutcome, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(input: &str) -> Result<CliCommand, clap::Error> {
        let words: Vec<&str> = input.split_whitespace().collect();
        CommandParser::try_parse_from(words).map(|p| p.command)
    }

    #[test]
    fn test_parse_tick_default() {
        let cmd = parse("tick").unwrap();
        assert!(matches!(cmd, CliCommand::Tick { n: None }));
    }

    #[test]
    fn test_parse_tick_n() {
        let cmd = parse("tick 5").unwrap();
        assert!(matches!(cmd, CliCommand::Tick { n: Some(5) }));
    }

    #[test]
    fn test_parse_do() {
        let cmd = parse("do 3").unwrap();
        assert!(matches!(cmd, CliCommand::Do { n: 3 }));
    }

    #[test]
    fn test_parse_switch() {
        let cmd = parse("switch Kael").unwrap();
        match cmd {
            CliCommand::Switch { name } => assert_eq!(name, "Kael"),
            other => panic!("expected Switch, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_events_default() {
        let cmd = parse("events").unwrap();
        assert!(matches!(cmd, CliCommand::Events { n: None }));
    }

    #[test]
    fn test_parse_inventory_no_arg() {
        let cmd = parse("inventory").unwrap();
        assert!(matches!(cmd, CliCommand::Inventory { entity: None }));
    }

    #[test]
    fn test_parse_inventory_with_arg() {
        let cmd = parse("inventory Kael").unwrap();
        match cmd {
            CliCommand::Inventory { entity } => assert_eq!(entity, Some("Kael".to_string())),
            other => panic!("expected Inventory, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_quit() {
        let cmd = parse("quit").unwrap();
        assert!(matches!(cmd, CliCommand::Quit));
    }

    #[test]
    fn test_parse_invalid() {
        let result = parse("foobar");
        assert!(result.is_err());
    }
}
