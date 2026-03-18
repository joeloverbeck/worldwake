# E16DPOLPLAN-003: Implement `apply_planner_step` for `PlannerOpKind::Bribe`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — goal_model.rs planner step logic
**Deps**: E16DPOLPLAN-002

## Problem

`apply_planner_step` in `goal_model.rs` has a `_ => state` catch-all that silently skips Bribe. The planner never observes commodity cost or support outcome from bribing, so it never selects Bribe in plans.

## Assumption Reassessment (2026-03-18)

1. `apply_planner_step` is in `goal_model.rs` with `payload_override: Option<&ActionPayload>` parameter — confirmed
2. `ActionPayload::as_bribe()` exists and returns bribe payload data — confirmed
3. `state.commodity_quantity(actor, commodity)` and `state.with_commodity_quantity()` exist on `PlanningState` — confirmed
4. `state.with_support_declaration(target, office, actor)` exists — confirmed
5. `enumerate_bribe_payloads` offers full commodity stock per payload — confirmed
6. Actor extraction via `state.snapshot().actor()` — confirmed pattern at goal_model.rs:424

## Architecture Check

1. Outcome-based planning (support declaration) is an intentional optimistic GOAP approximation — authoritative handler does loyalty increase, not direct support
2. Follows existing `DeclareSupport` pattern which also writes support declarations directly in `apply_planner_step`

## What to Change

### 1. Replace `_ => state` catch-all arm for Bribe

Under `GoalKind::ClaimOffice { office }`, add explicit `PlannerOpKind::Bribe` arm:
- Read bribe payload from `payload_override.and_then(ActionPayload::as_bribe)`
- Check `commodity_quantity(actor, offered_commodity) >= offered_quantity`
- If sufficient: deduct commodity via `with_commodity_quantity` + add support via `with_support_declaration`
- If insufficient or no payload: return state unchanged

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)

## Out of Scope

- Threaten arm (E16DPOLPLAN-004)
- Exhaustive match arm cleanup (done with E16DPOLPLAN-004)
- Unit tests (E16DPOLPLAN-005)
- Integration tests (E16DPOLPLAN-006)
- Golden tests
- Any changes to `commit_bribe` authoritative handler

## Acceptance Criteria

### Tests That Must Pass

1. Code compiles: `cargo build -p worldwake-ai`
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Bribe arm only activates under `GoalKind::ClaimOffice` — all other goals return state unchanged
2. Commodity quantity cannot go negative (saturating_sub)
3. No new `PlanningState` fields introduced

## Test Plan

### New/Modified Tests

1. Tests deferred to E16DPOLPLAN-005

### Commands

1. `cargo build -p worldwake-ai`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
