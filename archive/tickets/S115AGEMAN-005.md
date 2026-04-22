# S115AGEMAN-005: Live agenda lifecycle migration in agent tick

**Status**: COMPLETED  
**Priority**: HIGH  
**Effort**: Medium  
**Engine Changes**: Yes — calls `tick_agenda` at the live post-ranking seam, moves committed rejection parking onto the real `AgendaState`, parks satisfied goals into `suspended`, and aligns abandonment cleanup with agenda-state preservation.  
**Deps**: [archive/tickets/S115AGEMAN-003](../archive/tickets/S115AGEMAN-003.md), [archive/tickets/S115AGEMAN-004](../archive/tickets/S115AGEMAN-004.md)

## Problem

`AgendaState`, `tick_agenda`, and `classify_rejection` existed, but the live agent tick still handled key lifecycle behavior through detached planner-local state:

1. committed-goal rejection handling mutated only the temporary committed slot instead of the real `pending` / `suspended` agenda maps
2. satisfied cargo-delivery goals relied on compatibility-shaped committed-slot parking rather than the real suspended store
3. end-of-tick abandonment cleanup could misclassify a parked goal as abandoned if it no longer occupied the committed slot

This ticket owns the live lifecycle migration that is actually truthful on the current branch: the agenda manager runs at the post-ranking seam, but downstream planning continues to consume the fresh ranked feed, and executable commitment remains finalized at selected-plan adoption. The real implementation target is the single stored agenda state, not a false “pre-planning agenda authority” story.

## Assumption Reassessment (2026-04-22)

1. The earlier draft’s `agenda_tick_system before candidate generation` boundary was false. The landed `tick_agenda(actor, state, fresh_candidates, ...)` contract consumes fresh ranked `AgendaEntry` values, so the truthful insertion point is after `read_result.ranked` has been feasibility-annotated and re-sorted in `crates/worldwake-ai/src/agent_tick/mod.rs`.
2. The stronger rewrite that made `tick_agenda` the only commitment-selection authority was also false on the live branch. `agent_tick::tests::cargo_satisfaction_at_destination_while_carrying` proved first-tick executable commitment still finalizes at selected-plan adoption, not raw ranked-candidate commitment.
3. Feeding the agenda manager’s merged `ordered_candidates` back into downstream planning/interrupt evaluation regressed `golden_s107_cooldown_spaces_proactive_exploration_attempts`. The truthful live seam keeps `tick_agenda` for lifecycle state mutation while downstream search still uses the fresh ranked feed.
4. The committed-rejection classifier seam from ticket 004 was still valid, but its caller had to mutate the real `AgendaState`: `Satisfied` must move the entry into `suspended`, `InfeasibleUntil` into `pending`, and `Dead` must clear commitment plus write discrepancy memory.
5. `GoalCommitted` / `PlanAdopted` same-tick ordering remains a real contract, but on the live branch it is still authored at the plan-selection seam rather than from `AgendaTransitions`. Preserving that seam keeps the selected executable goal and the emitted commitment event aligned.

## Architecture Check

1. The landed change removes the false detached-lifecycle path and makes `AgendaState` the real stored authority for parked and suspended goals.
2. No compatibility alias or shadow `ActiveGoal` path was reintroduced.
3. The remaining executable-commitment seam is explicit: ranking/lifecycle state updates happen in `tick_agenda`, while commitment to a concrete plan still finalizes at selected-plan adoption. The ticket is written to that truthful boundary rather than overclaiming a stronger migration.

## What to Change

### 1. Run `tick_agenda` at the true ranked-candidate seam

In `crates/worldwake-ai/src/agent_tick/mod.rs`, after read-phase ranking and feasibility annotation:

- call `tick_agenda` with the live `AgendaState`, live belief view, discrepancy memory, and `AgendaTickPolicy`
- keep downstream planning and interruption on the fresh ranked feed, because the merged agenda-manager pool is not the live lawful consumer path yet

### 2. Move committed rejection parking onto the real agenda maps

In `crates/worldwake-ai/src/agent_tick/planning.rs`:

