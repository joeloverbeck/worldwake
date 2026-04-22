# S115AGEMAN-006: Unit + integration tests — agenda lifecycle correctness

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — this ticket adds tests only; no production-code changes beyond test support helpers.
**Deps**: [archive/tickets/S115AGEMAN-005](../archive/tickets/S115AGEMAN-005.md)

## Problem

The spec's Validation and Falsification section lists 16 unit/integration tests (items 1-16) that collectively prove the agenda lifecycle works correctly. Most are already partially covered by tickets 003 (module-level unit tests) and 004 (classifier tests), but the integration-level coverage — two-tick commit persistence, replay determinism, cargo-delivery with classifier-driven suspension, portfolio rejection routing through the classifier, and the `ActiveGoal` zero-match grep validation — must be consolidated and verified end-to-end. This ticket is the validation-suite ticket: it writes the tests that prove the agenda manager composes correctly with perception, ranking, and the event log. Without it, lifecycle correctness is asserted only at the unit level and we cannot claim the S115 contract is satisfied.

## Assumption Reassessment (2026-04-22)

1. The drafted “new integration tests” for unchanged commit persistence, commodity-trigger revival, tick-expiry kill, and capacity eviction are already covered at the pure state-transition layer in `crates/worldwake-ai/src/agenda_manager.rs` by `unchanged_commit_does_not_recommit_when_current_goal_survives`, `revival_trigger_commodity_available_fires_when_belief_confirms_quantity`, `kill_condition_tick_expiry_drops_entry_on_or_after_expiry`, and `capacity_overflow_evicts_smallest_last_reconsidered_tick`.
2. Existing harness `crates/worldwake-ai/src/agent_tick/tests.rs` already exercises `runtime_by_agent` and reads `AgendaState` via `runtime.agenda_state`. `cargo_satisfaction_at_destination_while_carrying` already proves the satisfied cargo goal is parked in `AgendaPhase::Suspended`, so that drafted edit is already landed.
3. `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` in `crates/worldwake-ai/tests/golden_portfolio_planning.rs` is the strongest live integration/golden seam for classifier-driven rejection routing. The remaining live delta is to prove dead-routing aftermath at the real runtime/event seam: the stale committed `ReportMissing` goal is abandoned and does not linger in `AgendaState.pending` or `AgendaState.suspended`.
4. The shared boundary under audit is the live `AgendaState` + selected-plan event seam after ticket 005: `tick_agenda` mutates lifecycle state at the post-ranking caller seam, `apply_committed_rejection_lifecycle` maps rejected committed goals to `Satisfied` / `InfeasibleUntil` / `Dead`, and `GoalCommitted` / `GoalAbandoned` still emit from the selected-plan plus outer tick closeout seams. This ticket should prove those surfaces stay coherent rather than duplicating `AgendaTransitions` unit coverage in a new file.
5. `ActiveGoal` removal is already complete on the live branch: `rg -n "ActiveGoal|get_component_active_goal|set_component_active_goal|has_component_active_goal|insert_component_active_goal|remove_component_active_goal|iter_active_goals|entities_with_active_goal|query_active_goal|count_with_active_goal" crates/` returns zero matches. The honest remaining work is to make that grep invariant CI-visible via a script.
6. Replay/runtime round-trip proof belongs to the AI runtime serialization seam, not `worldwake-sim/tests/save_load.rs`: `AgentDecisionRuntime` owns `agenda_state`, `crates/worldwake-ai/src/decision_runtime.rs` already has the focused bincode round-trip test, and `crates/worldwake-ai/tests/golden_harness/mod.rs` already covers save/load preservation of driver runtime bytes. This ticket should extend the focused runtime serialization test with a populated non-default `AgendaState`.

## Reassessment Notes

- Already landed:
  - Agenda-manager lifecycle unit coverage in `crates/worldwake-ai/src/agenda_manager.rs`
  - Satisfied cargo suspension proof in `crates/worldwake-ai/src/agent_tick/tests.rs`
  - Runtime save/load harness coverage in `crates/worldwake-ai/tests/golden_harness/mod.rs`
- Still live:
  - Dead-routing aftermath assertion in `crates/worldwake-ai/tests/golden_portfolio_planning.rs`
  - Populated `AgendaState` round-trip assertion in `crates/worldwake-ai/src/decision_runtime.rs`
  - `ActiveGoal` grep regression script plus `scripts/verify.sh` hook
- No-change cited files:
  - `crates/worldwake-ai/src/agent_tick/tests.rs`
  - `crates/worldwake-sim/src/save_load.rs`

## Architecture Check

1. Tests stay at the strongest existing layer: lifecycle mechanics remain in `agenda_manager.rs`, runtime serialization stays in `decision_runtime.rs`, integration/golden rejection routing stays in `golden_portfolio_planning.rs`, and the `ActiveGoal` invariant becomes a repo-level script. This avoids duplicating already-landed unit coverage in a new `agenda_integration.rs` file.
2. No new abstractions. Test helpers (`MockGoalBeliefView`, harness builders) reuse existing patterns.

## Verification Layers

