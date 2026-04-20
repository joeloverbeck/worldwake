# AIPATREG-001: Reassess and fix `degenerate_zero_step_loop_blocks_actionable_goals`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — reassessment shows this is a stale late-run golden proof seam, not a new planner/runtime regression.
**Deps**: None

## Problem

Broader verification during `AIDECREG-002` still fails in `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` with `expected decision traces throughout the late-run pathology window, saw 79`. This blocks honest same-crate verification and leaves the active planner-pathology surface inconsistent with archived `S91PLAPATGOL-002` and active spec claims that `golden_planner_pathology.rs` passes.

## Assumption Reassessment (2026-04-20)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`; this is not a broad-suite artifact.
2. The failing assertion is currently at `crates/worldwake-ai/tests/golden_planner_pathology.rs:885` and fires before the zero-step-loop behavior assertions. The immediate missing proof is late-run decision-trace coverage (`window_traces >= 80`) during the observation window.
3. Archived `archive/tickets/S91PLAPATGOL-002.md` introduced this golden as a reproduction guard for the Forager Lina `FreeCarryCapacity` zero-step planner pathology. That archived handoff claims the scenario passed and that the golden inventory/docs were refreshed at landing time.
4. Shared abstraction boundary under audit: late-run planner-pathology scenario setup in `golden_planner_pathology.rs`, decision-trace capture/emission across the observation window, and the live `GoalKind::FreeCarryCapacity` planning/runtime path the golden is meant to guard.
5. The live contradiction is now classified. `DecisionOutcome` is one-per-agent-per-tick and can be either `Planning` or `ActiveAction` (`crates/worldwake-ai/src/decision_trace.rs`), while the scenario helper `planning_trace_at()` in `crates/worldwake-ai/tests/golden_planner_pathology.rs` explicitly discards `ActiveAction` ticks. The missing `window_traces` are therefore late-run execution ticks, not missing decision records.
6. Existing lower-layer `FreeCarryCapacity` fix coverage is still present in `crates/worldwake-ai/src/goal_model.rs` (`free_carry_capacity_is_not_satisfied_above_disposal_threshold`, `free_carry_capacity_requires_progress_below_threshold`, `free_carry_capacity_remains_unsatisfied_after_partial_progress_that_still_exceeds_threshold`, and nearby candidate-availability tests). Reassessment found no evidence that the underlying S92 planner/runtime contract regressed.
7. The honest live invariant for Scenario 143 remains: no repeated zero-step `FreeCarryCapacity` plans, lawful disposal or goal switching instead, and self-care recovery in the late-run window. The stale part is the stronger drafted seam `window_traces >= 80`, which incorrectly assumed planning-only traces on ticks where the fixed scenario is lawfully executing active actions.
8. Active spec drift is conditional rather than current fact. `specs/S115-agenda-manager.md` and `specs/S113-belief-envelope.md` only become stale if the repaired golden still cannot pass after the proof-seam correction. If the isolated golden returns green after the test rewrite, no active-spec update is needed.
9. Intended layer is golden-pathology coverage backed by the existing lower-layer `FreeCarryCapacity` proofs, not new production ownership.

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than folding this unrelated planner-pathology failure into `AIDECREG-002` or treating the failing golden as ambient suite noise.
2. The earliest honest contradiction is the golden's late-run proof seam: it overclaims planning-only trace continuity in a fixed scenario that now lawfully spends part of the late-run window executing active actions.

## Verification Layers

1. Late-run observation window still captures one decision outcome per tick, whether planning or active-action execution -> focused golden-pathology trace proof
2. `FreeCarryCapacity` no longer collapses into unlawful zero-step dominance, and lawful late-run execution ticks are accepted as part of recovery -> focused runtime/decision-trace proof
3. Same-crate planner-pathology coverage is green again -> `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`

## What to Change

### 1. Reassess the failing pathology golden against live code

- Name the exact late-run trace carrier, observation-window assumptions, and `FreeCarryCapacity` planning/runtime symbols under audit.
- Correct the ticket/golden to the strongest honest late-run seam now that reassessment shows the missing `window_traces` are filtered `ActiveAction` ticks rather than missing decision records.

### 2. Land the smallest honest fix

- Rewrite the Scenario 143 golden so it measures full late-run decision coverage and keeps the zero-step / disposal / self-care recovery assertions on the planning ticks that still exist.
- Do not change production code unless a narrower focused proof unexpectedly falsifies the current S92 fix contract during implementation.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `crates/worldwake-ai/src/...` (no changes expected after reassessment)
- `specs/S115-agenda-manager.md` (modify if active spec drift remains after the fix)
- `specs/S113-belief-envelope.md` (modify if active spec drift remains after the fix)

## Out of Scope

- Unrelated `golden_ai_decisions` scenarios
- Broad cleanup of archived `S91PLAPATGOL-*` tickets beyond factual dependency/spec alignment
- New planner-pathology scenarios unrelated to the late-run Lina `FreeCarryCapacity` guard

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
2. Existing focused `FreeCarryCapacity` lower-layer regressions continue to prove the S92 production contract
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves an honest planner-pathology contract for the late-run Lina scenario rather than weakening the proof to hide missing zero-step or recovery behavior.
2. The owning golden proves the real `FreeCarryCapacity` recovery behavior at the strongest honest live seam: mixed planning and active-action ticks across the observation window.
3. Active spec references to `golden_planner_pathology.rs` remain unchanged unless the repaired isolated golden still fails.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` — repaired late-run planner-pathology guard with honest mixed planning/execution proof surface
2. No new lower-layer regression expected; existing `FreeCarryCapacity` focused coverage in `crates/worldwake-ai/src/goal_model.rs` remains the production proof surface

### Commands

1. `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
2. `cargo test -p worldwake-ai --lib free_carry_capacity`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

1. Reassessed `degenerate_zero_step_loop_blocks_actionable_goals` against the live decision-trace contract and existing S92 lower-layer `FreeCarryCapacity` coverage.
2. Confirmed the failure was a stale golden proof seam: the test counted only `DecisionOutcome::Planning` ticks via `planning_trace_at()`, so lawful late-run `ActiveAction` execution ticks were incorrectly treated as missing traces.
3. Repaired Scenario 143 in `crates/worldwake-ai/tests/golden_planner_pathology.rs` to count one decision outcome per late-run tick, keep the zero-step and executable-disposal assertions on the planning subset, and preserve the eat/hunger recovery proof across the full window.

## Deviations

1. No production code changes landed. The honest fix was narrower than the drafted ticket's initial production-vs-trace ambiguity and stayed entirely on the golden proof surface.
2. Existing `FreeCarryCapacity` lower-layer coverage in `crates/worldwake-ai/src/goal_model.rs` and neighboring modules remained the production proof surface, so no new focused production regression was added.

## Verification Result

1. Passed: `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
2. Passed: `cargo test -p worldwake-ai --lib free_carry_capacity`
3. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
4. Broader same-crate rerun still fails in unrelated existing travel-physiology coverage: `crates/worldwake-ai/tests/golden_travel_physiology.rs::golden_travel_interrupt_from_bladder_escalation`
