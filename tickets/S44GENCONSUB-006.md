# S44GENCONSUB-006: Contention-aware action validation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — action validation pipeline in worldwake-sim, action handlers in worldwake-systems
**Deps**: S44GENCONSUB-004, S44GENCONSUB-005

## Problem

Loot, bury, and heal actions currently resolve contention by implicit tick order — whoever's action starts first wins. FOUNDATIONS P8 requires explicit resolution through the contention substrate. Action validation must gate these actions through `ContentionQueue` grant checks, returning structured results (queued, rejected) instead of silent success/failure.

## Assumption Reassessment (2026-04-03)

1. `register_loot_action()` at `crates/worldwake-systems/src/combat.rs:55-64`. Action handler: `start_loot, tick_loot, commit_loot, abort_loot`. Confirmed.
2. `register_bury_action()` at same file lines 66-77. Confirmed.
3. `register_heal_action()` at same file lines 79-90. Domain `ActionDomain::Care`. Confirmed.
4. `validate_action_def_authoritatively()` at `crates/worldwake-sim/src/action_validation.rs:145-164` checks actor constraints and preconditions. No contention check currently. Confirmed.
5. `ActionTraceKind::StartFailed` at `crates/worldwake-sim/src/action_trace.rs:62-66` provides structured failure with reason string. This is the existing pattern for contention rejection.
6. `start_gate.rs` in worldwake-sim handles action start pipeline including BestEffort mode.
7. After S44GENCONSUB-004 and 005, `ContentionQueue`, `ContentionPolicy`, and `ContentionStatus` are available.
8. Contention check must happen in the action start pipeline — if the target entity has a `ContentionQueue` and the actor doesn't hold the grant, the action should either enqueue the actor or reject.

## Architecture Check

1. Contention checks integrate into the existing validation pipeline — no new pipeline stage, just additional checks during action start. Clean extension of existing pattern.
2. No backward-compatibility shims.

## Verification Layers

1. Action on contention-managed entity without grant → StartFailed with structured reason → action trace
2. Action on contention-managed entity with grant → action starts normally → action trace
3. Enqueue on available queue → actor added to waiting → authoritative world state
4. Reject on full queue → StartFailed with "contention_rejected" → action trace
5. Cross-layer: action start (sim) reads contention state (core), produces traces (sim). All state-mediated (P26).

## What to Change

### 1. Add contention gate to action start pipeline

In `crates/worldwake-sim/src/start_gate.rs` (or appropriate action start location): before starting a loot/bury/heal action on an entity with a `ContentionQueue`, check:
- If actor holds the grant → proceed normally
- If queue has room → enqueue actor, update `ContentionIntents`, return "queued, not started"
- If queue full (including race mode `max_waiters: Some(0)`) → return structured "contention_rejected" StartFailed

### 2. Add contention enqueue helper

Create a helper function that handles the enqueue-and-track flow: adds to `ContentionQueue.waiting`, updates the actor's `ContentionIntents`, emits appropriate event log entry.

### 3. Release grant on action completion

In loot/bury/heal commit and abort handlers: when the action completes or is aborted, if the actor holds the contention grant on the target entity, clear the grant. This allows the contention system to promote the next waiter.

### 4. Update ContentionIntents on dequeue

When an actor's action completes or is aborted, remove the entity from the actor's `ContentionIntents`.

## Files to Touch

- `crates/worldwake-sim/src/start_gate.rs` (modify — add contention gate)
- `crates/worldwake-systems/src/combat.rs` (modify — release grant in loot/bury/heal commit/abort)
- `crates/worldwake-sim/src/action_trace.rs` (modify — if structured contention reason needed)

## Out of Scope

- Attaching ContentionQueue to entities (S44GENCONSUB-007)
- Perception of contention (S44GENCONSUB-008)
- Golden tests (S44GENCONSUB-009)

## Acceptance Criteria

### Tests That Must Pass

1. Loot action on entity with ContentionQueue where actor holds grant → action starts
2. Loot action on entity with ContentionQueue where actor does NOT hold grant and queue has room → actor enqueued, action not started
3. Loot action on entity with full queue → StartFailed with contention_rejected
4. Grant released on loot commit
5. Grant released on loot abort
6. Existing suite: `cargo test --workspace`

### Invariants

1. Actions on entities without ContentionQueue are unaffected
2. Grant holder identity matches actor identity check (no spoofing)
3. ContentionIntents stays in sync with ContentionQueue (add on enqueue, remove on dequeue/complete)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/start_gate.rs` (tests) — contention gate behavior
2. `crates/worldwake-systems/src/combat.rs` (tests) — grant release on loot/bury/heal completion

### Commands

1. `cargo test -p worldwake-sim start_gate`
2. `cargo test -p worldwake-systems combat`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
