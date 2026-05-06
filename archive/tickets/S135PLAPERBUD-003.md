# S135PLAPERBUD-003: Remove planner snapshot per-place cap; integrate perception omission write

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `planning_snapshot` truncation logic, perception batch write, scenario `CognitiveProfile` schema, RON fixtures, current save-format version
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`

## Problem

The double-cap that S135 was written to remove lives in two places: (a) `CognitiveProfile.max_snapshot_entities_per_place=50` in `worldwake-core` and the planner-side `truncate(max_per_place)` at `crates/worldwake-ai/src/planning_snapshot.rs:1264`, called from `agent_tick/planning.rs:537-546`, and (b) the absence of any audit trail when perception's per-tick budget at `crates/worldwake-systems/src/perception.rs:665` drops entities. This ticket deletes the planner cap (so the planner consumes the full set of accumulated belief entities at a place) and adds the perception write that populates `ObservationOmissionLog` whenever the budget truncation fires.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `CognitiveProfile.max_snapshot_entities_per_place` exists at `crates/worldwake-core/src/cognitive_profile.rs:96` (`u16`, default `50` at line 143). Workspace-wide references: 24 source-tree sites — 1 active runtime read at `crates/worldwake-ai/src/agent_tick/planning.rs:546`; 6 test-mock construction sites (`failure_handling.rs:1906`, `decision_runtime.rs:452`, `goal_model.rs:2405`, `agent_tick/planning.rs:2416`, `agent_tick/tests.rs:200`, `search/tests.rs:84`); the field def + Default + 2 tests in `cognitive_profile.rs` (lines 285, 322); scenario authoring in `crates/worldwake-cli/src/scenario/types.rs:1537`; test in `delta.rs:622`. Plus 11 RON fixture authoring sites: `crates/worldwake-cli/tests/fixtures/observer_anomalies/{stuck_detector_wash_travel_cycle, acute_thirst_spike, maintenance_starvation_wash_gap, convergence_hub, recipe_monoculture_apples_vs_grain}.ron` and `crates/worldwake-ai/tests/fixtures/portfolio-planning.ron`. No production scenarios in `scenarios/` author this field.
2. `collect_direct_local_observation_batch` lives at `crates/worldwake-systems/src/perception.rs:639`. Truncation at line 665: `prioritized_entities.truncate(usize::from(profile.observation_budget));`. Priority composition at `compute_observation_priority` (line 714) reads `profile.need_salience_boost`. Inline tests in `perception.rs` cfg-test block start at line 1407 — relevant truncation-touching tests: `agent_observes_place_without_scene_evidence`:2163, `active_action_does_not_cross_place_boundaries_or_self_observe`:2684, plus ~25 other inline tests at lines 1688–2683.
3. `build_planning_snapshot_with_blocked_facility_uses` lives at `crates/worldwake-ai/src/planning_snapshot.rs:1149` and accepts a `max_per_place: u16` parameter; per-place `filtered.truncate(usize::from(max_per_place));` at line 1264. Removing the parameter is a workspace-compile-blocking change — the lone runtime caller at `agent_tick/planning.rs:537-546` and the function signature must change in the same ticket.
4. Shared abstraction boundary under audit: the perception → belief-store delta path (`worldwake-systems/src/perception.rs` writes through `BeliefStoreDiff`) plus the planner snapshot construction path (`worldwake-ai/src/planning_snapshot.rs` reads accumulated beliefs via `RuntimeBeliefView`). The change replaces planner-side re-truncation with perception-side audit; the snapshot becomes a derived view over the agent's already-truncated belief observations. Existing trace/integration coverage: `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_prioritizes_agents_and_facilities_over_waste`:606 (per-tick priority truncation; expected to remain green post-ticket).
5. Scenario `CognitiveProfile` definition at `crates/worldwake-cli/src/scenario/types.rs:1537` currently carries `max_snapshot_entities_per_place: 50` as a default literal. Removing the field but leaving `max_snapshot_entities_per_place: 50` in 11 RON fixtures will cause deserialization failures (verify `CognitiveProfile`'s serde mode during reassessment — `#[serde(deny_unknown_fields)]` is the assumed default). Fixture cleanups land atomically in this ticket.
6. `scripts/profile_docs.py` regenerates `docs/profiles/all-profiles.md` from `CognitiveProfile`; the field deletion requires a regen.
7. The perception omission-write path mutates `AgentBeliefStore.observation_omission_log` via the existing `BeliefStoreDiff` paired-field path (`omission_log_added` / `omission_log_removed_count`) added in ticket 001, so the existing event-log delta compaction handles the new fields without a separate ticket.
8. Live save-shape reassessment corrected the ticket-001 handoff: `CognitiveProfile` is a persisted component, so removing `max_snapshot_entities_per_place` changes the current save format. This ticket bumps `SAVE_FORMAT_VERSION` 67→68; older versions remain rejected per the repo's no-backward-compatibility rule.

