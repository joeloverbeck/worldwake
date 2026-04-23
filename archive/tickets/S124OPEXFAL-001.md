# S124OPEXFAL-001: Preserve committed source provenance for retained plans

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI committed-plan/runtime provenance contract
**Deps**: `specs/S124-canonical-opportunity-expectation-failure.md`

## Problem

`SURVPREF-001` landed a truthful survival-time source-failure memory update, but the live path still loses concrete source identity once a source-backed opportunity becomes the retained current plan. `PlannedPlan` preserves only `OpportunityKey`, so the read phase in [`crates/worldwake-ai/src/agent_tick/observation.rs`](../../crates/worldwake-ai/src/agent_tick/observation.rs) currently has to call `hydrate_reinstated_current_plan_source_entity(...)` and rediscover the source from `resource_sources_at(place, commodity)` before it can detect local depletion. That keeps committed source provenance implicit, duplicates lawful inference paths, and blocks the canonical S124 architecture where source-backed expectation failure starts from preserved committed provenance rather than reconstruction.

## Assumption Reassessment (2026-04-23)

1. The motivating live production seam is the `SURVPREF-001` landing in [`crates/worldwake-ai/src/agent_tick/observation.rs`](../../crates/worldwake-ai/src/agent_tick/observation.rs), [`crates/worldwake-ai/src/agent_tick/planning.rs`](../../crates/worldwake-ai/src/agent_tick/planning.rs), [`crates/worldwake-ai/src/agent_tick/mod.rs`](../../crates/worldwake-ai/src/agent_tick/mod.rs), [`crates/worldwake-ai/src/candidate_generation.rs`](../../crates/worldwake-ai/src/candidate_generation.rs), [`crates/worldwake-ai/src/ranking.rs`](../../crates/worldwake-ai/src/ranking.rs), and [`crates/worldwake-ai/tests/golden_survival_preferences.rs`](../../crates/worldwake-ai/tests/golden_survival_preferences.rs).
2. The exact shared abstraction boundary under audit is: committed source-backed opportunity provenance carried from goal selection into `runtime.current_plan`, then consumed by retained-plan observation / planning failure handling / later source-reliability learning.
3. The current runtime carrier does not preserve that provenance. [`PlannedPlan`](../../crates/worldwake-ai/src/planner_ops.rs) stores `goal`, `opportunity`, `steps`, `total_estimated_ticks`, and `terminal_kind`, but no concrete source identity beyond `OpportunityKey`.
4. The current retained-plan repair is explicit reconstruction, not preserved state. [`hydrate_reinstated_current_plan_source_entity(...)`](../../crates/worldwake-ai/src/agent_tick/observation.rs) rehydrates `GoalOffer::evidence_entities` by searching `resource_sources_at(place, commodity)` when the reinstated candidate matches the current plan's `OpportunityKey`.
5. The current read-phase failure detector depends on that reconstruction. [`pending_local_source_reliability_failures(...)`](../../crates/worldwake-ai/src/agent_tick/observation.rs) reads the reinstated candidate's `evidence_entities` and only emits a `SourceKey` when there is exactly one source entity attached after hydration.
6. The current planning-side same-goal failure hook already proves the runtime wants one concrete current source identity. [`same_goal_search_failed_source_keys(...)`](../../crates/worldwake-ai/src/agent_tick/planning.rs) reads the ranked current opportunity's single `evidence_entities` entry and persists failure through `apply_source_reliability_failure_observations(...)`, but it still relies on ranked candidate evidence rather than committed-plan provenance.
7. This is an AI runtime / `agent_tick` contract ticket, not a scenario-only ticket. The owning change is the committed-plan/runtime carrier, and the survival golden is downstream proof rather than the primary implementation seam.
8. The live `GoalKind` under audit is the source-backed acquisition family already used by `SourceReliability`: `GoalKind::AcquireCommodity { .. }` and `GoalKind::RestockCommodity { .. }`.
9. The current truthful first failure boundary for the retained-plan path is read-phase local contradiction in `observation.rs`, not the old drafted authoritative harvest-start-only story from `SURVPREF-001`.
10. The current test inventory already has one focused unit proof for ranking fallout and one golden proof for the survival scenario:
    - `ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence`
    - `survival_preferences_keeps_proactive_diversification_alive_under_survival`
11. Mismatch + correction: the live bug is no longer "source reliability missing entirely." The narrower architectural gap is that committed source identity is still reconstructed after commitment instead of being preserved as committed runtime state.
12. New fallout discovered during implementation: `PlannedPlan` is serialized through `AgentDecisionRuntime`, so preserving committed source provenance changes the current save shape and requires a `SAVE_FORMAT_VERSION` bump in [`crates/worldwake-sim/src/save_load.rs`](../../crates/worldwake-sim/src/save_load.rs) to stay aligned with the repo's no-backwards-compat policy.
13. No-change cited files after live reassessment: [`crates/worldwake-ai/src/decision_runtime.rs`](../../crates/worldwake-ai/src/decision_runtime.rs) did not need direct edits because the persisted runtime carrier already stores `PlannedPlan`; [`crates/worldwake-ai/tests/golden_survival_preferences.rs`](../../crates/worldwake-ai/tests/golden_survival_preferences.rs) remained truthful without assertion changes.

## Architecture Check

