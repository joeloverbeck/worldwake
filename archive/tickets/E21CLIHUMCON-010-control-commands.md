**Status**: ✅ COMPLETED

# E21CLIHUMCON-010: Control Commands (switch, observe)

## Summary

Implement agent switching and observer mode: `switch <name>` transfers human control to another agent, `observe` releases control entirely.

## Depends On

- E21CLIHUMCON-003 (REPL loop)
- E21CLIHUMCON-004 (command enum)
- E21CLIHUMCON-005 (display helpers — entity resolution)

## Files to Touch

- `crates/worldwake-cli/src/handlers/control.rs` — **create**: `handle_switch()`, `handle_observe()`
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `Switch`, `Observe` variants

## Out of Scope

- Other command handlers (006–009, 011–012)
- Modifying `ControllerState`, `AgentData`, or `ControlSource` types
- Agent death handling during switch (that's an edge case for integration tests in 013)
- Changes to any crate other than `worldwake-cli`

## Deliverables

### `handle_switch(sim: &mut SimulationState, name: &str)`

Per spec lines 104–109:
1. Resolve `name` → `EntityId` via `resolve_entity()`
2. Validate target:
   - Must be an agent (has `AgentData` component) → error if not
   - Must be alive (entity is live in allocator) → error if not
   - Must not already be the controlled agent → print "already controlling {name}"
3. If currently controlling an agent (old agent):
   - Set old agent's `AgentData.control_source` to `ControlSource::Ai` via `WorldTxn`
4. Set new agent's `AgentData.control_source` to `ControlSource::Human` via `WorldTxn`
5. Update `ControllerState` to track new agent
6. Print confirmation: `"Now controlling {name} at {place}"`

### `handle_observe(sim: &mut SimulationState)`

Per spec lines 110–111:
1. If currently controlling an agent:
   - Set agent's `AgentData.control_source` to `ControlSource::Ai` via `WorldTxn`
2. Clear controlled entity in `ControllerState`
3. Print: `"Observer mode — simulation runs without human control"`

### WorldTxn Usage
- Both commands modify `AgentData.control_source` — this must go through `WorldTxn` (staged mutations, atomic commit)
- The control source change takes effect immediately (no tick needed)
- Create appropriate `EventRecord` for the control transfer if the event system requires it, or skip if control changes are not event-logged

### Error Cases
- `switch` to non-agent entity → "entity is not an agent"
- `switch` to dead agent → "agent is not alive"
- `switch` to self → "already controlling {name}"
- `switch` to nonexistent name → entity resolution error with suggestions
- `observe` when already in observer mode → "already in observer mode"

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli` — T24 and related:
  - `test_switch_transfers_control` (T24): switch to agent B → B is Human, A is Ai
  - `test_switch_preserves_world_state`: world state unchanged after switch (same tick, same entities)
  - `test_switch_to_non_agent`: switch to item → error
  - `test_switch_to_self`: switch to already-controlled agent → "already controlling" message
  - `test_observe_releases_control`: observe → no controlled agent, old agent becomes Ai
  - `test_observe_already_observer`: observe when already observer → message
  - `test_switch_from_observer`: switch from observer mode → new agent becomes Human
  - `test_switch_new_agent_affordances` (T12): after switch to merchant, affordances reflect merchant's context

### Invariants That Must Remain True
- Invariant 9.12: no special player actions — switch just changes `ControlSource`, affordances are determined by agent's context
- At most one agent has `ControlSource::Human` at any time
- World simulation state is preserved across switches (no reset)
- `cargo clippy -p worldwake-cli` passes with no warnings

## Outcome

- **Completion date**: 2026-03-13
- **What changed**:
  - Created `crates/worldwake-cli/src/handlers/control.rs` with `handle_switch()`, `handle_observe()`, and `set_control_source()` helper
  - Modified `crates/worldwake-cli/src/handlers/mod.rs` to wire `Switch`/`Observe` variants
  - Added `world_and_event_log_mut()` split-borrow method to `SimulationState` in `crates/worldwake-sim/src/simulation_state.rs` (needed to construct `WorldTxn` and commit from a single `SimulationState`)
- **Deviations from original plan**:
  - Ticket scoped changes to `worldwake-cli` only, but `SimulationState::world_and_event_log_mut()` was added in `worldwake-sim` to solve the split-borrow problem required by `WorldTxn` usage. This is a minimal, general-purpose accessor that benefits other callers too.
  - Control source mutations use `CauseRef::ExternalInput(0)` and `VisibilitySpec::Hidden` to mark them as meta-operations rather than simulation events.
- **Verification results**:
  - 8/8 tests pass (`cargo test -p worldwake-cli --lib handlers::control`)
  - `cargo clippy -p worldwake-cli` clean (0 warnings)
  - `worldwake-sim` tests unaffected by split-borrow addition