## Architecture Check

1. **FND-12 alignment**: the planner cap was a causality-compressing cache. Removing it means the planner sees exactly what perception delivered — single source of truth for per-place observability.
2. **FND-28 alignment**: no shim, no deprecated wrapper. The field, the parameter, the truncate call, and all RON authoring sites are removed atomically. Workspace compiles after the ticket because the per-place cap is no longer a parameter or call argument anywhere.
3. **Information path**: perception writes `AgentBeliefStore.observation_omission_log` via the existing `BeliefStoreDiff` paired-field path (added in ticket 001). The AI crate reads beliefs (no per-place cap) and the omission log (ticket 002's accessor); no new cross-crate calls. The same fact (which entities were dropped this tick) traveled through *no* observable path before this ticket; after, it travels through the nested `ObservationOmissionLog` only — single canonical path.
4. **Heuristic removal discipline**: `max_snapshot_entities_per_place` was standing in for "bounded planner-snapshot memory at long-lived crowded places." This ticket does not introduce a substitute substrate — instead, the spec accepts unbounded accumulated-belief reads at the planner. If real scenarios later exceed reasonable bounds, the FND-12-clean fix is to tighten activation-based decay (S101) rather than re-introduce a snapshot cap. This is documented in the spec's Risks section.

## Verification Layers

1. Per-place cap is gone from runtime → focused unit test asserting `CognitiveProfile` no longer has `max_snapshot_entities_per_place` (compile-time invariant via removed field).
2. Perception write generates `ObservationOmission` records when the budget truncation fires → focused unit test in `perception.rs` cfg-test block: setup 30 entities co-located with an agent whose `observation_budget = 12`, run `collect_direct_local_observation_batch`, assert 18 `ObservationOmission { reason: OmissionReason::OverBudget { budget: 12, candidates_seen: 30 }, .. }` records appended to the agent's `ObservationOmissionLog`.
3. Snapshot construction consumes full belief set → focused unit test in `planning_snapshot.rs` confirming the snapshot's per-place entity count matches `AgentBeliefStore.known_entities` filtered to the place, with no truncation.
4. RON fixtures and scenario `CognitiveProfile` no longer carry the deleted field → fixture deserialization succeeds (compile-time and `cargo test -p worldwake-cli` runtime check).
5. Determinism: omission entries emit in `BTreeMap`-stable order with FIFO ring-buffer eviction → focused unit test exercising 20 dropped entities against `omission_log_capacity = 5`.
6. Save/load current-format boundary → focused save-load test verifies `SAVE_FORMAT_VERSION = 68` after the serialized `CognitiveProfile` shape changed.

## What to Change

### 1. Remove `max_snapshot_entities_per_place` from `CognitiveProfile`

Delete the field from `crates/worldwake-core/src/cognitive_profile.rs:96` plus the `Default` initializer at line 143 and the 2 tests at lines 285 and 322. Update all 6 test-mock construction sites in `worldwake-ai` (`failure_handling.rs:1906-1907`, `decision_runtime.rs:452-453`, `goal_model.rs:2405-2406`, `agent_tick/planning.rs:2416-2417`, `agent_tick/tests.rs:200-201`, `search/tests.rs:84-85`) to drop the field assignment. Update the `delta.rs:622` test site.

### 2. Remove per-place cap from snapshot construction

In `crates/worldwake-ai/src/planning_snapshot.rs`, remove the `max_per_place: u16` parameter from `build_planning_snapshot_with_blocked_facility_uses` and the per-place `filtered.truncate(usize::from(max_per_place));` at line 1264. The function now consumes whatever `BTreeSet<EntityId>` the per-place filter produces.

In `crates/worldwake-ai/src/agent_tick/planning.rs:537-546`, update the call site to no longer pass `cognitive.max_snapshot_entities_per_place`.

### 3. Remove field from scenario `CognitiveProfile`

In `crates/worldwake-cli/src/scenario/types.rs:1537`, delete the `max_snapshot_entities_per_place: 50,` field from the scenario `CognitiveProfile` `Default` literal (and any other occurrences of the field within `types.rs`).

### 4. Remove field from RON fixtures

Delete `max_snapshot_entities_per_place: 50,` (or other authored values) from 11 RON fixture sites:
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron`
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron`
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` (2 occurrences)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` (3 occurrences)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` (2 occurrences)
- `crates/worldwake-ai/tests/fixtures/portfolio-planning.ron`

### 5. Wire perception omission write

In `crates/worldwake-systems/src/perception.rs` near the existing `truncate(usize::from(profile.observation_budget))` at line 665:

1. Capture the dropped entities (those at indices `[profile.observation_budget..]` of the priority-sorted vector) before truncation.
2. For each dropped entity, construct `ObservationOmission { omitted_entity, reason: OmissionReason::OverBudget { budget: profile.observation_budget, candidates_seen: prioritized_entities.len() as u16 }, observed_tick: tick }`.
3. Append each to the agent's `ObservationOmissionLog` via the belief-store delta path (the same path the existing `social_observations_added` field uses).
4. Apply ring-buffer eviction (FIFO) when the log exceeds `profile.omission_log_capacity`.

The `ObservationOmissionLog` mutation routes through `AgentBeliefStore.observation_omission_log` and the same `BeliefStoreDiff` paired-field path (`omission_log_added` / `omission_log_removed_count`) added in ticket 001.

### 6. Regenerate profile docs

Run `python3 scripts/profile_docs.py --write`. Commit the regenerated `docs/profiles/all-profiles.md`.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify) — field + Default + tests removal
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify) — remove `max_per_place` parameter and per-place truncate
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify) — call site update at line 546; remove test-mock construction at lines 2416-2417
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — line 200-201 mock removal
- `crates/worldwake-ai/src/decision_runtime.rs` (modify) — line 452-453 mock removal
- `crates/worldwake-ai/src/goal_model.rs` (modify) — line 2405-2406 mock removal
- `crates/worldwake-ai/src/failure_handling.rs` (modify) — line 1906-1907 mock removal
- `crates/worldwake-ai/src/search/tests.rs` (modify) — line 84-85 mock removal
- `crates/worldwake-core/src/delta.rs` (modify) — line 622 test removal
- `crates/worldwake-sim/src/save_load.rs` (modify) — current save-format version bump 67→68
- `crates/worldwake-cli/src/scenario/types.rs` (modify) — line 1537 scenario default literal
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` (modify)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron` (modify)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` (modify)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` (modify)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` (modify)
- `crates/worldwake-ai/tests/fixtures/portfolio-planning.ron` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify) — perception omission write integration
- `docs/profiles/all-profiles.md` (modify, regenerated)

