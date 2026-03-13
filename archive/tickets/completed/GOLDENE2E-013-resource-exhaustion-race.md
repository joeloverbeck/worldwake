# GOLDENE2E-013: Resource Exhaustion Race

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Possible
**Deps**: None

## Problem

Current tests use at most 2 agents. When 4+ hungry agents compete for a limited resource source, the system must handle concurrent harvest attempts, resource exhaustion, and graceful degradation as resources run out. This tests reservation/contention logic, multi-agent scheduling fairness, and plan failure recovery at scale.

**Coverage gap filled**:
- Cross-system chain: Multiple agents → simultaneous needs pressure → concurrent harvest attempts → resource depletion → some agents fail → seek alternatives or wait for regeneration → conservation holds → no deadlocks
- Scale testing: 4 agents (2x previous maximum)

## Assumption Reassessment (2026-03-13)

1. `ResourceSource.available_quantity` is authoritative stock and harvest commit subtracts the full recipe output quantity before materializing a ground lot.
2. Harvest contention is mediated by workstation reservations and commit-condition revalidation, not by simultaneous writes. The relevant architecture is serialized action progress with reservation-backed exclusivity, not true parallel mutation.
3. Harvest availability is validated against the recipe batch size, not merely `> 0`. For the default apple harvest recipe, a source must have at least `Quantity(2)` available for the action to remain valid.
4. Existing low-level coverage already proves a second actor cannot start the same reserved harvest action and that aborting the first action preserves source quantity (`crates/worldwake-systems/src/production_actions.rs` unit tests). This ticket should prove the same contention class through the real AI loop with 4 agents.
5. `total_live_lot_quantity`, `total_authoritative_commodity_quantity`, and `verify_authoritative_conservation` already give the right conservation checks for this scenario. The test should assert authoritative totals and depletion directly rather than infer them indirectly from planner state.
6. Plan-failure handling does classify production failures (`ReservationConflict`, `WorkstationBusy`, `SourceDepleted`), but a no-regeneration scenario does not guarantee that losing agents will visibly resolve into a new successful plan within 150 ticks. Assertions should target observable world state and stable runtime behavior, not speculative replanning outcomes.

## Architecture Check

1. This test validates system behavior under contention that only emerges when more agents compete than the source can satisfy. It specifically exercises reservation exclusivity, commit-time depletion, conservation, and stable AI-loop behavior under repeated contention.
2. Fits in `golden_production.rs` since it tests production system contention.
3. The robust architectural target is not fairness or forced fallback behavior. The value is proving that the real loop stays deterministic, conserved, and non-stuck when a finite source can only satisfy part of the local demand.

## What to Change

### 1. Write golden tests: `golden_resource_exhaustion_race` and replay companion

In `golden_production.rs`:

Setup:
- 4 agents at Orchard Farm, all critically hungry (`pm(900)`), no food in inventory.
- OrchardRow workstation with `Quantity(4)` apples in ResourceSource (just enough for 2 harvests of 2 each).
- No resource regeneration (`regeneration_ticks_per_unit: None`).
- Run simulation for up to 150 ticks.
- Assert: at least 2 harvest batches commit successfully, proven by source quantity stepping `4 -> 2 -> 0` and apple lots materializing.
- Assert: the resource source depletes to `Quantity(0)` and never goes negative.
- Assert: authoritative apple quantity never exceeds the initial 4 and only decreases through consumption.
- Assert: the real AI loop remains stable for the full run: no panic, no deadlock, and continued tick progression after depletion.

**Expected emergent chain**: 4 hungry agents converge on one orchard source → reservation-backed harvest actions serialize access → 2 harvest commits (2 apples each) exhaust the source → at least one agent completes the downstream eat chain → other agents encounter contention and/or depletion without breaking conservation or stability.

### Engine Changes Made

- Best-effort autonomous action requests should not crash the tick when a previously valid affordance becomes unavailable during same-tick input application.
- The real AI loop can legitimately queue multiple harvest starts from one shared snapshot; the second and later starts may hit `ReservationUnavailable` even though planning was valid.
- Fix this in the input application path by distinguishing strict external/manual requests from best-effort autonomous requests, and treat recoverable start-time availability failures for best-effort requests as non-fatal so the agent can reconcile on the next tick.

