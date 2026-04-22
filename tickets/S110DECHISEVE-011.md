# S110DECHISEVE-011: Refresh observer decision-history golden after live commit-event drift

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — observer golden/fixture truthing only unless reassessment proves a production regression.
**Deps**: [archive/tickets/S110DECHISEVE-006](../archive/tickets/S110DECHISEVE-006.md), [archive/tickets/S112PORPLAN-005](../archive/tickets/S112PORPLAN-005.md)

## Problem

`cargo test --workspace` currently fails at `cargo test -p worldwake-cli --test observer_decision_history` because `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` no longer matches the live observer output for `scenarios/survival-baseline.ron`. The fixture still expects `GoalCommitted` rows at several early ticks where the current branch now emits only `GoalOffered`/`PlanAdopted` or otherwise different event ordering/content. This is observer golden drift and must be reassessed against the live decision-history event contract before broader verification can be treated as green again.

## Assumption Reassessment (2026-04-22)

1. The failing proof surface is `crates/worldwake-cli/tests/observer_decision_history.rs::survival_baseline_decision_history_section_matches_golden`, which runs the compiled observer against `scenarios/survival-baseline.ron` for 5 ticks and compares extracted Section 3 markdown against `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md`.
2. The live mismatch is concrete: the current output omits several previously-expected `GoalCommitted` rows while preserving later `GoalCommitted` / `GoalAbandoned` rows, so the first task is to determine whether the fixture is stale or whether a production event-emission regression has reopened in the AI/planning seam.
3. The shared boundary under audit is authoritative decision-history rendering: `worldwake-ai` emits `DecisionEventPayload` rows into the event log, and `crates/worldwake-cli/src/bin/observer.rs` renders those rows in log order. The ticket should prove which side changed truthfully before refreshing any golden.
4. Archived ticket `S112PORPLAN-005` previously refreshed this same observer fixture after portfolio-led planning changed the 5-tick decision-history output. The new drift may be another lawful downstream contract change rather than a fresh observer bug.

## Architecture Check

1. The correct fix is to reassess the authoritative event seam first, then either refresh the fixture or land the narrowest production regression fix. Do not patch the fixture blindly.
2. No new observer/report abstraction should be introduced; this is a truthing pass on an existing binary-driven golden seam.

## Verification Layers

1. Authoritative decision-history event presence/order -> focused `worldwake-ai` / event-log seam if reassessment shows emission changed unexpectedly.
2. Observer rendering output -> `cargo test -p worldwake-cli --test observer_decision_history`.
3. Broad regression guard -> `cargo test --workspace`.

## What to Change

### 1. Reassess the live authoritative event seam for the 5-tick survival-baseline run

Confirm whether the missing/shifted `GoalCommitted` rows are a lawful live behavior change or a production regression in event emission/order.

### 2. Make the observer golden truthful

If the live event stream is correct, refresh `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` to match the actual Section 3 output. If the live event stream is wrong, fix the production seam first and then refresh the golden only if the truthful output changed.

## Files to Touch

- `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` (modify if fixture is stale)
- `crates/worldwake-cli/tests/observer_decision_history.rs` (modify only if the extraction/assertion seam itself is stale)
- `crates/worldwake-ai/src/agent_tick/planning.rs` or other authoritative emitter files (modify only if reassessment proves a production regression)

## Out of Scope

- New decision-history event types or new observer sections
- Agenda-manager lifecycle work owned by S115

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --test observer_decision_history`
2. `cargo test --workspace`

### Invariants

1. The observer decision-history fixture matches the live authoritative Section 3 output for the 5-tick survival-baseline run.
2. Any refreshed fixture is justified by the real authoritative event-emission contract, not by a blind snapshot overwrite.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/tests/observer_decision_history.rs::survival_baseline_decision_history_section_matches_golden` — binary-driven golden for the live Section 3 output.

### Commands

1. `cargo test -p worldwake-cli --test observer_decision_history`
2. `cargo test --workspace`