## Out of Scope

- `Discrepancy::Omission` variant addition → ticket 004.
- AI-side reads of `ObservationOmissionLog` → tickets 002 (accessor), 004, 005.
- Observer rendering of the omission summary → ticket 006.
- New goldens → ticket 007.
- `OmissionReason::SalienceBelowFloor` write path — perception's priority truncation does not currently apply a salience floor distinct from the budget cut; that variant exists for future extension and is unused by this ticket. The variant remains addressable by `Discrepancy::Omission` at ticket 004 and by goldens at ticket 007 (`OverBudget` path only).

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems --lib perception` passes (new omission-write tests + existing inline tests in `perception.rs` cfg-test block at line 1407 still pass — `agent_observes_place_without_scene_evidence`:2163, `active_action_does_not_cross_place_boundaries_or_self_observe`:2684, etc.).
2. `cargo test -p worldwake-ai --test golden_perception_exposure` passes (`golden_observation_budget_prioritizes_agents_and_facilities_over_waste`:606 unchanged — its assertions exercise `observation_budget`, not the deleted planner cap).
3. `cargo test -p worldwake-cli` passes (RON fixtures deserialize without the deleted field).
4. `cargo build --workspace` succeeds.
5. `python3 scripts/profile_docs.py --write` regenerates docs, and a no-write generated output comparison confirms docs are current.

### Invariants

1. `CognitiveProfile` no longer carries `max_snapshot_entities_per_place` (compile-time invariant).
2. `build_planning_snapshot_with_blocked_facility_uses` no longer takes a `max_per_place` parameter (compile-time invariant).
3. Whenever perception's `truncate(observation_budget)` drops entities, `ObservationOmissionLog` receives one `OmissionReason::OverBudget` entry per dropped entity, in `BTreeMap`-stable order (sorted by `omitted_entity` ascending).
4. `ObservationOmissionLog` ring buffer respects `omission_log_capacity` with FIFO eviction (oldest entries evicted first).
5. Workspace source, RON fixtures, and generated profile docs build cleanly with no live `max_snapshot_entities_per_place` surface.
6. **Negative invariant**: an agent's `ObservationOmissionLog` never contains an entity that is also in their `BeliefStore.known_entities` for the same tick.
7. `SAVE_FORMAT_VERSION = 68` is the only version that round-trips post-ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` cfg-test block — new test: agent with `observation_budget = 12`, 30 co-located non-agent entities, run `collect_direct_local_observation_batch` for one tick, assert 18 `ObservationOmission { reason: OmissionReason::OverBudget { budget: 12, candidates_seen: 30 }, .. }` entries appended to the agent's `ObservationOmissionLog` and `BeliefStore.known_entities` contains 12 non-place observed entities.
2. `crates/worldwake-systems/src/perception.rs` cfg-test block — new test: 20 dropped entities against `omission_log_capacity = 5`, assert ring-buffer FIFO eviction (last 5 entries retained, first 15 evicted).
3. `crates/worldwake-systems/src/perception.rs` cfg-test block — new test: `BTreeMap`-stable insertion order across multiple ticks (assert deterministic entry ordering by `omitted_entity` ascending within each tick).
4. `crates/worldwake-systems/src/perception.rs` cfg-test block — negative-invariant test: assert `ObservationOmissionLog ∩ BeliefStore.known_entities == ∅` after running for 5 ticks.
5. Existing tests in `perception.rs` cfg-test block (line 1407+) — verify still pass with no behavioral change beyond the new omission-log side effect.
6. `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_prioritizes_agents_and_facilities_over_waste`:606 — verify still passes.

