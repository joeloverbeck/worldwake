# E12COMHEA-014: Combat system tick function + SystemDispatch wiring

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems combat module + system dispatch
**Deps**: E12COMHEA-008 (death detection), E12COMHEA-009 (wound progression), E12COMHEA-010 (Attack handler), E12COMHEA-011 (Defend handler)

## Problem

The combat system tick function needs to be wired into the `SystemDispatch` table, replacing the current `noop_system` placeholder for `SystemId::Combat`. This function orchestrates wound progression, death detection, and active combat action processing each tick.

## Assumption Reassessment (2026-03-11)

1. `SystemId::Combat` already exists as position 3 in `SystemManifest` — confirmed.
2. `dispatch_table()` in `crates/worldwake-systems/src/lib.rs` currently maps Combat to `noop_system` — confirmed.
3. `SystemExecutionContext` provides `world`, `event_log`, `rng`, `active_actions`, `action_defs`, `tick`, `system_id` — confirmed.
4. Pattern: system function signature is `fn combat_system(SystemExecutionContext) -> Result<(), SystemError>`.
5. System execution order: Needs(0), Production(1), Trade(2), Combat(3), Perception(4), Politics(5).

## Architecture Check

1. Combat system tick runs after Needs, Production, and Trade — correct per system manifest order.
2. Each tick, the combat system:
   a. Progresses all wounds (bleeding, clotting, recovery) for all living agents
   b. Checks for deaths (wound load >= capacity) and attaches `DeadAt`
   c. Active combat actions are processed by their handlers through the action framework — the system tick doesn't re-process them here
3. The system does NOT directly handle individual attack/defend/heal/loot actions — those are handled by the action handler framework during the action phase of the tick. The combat system tick handles wound progression and death detection only.

## What to Change

### 1. Implement `combat_system()` function

```rust
pub fn combat_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    // 1. For each living agent with wounds:
    //    a. Progress wound bleeding (increase severity from bleed_rate)
    //    b. Apply natural clotting (reduce bleed_rate)
    //    c. Apply natural recovery (reduce severity if conditions met)
    // 2. For each living agent, check death:
    //    a. If wound_load >= wound_capacity, attach DeadAt(tick)
    //    b. Emit death event
    //    c. Terminate active actions for dead agent
    Ok(())
}
```

### 2. Wire into SystemDispatch

Replace `noop_system` at position 3 (Combat) with `combat_system`.

### 3. Ensure proper transaction handling

Use `WorldTxn` for all mutations (wound updates, DeadAt attachment), commit with event log.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — implement combat_system)
- `crates/worldwake-systems/src/lib.rs` (modify — wire combat_system into dispatch_table)

## Out of Scope

- Action handler implementations (E12COMHEA-010/011/012/013 — already complete by this point)
- Affordance queries for combat actions (deferred to E13)
- AI combat decision making (E13)
- Perception system updates (E14)
- Integration tests across multiple ticks (E12COMHEA-015)

## Acceptance Criteria

### Tests That Must Pass

1. Combat system runs without error on agents with no wounds
2. Combat system progresses bleeding wounds correctly
3. Combat system detects death and attaches `DeadAt`
4. Combat system emits death events
5. Combat system does not crash on agents without `CombatProfile` (graceful skip)
6. Combat system respects Principle 12: reads from shared state, no direct system calls
7. `noop_system` is no longer used for Combat in dispatch table
8. System execution order unchanged (Combat at position 3)
9. `BodyCostPerTick` (E09) still accrues for dead agents (no special-case in combat system)
10. Existing suite: `cargo test --workspace`

### Invariants

1. Principle 12: system decoupling — combat system depends only on core + sim, not other system modules
2. Principle 6: deterministic execution
3. All mutations go through WorldTxn
4. Events emitted for all state changes (wounds, deaths)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — system tick integration tests
2. `crates/worldwake-systems/src/lib.rs` — verify dispatch table wiring

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`