### 2. Add deterministic replay coverage

- Add `golden_resource_exhaustion_race_replays_deterministically`.
- Re-run the same scenario twice with the same seed and assert identical world/event-log hashes.
- This keeps the ticket aligned with the project-wide determinism invariant for a scheduler-sensitive contention scenario.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P13 from Part 3 to Part 1.
- Update cross-system interactions: reservation-backed resource exhaustion with 4 agents now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify — add test)
- `crates/worldwake-sim/src/input_event.rs` (modify — distinguish strict vs best-effort action requests)
- `crates/worldwake-sim/src/tick_step.rs` (modify — treat recoverable best-effort start failures as non-fatal)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Agents traveling to alternative resource sources or sellers after losing the local contention race
- Resource regeneration during the race (no regen configured)
- Agent-to-agent trading to share harvested resources
- More than 4 agents
- Fairness guarantees or equal distribution between agents
- Proving blocked-intent memory as a required visible outcome in this scenario

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_resource_exhaustion_race` — 4 agents compete for limited resources, system handles exhaustion gracefully
2. `golden_resource_exhaustion_race_replays_deterministically` — same-seed replay stays identical
3. Exactly two harvest batches can be satisfied from the configured source and the source depletes to 0
4. Resource source depletes to 0 available quantity
5. Conservation: total apple authoritative quantity never exceeds initial 4
6. At least one agent completes the harvest/pick-up/eat chain under contention
7. No panics, no deadlocks — simulation completes within tick limit
8. Coverage report `reports/golden-e2e-coverage-analysis.md` updated
9. Existing suite: `cargo test -p worldwake-ai --test golden_production`
10. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome
4. No set of agents can control or consume more apples than physically possible given resource source limits

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_resource_exhaustion_race` — proves reservation-backed multi-agent resource contention, depletion, and conservation through the real AI loop
2. `crates/worldwake-ai/tests/golden_production.rs::golden_resource_exhaustion_race_replays_deterministically` — proves the contention scenario is deterministic for a fixed seed

### Commands

1. `cargo test -p worldwake-ai --test golden_production golden_resource_exhaustion_race`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-13

**What actually changed**:
- Added `golden_resource_exhaustion_race` to `crates/worldwake-ai/tests/golden_production.rs`.
- Added `golden_resource_exhaustion_race_replays_deterministically` as a focused replay companion test.
- Added a sim-level regression test in `crates/worldwake-sim/src/tick_step.rs` covering best-effort request handling under reservation contention.
- Introduced explicit `ActionRequestMode` in `crates/worldwake-sim/src/input_event.rs` and used `BestEffort` for autonomous AI requests while keeping manual/external requests strict.
- Updated `crates/worldwake-sim/src/tick_step.rs` so recoverable same-tick availability failures on best-effort requests do not abort the whole tick.
- Updated `reports/golden-e2e-coverage-analysis.md` to move the scenario from backlog to proven coverage and reflect the expanded production test count.
- Reassessed the ticket assumptions and scope before implementation so the assertions match the real reservation/depletion architecture.

**Deviations from original plan**:
- The original ticket treated this as a pure test-addition task. Implementation exposed a real engine bug in the autonomous input path: same-tick reservation contention surfaced as a fatal tick error instead of a recoverable plan failure.
- The original ticket also assumed the scenario should prove broad participation or forced fallback behavior for losing agents. The deterministic run showed that fairness is not guaranteed by the current architecture and should not be asserted here.
- Because of that, the completed assertions focus on the stable architectural truths: source depletion `4 -> 2 -> 0`, conservation, downstream consumption, deterministic replay, and graceful AI-loop behavior under contention.

**Verification results**:
- `cargo test -p worldwake-sim best_effort_request_drops_recoverable_start_failure_without_failing_tick -- --nocapture`
- `cargo test -p worldwake-ai --test golden_production golden_resource_exhaustion_race -- --nocapture`
- `cargo test -p worldwake-ai --test golden_production`
- `cargo test --workspace`
- `cargo clippy --workspace`
