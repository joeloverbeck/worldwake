# S174SHESLESUR-011: Scenario E — survival-failed-rest-cascade.ron (feed for S175 collapse golden)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend the S174 failed-rest forensic carrier to record repeated rough-sleep fallback from a known rest-site opportunity, then land the Scenario E golden.
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, `archive/tickets/S174SHESLESUR-004.md`, `archive/tickets/S174SHESLESUR-005.md`, `archive/tickets/S174SHESLESUR-006.md`

## Problem

S174's Scenario E is the feed for S175's exhaustion-collapse golden. The scenario exercises a sustained pattern of failed-rest opportunities: a tired agent must repeatedly attempt and fail rest at a perpetually-occupied shelter, then fall back to rough-sleep at an open camp. Over N cycles, the agent accumulates ≥ N `FailedRestOpportunity` records in the active critical-fatigue window. `HomeostaticNeeds.fatigue` enters critical exposure; `DeprivationExposure.fatigue_critical_ticks` accumulates. S175's spec proves the collapse cascade on top of this scenario — S174 owns proving the carrier exists.

Without this scenario, the failed-rest accumulation chain has no E2E proof, and S175 has no shared scenario substrate to extend.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: `DeprivationExposure.fatigue_critical_ticks` at `crates/worldwake-core/src/needs.rs:120` is incremented by the needs tick when fatigue is at critical level. `FailedRestOpportunity` and `FailedRestKind` types are introduced by archived ticket 006. `CriticalWindowFrame.failed_rest_opportunities` is the consumer surface.
2. Spec assumption corrected against live planner behavior and `docs/FOUNDATIONS.md`. The scenario uses one `shelter` with `RestCapacity(1)` occupied by a long-sleeping non-cooperating agent and a tired agent that can only fall back to rough sleep at the same place. The first contested start can fail with `PreconditionRejected`; later cycles must not force the agent to repeat a known-impossible start. FND-20/FND-21 require the agent to revise the broken rest-site intention and select lawful rough sleep while the rest-site opportunity remains known.
3. Shared abstraction boundary under audit: the `FailedRestOpportunity` aggregation across cycles. Per archived ticket 006's three populating paths: (a) sleep aborts mid-episode produce `Interrupted { cause }`, (b) sleep start failures produce `PreconditionRejected`, (c) preempted-by-higher-need produces `PreemptedByHigherNeed`. Live Scenario E needs a fourth derived-forensic path: during an active fatigue-critical window, a Sleep decision that emits a known rest-site opportunity but selects targetless rough sleep records a failed rest-site opportunity for the known place. This preserves S175's carrier without making the planner irrational.
4. Live `GoalKind` under test: `GoalKind::Sleep`. The scenario doesn't terminate the agent (S175 owns the collapse and death path); it terminates when N `FailedRestOpportunity` records have accumulated (e.g., N = 5).
5. Cumulative arithmetic: the rough-sleep recovery cap (`Permille::new(300)` ≈ 0.3x of `MetabolismProfile.rest_efficiency`) yields per-tick fatigue reduction insufficient to offset the metabolism's `fatigue_rate` accumulation between sleep attempts. Pick metabolism values such that: rest_efficiency × 0.3 < fatigue_rate × (interrupt_interval / sleep_interval), so each rough-sleep cycle leaves the agent's net fatigue trending upward. Verify the math at ticket-implementation time by running the scenario and observing `fatigue_critical_ticks` accumulation.
6. Scenario isolation: the intended branch under test is `FailedRestOpportunity` accumulation across cycles. Excluded: starvation/dehydration depletion (agent must be near-sated); the perpetually-sleeping `Sleeper` NPC must not interact with the tired agent socially.
7. Mismatch + correction: the original ticket claimed `Engine Changes: None` and required ≥ N repeated `PreconditionRejected` start failures. Focused probing showed that Aster emits repeated KnownRestSite candidates but, after the first failed start, selects targetless rough sleep on later cycles. Forcing repeated known-full starts would violate FND-20/FND-21. The implementation must instead extend `FailedRestKind` with a truthful rough-fallback/known-rest-site-unavailable variant and assert that repeated records use that carrier.

## Architecture Check

1. The scenario produces failed-rest opportunities through ordinary world processes (shelter capacity full + rough-sleep cap insufficient) — no hidden script or director. This is the canonical FND-1 maximal emergence test for the rest model.
2. Reusing archived ticket 006's `FailedRestOpportunity` records (rather than introducing a new aggregator) preserves the single-truth contract for forensic data. S175 reads the same records.
3. The new rough-fallback failed-rest kind is derived forensic state, not authoritative world state. It is reconstructed from decision trace evidence: the fatigue-critical actor generated a `GoalKind::Sleep` opportunity anchored to a known rest-site place, but the selected sleep opportunity was targetless rough sleep.
4. The terminating condition (N records accumulated) is a scenario-imposed bound, not an architectural cap. S175 will extend the same scenario shape to provoke collapse-via-wound; this scenario stops short of that and just proves the feed.

## Verification Layers

