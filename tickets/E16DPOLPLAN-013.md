# E16DPOLPLAN-013: Golden Scenario 16 — Survival pressure suppresses political goals

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test proves that survival needs (critical hunger) suppress political goals and that political goals emerge after survival pressure is relieved.

## Assumption Reassessment (2026-03-18)

1. `GoalKind::ClaimOffice` is Medium priority class — confirmed from goal_policy
2. Critical hunger generates Critical priority goals — confirmed (above Medium)
3. `is_suppressed()` / `evaluate_suppression()` gates goal generation — confirmed from S02
4. Food consumption reduces hunger below threshold — confirmed

## Architecture Check

1. Tests priority ordering: survival Critical > political Medium
2. Tests suppression lift: after eating, hunger drops -> political goal unblocked
3. Follows existing suppression pattern proven in golden_social.rs (Scenario 2e suppression)

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office at VillageSquare. Critically hungry agent with `enterprise_weight=pm(800)`, eligible. Food (bread) available locally.
- **Expected**: Agent suppresses `ClaimOffice` under survival pressure. Eats bread first. After hunger relief, generates `ClaimOffice` and declares. Politics system installs agent.
- **Assertions**: Bread consumed before DeclareSupport event in event log timeline. Agent eventually becomes office holder.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Thirst/fatigue/dirtiness suppression of political goals (same code path)
- Bribe/Threaten scenarios
- Changes to suppression logic
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_pressure_suppresses_political_goals` — eat before declare, eventually installed
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Critical survival goals always preempt Medium political goals
2. Political goals emerge once survival pressure drops below threshold
3. Event log ordering reflects temporal causality

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_survival_pressure_suppresses_political_goals`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
