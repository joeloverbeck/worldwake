# E15RUMWITDIS-005: Add TellActionPayload and Tell ActionDef Registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action payload variant in sim, new action def in registry
**Deps**: E15RUMWITDIS-002 (ActionDomain::Social), E15RUMWITDIS-003 (TellProfile)

## Problem

The Tell action needs its payload type added to the `ActionPayload` enum in worldwake-sim, and its `ActionDef` registered in the action definition registry with correct preconditions, duration, domain, and interruptibility settings per the E15 spec.

## Assumption Reassessment (2026-03-14)

1. `ActionPayload` in `crates/worldwake-sim/src/action_payload.rs` — confirmed. Currently 8 payload variants plus `None` default.
2. `ActionDef` in `crates/worldwake-sim/src/action_def.rs` — declarative action type definitions with name, domain, duration, interruptibility, etc.
3. Action registration pattern: `register_all_actions()` in `crates/worldwake-systems/src/action_registry.rs` calls domain-specific registration functions.
4. `TellProfile` will be available after E15RUMWITDIS-003.
5. Preconditions per spec: actor alive, listener exists/Agent/same-place/alive, actor has belief about subject, actor's TellProfile.max_relay_chain_len permits chain depth.

## Architecture Check

1. Follows established action registration pattern (see travel_actions, trade_actions, combat, etc.).
2. ActionDef uses the existing precondition/duration/domain system — no new framework needed.
3. No backwards-compatibility shims.

## What to Change

### 1. Add `TellActionPayload` struct

In `crates/worldwake-sim/src/action_payload.rs`, add:

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TellActionPayload {
    pub listener: EntityId,
    pub subject_entity: EntityId,
}
```

### 2. Add `Tell(TellActionPayload)` variant to `ActionPayload`

Add the new variant to the existing enum.

### 3. Create `tell_actions` module in worldwake-systems

Create `crates/worldwake-systems/src/tell_actions.rs` with:
- Tell `ActionDef` construction (name: "tell", domain: Social, duration: 2 ticks fixed, body cost: zero, interruptibility: FreelyInterruptible)
- `register_tell_action()` function following the pattern of other action registrations
- Stub handler (start: no-op, tick: no-op, commit: no-op for now — handler logic is E15RUMWITDIS-006)

### 4. Wire into `register_all_actions()`

In `crates/worldwake-systems/src/action_registry.rs`, add `register_tell_action(defs, handlers)` call.

### 5. Export tell_actions module

In `crates/worldwake-systems/src/lib.rs`, add `pub mod tell_actions;`.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add TellActionPayload, Tell variant)
- `crates/worldwake-systems/src/tell_actions.rs` (new — Tell ActionDef + stub handler + registration)
- `crates/worldwake-systems/src/action_registry.rs` (modify — wire register_tell_action)
- `crates/worldwake-systems/src/lib.rs` (modify — export tell_actions)

## Out of Scope

- Tell handler commit logic (source degradation, belief transfer) — that is E15RUMWITDIS-006
- Tell affordance enumeration — that is E15RUMWITDIS-007
- Mismatch detection or discovery events
- Any AI/planner changes
- belief_confidence() function

## Acceptance Criteria

### Tests That Must Pass

1. Tell action registered with `ActionDomain::Social`
2. Tell ActionDef has duration of 2 ticks
3. Tell ActionDef is FreelyInterruptible
4. `TellActionPayload` serializes and deserializes correctly
5. `ActionPayload::Tell(TellActionPayload { .. })` constructs correctly
6. `verify_completeness()` passes (every ActionDef has a handler)
7. Existing suite: `cargo test --workspace`
8. `cargo clippy --workspace`

### Invariants

1. All existing action registrations unchanged
2. ActionPayload enum remains Default (None)
3. Existing handler registry completeness maintained

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — test that Tell ActionDef is registered with Social domain, 2-tick duration, FreelyInterruptible
2. `crates/worldwake-systems/src/action_registry.rs` — verify_completeness still passes after adding Tell

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
