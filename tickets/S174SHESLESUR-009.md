# S174SHESLESUR-009: Scenario C — survival-rest-interrupted-by-danger.ron (HostileProximity wake cause)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (golden scenario + test file only); requires `archive/tickets/S174SHESLESUR-004.md` to map hostile-proximity abort to `SleepFailureCause::HostileProximity`
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, `archive/tickets/S174SHESLESUR-004.md`, 005, 006

## Problem

S174's Scenario C proves structured-cause interruption — when a hostile actor enters a sleeping agent's place mid-episode, the sleep aborts with `WakeReason::LocalDisturbance { cause: SleepFailureCause::HostileProximity }`. The `ActionTraceDetail::SleepInterrupted` payload carries the same cause + `accumulated_recovery` + `was_rough_sleep`. Partial recovery is preserved per S128's contract. The agent's next tick emits a different goal (flee/fight) via ordinary replan. Without this scenario, the structured wake-cause taxonomy is not exercised end-to-end.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: hostile-proximity sleep interruption is triggered by the existing interrupt/abort substrate when a hostile actor enters the sleeper's place. Per S173's `SelfCareOccupancy` and S174's `RestOccupancy` semantics, the sleep handler's abort path is invoked via `abort_sleep_episode` (`needs_actions.rs:667-682`). `archive/tickets/S174SHESLESUR-004.md` refined the cause mapping so the hostile-proximity branch supplies `SleepFailureCause::HostileProximity` rather than the transitional `Generic`.
2. Spec assumption verified against S174 Scenario C. The scenario uses one place (`shelter` with `RestCapacity(1)`) and an adjacent `outpost` that hosts a hostile agent. The hostile travels toward `shelter` mid-sleep. Assertions: sleep aborts mid-episode; `WakeReason::LocalDisturbance { cause: HostileProximity }` fires; `ActionTraceDetail::SleepInterrupted` populates; `RestOccupancy` releases; partial recovery preserved; agent's next tick emits a different goal.
3. Shared abstraction boundary under audit: the interrupt/abort path's classification of hostile-proximity events. Verify `archive/tickets/S174SHESLESUR-004.md` wires the abort handler to read the abort reason and supply the correct `SleepFailureCause`.
4. Live `GoalKind` under test: starts with `GoalKind::Sleep`; after interruption, the agent replans (likely to `GoalKind::Flee` or `GoalKind::Engage` depending on combat profile). The scenario doesn't strictly require the post-replan branch to be deterministic — only that the agent does NOT immediately re-attempt Sleep at the same hostile-occupied place.
5. Cumulative arithmetic: the sleep accumulates ~10-20 ticks of recovery before the hostile arrives (depending on edge travel_time and seed). The partial-recovery assertion checks `accumulated_recovery > 0 && accumulated_recovery < target_recovery`.
6. Scenario isolation: the intended branch under test is `SleepFailureCause::HostileProximity` wake cause routing + partial recovery preservation. Excluded: starvation/dehydration depletion (agent must be near-sated for non-fatigue needs); other hostiles (only one hostile actor).
7. Hostile actor scenario authoring: this requires a non-trivial agent with `CombatProfile` + a hostile relationship toward `Aster`. Existing scenarios with hostile actors include `survival-combat.ron` and `survival-theft.ron` — use those as templates for the hostile-agent authoring shape.

## Architecture Check

1. The HostileProximity cause maps to a specific abort-handler branch (`archive/tickets/S174SHESLESUR-004.md`'s refinement of `abort_sleep_episode`'s `SleepFailureCause` supply). Per FND-28, the cause taxonomy is a single structured surface — not an ad-hoc string description threaded through the abort path.
2. Partial recovery preservation is a S128 contract — `SleepEpisode.accumulated_recovery` survives the abort and persists into `HomeostaticNeeds.fatigue`. This scenario exercises that contract under the new structured-cause abort path.
3. Asserting on the action trace AND event log (`EventTag::SleepEpisodeEnded` payload) AND `CriticalWindowFrame.failed_rest_opportunities` ensures the proof is layer-strong rather than narrative-only.

