# GOLDE2E-007: Materialization Binding Failure

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible — materialization binding failure → plan failure → blocked intent path may be incomplete
**Deps**: None (materialization bindings exist from HARHYPENTIDE tickets)

## Problem

Scenario 4 proves the happy-path materialization barrier chain (harvest → pick-up → eat). This gap tests the failure path: an agent plans craft → consume, but between craft completion and the consume step, another agent picks up the crafted item. The materialization binding cannot resolve, triggering plan failure and blocked intent creation. This failure/recovery path is untested.

## Report Reference

Backlog item **P-NEW-6** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `MaterializationBindings` exists (from HARHYPENTIDE-005) and resolves planned-entity references to real entities.
2. `handle_plan_failure()` exists in `worldwake-ai/src/failure_handling.rs`.
3. `BlockedIntentMemory` and `BlockedIntent` exist in `worldwake-core/src/blocked_intent.rs`.
4. Two-agent scenarios at the same workstation are feasible in the golden harness.

## Architecture Check

1. Materialization failure should flow through the standard `handle_plan_failure()` path.
2. The blocked intent should record the barrier so the planner avoids the same plan on the next cycle.
3. No special-case error handling — the standard plan revalidation detects the stale binding.

## Engine-First Mandate

If implementing this e2e suite reveals that materialization binding failure detection, the plan-failure-to-blocked-intent pipeline, or the planner's ability to avoid re-selecting a broken plan is incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Two agents (Crafter, Thief) at the same workstation. Crafter has inputs for a recipe and is critically hungry. Thief is also critically hungry with no inputs. Crafter crafts an item; Thief opportunistically picks up the output before Crafter can.

**Assertions**:
- Crafter completes the craft action, output materializes on ground.
- Thief picks up the materialized output before Crafter's next step.
- Crafter's planned consume step fails (materialization binding cannot resolve).
- `handle_plan_failure()` fires; blocked intent or replan occurs.
- Crafter does not crash or deadlock — finds an alternative path or records the blocked intent.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- Engine files TBD if binding failure path is incomplete

## Out of Scope

- Multi-step binding chains (bind-of-bind failures)
- Materialization binding success path (already proven in scenario 4)
- Inventory theft during actions (item locking)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_materialization_binding_failure` — crafter's plan fails when output is stolen, agent replans or records blocked intent
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual plan injection
2. Conservation holds (items transferred, never duplicated)
3. No crash or deadlock on binding failure

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-6 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_materialization_binding_failure` — proves binding failure recovery

### Commands

1. `cargo test -p worldwake-ai golden_materialization_binding`
2. `cargo test --workspace && cargo clippy --workspace`
