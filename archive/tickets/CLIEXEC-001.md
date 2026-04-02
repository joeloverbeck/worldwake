# CLIEXEC-001: Add single-command execution mode to the CLI

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-cli` (main.rs, repl.rs, commands.rs)
**Deps**: None

## Problem

Claude Code cannot interact with the CLI REPL command-by-command — the Bash tool runs a process to completion before returning output. The only way to use the CLI is piping all commands at once (`echo -e "cmd1\ncmd2" | cargo run`), which means Claude cannot react to output between commands. This makes the `cli-improvement:evaluate` skill unable to authentically explore the CLI — it can't inspect an agent it just discovered via `look`, or choose an action based on what `actions` listed.

A single-command execution mode (`--exec "command" --state session.bin`) would let Claude run one command per Bash call, read the output, decide the next command, and preserve state across invocations via a state file. This unblocks truly interactive evaluation and any future automated CLI tooling.

## Assumption Reassessment (2026-04-02)

1. `main.rs` uses `clap::Parser` with a `Cli` struct containing one arg: `scenario: PathBuf`. Adding `--exec` and `--state` as optional args is straightforward.
2. `run_repl` at `repl.rs:58` takes `(&mut SimulationState, &mut AgentTickDriver, &ActionRegistries, &SystemDispatchTable)`. The single-command mode needs the same parameters but runs one command instead of looping.
3. `dispatch_command` at `handlers/mod.rs` takes a parsed `CliCommand` and returns `CommandResult`. This is the existing single-command dispatch — the exec mode calls it once.
4. `CommandParser::try_parse_from` at `commands.rs:8` parses a command string into `CliCommand`. The exec mode uses the same parser.
5. `handle_save` and `handle_load` in `handlers/persistence.rs` already serialize/deserialize `SimulationState` + `AgentTickDriver`. The `--state` flag reuses this same machinery.
6. `worldwake_sim::save(sim, Some(driver), path)` and `worldwake_sim::load(path)` are the public save/load API. The state file format is already proven (bincode serialization).
7. `ReplState` at `repl.rs:14` stores `last_affordances: Vec<Affordance>`. This transient state is lost between invocations. The `actions` → `do N` pattern needs affordances to be available — the exec mode must re-run `actions` internally (or serialize `ReplState` too, which is heavier). The simplest approach: if `--exec "do N"` is called and `last_affordances` is empty, auto-run `actions` first. Alternative: serialize `ReplState` into the state file.
8. `SpawnedSimulation` at `scenario/mod.rs` contains `state`, `action_registries`, and `dispatch_table`. The registries and dispatch table are rebuilt from action definitions during spawn — they're not serialized. On `--state` load, we still need to spawn a scenario to get the registries, then replace `state` with the loaded one.

## Architecture Check

1. The exec mode is a thin layer over existing infrastructure: same scenario loader, same command parser, same dispatch function, same save/load. No new systems or abstractions.
2. The `--state` file reuses the existing `worldwake_sim::save/load` serialization — no second format, no compatibility shim.
3. The exec mode coexists with the REPL: no `--exec` flag = normal REPL. The REPL is unchanged.

## Verification Layers

1. Single command produces correct output -> focused test (run `--exec "world"`, check stdout contains place names)
2. State persists across invocations -> focused test (run `--exec "tick 1" --state`, then `--exec "status" --state`, verify tick advanced)
3. `do N` works with auto-affordance resolution -> focused test
4. REPL mode unchanged -> existing tests pass
5. Single-layer ticket (CLI-only, no simulation changes).

## What to Change

### 1. Add `--exec` and `--state` args to `Cli` struct in `main.rs`

```rust
#[derive(Parser)]
#[command(name = "worldwake", about = "Causality-first emergent micro-world simulation")]
struct Cli {
    /// Path to RON scenario file
    #[arg(default_value_os_t = default_scenario_path())]
    scenario: PathBuf,

    /// Execute a single command and exit (non-interactive mode)
    #[arg(long)]
    exec: Option<String>,