1. Agenda lifecycle mechanics already proved at the pure state-transition layer -> `agenda_manager.rs` unit tests cover unchanged commit, revival, kill expiry, and capacity eviction.
2. Runtime serialization -> `crates/worldwake-ai/src/decision_runtime.rs` bincode round-trip proves populated `AgendaState` survives focused runtime serialization intact.
3. Satisfied cargo path -> existing `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` proves the committed cargo goal moves to `runtime.agenda_state.suspended` and does not emit `GoalAbandoned`.
4. Portfolio dead-routing via classifier -> `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` proves the stale committed `ReportMissing` goal is abandoned at the outer event seam and absent from pending/suspended agenda state after the feasible economic goal commits.
5. ActiveGoal zero-match -> `bash scripts/check_active_goal_removed.sh` fails on any reintroduced `ActiveGoal` production reference under `crates/`.

## What to Change

### 1. Extend focused runtime serialization proof

Extend `crates/worldwake-ai/src/decision_runtime.rs`'s bincode round-trip test so `AgentDecisionRuntime` carries a non-default `AgendaState` with committed, pending, and suspended entries. Assert round-trip equality on `decoded.agenda_state`.

### 2. Tighten the existing portfolio golden at the real classifier aftermath seam

Modify `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` to assert:
- the stale committed `ReportMissing` goal emits `GoalAbandoned` when the feasible economic goal wins
- the stale committed goal is absent from `runtime.agenda_state.pending` and `runtime.agenda_state.suspended` after the tick
- the feasible economic goal remains the sole committed agenda entry

### 3. Add an `ActiveGoal` grep-regression script and wire it into verify

Create `scripts/check_active_goal_removed.sh` to grep for `ActiveGoal`-family symbols under `crates/` and fail on any match. Invoke it from `scripts/verify.sh`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify — add classifier-path assertion)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — populated `AgendaState` serialization proof)
- `scripts/check_active_goal_removed.sh` (new)
- `scripts/verify.sh` (modify — invoke new grep script)

## Out of Scope

- Changes to agenda production logic. This ticket remains proof + repo-check only.
- Golden `golden_agenda_lifecycle.rs` scenario (ticket 007 — separate because goldens have their own harness and fixture authoring cost).
- Re-implementing lifecycle mechanics already covered in `agenda_manager.rs`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes with the dead-routing aftermath assertions.
2. `cargo test -p worldwake-ai agent_decision_runtime_bincode_round_trip_preserves_all_fields` passes with a populated non-default `AgendaState`.
3. `bash scripts/check_active_goal_removed.sh` exits 0.
4. Broad rerun: `cargo test --workspace` reaches only the unrelated `cargo test -p worldwake-cli --test observer_decision_history` fixture drift now tracked by [S110DECHISEVE-011](./S110DECHISEVE-011.md).

### Invariants

1. `AgentDecisionRuntime` round-trips a populated `AgendaState` without dropping committed, pending, or suspended entries.
2. Structurally dead committed goals are abandoned rather than lingering in agenda pending/suspended state after a feasible replacement goal commits.
3. Zero `ActiveGoal` references remain in production code under `crates/`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` — proves classifier dead-routing abandons the stale committed goal and keeps it out of parked agenda state.
2. `crates/worldwake-ai/src/decision_runtime.rs::agent_decision_runtime_bincode_round_trip_preserves_all_fields` — proves populated `AgendaState` survives focused runtime serialization.
3. `scripts/check_active_goal_removed.sh` — repo-level grep regression for the removed `ActiveGoal` surface.

### Commands

1. `cargo test -p worldwake-ai portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal`
2. `cargo test -p worldwake-ai agent_decision_runtime_bincode_round_trip_preserves_all_fields`
3. `bash scripts/check_active_goal_removed.sh`
4. `cargo test --workspace`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-22.

- Extended `crates/worldwake-ai/src/decision_runtime.rs` so the focused runtime bincode round-trip now carries a populated non-default `AgendaState` and proves committed, pending, and suspended entries survive serialization.
- Tightened `crates/worldwake-ai/tests/golden_portfolio_planning.rs` at the live classifier aftermath seam: the test now proves a structurally dead committed `ReportMissing` goal emits `GoalAbandoned`, disappears from parked agenda state, and leaves the feasible economic goal as the sole committed entry.
- Added `scripts/check_active_goal_removed.sh` and wired it into `scripts/verify.sh` so `ActiveGoal`-surface reintroduction under `crates/` becomes a CI-visible regression.
- Reassessed the drafted scope against the live branch and corrected the ticket to the truthful owned boundary instead of creating duplicate `agenda_manager` integration tests or touching `worldwake-sim/src/save_load.rs`.

## Deviations

- The drafted `agenda_integration.rs` file and the planned edit to `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` were already effectively landed on the branch, so this ticket narrowed to the remaining live delta.
- The replay proof landed in `crates/worldwake-ai/src/decision_runtime.rs`, which is the truthful owner of `AgentDecisionRuntime.agenda_state`, rather than in `crates/worldwake-sim/src/save_load.rs`.
- Broad verification exposed an unrelated observer decision-history golden mismatch in `worldwake-cli`; this ticket did not absorb that fixture refresh. Follow-up ticket: [S110DECHISEVE-011](./S110DECHISEVE-011.md).

## Verification Result

- Passed `cargo test -p worldwake-ai portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal`
- Passed `cargo test -p worldwake-ai agent_decision_runtime_bincode_round_trip_preserves_all_fields`
- Passed `bash scripts/check_active_goal_removed.sh`
- Failed unrelated broad check: `cargo test --workspace` at `cargo test -p worldwake-cli --test observer_decision_history` due stale fixture drift now tracked by [S110DECHISEVE-011](./S110DECHISEVE-011.md)
- Not run: `./scripts/verify.sh` because it would stop at the same unrelated workspace-test blocker above
