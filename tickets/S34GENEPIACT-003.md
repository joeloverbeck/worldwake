# S34GENEPIACT-003: verify_belief action handler — definition, registration, start/tick/commit/abort

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems: new epistemic_actions.rs module with verify_belief handler
**Deps**: S34GENEPIACT-001 (core types), S34GENEPIACT-002 (payload types)

## Problem

Agents cannot deliberately verify beliefs. The `verify_belief` action does not exist in the action framework. Without it, the planner cannot sequence epistemic actions and canonical Scenario D (rumor -> travel -> empty source -> discovery -> belief correction -> replan) cannot emerge through deliberate verification.

## Assumption Reassessment (2026-03-28)

1. `register_investigate_action()` in `crates/worldwake-systems/src/investigate_actions.rs:13-30` is the closest structural precedent. It registers an `ActionDef` with domain, duration expression, interruptibility, and visibility, then registers an `ActionHandler` with start/tick/commit/abort + payload validators.
2. `register_all_actions()` in `crates/worldwake-systems/src/action_registry.rs:20-44` is where all action registrations are called. The new `register_verify_belief_action()` must be added here.
3. `validate_investigate_payload_authoritatively()` at `investigate_actions.rs:295-335` validates world state (actor alive, not incapacitated, at target place). The `verify_belief` authoritative validator follows the same pattern but checks the actor is at the place being verified.
4. `commit_investigate()` at `investigate_actions.rs:135-218` reads authoritative world state to determine what the agent observes and writes to the agent's belief store via `WorldTxn`. The `verify_belief` commit handler follows the same pattern but dispatches on `VerificationSubject` variant.
5. The spec says `verify_belief` writes `ViolationKind::EntityMissing` and `ViolationKind::SupplyDepleted` to `ViolationMemory` when expectations are mismatched. These violation kinds already exist in `crates/worldwake-core/src/violation.rs:24-43`.
6. This is a handler-layer ticket. AI/belief-view logic is out of scope. The handler reads authoritative world state and writes to belief store and violation memory.
7. `ActionDuration` resolution uses `VerificationDispositionProfile::verify_belief_duration_ticks` — the handler must read this from the actor's component.
8. `enumerate_verify_belief_payloads` affordance enumerator is needed for the affordance system to generate available verify_belief payloads for the actor's current beliefs.

## Architecture Check

1. Creating a new `epistemic_actions.rs` module (parallel to `investigate_actions.rs`) keeps epistemic action handlers cleanly separated from investigation/justice handlers. Both `verify_belief` and `ask_witness` (ticket 004) will live here.
2. No backward-compatibility shims. New handler only.

## Verification Layers

1. verify_belief (EntityLocation) confirms present entity -> focused handler test: belief updated with DirectObservation, `observed_tick = current_tick`
2. verify_belief (EntityLocation) absent entity -> focused handler test: `ViolationKind::EntityMissing` in ViolationMemory
3. verify_belief (SupplyAvailability) productive source -> focused handler test: belief updated with fresh observation
4. verify_belief (SupplyAvailability) exhausted source -> focused handler test: `ViolationKind::SupplyDepleted` in ViolationMemory
5. verify_belief aborts on place inaccessibility -> focused handler test: stale belief retained, no violation recorded
6. verify_belief records violation when subject entity destroyed mid-action -> focused handler test: absent on commit -> EntityMissing
7. Action def registered correctly -> `build_full_action_registries` existing test passes with "verify_belief" in catalog

## What to Change

### 1. New `epistemic_actions.rs` module in worldwake-systems

Create `crates/worldwake-systems/src/epistemic_actions.rs` containing:

- `register_verify_belief_action(defs, handlers)`: Registers `ActionDef` with name `"verify_belief"`, domain `ActionDomain::Epistemic`, `FreelyInterruptible`, visibility `SamePlace`. Duration from `VerificationDispositionProfile::verify_belief_duration_ticks`.
- `enumerate_verify_belief_payloads(world, actor, defs)`: Returns payloads for beliefs the actor could verify at their current location. Reads the agent's belief store to identify verifiable subjects.
- `validate_verify_belief_payload_authoritatively(world, actor, payload)`: Actor alive, not incapacitated, at the place specified in the payload's `VerificationSubject`.
- `start_verify_belief(world_txn, instance)`: Validates preconditions, starts the action.
- `tick_verify_belief(world_txn, instance)`: Standard duration tick-down returning `ActionProgress`.
- `commit_verify_belief(world_txn, instance, event_log)`: Dispatch on `VerificationSubject`:
  - `EntityLocation { entity, place }`: Check if entity is at place. If present, update `BelievedEntityState` with `observed_tick = current_tick`, `source = DirectObservation`. If absent, record `ViolationKind::EntityMissing { entity, expected_place: place }` in actor's `ViolationMemory`.
  - `SupplyAvailability { commodity, source, place }`: Check if source has `available_quantity > 0` for commodity. If productive, update belief with fresh DirectObservation. If depleted, record `ViolationKind::SupplyDepleted { commodity, source, place }`.
- `abort_verify_belief(world_txn, instance)`: No-op (agent retains stale belief, spent ticks consumed).

### 2. Register in action_registry.rs

Add `register_verify_belief_action(&mut defs, &mut handlers);` call in `register_all_actions()` in `crates/worldwake-systems/src/action_registry.rs`.

Add `"verify_belief"` to the required action names list in the `build_full_action_registries_returns_complete_action_catalog` test.

### 3. Wire module

Add `pub mod epistemic_actions;` to `crates/worldwake-systems/src/lib.rs`.

## Files to Touch

- `crates/worldwake-systems/src/epistemic_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify — add registration call + test name)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)

## Out of Scope

- `ask_witness` handler — ticket 004 (same file, separate ticket for reviewability)
- Planner ops (`PlannerOpKind::VerifyBelief`) — ticket 005
- Candidate generation (`emit_verify_belief_goals`) — ticket 006
- Ranking integration — ticket 007
- Golden E2E tests — ticket 008
- Changes to `ViolationMemory`, `ViolationKind`, or `AgentBeliefStore` structures (these already exist and support the needed operations)
- Affordance payload enumeration for `ask_witness` — ticket 004

## Acceptance Criteria

### Tests That Must Pass

1. `verify_belief` (EntityLocation): confirms present entity with fresh `DirectObservation` (`observed_tick = current_tick`)
2. `verify_belief` (EntityLocation): generates `EntityMissing` violation for absent entity
3. `verify_belief` (SupplyAvailability): confirms productive source with fresh observation
4. `verify_belief` (SupplyAvailability): generates `SupplyDepleted` violation for exhausted source
5. `verify_belief` aborts cleanly if place accessibility changes mid-action; agent retains stale belief
6. `verify_belief` records violation if subject entity destroyed mid-action (absent on commit)
7. Authoritative payload validation rejects actor not at target place
8. Authoritative payload validation rejects incapacitated actor
9. Existing suite: `cargo test -p worldwake-systems`
10. Existing suite: `cargo test -p worldwake-core` (unchanged)

### Invariants

1. Epistemic actions update belief stores, never authoritative world state beyond the belief store and violation memory (P12)
2. Conservation invariant unaffected — no items created or destroyed
3. Determinism — handler uses no `HashMap`/`HashSet`, no floats, no wall-clock time
4. Existing `build_full_action_registries` test passes with new action in catalog

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/epistemic_actions.rs` (in-module tests) — 6 focused handler tests per spec test list items 1-6
2. `crates/worldwake-systems/src/action_registry.rs` (modify existing test) — add "verify_belief" to required names

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
3. `cargo build --workspace`
