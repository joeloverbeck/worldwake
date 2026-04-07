# S68GOASWICON-004: Clear ContentionIntents on remaining plan-clear paths

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — AI agent tick frame, observation, and active_action function signatures
**Deps**: S68GOASWICON-001 (completed)

## Problem

S68GOASWICON-001 fixed stale `ContentionIntents` on the goal-switch and lost-plan paths. However, there are 5 additional `materialization_bindings.clear()` sites that also clear plans without clearing `facility_intents`. These have the same stale-intent risk pattern: if an agent is in a `ContentionQueue` via `QueueForFacilityUse` and the plan is abandoned through one of these paths, the next plan may trigger `DuplicateActor` on re-enqueue.

## Assumption Reassessment (2026-04-07)

1. Remaining `materialization_bindings.clear()` sites that do NOT clear `facility_intents`:
   - `active_action.rs:218` — `ProgressBarrier` plan terminal (agent keeps goal, clears plan)
   - `active_action.rs:236` — `GoalSatisfied`/`CombatCommitment` plan terminal (goal cleared entirely)
   - `mod.rs:513` — assumption failure critical (plan cleared, dirty = ASSUMPTION_FAILED)
   - `frame.rs:198` — frame blockage/stalled (plan cleared, dirty = FRAME_BLOCKAGE)
   - `frame.rs:410` — patience exhausted (plan cleared, dirty = FRAME_PATIENCE)
   - `observation.rs:134` — pursuit invalidation (plan cleared, dirty = REPLAN_SIGNAL)
2. `mod.rs:513` has `current_facility_intents` in scope at the caller level — can be cleared directly without signature threading.
3. `observation.rs:134` is inside `refresh_runtime_for_read_phase` which already takes `facility_intents: &mut ContentionIntents` — can be cleared directly.
4. `frame.rs:198` (`handle_recoverable_travel_step_blockage`) and `frame.rs:410` (`check_patience_exhaustion`) do NOT take `facility_intents` — need parameter threading.
5. `active_action.rs:218,236` (`advance_completed_step`) does NOT take `facility_intents` — needs parameter threading.
6. The plan-terminal paths (active_action.rs:218,236) are lower risk because they fire after an action commits, and `reconcile_committed_facility_queue_intents` removes intents on Harvest/Craft terminal steps. However, if a goal is satisfied while the agent is queued (QueueForFacilityUse committed but no grant yet), stale intents would remain.
7. No adjacent contradictions.

## Architecture Check

1. Same pattern as S68GOASWICON-001: clear `ContentionIntents` on plan abandonment, let `prune_invalid_waiters` handle `ContentionQueue` cleanup. P26 state-mediated interaction.
2. No backwards-compatibility shims.

## Verification Layers

1. Each plan-clear path clears `facility_intents` alongside `materialization_bindings` -> focused test or code inspection
2. Single-layer ticket (AI planning lifecycle).

## What to Change

### 1. Clear facility_intents in mod.rs assumption-failure path

At mod.rs around line 513, after `runtime.materialization_bindings.clear()`, add `current_facility_intents.intents.clear()`. No signature threading needed — variable is in scope.

### 2. Clear facility_intents in observation.rs pursuit-invalidation path

At observation.rs around line 134, after `runtime.materialization_bindings.clear()`, add `facility_intents.intents.clear()`. No signature threading needed — parameter is already available.

### 3. Thread facility_intents into frame.rs functions and clear

Add `facility_intents: &mut ContentionIntents` to `handle_recoverable_travel_step_blockage` (frame.rs:134) and `check_patience_exhaustion` (frame.rs:382). Clear at lines 198 and 410. Update call sites in mod.rs.

### 4. Thread facility_intents into advance_completed_step and clear

Add `facility_intents: &mut ContentionIntents` to `advance_completed_step` (active_action.rs:176). Clear at lines 218 and 236. Update call sites.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — direct clear at assumption-failure, update call sites for frame.rs and active_action.rs)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — direct clear at pursuit-invalidation)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — add parameter, clear at blockage and patience paths)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — add parameter, clear at plan-terminal paths)

## Out of Scope

- Interrupt path — covered by S68GOASWICON-002
- Golden E2E test — covered by S68GOASWICON-003
- Direct ContentionQueue mutation

## Acceptance Criteria

### Tests That Must Pass

1. Existing suite: `cargo test -p worldwake-ai`
2. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every path that clears `materialization_bindings` must also clear `facility_intents.intents`
2. No direct `ContentionQueue` mutation — cleanup is state-mediated via the prune system (P26)

## Test Plan

### New/Modified Tests

1. None — existing test coverage plus S68GOASWICON-003 golden test covers the behavioral contract. The changes are mechanical signature threading following the established pattern from S68GOASWICON-001.

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo clippy --workspace --all-targets -- -D warnings`
