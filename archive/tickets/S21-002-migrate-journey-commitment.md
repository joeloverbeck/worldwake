# S21-002: Migrate journey commitment fields from runtime to JourneyCommitment component

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — AI crate runtime reads/writes migrated to World/WorldTxn component access
**Deps**: S21-001

## Problem

Six journey-related fields on `AgentDecisionRuntime` are causally relevant (affect multi-tick behavior) but lost on save/load. After S21-001 defines the `JourneyCommitment` component, this ticket migrates all reads and writes of those fields to use the authoritative component through `World`/`WorldTxn`.

## Assumption Reassessment (2026-03-23)

1. Journey fields to promote are at `decision_runtime.rs` lines 70–75: `journey_committed_goal`, `journey_committed_destination`, `journey_commitment_state`, `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`.
2. Read sites confirmed via grep `journey_committed` in AI crate:
   - `agent_tick/journey.rs` (lines 21–49, 51–100) — primary journey management
   - `agent_tick/active_action.rs` — journey state checks during active action handling
   - `agent_tick/mod.rs` — journey orchestration in per-tick driver
   - `agent_tick/planning.rs` — journey-aware plan adoption
   - `plan_selection.rs` (lines 504–505, 664–665) — journey relation in plan selection
   - `failure_handling.rs` (lines 1100–1102, 1145–1147) — journey clearing on failure
   - `interrupts.rs` (lines 484–485, 672–673, 849–850) — journey state in interrupt evaluation
3. Journey helper methods on `AgentDecisionRuntime` that access these fields: `has_journey_commitment()`, `journey_committed_destination()`, `has_active_journey_travel()`, `journey_runtime_snapshot()`, `classify_journey_plan_relation()`, `clear_journey_commitment()`.
4. `last_journey_clear_reason` (diagnostic-only) stays on `AgentDecisionRuntime` — NOT promoted per spec.
5. `remaining_travel_steps()` reads only `current_plan`/`current_step_index` — stays on runtime, no change needed.
6. Unit tests in `decision_runtime.rs` construct `AgentDecisionRuntime` with journey fields directly — must be updated to use component or refactored helpers.
7. This ticket does NOT touch `current_goal` or `queued_facility_intents` — those are S21-003 and S21-004.

## Architecture Check

1. Journey helpers become free functions with explicit `Option<&JourneyCommitment>` parameter instead of `&self` methods. This is cleaner: the parameter makes the data source explicit and enables callers to pass either a World read or a WorldTxn read.
2. No backward-compatibility shims — old methods are removed, not deprecated.

## Verification Layers

1. All journey reads now go through `world.get_component_journey_commitment(agent)` or `txn` equivalent → grep for `runtime.journey_committed` must return zero hits after migration
2. All journey writes go through `txn.set_component_journey_commitment(agent, ...)` or `txn.clear_component_journey_commitment(agent)` → grep for `runtime.journey_committed` assignment must return zero hits
3. No behavioral change → all golden tests pass with unchanged hashes
4. Journey fields removed from `AgentDecisionRuntime` → struct no longer has the six fields
5. Single-layer: focused AI unit tests + golden E2E coverage already verify journey behavior end-to-end

## What to Change

### 1. Refactor journey helpers to free functions

In `crates/worldwake-ai/src/decision_runtime.rs` (or a new `journey_helpers.rs` if cleaner), convert the following methods to free functions:

- `has_journey_commitment(jc: Option<&JourneyCommitment>) -> bool`
- `journey_committed_destination(jc: Option<&JourneyCommitment>) -> Option<EntityId>`
- `has_active_journey_travel(jc: Option<&JourneyCommitment>, plan: Option<&PlannedPlan>, step_index: usize) -> bool`
- `journey_runtime_snapshot(jc: Option<&JourneyCommitment>, runtime: &AgentDecisionRuntime) -> JourneyRuntimeSnapshot`
- `classify_journey_plan_relation(jc: Option<&JourneyCommitment>, plan: &PlannedPlan) -> JourneyPlanRelation`

### 2. Migrate reads in agent_tick/ modules

In each sub-module (`journey.rs`, `active_action.rs`, `mod.rs`, `planning.rs`):
- Read `JourneyCommitment` from `world.get_component_journey_commitment(agent)` (or WorldTxn read)
- Pass it to the refactored free functions
- Replace `runtime.journey_commitment_state`, `runtime.journey_established_at`, etc. with reads from the component

### 3. Migrate writes in agent_tick/ modules

- Journey establishment: `txn.set_component_journey_commitment(agent, JourneyCommitment { ... })`
- Journey clearing: `txn.clear_component_journey_commitment(agent)` (plus `runtime.last_journey_clear_reason = Some(...)` stays on runtime)
- Progress tick updates: `txn.set_component_journey_commitment(agent, updated_commitment)`
- Blocked tick increments: read-modify-write through txn

