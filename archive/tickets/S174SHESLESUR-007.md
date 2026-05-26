# S174SHESLESUR-007: Scenario A — survival-safe-rest.ron (rest-site contention golden)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — golden scenario plus a narrow AI planner fix for place-anchored Sleep target materialization and next-step revalidation.
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, `archive/tickets/S174SHESLESUR-004.md`, `archive/tickets/S174SHESLESUR-005.md`, `archive/tickets/S174SHESLESUR-006.md`

## Problem

S174's Scenario A is the first collision proof for the rest-site contention model. The scenario must demonstrate: two tired agents co-located at a shelter with `RestCapacity(1)`, one occupies the shelter, the other either rough-sleeps at the open camp or rough-sleeps at the same place (depending on Open Question #1 resolution at ticket-implementation time). Recovery modifier comparison proves rough sleep accumulates at the capped `rough_sleep_recovery_floor` while shelter sleep accumulates at the place's `SleepQualityProfile.recovery_modifier`. Without this scenario, the rest-site contention model has no E2E proof.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: scenario files live under `scenarios/*.ron`; their corresponding test harness files live under `crates/worldwake-ai/tests/scenarios/<name>.rs` and use `GoldenHarness` (path: `crates/worldwake-ai/tests/golden_harness/mod.rs`). Existing precedent scenarios: `survival-baseline.ron`, `survival-contested.ron`, `survival-trade.ron`, `survival-theft.ron`. The harness pattern: load the scenario via `worldwake_cli::scenario::load_scenario_file`, spawn, then step ticks via `step_tick` while asserting on event log + action trace + decision trace + authoritative state.
2. Spec assumption verified against S174 Scenario A. The scenario uses two places (`shelter_north` with `RestCapacity(1)` + `SleepQualityProfile { shelter: Roofed, recovery_modifier: 1100 permille }`; `open_camp` with no `SleepQualityProfile`) and two tired agents. Per spec, the assertions include candidate emission via FND-14A direct observation, occupancy write/release, recovery modifier comparison, and `FailedRestOpportunity::PreconditionRejected` recording.
3. Shared abstraction boundary under audit: scenario-level E2E composition of the rest-site model — exercises archived tickets 001-006 (foundation types, scenario contract, belief-view, handler RestOccupancy lifecycle, goal schema two-path, forensics records). Any failure in this golden likely indicates a regression in one of the upstream tickets.
4. Live `GoalKind` under test: `GoalKind::Sleep`. Current operator surface (post-ticket-005): `DECL_SLEEP.relevant_ops = [Sleep, QueueForFacilityUse]` and `DECL_SLEEP.progress_barrier_ops = [Sleep]`. The scenario primarily exercises `PlannerOpKind::Sleep` (queue-based promotion is exercised in Scenario B / ticket 008).
5. Cumulative arithmetic: per spec, the shelter occupant accumulates recovery at ≈ 1.1x while the rough sleeper accumulates at ≈ 0.3x (= `rough_sleep_recovery_floor`). Recovery delta per tick is determined by `MetabolismProfile.rest_efficiency` multiplied by the recovery modifier. Pick agent metabolisms such that 30-50 ticks of sleep are sufficient to observe a clear recovery delta between the two paths.
6. Scenario isolation: the intended branch under test is rest-site contention + recovery modifier divergence. Lawful competing affordances the architecture would permit but this scenario excludes: other survival actions (eat, drink, wash, relieve) — the agents must be near-sated for all non-fatigue needs so the only meaningful goal is Sleep.
7. Existing inline test conventions to mirror: per `tests/scenarios/sleep_episode.rs:1`, scenarios use `// Scenario N:` comment headers with sequentially chosen numbers that do not collide with existing tests. The landed test uses Scenario 481.
8. Scope correction: live planner behavior contradicted the draft's "no production code changes" assumption. Both agents saw the KnownRestSite opportunity, but the selected `sleep` step materialized as targetless rough sleep because the `sleep` action has no declared target spec. The ticket therefore required a production AI fix, not only a golden.
9. Canonical path after the correction: rough Sleep remains targetless (`OpportunityAnchor::None`), while co-located `OpportunityAnchor::Place(rest_site)` Sleep materializes the selected place as the authoritative action target. No duplicate lawful transport path was removed; the two paths remain the S174 D5 KnownRestSite/RoughSleep split.
10. Next-step revalidation also needed to accept the materialized place target for `BindingStrictness::AnyLegalTarget` actions with empty declared target specs. Without that, the planner found the correct targeted step during search but discarded it before execution.

