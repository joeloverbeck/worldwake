# S115AGEMAN-006: Unit + integration tests — agenda lifecycle correctness

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — this ticket adds tests only; no production-code changes beyond test support helpers.
**Deps**: [S115AGEMAN-005](S115AGEMAN-005.md)

## Problem

The spec's Validation and Falsification section lists 16 unit/integration tests (items 1-16) that collectively prove the agenda lifecycle works correctly. Most are already partially covered by tickets 003 (module-level unit tests) and 004 (classifier tests), but the integration-level coverage — two-tick commit persistence, replay determinism, cargo-delivery with classifier-driven suspension, portfolio rejection routing through the classifier, and the `ActiveGoal` zero-match grep validation — must be consolidated and verified end-to-end. This ticket is the validation-suite ticket: it writes the tests that prove the agenda manager composes correctly with perception, ranking, and the event log. Without it, lifecycle correctness is asserted only at the unit level and we cannot claim the S115 contract is satisfied.

## Assumption Reassessment (2026-04-22)

1. Existing harness `crates/worldwake-ai/src/agent_tick/tests.rs` already exercises `runtime_by_agent` and observes per-agent state. Post-ticket 002 it reads `AgendaState` via `runtime.agenda_state` rather than `get_component_active_goal`.
2. The scenario fixtures used by existing agent-tick tests (e.g., `cargo_harness`) already construct agents with `MerchandiseProfile` and spawn them in a small world. For two-tick commit persistence and cargo-delivery tests, these fixtures are the natural base.
3. `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` at `crates/worldwake-ai/tests/golden_portfolio_planning.rs:210` is an integration test using a golden harness. Post-ticket 005 it passes via the classifier's `Dead` routing for structurally-infeasible commitments. This ticket verifies it still passes AND adds a trace-level assertion that the transition happens via `classify_rejection` (decision-trace surface) rather than the old carve-out (confirmed absent by the grep invariant).
4. The shared boundary under audit is the `AgendaTransitions` → event-log pipeline (from ticket 005). Integration tests assert that emitted events match agenda state mutations cycle-for-cycle.
5. `ActiveGoal` zero-match grep: after ticket 002, no references remain to `ActiveGoal`, `get_component_active_goal`, `set_component_active_goal`, etc. This ticket adds a grep-regression test (e.g., a `compile_fail` doctest or a small bash script in `scripts/`) that asserts zero matches — catches future reintroduction.
6. Replay determinism: `crates/worldwake-sim/tests/save_load.rs` (or equivalent) already covers full-simulation replay. This ticket extends the coverage to populate `AgendaState` during a recorded run and verify re-load produces identical state.

## Architecture Check

1. Tests consolidated at the right layer: unit tests inside `agenda_manager.rs` (tickets 003/004), integration tests in `crates/worldwake-ai/tests/` (this ticket), and grep-regression via a CI-runnable script. No duplication — each layer proves a different invariant.
2. No new abstractions. Test helpers (`MockGoalBeliefView`, harness builders) reuse existing patterns.

## Verification Layers

1. Two-tick commit persistence — integration test using harness; assertion on `runtime.agenda_state.committed.as_ref().map(|e| e.key)` across two ticks.
2. Replay determinism — save/load round-trip test; `AgendaState` inside `AgentDecisionRuntime` survives bincode round-trip byte-identically.
3. Cargo-delivery via classifier — `cargo_satisfaction_at_destination_while_carrying` extended or mirrored to assert the committed entry has `phase: Suspended` after the satisfied pre-check fires (decision-trace proof that classifier was invoked, not the old carve-out).
4. Portfolio rejection via classifier — `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` extended with decision-trace assertion that the rejected `Sleep`/`ReportMissing` slots transit `RejectionLifecycle::Dead` rather than being dropped by ad-hoc filtering.
5. ActiveGoal zero-match — grep invariant: `scripts/` or CI check runs `rg -q 'ActiveGoal|get_component_active_goal|set_component_active_goal' crates/` and fails on any match outside comments, archived specs, or this ticket's own test assertion strings.

## What to Change

### 1. New integration tests in `crates/worldwake-ai/tests/agenda_integration.rs`

