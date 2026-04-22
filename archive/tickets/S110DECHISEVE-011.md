# S110DECHISEVE-011: Refresh observer decision-history golden after live commit-event drift

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — observer golden/fixture truthing only unless reassessment proves a production regression.
**Deps**: [archive/tickets/S110DECHISEVE-006](../archive/tickets/S110DECHISEVE-006.md), [archive/tickets/S112PORPLAN-005](../archive/tickets/S112PORPLAN-005.md)

## Problem

`cargo test --workspace` currently fails at `cargo test -p worldwake-cli --test observer_decision_history` because `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` no longer matches the live observer output for `scenarios/survival-baseline.ron`. The fixture still expects `GoalCommitted` rows at several early ticks where the current branch now emits only `GoalOffered`/`PlanAdopted` or otherwise different event ordering/content. This is observer golden drift and must be reassessed against the live decision-history event contract before broader verification can be treated as green again.

## Assumption Reassessment (2026-04-22)

1. The failing proof surface is `crates/worldwake-cli/tests/observer_decision_history.rs::survival_baseline_decision_history_section_matches_golden`, which runs the compiled observer against `scenarios/survival-baseline.ron` for 5 ticks and compares extracted Section 3 markdown against `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md`.
2. The live mismatch is concrete: the current output omits several previously-expected `GoalCommitted` rows and the paired `GoalAbandoned` rows that depended on them, while preserving later `GoalCommitted` / `GoalAbandoned` rows when the selected plan actually changes the committed goal.
3. The shared boundary under audit is authoritative decision-history rendering: `worldwake-ai` emits `DecisionEventPayload` rows into the event log, and `crates/worldwake-cli/src/bin/observer.rs` renders those rows in log order. The observer render path remains truthful; the question is whether the upstream event seam changed lawfully.
4. The live emitter at `crates/worldwake-ai/src/agent_tick/planning.rs::emit_plan_selection_events` now emits `GoalCommitted` only when `current_goal_before_selection != Some(selected_plan.goal)`, while `PlanAdopted` still emits for every selected plan. That matches the observed 5-tick output: continued adoption of an already-committed agenda goal no longer produces a duplicate commit event.
5. Archived ticket `S112PORPLAN-005` previously refreshed this same observer fixture after portfolio-led planning changed the 5-tick decision-history output. This ticket is the next truthful fixture refresh for the post-S115 agenda-manager contract, not a new observer or renderer bug.

## Architecture Check

1. The correct fix is to reassess the authoritative event seam first, then either refresh the fixture or land the narrowest production regression fix. Do not patch the fixture blindly.
2. No new observer/report abstraction should be introduced; this is a truthing pass on an existing binary-driven golden seam.

## Verification Layers

1. Authoritative decision-history event presence/order -> focused `cargo test -p worldwake-ai --test golden_decision_history_events`.
2. Observer rendering output -> `cargo test -p worldwake-cli --test observer_decision_history`.
3. Broad regression guard -> `cargo test --workspace`.

## What to Change

### 1. Reassess the live authoritative event seam for the 5-tick survival-baseline run

Confirm whether the missing/shifted `GoalCommitted` rows are a lawful live behavior change or a production regression in event emission/order.

### 2. Refresh the observer golden to the live agenda-manager event contract

Refresh `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` to match the actual Section 3 output, including the omission of duplicate `GoalCommitted` rows when the selected plan continues an already-committed goal.

## Files to Touch

- `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` (modify if fixture is stale)
- `crates/worldwake-cli/tests/observer_decision_history.rs` (no change cited — extraction/assertion seam remained correct)
- `crates/worldwake-ai/src/agent_tick/planning.rs` or other authoritative emitter files (no change cited — reassessment proved lawful emitter drift, not a production regression)

## Out of Scope

- New decision-history event types or new observer sections
- Agenda-manager lifecycle work owned by S115

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_decision_history_events`
2. `cargo test -p worldwake-cli --test observer_decision_history`
3. `cargo test --workspace`

### Invariants

1. The observer decision-history fixture matches the live authoritative Section 3 output for the 5-tick survival-baseline run.
2. The fixture omits duplicate `GoalCommitted` rows when `PlanAdopted` continues an already-committed goal, matching the live `emit_plan_selection_events` contract.
3. The refresh is justified by the real authoritative event-emission contract, not by a blind snapshot overwrite.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_decision_history_events.rs::survival_baseline_emits_goal_commit_and_plan_adoption_in_order` — proves the `GoalCommitted` / `PlanAdopted` family still exists and remains ordered on the live branch.
2. `crates/worldwake-cli/tests/observer_decision_history.rs::survival_baseline_decision_history_section_matches_golden` — binary-driven golden for the live Section 3 output.

### Commands

1. `cargo test -p worldwake-ai --test golden_decision_history_events`
2. `cargo test -p worldwake-cli --test observer_decision_history`
3. `cargo test --workspace`

## Outcome

Completed on 2026-04-22.

- Reassessed the live decision-event seam before touching the fixture. `crates/worldwake-ai/src/agent_tick/planning.rs::emit_plan_selection_events` now emits `GoalCommitted` only when the selected plan changes the committed goal, while `PlanAdopted` still emits for every selected plan.
- Confirmed the event family remains live and ordered via `crates/worldwake-ai/tests/golden_decision_history_events.rs::survival_baseline_emits_goal_commit_and_plan_adoption_in_order`; this ticket did not need a production fix.
- Refreshed `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` so Section 3 matches the post-S115 agenda-manager contract for the 5-tick survival-baseline run.

## Deviations

- The original draft left open a possible production regression in the AI/planning emitter. Live reassessment showed truthful upstream contract drift instead: duplicate `GoalCommitted` rows are no longer emitted when `PlanAdopted` continues an already-committed goal.
- No changes were needed in `crates/worldwake-cli/tests/observer_decision_history.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs`; the ticket narrowed to fixture truthing plus authoritative seam verification.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_decision_history_events`
- Passed `cargo test -p worldwake-cli --test observer_decision_history`
- Passed `cargo test --workspace`