## Architecture Check

1. The scenario authors `RestCapacity(1)` on the shelter Place to exercise single-slot contention — the smallest collision-worthy capacity. Larger capacities are tested in Scenario B (ticket 008).
2. Asserting on multiple verification layers (action trace for lifecycle, event log delta for `RestOccupancy` writes, decision trace for candidate emission, authoritative world state for occupancy, `CriticalWindowFrame.failed_rest_opportunities` for forensic chain) prevents collapsing the proof into a single weaker surface.
3. Deterministic replay: the scenario seeds the harness with a known ChaCha8Rng seed; replaying produces identical outcomes per FOUNDATIONS determinism invariant.

## Verified Layers

1. KnownRestSite candidate emission via FND-14A co-located observation -> decision trace assertion (both agents emit a KnownRestSite candidate at the shelter on the same tick)
2. Sleep action start writes `RestOccupancy.occupants = {first_agent}` -> event-log delta + authoritative world state query
3. Sleep action start for the losing agent returns `Err(ActionError)` (precondition: rest site full) -> action trace assertion (the Aborted event with precondition rejection reason)
4. Losing agent emits a RoughSleep candidate next tick -> decision trace assertion
5. Recovery modifier divergence: shelter occupant accumulates at approx 1.1x; rough sleeper accumulates at approx 0.3x cap -> authoritative `SleepEpisode.accumulated_recovery` comparison
6. Commit releases `RestOccupancy` -> event-log delta (component lifecycle)
7. `EventTag::SleepEpisodeEnded` carries `WakeReason::TargetRecovery` for both agents -> event-log assertion
8. `FailedRestOpportunity::PreconditionRejected` is recorded in the losing agent's active critical fatigue window -> `CriticalWindowFrame.failed_rest_opportunities` query
9. Deterministic replay -> repeat the harness twice with the same seed and assert identical state hashes

## Landed Changes

### 1. Authored the scenario RON file

Created `scenarios/survival-safe-rest.ron` with:

- Two places: `shelter_north` (with `SleepQualityProfile { shelter: Roofed, ground_comfort: Soft, recovery_modifier: 1100 }`, `rest_capacity: Some(1)`) and `open_camp` (no SleepQualityProfile, no rest_capacity)
- One edge connecting them (or both co-located at the spawn place)
- Two agents (`Aster`, `Bram`) with high fatigue and near-sated other needs; their `MetabolismProfile.rough_sleep_recovery_floor` is the default `Permille::new(300)` so rough sleep caps at ≈ 0.3x
- Seed: pick a stable ChaCha8Rng seed (precedent: existing scenarios use seed values like `42`, `1234`)

Follow `scenarios/survival-baseline.ron` and `survival-contested.ron` as the structural template — match the existing field ordering, comment-header conventions, and tag conventions.

### 2. Authored the corresponding test file

Created `crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs` following the precedent at `tests/scenarios/sleep_episode.rs` and `tests/scenarios/survival_contested.rs`. The test file:

- Loads `scenarios/survival-safe-rest.ron` via `worldwake_cli::scenario::load_scenario_file`
- Spawns the simulation via `spawn_scenario`
- Steps ticks via the persistent `AgentTickDriver` + `TickStepServices` pattern (per the Read-Only Tooling Consumer pattern in `reassess-spec/references/worldwake-validation-patterns.md`)
- Captures action-trace and decision-trace sinks per `tick`
- Runs until both agents complete a Sleep episode (commit) or hit a tick budget cap (e.g., 200 ticks)
- Asserts all 9 verification-layer claims above

