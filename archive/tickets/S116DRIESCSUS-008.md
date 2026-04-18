# S116DRIESCSUS-008: Audit planner snapshot remote entity visibility against the belief barrier

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planning snapshot/entity admission, planner-focused tests, planner contract docs
**Deps**: docs/FOUNDATIONS.md, docs/planner-contracts.md, tickets/S116DRIESCSUS-006.md

## Problem

Focused reassessment for `S116DRIESCSUS-006` exposed a planner-boundary risk: [`PlanningSnapshot::collect_entities()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs:1128) currently pulls authoritative `entities_at(place)` for every included place within travel horizon, and [`build_snapshot_entity()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs:844) then copies authoritative facility/entity fields such as `workstation_tag`, `resource_source`, and `office_data` into snapshot-backed planner state. If those remote entities were not carried by the agent's beliefs or explicit grounded evidence, this can make the planner act on facts the agent never perceived, violating FND-14 and FND-15.

This ticket audits and repairs that boundary without broadening into a second travel-topology redesign. The target is specific: remote entities and their planner-relevant fields must not become planner-visible solely because their place is within travel horizon.

## Assumption Reassessment (2026-04-17)

1. The live shared boundary under audit is `PlanningSnapshot::build_with_blocked_facility_uses()` -> `collect_places()` / `collect_entities()` / `build_snapshot_entity()` in [planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), consumed through snapshot-backed planner queries in `PlanningState` and goal helpers such as [`GoalKind::Wash.goal_relevant_places()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs:1464).
2. `collect_places()` currently includes the actor's authoritative current place plus all adjacent places out to `travel_horizon`, using [`adjacent_places_with_travel_ticks()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs:1090). Per [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md), authoritative travel topology and raw travel duration are already a declared planner contract. This ticket does not re-open that topology contract.
3. `collect_entities()` currently extends the snapshot with authoritative `view.entities_at(place)` for every included place, filtered only by `SnapshotEntityFilter` and per-place cap ordering, not by whether the actor believes those entities exist there. This is the first concrete suspected FND-14 breach surface.
4. `build_snapshot_entity()` then copies authoritative entity/facility fields including `workstation_tag`, `resource_source`, `office_data`, `seller_for_sale_lot`, `has_sale_listing`, `record_data`, `controllable_by_actor`, and `has_control` for every included entity. If an entity was admitted unlawfully, all of those facts become unlawfully planner-visible.
5. Live `GoalKind::Wash` has two distinct surfaces that must not be conflated:
   - candidate/admission surface: [`candidate_generation::tests::wash_requires_dirtiness_and_local_water`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs:8168) proves `Wash` is admitted from dirtiness plus directly possessed local water
   - planner place-guidance surface: [`goal_model::tests::wash_returns_places_with_wash_basins`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs:6665) proves `Wash` guidance currently reads `places_with_workstation(..., WorkstationTag::WashBasin)`
