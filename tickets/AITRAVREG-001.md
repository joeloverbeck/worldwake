# AITRAVREG-001: Reassess and fix `golden_travel_interrupt_from_bladder_escalation`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — likely `worldwake-ai` interrupt/travel behavior and/or the golden's active-travel proof seam
**Deps**: None

## Problem

Broader verification during `AIPATREG-001` still fails in `crates/worldwake-ai/tests/golden_travel_physiology.rs::golden_travel_interrupt_from_bladder_escalation` with `CriticalSurvival interrupt should have fired during active travel`. This blocks honest same-crate verification and leaves the travel-physiology golden surface inconsistent with archived `E20COMBEH-006A`, which claimed this scenario proved the mid-travel bladder-escalation interrupt contract.

## Assumption Reassessment (2026-04-20)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai --test golden_travel_physiology golden_travel_interrupt_from_bladder_escalation`; this is not a broad-suite artifact.
2. The failing assertion currently lives in `crates/worldwake-ai/tests/golden_travel_physiology.rs:653` and claims that a `CriticalSurvival` interrupt must fire during active travel.
3. Archived `archive/tickets/completed/E20COMBEH-006A.md` explicitly introduced this exact golden as the replacement for the earlier mis-scoped local-relief test. That archived handoff claimed the scenario proved active travel interruption, abort of the `InterruptibleWithPenalty` travel action, replan to `Relieve`, and relief commit.
4. Shared abstraction boundary under audit: travel-physiology scenario setup in `golden_travel_physiology.rs`, interrupt decision tracing for active travel, and the live travel / bladder-escalation / `GoalKind::Relieve` path the golden is meant to guard.
5. The live contradiction is not yet classified. The isolated rerun proves the scenario no longer observes the expected `CriticalSurvival` interrupt, but it does not yet distinguish stale scenario setup/proof seam from a real regression in interrupt generation or travel physiology.
6. Intended layer is mixed: focused golden-pathology coverage plus the strongest lower-layer interrupt/runtime proof needed to distinguish “stale travel scenario seam” from “real interrupt regression.”

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than folding this unrelated travel-interrupt golden into `AIPATREG-001` or treating it as ambient same-crate noise.
2. The fix should target the earliest honest contradiction after reassessment: either the golden's active-travel proof seam if the scenario no longer lawfully holds the travel interrupt window, or the underlying interrupt/travel behavior if the contract really regressed.

## Verification Layers

1. Scenario still reaches the intended active-travel bladder-escalation window -> focused golden trace/runtime proof
2. `CriticalSurvival` interrupt and post-abort recovery still occur, or the golden is corrected to the strongest honest live seam if that contract changed -> focused interrupt/runtime proof
3. Same-crate travel-physiology coverage is green again -> `cargo test -p worldwake-ai --test golden_travel_physiology golden_travel_interrupt_from_bladder_escalation`
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`

## What to Change

### 1. Reassess the failing travel-physiology golden against live code

- Name the exact active-travel interrupt carrier, bladder-escalation assumptions, and `GoalKind::Relieve` / travel interrupt symbols under audit.
- Determine whether the missing `CriticalSurvival` interrupt is caused by stale scenario setup/window assumptions, trace-surface drift, or a real runtime regression.

### 2. Land the smallest honest fix

- If the underlying interrupt contract regressed, fix the production path narrowly and add/retain the strongest focused lower-layer proof.
- If the golden's scenario seam is stale, rewrite the test and any owning active spec claims to the strongest honest live proof surface.

## Files to Touch

- `crates/worldwake-ai/tests/golden_travel_physiology.rs` (modify)
- `crates/worldwake-ai/src/...` (modify only if reassessment proves a real interrupt/travel regression)

## Out of Scope

- Unrelated planner-pathology or `golden_ai_decisions` scenarios
- Broad cleanup of archived `E20COMBEH-*` tickets beyond factual reassessment and proof-surface alignment
- New travel-physiology scenarios unrelated to the active-travel bladder-escalation interrupt guard

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_travel_physiology golden_travel_interrupt_from_bladder_escalation`
2. Any new focused regression added at the true interrupt/runtime boundary
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves an honest active-travel interrupt contract rather than weakening the proof to hide a missing interrupt window or runtime behavior.
2. The owning golden proves either the real `CriticalSurvival`-during-travel recovery behavior or the strongest honest live seam if the original window is no longer lawful.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_travel_physiology.rs::golden_travel_interrupt_from_bladder_escalation` — repaired active-travel interrupt golden with honest proof surface
2. Focused lower-layer regression at the true interrupt/runtime boundary — exact test to be chosen during reassessment if production code changes

### Commands

1. `cargo test -p worldwake-ai --test golden_travel_physiology golden_travel_interrupt_from_bladder_escalation`
2. `<exact focused regression command added during implementation if production code changes>`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