Use a `// Scenario N:` comment header matching the next available scenario number (verify by grep at ticket-implementation time).

### 3. Hooked the test into the test binary

Per the existing precedent (`tests/scenarios/mod.rs` or equivalent), added a `mod survival_safe_rest;` declaration so cargo discovers the test.

## Landed Files

- `scenarios/survival-safe-rest.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `mod survival_safe_rest;`)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — materialize co-located place-anchored Sleep as a targeted rest-site action)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — revalidate `AnyLegalTarget` steps with materialized targets and empty declared target specs)
- `crates/worldwake-ai/src/search/tests.rs` (modify — focused regression for place-anchored Sleep target materialization)

## Out of Scope

- No multi-slot rest-site contention (Scenario B / ticket 008)
- No hostile-proximity interruption (Scenario C / ticket 009)
- No CLI player-POV assertions (Scenario D / ticket 010)
- No failed-rest cascade for S175 collapse golden (Scenario E / ticket 011)
- No multi-action queue promotion assertions beyond the losing agent's direct rough-sleep fallback
- No unrelated planner refactor beyond the Sleep target materialization/revalidation correction required by the golden

## Acceptance Criteria

### Passed Tests

1. Scenario test `survival_safe_rest::scenario_a_rest_site_contention` passes all 9 verification-layer assertions.
2. Deterministic replay test passes: running the scenario twice with the same seed produces identical state hashes.
3. Existing suite passed via `cargo test --workspace`.

### Invariants

1. The scenario isolates rest-site contention — no other survival action competes for planner attention (agents are near-sated for non-fatigue needs)
2. The scenario does not reach into authoritative world state from the agent's planner — all candidate emission flows through belief-view accessors per S174 D5
3. The scenario uses scenario-authored topology (`RestCapacity`, `SleepQualityProfile`) — no runtime mutation of topology

## Test Plan Result

### Landed Tests

1. `crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs` — Scenario A E2E golden coverage

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest` (targeted)
2. `cargo test --workspace` (full regression)
3. `./scripts/verify.sh` was not run for this per-ticket archive; the component gates it wraps are covered below, and the wrapper remains the final pre-PR gate for the full S174 branch.

## Outcome

Implemented Scenario A as `scenarios/survival-safe-rest.ron` and `crates/worldwake-ai/tests/scenarios/survival_safe_rest.rs`.

The golden now proves both agents emit the shelter KnownRestSite Sleep opportunity, one agent occupies the one-slot shelter, the other receives a rest-site precondition rejection, the losing agent starts targetless rough sleep after that rejection, shelter recovery uses the authored 1100 permille modifier, rough sleep uses the 300 permille floor, sleep completion releases `RestOccupancy`, the losing agent records one `FailedRestKind::PreconditionRejected`, and replay is deterministic.

The implementation also fixed the planner contradiction exposed by the golden: co-located place-anchored Sleep candidates now carry the selected rest-site target, and revalidation preserves that targeted step for `BindingStrictness::AnyLegalTarget` Sleep.

## Deviations

- The draft expected no production code changes. Live code proved that false; the golden could not pass truthfully until the AI planner materialized and revalidated the rest-site target for place-anchored Sleep.
- RoughSleep candidates are already emitted alongside KnownRestSite candidates in the current S174 planner surface. The golden therefore asserts the stronger lifecycle outcome that the rough action starts after the rest-site precondition rejection, rather than requiring candidate emission to first appear on the next tick.

## Verification Result

Completed verification:

1. Passed: `cargo test -p worldwake-ai search::tests::place_anchored_sleep_materializes_rest_site_target`
2. Passed: `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest`
3. Passed: `cargo test -p worldwake-ai`
4. Passed: `cargo fmt --all -- --check`
5. Passed: `cargo test --workspace`
6. Passed: `cargo clippy --workspace`
7. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
8. Waived: `./scripts/verify.sh` for this per-ticket archive because the same cargo fmt/test/clippy component gates were run directly; run the wrapper before final S174 PR push.
