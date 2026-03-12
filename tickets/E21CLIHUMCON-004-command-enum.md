# E21CLIHUMCON-004: Command Enum (Clap Subcommands)

## Summary

Define the `CliCommand` enum with clap derive subcommands for all CLI commands. Pure type definition — no handler logic.

## Depends On

None.

## Files to Touch

- `crates/worldwake-cli/src/commands.rs` — **create**: `CliCommand` enum with all subcommands
- `crates/worldwake-cli/src/handlers/mod.rs` — **create**: `dispatch_command()` function signature (body delegates to stubs or prints "not implemented")

## Out of Scope

- REPL loop (E21CLIHUMCON-003)
- Individual handler implementations (006–012)
- Display helpers (E21CLIHUMCON-005)
- Changes to any crate other than `worldwake-cli`

## Deliverables

### `CliCommand` enum
```rust
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
```

### `dispatch_command()` signature
```rust
pub fn dispatch_command(
    cmd: CliCommand,
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    repl_state: &mut ReplState,
) -> CommandResult
```

- `CommandResult` = `Result<CommandOutcome, CommandError>`
- `CommandOutcome` enum: `Continue`, `Quit`
- `CommandError`: wraps display-friendly error messages
- Initial body: match on all variants, print "not implemented" for each handler (stubs filled in 006–012)

### Clap Wrapper Struct
```rust
#[derive(Parser, Debug)]
#[command(multicall = true, disable_help_subcommand = false)]
struct CommandParser {
    #[command(subcommand)]
    command: CliCommand,
}
```
- `multicall = true` so the parser works with REPL input (no binary name prefix)

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_parse_tick_default`: `"tick"` → `Tick { n: None }`
  - `test_parse_tick_n`: `"tick 5"` → `Tick { n: Some(5) }`
  - `test_parse_do`: `"do 3"` → `Do { n: 3 }`
  - `test_parse_switch`: `"switch Kael"` → `Switch { name: "Kael".into() }`
  - `test_parse_events_default`: `"events"` → `Events { n: None }`
  - `test_parse_inventory_no_arg`: `"inventory"` → `Inventory { entity: None }`
  - `test_parse_inventory_with_arg`: `"inventory Kael"` → `Inventory { entity: Some("Kael".into()) }`
  - `test_parse_quit`: `"quit"` → `Quit`
  - `test_parse_invalid`: `"foobar"` → parse error

### Invariants That Must Remain True
- Every command in the spec's command table (lines 64–88) has a corresponding variant
- No handler logic in this file — only type definitions and dispatch routing
- `cargo clippy -p worldwake-cli` passes with no warnings
