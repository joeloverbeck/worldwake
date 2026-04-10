# S82WASDISINV-002: Register drop_item action and handler

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new action definition and handler in transport domain
**Deps**: S82WASDISINV-001

## Problem

No action exists for agents to voluntarily remove items from inventory by placing them on the ground. This ticket adds the `drop_item` action following the existing `put_down` pattern in `transport_actions.rs`.

## Assumption Reassessment (2026-04-10)

1. `register_transport_actions()` exists at `transport_actions.rs:18` with signature `pub fn register_transport_actions(defs: &mut ActionDefRegistry, handlers: &mut ActionHandlerRegistry) -> Vec<ActionDefId>`.
2. `put_down` ActionDef exists at lines 78-108 and remains the authoritative pattern to mirror, but the live transport contract now uses `DurationExpr::Fixed(NonZeroU32::MIN)`, `Permille::new_unchecked(100)`, and `Interruptibility::InterruptibleWithPenalty` rather than the older spec draft's `2` ticks / `50‰` / `FreelyInterruptible`. `commit_put_down` at lines 575-592 uses `txn.clear_possessor`, `txn.set_ground_location`, `txn.add_target`.
3. Shared functions `tick_transport` (lines 652-660) and `abort_transport` (lines 663-671) exist and can be reused.
4. All referenced types confirmed: `ActionDomain::Transport`, `TargetSpec::EntityDirectlyPossessedByActorAnyOf` (kinds: `[EntityKind; 2]`), `Precondition::TargetDirectlyPossessedByActor`, `Precondition::TargetAtActorPlace`, `ActionPayload::None`, `Interruptibility::InterruptibleWithPenalty`, `VisibilitySpec::ParticipantsOnly`, `EventTag::{WorldMutation, Inventory, Transfer}`, `DurationExpr::Fixed`, `BodyCostPerTick::zero()`, `Constraint::{ActorAlive, ActorHasControl}`.

## Architecture Check

1. Follows existing `put_down` pattern exactly — same preconditions, commit logic, and transport domain. Distinguished only by action name for planner routing.
2. No backward-compatibility shims. New action only.

## Verification Layers

1. Action registers without error -> focused unit test confirming `drop_item` appears in registry
2. Commit transfers possession correctly -> focused test: actor possesses item, commit clears possessor and sets ground location
3. Single-layer ticket (action framework only) — authoritative mutation verified via world state assertions

## What to Change

### 1. Action definition

In `crates/worldwake-systems/src/transport_actions.rs`, within `register_transport_actions()`, add a new `ActionDef` for `drop_item` alongside the existing `put_down` definition. Use identical `targets`, `preconditions`, `commit_conditions`, duration, attention cost, and interruptibility as the live `put_down` definition so the transport domain stays internally consistent.

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

1. `drop_item` action definition appears in registry with correct domain, preconditions, and the same timing/interruptibility contract as live `put_down`
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

## Outcome

- Completed: 2026-04-10
- Changed `crates/worldwake-systems/src/transport_actions.rs` to register a distinct Transport-domain `drop_item` action, add dedicated `start_drop_item` / `commit_drop_item` handlers, and expand the transport test harness for the extra action id.
- Added focused proof that `drop_item` registers with the same live timing/interruptibility contract as `put_down`, commits by moving directly possessed items back to ground without changing owner, and aborts without mutating inventory state.
- Deviations from original plan: the ticket and active spec initially described the older `2`-tick / `50‰` / `FreelyInterruptible` draft contract. Reassessment showed live `put_down` already uses `DurationExpr::Fixed(NonZeroU32::MIN)`, `Permille::new_unchecked(100)`, and `Interruptibility::InterruptibleWithPenalty`, so the implemented `drop_item` action mirrored the live transport contract instead.
- Verification results:
  - `cargo test -p worldwake-systems transport_actions::tests::`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace --all-targets -- -D warnings`
