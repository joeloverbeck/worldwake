# S73PLASNAENT-002: Goal-aware entity filtering in planning snapshot

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — worldwake-ai planning snapshot construction, snapshot builder signatures
**Deps**: S73PLASNAENT-001

## Problem

Planning snapshot entity accumulation causes O(accumulated_entities) per-expansion cost in GOAP search. Over 10,000+ tick soak tests, per-agent-tick planning cost grows 25x (0.8ms → 20.7ms). The root cause is `collect_entities` including all believed entities at places regardless of goal relevance. This ticket implements goal-aware Tier 2 filtering and a per-place entity cap.

## Assumption Reassessment (2026-04-08)

1. `collect_entities` at `planning_snapshot.rs:867` takes `(view, actor, evidence_entities, included_places)` and calls `view.entities_at(*place)` at line 878 to collect all entities, then walks containment graph (lines 881-898). The filter must be applied at the `entities_at` collection point (line 878), before the containment walk.
2. `PlanningSnapshot::build_with_blocked_facility_uses` at `planning_snapshot.rs:286` calls `collect_entities` at line 301. The public wrapper `build_planning_snapshot_with_blocked_facility_uses` at line 797 delegates to it.
3. `build_candidate_plans` at `agent_tick/planning.rs:262` calls `build_planning_snapshot_with_blocked_facility_uses`, passing `ranked.grounded.evidence_entities`, `ranked.grounded.evidence_places`, `cognitive.snapshot_travel_horizon`. The goal kind is available as `ranked.grounded.key.kind` — `GoalKind::relevant_op_kinds()` is callable on it via `GoalKindPlannerExt` trait.
4. `PlannerOpKind` at `planner_ops.rs:13` has 37 variants. Item-interacting ops confirmed: `Consume`, `Trade`, `Craft`, `Loot`, `Harvest`, `MoveCargo`, `StockManagement`, `Heal`. Institutional ops confirmed: `ConsultRecord`, `PostBounty`, `ClaimBounty`, `PostNotice`, `Accuse`, `Fine`, `Investigate`, `Bribe`, `Threaten`, `Exile`, `DeclareSupport`, `PressForceClaim`, `YieldForceClaim`.
5. `EntityKind` at `entity.rs:8` has 10 variants: `Agent`, `ItemLot`, `UniqueItem`, `Container`, `Facility`, `Place`, `Faction`, `Office`, `Record`, `SocialArtifact`. All must be handled in the filter.
6. `view.entity_kind(entity)` is available on `RuntimeBeliefView` — confirmed at `planning_snapshot.rs:996` in test mock and `belief_view.rs:46` in trait def.
7. `BelievedEntityState.observed_tick` at `belief.rs:1295` exists and is `Tick` type — usable for per-place cap recency ordering.
8. `BelievedEntityState.alive` at `belief.rs:1285` exists — usable for agent alive check in filter.
9. `build_planning_snapshot` (non-blocked variant, line 780) has ~240 call sites across 10 files — almost all in tests. These must accept a filter parameter. Providing `SnapshotEntityFilter::unfiltered()` preserves existing test behavior with minimal call-site changes.
10. `build_planning_snapshot` is re-exported from `lib.rs:93` — signature change affects the public API of the `worldwake-ai` crate (crate-internal only, no external consumers).

## Architecture Check

1. The filter is a transient derived computation (per planning pass), not stored state — aligns with P27. It is constructed from `relevant_op_kinds()` which already exists as the goal's declared planner operation set.
2. No cross-system coupling — the filter reads `EntityKind` (worldwake-core) and `PlannerOpKind` (worldwake-ai internal). Both are already in scope at the snapshot construction site.
3. No backward-compatibility shims. All callers are updated. Test callers use `SnapshotEntityFilter::unfiltered()`.

## Verification Layers

1. Tier 1 entities (actor, evidence, places, possession chain) always included regardless of filter -> focused unit tests in `planning_snapshot.rs`
2. Tier 2 filtering excludes items when goal has no item ops -> focused unit test
3. Tier 2 filtering includes items when goal has item ops -> focused unit test
4. Tier 2 filtering excludes dead agents when goal has no `Loot` op -> focused unit test
5. Tier 2 filtering includes dead agents when goal has `Loot` op -> focused unit test
6. Tier 2 filtering excludes records/artifacts when goal has no institutional ops -> focused unit test
7. Per-place cap limits entities and prefers most recently observed -> focused unit test
8. Per-place cap tie-breaking by EntityId is deterministic -> focused unit test
9. All golden tests pass (correctness preservation) -> `cargo test -p worldwake-ai`
10. `golden_loot_corpse_*` tests pass (dead agent inclusion for Loot goals) -> golden E2E

