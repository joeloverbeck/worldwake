# S21-005: Update save/load round-trip test to verify commitment preservation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: S21-002, S21-003, S21-004

## Problem

The existing `golden_save_load_round_trip_under_ai` test passes because agents re-derive equivalent behavior within 30 post-resume ticks. But it does not assert that causally relevant state (active goal, journey commitment, facility intents) is actually preserved across save/load. This ticket adds explicit assertions proving the promoted components survive the round-trip.

## Assumption Reassessment (2026-03-23)

1. `golden_save_load_round_trip_under_ai` lives in `crates/worldwake-ai/tests/golden_determinism.rs`. Confirmed.
2. `GoldenHarness::save_load_roundtrip()` at `golden_harness/mod.rs` lines 1161–1167 uses `snapshot_state()` → `save_to_bytes()` → `load_from_bytes()` → `from_simulation_state()`.
3. `from_simulation_state()` at line 1182 creates `AgentTickDriver::new(PlanningBudget::default())` — after S21-002/003/004, the promoted state lives in `World` components and survives this reconstruction.
4. The scenario must engineer at least one agent mid-journey at save time. This requires a multi-place topology with a travel edge long enough (5+ ticks) that the save falls within the travel window.
5. `ActiveGoal` preservation: any agent with an active goal at save time should have the same `GoalKey` after load.
6. `JourneyCommitment` preservation: the mid-journey agent should have matching `destination`, `state == Active`, `established_at`.
7. `FacilityQueueIntents` preservation: if any agent has queued facility intents at save time, they should match after load. (May need to engineer this or assert empty-is-preserved.)

## Architecture Check

1. The test uses the standard golden harness save/load mechanism — no custom serialization needed.
2. No backward-compatibility shims. The test directly asserts component equality.

## Verification Layers

1. `ActiveGoal` round-trip → component read before save matches component read after load
2. `JourneyCommitment` round-trip → component read before save matches component read after load
3. `FacilityQueueIntents` round-trip → component read before save matches component read after load
4. Post-load agent continues journey (not restarting) → agent reaches destination within expected remaining ticks
5. Deterministic replay of the post-load segment → replay hashes match

## What to Change

### 1. Extend or add golden save/load test

In `crates/worldwake-ai/tests/golden_determinism.rs`, add a test (or extend the existing `golden_save_load_round_trip_under_ai`) that:

1. Sets up a multi-place topology with at least one long travel edge (5+ tick duration)
2. Places a goal-satisfying resource at a remote place so an agent must travel
3. Runs ticks until the agent is mid-travel (verify via `agent_active_action_name` or action traces)
4. Reads `ActiveGoal`, `JourneyCommitment`, `FacilityQueueIntents` components for the traveling agent
5. Performs save/load round-trip via `h.save_load_roundtrip()`
6. Reads the same components after load
7. Asserts field-level equality: `goal_key`, `destination`, `state`, `established_at`, `last_progress_tick`, `consecutive_blocked_leg_ticks`
8. Runs remaining ticks and asserts the agent reaches the destination within expected time (not full journey restart)

### 2. Add deterministic replay companion

If the test is new, add a replay companion that verifies the post-load segment produces identical state hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — add/extend save/load commitment test)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — may need helper to read promoted components from World)

## Out of Scope

- Component definitions or registration (S21-001)
- AI code migration (S21-002, S21-003, S21-004)
- Changes to `save_to_bytes`/`load_from_bytes` (components auto-serialize via `ComponentTables`)
- Testing non-AI save/load paths (sim-level save/load tests already exist)
- Facility queue intent scenario engineering if no current golden scenario naturally produces intents at save time (assert empty-round-trips correctly in that case)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_determinism` — new/extended save/load test passes
2. `cargo test -p worldwake-ai --test golden_determinism -- save_load` — targeted run of save/load tests
3. The test asserts non-trivial state: at least one agent has `Some(ActiveGoal)` and `Some(JourneyCommitment)` at save time (not just asserting that `None == None`)
4. `cargo test --workspace` — no regressions

### Invariants

1. `JourneyCommitment.destination` after load matches before save
2. `JourneyCommitment.state` after load is `Active` (same as before save)
3. `JourneyCommitment.established_at` after load matches before save
4. `ActiveGoal.goal_key` after load matches before save
5. Post-load agent does not restart its journey — it continues from where it was

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_determinism.rs::golden_save_load_preserves_promoted_commitments` (or extended `golden_save_load_round_trip_under_ai`) — proves active goal and journey commitment survive save/load
2. Deterministic replay companion for the above — proves post-load execution is deterministic

### Commands

1. `cargo test -p worldwake-ai --test golden_determinism`
2. `cargo test --workspace`