### 4. Migrate reads/writes in plan_selection.rs, failure_handling.rs, interrupts.rs

Replace `runtime.journey_committed_goal` / `runtime.journey_commitment_state` reads with component reads. Update test fixtures in those files to set up `JourneyCommitment` components instead of runtime fields.

### 5. Remove journey fields from AgentDecisionRuntime

Delete the six promoted fields from the struct definition (lines 81–86). Update the `Default` impl. Remove now-dead helper methods.

### 6. Update unit tests in decision_runtime.rs

Tests that construct `AgentDecisionRuntime` with journey fields must either:
- Use the refactored free functions with explicit `JourneyCommitment` values, or
- Be moved to integration-style tests that set up a World with the component

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove 6 fields, convert helpers to free functions)
- `crates/worldwake-ai/src/agent_tick/journey.rs` (modify — read/write through World/WorldTxn)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — read journey component)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — pass journey component through driver)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — journey-aware plan adoption via component)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update test fixtures)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — read component in plan selection + tests)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — journey clearing via txn + tests)
- `crates/worldwake-ai/src/interrupts.rs` (modify — read component in interrupt eval + tests)

## Out of Scope

- `current_goal` field migration (S21-003)
- `queued_facility_intents` field migration (S21-004)
- New save/load golden tests proving commitment preservation (S21-005)
- Any changes to `worldwake-core` component definitions (S21-001)
- Any changes to `worldwake-sim` or `worldwake-systems`
- Changes to `GoldenHarness::from_simulation_state` (journey state now auto-survives via component; the fresh `AgentTickDriver` no longer needs to carry it)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all unit tests and golden tests pass
2. `cargo test -p worldwake-ai -- journey` — journey-specific unit tests updated and passing
3. `cargo test -p worldwake-ai --test golden_determinism` — deterministic replay hashes unchanged
4. `cargo test -p worldwake-ai --test golden_ai_decisions` — all AI decision goldens pass
5. `cargo test -p worldwake-ai --test golden_emergent` — all emergent goldens pass
6. `cargo clippy --workspace` — no new warnings

### Invariants

1. Zero occurrences of `runtime.journey_committed_goal`, `runtime.journey_committed_destination`, `runtime.journey_commitment_state`, `runtime.journey_established_at`, `runtime.journey_last_progress_tick`, `runtime.consecutive_blocked_leg_ticks` in the codebase (grep verification)
2. `AgentDecisionRuntime` no longer has the six journey fields
3. `last_journey_clear_reason` remains on `AgentDecisionRuntime` (diagnostic-only, not promoted)
4. `remaining_travel_steps()` remains on `AgentDecisionRuntime` (reads only plan/step_index)
5. All journey writes go through `WorldTxn`, producing `ComponentDelta` entries in the event log
6. No golden test behavioral changes — agents make the same decisions with the same hashes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` tests — updated to use free functions with explicit `JourneyCommitment` values
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — updated fixtures to set `JourneyCommitment` component on agents
3. `crates/worldwake-ai/src/plan_selection.rs` tests — updated to set component instead of runtime fields
4. `crates/worldwake-ai/src/failure_handling.rs` tests — updated to set component instead of runtime fields
5. `crates/worldwake-ai/src/interrupts.rs` tests — updated to set component instead of runtime fields

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-23
- **What changed**: Removed 6 journey fields from `AgentDecisionRuntime`. Created 5 free functions (`has_journey_commitment`, `journey_committed_destination`, `has_active_journey_travel`, `journey_runtime_snapshot`, `classify_journey_plan_relation`). Added `persist_journey_commitment` helper using WorldTxn (produces `ComponentDelta` in event log). Threaded `Option<JourneyCommitment>` through the entire agent_tick pipeline. Updated 12 source files (706 insertions, 477 deletions).
- **Deviations**: `update_journey_fields_for_adopted_plan` renamed to `update_journey_for_adopted_plan` and changed to return `Option<JourneyCommitment>` rather than mutating runtime fields. `handle_recoverable_travel_step_blockage` returns `(bool, Option<JourneyCommitment>)` tuple instead of just `bool`. `advance_completed_step` returns `Option<JourneyCommitment>`. All functions that previously read/wrote runtime journey fields now take `Option<&JourneyCommitment>` or `&mut Option<JourneyCommitment>` parameters. `planner_ops.rs` was confirmed to have no journey references and was not touched (corrected in ticket reassessment).
- **Verification**: `cargo test -p worldwake-ai` 32 passed. `cargo test --workspace` 2400+ passed. `cargo clippy --workspace` zero warnings. Zero occurrences of `runtime.journey_committed_*` in codebase. Golden test hashes unchanged.