1. The clean fix is to preserve source-backed provenance on the committed plan/runtime path and let retained-plan readers consume that preserved identity directly. That is cleaner than continuing to rediscover the source from local world/belief reads each tick because it makes the causal chain explicit and keeps one fact on one lawful transport path.
2. No backwards-compatibility shim should remain once the new carrier exists. `hydrate_reinstated_current_plan_source_entity(...)` should be removed rather than preserved as a fallback alias path.

## Verification Layers

1. Committed source-backed plan retains exact concrete source identity across retained-plan ticks -> focused `worldwake-ai` unit/runtime coverage at the `PlannedPlan` / runtime read-phase seam
2. Read-phase local depletion detection consumes preserved committed provenance without candidate rehydration -> focused `agent_tick/observation` regression
3. Same-goal survival divergence still lands after the provenance-carrier cleanup -> `golden_survival_preferences` ignored scenario test
4. Existing source-reliability ranking fallout still holds after the carrier change -> `ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence`
5. If traces still prove survival divergence but not enough provenance to explain the new carrier contract, add the strongest lower-layer proof at the retained-plan/runtime seam rather than broadening scenario assertions
6. Additional layer mapping is required because this ticket changes runtime carrier state while preserving downstream scenario behavior

## What to Change

### 1. Add a canonical committed source-provenance carrier

Extend the committed plan/runtime path so a source-backed acquisition plan can preserve its exact concrete `SourceKey` (or equivalent committed provenance record) once selected.

### 2. Consume preserved provenance in retained-plan observation

Rewrite the retained-plan read-phase source-failure path to use the committed provenance carrier directly. Remove `hydrate_reinstated_current_plan_source_entity(...)` and any equivalent source rediscovery path that exists only to recover what the commitment should already know.

### 3. Keep downstream planning/ranking behavior truthful

Update any same-goal failure or ranking integration that currently depends on rehydrated candidate evidence so it reads the canonical committed provenance instead. The ticket does not need to land the full S124 normalized incident model; it does need to leave committed provenance on a single honest runtime path.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)

## Out of Scope

- The full S124 normalized `OpportunityExpectationFailureIncident` substrate
- Unifying every source-failure write seam in one ticket
- Broad generalization to non-source-backed goals outside `AcquireCommodity` / `RestockCommodity`

## Acceptance Criteria

### Tests That Must Pass

1. Retained current plans for source-backed acquisition preserve their concrete source identity without rehydration from `resource_sources_at(...)`
2. The retained-plan local depletion path still records source-failure learning for the preserved concrete source
3. Existing suite: `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`

### Invariants

1. A committed source-backed opportunity must not require fresh source rediscovery merely to remember which concrete source it committed to
2. Source-failure learning must remain tied to one concrete source identity, not a place-wide or commodity-wide abstraction

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` or `crates/worldwake-ai/src/agent_tick/observation.rs` tests — prove retained current-plan provenance survives without candidate rehydration
2. `crates/worldwake-ai/src/agent_tick/planning.rs` tests — prove selected plans persist committed source provenance at adoption time
3. `crates/worldwake-ai/tests/golden_survival_preferences.rs` — keep the scenario-backed survival proof green after the carrier cleanup

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::adopt_selected_plan_populates_expected_commodity_assumption_immediately -- --exact`
3. `cargo test -p worldwake-ai --lib ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence -- --exact`
4. `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
5. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_writes_current_format_version -- --exact`
6. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-23.

- Added `PlannedPlan.committed_source: Option<SourceKey>` in [`crates/worldwake-ai/src/planner_ops.rs`](../../crates/worldwake-ai/src/planner_ops.rs) plus a small helper that derives committed source provenance from a ranked source-backed offer.
- Updated [`crates/worldwake-ai/src/agent_tick/planning.rs`](../../crates/worldwake-ai/src/agent_tick/planning.rs) so adopted plans persist committed source provenance, and same-goal sibling failure attribution now reads that committed runtime carrier instead of re-reading ranked candidate evidence.
- Updated [`crates/worldwake-ai/src/agent_tick/observation.rs`](../../crates/worldwake-ai/src/agent_tick/observation.rs) so retained-plan local depletion detection reads `runtime.current_plan.committed_source` directly and removes `hydrate_reinstated_current_plan_source_entity(...)`.
- Added focused runtime coverage in [`crates/worldwake-ai/src/agent_tick/tests.rs`](../../crates/worldwake-ai/src/agent_tick/tests.rs) for both plan adoption and read-phase local failure detection through the committed source carrier.
- Bumped [`crates/worldwake-sim/src/save_load.rs`](../../crates/worldwake-sim/src/save_load.rs) `SAVE_FORMAT_VERSION` from `41` to `42` because the persisted `PlannedPlan` shape changed.

## Deviations

- The live landing did not require direct edits to `decision_runtime.rs`; `PlannedPlan` already sits on the persisted runtime carrier, so the owned change stayed at the shared plan shape plus its producer/consumer seams.
- The survival golden stayed truthful without source-facing assertion changes, so `golden_survival_preferences.rs` remained a verification-only seam.
- `cargo fmt --all` produced formatter spillover on already-dirty AI files outside this ticket's owned seam (`agent_tick/mod.rs`, `candidate_generation.rs`, `ranking.rs`, `tests/golden_survival_preferences.rs`). Those paths were left untouched after formatting and are not attributed as `S124OPEXFAL-001` implementation work.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::adopt_selected_plan_populates_expected_commodity_assumption_immediately -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::pending_source_reliability_failure_reorders_candidates_before_persistence -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_writes_current_format_version -- --exact`
- Passed `cargo test -p worldwake-ai`
