# S174SHESLESUR-011: Scenario E — survival-failed-rest-cascade.ron (feed for S175 collapse golden)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (golden scenario + test file only)
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, 004, 005, 006

## Problem

S174's Scenario E is the feed for S175's exhaustion-collapse golden. The scenario exercises a sustained pattern of failed-rest opportunities: a tired agent must repeatedly attempt and fail rest at a perpetually-occupied shelter, then fall back to rough-sleep at an open camp. Over N cycles, the agent accumulates ≥ N `FailedRestOpportunity` records in the active critical-fatigue window. `HomeostaticNeeds.fatigue` enters critical exposure; `DeprivationExposure.fatigue_critical_ticks` accumulates. S175's spec proves the collapse cascade on top of this scenario — S174 owns proving the carrier exists.

Without this scenario, the failed-rest accumulation chain has no E2E proof, and S175 has no shared scenario substrate to extend.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: `DeprivationExposure.fatigue_critical_ticks` at `crates/worldwake-core/src/needs.rs:120` is incremented by the needs tick when fatigue is at critical level. `FailedRestOpportunity` and `FailedRestKind` types are introduced by ticket 006. `CriticalWindowFrame.failed_rest_opportunities` is the consumer surface.
2. Spec assumption verified against S174 Scenario E. The scenario uses one `shelter` with `RestCapacity(1)` perpetually occupied by a non-cooperating agent (e.g., a sleeping invalid actor or a perpetually-sleeping NPC) and one adjacent `open_field` supporting only rough sleep. The tired agent fails at `shelter`, falls back to rough-sleep, capped recovery is insufficient to prevent fatigue critical exposure accumulation.
3. Shared abstraction boundary under audit: the `FailedRestOpportunity` aggregation across cycles. Per ticket 006's three populating paths: (a) sleep aborts mid-episode produce `Interrupted { cause }`, (b) sleep start failures produce `PreconditionRejected`, (c) preempted-by-higher-need is deferred per ticket 006 Assumption 5. This scenario primarily exercises (b) — the shelter is always full, so the tired agent's KnownRestSite emissions consistently fail at start.
4. Live `GoalKind` under test: `GoalKind::Sleep`. The scenario doesn't terminate the agent (S175 owns the collapse and death path); it terminates when N `FailedRestOpportunity` records have accumulated (e.g., N = 5).
5. Cumulative arithmetic: the rough-sleep recovery cap (`Permille::new(300)` ≈ 0.3x of `MetabolismProfile.rest_efficiency`) yields per-tick fatigue reduction insufficient to offset the metabolism's `fatigue_rate` accumulation between sleep attempts. Pick metabolism values such that: rest_efficiency × 0.3 < fatigue_rate × (interrupt_interval / sleep_interval), so each rough-sleep cycle leaves the agent's net fatigue trending upward. Verify the math at ticket-implementation time by running the scenario and observing `fatigue_critical_ticks` accumulation.
6. Scenario isolation: the intended branch under test is `FailedRestOpportunity` accumulation across cycles. Excluded: starvation/dehydration depletion (agent must be near-sated); the perpetually-sleeping `Sleeper` NPC must not interact with the tired agent socially.
7. Mismatch + correction: the scenario relies on a "perpetually-sleeping invalid actor" — the cleanest authoring pattern is an agent with very low `MetabolismProfile.rest_efficiency` so it sleeps essentially forever, or an explicit scenario directive that holds the actor's Sleep state via a long `target_recovery`. Verify at ticket-implementation time which mechanism the existing harness supports.

## Architecture Check

1. The scenario produces failed-rest opportunities through ordinary world processes (shelter capacity full + rough-sleep cap insufficient) — no hidden script or director. This is the canonical FND-1 maximal emergence test for the rest model.
2. Reusing ticket 006's `FailedRestOpportunity` records (rather than introducing a new aggregator) preserves the single-truth contract for forensic data. S175 reads the same records.
3. The terminating condition (N records accumulated) is a scenario-imposed bound, not an architectural cap. S175 will extend the same scenario shape to provoke collapse-via-wound; this scenario stops short of that and just proves the feed.

