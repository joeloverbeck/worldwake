# S84SHBELOP-001: Diagnose and fix ShareBelief frontier exhaustion

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning snapshot place indexing (`worldwake-ai`)
**Deps**: S69 (completed)

## Problem

ShareBelief goals consistently frontier-exhaust at depth 0 with 1 expansion despite agents being co-located. 14 of 20 failed plans in simulation are frontier-exhausted ShareBelief goals. The planner finds the Tell action def but cannot find any affordances for it because the listener entity, while included in the snapshot's entity set via `evidence_entities`, is not indexed at the actor's place in the snapshot's place map — `build_snapshot_places` only indexes entities whose `effective_place` matches an included place. If the agent lacks a belief about the listener's location, the listener is invisible to the affordance query.

## Assumption Reassessment (2026-04-10)

1. **Tell action preconditions confirmed**: `tell_action_def` at `crates/worldwake-systems/src/tell_actions.rs:38-77` has `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }`, `Precondition::ActorAlive`, `TargetExists(0)`, `TargetAtActorPlace(0)`, `TargetKind(0, Agent)`, `TargetAlive(0)`. All confirmed via grep.
2. **Snapshot entity inclusion confirmed**: `collect_entities` at `planning_snapshot.rs:1108` unconditionally includes `evidence_entities`. The listener is an evidence entity via `OpportunityAnchor::Entity(listener)` at `observation.rs:232-233`. However, `build_snapshot_places` at `planning_snapshot.rs:789-792` only indexes entities at places where `view.effective_place(*entity) == Some(place)`.
3. **Shared boundary**: The boundary under audit is the planning snapshot's `entities_at` index — specifically `build_snapshot_places` which bridges entity inclusion (correct) with place-indexed visibility (broken when `effective_place` is `None`).
5. **Live GoalKind**: `GoalKind::ShareBelief { listener, topic, communication_class }` at `goal.rs:95-99`. Three dispatch variants (Alarm, Testimony, Gossip) all use `relevant_ops: [PlannerOpKind::Tell]`, `feasibility_strategy: ColocationOrDead` at `goal_dispatch_decl.rs:507-533`.
6. **Layer**: Planning snapshot construction (AI crate), not candidate generation or runtime `agent_tick`. Full action registries not required for the fix — the snapshot is built from belief view data only.

## Architecture Check

1. **Option A (preferred)**: For evidence entities whose `effective_place` is `None`, derive their place from the candidate generation's evidence — if the entity was included as an evidence entity for a goal anchored at a specific place, that place is already in the snapshot's included places. This is cleaner than patching the perception pipeline because it fixes the snapshot's self-consistency without requiring broader changes. It preserves FND-14 (belief-only planning) since the evidence entity's inclusion was already determined from the agent's belief state at candidate generation time.
2. No backward-compatibility shims. The fix modifies `build_snapshot_places` to handle evidence entities more completely — existing behavior for non-evidence entities is unchanged.

## Verification Layers

1. Listener appears in `snapshot.entities_at(actor_place)` after fix -> focused unit test on `build_snapshot_places` with evidence entity lacking `effective_place`
2. Tell affordance found for co-located listener -> focused unit test on `get_affordances_for_defs` with the fixed snapshot
3. ShareBelief plan search succeeds -> decision trace in golden test (S84SHBELOP-004)
6. Single-layer ticket: the fix is entirely within snapshot construction. The downstream proof (plan search succeeding) is covered by S84SHBELOP-004.

## What to Change

### 1. Investigate root cause

Run the diagnostic steps from the spec:
- Instrument or trace `build_snapshot_places` to confirm whether the listener entity's `effective_place` returns `None` during snapshot construction for ShareBelief goals.
- If confirmed (hypothesis d), proceed with the fix below.
- If not confirmed, trace `get_affordances_for_defs` to identify the actual failure point and adjust the fix accordingly.

### 2. Fix `build_snapshot_places` for evidence entities

In `crates/worldwake-ai/src/planning_snapshot.rs`, function `build_snapshot_places` (line 779):

Currently, the entity-to-place assignment at line 792 filters by `view.effective_place(*entity) == Some(place)`. For evidence entities whose `effective_place` is `None`, this silently drops them from all place indexes.

Add a fallback: after the primary `effective_place` pass, iterate over `evidence_entities` that were not assigned to any place. For each, check if any included place's `entities_at` (from the belief view, not the snapshot) contains the entity. If found, add the entity to that place's index.

This preserves FND-14: the fallback still queries the belief view (`view.entities_at`), not authoritative state. The entity was already in the snapshot's entity set — this just ensures it appears in the correct place index.

### 3. Add focused test for snapshot place indexing of evidence entities

Add a test in `planning_snapshot.rs` tests that:
- Creates a belief view where entity A is at place P, entity B is alive but has no `effective_place` belief
- Builds a snapshot with entity B as an evidence entity
- Asserts entity B appears in `snapshot.entities_at(place_P)` after the fix

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)

## Out of Scope

- Reworking the tell action semantics or payload structure
- Adding travel-to-listener planning
- Modifying goal ranking or priority for ShareBelief
- Changes to the perception pipeline (Option B from spec — deferred unless investigation proves Option A insufficient)
- Pre-search target validation (S84SHBELOP-002)
- Diagnostic enhancement to `PlanAttemptTrace` (S84SHBELOP-003)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: evidence entity without `effective_place` appears in snapshot `entities_at` for its evidence place
2. New focused test: `get_affordances_for_defs` returns Tell affordance when listener is an evidence entity at actor's place
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Evidence entities are always indexed in at least one place in the snapshot's place map (if they are at any included place per the belief view)
2. FND-14: snapshot construction never reads authoritative world state — all place resolution uses the belief view
3. Non-evidence entity behavior unchanged — only evidence entities with missing `effective_place` gain the fallback

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` (test module) — evidence entity place indexing fallback
2. `crates/worldwake-ai/src/planning_snapshot.rs` (test module) — affordance query with fixed snapshot returns Tell affordance

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-10.

- Updated `crates/worldwake-ai/src/planning_snapshot.rs` so `build_snapshot_places` falls back to the belief-view `entities_at(place)` index for evidence entities that are included in the snapshot but lack `effective_place`.
- Kept the fallback scoped to evidence entities only, preserving the existing primary `effective_place` indexing path for all other snapshot entities.
- Added focused planner-boundary regressions proving both the snapshot place index and Tell affordance enumeration now succeed when the co-located listener is carried only through `evidence_entities`.
- Deviation from original S84 spec scope: this ticket closed only the snapshot-indexing root cause. Early pruning, richer frontier-exhaustion diagnostics, and end-to-end golden coverage remain separately owned by `S84SHBELOP-002`, `S84SHBELOP-003`, and `S84SHBELOP-004`.

## Verification Result

- Passed `cargo test -p worldwake-ai evidence_entity_at_place_when_effective_place_is_missing`
- Passed `cargo test -p worldwake-ai tell_affordance_surfaces_for_evidence_listener_without_effective_place`
- Passed `cargo test -p worldwake-ai planning_snapshot`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Notes

- Active ticket file status: untracked draft (`tickets/S84SHBELOP-001.md`).
