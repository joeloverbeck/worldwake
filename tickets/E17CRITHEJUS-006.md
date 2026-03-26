# E17CRITHEJUS-006: Implement steal action in worldwake-systems

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definition + handler in systems crate
**Deps**: E17CRITHEJUS-001 (needs `TheftDispositionProfile`), E17CRITHEJUS-004 (needs `GoalKind::StealItem`)

## Problem

No action allows taking items owned by others. The only acquisition paths are lawful `pick_up` (requires `can_exercise_control`) and `trade`. E17 needs a `steal` action that transfers possession without transferring ownership, with `VisibilitySpec::Hidden` and `EventTag::Crime`.

## Assumption Reassessment (2026-03-25)

1. `transport_actions.rs` in worldwake-systems contains `pick_up` and `put_down` handlers. Steal shares the same relation mutation (`set_possessor`) but inverts the `can_exercise_control` gate.
2. `register_investigate_action()` in `investigate_actions.rs` is the closest structural precedent for a new action registration function.
3. `action_registry.rs` contains `register_all_actions()` which calls all per-domain registration functions.
4. `ActionDomain::Transport` exists in `crates/worldwake-sim/src/action_domain.rs`.
5. `VisibilitySpec::Hidden` exists in the event/visibility types.
6. `EventTag::Crime` already exists (confirmed unused) in `crates/worldwake-core/src/event_tag.rs`.
7. `TheftDispositionProfile.steal_duration_ticks` provides the profile-driven duration (P2 — no magic numbers).
8. N/A — not a start-failure ticket, though start-fail path uses established S08 contract.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. A new `steal_actions.rs` module follows the established one-module-per-action-family pattern (`transport_actions.rs`, `investigate_actions.rs`, `combat.rs`). Registration function wired into `register_all_actions()`.
2. No backwards-compatibility aliasing introduced. The steal handler is a new code path; `pick_up` is not modified.

## Verification Layers

1. Steal transfers possession (not ownership) -> authoritative world state check in focused test
2. Conservation maintained after steal -> `verify_live_lot_conservation()` in focused test
3. Event emitted with `EventTag::Crime` and `VisibilitySpec::Hidden` -> event log delta check
4. Abort produces no transfer -> authoritative world state check
5. Start-fail when `can_exercise_control == true` -> S08 start-failure contract
6. Start-fail when item possessed by another agent -> S08 contract
7. Start-fail when insufficient load capacity -> S08 contract

## What to Change

### 1. New `steal_actions.rs` module in worldwake-systems

- `register_steal_action()` following `register_investigate_action()` pattern.
- Action definition: name `"steal"`, domain `ActionDomain::Transport`, `TargetSpec::SpecificEntity`, `VisibilitySpec::Hidden`, tags `[EventTag::Crime, EventTag::Transfer]`.
- Duration: `DurationExpr::ProfileDriven` reading `TheftDispositionProfile.steal_duration_ticks`.
- `Interruptibility::FreelyInterruptible`.

### 2. Start handler

Validate authoritatively:
- Actor and target at same place
- Target is `EntityKind::ItemLot`
- Target has an owner other than actor
- `can_exercise_control(actor, target) == false`
- Target not currently possessed by another agent
- Target not reserved
- Actor has remaining load capacity

Return `StartFailed` on any precondition failure.

### 3. Commit handler

- `txn.set_possessor(target_item, actor)` — transfer possession
- Ownership relation unchanged
- Emit event with `EventTag::Crime`, `VisibilitySpec::Hidden`, `WitnessData` with actor as sole direct participant

### 4. Abort handler

No-op. Interrupted theft produces no transfer.

### 5. Register in action_registry.rs

Wire `register_steal_action()` into `register_all_actions()`.

### 6. Export from lib.rs

Add `pub mod steal_actions;` and `pub use steal_actions::register_steal_action;`.

## Files to Touch

- `crates/worldwake-systems/src/steal_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify — wire registration)
- `crates/worldwake-systems/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Accuse/Fine/Exile actions (E17CRITHEJUS-008/009)
- Investigate commit extension for SuspectedTheft (E17CRITHEJUS-007)
- AI candidate generation for theft (E17CRITHEJUS-010)
- Perception system changes (none needed per spec)
- Steal affordance generation in worldwake-ai (covered by planner ops + candidate generation tickets)
- Modifying `pick_up` or `transport_actions.rs` in any way
- Golden tests (E17CRITHEJUS-012)

## Acceptance Criteria

### Tests That Must Pass

1. Steal transfers possession: after commit, `possessor_of(item) == actor`
2. Steal does NOT transfer ownership: `owner_of(item)` unchanged
3. Conservation maintained: `verify_live_lot_conservation()` passes before and after steal
4. Event emitted with `EventTag::Crime` tag
5. Event emitted with `VisibilitySpec::Hidden`
6. Abort produces no possession change
7. Start-fail when `can_exercise_control(actor, item) == true` (item is lawfully accessible)
8. Start-fail when item possessed by another agent (robbery is out of scope)
9. Start-fail when actor lacks load capacity
10. Action duration matches `TheftDispositionProfile.steal_duration_ticks`
11. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `pick_up` handler completely unchanged
2. Conservation invariant holds for all steal outcomes (commit and abort)
3. Ownership relation never mutated by steal
4. `VisibilitySpec::Hidden` on all steal events (no crime event is public)
5. Only agents with `TheftDispositionProfile` can have a steal action started (enforced by duration resolution)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/steal_actions.rs` — focused tests for commit (possession transfer, conservation, event tags, visibility), abort (no-op), and start-failure (control gate, possession gate, capacity gate)

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
3. `cargo build --workspace`
