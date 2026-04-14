# S102FROAWAEXP-004: Planner failure tracking + exploration gate modification

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — read-phase deferred reset plumbing, plan search outcome handling, exploration gate logic, tracker persistence
**Deps**: archive/tickets/S102FROAWAEXP-001.md, archive/tickets/S102FROAWAEXP-002.md, archive/tickets/S102FROAWAEXP-003.md

## Problem

Two interacting defects prevent agents from exploring when they should:

1. The exploration gate in `emit_exploration_candidates()` suppresses exploration whenever `need_has_known_acquisition_path()` returns true — even when the planner repeatedly budget-exhausts trying to plan acquisition. There's no feedback from planning failures to the exploration gate.

2. There's no mechanism to increment or reset the `AcquisitionExhaustionTracker` (from ticket 002) based on plan outcomes.

This ticket wires up both: incrementing the tracker on budget-exhaustion, and reading it in the gate to bypass suppression when the count exceeds threshold.

## Assumption Reassessment (2026-04-14)

1. The ticket’s referenced spec path is stale. The live spec is `specs/S102-frontier-aware-exploration.md`, and its Deliverables 3-4 are the controlling contract for this ticket.
2. `emit_exploration_candidates()` lives at `crates/worldwake-ai/src/candidate_generation.rs:2331-2398`. The current suppression condition still short-circuits on `pressure < threshold || any_local_need_relief || need_has_known_acquisition_path`, with `need_id` already in scope for Hunger, Thirst, and Dirtiness.
3. `generate_candidates_with_travel_horizon()` already has a lawful deferred side-effect channel: it returns `CandidateGenerationResult`, and `agent_tick/observation.rs` already carries `pending_violations` through the read phase. That is the right architectural seam for pending tracker resets as well.
4. `PlanSearchResult::BudgetExhausted` still originates in `crates/worldwake-ai/src/search/mod.rs`, but the AI-owned post-search handling point is `record_exhausted_goals()` in `crates/worldwake-ai/src/agent_tick/planning.rs`, not a generic tick-step file.
5. `process_agent()` in `crates/worldwake-ai/src/agent_tick/mod.rs` is the live writeback boundary. It already persists `ExplorationProfile.consecutive_exploration_count` through `update_exploration_counter_for_adopted_goal()`, so the new `AcquisitionExhaustionTracker` writes should follow the same hidden-transaction pattern there.
6. `GoalBeliefView::acquisition_exhaustion_count()` is already live from ticket 003.
7. Commodity-to-need mapping cannot lawfully collapse to “component present vs. absent.” The live exploration gate has distinct Hunger / Thirst / Dirtiness branches, and `relieves_dirtiness()` is a water-specific special case. The reverse mapping must preserve every need a commodity can relieve, or dirtiness-driven water failures would never open the dirtiness exploration gate.

## Architecture Check

1. Incrementing on budget exhaustion and reading in the gate keeps the feedback loop entirely within `worldwake-ai`, mediated by stored ECS state (FND-26). No cross-system function calls are introduced.
2. The lazy reset still belongs to candidate generation semantically, but the actual mutation must be deferred through the existing read-phase result and then persisted in `agent_tick/mod.rs` via `WorldTxn`.
3. Planner result handling and component persistence remain split intentionally: `agent_tick/planning.rs` decides which needs exhausted, while `agent_tick/mod.rs` owns the authoritative writeback.
4. No backward-compatibility shims. The exploration gate and planning outcome handling are modified in place.

## Verification Layers

1. Candidate generation emits pending reset signals when pressure drops below threshold for a need with non-zero exhaustion
2. Tracker increment bookkeeping identifies need-satisfying BudgetExhausted `AcquireCommodity` and `ProduceCommodity` goals
3. Gate bypasses the known-path suppression when count >= threshold
4. Gate still suppresses exploration when count < threshold and a known path exists
5. Existing exploration behavior and workspace checks still pass

## What to Change

### 1. Commodity-to-need reverse mapping

Create a shared AI helper that maps `CommodityKind` to the full set of `HomeostaticNeedId` values it can relieve, reusing the live Hunger / Thirst / Dirtiness gating semantics. `Water` must continue to count for dirtiness as well as thirst.

### 2. Surface tracker increments out of planning

At the point where plan search returns `BudgetExhausted` for an `AcquireCommodity` or `ProduceCommodity` goal, derive the relieved need set from the goal's target commodity and surface those pending increments out of `agent_tick/planning.rs`. Persist them in `agent_tick/mod.rs` through the same hidden `WorldTxn` writeback style already used for exploration-profile updates.

