# S102FROAWAEXP-004: Planner failure tracking + exploration gate modification

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — plan search outcome handler (worldwake-ai), emit_exploration_candidates gate logic, tracker reset
**Deps**: S102FROAWAEXP-001, S102FROAWAEXP-002, S102FROAWAEXP-003

## Problem

Two interacting defects prevent agents from exploring when they should:

1. The exploration gate in `emit_exploration_candidates()` suppresses exploration whenever `need_has_known_acquisition_path()` returns true — even when the planner repeatedly budget-exhausts trying to plan acquisition. There's no feedback from planning failures to the exploration gate.

2. There's no mechanism to increment or reset the `AcquisitionExhaustionTracker` (from ticket 002) based on plan outcomes.

This ticket wires up both: incrementing the tracker on budget-exhaustion, and reading it in the gate to bypass suppression when the count exceeds threshold.

## Assumption Reassessment (2026-04-14)

1. `emit_exploration_candidates()` at `crates/worldwake-ai/src/candidate_generation.rs:2331-2398`. Suppression condition at lines 2379-2383 checks `pressure < threshold || any_local_need_relief || need_has_known_acquisition_path`. Variable `need_id` is in scope (loop at line 2362 iterates `(need_id, pressure, matches_need)` over Hunger, Thirst, Dirtiness).
2. `PlanSearchResult::BudgetExhausted` at `crates/worldwake-ai/src/search/mod.rs:246`. Budget exhaustion detected at lines 391-392 and 512-513.
3. `GoalKind::AcquireCommodity` at `crates/worldwake-core/src/goal.rs:22-25` has `commodity: CommodityKind` and `purpose: CommodityPurpose`. `ProduceCommodity` at lines 64-66 has `recipe_id: RecipeId`.
4. Commodity-to-need mapping: filter functions `relieves_hunger`, `relieves_thirst`, `relieves_dirtiness` exist in candidate_generation.rs. `CommodityKindSpec.consumable_profile` has `hunger_relief_per_unit`, `thirst_relief_per_unit`, `bladder_fill_per_unit` fields.
5. `GoalBeliefView::acquisition_exhaustion_count()` will be available from ticket 003.
6. Tracker reset via mutable access: `emit_exploration_candidates` reads through `&dyn GoalBeliefView` (read-only). Reset mutations must be deferred — accumulated as pending and applied after candidate generation, or the reset can be done in the decision tick's write phase via `WorldTxn`.

## Architecture Check

1. Incrementing on budget-exhaustion and reading in the gate keeps the feedback loop entirely within the AI crate, mediated by stored ECS state (FND-26). No cross-system function calls.
2. The lazy reset (when need < threshold) in candidate generation avoids coupling the needs system to exploration-awareness. The reset is safe because a low-pressure need wouldn't trigger exploration regardless of counter value.
3. No backward-compatibility shims. The gate condition is modified in-place.

## Verification Layers

1. Tracker incremented on BudgetExhausted for need-satisfying goals → focused unit test or decision trace
2. Gate bypassed when count >= threshold → decision trace showing ExploreLocation emitted despite known path
3. Counter resets when need drops below threshold → focused test or decision trace
4. Existing exploration behavior preserved when count < threshold → existing golden tests pass

## What to Change

### 1. Commodity-to-need reverse mapping

Create a helper function (in `candidate_generation.rs` or a shared utility) that maps `CommodityKind` → `Option<HomeostaticNeedId>` by checking `CommodityKindSpec.consumable_profile` fields for non-zero relief values. Multiple needs may map — return the primary (highest relief).

### 2. Increment tracker on budget exhaustion

At the point where plan search returns `BudgetExhausted` for an `AcquireCommodity` or `ProduceCommodity` goal, determine the `HomeostaticNeedId` via the reverse mapping and call `tracker.increment(need_id)` on the agent's `AcquisitionExhaustionTracker` via `WorldTxn`.

### 3. Modify exploration gate

In `emit_exploration_candidates()` at the suppression condition (line ~2379):

- When `pressure < profile.need_activation_threshold`: accumulate a pending tracker reset for this need (deferred to write phase)
- Otherwise: read `ctx.view.acquisition_exhaustion_count(ctx.agent, need_id)`, compute `path_reliable = count < profile.acquisition_failure_threshold`, and only suppress if `path_reliable && need_has_known_acquisition_path()`

### 4. Implement deferred tracker reset

Design the reset mechanism following the existing pattern for mutations during the decision tick. Options: (a) return pending resets from candidate generation for the caller to apply, (b) use a mutation accumulator. The exact mechanism should match existing worldwake-ai patterns for deferred writes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — gate logic, reset accumulation)
- `crates/worldwake-ai/src/planning.rs` or `agent_tick/` (modify — increment on BudgetExhausted)
- Possibly `crates/worldwake-ai/src/agent_tick/tick_step.rs` (modify — apply deferred resets)

## Out of Scope

- Multi-hop BFS target selection (ticket 005)
- Belief protection via synthetic ticks (ticket 006)
- Modifying the existing per-goal exhaustion_cache or retry backoff logic
- Changing how ExploreLocation goals dispatch, plan, or execute
- Changing exploration caps (curiosity_weight=0, max_consecutive_explorations)

## Acceptance Criteria

### Tests That Must Pass

1. Budget-exhausted AcquireCommodity for food increments Hunger count
2. Budget-exhausted ProduceCommodity for food increments Hunger count
3. Gate emits ExploreLocation when count >= threshold despite known acquisition path
4. Gate suppresses ExploreLocation when count < threshold and known acquisition path exists
5. Counter resets to 0 when need pressure drops below threshold on next candidate generation pass
6. Existing suite: `cargo test --workspace`

### Invariants

1. Tracker is only incremented for need-satisfying goals (AcquireCommodity, ProduceCommodity) — not for travel, combat, social goals
2. Systems interact through stored state only (FND-26) — no direct cross-system calls
3. Exploration still suppressed when `curiosity_weight == 0` or consecutive cap exceeded (existing guards unchanged)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for gate bypass when tracker count >= threshold
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused test for gate suppression when count < threshold
3. `crates/worldwake-ai/` — focused test for tracker increment on BudgetExhausted
4. `crates/worldwake-ai/` — focused test for lazy reset when need < threshold

### Commands

1. `cargo test -p worldwake-ai -- emit_exploration`
2. `cargo test -p worldwake-ai -- exhaustion`
3. `cargo build --workspace && cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
