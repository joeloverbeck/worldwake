# S82WASDISINV-002: Register drop_item action and handler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new action definition and handler in transport domain
**Deps**: S82WASDISINV-001

## Problem

No action exists for agents to voluntarily remove items from inventory by placing them on the ground. This ticket adds the `drop_item` action following the existing `put_down` pattern in `transport_actions.rs`.

## Assumption Reassessment (2026-04-10)

1. `register_transport_actions()` exists at `transport_actions.rs:18` with signature `pub fn register_transport_actions(defs: &mut ActionDefRegistry, handlers: &mut ActionHandlerRegistry) -> Vec<ActionDefId>`.
2. `put_down` ActionDef exists at lines 86-116 with the exact precondition/commit pattern the spec references. `commit_put_down` at lines 575-592 uses `txn.clear_possessor`, `txn.set_ground_location`, `txn.add_target`.
3. Shared functions `tick_transport` (lines 652-660) and `abort_transport` (lines 663-671) exist and can be reused.
4. All referenced types confirmed: `ActionDomain::Transport`, `TargetSpec::EntityDirectlyPossessedByActorAnyOf` (kinds: `[EntityKind; 2]`), `Precondition::TargetDirectlyPossessedByActor`, `Precondition::TargetAtActorPlace`, `ActionPayload::None`, `Interruptibility::FreelyInterruptible`, `VisibilitySpec::ParticipantsOnly`, `EventTag::{WorldMutation, Inventory, Transfer}`, `DurationExpr::Fixed`, `BodyCostPerTick::zero()`, `Constraint::{ActorAlive, ActorHasControl}`.

## Architecture Check

1. Follows existing `put_down` pattern exactly — same preconditions, commit logic, and transport domain. Distinguished only by action name for planner routing.
2. No backward-compatibility shims. New action only.

## Verification Layers

1. Action registers without error -> focused unit test confirming `drop_item` appears in registry
2. Commit transfers possession correctly -> focused test: actor possesses item, commit clears possessor and sets ground location
3. Single-layer ticket (action framework only) — authoritative mutation verified via world state assertions

## What to Change

### 1. Action definition

In `crates/worldwake-systems/src/transport_actions.rs`, within `register_transport_actions()`, add a new `ActionDef` for `drop_item` alongside the existing `put_down` definition. Use identical `targets`, `preconditions`, and `commit_conditions`. Duration: 2 ticks, attention cost: 50 permille, freely interruptible.

### 2. Handler functions

In the same file:
- `start_drop_item`: Validate actor possesses target, return `ActionState::Empty`.
- `commit_drop_item`: `txn.clear_possessor(target)` + `txn.set_ground_location(target, actor_place)` + `txn.add_target(target)`. Identical to `commit_put_down`.
- Reuse `tick_transport` and `abort_transport` for tick and abort.

### 3. Handler registration

Register the new `ActionHandler` struct via `handlers.register(drop_item_id, drop_item_handler)`.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs` (modify)

## Out of Scope

- Planner integration for `drop_item` (ticket 004)
- Candidate generation (ticket 007)
- Golden tests (ticket 008)
- Any changes to `put_down` behavior

## Acceptance Criteria

### Tests That Must Pass

1. `drop_item` action definition appears in registry with correct domain, preconditions, and duration
2. `commit_drop_item` clears possessor and sets ground location at actor's place
3. `abort_drop_item` is a no-op (item stays in inventory)
4. `verify_conservation` passes after drop_item commit
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Item identity preserved — same EntityId before and after drop
2. Item count conserved — no items created or destroyed
3. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/transport_actions.rs` (test module) — test drop_item commit transfers possession to ground
2. `crates/worldwake-systems/src/transport_actions.rs` (test module) — test drop_item abort leaves item in inventory

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