### 3. Modify exploration gate

In `emit_exploration_candidates()` at the suppression condition (line ~2379):

- When `pressure < profile.need_activation_threshold`: accumulate a pending tracker reset for this need through `CandidateGenerationResult`
- Otherwise: read `ctx.view.acquisition_exhaustion_count(ctx.agent, need_id)`, compute `path_reliable = count < profile.acquisition_failure_threshold`, and only suppress if `path_reliable && need_has_known_acquisition_path()`

### 4. Carry deferred tracker resets through the read/write boundary

Extend the existing candidate-generation/read-phase plumbing so pending need resets are carried out of `generate_candidates_with_travel_horizon()` and applied in `agent_tick/mod.rs` before the later planning/execution writeback completes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — gate logic, reset accumulation)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — carry pending reset data through the read phase)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — surface need-tracker increments from BudgetExhausted results)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — persist pending resets and increments)

## Out of Scope

- Multi-hop BFS target selection (ticket 005)
- Belief protection via synthetic ticks (ticket 006)
- Modifying the existing per-goal exhaustion_cache or retry backoff logic
- Changing how ExploreLocation goals dispatch, plan, or execute
- Changing exploration caps (curiosity_weight=0, max_consecutive_explorations)

## Acceptance Criteria

### Tests That Must Pass

1. Budget-exhausted `AcquireCommodity` for food increments Hunger tracking
2. Budget-exhausted `ProduceCommodity` for food increments Hunger tracking
3. Gate emits `ExploreLocation` when count >= threshold despite known acquisition path
4. Gate suppresses `ExploreLocation` when count < threshold and known acquisition path exists
5. Candidate generation emits a pending reset when need pressure drops below threshold and exhaustion state is non-zero
6. Existing suite: `cargo test --workspace`

### Invariants

1. Tracker increments are only emitted for need-satisfying goals (`AcquireCommodity`, `ProduceCommodity`) — not for travel, combat, or social goals
2. Systems interact through stored state only (FND-26) — no direct cross-system calls
3. Exploration still suppressed when `curiosity_weight == 0` or consecutive cap exceeded (existing guards unchanged)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for gate bypass when tracker count >= threshold
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused test for gate suppression when count < threshold
3. `crates/worldwake-ai/src/candidate_generation.rs` — focused test for pending reset emission when need pressure drops below threshold
4. `crates/worldwake-ai/src/agent_tick/planning.rs` — focused tests for emitted tracker increments on BudgetExhausted `AcquireCommodity` / `ProduceCommodity`

### Commands

1. `cargo test -p worldwake-ai --lib <exact candidate_generation test ids>`
2. `cargo test -p worldwake-ai --lib <exact agent_tick::planning test ids>`
3. `cargo test -p worldwake-ai`
4. `cargo build --workspace`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-14.

- `emit_exploration_candidates()` now treats known acquisition paths as unreliable once `acquisition_exhaustion_count >= acquisition_failure_threshold`, while preserving the existing local-relief and consecutive-exploration guards.
- Candidate generation now emits deferred per-need tracker resets when pressure drops below the exploration activation threshold and existing exhaustion state is non-zero.
- The AI read/write seam was extended so `agent_tick/observation.rs` carries those pending resets and `agent_tick/mod.rs` persists them through hidden `WorldTxn` writes before the planning/execution phase completes.
- `record_exhausted_goals()` now surfaces per-need tracker increments for budget-exhausted `AcquireCommodity` and `ProduceCommodity` goals, and `agent_tick/mod.rs` persists those increments after planning.
- The commodity-to-need helper landed as a full relieved-need set rather than a single “primary” need so water continues to drive both thirst and dirtiness exploration recovery paths lawfully.

## Deviations

- Reassessment corrected the stale spec reference from `specs/S102FROAWAEXP-004.md` to the live `specs/S102-frontier-aware-exploration.md`.
- The original ticket phrasing implied a single commodity-to-need reverse mapping result. The landed implementation intentionally preserves all relieved needs for a commodity because the live exploration gate has separate Hunger / Thirst / Dirtiness branches and collapsing water to one need would break the dirtiness path.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_emits_exploration_when_food_path_is_known_but_exhausted -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_records_pending_reset_when_need_pressure_drops_below_threshold -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_skips_exploration_when_food_path_is_known -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::relieved_needs_for_commodity_keeps_water_multi_need_mapping -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_replaces_frontier_suppression_with_budget_retry_state -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_emits_hunger_increment_for_budget_exhausted_produce_goal -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