## Verification Layers

1. Over a horizon of M ticks (e.g., 100), the tired agent emits ≥ N (e.g., 5) KnownRestSite candidates targeting `shelter` -> decision trace assertion
2. All N candidates fail the precondition gate (`Err(ActionError)` at start, action trace) -> action trace aggregate assertion
3. Each precondition rejection appends a `FailedRestOpportunity { tick, place: shelter, kind: PreconditionRejected, was_rough: false }` to the active critical-fatigue window -> `CriticalWindowFrame.failed_rest_opportunities` length assertion (≥ N entries)
4. Between failed attempts, the agent rough-sleeps at `open_field` (RoughSleep candidate emitted, start succeeds without `RestOccupancy` write) -> decision trace + event-log assertion
5. Rough-sleep accumulation is bounded by `rough_sleep_recovery_floor` — net fatigue trends upward despite repeated rough-sleep -> authoritative `HomeostaticNeeds.fatigue` trajectory assertion
6. `DeprivationExposure.fatigue_critical_ticks` accumulates monotonically across the scenario -> authoritative state trajectory assertion
7. No hidden rescue: no event in the log corresponds to a fatigue refill from outside the rough-sleep recovery system -> event-log audit assertion (negative branch)
8. Deterministic replay -> identical state hashes

## What to Change

### 1. Author the scenario RON file

Create `scenarios/survival-failed-rest-cascade.ron` with:

- Two places: `shelter` (with `SleepQualityProfile { shelter: Roofed, recovery_modifier: 1100 }`, `rest_capacity: Some(1)`) and `open_field` (no SleepQualityProfile, no rest_capacity)
- An edge connecting them with short travel time (e.g., 3-5 ticks)
- Two agents:
  - `Sleeper`: spawns at `shelter`; metabolism configured to sleep indefinitely (very low `rest_efficiency` OR very high `target_recovery` OR equivalent harness directive)
  - `Aster`: tired agent at `open_field`; metabolism configured so rough-sleep accumulation cannot offset fatigue accumulation
- Stable seed
- Hostile relationship: none (the scenario isolates failure-attribution, not danger)

### 2. Author the corresponding test file

Create `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs`. Assertions per the 8 verification layers. The test runs for M ticks (e.g., 100-200) and asserts ≥ N `FailedRestOpportunity` records (e.g., 5).

### 3. Hook the test

Add `mod survival_failed_rest_cascade;` to `tests/scenarios/mod.rs`.

### 4. Outcome / S175 handoff

In the test file's comment block, explicitly document: "This scenario is the feed for S175's exhaustion-collapse golden. S175 will extend the same scenario shape to provoke collapse-via-wound and eventual death." This handoff prevents future readers from treating the failed-rest accumulation as an end in itself.

## Files to Touch

- `scenarios/survival-failed-rest-cascade.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `mod survival_failed_rest_cascade;`)

## Out of Scope

- No collapse / wound creation / death path (S175 spec territory)
- No `FailedRestKind::PreemptedByHigherNeed` exercise (ticket 006 deferred path (c); if path (c) lands within ticket 006, an additional assertion could cover it, but the scenario does not depend on it)
- No hostile interruption — that's Scenario C / ticket 009
- No CLI surface — that's Scenario D / ticket 010
- No production code changes

## Acceptance Criteria

### Tests That Must Pass

1. New scenario test `survival_failed_rest_cascade::scenario_e_failed_rest_feed` passes all 8 verification-layer assertions
2. Deterministic replay test passes
3. Existing suite: `cargo test --workspace` passes

### Invariants

1. `CriticalWindowFrame.failed_rest_opportunities.len() >= 5` after the scenario completes
2. All recorded `FailedRestOpportunity` entries have `kind: PreconditionRejected` (this scenario does not exercise other kinds — that's verified in Scenario C / ticket 009)
3. `DeprivationExposure.fatigue_critical_ticks` is strictly monotonic non-decreasing throughout the scenario
4. No fatigue refill outside the rough-sleep recovery system — no hidden rescue path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs` (new) — Scenario E E2E feed for S175

### Commands

1. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_failed_rest_cascade`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
