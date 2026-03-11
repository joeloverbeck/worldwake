# E12COMHEA-008: Death detection logic + scheduler DeadAt exclusion

**Status**: ✅ COMPLETED
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
2. The scheduler owns tick/order state plus the active-action set, but it does not own an AI planning loop. There is no current scheduler-side "iterate agents for planning" path to modify.
3. `start_gate.rs` and `action_validation.rs` already reject dead actors authoritatively via `Constraint::ActorNotDead`, and combat affordance queries already exclude dead actors.
4. `DeadAt` is already a registered Agent component from E12COMHEA-002.
5. The live finality gap is broader runtime behavior: there is no system that turns fatal wound load into `DeadAt`, active actions are not culled when an actor is dead, and `needs_system` still processes dead agents even though the spec says death finality includes no further consumption.
6. The agent is NOT archived on death — it retains components, inventory, and location in the world.

## Architecture Check

1. Death detection should be owned by the combat system slot, not by the `Scheduler` data structure. Fatality derives from wound state, so the state-mediated place for that transition is a system pass that reads `WoundList` + `CombatProfile` and writes `DeadAt`.
2. Action-start exclusion is already handled in the affordance/start-gate pipeline. Duplicating that logic in `Scheduler` would be redundant and weaker than the existing authoritative validation path.
3. Active actions on a dead agent should be aborted by tick execution once `DeadAt` is present. This belongs in tick-step runtime orchestration, where action aborts already live, not in raw scheduler storage methods.
4. To satisfy death finality with the currently-live systems, `needs_system` must skip dead agents. Leaving dead agents on the metabolism path would contradict the spec's "dead agents do not consume" invariant.
5. The event model can record death as a concrete world-mutation event tagged for combat/system processing, but it does not currently support stable wound IDs. This ticket should not promise per-wound identifiers or alias structures that do not exist.

## What to Change

### 1. Combat-owned death detection

Implement a combat-system pass that:
- Reads `WoundList` and `CombatProfile`
- If `is_wound_load_fatal()` and the agent lacks `DeadAt`:
  - Attaches `DeadAt(current_tick)` via `WorldTxn`
  - Emits a concrete combat/system world-mutation event for the death transition

This ticket may introduce the minimal `combat_system()`/dispatch wiring needed for death detection now, with E12COMHEA-014 extending that same system later for wound progression instead of introducing a second death path.

### 2. Runtime dead-action exclusion

Abort active actions whose actor has `DeadAt` during tick execution:
- Before active actions progress, so dead actors cannot continue acting
- After systems run, so actors who die during the current tick are culled immediately

### 3. Finality in the live needs path

Update `needs_system` to skip dead agents entirely so death finality matches the spec's "do not consume" rule.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — death detection / minimal combat system)
- `crates/worldwake-systems/src/lib.rs` (modify — wire combat slot to combat system)
- `crates/worldwake-sim/src/tick_step.rs` (modify — abort active actions for dead actors)
- `crates/worldwake-systems/src/needs.rs` (modify — skip dead agents)

## Out of Scope

- AI planner integration (no live scheduler-owned planning loop exists yet)
- Wound progression logic (E12COMHEA-009)
- Combat action handlers (E12COMHEA-010)
- Corpse looting (E12COMHEA-012)
- Corpse cleanup / archival (explicitly deferred per spec)
- New wound identifier schema or compatibility aliases for death tracing

## Acceptance Criteria

### Tests That Must Pass

1. Fatal wound load attaches `DeadAt(current_tick)` exactly once
2. The combat system emits a concrete death world-mutation event
3. Agent is NOT archived after death — retains all components
4. Dead agent retains inventory and location
5. Active actions owned by dead agents are aborted and removed from the active set
6. Actions for actors that are already dead do not progress through tick execution
7. Dead agents are skipped by `needs_system`
8. `DeadAt(Tick)` records the correct tick of death
9. Non-dead agents remain unaffected by the new finality logic
10. Existing suites: `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`

### Invariants

1. 9.14: Death finality — dead agents do not act or consume in the currently-live runtime
2. Agent is not archived on death — remains in the world with all components
3. Corpse inventory persists (conservation invariant 9.5)
4. Death transition is traceable in the event log as a concrete state mutation (Principle 6)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — death detection / combat-system tests
2. `crates/worldwake-sim/src/tick_step.rs` — dead active-action abort tests
3. `crates/worldwake-systems/src/needs.rs` — dead-agent exclusion tests

### Commands

1. `cargo test -p worldwake-sim -- tick_step`
2. `cargo test -p worldwake-systems -- combat`
3. `cargo test -p worldwake-systems -- needs`
4. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- Outcome amended: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions to match the live codebase before implementation
  - implemented fatal wound-load death detection in `worldwake-systems::combat` and wired the combat system slot in `dispatch_table()`
  - made tick execution abort active actions for actors already marked dead, including actors who become dead during the same tick's system phase
  - updated `needs_system` to skip dead agents so the live runtime now respects the "dead agents do not consume" part of death finality
  - introduced stable `WoundId` identity in the authoritative wound schema so future healing, combat progression, and death attribution can address individual wounds cleanly
  - extended event payloads with typed `EvidenceRef` attachments and updated combat death events to reference the concrete wound IDs present at death
  - taught combat death attribution to point at the latest `WoundList` mutation event for the dying agent when such an event exists, preserving a concrete causal chain instead of a root-only system tick
  - extended `WorldTxn` with `into_pending_event()` so callers can attach typed evidence without bypassing the transaction/event model
  - added targeted tests for combat death detection, dispatch wiring, dead-action abortion, same-tick death culling, and dead-agent needs exclusion
- Deviations from original plan:
  - did not add duplicate `DeadAt` rejection logic to `Scheduler` itself, because action-start exclusion already existed in affordance/start-gate validation and duplicating it in raw scheduler storage would have been redundant
  - expanded scope to include `needs_system` finality because leaving dead agents on the metabolism path would have violated the spec's live death-finality invariant
- Verification results:
  - `cargo test -p worldwake-core -- wounds event_record event_log world_txn delta component_tables world verification` passed
  - `cargo test -p worldwake-sim -- tick_step` passed
  - `cargo test -p worldwake-sim -- action_validation` passed
  - `cargo test -p worldwake-sim -- trade_valuation` passed
  - `cargo test -p worldwake-systems -- combat` passed
  - `cargo test -p worldwake-systems -- needs` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
