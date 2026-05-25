# S172WASDISBUD-003: Belief-only Wash regression in scattered/contested topologies

**Status**: COMPLETED
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
7. Implementation reassessment corrected two draft proof details. The existing drive-escalation test function is `scenarios::survival_drive_escalation::escalation_respects_belief_only_planning`; `run_escalation_respects_belief_only_planning` is a helper and an exact Cargo selector runs zero tests. Also, the strongest honest scattered/contested isolation proof disables exploration and non-dirtiness needs for the selected authored agent, so the test proves no Wash candidate/plan/selection/commit plus unresolved dirtiness rather than unrelated other-self-care action progress.
12. Scenario isolation: the new sub-scenario(s) must deliberately exclude any belief-population path that would lawfully expose the remote basin (no exploration, no witness reports, no perception of the basin's place during the run window). Lawful competing affordances kept intentionally out: any agent action that would land at the basin's place, any messenger/rumor carrier, any belief-store seeding for the basin entity.
13. Adjacent contradictions: if the regression surfaces an existing remote-truth read in the candidate path, that is a CRITICAL planner regression — open a separate ticket and block S172 archival until fixed.

## Architecture Check

1. Generalizing the existing precedent (single-topology belief-only proof → multi-topology) is cleaner than introducing a topology-agnostic abstract test; the regression value comes from running against the actual scattered and contested setups so place-graph distances and contention pressure participate in candidate-emission.
2. No backwards-compatibility aliasing: the new sub-scenario(s) or parameterized variants stand alongside the existing belief-only proof, not as parallel implementations.
3. FND-14 + FND-14B alignment: the regression directly proves the "no remote-truth leak" invariant for two additional topologies, strengthening the per-topology coverage matrix.

## Verified Layers

1. No candidate references remote basin entity-id → focused test via `CandidateGenerationDiagnostics` (per-tick zero Wash candidates for the unseen basin) AND assertion that no `GoalKey` materialized references the remote basin.
2. Decision trace → no `SelectionTrace.selected_opportunity` resolves to a `GoalKey` whose anchor references the remote basin id.
3. Dirtiness still rises (per FND-16 ignorance is lawful) → authoritative world-state assertion on `HomeostaticNeeds::dirtiness` for the dirty agent across the run.
4. D4 isolation branch remains local-only and does not launder the unseen basin through exploration → test harness disables exploration pressure and non-dirtiness needs for the selected authored agent, then proves dirtiness does not drop and no `wash` action commits during the run.

## Landed Changes

### 1. Constructed belief-only Wash harness builders for scattered/contested

Following the precedent of `survival_drive_escalation.rs:244` `build_belief_only_wash_harness`, this ticket added two new harness builders backed by a shared golden-harness helper:

- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs::build_belief_only_wash_harness_scattered` — loads `scenarios/survival-scattered.ron`, uses the authored remote `WashBasin`, and omits any belief seeding for the remote basin.
- `crates/worldwake-ai/tests/scenarios/survival_contested.rs::build_belief_only_wash_harness_contested` — loads `scenarios/survival-contested.ron`, uses the authored remote `WashBasin`, and omits any belief seeding for the remote basin.
- `crates/worldwake-ai/tests/golden_harness/mod.rs::configure_belief_only_wash_barrier_agent` and `run_belief_only_wash_barrier` centralize the local-only belief setup and trace assertions for both topologies.

The shared helper preserves the authored scenario topology and existing remote `WashBasin`; it does not relocate the basin. It clears the selected agent's belief store, seeds only local beliefs, disables exploration pressure, leaves only dirtiness pressure active, and runs the real full-registry `GoldenHarness`.

### 2. Added belief-only regression tests

- `#[test] fn no_wash_plan_for_unseen_remote_basin_under_scattered_topology()` in `survival_scattered.rs`.
- `#[test] fn no_wash_plan_for_unseen_remote_basin_under_contested_topology()` in `survival_contested.rs`.

Each test runs the harness for `BELIEF_ONLY_TICKS` (matching the drive-escalation precedent at `survival_drive_escalation.rs`) and asserts:
- No `emit_wash_goal` candidate references the remote basin entity-id (verified via `CandidateGenerationDiagnostics`).
- No `SelectionTrace.selected_opportunity` resolves to a `GoalKey` anchored on the remote basin.
- No Wash plan is found or selected.
- `HomeostaticNeeds::dirtiness` never drops and final dirtiness is at least the initial value — FND-16 ignorance produces lawful drift.
- No `wash` action commits.

### 3. Added negative-case assertion

For each test, the negative-case assertion explicitly fails if ANY candidate references the remote basin id. The test name and assertion message surface the entity-id in the failure output so a regression's root cause is immediately legible.

## Landed Files

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modified — shared belief-only Wash barrier helper and observation record)
- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` (modified — scattered harness builder + ignored golden Scenario 468)
- `crates/worldwake-ai/tests/scenarios/survival_contested.rs` (modified — contested harness builder + ignored golden Scenario 477; also renumbered pre-existing contested scenarios to 470-476 to resolve a generator duplicate)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-details/survival-contested.md` (regenerated)
- `docs/generated/golden-scenario-details/survival-scattered.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)

## Out of Scope

- Any new test in `survival_drive_escalation.rs` — the precedent already exists there.
- Any change to `emit_wash_goal`, `wash_access_opportunities`, or the belief-view accessor — the test exists to lock the current behavior; changes belong in a separate ticket.
- Any new `MayContainWashBasin` exploration logic — covered by existing candidate-emission code; the regression proves the absence of exploration without belief.
- Player POV CLI assertion — covered by ticket 004.
- Test consolidation / parameterization across the three topologies (drive-escalation, scattered, contested) — defer unless the duplication is significant.

## Acceptance Result

### Verification Commands

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_wash_plan_for_unseen_remote_basin_under_scattered_topology -- --ignored --exact` — new test passes.
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested::no_wash_plan_for_unseen_remote_basin_under_contested_topology -- --ignored --exact` — new test passes.
3. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::escalation_respects_belief_only_planning -- --ignored --exact` — existing precedent still passes.
4. Existing affected crate: `cargo test -p worldwake-ai`.
5. Generated docs: `python3 scripts/golden_inventory.py --write --check-docs`.

