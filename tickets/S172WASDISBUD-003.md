# S172WASDISBUD-003: Belief-only Wash regression in scattered/contested topologies

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: specs/S172-wash-discovery-budget-closure.md (D4, D5 distributed — no-candidate surface)

## Problem

`crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs:244` (`build_belief_only_wash_harness`) already proves that under drive-escalation topology a `WashBasin` invisible to the agent — placed at a remote, never-perceived location — produces no Wash candidate. The proof shape covers FND-14B (belief-backed planner inputs only). S172 D4 requires generalizing this regression to scattered and contested topologies, where the relevant placement geometry differs (resources spread across multiple places under travel pressure; shared contested facilities). Without the generalization, the project has a single-topology proof that a planner regression introducing remote-truth reads under scattered or contested conditions would not be caught by existing goldens.

## Assumption Reassessment (2026-05-25)

1. Existing belief-only Wash proof: `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs:244` `build_belief_only_wash_harness` constructs a harness with `WashBasin` at `VILLAGE_SQUARE`, agent at `ORCHARD_FARM`, no belief seeded for the remote basin; the test at line 318 (`run_escalation_respects_belief_only_planning`) runs `BELIEF_ONLY_TICKS` and verifies no Wash candidate references the remote basin. Sibling pattern in `survival_scattered.rs:547` `seeded_target_location_belief_decays_to_stale_without_refresh` exercises belief decay (a different but adjacent concern).
2. S172 D4 (`specs/S172-wash-discovery-budget-closure.md`): "Add the same proof shape to `survival-scattered` and `survival-contested`." D5's no-candidate branch (`emit_wash_goal` empty-emit) is the failure-attribution surface this regression filters on.
3. Shared abstraction boundary: the cross-system contract is "no `emit_wash_goal` candidate may reference a `WashBasin` entity-id the agent has no belief about." This is a candidate-emission-layer invariant proven via `CandidateGenerationDiagnostics` and decision trace.
4. Intended invariant: candidate enumeration must not synthesize a Wash candidate from authoritative remote state. Hereby applies to scattered, contested, and drive-escalation topologies symmetrically.
5. Live `GoalKind` under test: `GoalKind::Wash`. The candidate-emission helper under test is `emit_wash_goal` at `crates/worldwake-ai/src/candidate_generation.rs:4607`, which routes through `wash_access_opportunities` at line 4744. The belief-view accessor at `crates/worldwake-sim/src/per_agent_belief_view.rs:824` is the dual-mode FND-14A authoritative-or-stored-belief accessor.
6. AI regression layer: focused candidate-generation coverage is sufficient if the harness is needs-only AND exercises a single tick; full action registries are required if the test must run for multiple ticks to verify steady-state absence. Per the drive-escalation precedent (`BELIEF_ONLY_TICKS`-long run), prefer full registries.
12. Scenario isolation: the new sub-scenario(s) must deliberately exclude any belief-population path that would lawfully expose the remote basin (no exploration, no witness reports, no perception of the basin's place during the run window). Lawful competing affordances kept intentionally out: any agent action that would land at the basin's place, any messenger/rumor carrier, any belief-store seeding for the basin entity.
13. Adjacent contradictions: if the regression surfaces an existing remote-truth read in the candidate path, that is a CRITICAL planner regression — open a separate ticket and block S172 archival until fixed.

## Architecture Check

1. Generalizing the existing precedent (single-topology belief-only proof → multi-topology) is cleaner than introducing a topology-agnostic abstract test; the regression value comes from running against the actual scattered and contested setups so place-graph distances and contention pressure participate in candidate-emission.
2. No backwards-compatibility aliasing: the new sub-scenario(s) or parameterized variants stand alongside the existing belief-only proof, not as parallel implementations.
3. FND-14 + FND-14B alignment: the regression directly proves the "no remote-truth leak" invariant for two additional topologies, strengthening the per-topology coverage matrix.

## Verification Layers

1. No candidate references remote basin entity-id → focused test via `CandidateGenerationDiagnostics` (per-tick zero Wash candidates for the unseen basin) AND assertion that no `GoalKey` materialized references the remote basin.
2. Decision trace → no `SelectionTrace.selected_opportunity` resolves to a `GoalKey` whose anchor references the remote basin id.
3. Dirtiness still rises (per FND-16 ignorance is lawful) → authoritative world-state assertion on `HomeostaticNeeds::dirtiness` for the dirty agent across the run.
4. Other self-care families continue to be exercised → action-trace presence of `eat`, `drink`, `sleep`, `toilet`/`relieve_wilderness` actions per agent (failure mode: if absent, the topology is starving the agent of all self-care, not just Wash — a different bug surface).

## What to Change

### 1. Construct belief-only Wash harness builders for scattered/contested

Following the precedent of `survival_drive_escalation.rs:244` `build_belief_only_wash_harness`, add two new harness builders:

- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs::build_belief_only_wash_harness_scattered` — loads `scenarios/survival-scattered.ron`, repositions or relocates the `WashBasin` to a place no agent has perceived during the seed window, omits any belief seeding for the remote basin.
- `crates/worldwake-ai/tests/scenarios/survival_contested.rs::build_belief_only_wash_harness_contested` — loads `scenarios/survival-contested.ron`, similar relocation + belief-omission.

If the chosen approach is parameterized sub-scenarios (single shared harness builder taking a topology selector), consolidate into a shared helper module — but only if the resulting code is clearer than two side-by-side builders. Default to two builders for visibility.

### 2. Add belief-only regression tests

- `#[test] fn no_wash_plan_for_unseen_remote_basin_under_scattered_topology()` in `survival_scattered.rs`.
- `#[test] fn no_wash_plan_for_unseen_remote_basin_under_contested_topology()` in `survival_contested.rs`.

Each test runs the harness for at least `BELIEF_ONLY_TICKS` (matching the drive-escalation precedent at `survival_drive_escalation.rs`) and asserts:
- No `emit_wash_goal` candidate references the remote basin entity-id (verified via `CandidateGenerationDiagnostics`).
- No `SelectionTrace.selected_opportunity` resolves to a `GoalKey` anchored on the remote basin.
- `HomeostaticNeeds::dirtiness` rises monotonically (or at least never drops to clean) — FND-16 ignorance produces lawful drift.
- Other self-care families produce at least one action each per dirty agent (food, water, sleep, relieve).

### 3. Negative-case assertion

For each test, the negative-case assertion explicitly fails if ANY candidate references the remote basin id. The test name and assertion message must surface the entity-id in the failure output so a regression's root cause is immediately legible.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` (modify — add harness builder + test)
- `crates/worldwake-ai/tests/scenarios/survival_contested.rs` (modify — add harness builder + test)

## Out of Scope

- Any new test in `survival_drive_escalation.rs` — the precedent already exists there.
- Any change to `emit_wash_goal`, `wash_access_opportunities`, or the belief-view accessor — the test exists to lock the current behavior; changes belong in a separate ticket.
- Any new `MayContainWashBasin` exploration logic — covered by existing candidate-emission code; the regression proves the absence of exploration without belief.
- Player POV CLI assertion — covered by ticket 004.
- Test consolidation / parameterization across the three topologies (drive-escalation, scattered, contested) — defer unless the duplication is significant.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_wash_plan_for_unseen_remote_basin_under_scattered_topology -- --ignored --exact` — new test passes.
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested::no_wash_plan_for_unseen_remote_basin_under_contested_topology -- --ignored --exact` — new test passes.
3. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::run_escalation_respects_belief_only_planning -- --ignored --exact` — existing precedent still passes.
4. Existing modules: `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored`, `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested:: -- --ignored`, and `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation:: -- --ignored`.

### Invariants

1. Under all three topologies (drive-escalation, scattered, contested), no Wash candidate may be emitted for a `WashBasin` the agent has not perceived or been told about.
2. The negative-case assertion fails loudly with the offending basin entity-id in the failure message, not silently.
3. FND-16 ignorance is preserved — the agent's dirtiness rises and is not corrected by remote-truth reads.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — add `build_belief_only_wash_harness_scattered` + `no_wash_plan_for_unseen_remote_basin_under_scattered_topology`.
2. `crates/worldwake-ai/tests/scenarios/survival_contested.rs` — add `build_belief_only_wash_harness_contested` + `no_wash_plan_for_unseen_remote_basin_under_contested_topology`.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_wash_plan_for_unseen_remote_basin_under_scattered_topology -- --ignored --exact` — targeted new-test verification.
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested::no_wash_plan_for_unseen_remote_basin_under_contested_topology -- --ignored --exact` — targeted new-test verification.
3. `cargo test -p worldwake-ai` — full AI-crate suite to confirm no regression.
4. `./scripts/verify.sh` — pre-PR full verification.
