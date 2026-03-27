# S30-007: Raise the exhaustion skip window from 16 to 20 ticks

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — planner exhaustion skip policy in `planning.rs`, focused planner tests
**Deps**: archive/tickets/S30-006-remove-driver-reset-workaround.md, specs/S30-ai-runtime-save-load-parity.md, specs/S31-goal-aware-exhaustion-invalidation.md

## Problem

The live planner still uses a coarse time-based skip window for recently exhausted goals in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). The runtime save/load parity work from S30 is already landed, but the planner still carries the pre-parity conservative skip window of `16` ticks. That value causes the planner to re-search known-unsatisfied goals more often than necessary even though the exhaustion cache now survives save/load boundaries.

The clean long-term architecture is still S31's condition-based invalidation, not a larger TTL. But until S31 lands, the current architecture should use the strongest value we can justify with live code and tests, without inventing new heuristics or retaining historical workarounds.

## Assumption Reassessment (2026-03-27)

1. The live abstraction boundary under audit is the planner-local exhaustion cache contract in [`AgentDecisionRuntime::exhaustion_cache`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) using [`ExhaustionEntry { exhausted_at, count }`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), plus its consumers [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), [`reset_exhausted_goals_if_needed()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), and [`record_exhausted_goals()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs).
2. `EXHAUSTION_SKIP_TTL` still exists only as a local constant in [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and is currently `16`.
3. The save/load parity dependency is real and already implemented. The runtime save path now persists the exhaustion cache through [`SaveableRuntime::save_runtime_state()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs), restores it through [`AgentTickDriver::from_saved_runtime(...)`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs), and the save format is already version `6` in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs).
4. The prior ticket narrative is stale in two ways:
5. First mismatch: the repo no longer has an in-tree "golden-perf harness" or committed experiment runner that can justify an "optimal" search over `32`, `64`, and indefinite TTLs. The active spec and ticket references still mention historical experiments (`exp-005`, `exp-016`), but those are not a present verification surface.
6. Second mismatch: the old ticket claimed no new or modified tests were needed. That is too weak for a planner-runtime policy change. The live code has focused exhaustion-cache tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), and this ticket should extend that focused proof surface rather than rely only on broad goldens.
7. This is a planner ticket, but not a root-synthesis ticket. No single `GoalKind` family or operator surface is being changed. The behavior under audit is the generic pre-search candidate filter in `build_candidate_plans()` that applies before goal-specific operator expansion. Focused tests may use `GoalKind::ConsumeOwnedCommodity` only as a stable fixture key, not as a claim about that goal family specifically.
8. Save/load coverage is still relevant because the historical reason for the conservative TTL was save/load divergence. The live tests `golden_save_load_round_trip_under_ai` in [`crates/worldwake-ai/tests/golden_determinism.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) and `save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation` in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) remain the right regression boundary.
9. S31 remains the architecturally superior end-state. [`specs/S31-goal-aware-exhaustion-invalidation.md`](/home/joeloverbeck/projects/worldwake/specs/S31-goal-aware-exhaustion-invalidation.md) still plans to remove `EXHAUSTION_SKIP_TTL` entirely in favor of goal-aware invalidation. This ticket must not expand into that redesign.
10. Additional live mismatch discovered during implementation: `32` is still not lawful today. Re-running `golden_save_load_round_trip_under_ai` with raised TTL values shows `21+` reintroduces save/load divergence, while `20` still passes. The safe ceiling currently demonstrated by live tests is `20`, not `32`.
11. Corrected scope: implement the strongest evidence-backed improvement now by raising the skip window from `16` to `20`, prove the skip-window boundary with focused tests, and explicitly defer `21+` or TTL removal to S31 or a future benchmark-backed ticket.

## Architecture Check

1. Raising the current TTL from `16` to `20` is cleaner than keeping the stale conservative value because it recovers a bounded planner-efficiency benefit without changing the runtime shape, adding alias paths, or creating new persistence semantics.
2. Raising the TTL beyond `20` would not be a clean architectural improvement in the current codebase because live determinism coverage already shows `21+` breaks the save/load parity contract. S31 still names the better architecture: remove the time-based heuristic and invalidate by concrete goal conditions.
3. Jumping straight to S31 from this ticket would broaden scope from a small planner policy correction into a structural invalidation redesign. That is a different ticket with a different proof surface. The clean move here is a narrow correction now, then the real architectural replacement in S31.

## Verification Layers

1. Exhausted goals stay skipped for the intended active window and become searchable again at the boundary -> focused planner unit tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
2. Exhaustion cache serialization and restore still preserve runtime state -> focused runtime tests in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
3. Save/load parity still holds across the live golden determinism boundary -> [`golden_save_load_round_trip_under_ai`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs)
4. Broader planner and workspace regressions remain absent -> `worldwake-ai` package tests, workspace tests, and workspace clippy

## What to Change

### 1. Raise the planner exhaustion skip window to 20 ticks

Change the live planner skip window in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) from `16` to `20`.

### 2. Make the skip-window contract directly testable

Expose the TTL boundary in a focused planner-local way so unit tests can prove:

- entries with `exhausted_at: Some(tick)` stay skipped for ticks `< 20`
- entries become searchable again exactly at the `20`-tick boundary
- entries with `exhausted_at: None` still keep cumulative backoff history without remaining inside the active skip window

This can be done with a small helper or equivalent local refactor in `planning.rs`. Do not broaden the runtime model.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — raise skip window and add focused boundary tests)

## Out of Scope

- Determining a supposedly optimal TTL by sweeping `64+` or indefinite values
- Any new benchmarking harness or experiment infrastructure
- S31 goal-aware invalidation design or removing `EXHAUSTION_SKIP_TTL` entirely
- Save format changes or runtime persistence refactors
- Goal-specific root-synthesis, candidate-generation, or ranking changes

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner tests prove the `20`-tick skip boundary.
2. `golden_save_load_round_trip_under_ai` passes with the new TTL.
3. `cargo test -p worldwake-ai` passes.
4. `cargo clippy --workspace` passes.
5. `cargo test --workspace` passes.

### Invariants

1. Save/load boundaries preserve the exhaustion cache strongly enough that the longer skip window does not reintroduce resumed-vs-uninterrupted divergence.
2. The exhaustion cache still separates active skip state (`exhausted_at`) from cumulative backoff history (`count`).
3. No compatibility alias path or duplicate exhaustion mechanism is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — add focused unit coverage for the active skip-window boundary and for `count`-only entries outside the active TTL window.
2. `crates/worldwake-ai/tests/golden_determinism.rs` — keep the save/load parity golden passing with the longer skip window.
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — keep existing runtime persistence coverage passing to prove the exhaustion cache still round-trips.

### Commands

1. `cargo test -p worldwake-ai exhausted_goal -- --nocapture`
2. `cargo test -p worldwake-ai golden_save_load_round_trip_under_ai -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed: raised `EXHAUSTION_SKIP_TTL` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) from `16` to `20`; extracted a planner-local helper so the active skip-window contract is directly testable; added focused tests for the active TTL boundary and for count-only entries outside the active skip window.
- Deviations from original plan: the original ticket and spec-adjacent narrative claimed `32+` should now be safe after save/load parity. Live verification disproved that. `golden_save_load_round_trip_under_ai` still fails at `21+`, so the final delivered value is `20`, the highest value proven safe by the live determinism boundary during implementation.
- Verification results: `cargo test -p worldwake-ai exhausted_goal -- --nocapture`, `cargo test -p worldwake-ai golden_save_load_round_trip_under_ai -- --nocapture`, `cargo test -p worldwake-ai`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