    /// Path to state file for persisting between --exec invocations
    #[arg(long)]
    state: Option<PathBuf>,
}
```

### 2. Add exec mode branch in `main()`

After scenario loading:

```rust
if let Some(command) = &cli.exec {
    // If --state exists and file is present, load state from it
    // (replacing the freshly-spawned sim state, but keeping registries/dispatch)
    // Parse and dispatch the single command
    // If --state provided, save state after command
    // Exit
} else {
    // Existing REPL path
    run_repl(...)
}
```

### 3. Implement `run_single_command` in `repl.rs`

```rust
pub fn run_single_command(
    sim: &mut SimulationState,
    driver: &mut AgentTickDriver,
    registries: &ActionRegistries,
    dispatch: &SystemDispatchTable,
    command_str: &str,
) -> Result<(), Box<dyn Error>>
```

This function:
1. Creates a fresh `ReplState`
2. If the command is `do N`, first runs `actions` internally to populate `last_affordances`
3. Parses the command via `CommandParser::try_parse_from`
4. Dispatches via `dispatch_command`
5. Returns

### 4. Handle `--state` load/save in `main()`

- **Load**: If `--state <path>` is provided and the file exists, call `worldwake_sim::load(path)` to get the saved `SimulationState` + driver bytes. Replace `sim` and `driver` with the loaded versions. Keep `action_registries` and `dispatch_table` from the spawned scenario (they're not serialized).
- **Save**: After command execution, if `--state` is provided, call `worldwake_sim::save(sim, Some(driver), path)`.
- **First invocation** (no state file yet): Use the freshly-spawned state. Save it after the command.

### 5. Handle `ReplState` persistence for `do N`

The simplest approach: when `--exec "do N"` is executed, auto-run `actions` first to populate `last_affordances`. This is a single extra dispatch call and avoids serializing `ReplState`. The user sees only the `do` output, not the `actions` output.

Alternative (heavier): serialize `ReplState.last_affordances` alongside the simulation state. This preserves the exact affordance list from a prior `--exec "actions"` call. More faithful but more complex.

Recommend the simple approach for now.

## Files to Touch

- `crates/worldwake-cli/src/main.rs` (modify — add `--exec`/`--state` args, exec mode branch)
- `crates/worldwake-cli/src/repl.rs` (modify — add `run_single_command` function)
- `crates/worldwake-cli/src/commands.rs` (no changes expected — parser already works for single commands)

## Out of Scope

- Changes to the REPL loop (it continues working exactly as before)
- Serializing `ReplState` (use auto-affordance resolution instead)
- Changes to simulation, AI, or systems crates
- Batched multi-command execution (one command per invocation is sufficient)

## Acceptance Criteria

### Tests That Must Pass

1. `--exec "world"` with a scenario produces place summary output and exits cleanly
2. `--exec "tick 1" --state /tmp/test.bin` creates the state file
3. `--exec "status" --state /tmp/test.bin` loads state and shows tick 1 (not tick 0)
4. `--exec "actions" --state /tmp/test.bin` lists available actions
5. `--exec "do 1" --state /tmp/test.bin` auto-resolves affordances and executes action 1
6. No `--exec` flag runs the normal REPL (existing behavior unchanged)
7. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The REPL mode is completely unchanged — `--exec` is purely additive.
2. The state file uses the same `worldwake_sim::save/load` format as the `save`/`load` REPL commands.
3. Determinism is preserved — same scenario + same command sequence via `--exec` produces identical state as the REPL.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/main.rs` (new tests) — `--exec` arg parsing, `--state` arg parsing
2. `crates/worldwake-cli/src/repl.rs` (new tests) — `run_single_command` unit tests
3. Integration test (new or in existing test file) — full `--exec` + `--state` round-trip

### Commands

1. `cargo test -p worldwake-cli` — targeted CLI tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- **Completion date**: 2026-04-02
- **What changed**: Added `--exec` and `--state` Clap args to `main.rs`, `run_single_command` function to `repl.rs` with auto-affordance resolution for `do N`, state load/save via existing `worldwake_sim::save/load`.
- **Deviations from original plan**: Auto-affordance for `do N` calls `get_affordances` directly instead of dispatching through `handle_actions` (avoids printing the action list). No separate `ReplState` serialization needed.
- **Verification**: `cargo test --workspace` all pass, `cargo clippy -p worldwake-cli --all-targets -- -D warnings` clean. Manual smoke test: `--exec "world"`, `--exec "tick 3" --state`, `--exec "status" --state` (shows tick 3 needs), `--exec "do 1" --state` (auto-resolves affordances).
