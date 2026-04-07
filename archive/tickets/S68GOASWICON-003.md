# S68GOASWICON-003: Golden test — goal-switch contention cleanup

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S68GOASWICON-001

## Problem

The goal-switch contention cleanup fix (S68GOASWICON-001) needs an E2E golden test proving the full failure path no longer produces `DuplicateActor` errors. Without this test, the fix could regress silently.

## Assumption Reassessment (2026-04-07)

1. `golden_dead_agent_pruned_from_facility_queue` (golden_production.rs:3118) confirmed as the pattern to follow — it tests contention queue cleanup after agent death. The goal-switch cleanup test follows the same structure but triggers cleanup via goal switch instead of death.
2. **Corrected**: Original narrative used TreatWounds → EscortToSafety, but EscortToSafety ops (`[Travel, EscortToSafety]`) do NOT include `QueueForFacilityUse`. The "re-enqueue under new goal" criterion would require both goals to queue at the same entity, demanding contrived setup that violates P1 (Maximal Emergence). Narrowed to production domain: hungry agent enters exclusive workstation queue, fatigue metabolism drives Sleep goal to overtake hunger motive. This is genuine emergence from interacting systems (Needs + Production + Contention).
3. The honest golden contract is: (a) agent enters contention queue via production goal, (b) goal revision occurs via competing need, (c) stale queue entry is pruned via state-mediated intent cleanup (P26), (d) simulation completes without DuplicateActor panic.
4. `PerceptionProfile` must be present on the test agent — per CLAUDE.md golden test requirement.
5. Existing pattern from `golden_facility_queue_patience_timeout` provides the setup template (grant holder blocking workstation, test agent queues). Difference: this test triggers a goal switch via metabolism-driven motive change, not patience exhaustion.
6. Sleep goal requires travel to CommonHouse (Inn), ensuring the agent leaves OrchardFarm after goal switch. The travel departure also exercises the departed-waiter prune path as a secondary verification.
7. S68GOASWICON-001 and S68GOASWICON-004 (completed) provide the production fix. S68GOASWICON-002 (completed) covers the failure-handling path.
8. No adjacent contradictions.

## Architecture Check

1. Golden test follows the established pattern from `golden_dead_agent_pruned_from_facility_queue` — scenario setup, simulation run, outcome struct, assertions on contention state.
2. No backwards-compatibility shims. Pure test addition.

## Verification Layers

1. No `DuplicateActor` error after goal switch -> golden E2E test completes without panic
2. Stale `ContentionQueue` entry is pruned after intent clear -> authoritative world state assertion on queue contents
3. Single-layer ticket (golden E2E) — the lower-layer proof is provided by S68GOASWICON-001's unit tests.

## What to Change

### 1. Design the scenario

Create a scenario at OrchardFarm with:
- A grant holder agent blocking the exclusive workstation (long grant duration)
- A test agent with high hunger (pm(900)) and moderate fatigue (pm(600)), slow hunger metabolism and fast fatigue metabolism
- An exclusive workstation with apple resource source
- PerceptionProfile and local beliefs seeded on the test agent

The test agent's hunger drives initial AcquireCommodity(Apple, SelfConsume) goal, which plans QueueForFacilityUse → Harvest at the blocked workstation. While waiting in queue, fatigue metabolism raises fatigue until Sleep motive overtakes hunger motive, triggering a goal switch. The S68 fix clears ContentionIntents, allowing prune_invalid_waiters to remove the stale queue entry.

### 2. Write the golden test

Follow the pattern of `run_dead_agent_pruned_from_facility_queue_scenario`:

1. Build scenario with agents, workstation, and beliefs
2. Run simulation ticks, observing queue state and active goal each tick
3. Track milestones: agent joined queue, goal changed while queued, agent pruned from queue
4. Break when all milestones observed or tick budget exhausted
5. Assert all milestones were hit
6. Assert simulation completed without DuplicateActor panic (implicit — would be a test panic)

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

1. New golden test: `golden_goal_switch_clears_contention_queue_entry` — simulation completes without `DuplicateActor` panic, stale queue entry pruned
2. New replay test: `golden_goal_switch_clears_contention_queue_entry_replays_deterministically` — same seed produces same hashes
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. After goal switch, the agent must not appear in the contention queue under the old goal's intended action
2. Simulation completes without DuplicateActor panic (implicit — a panic would fail the test)
3. Deterministic replay produces identical world and event log hashes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — `golden_goal_switch_clears_contention_queue_entry`: E2E proof that goal-switch contention cleanup prunes stale queue entries
2. `crates/worldwake-ai/tests/golden_production.rs` — `golden_goal_switch_clears_contention_queue_entry_replays_deterministically`: determinism guard

### Commands

1. `cargo test -p worldwake-ai golden_goal_switch_clears_contention_queue_entry`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-07.

**What changed**:
- Added Scenario 123 to `golden_production.rs`: "Goal Switch Clears Contention Queue Entry"
- Scenario: grant holder blocks exclusive OrchardFarm workstation. Hungry test agent queues via QueueForFacilityUse. Fast fatigue metabolism drives competing need to overtake hunger motive, causing goal switch. S68 intent-clear ensures prune_invalid_waiters removes stale queue entry. Simulation completes without DuplicateActor panic.
- Outcome struct: `GoalSwitchContentionCleanupOutcome` tracks 4 milestones (joined_queue, had_acquire_goal_while_queued, goal_changed_after_joining, pruned_from_queue) plus world/log hashes.
- Companion replay test confirms determinism.

**Deviations from original ticket**:
- Narrowed from TreatWounds → EscortToSafety (Care domain) to AcquireCommodity → competing need (Production domain). Reason: EscortToSafety ops do not include QueueForFacilityUse, making re-enqueue unprovable. The production domain is the most exercised contention domain and produces genuine P1 emergence from interacting systems (Needs metabolism + Production planning + Contention).
- Dropped "re-enqueue under new goal" acceptance criterion. The honest golden contract is: stale queue entry pruned after goal switch, simulation completes without DuplicateActor.

**Generated docs**: Regenerated via `python3 scripts/golden_inventory.py --write --check-docs` — 320 tests, 142 scenario blocks.

## Verification Result

- Passed `cargo test -p worldwake-ai golden_goal_switch_clears_contention_queue_entry` (2 tests)
- Passed `cargo test -p worldwake-ai --test golden_production` (45 tests)
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