- `two_tick_commit_persists_when_belief_still_viable`: construct a harness with one agent + one goal; tick 1 commits; tick 2 verifies `AgendaState.committed` unchanged AND no re-commit event in event log (FND-21 stability).
- `revival_trigger_commodity_available_commits_pending_goal`: seed pending entry with `CommodityAvailable` trigger; belief update reports quantity; assert `GoalCommitted` emitted and pending → committed transition occurs.
- `kill_condition_tick_expiry_emits_abandoned`: seed pending entry with `KillCondition::TickExpiry`; tick past expiry; assert `GoalAbandoned` emitted and entry dropped.
- `capacity_overflow_evicts_oldest`: populate pending to capacity+1 entries; assert smallest-`last_reconsidered_tick` evicted, no crash.

### 2. Extend existing tests to assert classifier path

- `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` (modify): after the test's existing assertions, add:
  ```rust
  let committed = runtime.agenda_state.committed.as_ref().expect("committed entry");
  assert_eq!(committed.phase, AgendaPhase::Suspended);
  assert!(committed.revival_trigger.is_none());
  ```
  This asserts the classifier fired the Satisfied pre-check, not the carve-out (which no longer exists).
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` (modify): assert one `GoalAbandoned` event in the tick where the structurally-infeasible commitment is dropped.

### 3. Grep-regression script

Create `scripts/check_active_goal_removed.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
matches=$(rg -l 'ActiveGoal|get_component_active_goal|set_component_active_goal|has_component_active_goal|insert_component_active_goal|remove_component_active_goal|iter_active_goals|entities_with_active_goal|query_active_goal|count_with_active_goal' crates/ 2>/dev/null || true)
if [ -n "$matches" ]; then
    echo "ActiveGoal references found in:" >&2
    echo "$matches" >&2
    exit 1
fi
echo "✓ ActiveGoal removal verified — zero references"
```

Make it executable and invoke from `scripts/verify.sh` (append to existing verify pipeline).

### 4. Replay determinism extension

In `crates/worldwake-sim/tests/save_load.rs` (or closest existing replay test), add:
- `replay_preserves_populated_agenda_state`: construct `AgentDecisionRuntime` with populated `AgendaState` (1 committed + 2 pending + 1 suspended); save; load; assert round-trip equality on the full runtime.

## Files to Touch

- `crates/worldwake-ai/tests/agenda_integration.rs` (new — integration tests)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — extend `cargo_satisfaction_at_destination_while_carrying`)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify — add classifier-path assertion)
- `crates/worldwake-sim/tests/save_load.rs` (modify — add replay-preservation test; exact path depends on existing save/load test layout)
- `scripts/check_active_goal_removed.sh` (new)
- `scripts/verify.sh` (modify — invoke new grep script)

## Out of Scope

- Changes to production code (this ticket is test-only). If tests reveal production bugs, file a follow-up ticket per 1-3-1 instead of silently fixing in-scope.
- Golden `golden_agenda_lifecycle.rs` scenario (ticket 007 — separate because goldens have their own harness and fixture authoring cost).
- Performance-regression benchmarks — S115 is not a performance spec; no throughput guards needed.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- agenda_integration` — all 4 new integration tests pass.
2. `cargo test -p worldwake-ai -- cargo_satisfaction_at_destination_while_carrying` — passes with the new classifier-path assertion.
3. `cargo test -p worldwake-ai -- portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` — passes with the new event-log assertion.
4. `cargo test -p worldwake-sim -- replay_preserves_populated_agenda_state` — passes.
5. `bash scripts/check_active_goal_removed.sh` — exits 0.
6. Existing suite: `cargo test --workspace` passes.

### Invariants

1. Two-tick commit persistence: `AgendaState.committed` key is stable across ticks when belief and ranking inputs are unchanged.
2. Replay equivalence: bincode round-trip of a populated `AgentDecisionRuntime` is byte-identical.
3. Zero `ActiveGoal` references in production code.
4. Every observable lifecycle transition in these tests emits the correct S110 event exactly once.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/agenda_integration.rs` (new) — 4 integration tests covering commit persistence, revival-fires, kill-expiry, and capacity overflow.
2. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` (modify) — classifier-path assertion.
3. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` (modify) — event-log assertion.
4. `crates/worldwake-sim/tests/save_load.rs::replay_preserves_populated_agenda_state` (new).
5. `scripts/check_active_goal_removed.sh` (new) — CI grep regression.

### Commands

1. `cargo test -p worldwake-ai -- agenda_integration`
2. `cargo test -p worldwake-ai -- cargo_satisfaction portfolio_rejects`
3. `cargo test -p worldwake-sim -- save_load`
4. `bash scripts/check_active_goal_removed.sh`
5. `cargo test --workspace`
6. `./scripts/verify.sh`