## What to Change

### 1. Add SnapshotEntityFilter struct

In `crates/worldwake-ai/src/planning_snapshot.rs`, add:

```rust
pub(crate) struct SnapshotEntityFilter {
    needs_items: bool,
    needs_institutional: bool,
    needs_dead_agents: bool,
}
```

With constructors:
- `fn from_relevant_ops(ops: &[PlannerOpKind]) -> Self` — derives the three booleans by checking intersection with the item-interacting and institutional op sets. `needs_dead_agents` is true if ops contains `PlannerOpKind::Loot`.
- `fn unfiltered() -> Self` — returns `Self { needs_items: true, needs_institutional: true, needs_dead_agents: true }` for test callers and the non-goal-aware `build_planning_snapshot` path.

Add a `fn includes(&self, kind: EntityKind, alive: bool) -> bool` method implementing the Tier 2 predicate per the spec's entity-kind rules.

### 2. Modify collect_entities

Change signature to accept `filter: &SnapshotEntityFilter` and `max_per_place: u16` and `view` (for observed_tick lookups).

Replace the blanket `included.extend(view.entities_at(*place))` (line 878) with:
1. Collect `view.entities_at(*place)` into a temporary vec.
2. Filter by `filter.includes(view.entity_kind(entity), view.believed_alive(entity))` — need to check if `believed_alive` or equivalent exists, otherwise use `BelievedEntityState.alive` through the view.
3. If filtered count > `max_per_place`, sort by `(observed_tick desc, entity_id desc)` and truncate.
4. Extend `included` with the filtered+capped set.

The containment walk (lines 881-898) remains unchanged — it operates on whatever was included.

### 3. Modify PlanningSnapshot::build and build_with_blocked_facility_uses

Add `filter: &SnapshotEntityFilter` and `max_per_place: u16` parameters to both `PlanningSnapshot::build` and `PlanningSnapshot::build_with_blocked_facility_uses`. Thread them to `collect_entities`.

### 4. Modify public wrapper functions

Update `build_planning_snapshot` (line 780) and `build_planning_snapshot_with_blocked_facility_uses` (line 797) signatures to accept `relevant_ops: &[PlannerOpKind]` and `max_per_place: u16`. Construct the `SnapshotEntityFilter` internally from `relevant_ops`.

For `build_planning_snapshot` specifically, also provide a convenience that test callers can use. Options:
- Add `relevant_ops` and `max_per_place` parameters with test callers passing `&[]` and `u16::MAX` (unfiltered equivalent). OR
- Keep the old signature as `build_planning_snapshot_unfiltered` for tests.

Recommendation: add the parameters to both public functions. Test call sites pass `&[]` (empty ops = unfiltered via `SnapshotEntityFilter::from_relevant_ops(&[])` which should return unfiltered) and `u16::MAX` (effectively no cap). This is a mechanical find-and-replace across test sites.

### 5. Thread from build_candidate_plans

In `crates/worldwake-ai/src/agent_tick/planning.rs` at line 262, change:
```rust
let snapshot = build_planning_snapshot_with_blocked_facility_uses(
    &view, agent,
    &ranked.grounded.evidence_entities,
    &ranked.grounded.evidence_places,
    cognitive.snapshot_travel_horizon,
    blocked_memory,
    current_tick,
);
```
to:
```rust
let snapshot = build_planning_snapshot_with_blocked_facility_uses(
    &view, agent,
    &ranked.grounded.evidence_entities,
    &ranked.grounded.evidence_places,
    cognitive.snapshot_travel_horizon,
    blocked_memory,
    current_tick,
    ranked.grounded.key.kind.relevant_op_kinds(),
    cognitive.max_snapshot_entities_per_place,
);
```

This requires importing `GoalKindPlannerExt` trait if not already in scope.

### 6. Update all test call sites

Grep for `build_planning_snapshot(` and `build_planning_snapshot_with_blocked_facility_uses(` across `worldwake-ai`. Update each call to include the two new parameters (`&[]` and `u16::MAX` for unfiltered tests, or goal-specific ops for filter-exercising tests).

Files with call sites (from grep): `planning_state.rs` (~40), `planning_snapshot.rs` (~17), `goal_model.rs` (~71 — includes non-call-site matches), `search/tests.rs` (~100), `planner_ops.rs` (~3), `agent_tick/tests.rs` (~1), `golden_offices.rs` (~2), `golden_care.rs` (~3), `planner_conformance.rs` (~2).

### 7. Add focused unit tests for the filter

