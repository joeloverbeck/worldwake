# E21CLIHUMCON-003: CLI Args, Bootstrap, and REPL Loop

## Summary

Wire up `main.rs` with clap CLI argument parsing, bootstrap the simulation from a scenario file, create the `AgentTickDriver` for AI, and run the interactive REPL loop via rustyline. The REPL dispatches parsed commands to handler stubs (actual handlers in later tickets).

## Depends On

- E21CLIHUMCON-002 (scenario spawning)

## Files to Touch

- `crates/worldwake-cli/src/main.rs` — **rewrite**: clap `#[derive(Parser)]` for CLI args, bootstrap sequence, REPL entry
- `crates/worldwake-cli/src/repl.rs` — **create**: rustyline REPL loop, prompt formatting, command dispatch skeleton
- `crates/worldwake-cli/src/lib.rs` — **create**: module declarations for `scenario`, `repl`, `commands`, `display`, `handlers`

## Out of Scope

- Command enum definition — that's E21CLIHUMCON-004
- Display helpers — that's E21CLIHUMCON-005
- Individual command handlers (006–012)
- Scenario types and spawning (001, 002 — already done)
- Changes to any crate other than `worldwake-cli`

## Deliverables

### CLI Args (`main.rs`)
```rust
#[derive(Parser)]
#[command(name = "worldwake", about = "Causality-first emergent micro-world simulation")]
struct Cli {
    /// Path to RON scenario file
    scenario: PathBuf,
}
```

### Bootstrap (`main.rs::main()`)
1. Parse CLI args via clap
2. `load_scenario_file(&cli.scenario)` → `ScenarioDef`
3. `spawn_scenario(def)` → `SimulationState`
4. `AgentTickDriver::new(PlanningBudget::default())` — AI controller
5. Call `run_repl(sim, driver)`
6. On error, print human-readable message and exit with code 1

### REPL Loop (`repl.rs`)

`pub fn run_repl(sim: &mut SimulationState, driver: &mut AgentTickDriver) -> Result<(), Box<dyn Error>>`

- Create `rustyline::DefaultEditor` with history
- Loop:
  1. Format prompt: `"[tick {t}] {agent_name} @ {place_name} > "` (or `"[tick {t}] observer > "` if no controlled agent)
  2. `editor.readline(&prompt)`
  3. On `Ok(line)`: add to history, trim, skip empty, parse via `CliCommand::try_parse_from()`
  4. On parse success: `dispatch_command(cmd, sim, driver, &mut last_affordances)`
  5. On parse error: print clap's error message (usage help)
  6. On `Err(ReadlineError::Interrupted | Eof)`: break loop
- Store `last_affordances: Vec<Affordance>` for `do <n>` command (passed mutably to dispatch)

### `ReplState` (optional internal struct)
- Holds `last_affordances`, any other ephemeral UI state
- Not serialized, not part of simulation

### Prompt Formatting
- Needs access to `ControllerState` to get controlled entity
- Needs `World` to look up entity `Name` and placement relation for location name
- If no controlled agent: `"[tick 5] observer > "`

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_cli_args_parse`: clap parses a scenario path correctly
  - `test_prompt_with_agent`: prompt includes tick, agent name, and location
  - `test_prompt_observer_mode`: prompt shows "observer" when no controlled agent
- `cargo build -p worldwake-cli` succeeds (binary compiles)

### Invariants That Must Remain True
- REPL never mutates world state directly — all mutations through `InputQueue` or `step_tick()`
- `quit` and Ctrl-C/Ctrl-D exit cleanly without panic
- Invalid commands show usage help, don't crash
- `cargo clippy -p worldwake-cli` passes with no warnings