- `Satisfied` moves the committed entry into `agenda_state.suspended`
- `InfeasibleUntil` moves it into `agenda_state.pending` with the classified revival trigger
- `Dead` clears `agenda_state.committed` and records discrepancy memory

### 3. Park satisfied committed goals through `AgendaState`

In `crates/worldwake-ai/src/agent_tick/mod.rs`, the satisfied-goal short-circuit now removes the committed entry from the committed slot and inserts it into `agenda_state.suspended` with `KillCondition::External`.

### 4. Keep abandonment cleanup aligned with parked agenda entries

At end-of-tick cleanup in `mod.rs`, a previously committed goal is not emitted as `GoalAbandoned` if it still exists in `AgendaState.pending` or `AgendaState.suspended`.

## Files Touched

- `crates/worldwake-ai/src/agenda_manager.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/lib.rs`

## Out of Scope

- Full unification of executable commitment selection under `tick_agenda`
- Routing downstream planning/interrupt evaluation through `AgendaTransitions.ordered_candidates`
- Golden agenda lifecycle coverage (`S115AGEMAN-007`)
- Broader two-tick runtime/traceability proofs (`S115AGEMAN-006`)
- Event-log payload shape changes in `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. Focused agenda-manager tests covering margin and parking behavior.
2. `cargo test -p worldwake-ai agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
3. `cargo test -p worldwake-ai --test golden_portfolio_planning portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal -- --exact`
4. `cargo test -p worldwake-ai --test golden_exploration golden_s107_cooldown_spaces_proactive_exploration_attempts -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The committed rejection lifecycle updates the real `AgendaState`, not a detached temporary slot.
2. Satisfied cargo delivery parks the goal in `AgendaState.suspended` rather than leaving a non-committed entry in the committed slot.
3. A goal parked into `pending` or `suspended` is not emitted as abandoned in the same tick.
4. Downstream planning keeps using the fresh ranked feed until a later ticket proves the merged agenda-manager pool is a safe lawful replacement.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs`
   - updated cargo-delivery witness to assert the goal moves into `agenda_state.suspended`
   - added a no-`GoalAbandoned` assertion for the satisfied cargo path
2. `crates/worldwake-ai/src/agent_tick/planning.rs`
   - retained the planner-side `GoalCommitted` / `PlanAdopted` ordering test because that remains the truthful event-emission seam
3. `crates/worldwake-ai/src/agenda_manager.rs`
   - kept the manager-local margin and parking tests

### Commands

1. `cargo test -p worldwake-ai agenda_manager::tests -- --list`
2. `cargo test -p worldwake-ai agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
3. `cargo test -p worldwake-ai --test golden_portfolio_planning portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal -- --exact`
4. `cargo test -p worldwake-ai --test golden_exploration golden_s107_cooldown_spaces_proactive_exploration_attempts -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo fmt --all`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Implemented the live lifecycle migration that is truthful on the current branch.

- `tick_agenda` now runs in `agent_tick/mod.rs` at the post-ranking seam.
- committed rejection handling in `agent_tick/planning.rs` now parks goals into the real `AgendaState.pending` / `AgendaState.suspended` maps.
- satisfied-goal short-circuiting in `agent_tick/mod.rs` now suspends the goal in the agenda map instead of leaving a non-committed entry in the committed slot.
- abandonment cleanup now preserves parked agenda entries.
- the stronger intermediate rewrite was backed out where live evidence disproved it:
  - downstream planning still consumes the fresh ranked feed, not `AgendaTransitions.ordered_candidates`
  - `GoalCommitted` remains emitted from selected-plan adoption, because that is still the truthful executable-commitment seam on the live branch

## Verification Result

Passed:

- `cargo test -p worldwake-ai agenda_manager::tests -- --list`
- `cargo test -p worldwake-ai agent_tick::tests::cargo_satisfaction_at_destination_while_carrying -- --exact`
- `cargo test -p worldwake-ai --test golden_portfolio_planning portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal -- --exact`
- `cargo test -p worldwake-ai --test golden_exploration golden_s107_cooldown_spaces_proactive_exploration_attempts -- --exact`
- `cargo test -p worldwake-ai`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
