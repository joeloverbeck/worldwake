# S30-006: Remove golden_determinism driver reset workaround

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test-only change
**Deps**: S30-004 (runtime save/restore wired), S30-005 (post-load validation wired)

## Problem

The `golden_save_load_round_trip_under_ai` test currently works around save/load parity loss by creating a fresh `AgentTickDriver::new(PlanningBudget::default())` in `GoldenHarness::from_simulation_state()` (line 1181 of `golden_harness/mod.rs`). This masks the real problem: AI runtime state was lost at the save/load boundary. With S30-004 and S30-005 complete, the driver's state is now preserved — the workaround is no longer needed and should be removed.

## Assumption Reassessment (2026-03-27)

1. `GoldenHarness::from_simulation_state()` at `golden_harness/mod.rs:1168-1188` creates `driver: AgentTickDriver::new(PlanningBudget::default())` — fresh driver with empty `runtime_by_agent`.
2. `golden_save_load_round_trip_under_ai` at `golden_determinism.rs:190-199` calls `save_load_roundtrip()` (line ~100 of same file) which uses `from_simulation_state()`.
3. After S30-003/004/005, the roundtrip flow should: save with driver → load bytes → restore driver from AI payload → post_load_validate → resume.
4. `golden_save_load_preserves_promoted_commitments` also uses this flow — must remain passing.
5. This is the acceptance-criteria gate for the entire S30 spec: the test must pass WITHOUT the driver reset.

## Architecture Check

1. Removing the workaround is the correct approach — the fresh driver was never architecturally justified; it was a workaround for missing serialization.
2. No shims or compatibility code needed — the old workaround is simply deleted.

## Verification Layers

1. Save/load parity → `golden_save_load_round_trip_under_ai` passes without driver reset
2. Promoted commitments → `golden_save_load_preserves_promoted_commitments` continues passing
3. Full determinism → all golden replay companions pass
4. This is a golden/E2E verification ticket — the proof is the passing golden suite.

## What to Change

### 1. Update `GoldenHarness::from_simulation_state()` to accept and use restored driver

Instead of `driver: AgentTickDriver::new(PlanningBudget::default())`, the method should accept the restored driver (already populated from AI payload bytes in the save/load roundtrip flow).

Option A: `from_simulation_state()` gains a `driver: AgentTickDriver` parameter.
Option B: `from_simulation_state()` gains `ai_runtime_bytes: Option<&[u8]>` and internally creates + restores the driver.

Either way, the fresh `AgentTickDriver::new()` reset is removed.

### 2. Update `save_load_roundtrip()` helper

Ensure the roundtrip flow:
1. Saves with `save_to_bytes(&state, Some(&driver))`
2. Loads via `load_from_bytes(&bytes)` → `(state, Some(ai_bytes))`
3. Constructs harness with the restored driver (not a fresh one)
4. Calls `post_load_validate(&world)` on the driver

### 3. Verify determinism

The uninterrupted run and the resumed run must produce identical world hashes at every tick after the save point. This is the core invariant of S30.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — remove `AgentTickDriver::new()` in `from_simulation_state()`, accept restored driver)
- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — only if `save_load_roundtrip` signature changes require call-site updates)

## Out of Scope

- Changing `EXHAUSTION_SKIP_TTL` value (S30-007)
- Any AI decision logic changes
- Any save format or trait changes (S30-003)
- Adding new golden tests beyond verifying existing ones pass

## Acceptance Criteria

### Tests That Must Pass

1. `golden_save_load_round_trip_under_ai` passes WITHOUT `AgentTickDriver::new()` reset
2. `golden_save_load_preserves_promoted_commitments` continues passing
3. All golden replay companion tests pass: `cargo test -p worldwake-ai golden`
4. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. Save/load parity: uninterrupted and resumed runs produce identical world hashes at every post-save tick (Principle 11)
2. No driver reset at save/load boundary — runtime state is preserved
3. `post_load_validate()` is called after restore (guaranteed by S30-005)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — remove workaround in `from_simulation_state()`
2. No new tests — existing `golden_save_load_round_trip_under_ai` IS the acceptance test

### Commands

1. `cargo test -p worldwake-ai golden_save_load`
2. `cargo test -p worldwake-ai golden`
3. `cargo clippy --workspace && cargo test --workspace`
