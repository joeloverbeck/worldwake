# S92FRECARCAP-002: Migrate emission and ranking to shared contract

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate emission and ranking for `FreeCarryCapacity`
**Deps**: `archive/tickets/S92FRECARCAP-001.md`, `specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

`emit_disposal_candidates()` and `motive_score()` each compute load and threshold independently using `GoalBeliefView`, with slightly different logic from each other and from `is_satisfied()`. After ticket 001 introduces the shared contract helper, emission and ranking must consume it so all three sites agree on "needs disposal" and "already solved."

## Assumption Reassessment (2026-04-11)

1. `emit_disposal_candidates()` at `crates/worldwake-ai/src/candidate_generation.rs:3097` computes load by summing `CommodityKind::ALL * load_per_unit()` via `ctx.view`. Uses default threshold 800 when no `DisposalProfile`. Emits one candidate per directly-possessed Waste lot from `known_entity_beliefs()`. Confirmed 2026-04-11.
2. `FreeCarryCapacity` branch in `motive_score()` at `crates/worldwake-ai/src/ranking.rs:602` computes strain as `(carried_commodity_load * 1000) / capacity` with no threshold check. Returns `score_product(enterprise_weight, strain)`. Currently never returns 0 for sub-threshold agents. Confirmed 2026-04-11.
3. Shared boundary: both functions receive `GoalBeliefView` via their context. The clean DRY implementation is a thin extractor helper in `goal_model.rs` that derives `(load, capacity, threshold, has_waste_targets)` from the view and returns a `FreeCarryCapacityContract` for both call sites to consume. Corrected 2026-04-11.
4. `S92FRECARCAP-001` is no longer an active ticket path; the completed dependency now lives at `archive/tickets/S92FRECARCAP-001.md`. Corrected 2026-04-11.
5. Live `GoalKind`: `FreeCarryCapacity`. Operator: `PlannerOpKind::DropItem`. The `carried_commodity_load()` helper at `ranking.rs:1208` sums `CommodityKind::ALL` quantities — same approach as emission. After this ticket, both use the shared contract's actionability check.
6. Ticket said "focused parity tests are in S92FRECARCAP-003" and "None in this ticket", but live focused tests already exist in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs` for `FreeCarryCapacity` emission and motive behavior. Honest proof for this ticket is the code change plus those existing focused tests, while 003 still owns additional parity coverage beyond the already-present module-local tests. Corrected 2026-04-11.

## Architecture Check

1. Consuming the shared helper from both `GoalBeliefView`-based call sites ensures emission, ranking, and satisfaction all agree on actionability. Each site extracts pre-computed values from its view and delegates the decision to one function. No new abstraction layers or traits needed.
2. No backward-compatibility shims. The old inline threshold checks in emission and ranking are replaced.

## Verification Layers

1. Candidate emission occurs only when shared contract says actionable -> existing focused unit tests in `crates/worldwake-ai/src/candidate_generation.rs`
2. Motive score returns 0 when not actionable -> existing focused unit tests in `crates/worldwake-ai/src/ranking.rs`
3. S82 physical operator path preserved -> `golden_waste_disposal_cycle` continues passing
4. Single-layer ticket: changes are within `worldwake-ai` planner logic. No cross-system interaction.

## What to Change

### 1. Update `emit_disposal_candidates()` to use shared helper

Replace the inline load computation and threshold check with:
1. Build the shared contract from `ctx.view`
2. Call the shared helper's `is_actionable()` — return early if not actionable
3. Keep the existing waste-lot iteration for candidate emission (iterating `known_entity_beliefs` for directly-possessed Waste lots)

### 2. Update `FreeCarryCapacity` branch in `motive_score()` to use shared helper

Replace the inline strain computation with:
1. Build the shared contract from `context.view`
2. Call the shared helper's `is_actionable()` — return 0 if not actionable
3. When actionable, compute strain using the same load/capacity values and return `score_product(enterprise_weight, strain)`

### 3. Remove `carried_commodity_load()` if now unused

If the private `carried_commodity_load()` helper at `ranking.rs:1208` is only used by the `FreeCarryCapacity` branch and the new code computes load differently, remove it. If used elsewhere, leave it.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
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

1. Update the existing focused `FreeCarryCapacity` tests in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/ranking.rs` so their expectations reflect the unified actionability contract.

### Commands

1. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
2. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-11.

- Added `free_carry_capacity_contract_from_view()` in `crates/worldwake-ai/src/goal_model.rs` so both `GoalBeliefView` call sites derive the same `FreeCarryCapacityContract` inputs instead of recomputing load, threshold, and waste-target actionability separately.
- Rewrote `emit_disposal_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` to gate emission on the shared contract and to require directly possessed Waste lots with positive believed quantity.
- Rewrote the `FreeCarryCapacity` motive branch in `crates/worldwake-ai/src/ranking.rs` to return 0 when disposal is not actionable, compute strain from the shared contract when it is actionable, and remove the now-unused `carried_commodity_load()` helper.
- Updated the existing focused `FreeCarryCapacity` tests in `crates/worldwake-ai/src/ranking.rs` and relied on the existing focused candidate-generation tests as the narrow proof surface for the unified contract.

## Verification Result

- Passed `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
- Passed `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
