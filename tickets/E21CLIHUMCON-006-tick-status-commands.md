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

### `handle_tick(n: u32, sim: &mut SimulationState, driver: &mut AgentTickDriver)`

For each of the `n` ticks:
1. Wrap `driver` in `AutonomousControllerRuntime` as the `TickInputProducer`
2. Call `step_tick()` with the runtime
3. Print a summary line per tick: `"--- Tick {t} ---"` + count of events generated

Per spec: AI runs each tick for all `ControlSource::Ai` agents via `AgentTickDriver`.

### `handle_status(sim: &SimulationState)`

Requires a controlled agent (error if observer mode). Display:
- Agent name and location
- Current action (if any) with remaining ticks
- Homeostatic needs (all 5) with urgency bands via `format_needs_bar()`
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
