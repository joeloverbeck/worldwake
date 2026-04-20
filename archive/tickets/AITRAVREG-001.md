# AITRAVREG-001: Reassess and fix the stale travel-bladder interruption golden

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No — reassessment proved a stale golden proof seam, not a live production regression
**Deps**: None

## Problem

Broader verification during `AIPATREG-001` still fails in `crates/worldwake-ai/tests/golden_travel_physiology.rs::golden_travel_interrupt_from_bladder_escalation` with `CriticalSurvival interrupt should have fired during active travel`. Reassessment now shows that expectation is stale: the live interrupt policy deliberately refuses to rotate `InterruptibleWithPenalty` actions between self-care goal families mid-action, even when the challenger becomes critical. This blocks honest same-crate verification because the golden still asserts an obsolete branch.

## Assumption Reassessment (2026-04-20)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai --test golden_travel_physiology golden_travel_interrupt_from_bladder_escalation`; this is not a broad-suite artifact.
2. The failing assertion currently lives in `crates/worldwake-ai/tests/golden_travel_physiology.rs:653` and claims that a `CriticalSurvival` interrupt must fire during active travel.
3. Archived `archive/tickets/completed/E20COMBEH-006A.md` explicitly introduced this exact golden as the replacement for the earlier mis-scoped local-relief test. That archived handoff claimed the scenario proved active travel interruption, abort of the `InterruptibleWithPenalty` travel action, replan to `Relieve`, and relief commit.
4. Shared abstraction boundary under audit: travel-physiology scenario setup in `golden_travel_physiology.rs`, interrupt decision tracing for active travel, and the live travel / bladder-escalation / `GoalKind::Relieve` path the golden is meant to guard.
5. The isolated rerun plus decision-trace inspection now classify the contradiction. During active `travel` ticks, the top challenger is already `GoalKind::Relieve` at `GoalPriorityClass::Critical`, but `evaluate_interrupt()` still returns `NoInterrupt`.
6. This is lawful live behavior, not a regression. `Interruptibility::InterruptibleWithPenalty` uses `penalty_interrupt_trigger()` on both the challenger and the effective active goal; when both are in the `CriticalSurvival` self-care family, `interrupts.rs` intentionally returns `NoInterrupt` to avoid rotating between critical self-care goals mid-action.
7. The scenario therefore never reaches the drafted failure boundary of “mid-travel interrupt.” The first honest live seam is: bladder escalation makes `Relieve` the critical top challenger during travel, the current travel leg completes without interrupt, and the agent switches to `Relieve` on the next planning seam.

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than folding this unrelated travel-physiology golden into `AIPATREG-001` or treating it as ambient same-crate noise.
2. The fix should target the earliest honest contradiction after reassessment. Here that seam is the planning boundary between short travel legs, not an active-action interrupt.

## Verification Layers

1. Scenario still reaches the intended bladder-escalation window during travel -> focused golden trace proof
2. The live policy is proved honestly: `Relieve` becomes the critical challenger during active travel, no mid-leg interrupt occurs between self-care goals, and the switch to `Relieve` happens on the next planning seam -> focused golden trace proof
3. Same-crate travel-physiology coverage is green again -> `cargo test -p worldwake-ai --test golden_travel_physiology`
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`

## What to Change

### 1. Rewrite the stale travel-physiology golden to the live contract

- Name the exact active-travel policy and symbols under audit: `evaluate_interrupt()`, `Interruptibility::InterruptibleWithPenalty`, `penalty_interrupt_trigger()`, and the `GoalKind::Relieve` challenger during travel.
- Replace the obsolete “mid-travel interrupt” proof seam with the first reachable live seam: critical `Relieve` challenger during travel, no mid-leg interrupt, and switch/recovery at the next planning boundary.

### 2. Land the smallest honest fix

- Rewrite the golden name, scenario prose, and assertions to the strongest honest live proof surface.
- Do not change production interrupt behavior unless new evidence contradicts the live self-care rotation guard.

## Files to Touch

- `crates/worldwake-ai/tests/golden_travel_physiology.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (modify if this repo keeps generated test names in sync manually)
- `docs/generated/golden-scenario-index.md` (modify if this repo keeps generated scenario prose in sync manually)
- `docs/generated/golden-scenario-details/travel-physiology.md` (modify if this repo keeps generated scenario prose in sync manually)

## Out of Scope

- Unrelated planner-pathology or `golden_ai_decisions` scenarios
- Broad cleanup of archived `E20COMBEH-*` tickets beyond factual reassessment and proof-surface alignment
- Any production interrupt-policy redesign that would re-enable mid-action rotation between self-care goal families

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_travel_physiology`
2. No new lower-layer regression is required unless production code changes during implementation
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix does not silently weaken the proof; it explicitly records that mid-action interruption is no longer the live contract for competing self-care goals.
2. The owning golden proves the strongest honest live seam: bladder escalation reaches a critical `Relieve` challenger during travel, no mid-leg self-care rotation occurs, and the agent switches to `Relieve` on the next planning seam.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_travel_physiology.rs` — renamed and rewritten scenario proving the live between-legs relief switch seam
2. No additional lower-layer regression unless production code changes

### Commands

1. `cargo test -p worldwake-ai --test golden_travel_physiology`
2. `<focused lower-layer regression command only if production code changes>`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-04-20
- **What changed**: Reassessment proved the original golden was asserting an obsolete branch. The live self-care interrupt policy intentionally does not rotate an `InterruptibleWithPenalty` action mid-leg when both the running goal and the challenger are in the `CriticalSurvival` family. The scenario was renamed to `golden_travel_bladder_escalation_switches_to_relief_between_legs`, its prose and assertions were rewritten to the honest live seam, and the generated golden inventory/details files were updated to match the new scenario name and contract.
- **Deviations from original plan**: No production interrupt behavior changed. The ticket narrowed from a possible mixed production/test fix to a test-only reassessment because the drafted “mid-travel interrupt” branch is no longer the lawful live contract.
- **Verification results**:
  1. `cargo test -p worldwake-ai --test golden_travel_physiology`
  2. `cargo test -p worldwake-ai`
  3. `cargo clippy --workspace --all-targets -- -D warnings`

## Verification Result

1. `cargo test -p worldwake-ai --test golden_travel_physiology`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
