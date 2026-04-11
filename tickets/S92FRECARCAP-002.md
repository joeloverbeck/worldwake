# S92FRECARCAP-002: Migrate emission and ranking to shared contract

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate emission and ranking for `FreeCarryCapacity`
**Deps**: `S92FRECARCAP-001`, `specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

`emit_disposal_candidates()` and `motive_score()` each compute load and threshold independently using `GoalBeliefView`, with slightly different logic from each other and from `is_satisfied()`. After ticket 001 introduces the shared contract helper, emission and ranking must consume it so all three sites agree on "needs disposal" and "already solved."

## Assumption Reassessment (2026-04-11)

1. `emit_disposal_candidates()` at `crates/worldwake-ai/src/candidate_generation.rs:3097` computes load by summing `CommodityKind::ALL * load_per_unit()` via `ctx.view`. Uses default threshold 800 when no `DisposalProfile`. Emits one candidate per directly-possessed Waste lot from `known_entity_beliefs()`. Confirmed 2026-04-11.
2. `FreeCarryCapacity` branch in `motive_score()` at `crates/worldwake-ai/src/ranking.rs:602` computes strain as `(carried_commodity_load * 1000) / capacity` with no threshold check. Returns `score_product(enterprise_weight, strain)`. Currently never returns 0 for sub-threshold agents. Confirmed 2026-04-11.
3. Shared boundary: both functions receive `GoalBeliefView` via their context. They must extract `(load, capacity, threshold, has_waste_targets)` from the view and pass to the shared helper from 001 to determine actionability.
5. Live `GoalKind`: `FreeCarryCapacity`. Operator: `PlannerOpKind::DropItem`. The `carried_commodity_load()` helper at `ranking.rs:1208` sums `CommodityKind::ALL` quantities — same approach as emission. After this ticket, both use the shared contract's actionability check.

## Architecture Check

1. Consuming the shared helper from both `GoalBeliefView`-based call sites ensures emission, ranking, and satisfaction all agree on actionability. Each site extracts pre-computed values from its view and delegates the decision to one function. No new abstraction layers or traits needed.
2. No backward-compatibility shims. The old inline threshold checks in emission and ranking are replaced.

## Verification Layers

1. Candidate emission occurs only when shared contract says actionable -> focused unit test (S92FRECARCAP-003)
2. Motive score returns 0 when not actionable -> focused unit test (S92FRECARCAP-003)
3. S82 physical operator path preserved -> `golden_waste_disposal_cycle` continues passing
6. Single-layer ticket: changes are within `worldwake-ai` candidate generation and ranking. No cross-system interaction.

## What to Change

### 1. Update `emit_disposal_candidates()` to use shared helper

Replace the inline load computation and threshold check with:
1. Extract `current_load`, `carry_capacity`, `disposal_threshold`, `has_waste_targets` from `ctx.view`
2. Call the shared helper's `is_actionable()` — return early if not actionable
3. Keep the existing waste-lot iteration for candidate emission (iterating `known_entity_beliefs` for directly-possessed Waste lots)

### 2. Update `FreeCarryCapacity` branch in `motive_score()` to use shared helper

Replace the inline strain computation with:
1. Extract `current_load`, `carry_capacity`, `disposal_threshold`, `has_waste_targets` from `context.view`
2. Call the shared helper's `is_actionable()` — return 0 if not actionable
3. When actionable, compute strain using the same load/capacity values and return `score_product(enterprise_weight, strain)`

### 3. Remove `carried_commodity_load()` if now unused

If the private `carried_commodity_load()` helper at `ranking.rs:1208` is only used by the `FreeCarryCapacity` branch and the new code computes load differently, remove it. If used elsewhere, leave it.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Modifying `is_satisfied()` — done in S92FRECARCAP-001
- Adding new components, actions, or entities
- Changing `PlannerOpKind::DropItem` operator wiring or the `drop_item` action path
- Generic planner-wide zero-step plan suppression
- Rebalancing hunger, metabolism, or utility weights

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_waste_disposal_cycle` — S82 disposal contract preserved
2. `cargo test -p worldwake-ai` — no regressions
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean clippy

### Invariants

1. Candidate emission only occurs when the shared contract says disposal is actionable
2. Only directly possessed, non-empty Waste lots qualify as disposal targets
3. Motive score returns 0 when disposal is not actionable
4. `PlannerOpKind::DropItem` remains the only operator for `FreeCarryCapacity` resolution

## Test Plan

### New/Modified Tests

1. None in this ticket — focused parity tests are in S92FRECARCAP-003. Existing golden tests verify no regression.

### Commands

1. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
