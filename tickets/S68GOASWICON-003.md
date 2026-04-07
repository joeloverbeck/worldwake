# S68GOASWICON-003: Golden test — goal-switch contention cleanup

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S68GOASWICON-001

## Problem

The goal-switch contention cleanup fix (S68GOASWICON-001) needs an E2E golden test proving the full failure path no longer produces `DuplicateActor` errors. Without this test, the fix could regress silently.

## Assumption Reassessment (2026-04-07)

1. `golden_dead_agent_pruned_from_facility_queue` (golden_production.rs:3118) confirmed as the pattern to follow — it tests contention queue cleanup after agent death. The goal-switch cleanup test follows the same structure but triggers cleanup via goal switch instead of death.
2. `GoalKind::TreatWounds { patient: EntityId }` confirmed at goal.rs:41-43.
3. `GoalKind::EscortToSafety { subject: EntityId, destination: EntityId }` confirmed at goal.rs:53-56.
4. `enqueue_for_contention` confirmed at `facility_queue_actions.rs:182` — called from `commit_escort_to_safety` (escort_actions.rs:480).
5. `ContentionQueue::enqueue` confirmed at contention.rs:90 — the call that would produce `DuplicateActor` if stale entries remain.
6. `DuplicateActor(EntityId)` confirmed as a variant of `ContentionError` at contention.rs:71.
7. The golden test must exercise the contention_system tick between goal switch and re-enqueue to verify the prune system removes the stale entry.
8. `PerceptionProfile` must be present on agents that need to observe post-production output — per CLAUDE.md golden test requirement.
9. No adjacent contradictions. The scenario requires both care actions (TreatWounds, EscortToSafety) and contention infrastructure, all of which are implemented.

## Architecture Check

1. Golden test follows the established pattern from `golden_dead_agent_pruned_from_facility_queue` — scenario setup, simulation run, outcome struct, assertions on contention state.
2. No backwards-compatibility shims. Pure test addition.

## Verification Layers

1. No `DuplicateActor` error on re-enqueue after goal switch -> golden E2E test assertions on simulation completion without panic
2. Stale `ContentionQueue` entry is pruned after intent clear -> authoritative world state assertion on queue contents
3. Agent successfully re-enqueues under new goal -> authoritative world state assertion on queue membership
4. Single-layer ticket (golden E2E) — the lower-layer proof is provided by S68GOASWICON-001's unit tests.

## What to Change

### 1. Design the scenario

Create a scenario with:
- A place with a facility (e.g., a healing station or care facility)
- A wounded agent W at the facility
- A caregiver agent C with both TreatWounds and EscortToSafety capabilities
- A safe destination for escort
- Initial conditions that cause C to adopt TreatWounds for W and enter the contention queue
- A trigger (e.g., threat arrival, priority shift) that causes C to switch goals to EscortToSafety for W

### 2. Write the golden test

Follow the pattern of `run_dead_agent_pruned_from_facility_queue_scenario`:

1. Build scenario with agents, places, and facility
2. Run simulation ticks until C enters W's contention queue for TreatWounds
3. Introduce conditions that trigger goal switch to EscortToSafety
4. Run simulation through contention_system prune tick
5. Verify C is no longer in W's contention queue under the old goal
6. Run simulation until EscortToSafety commits and re-enqueues C
7. Assert no `DuplicateActor` error occurred
8. Assert C is in W's contention queue under the new goal

### 3. Add deterministic replay test

Following the established pattern, add a companion test that runs the scenario with the same seed twice and asserts world hash and event log hash match.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify — add scenario runner, outcome struct, golden test, replay test)

## Out of Scope

- The actual fix (S68GOASWICON-001)
- Interrupt path audit (S68GOASWICON-002)
- New scenario RON files — the golden test builds its scenario programmatically
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. New golden test: `golden_goal_switch_contention_cleanup` — simulation completes without `DuplicateActor` panic, agent re-enqueues successfully
2. New replay test: `golden_goal_switch_contention_cleanup_replays_deterministically` — same seed produces same hashes
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. After goal switch, the agent must not appear in the contention queue under the old goal's intended action
2. After re-enqueue under the new goal, the agent must appear exactly once in the contention queue
3. Deterministic replay produces identical world and event log hashes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — `golden_goal_switch_contention_cleanup`: E2E proof that goal-switch contention cleanup prevents `DuplicateActor`
2. `crates/worldwake-ai/tests/golden_production.rs` — `golden_goal_switch_contention_cleanup_replays_deterministically`: determinism guard

### Commands

1. `cargo test -p worldwake-ai golden_goal_switch_contention_cleanup`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
