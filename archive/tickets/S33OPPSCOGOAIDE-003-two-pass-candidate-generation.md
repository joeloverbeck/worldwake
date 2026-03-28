# S33OPPSCOGOAIDE-003: Two-pass candidate generation with per-opportunity blocker filtering

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation pipeline restructured
**Deps**: S33OPPSCOGOAIDE-002

## Problem

The original S33 split reserved blocker-scoped two-pass candidate generation for a follow-up ticket. After reassessment during S33OPPSCOGOAIDE-002, that work proved inseparable from the opportunity-emission refactor because leaving global blocker suppression in place would have preserved the same desire-level aliasing the refactor was meant to remove.

## Assumption Reassessment (2026-03-28)

1. The old global `is_blocked(&key, None, None, None, current_tick)` emission-time suppression path was removed during S33OPPSCOGOAIDE-002 in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
2. The live blocker-query boundary remains [`BlockedIntentMemory`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs), which already stores `{ goal_key, place, target, action_def }` scope and did not require schema changes.
3. The actual delivered behavior is slightly stronger than the original ticket narrative: blocker matching now uses `GroundedGoal.anchor` plus candidate evidence entities/places where anchor alone is too weak to represent the concrete blocked substrate.
4. This means the ticket’s original “separate pass to be added later” scope was already consumed by archived S33OPPSCOGOAIDE-002. Leaving this ticket active would duplicate delivered architecture.

## Architecture Check

1. Folding the blocker pass into the opportunity-emission refactor was cleaner than preserving a temporary mixed model. Opportunity identity without opportunity-scoped blocker matching would have violated P18 and P26 by keeping the old alias path alive.
2. No backward-compatibility shim was left behind. The emission-time global blocker path was removed rather than retained alongside the new filter.

## Verification Layers

1. Per-opportunity blocker isolation -> focused candidate-generation tests
2. Queue/facility blocker persistence reaching candidate filtering -> focused runtime + golden coverage
3. Single-layer archival ticket; no additional verification mapping is needed because this documents delivered work

## What Changed

### 1. Removed global blocker suppression during emission

Candidate emission no longer performs desire-wide `is_blocked(... None, None, None ...)` checks before the full opportunity set exists.

### 2. Added post-emission filtering

Candidate generation now filters after emission so each concrete opportunity is evaluated independently.

### 3. Used concrete opportunity scope rather than desire-level aliasing

Filtering now matches against:

- `GroundedGoal.anchor`
- `GroundedGoal.evidence_places`
- `GroundedGoal.evidence_entities`

This preserves facility/source/seller-specific blocker behavior.

## Files Touched

- [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) (modified in archived S33OPPSCOGOAIDE-002 work)

## Out of Scope

- opportunity-keyed exhaustion
- post-rank dedup with exhaustion fallthrough
- `PlannedPlan.opportunity`
- desire-level blocked diagnostics in decision traces

## Acceptance Criteria

### Tests That Must Pass

1. Matching blocked opportunity is suppressed without suppressing sibling opportunities for the same `GoalKey`.
2. Existing suite: `cargo test -p worldwake-ai`
3. Existing suite: `cargo clippy --workspace`

### Invariants

1. Candidate filtering is no longer desire-wide at emission time.
2. Opportunity-scoped blocker matching is the only candidate-generation blocker gate.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — blocker isolation tests were delivered under archived S33OPPSCOGOAIDE-002.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - the intended two-pass blocker behavior shipped as part of archived S33OPPSCOGOAIDE-002
  - candidate-generation blocker matching now operates on concrete opportunity scope instead of desire-wide emission-time suppression
- Deviations from original plan:
  - this work was not left as a separate pending implementation ticket because the live refactor showed the behaviors were architecturally inseparable
- Verification results:
  - delivered behavior is covered by the archived S33OPPSCOGOAIDE-002 verification set, including `cargo test -p worldwake-ai` and `cargo clippy --workspace`
