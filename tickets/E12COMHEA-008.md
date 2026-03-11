# E12COMHEA-008: Death detection logic + scheduler DeadAt exclusion

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim scheduler + death detection
**Deps**: E12COMHEA-002 (DeadAt component), E12COMHEA-006 (wound helpers)

## Problem

When an agent's wound load reaches `wound_capacity`, the agent dies. Death must:
1. Attach `DeadAt(current_tick)` component to the agent
2. Emit a death event with cause chain
3. The scheduler must exclude agents with `DeadAt` from planning and action starts (invariant 9.14)

## Assumption Reassessment (2026-03-11)

1. Scheduler exists in `crates/worldwake-sim/src/scheduler.rs` — confirmed.
2. Scheduler manages active actions and determines which agents can start new actions.
3. `DeadAt` will be a component on Agent (E12COMHEA-002).
4. Death finality is invariant 9.14: dead agents do not plan, act, trade, vote, or consume.
5. The agent is NOT archived on death — retains all components, remains in the world.

## Architecture Check

1. Death detection is a function called during wound progression (E12COMHEA-009/014) or after combat resolution. It checks `is_wound_load_fatal()` and attaches `DeadAt` if true.
2. Scheduler exclusion is a filter: before starting any new action for an agent, check `DeadAt`. This is a simple guard.
3. Active actions on a dead agent should be terminated (agent can't continue acting after death).
4. Death event should reference the wound(s) that caused death for causal tracing.

## What to Change

### 1. Death detection function

Create a function that checks if an agent should die:
- Reads `WoundList` and `CombatProfile`
- If `is_wound_load_fatal()` and agent lacks `DeadAt`:
  - Attach `DeadAt(current_tick)` via WorldTxn
  - Emit death event with cause chain (which wounds pushed past capacity)

### 2. Scheduler exclusion

Modify scheduler to skip agents with `DeadAt`:
- When iterating agents for planning: skip if `DeadAt` present
- When validating action start: reject if actor has `DeadAt`
- When processing active actions: terminate any actions belonging to dead agents

### 3. Active action termination on death

When `DeadAt` is attached, any in-progress actions for that agent should be immediately terminated/aborted.

## Files to Touch

- `crates/worldwake-sim/src/scheduler.rs` (modify — DeadAt exclusion)
- `crates/worldwake-systems/src/combat.rs` (new or modify — death detection function)
- `crates/worldwake-sim/src/tick_step.rs` (modify — if death check needs to run during tick)

## Out of Scope

- Wound progression logic (E12COMHEA-009)
- Combat action handlers (E12COMHEA-010)
- Corpse looting (E12COMHEA-012)
- Corpse cleanup / archival (explicitly deferred per spec)
- E09 body-cost accrual for dead agents (spec says it continues — no special-case)

## Acceptance Criteria

### Tests That Must Pass

1. Dead agents (with `DeadAt`) generate no new plans or actions
2. Scheduler excludes agents with `DeadAt` from planning
3. Scheduler rejects action starts for agents with `DeadAt`
4. `DeadAt` component is attached when wound load reaches `wound_capacity`
5. Death event is emitted with cause chain referencing the fatal wounds
6. Agent is NOT archived after death — retains all components
7. Dead agent retains inventory and location
8. Active actions on a dying agent are terminated
9. `DeadAt(Tick)` records the correct tick of death
10. Agent that was not dead remains active in scheduler
11. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. 9.14: Death finality — dead agents do not plan, act, trade, vote, or consume
2. Agent is not archived on death — remains in the world with all components
3. Corpse inventory persists (conservation invariant 9.5)
4. Death event traceable in event log (Principle 6)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/scheduler.rs` — DeadAt exclusion tests
2. `crates/worldwake-systems/src/combat.rs` — death detection tests

### Commands

1. `cargo test -p worldwake-sim -- scheduler`
2. `cargo test --workspace && cargo clippy --workspace`
