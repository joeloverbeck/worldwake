# GOLDE2E-004: Goal-Switch Margin Boundary

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None expected — margin math exists in `goal_switching.rs`
**Deps**: None

## Problem

Goal switching occurs in several golden scenarios (2, 3c, 7d) but the exact margin threshold math is never directly exercised. A regression in `compare_goal_switch()` that shifts the margin boundary by even 1 permille would go undetected. This test proves the boundary: a challenger motive just below margin → no switch; a challenger that exceeds margin → switch occurs.

## Report Reference

Backlog item **P-NEW-3** in `reports/golden-e2e-coverage-analysis.md` (Tier 2, composite score 4).

## Assumption Reassessment (2026-03-13)

1. `compare_goal_switch()` exists in `worldwake-ai/src/goal_switching.rs`.
2. `GoalSwitchKind` and margin thresholds use `Permille` arithmetic.
3. The golden harness can configure agents with precise need levels and utility weights.
4. Existing goal-switching tests verify that switching happens but not the exact boundary.

## Architecture Check

1. This test exercises existing margin math — no new architecture needed.
2. Precise `Permille` setup avoids floating-point ambiguity.

## Engine-First Mandate

If implementing this e2e suite reveals that the goal-switch margin math is architecturally unsound, inconsistent across priority classes, or relies on magic numbers rather than profile-driven parameters — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that makes goal-switch margins clean, robust, and extensible. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_ai_decisions.rs`

**Setup**: Agent with two competing needs. Configure metabolism rates and initial levels so that:
- Phase 1: Challenger motive is just below the margin threshold relative to the incumbent goal → agent continues current action.
- Phase 2: After a few more ticks, challenger crosses the margin → agent switches goals.

**Assertions**:
- Agent starts on the incumbent goal.
- During the sub-margin phase, the agent does NOT switch despite the challenger being elevated.
- After the margin is crossed, the agent switches to the challenger goal.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)

## Out of Scope

- Testing all possible goal-switch combinations
- Changing margin threshold values
- Multi-way goal competition (already tested in 7d)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_goal_switch_margin_boundary` — sub-margin challenger does not switch; super-margin challenger does
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual goal injection
2. Margin math uses `Permille` exclusively
3. Conservation holds throughout

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-3 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_goal_switch_margin_boundary` — proves exact margin threshold

### Commands

1. `cargo test -p worldwake-ai golden_goal_switch_margin`
2. `cargo test --workspace && cargo clippy --workspace`
