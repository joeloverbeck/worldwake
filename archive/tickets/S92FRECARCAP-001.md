# S92FRECARCAP-001: Shared disposal contract helper and satisfaction rewrite

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner goal satisfaction logic for `FreeCarryCapacity`
**Deps**: `archive/specs/S82-waste-disposal-inventory-management.md`, `archive/specs/S92-free-carry-capacity-zero-step-loop-fix.md`

## Problem

`FreeCarryCapacity` has four divergent contracts across candidate emission, ranking, goal satisfaction, and terminal transition. At the planning root, `is_satisfied()` can return `true` while `emit_disposal_candidates()` still emits candidates, yielding a 0-step `GoalSatisfied` plan that blocks self-care goals. The root cause is that satisfaction, emission, and ranking each compute load and threshold differently and on different data surfaces (`PlanningState` vs `GoalBeliefView`).

## Assumption Reassessment (2026-04-11)

1. `GoalKind::FreeCarryCapacity` exists in `crates/worldwake-ai/src/goal_model.rs` with `is_satisfied()` at line 1255. Current satisfaction logic: with `DisposalProfile`, checks `current_load * 1000 < capacity * threshold`; without profile, compares against baseline load from fresh `PlanningState`. Load computed via `carried_load_of_actor()` (capacity minus remaining, recursive BFS). Confirmed 2026-04-11.
2. `emit_disposal_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` at line 3097 computes load by summing `CommodityKind::ALL` quantities times `load_per_unit()` via `GoalBeliefView`. Uses default threshold 800 when `DisposalProfile` absent. Different load-computation method from `is_satisfied()`. Confirmed 2026-04-11.
3. Shared abstraction boundary: the new helper accepts pre-computed `(load, capacity, threshold, has_waste_targets, root_baseline_load)` so it can be called from both `GoalBeliefView` (emission/ranking) and `PlanningState` (satisfaction) surfaces without coupling them.
4. Ticket said "focused parity tests are in S92FRECARCAP-003" and "None in this ticket", but live `crates/worldwake-ai/src/goal_model.rs` already contains focused `FreeCarryCapacity` satisfaction tests. Honest proof for this ticket is the satisfaction rewrite plus those existing module-local tests, with any renamed/adjusted assertions kept aligned to the new default-threshold contract. Corrected 2026-04-11.
5. Live `GoalKind` under test: `GoalKind::FreeCarryCapacity`. Operator surface: `PlannerOpKind::DropItem` (confirmed in `goal_dispatch_decl.rs`). Satisfaction feeds `terminal_kind()` in `search/transition.rs` line 241.

## Architecture Check

1. A pre-computed-inputs helper is cleaner than a trait abstraction or PlanningState-bound helper because it avoids coupling the contract to a specific data surface. Each call site extracts values from its own surface and passes them to a single canonical decision function. No new traits or generics needed.
2. No backward-compatibility shims. The old `is_satisfied()` implementation is replaced entirely, not wrapped.

## Verification Layers

1. Root satisfaction must be `false` when disposal is actionable -> focused unit test on `is_satisfied()` with strained root state in `crates/worldwake-ai/src/goal_model.rs`
2. Satisfaction becomes `true` only after simulated disposal progress below threshold -> focused unit test with modified `PlanningState` in `crates/worldwake-ai/src/goal_model.rs`
3. Default-threshold satisfaction path without an explicit `DisposalProfile` stays aligned with the unified contract -> focused unit test in `crates/worldwake-ai/src/goal_model.rs`
6. Single-layer ticket: all changes are within `worldwake-ai` planner contract logic. No cross-system or authoritative-layer interaction introduced.

## What to Change

### 1. Create shared disposal contract helper

Add a helper function (in `goal_model.rs` or a new planner-local module) that accepts pre-computed values:

- `current_load: LoadUnits`
- `carry_capacity: LoadUnits`
- `disposal_threshold: Permille` (from `DisposalProfile` or default 800)
- `has_waste_targets: bool`
- `root_baseline_load: Option<LoadUnits>` (only for satisfaction path)

The helper computes:
- `is_actionable() -> bool`: load exceeds threshold AND waste targets exist
- `is_satisfied(root_baseline_load) -> bool`: not actionable, OR (load decreased relative to root baseline AND load now below threshold)

### 2. Rewrite `FreeCarryCapacity` branch in `is_satisfied()`

Replace the current `GoalKind::FreeCarryCapacity` branch in `is_satisfied()` to:
1. Extract `current_load`, `carry_capacity`, `disposal_threshold`, `has_waste_targets` from `PlanningState`
2. Compute `root_baseline_load` from `PlanningState::new(state.snapshot())`
3. Call the shared helper

The new satisfaction contract:
- If not actionable (below threshold or no waste targets): satisfied (goal should not compete)
- If actionable at root: root satisfaction is `false`
- After simulated disposal: satisfied only when load decreased relative to root baseline AND load is now below threshold

### 3. Export helper for consumption by emission and ranking (ticket 002)

Ensure the helper function has appropriate visibility (`pub(crate)`) for use by `candidate_generation.rs` and `ranking.rs`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)

## Out of Scope

- Modifying `emit_disposal_candidates()` or `motive_score()` — that is S92FRECARCAP-002
- Adding new components, actions, or entities
- Changing `PlannerOpKind::DropItem` operator wiring
- Generic planner-wide zero-step plan suppression
- Rebalancing hunger, metabolism, or utility weights
- Changing unrelated S91 pathologies

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_waste_disposal_cycle` — existing S82 disposal contract preserved
2. `cargo test -p worldwake-ai` — no regressions in AI test suite
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean clippy

### Invariants

1. `FreeCarryCapacity.is_satisfied()` must return `false` at the planning root when the unified contract says disposal is actionable
2. Satisfaction requires disposal progress (load decrease) relative to root baseline, not merely being in a disposal-compatible state
3. When disposal is not actionable (below threshold or no waste), the goal is satisfied and should not compete

## Test Plan

### New/Modified Tests

1. Modify the existing focused `FreeCarryCapacity` satisfaction tests in `crates/worldwake-ai/src/goal_model.rs` so their names/assertions reflect the unified default-threshold contract.

### Commands

1. `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
2. `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-11.

- Added a shared `FreeCarryCapacityContract` helper in `crates/worldwake-ai/src/goal_model.rs` that accepts pre-computed load/capacity/threshold/waste-target inputs and exposes the canonical `is_actionable()` / `is_satisfied()` decision surface needed by this ticket and follow-up ticket 002.
- Rewrote `GoalKind::FreeCarryCapacity.is_satisfied()` to use the unified contract with the planning snapshot's root baseline load, the default disposal threshold of 800 when no `DisposalProfile` exists, and direct-waste-target availability from `free_carry_capacity_drop_targets(state)`.
- Updated the existing focused satisfaction test naming/assertion in `crates/worldwake-ai/src/goal_model.rs` so the local proof surface reflects the new default-threshold contract instead of the old "no-profile load reduction fallback" wording.
- Ticket file remains untracked in the current worktree; code change is tracked in `crates/worldwake-ai/src/goal_model.rs`.

## Verification Result

- Passed `cargo test -p worldwake-ai free_carry_capacity_ -- --nocapture`
- Passed `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
