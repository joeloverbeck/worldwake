# GOLDENE2E-012: Death While Traveling

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Possible
**Deps**: GOLDENE2E-003 (Multi-Hop Travel Plan — needs multi-tick travel)

## Problem

What happens when an agent dies from deprivation after departing on a multi-hop food-seeking journey? This edge case was untested and could reveal bugs in cross-system sequencing, body placement on route, and item conservation before arrival.

**Coverage gap filled**:
- Cross-system chain: hunger-driven departure → route progress → metabolism → deprivation → wound accumulation → death before destination → concrete body placement on route → item conservation
- Tests death handling for an agent that has already committed to a multi-hop travel journey

## Assumption Reassessment (2026-03-13)

1. Deprivation escalation starts in the needs system, but death finalization does not. Needs advances `DeprivationExposure`; the combat system progresses wounds and attaches `DeadAt` once wound load exceeds `wound_capacity`.
2. `DeadAt` marks dead agents, and the existing harness helper `agent_is_dead()` already checks it directly.
3. Dead-action cleanup already exists in the tick loop. `step_tick()` aborts active actions for actors marked dead both before action progress and again after systems run, but that only applies if the actor is still inside an active action when death is finalized.
4. There is no separate corpse entity created by death. In current architecture, the dead agent itself becomes the lootable corpse target.
5. Multi-hop travel is modeled as one committed edge at a time with replanning at intermediate places. A natural starvation-on-route scenario does not necessarily die inside an active travel instance; it can die after departing, before destination, at a concrete intermediate place between legs.
6. Scenario 8 already proves per-tick coin conservation across death and loot. This ticket should extend that coverage specifically to death after departing on a multi-hop food-seeking journey rather than re-proving generic death behavior.

## Architecture Check

1. This test validates a real cross-system edge: needs-driven multi-hop travel can end in death before destination, and the simulation must resolve that to one consistent concrete place with no dangling state.
2. Depends on GOLDENE2E-003 establishing that multi-hop travel works. This ticket adds death after departure and before destination on that route, not another travel-planning test.
3. `golden_combat.rs` remains the correct home because the behavior under test is death handling and dead-actor cleanup, even though travel is part of the chain.

## What to Change

### 1. Write golden test: `golden_death_while_traveling`

In `golden_combat.rs`:

Setup:
- Fragile agent at Bandit Camp so reaching food requires a real multi-hop route before any harvest is possible.
- Use the same deprivation-victim profile pattern as Scenario 8, but tuned so the agent survives long enough to start the hunger-driven trip and then dies before arrival.
  - Very low `wound_capacity: pm(200)`
  - Existing starvation wound `severity: pm(150)`
  - Fast hunger metabolism with low `hunger_critical_ticks` in `DeprivationExposure`
  - Critical hunger high enough to force `AcquireCommodity` planning
- Food only available at distant Orchard Farm via a real `ResourceSource`.
- Give the agent coins to verify item conservation.
- Run simulation for up to 100 ticks.
- Assert: agent starts traveling from Bandit Camp and enters real in-transit state.
- Assert: death occurs before the agent reaches Orchard Farm.
- Assert: agent dies during the simulation (wound accumulation from deprivation).
- Assert: by the time death resolves, the dead agent has no lingering active action and is not in transit.
- Assert: the dead agent resolves to a concrete intermediate route place rather than the destination. In the deterministic golden scenario this is `ForestPath`.
- Assert: item conservation holds every tick (coins never created/destroyed).
- Assert: agent's death does not crash or cause action framework errors.

**Expected emergent chain**: Hunger drives travel to food → travel begins → metabolism continues across route progression → deprivation wounds accumulate before arrival → death at an intermediate place on the route → body remains concretely located there → items conserved.

### 2. Document resolved body-location rule

This is an implementation discovery item, but not the one the original ticket assumed. In the natural deterministic scenario, the agent dies after leaving the origin but before reaching the destination, while already grounded at the first intermediate route place. The test should assert that concrete body location and note that the dead agent remains the corpse target.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P12 from Part 3 to Part 1.
- Update cross-system interactions: "Death after departure on multi-hop travel" now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Looting the traveler after death. Scenario 8 already proves opportunistic corpse looting; this ticket is about death after departure and before destination on a hunger-driven journey.
- Death during non-travel actions
- Changing travel architecture to force continuous multi-edge travel without intermediate replanning
- Revival or resurrection mechanics
- Other agents reacting to the death

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

1. `golden_death_while_traveling` proves a fragile agent begins multi-hop travel and dies before arrival from deprivation-driven wound progression.
2. The agent enters real travel state (`is_in_transit` and/or active travel action observed after departure).
3. The agent dies during the simulation (`agent_is_dead()` returns true).
4. The dead agent no longer has an active action and no longer remains in transit after the death tick resolves.
5. The dead agent resolves to a concrete intermediate route place before Orchard Farm. In the deterministic scenario this is `ForestPath`.
6. Conservation: coin quantity is constant every tick.
7. Coverage report `reports/golden-e2e-coverage-analysis.md` updated.
8. Existing suite: `cargo test -p worldwake-ai --test golden_combat`
9. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for tracked commodity totals every tick
3. Determinism: same seed produces same outcome
4. No dangling active actions or lingering in-transit state for dead agents after the death tick resolves

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_death_while_traveling` — proves death before destination on a real multi-hop journey, with concrete route placement and cleanup assertions
2. `crates/worldwake-ai/tests/golden_combat.rs::golden_death_while_traveling_replays_deterministically` — proves the same journey/death sequence is deterministic for a fixed seed

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_death_while_traveling`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-13

**What actually changed**:
- Added `golden_death_while_traveling` to `crates/worldwake-ai/tests/golden_combat.rs`.
- Added `golden_death_while_traveling_replays_deterministically` as a focused replay companion test.
- Updated `reports/golden-e2e-coverage-analysis.md` to move the scenario from backlog to proven coverage and to reflect the new test counts.
- Reassessed the ticket assumptions to match the real travel/death architecture before implementation.

**Deviations from original plan**:
- No engine changes were required.
- The original ticket assumed death would occur during an active travel action and might resolve via travel-abort-to-origin semantics.
- The implemented deterministic scenario showed a cleaner and more natural architecture truth: multi-hop travel is per-leg with intermediate replanning, and the traveler dies after departure but before destination at `ForestPath`, already grounded between legs.
- Because of that, the completed assertions target concrete intermediate body placement and post-death cleanup, not forced mid-edge death.

**Verification results**:
- `cargo test -p worldwake-ai --test golden_combat golden_death_while_traveling`
- `cargo test -p worldwake-ai --test golden_combat`
- `cargo test --workspace`
- `cargo clippy --workspace`
