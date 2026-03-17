# S03PLATARIDE-004: Search integration tests for exact target binding

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: S03PLATARIDE-001, S03PLATARIDE-002, S03PLATARIDE-003

## Problem

Unit tests for `matches_binding()` prove the method's logic in isolation, but do not prove that the planner actually rejects wrong-target affordances during search. Integration tests must demonstrate that when multiple same-type entities exist at the same place, the planner only uses the one matching the goal's canonical target. These tests also verify that binding rejection traces are populated correctly.

## Assumption Reassessment (2026-03-17)

1. Golden test infrastructure exists in `crates/worldwake-ai/src/agent_tick.rs` — test harness with `h.step_once()`, `h.driver.enable_tracing()`, trace sink queries.
2. `LootCorpse { corpse }` and `EngageHostile { target }` are exact-bound goals that already have action defs and handlers registered.
3. `Sleep` is a flexible goal with no target binding.
4. The test harness can place multiple entities at the same place and create affordances for each.
5. `DecisionTraceSink` supports `trace_at()`, `traces_for()`, and `dump_agent()` queries.
6. `PlanAttemptTrace` will have `binding_rejections: Vec<BindingRejection>` after S03PLATARIDE-003.

## Architecture Check

1. Integration tests use the existing golden test harness — no new test infrastructure needed.
2. Tests prove the full pipeline: candidate generation → ranking → search (with binding filter) → plan selection.
3. Tests verify both the behavioral outcome (correct plan target) and the diagnostic output (binding rejection traces).

## What to Change

### 1. Two-corpses-at-same-place test

Set up a scenario with an agent and two corpses (X and Y) at the same place. Give the agent a `LootCorpse { corpse: X }` goal. Verify:
- Search produces a plan that targets corpse X specifically.
- The Loot step's targets include X, not Y.
- If tracing is enabled, `BindingRejection` entries appear for any Loot affordance targeting Y.

### 2. Two-hostiles-at-same-place test

Set up a scenario with an agent and two hostiles (A and B) at the same place. Give the agent an `EngageHostile { target: A }` goal. Verify:
- Search produces a plan with an Attack step targeting A, not B.
- Binding rejection traces show rejection of Attack affordances targeting B.

### 3. Flexible-goal-unaffected test

Set up a scenario with an agent and multiple sleep-compatible entities at the same place. Give the agent a `Sleep` goal. Verify:
- Search accepts any available sleep affordance — no binding rejections occur.
- The binding filter does not narrow down candidates for flexible goals.

### 4. Binding rejection trace verification test

Enable tracing. Run a scenario where wrong-target candidates exist. Query `PlanAttemptTrace.binding_rejections` and verify:
- `def_id` matches the rejected action def.
- `rejected_targets` contains the wrong entity.
- `required_target` contains the goal's canonical target entity.
- `dump_agent()` output includes binding rejection lines (test via string inspection or just verify non-panic).

### 5. Empty-targets planner-only candidate test

Verify that planner-only synthetic candidates (from `planner_only_candidates()`) with empty `authoritative_targets` are not rejected by the binding filter even for exact-bound goals.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add new `#[test]` functions in the test module)

## Out of Scope

- `matches_binding()` implementation — S03PLATARIDE-001.
- Search filter wiring — S03PLATARIDE-002.
- Trace struct definition — S03PLATARIDE-003.
- `BuryCorpse` tests (no action def exists yet — documented for future).
- `ShareBelief` integration test (already covered by E15b golden tests; binding for Tell is tested in S03PLATARIDE-001 unit tests).
- Any changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`.

## Acceptance Criteria

### Tests That Must Pass

1. `test_binding_two_corpses_same_place` — agent with `LootCorpse { corpse: X }` only loots X, not Y.
2. `test_binding_two_hostiles_same_place` — agent with `EngageHostile { target: A }` only attacks A, not B.
3. `test_binding_flexible_goal_unaffected` — `Sleep` goal accepts any sleep affordance.
4. `test_binding_rejection_trace_populated` — trace includes `BindingRejection` entries for rejected wrong-target candidates.
5. `test_binding_empty_targets_planner_only_bypass` — synthetic candidates with empty targets pass binding filter.
6. All existing golden tests: `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`

### Invariants

1. Exact-bound goals cannot silently retarget to a sibling affordance of the same family.
2. Flexible goals remain flexible — no false rejections.
3. Planner determinism preserved across all new test scenarios.
4. `DeclareSupport` with empty `bound_targets` continues to work via payload override.
5. Planner-only synthetic candidates with empty `authoritative_targets` bypass binding.
6. Binding rejection traces expose which candidates were rejected and why.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — 5 new golden-style integration tests covering two-corpses, two-hostiles, flexible-goal, rejection-trace, and empty-targets-bypass scenarios.

### Commands

1. `cargo test -p worldwake-ai -- test_binding`
2. `cargo test --workspace && cargo clippy --workspace`