Add tests in `planning_snapshot.rs` `#[cfg(test)]` module:
- `snapshot_filter_excludes_items_for_travel_only_goal` — set up entities at a place with mixed kinds, filter with `relevant_ops = &[PlannerOpKind::Travel]`, verify `ItemLot`/`UniqueItem` excluded.
- `snapshot_filter_includes_items_for_trade_goal` — filter with ops including `Trade`, verify items included.
- `snapshot_filter_excludes_dead_agents_without_loot` — dead agent at place, filter without `Loot`, verify excluded.
- `snapshot_filter_includes_dead_agents_with_loot` — filter with `Loot`, verify dead agent included.
- `snapshot_filter_excludes_records_without_institutional_ops` — filter with `Travel` only, verify `Record`/`SocialArtifact`/`Faction` excluded.
- `snapshot_filter_includes_records_with_institutional_ops` — filter with `ConsultRecord`, verify included.
- `snapshot_per_place_cap_limits_entities` — place with 60 items, cap at 50, verify only 50 included.
- `snapshot_per_place_cap_prefers_recent` — entities with different `observed_tick`, verify most recent kept.
- `snapshot_per_place_cap_tiebreak_by_entity_id` — same `observed_tick`, verify deterministic EntityId ordering.
- `snapshot_filter_always_includes_evidence_entities` — evidence entity is an item, filter excludes items, verify evidence entity still included (Tier 1).
- `snapshot_filter_containment_walk_includes_inventory` — agent included, their possessed items should be in snapshot even if items filtered at place level.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add `SnapshotEntityFilter`, modify `collect_entities`, modify `build`/`build_with_blocked_facility_uses`, update test call sites, add new tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — thread `relevant_op_kinds()` and `max_snapshot_entities_per_place`)
- `crates/worldwake-ai/src/planning_state.rs` (modify — update test call sites)
- `crates/worldwake-ai/src/goal_model.rs` (modify — update test call sites)
- `crates/worldwake-ai/src/search/tests.rs` (modify — update test call sites)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — update test call sites)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update test call sites)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — update test call sites)
- `crates/worldwake-ai/tests/golden_care.rs` (modify — update test call sites)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify — update test call sites)

## Out of Scope

- Belief store changes — unchanged (P14, P15, P16)
- Authoritative world state — unchanged (P4)
- Perception system — unchanged
- New CognitiveProfile fields beyond what ticket 001 added
- Soak performance benchmarking — that is ticket 003
- Changes to `get_affordances_for_defs` or `search_candidates` — only snapshot construction changes
- Scenario RON file changes — default CognitiveProfile covers existing scenarios

## Acceptance Criteria

### Tests That Must Pass

1. All new focused unit tests for `SnapshotEntityFilter` (11 tests listed in What to Change section 7)
2. All existing `planning_snapshot` tests pass with unfiltered parameters
3. All golden tests: `cargo test -p worldwake-ai --test golden_emergent`
4. `golden_loot_corpse_self_care_chain` passes (dead agent included for Loot goal)
5. `golden_loot_corpse_self_care_chain_replays_deterministically` passes
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Tier 1 entities (actor, evidence, places, possession chain) are always included regardless of filter settings
2. `SnapshotEntityFilter::from_relevant_ops(&[])` (empty ops) produces unfiltered behavior — all entity kinds included
3. The containment walk is unaffected by the filter — possessed items of included entities are always reachable
4. Per-place cap `u16::MAX` effectively disables capping
5. Planning snapshot remains a derived view over belief state (P27) — no stored filter state
6. `PlanningSnapshot` determinism is preserved — same inputs produce same snapshot (BTreeMap ordering, EntityId tiebreak)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_excludes_items_for_travel_only_goal` — verifies items excluded when irrelevant
2. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_includes_items_for_trade_goal` — verifies items included when relevant
3. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_excludes_dead_agents_without_loot` — verifies dead agent exclusion
4. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_includes_dead_agents_with_loot` — verifies dead agent inclusion for Loot
5. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_excludes_records_without_institutional_ops` — verifies record/artifact exclusion
6. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_includes_records_with_institutional_ops` — verifies record/artifact inclusion
7. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_per_place_cap_limits_entities` — verifies cap enforcement
8. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_per_place_cap_prefers_recent` — verifies recency ordering
9. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_per_place_cap_tiebreak_by_entity_id` — verifies deterministic tiebreak
10. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_always_includes_evidence_entities` — verifies Tier 1 preservation
11. `crates/worldwake-ai/src/planning_snapshot.rs::snapshot_filter_containment_walk_includes_inventory` — verifies possession walk unaffected

### Commands

1. `cargo test -p worldwake-ai -- snapshot_filter`
2. `cargo test -p worldwake-ai -- snapshot_per_place_cap`
3. `cargo test -p worldwake-ai --test golden_emergent -- golden_loot_corpse`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