## Verification Layers

1. Sleep starts at `shelter`; `RestOccupancy.occupants = {tired_agent}` -> event-log delta + authoritative world state
2. Hostile arrives at `shelter` mid-sleep; abort fires -> action trace (`ActionTraceDetail::SleepInterrupted` event)
3. `WakeReason::LocalDisturbance { cause: SleepFailureCause::HostileProximity }` in the `SleepEpisodeEnded` payload -> event-log assertion
4. `ActionTraceDetail::SleepInterrupted { place, cause: HostileProximity, accumulated_recovery: <Permille>, was_rough_sleep: false }` -> action-trace assertion
5. `RestOccupancy.occupants` no longer contains the tired agent after abort -> authoritative world state query
6. `HomeostaticNeeds.fatigue` reduced by `accumulated_recovery` (partial recovery preserved) -> authoritative state assertion
7. `FailedRestOpportunity { tick, place: shelter, kind: Interrupted { cause: HostileProximity }, was_rough: false }` appended to the active critical-fatigue window's frame -> `CriticalWindowFrame.failed_rest_opportunities` query
8. Agent's next tick does NOT emit another Sleep candidate targeting `shelter` (hostile-occupied) — replans to flee/fight/move-elsewhere -> decision trace assertion
9. Deterministic replay -> identical state hashes

## What to Change

### 1. Author the scenario RON file

Create `scenarios/survival-rest-interrupted-by-danger.ron` with:

- Two places: `shelter` (with `SleepQualityProfile { shelter: Roofed, ground_comfort: Soft, recovery_modifier: 1100 }`, `rest_capacity: Some(1)`) and `outpost` (where the hostile starts)
- One edge connecting them with a short `travel_time_ticks` (e.g., 5-8 ticks) so the hostile arrives mid-sleep
- Two agents: `Aster` (tired, peaceful, target_recovery ≈ 50 ticks of accumulation) at `shelter`; `Marauder` (hostile combat profile, programmed to travel to `shelter`) at `outpost`
- Hostile relationship between `Marauder` and `Aster` per the existing combat scenario authoring pattern
- Stable seed
- `MetabolismProfile.rough_sleep_recovery_floor` default

Follow `survival-combat.ron` and `survival-theft.ron` as templates for the hostile-agent authoring shape.

### 2. Author the corresponding test file

Create `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs`. Assertions:

- Aster's Sleep starts on tick 0-2 (first emit cycle after spawn)
- Marauder begins traveling on tick 0
- Marauder arrives at `shelter` mid-sleep (around tick 5-8)
- Sleep aborts; the assertion checks all 9 verification-layer items above

### 3. Hook the test

Add `mod survival_rest_interrupted_by_danger;` to `tests/scenarios/mod.rs`.

## Files to Touch

- `scenarios/survival-rest-interrupted-by-danger.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `mod survival_rest_interrupted_by_danger;`)

## Out of Scope

- Other wake-cause variants (`RestSiteContended`, `SurfaceInvalidated`, `ActorIncapacitated`) — scope creep; these are exercised by other variants or future scenarios
- Predator ecology (S61 territory)
- Combat scenario design (existing `survival-combat.ron` covers that; this scenario uses the existing combat substrate as a trigger, not as its primary contract)
- No production code changes

## Acceptance Criteria

### Tests That Must Pass

1. New scenario test `survival_rest_interrupted_by_danger::scenario_c_hostile_proximity_wake` passes all 9 verification-layer assertions
2. Deterministic replay test passes
3. Existing suite: `cargo test --workspace` passes

### Invariants

1. `WakeReason::LocalDisturbance { cause: HostileProximity }` is the only cause variant emitted in this scenario
2. Partial recovery preserved — `accumulated_recovery > 0 && accumulated_recovery < target_recovery` at abort tick
3. The agent does not immediately re-attempt Sleep at the hostile-occupied shelter — replan must reflect updated belief state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs` (new) — Scenario C E2E

### Commands

1. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_rest_interrupted_by_danger`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