### Invariants

1. Under all three topologies (drive-escalation, scattered, contested), no Wash candidate may be emitted for a `WashBasin` the agent has not perceived or been told about.
2. The negative-case assertion fails loudly with the offending basin entity-id in the failure message, not silently.
3. FND-16 ignorance is preserved — the agent's dirtiness rises and is not corrected by remote-truth reads.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — add `build_belief_only_wash_harness_scattered` + `no_wash_plan_for_unseen_remote_basin_under_scattered_topology`.
2. `crates/worldwake-ai/tests/scenarios/survival_contested.rs` — add `build_belief_only_wash_harness_contested` + `no_wash_plan_for_unseen_remote_basin_under_contested_topology`.
3. `crates/worldwake-ai/tests/golden_harness/mod.rs` — add shared `configure_belief_only_wash_barrier_agent` and `run_belief_only_wash_barrier`.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_wash_plan_for_unseen_remote_basin_under_scattered_topology -- --ignored --exact` — targeted new-test verification.
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested::no_wash_plan_for_unseen_remote_basin_under_contested_topology -- --ignored --exact` — targeted new-test verification.
3. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::escalation_respects_belief_only_planning -- --ignored --exact` — existing precedent verification.
4. `cargo test -p worldwake-ai` — full AI-crate suite to confirm no regression.
5. `python3 scripts/golden_inventory.py --write --check-docs` — regenerated golden inventory/docs and confirmed globally unique scenario IDs.
6. `./scripts/verify.sh` — waived for per-ticket closeout because `implement-spec-tickets` owns the final pre-push verification gate after the S172 family lands.

## Outcome

Completed on 2026-05-25.

- Added shared golden-harness support for a belief-only Wash barrier over authored scenario topologies.
- Added ignored golden Scenario 468 for scattered and Scenario 477 for contested, each proving an unseen remote `WashBasin` is not turned into a Wash candidate, found plan, selected plan, or committed wash action.
- Regenerated golden inventory/docs after adding scenario metadata.
- Resolved pre-existing generated-inventory duplicate scenario numbering between scattered Scenario 158 and contested Scenario 158 by renumbering the contested metadata block to 470-476.

## Deviations

- The test harness preserves the authored remote basin instead of relocating it. This is stronger for S172 because the assertion targets the scenario-authored facility while keeping the agent's belief store local-only.
- The scattered/contested isolation proof disables exploration pressure and non-dirtiness needs for the selected agent. That keeps the invariant at the no-remote-truth candidate boundary; it does not claim unrelated Eat/Drink/Sleep/Relieve progress in the same isolated run.
- The drafted drive-escalation exact selector named helper `run_escalation_respects_belief_only_planning`; the real test selector is `scenarios::survival_drive_escalation::escalation_respects_belief_only_planning`.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_wash_plan_for_unseen_remote_basin_under_scattered_topology -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested::no_wash_plan_for_unseen_remote_basin_under_contested_topology -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_drive_escalation::escalation_respects_belief_only_planning -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Waived `./scripts/verify.sh` for per-ticket closeout because the S172 `implement-spec-tickets` harness owns the final full pre-push gate after all S172 tickets land.
