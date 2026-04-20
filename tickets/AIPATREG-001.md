# AIPATREG-001: Reassess and fix `degenerate_zero_step_loop_blocks_actionable_goals`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — likely `worldwake-ai` planner/runtime behavior and/or the golden's late-run traceability contract
**Deps**: None

## Problem

Broader verification during `AIDECREG-002` still fails in `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` with `expected decision traces throughout the late-run pathology window, saw 79`. This blocks honest same-crate verification and leaves the active planner-pathology surface inconsistent with archived `S91PLAPATGOL-002` and active spec claims that `golden_planner_pathology.rs` passes.

## Assumption Reassessment (2026-04-20)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`; this is not a broad-suite artifact.
2. The failing assertion is currently at `crates/worldwake-ai/tests/golden_planner_pathology.rs:885` and fires before the zero-step-loop behavior assertions. The immediate missing proof is late-run decision-trace coverage (`window_traces >= 80`) during the observation window.
3. Archived `archive/tickets/S91PLAPATGOL-002.md` introduced this golden as a reproduction guard for the Forager Lina `FreeCarryCapacity` zero-step planner pathology. That archived handoff claims the scenario passed and that the golden inventory/docs were refreshed at landing time.
4. Shared abstraction boundary under audit: late-run planner-pathology scenario setup in `golden_planner_pathology.rs`, decision-trace capture/emission across the observation window, and the live `GoalKind::FreeCarryCapacity` planning/runtime path the golden is meant to guard.
5. The live contradiction is not yet classified. The isolated rerun only proves that the current scenario no longer yields the expected number of planning traces in the late-run window; it does not yet prove whether the real regression is trace collection, scenario timing/window assumptions, or the underlying `FreeCarryCapacity` pathology/repair contract.
6. Active spec drift is already visible: `specs/S115-agenda-manager.md` and `specs/S113-belief-envelope.md` still claim existing `golden_planner_pathology.rs` coverage passes. This ticket should either restore that truth or update the owning active spec/ticket surfaces factually during implementation.
7. Intended layer is mixed: focused golden-pathology coverage plus the strongest lower-layer runtime/trace proof needed to distinguish “trace window drift” from “planner behavior regression.”

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than folding this unrelated planner-pathology failure into `AIDECREG-002` or treating the failing golden as ambient suite noise.
2. The fix should target the earliest honest contradiction after reassessment: either the golden's late-run proof seam if the scenario window drifted, or the underlying planner/runtime behavior if the zero-step pathology or its repair regressed.

## Verification Layers

1. Late-run observation window still captures the intended planner ticks -> focused golden-pathology trace proof
2. `FreeCarryCapacity` no longer collapses into unlawful zero-step dominance, or the golden is corrected to the strongest honest live seam if that contract changed -> focused runtime/decision-trace proof
3. Same-crate planner-pathology coverage is green again -> `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`

## What to Change

### 1. Reassess the failing pathology golden against live code

- Name the exact late-run trace carrier, observation-window assumptions, and `FreeCarryCapacity` planning/runtime symbols under audit.
- Determine whether the missing decision traces are caused by stale windowing assumptions, trace-surface drift, or a real planner/runtime regression.

### 2. Land the smallest honest fix

- If the underlying planner pathology guard regressed, fix the production path narrowly and add/retain the strongest focused lower-layer proof.
- If the golden's observation window or trace seam is stale, rewrite the test and any owning active spec claims to the strongest honest live proof surface.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `crates/worldwake-ai/src/...` (modify only if reassessment proves a real runtime/planner regression)
- `specs/S115-agenda-manager.md` (modify if active spec drift remains after the fix)
- `specs/S113-belief-envelope.md` (modify if active spec drift remains after the fix)

## Out of Scope

- Unrelated `golden_ai_decisions` scenarios
- Broad cleanup of archived `S91PLAPATGOL-*` tickets beyond factual dependency/spec alignment
- New planner-pathology scenarios unrelated to the late-run Lina `FreeCarryCapacity` guard

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
2. Any new focused regression added at the true planner/trace boundary
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves an honest planner-pathology contract for the late-run Lina scenario rather than weakening the proof to hide missing planner traces or behavior.
2. The owning golden proves either the real `FreeCarryCapacity` recovery behavior or the strongest honest live seam if the original trace window is no longer lawful.
3. Active spec references to `golden_planner_pathology.rs` do not remain factually false after the fix lands.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs::degenerate_zero_step_loop_blocks_actionable_goals` — repaired late-run planner-pathology guard with honest proof surface
2. Focused lower-layer regression at the true runtime/trace boundary — exact test to be chosen during reassessment if production code changes

### Commands

1. `cargo test -p worldwake-ai --test golden_planner_pathology degenerate_zero_step_loop_blocks_actionable_goals`
2. `<exact focused regression command added during implementation if production code changes>`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