### Commands

1. `cargo test -p worldwake-systems --lib perception`
2. `cargo test -p worldwake-ai --test golden_perception_exposure`
3. `cargo test -p worldwake-cli`
4. `cargo build --workspace`
5. `python3 scripts/profile_docs.py --write`; `python3 scripts/profile_docs.py > /tmp/worldwake-profile-docs-current.md`; `cmp -s /tmp/worldwake-profile-docs-current.md docs/profiles/all-profiles.md`
6. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-06.

- Removed `CognitiveProfile.max_snapshot_entities_per_place` from the core profile, scenario defaults, explicit test/mock construction sites, RON fixtures, and generated profile docs.
- Removed the planner snapshot per-place cap parameter and truncate call; `PlanningSnapshot` now consumes the full filtered believed entity set for each included place.
- Wired perception over-budget truncation to append `ObservationOmission { reason: OmissionReason::OverBudget { .. } }` entries through `AgentBeliefStore.observation_omission_log`, with FIFO eviction by `PerceptionProfile.omission_log_capacity`.
- Added focused perception tests for over-budget writes, FIFO eviction, multi-tick stable omission ordering, and omission/known-entity disjointness, plus snapshot tests proving all believed entities remain present.
- Bumped `SAVE_FORMAT_VERSION` 67→68 because removing a field from serialized `CognitiveProfile` changes the current save shape.

## Deviations

- The draft said no S135 ticket after 001 would bump the save format. Live persisted-shape reassessment showed `CognitiveProfile` is serialized, so this ticket owns the 67→68 bump.
- The over-budget write records the priority-sorted dropped tail. The focused tests assert deterministic `EntityId` order where all candidates have equal priority; mixed-priority ordering remains the live salience order followed by `EntityId` tie-breaks.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib perception -- --list`.
- Passed `cargo test -p worldwake-systems --lib perception`.
- Passed `cargo test -p worldwake-systems --lib perception` after adding the post-review multi-tick stable-order test.
- Passed `cargo test -p worldwake-ai --lib planning_snapshot -- --list`.
- Passed `cargo test -p worldwake-ai --lib planning_snapshot`.
- Passed `cargo test -p worldwake-ai --test golden_perception_exposure`.
- Passed `cargo test -p worldwake-sim --lib save_load`.
- Passed `cargo test -p worldwake-cli`.
- Passed `cargo build --workspace`.
- Passed `python3 scripts/profile_docs.py --write` with existing profile doc-comment warnings, then `python3 scripts/profile_docs.py > /tmp/worldwake-profile-docs-current.md` and `cmp -s /tmp/worldwake-profile-docs-current.md docs/profiles/all-profiles.md`.
- Passed `./scripts/verify.sh`, whose live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `./scripts/verify.sh` again after adding the post-review multi-tick stable-order test.
