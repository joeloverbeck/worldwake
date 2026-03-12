**Status**: ✅ COMPLETED

# E21CLIHUMCON-006: Tick and Status Commands

## Summary

Implement the `tick [n]` and `status` command handlers. `tick` advances the simulation with AI running each tick. `status` shows the controlled agent's state.

## Depends On

- E21CLIHUMCON-003 (REPL loop and bootstrap)
- E21CLIHUMCON-004 (command enum)
- E21CLIHUMCON-005 (display helpers)

## Files to Touch

- `crates/worldwake-cli/src/handlers/tick.rs` — **create**: `handle_tick()` and `handle_status()` functions
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `Tick` and `Status` variants in `dispatch_command()`

## Out of Scope

- Other command handlers (007–012)
- REPL loop changes (003)
- Scenario loading (001, 002)
- AI logic changes — use `AgentTickDriver` as-is
- Changes to `worldwake-sim::step_tick()` or any other crate

## Deliverables

### `handle_tick(n: u32, sim: &mut SimulationState, driver: &mut AgentTickDriver, registries: &ActionRegistries, dispatch_table: &SystemDispatchTable)`

> **Corrected**: `step_tick()` requires `TickStepServices` which needs `ActionRegistries` and `SystemDispatchTable`. These are not in `SimulationState` — they live in `SpawnedSimulation`. `dispatch_command()` must also be updated to receive and forward these.

For each of the `n` ticks:
1. Wrap `driver` in `AutonomousControllerRuntime` as the `TickInputProducer`
2. Call `step_tick()` with the runtime
3. Print a summary line per tick: `"--- Tick {t} ---"` + count of events generated

Per spec: AI runs each tick for all `ControlSource::Ai` agents via `AgentTickDriver`.

### `handle_status(sim: &SimulationState)`

Requires a controlled agent (error if observer mode). Display:
- Agent name and location
- Current action (if any) with remaining ticks (look up action name via `ActionDefRegistry`)
- Homeostatic needs (all 5) with urgency bands via `format_needs_bar()` using a display-only default `ThresholdBand` (no per-agent bands exist yet; this is derived, not authoritative state).
- Wound count (if any wounds exist)
- Control source (always `[human]` for controlled agent)

### Error Cases
- `tick 0` → print "nothing to do"
- `status` with no controlled agent → print "no controlled agent (observer mode)"

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_tick_advances_simulation`: tick 1 → sim tick increments by 1
  - `test_tick_n_advances_n`: tick 5 → sim tick increments by 5
  - `test_tick_runs_ai`: after tick, AI-controlled agents may have started actions (verify event log not empty or action started)
  - `test_status_shows_needs`: status output includes all 5 need names
  - `test_status_shows_location`: status output includes agent's current place name
  - `test_status_no_controlled_agent`: status in observer mode → error message

### Invariants That Must Remain True
- Invariant 9.1: tick handler never mutates world directly — only through `step_tick()`
- Determinism: same scenario + same tick count → same resulting state
- AI agents run every tick (not skipped)
- `cargo clippy -p worldwake-cli` passes with no warnings

## Outcome

- **Completion date**: 2026-03-12
- **What changed**:
  - Created `crates/worldwake-cli/src/handlers/tick.rs` with `handle_tick()` and `handle_status()`
  - Updated `crates/worldwake-cli/src/handlers/mod.rs` — wired `Tick`/`Status` variants, added `ActionRegistries` + `SystemDispatchTable` params to `dispatch_command()`
  - Updated `crates/worldwake-cli/src/repl.rs` — integrated command parsing via `CommandParser` and dispatch through `dispatch_command()`
  - Added `tick_parts_mut()` to `SimulationState` in `crates/worldwake-sim/src/simulation_state.rs` (public split-borrow accessor)
- **Deviations from original plan**:
  1. Handler signatures expanded to include `ActionRegistries` and `SystemDispatchTable` (not in `SimulationState`)
  2. `handle_status` uses a display-only `default_display_band()` since no per-agent `ThresholdBand` exists yet
  3. Added `tick_parts_mut()` to `SimulationState` — needed to split borrows for `step_tick()` call (Rust borrow checker limitation through method calls)
  4. `repl.rs` updated to integrate command parsing and dispatch (not originally listed in "Files to Touch" but necessary for wiring)
- **Verification**: 46 tests pass (6 new + 40 existing), `cargo clippy --workspace` zero warnings
