# S31-005: Update `record_exhausted_goals` to Capture Conditions and Baseline

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planning pipeline
**Deps**: S31-001, S31-002

## Problem

S31-005 was drafted assuming `record_exhausted_goals` still only refreshed `exhausted_at` and `count`. Live code has already implemented goal-aware exhaustion recording, but the ticket was never updated to reflect the delivered architecture, the current test surface, or the remaining verification-and-archive work.

## Assumption Reassessment (2026-03-27)

Shared abstraction boundary under audit: `AgentDecisionRuntime.exhaustion_cache` persisted shape and the `GoalBeliefView`-based invalidation contract that feeds planner skip/backoff behavior.

1. Live `record_exhausted_goals` already has the extended signature in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and is called with `view`, `agent`, and `recipe_registry`.
2. Live `record_exhausted_goals` already calls `derive_invalidation_conditions(&key.kind, agent, view, recipe_registry)` and stores both `invalidation_conditions` and `baseline` on insert and refresh.
3. Live refresh semantics already preserve `count` on re-exhaustion while updating `exhausted_at`, `invalidation_conditions`, and `baseline`. This matches the cleaner backoff contract: `count` tracks invalidation-cycle history, not repeated refreshes inside one stale world state.
4. Live `ExhaustionEntry` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) no longer derives `Copy`, and the new fields are persisted directly.
5. The ticket’s original persistence assumption was stale: the live save format is `SAVE_FORMAT_VERSION = 7` in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs), and the live `ExhaustionEntry` intentionally removed the older `#[serde(default)]` compatibility path on the new fields. That strict contract is cleaner than carrying an empty-condition alias path.
6. Focused tests already exist in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs), and save/load coverage in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs).
7. The remaining work is verification, ticket cleanup, and archival, not fresh planner implementation.

## Architecture Check

1. The landed design is better than the old architecture. Recording per-goal invalidation conditions and a baseline snapshot inside the persisted exhaustion entry replaces the coarse dirty-bit/TTL-only behavior with goal-local evidence about when a retry is justified.
2. The strict persistence contract is also the cleaner architecture. Removing `#[serde(default)]` from the new fields avoids a backward-compatibility alias path that would otherwise let "exhausted entry with no conditions" exist indefinitely in a format-7 world.
3. The current split is acceptable: `planning.rs` decides when to record exhaustion, while `exhaustion.rs` owns derivation and invalidation rules. A heavier refactor to merge those concerns would not buy enough architectural clarity to justify churn here.
4. One spec-level mismatch remains worth calling out: `specs/S31-goal-aware-exhaustion-invalidation.md` still describes a compatibility fallback for old entries. The code and this ticket take the cleaner route instead. That divergence should be treated as intentional cleanup, not as a regression.

## Verification Layers

1. Exhausted goal entries store goal-local invalidation conditions and baselines -> `agent_tick::planning` focused unit tests
2. Goal-kind coverage and per-condition delta semantics stay deterministic -> `exhaustion` focused unit tests
3. Persisted runtime shape round-trips populated exhaustion entries under save format 7 -> `agent_tick` save/load tests
4. Post-load validation prunes stale runtime references without preserving empty-condition alias entries -> `agent_tick` and golden harness save/load tests
5. AI planner behavior remains green under the live planner/golden suite -> `cargo test -p worldwake-ai`
6. Workspace-wide lint and regression surface remain green after ticket finalization -> `cargo clippy --workspace`, `cargo test --workspace`

## Updated Scope

### 1. Verify the delivered architecture against the ticket narrative

Confirm that the live implementation in `planning.rs`, `exhaustion.rs`, and `decision_runtime.rs` matches the intended S31 contract:
- exhausted goals record invalidation conditions and baseline snapshots
- refreshed exhausted goals preserve `count`
- non-exhausted searched goals are removed from the cache
- persisted runtime requires explicit conditions/baseline in the format-7 shape

### 2. Verify the save/load contract instead of reintroducing compatibility

Do not restore `#[serde(default)]` or any equivalent fallback for `ExhaustionEntry`. If verification uncovers a surviving alias path, remove it rather than preserving it.

### 3. Finalize the ticket and archive it if verification passes

If focused and full verification pass, mark the ticket complete, record the actual outcome, and move it under `archive/tickets/`.

