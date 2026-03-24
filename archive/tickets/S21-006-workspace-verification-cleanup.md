# S21-006: Workspace verification and cleanup

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — cleanup and verification only
**Deps**: S21-001, S21-002, S21-003, S21-004, S21-005

## Problem

After all migration tickets are complete, verify the entire workspace is clean: no stale re-exports, no new warnings, all invariant tests pass, and deterministic replay is unaffected.

## Assumption Reassessment (2026-03-23)

1. S21-001 may add temporary re-exports of `JourneyCommitmentState` and `QueuedFacilityIntent` in `decision_runtime.rs` to keep the AI crate compiling during incremental migration. After S21-002 and S21-004 complete, these re-exports may be unnecessary if all imports have been updated to reference `worldwake_core` directly.
2. `agent_decision_runtime_is_not_registered_as_a_component` test exists — grep confirms this. `AgentDecisionRuntime` must remain unregistered.
3. `golden_deterministic_replay_fidelity` test exists in `golden_determinism.rs` — must pass with unchanged hashes.
4. `load_rejects_wrong_version` test in `save_load.rs` uses relative `SAVE_FORMAT_VERSION + 1` — should pass with v5.
5. `save_to_bytes_roundtrip_preserves_full_nondefault_state` exercises `ComponentTables` serialization — should pass with new components.
6. `SAVE_FORMAT_VERSION` should be `5` after S21-001.

## Architecture Check

1. This is a cleanup and verification pass — no architectural decisions.
2. Explicitly removes any backward-compatibility re-exports added during incremental migration.

## Verification Layers

1. Full workspace test suite → `cargo test --workspace`
2. No new clippy warnings → `cargo clippy --workspace`
3. Deterministic replay → `golden_deterministic_replay_fidelity` passes
4. Save format version → `SAVE_FORMAT_VERSION == 5`
5. Runtime not registered → `agent_decision_runtime_is_not_registered_as_a_component` passes
6. No stale re-exports → grep for `pub use worldwake_core::JourneyCommitmentState` in AI crate returns zero hits (if imports were updated), or re-exports are confirmed still needed

## What to Change

### 1. Remove temporary re-exports (if unused)

In `crates/worldwake-ai/src/decision_runtime.rs`, check if `pub use worldwake_core::{JourneyCommitmentState, QueuedFacilityIntent};` re-exports are still imported by any module. If all AI code now imports from `worldwake_core` directly, remove the re-exports.

### 2. Grep sweep for stale references

- `runtime.current_goal` → zero hits
- `runtime.journey_committed` → zero hits
- `runtime.queued_facility_intents` → zero hits
- `runtime.journey_commitment_state` → zero hits
- `runtime.journey_established_at` → zero hits
- `runtime.journey_last_progress_tick` → zero hits
- `runtime.consecutive_blocked_leg_ticks` → zero hits

### 3. Run full verification suite

- `cargo test --workspace`
- `cargo clippy --workspace`

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove stale re-exports if any)
- No other file changes expected unless stale imports are found

## Out of Scope

- Any functional changes to components, AI code, or test logic
- Adding new tests (S21-005 handles that)
- Modifying component definitions (S21-001)
- Any changes outside `worldwake-ai` crate

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all pass (zero failures)
2. `cargo clippy --workspace` — zero new warnings
3. `cargo test -p worldwake-ai --test golden_determinism -- golden_deterministic_replay_fidelity` — deterministic replay hashes unchanged
4. `cargo test -p worldwake-sim -- load_rejects_wrong_version` — passes with `SAVE_FORMAT_VERSION == 5`
5. `cargo test -p worldwake-sim -- save_to_bytes_roundtrip` — passes with new components in `ComponentTables`

### Invariants

1. `SAVE_FORMAT_VERSION == 5` in `crates/worldwake-sim/src/save_load.rs`
2. `AgentDecisionRuntime` is NOT registered as a component (existing test passes)
3. `AgentDecisionRuntime` has zero promoted fields (only ephemeral/diagnostic fields remain)
4. No stale re-exports of moved types in `worldwake-ai`
5. All golden test hashes unchanged from pre-S21 baseline

## Test Plan

### New/Modified Tests

1. None — verification-only ticket; all tests already exist from S21-001 through S21-005.

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. Grep sweep: `grep -r "runtime\.current_goal\|runtime\.journey_committed\|runtime\.queued_facility_intents\|runtime\.journey_commitment_state\|runtime\.journey_established_at\|runtime\.journey_last_progress_tick\|runtime\.consecutive_blocked_leg_ticks" crates/worldwake-ai/src/` — must return zero hits

## Outcome

- **Completion date**: 2026-03-24
- **What changed**: Removed the `pub use worldwake_core::JourneyCommitmentState;` re-export from `decision_runtime.rs` and the corresponding `JourneyCommitmentState` entry from `lib.rs` re-exports. Updated 4 files (`decision_runtime.rs`, `agent_tick/active_action.rs`, `agent_tick/journey.rs`, `tests/golden_ai_decisions.rs`) to import `JourneyCommitmentState` from `worldwake_core` directly. No `QueuedFacilityIntent` re-export existed (already clean from S21-004).
- **Deviations from plan**: None. The ticket anticipated the re-export might not exist; one of the two (`JourneyCommitmentState`) was still present and was removed. The other (`QueuedFacilityIntent`) was already clean.
- **Verification results**: `cargo test --workspace` — all pass (zero failures). `cargo clippy --workspace` — zero warnings. `SAVE_FORMAT_VERSION == 5`. Stale runtime field grep sweep — zero hits. All golden/determinism/save-load tests pass.