1. Over a horizon of M ticks (e.g., 100), the tired agent emits ≥ N (e.g., 5) KnownRestSite candidates targeting `shelter` -> decision trace assertion
2. The first contested known-rest-site attempt can fail the precondition gate (`Err(ActionError)` at start, action trace) -> action trace assertion
3. Subsequent cycles append `FailedRestOpportunity { tick, place: shelter, kind: RoughFallbackToKnownRestSite, was_rough: true }` when the active fatigue-critical decision emits the known rest-site candidate but selects targetless rough sleep -> `CriticalWindowFrame.failed_rest_opportunities` length assertion (≥ N entries)
4. Between failed opportunities, the agent rough-sleeps targetlessly at the current place (RoughSleep candidate emitted, start succeeds without `RestOccupancy` write) -> decision trace + event-log assertion
5. Rough-sleep accumulation is bounded by `rough_sleep_recovery_floor` — net fatigue trends upward despite repeated rough-sleep -> authoritative `HomeostaticNeeds.fatigue` trajectory assertion
6. `DeprivationExposure.fatigue_critical_ticks` accumulates monotonically across the scenario -> authoritative state trajectory assertion
7. No hidden rescue: no event in the log corresponds to a fatigue refill from outside the rough-sleep recovery system -> event-log audit assertion (negative branch)
8. Deterministic replay -> identical state hashes

## What to Change

### 1. Author the scenario RON file

Create `scenarios/survival-failed-rest-cascade.ron` with:

- One place: `shelter` (with `SleepQualityProfile { shelter: Roofed, recovery_modifier: 1100 }`, `rest_capacity: Some(1)`). Targetless rough sleep at the current place is the lawful fallback and writes no `RestOccupancy`.
- Two agents:
  - `Sleeper`: spawns at `shelter`; metabolism configured for a long sleep episode that holds the one rest slot during Aster's failed-rest cycles.
  - `Aster`: tired agent at `shelter`; metabolism configured so capped rough-sleep recovery cannot leave fatigue critical exposure.
- Stable seed
- Hostile relationship: none (the scenario isolates failure-attribution, not danger)

### 2. Author the corresponding test file

Create `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs`. Assertions per the 8 verification layers. The test runs for M ticks (e.g., 100-200) and asserts ≥ N `FailedRestOpportunity` records (e.g., 5), with the repeated records coming from the rough-fallback known-rest-site-unavailable path rather than repeated start failures.

### 3. Hook the test

Add `mod survival_failed_rest_cascade;` to `tests/scenarios/mod.rs`.

### 4. Outcome / S175 handoff

In the test file's comment block, explicitly document: "This scenario is the feed for S175's exhaustion-collapse golden. S175 will extend the same scenario shape to provoke collapse-via-wound and eventual death." This handoff prevents future readers from treating the failed-rest accumulation as an end in itself.

## Files to Touch

- `scenarios/survival-failed-rest-cascade.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `mod survival_failed_rest_cascade;`)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — add and populate rough-fallback failed-rest kind)

## Out of Scope

- No collapse / wound creation / death path (S175 spec territory)
- No `FailedRestKind::PreemptedByHigherNeed` exercise — archived ticket 006 covers that path with focused forensics tests, and this scenario intentionally concentrates on repeated rest-site precondition rejection.
- No hostile interruption — that's Scenario C / ticket 009
- No CLI surface — that's Scenario D / ticket 010
- No authored/scripted external action requests

## Acceptance Criteria

### Tests That Must Pass

1. New scenario test `survival_failed_rest_cascade::scenario_e_failed_rest_feed` passes all 8 verification-layer assertions
2. Deterministic replay test passes
3. Existing suite: `cargo test --workspace` passes

### Invariants

1. `CriticalWindowFrame.failed_rest_opportunities.len() >= 5` after the scenario completes
2. Recorded `FailedRestOpportunity` entries include the initial `PreconditionRejected` if the one-slot race loses at start, and at least five rough-fallback known-rest-site-unavailable records. No hostile `Interrupted` or `PreemptedByHigherNeed` records appear in this scenario.
3. `DeprivationExposure.fatigue_critical_ticks` is strictly monotonic non-decreasing throughout the scenario
4. No fatigue refill outside the rough-sleep recovery system — no hidden rescue path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs` (new) — Scenario E E2E feed for S175

### Commands

1. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_failed_rest_cascade`
2. `cargo test --workspace`
3. `./scripts/verify.sh`

## Outcome

Completed: 2026-05-26

What changed:
- Added `FailedRestKind::RoughFallbackToKnownRestSite` and populated it from decision-trace evidence when a fatigue-critical actor emits a known-rest-site Sleep opportunity but selects targetless rough sleep.
- Added `scenarios/survival-failed-rest-cascade.ron` with a one-slot occupied shelter and a critically tired Aster whose rough-sleep fallback cannot leave fatigue critical exposure.
- Added `crates/worldwake-ai/tests/scenarios/survival_failed_rest_cascade.rs` with Scenario 485 coverage and deterministic replay.
- Regenerated generated golden coverage docs and synchronized `specs/S174-shelter-sleep-surfaces-safe-rest.md`.

Deviations from original plan:
- The original ticket required ≥ N repeated `PreconditionRejected` start failures and claimed no engine changes. Reassessment against live behavior and `docs/FOUNDATIONS.md` showed that forcing repeated known-full starts would violate FND-20/FND-21. The landed carrier records repeated rough fallback from a known rest-site opportunity instead.
- Scenario E uses targetless rough sleep at the occupied shelter's current place rather than a separate `open_field`, because current Sleep semantics make targetless sleep the rough fallback and keep `RestOccupancy` untouched.

Verification:
- `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_failed_rest_cascade`
- `cargo test -p worldwake-ai survival_forensics`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest scenarios::survival_sleep_contention scenarios::survival_rest_interrupted_by_danger scenarios::survival_failed_rest_cascade`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check`