## Files to Touch

- `tickets/S31-005-record-exhausted-with-conditions.md` (modify — reassessment, corrected scope, completion metadata)
- No code changes should be made for this ticket unless verification reveals a real mismatch between the ticket and the already-landed implementation

## Out of Scope

- Further refactors to merge `record_exhausted_goals` into `exhaustion.rs`
- TTL removal and skip-predicate redesign (`S31-006`)
- Additional golden-scenario expansion beyond proving the live planner/save-load surfaces remain green

## Acceptance Criteria

### Tests That Must Pass

1. `agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline`
2. `agent_tick::planning::tests::record_exhausted_goals_refreshes_tick_without_resetting_count`
3. `agent_tick::planning::tests::record_exhausted_goals_removes_only_successful_goal_entry`
4. Relevant `exhaustion::tests::*` focused invalidation-condition coverage
5. `agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
6. `agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
7. `agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
8. `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
9. Workspace verification: `cargo clippy --workspace` and `cargo test --workspace`

### Invariants

1. Every newly recorded exhausted entry carries explicit invalidation conditions plus a baseline snapshot
2. Baseline data is derived from the `GoalBeliefView` boundary rather than from authoritative-world shortcuts
3. Re-refreshing an already exhausted entry updates the world-dependent snapshot without resetting backoff history
4. A format-7 runtime does not preserve an empty-condition compatibility alias path for new exhaustion entries

## Test Plan

### New/Modified Tests

1. `agent_tick::planning::tests::record_exhausted_goals_refreshes_tick_without_resetting_count`
Rationale: proves re-exhaustion refreshes `exhausted_at`, conditions, and baseline without rewriting backoff history.
2. `agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline`
Rationale: proves exhausted entries persist concrete goal-local invalidation data instead of only a TTL marker.
3. `agent_tick::planning::tests::record_exhausted_goals_removes_only_successful_goal_entry`
Rationale: proves searched non-exhausted goals clear their own cache entry without disturbing unrelated exhaustion history.
4. `exhaustion::tests::derive_invalidation_conditions_*` and `exhaustion::tests::condition_changed_*`
Rationale: prove per-goal condition derivation and invalidation deltas at the strongest lower layer, independent of planner orchestration.
5. `agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
Rationale: proves populated `ExhaustionEntry` values serialize in the live runtime contract.
6. `agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
Rationale: proves format-7 runtime restore keeps valid populated entries intact.
7. `agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
Rationale: proves stale references are removed at post-load validation instead of being masked by compatibility defaults.
8. `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
Rationale: proves the same runtime contract survives real save/load integration.

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline`
2. `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_refreshes_tick_without_resetting_count`
3. `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_removes_only_successful_goal_entry`
4. `cargo test -p worldwake-ai exhaustion::tests::derive_invalidation_conditions_covers_every_live_goalkind_variant`
5. `cargo test -p worldwake-ai exhaustion::tests::invalidate_exhausted_goals_removes_only_entries_with_fired_conditions`
6. `cargo test -p worldwake-ai agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
7. `cargo test -p worldwake-ai agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
8. `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
9. `cargo test -p worldwake-ai`
10. `cargo clippy --workspace`
11. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed: the ticket was corrected to match the live codebase. Reassessment confirmed that `record_exhausted_goals`, `derive_invalidation_conditions`, runtime persistence, and focused save/load coverage were already implemented. No production code changes were required.
- Deviations from original plan: the original ticket described planned planner/runtime changes that had already landed. The completed work for this ticket was verification, scope correction, and archival preparation. The codebase also intentionally diverges from the older spec's backward-compatibility fallback by requiring explicit exhaustion-condition data in the format-7 runtime shape.
- Verification results:
  - `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_derives_goal_aware_conditions_and_baseline`
  - `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_refreshes_tick_without_resetting_count`
  - `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_removes_only_successful_goal_entry`
  - `cargo test -p worldwake-ai exhaustion::tests::derive_invalidation_conditions_covers_every_live_goalkind_variant`
  - `cargo test -p worldwake-ai exhaustion::tests::invalidate_exhausted_goals_removes_only_entries_with_fired_conditions`
  - `cargo test -p worldwake-ai agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
  - `cargo test -p worldwake-ai agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
  - `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