6. The tick-0 `Wash` concern that surfaced during `S116DRIESCSUS-006` reassessment is not itself proof of a belief leak. A second pass showed a lawful local path already exists: `Wash` is feasible with possessed water ([feasibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs:117)), can surface as a depth-0 affordance root candidate via [`search_candidates_with_expansion_trace()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs:205), and does not require a basin at action validation time ([needs_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs_actions.rs:204)).
7. The honest invariant is therefore narrower and stronger than the original S116 golden wording: remote facilities, items, offices, and other non-evidence entities must not become planner-visible through the snapshot unless that visibility is carried by belief state or explicit grounded evidence. Local-water `Wash` must remain lawful and unaffected.
8. This is a planner-boundary ticket, not a world-sim or action-validation ticket. `wash_preconditions`, `candidate_generation` local-water admission, and `PlannerOpKind::Wash` execution semantics are out of scope unless focused proof shows they are directly coupled to the snapshot leak.
9. Existing focused proof surfaces already available for this work:
   - `planning_snapshot::tests::*` in `worldwake-ai` for snapshot admission rules
   - `goal_model::tests::wash_returns_empty_without_wash_basins`
   - `planner_conformance::conformance_wash`
   - decision-trace/search tests under `search::tests::*` for planner-visible root candidates and found-plan behavior
10. Adjacent contradiction classification:
   - required consequence of this ticket: clarify the canonical planner contract in [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) for remote entity/facility admission
   - separate issue already corrected: `S116DRIESCSUS-006` no longer treats the tick-0 `Wash` path as proof of a belief leak
11. Mismatch + correction: the earlier reassessment wording in `S116DRIESCSUS-006` overfit the tick-0 `Wash` symptom to the snapshot issue. This ticket corrects the scope to the real suspected violation: snapshot admission of remote non-believed entities and their fields.

## Architecture Check

1. The clean fix is to repair the single planner snapshot admission path, not to sprinkle belief checks across goal helpers, heuristic code, or individual `GoalKind` branches. That preserves one authoritative carrier for planner-visible runtime data.
2. Keeping authoritative travel topology out of scope avoids conflating two different contracts: route graph availability and remote entity/facility visibility. The former is a declared planner contract today; the latter is the suspected violation.
3. No backwards-compatibility aliasing or duplicate planner-state carriers should be introduced. The end state is one lawful snapshot path: belief-backed entity visibility plus explicit grounded evidence, with no parallel “remote authoritative entity” fallback.

## Verification Layers

1. Remote entity/facility admission respects the belief barrier -> focused `planning_snapshot::tests::*` coverage against `build_planning_snapshot[_with_blocked_facility_uses]`
2. `GoalKind::Wash` place guidance cannot see an un-believed remote wash basin through snapshot-backed state -> focused `goal_model` / `search` planner tests
3. Lawful local-water `Wash` remains available without a basin belief -> existing `candidate_generation::tests::wash_requires_dirtiness_and_local_water` plus `planner_conformance::conformance_wash`
4. Planner traceability remains strong enough to explain the repaired surface -> focused `search::tests::*` or decision-trace assertions if root candidate omission/absence depends on the new admission rule
5. Strongest proof surface for the core invariant is planner snapshot + planner search tests, not an end-to-end golden. Goldens may follow later if behavior changes materially, but they are not the first correctness boundary for this ticket.

## What to Change

### 1. Audit and tighten snapshot entity admission

Rework `collect_entities()` and any helper seams it depends on so that remote entities at included places are admitted into `PlanningSnapshot` only when they are:

- the actor
- an explicit grounded evidence entity
- required containment/possession relatives of an already admitted entity
- or present through the actor's belief-visible entity set, not raw `entities_at(place)` truth alone

Preserve the current authoritative place graph and travel-duration matrix behavior unless focused proof shows a narrower lawful adjustment is required.

### 2. Tighten snapshot field carriage to match lawful admission

Ensure `build_snapshot_entity()` and `build_snapshot_places()` do not reintroduce unlawful remote visibility by copying authoritative fields for entities or place membership that should not have been admitted. The ticket should name the canonical lawful carrier for each retained planner-visible fact: belief-backed entity state, explicit grounded evidence, or declared travel-topology contract.

### 3. Add focused planner-boundary coverage and doc correction

Add focused tests that:

- prove a remote wash-basin facility within travel horizon but absent from beliefs does not appear in snapshot-backed `goal_relevant_places`
- prove a believed/evidence-backed remote facility still appears lawfully
- prove local-water `Wash` still works without any basin belief

Update [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) so the live planner contract explicitly distinguishes authoritative travel topology from belief-backed remote entity visibility.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify only if focused proof shows snapshot-carrier fallout reaches goal helpers/tests)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `docs/planner-contracts.md` (modify)
- `tickets/S116DRIESCSUS-006.md` (modify only for dependency/scope alignment already exposed by reassessment)

## Out of Scope

- Rewriting the authoritative travel-topology contract in `PlanningSnapshot::collect_places()`
- Changing `wash_preconditions` or the lawful local-water `Wash` path
- Broad belief-envelope work from [specs/S113-belief-envelope.md](/home/joeloverbeck/projects/worldwake/specs/S113-belief-envelope.md)
- New golden coverage unless focused planner-boundary proof exposes a user-visible emergent regression that cannot be represented honestly at the planner layer

## Acceptance Criteria

### Tests That Must Pass

1. New `planning_snapshot` regression proves a remote un-believed wash-basin facility within travel horizon is absent from snapshot-backed planner entity/facility visibility.
2. New focused planner/search regression proves `GoalKind::Wash` does not derive remote basin-guidance from that hidden facility.
3. New or existing focused regression proves a believed/evidence-backed remote facility still surfaces lawfully.
4. Existing `candidate_generation::tests::wash_requires_dirtiness_and_local_water` passes unchanged.
5. Existing `planner_conformance::conformance_wash` passes unchanged.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner-visible remote entities and their fields must come from belief carriage or explicit grounded evidence, not from authoritative `entities_at(place)` truth alone.
2. Authoritative travel topology remains the only in-scope non-belief planner substrate preserved by this ticket; no broader omniscient snapshot fallback remains.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — focused snapshot admission regressions for remote un-believed facilities versus believed/evidence-backed facilities
2. `crates/worldwake-ai/src/search/tests.rs` or `crates/worldwake-ai/src/goal_model.rs` tests — planner-visible `Wash` guidance regression covering the repaired snapshot path
3. Existing `tests/planner_conformance.rs` usage in verification — confirms the lawful local-water `Wash` path stays intact

### Commands

1. `cargo test -p worldwake-ai planning_snapshot::tests`
2. `cargo test -p worldwake-ai goal_model::tests::wash_returns_empty_without_wash_basins -- --exact`
3. `cargo test -p worldwake-ai --test planner_conformance conformance_wash -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Repaired `PlanningSnapshot` admission so authoritative `entities_at(place)` is only used for the actor's current place; remote places now admit entities through remembered entity beliefs, explicit grounded evidence, and required containment/possession relatives rather than raw remote place truth.
- Tightened remote field carriage in `build_snapshot_entity()` and `build_snapshot_places()` so belief-backed remote entities preserve believed place/inventory/workstation/resource/wound state instead of silently rehydrating those fields from authority.
- Added focused planner-boundary regressions proving an un-believed remote wash basin stays hidden, a believed remote basin remains visible at the believed place, local `Wash` still works through the lawful possessed-water path, and the broader search/strategic fixtures now express remote knowledge through explicit beliefs instead of omniscient snapshot admission.
- Updated `docs/planner-contracts.md` to state the live split explicitly: authoritative current-place entity visibility, authoritative travel topology, and belief-backed remote entity visibility are distinct contracts.

## Deviations

- Reassessment showed the truthful fallout surface extended past `planning_snapshot.rs`: several existing `goal_model`, `search`, and `search::strategic` tests were relying on the unlawful remote-entity fallback. The production fix stayed at the snapshot boundary, and the dependent planner tests were corrected to seed the remote beliefs they actually needed.
- The live lawful carrier for remote planner visibility is narrower than the draft ticket's initial language implied. The landed fix preserves authoritative local-place admission and authoritative place graph visibility while removing only the raw remote entity/facility fallback.

## Verification Result

- Passed `cargo test -p worldwake-ai planning_snapshot::tests::build_snapshot_excludes_remote_unbelieved_facility_within_horizon -- --exact`
- Passed `cargo test -p worldwake-ai planning_snapshot::tests::build_snapshot_uses_belief_summary_for_remote_facility_visibility -- --exact`
- Passed `cargo test -p worldwake-ai goal_model::tests::wash_ignores_unbelieved_remote_wash_basin -- --exact`
- Passed `cargo test -p worldwake-ai goal_model::tests::wash_returns_places_with_wash_basin_belief -- --exact`
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_wash -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
