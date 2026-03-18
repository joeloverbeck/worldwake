# E16DPOLPLAN-005: Unit tests for Bribe/Threaten planning state transitions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-003, E16DPOLPLAN-004

## Problem

The new Bribe and Threaten arms in `apply_planner_step` need targeted unit tests verifying state transitions.

## Assumption Reassessment (2026-03-18)

1. `apply_planner_step` is a method on `GoalKind` — confirmed
2. Test setup requires `PlanningState` with a `SnapshotEntity` containing commodity quantities, combat profile, and courage — confirmed
3. `GoalKind::ClaimOffice { office }` is the only goal kind that uses Bribe/Threaten — confirmed

## Architecture Check

1. Tests isolated to planning state transitions — no authoritative world state needed
2. Each test verifies a single state transition outcome

## What to Change

### 1. Add test module in `goal_model.rs`

9 unit tests:
1. **Bribe with sufficient goods**: actor has 5 bread, bribes target -> commodity drops, support added
2. **Bribe with insufficient goods**: actor has 0 bread -> state unchanged
3. **Bribe with no payload**: step has no bribe payload -> state unchanged
4. **Threaten yield**: attack_skill=800 > courage=200 -> support added
5. **Threaten resist**: attack_skill=200 < courage=800 -> state unchanged
6. **Threaten missing combat profile**: no combat profile on actor -> state unchanged
7. **Threaten missing target courage**: target has no courage -> state unchanged (defaults MAX)
8. **Bribe under non-ClaimOffice goal**: e.g. ConsumeOwnedCommodity -> state unchanged
9. **Threaten under non-ClaimOffice goal**: -> state unchanged

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — test module)

## Out of Scope

- Integration tests with full planner search (E16DPOLPLAN-006)
- Golden E2E tests
- Changes to production code
- Authoritative handler tests

## Acceptance Criteria

### Tests That Must Pass

1. All 9 unit tests pass
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Tests verify state transitions only — no side effects on authoritative world
2. Each test is independent (no shared mutable state)

## Test Plan

### New/Modified Tests

1. `goal_model.rs::tests::bribe_sufficient_goods_deducts_and_adds_support`
2. `goal_model.rs::tests::bribe_insufficient_goods_unchanged`
3. `goal_model.rs::tests::bribe_no_payload_unchanged`
4. `goal_model.rs::tests::threaten_yield_adds_support`
5. `goal_model.rs::tests::threaten_resist_unchanged`
6. `goal_model.rs::tests::threaten_missing_combat_profile_unchanged`
7. `goal_model.rs::tests::threaten_missing_courage_unchanged`
8. `goal_model.rs::tests::bribe_non_claim_office_unchanged`
9. `goal_model.rs::tests::threaten_non_claim_office_unchanged`

### Commands

1. `cargo test -p worldwake-ai goal_model::tests`
2. `cargo test -p worldwake-ai`
